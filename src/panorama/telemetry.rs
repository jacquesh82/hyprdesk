//! Capteurs locaux : aucune valeur simulée lorsqu'un capteur est absent.
use crate::widgets::SysLine;

/// Carte NVIDIA (pilote propriétaire, absent de hwmon) : température (°C),
/// charge (%), mémoire utilisée et totale (Mio), par `nvidia-smi`, relue au
/// plus toutes les 2 s.
fn nvidia() -> Option<(f32, f32, f32, f32)> {
    use std::sync::Mutex;
    use std::time::{Duration, Instant};
    type Reading = Option<(f32, f32, f32, f32)>;
    static LAST: Mutex<Option<(Instant, Reading)>> = Mutex::new(None);
    let mut last = LAST.lock().ok()?;
    if let Some((at, v)) = *last
        && at.elapsed() < Duration::from_secs(2)
    {
        return v;
    }
    let v = (|| {
        let out = std::process::Command::new("nvidia-smi")
            .args([
                "--query-gpu=temperature.gpu,utilization.gpu,memory.used,memory.total",
                "--format=csv,noheader,nounits",
            ])
            .stderr(std::process::Stdio::null())
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        parse_nvidia(&String::from_utf8_lossy(&out.stdout))
    })();
    *last = Some((Instant::now(), v));
    v
}

/// Première ligne de `nvidia-smi --format=csv,noheader,nounits`.
fn parse_nvidia(s: &str) -> Option<(f32, f32, f32, f32)> {
    let f: Vec<f32> = s
        .lines()
        .next()?
        .split(',')
        .map(|x| x.trim().parse().ok())
        .collect::<Option<_>>()?;
    (f.len() >= 4).then(|| (f[0], f[1], f[2], f[3]))
}

fn temperature(gpu: bool) -> Option<f32> {
    let devices = std::fs::read_dir("/sys/class/hwmon").ok()?;
    for entry in devices.flatten() {
        let dir = entry.path();
        let name = std::fs::read_to_string(dir.join("name")).unwrap_or_default();
        let relevant = if gpu {
            ["amdgpu", "nouveau", "nvidia"].contains(&name.trim())
        } else {
            ["coretemp", "k10temp", "zenpower", "cpu_thermal"].contains(&name.trim())
        };
        if !relevant {
            continue;
        }
        let mut values: Vec<_> = std::fs::read_dir(&dir)
            .ok()?
            .flatten()
            .filter(|e| {
                e.file_name().to_string_lossy().starts_with("temp")
                    && e.file_name().to_string_lossy().ends_with("_input")
            })
            .filter_map(|e| {
                std::fs::read_to_string(e.path())
                    .ok()?
                    .trim()
                    .parse::<f32>()
                    .ok()
            })
            .map(|n| n / 1000.0)
            .filter(|n| n.is_finite() && (-20.0..=150.0).contains(n))
            .collect();
        values.sort_by(f32::total_cmp);
        if let Some(v) = values.last() {
            return Some(*v);
        }
    }
    None
}

/// Relevé de chaque mesure demandée : (identifiant, ligne).
pub fn sample_ids(ids: &[String]) -> Vec<(String, SysLine)> {
    ids.iter()
        .filter_map(|id| sample(id).pop().map(|l| (id.clone(), l)))
        .collect()
}

pub fn sample(metrics: &str) -> Vec<SysLine> {
    metrics
        .split(',')
        .filter_map(|id| {
            use crate::config::SysItem;
            let item = match id.trim() {
                "cpu" => SysItem::Cpu,
                "ram" => SysItem::Ram,
                "disk" => SysItem::Disk("/".into()),
                "load" => SysItem::Load,
                "uptime" => SysItem::Uptime,
                "gpu" => {
                    let g = nvidia();
                    return Some(SysLine {
                        label: "GPU".into(),
                        value: g.map_or_else(|| "—".into(), |g| format!("{:.0} %", g.1)),
                        ratio: g.map(|g| (g.1 / 100.0).clamp(0.0, 1.0)),
                    });
                }
                "vram" => {
                    let g = nvidia().filter(|g| g.3 > 0.0);
                    return Some(SysLine {
                        label: "VRAM".into(),
                        value: g.map_or_else(
                            || "—".into(),
                            |g| format!("{:.1} GiB / {:.1} GiB", g.2 / 1024.0, g.3 / 1024.0),
                        ),
                        ratio: g.map(|g| (g.2 / g.3).clamp(0.0, 1.0)),
                    });
                }
                "cpu_temp" | "gpu_temp" => {
                    let gpu = id.trim() == "gpu_temp";
                    let temp = temperature(gpu).or_else(|| gpu.then(nvidia).flatten().map(|g| g.0));
                    return Some(SysLine {
                        label: if id.trim() == "gpu_temp" {
                            "GPU °C"
                        } else {
                            "CPU °C"
                        }
                        .into(),
                        value: temp.map_or_else(|| "—".into(), |n| format!("{n:.1} °C")),
                        ratio: temp.map(|n| (n / 100.0).clamp(0.0, 1.0)),
                    });
                }
                _ => return None,
            };
            crate::widgets::sample_system(&[item]).pop()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn nvidia_smi_lu() {
        assert_eq!(
            super::parse_nvidia("61.12, 38, 1118, 8192\n"),
            Some((61.12, 38.0, 1118.0, 8192.0))
        );
        assert_eq!(super::parse_nvidia("[N/A], 3, 1, 2"), None);
        assert_eq!(super::parse_nvidia(""), None);
    }
}
