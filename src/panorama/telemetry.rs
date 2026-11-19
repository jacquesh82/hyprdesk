//! Capteurs locaux : aucune valeur simulée lorsqu'un capteur est absent.
use crate::widgets::SysLine;

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
                "cpu_temp" | "gpu_temp" => {
                    let temp = temperature(id.trim() == "gpu_temp");
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
