//! Widget « system » : CPU, mémoire, charge, disques, durée d'allumage.

use std::sync::{LazyLock, Mutex};

use sysinfo::{Disks, System};

use super::SysLine;
use crate::config::SysItem;

/// Un seul `System` partagé : l'usage CPU se calcule entre deux relevés.
static SYS: LazyLock<Option<Mutex<System>>> = LazyLock::new(|| System::new().ok().map(Mutex::new));

fn human_bytes(b: u64) -> String {
    // Unités binaires : octets en français, bytes en anglais.
    const FR: [&str; 5] = ["o", "Kio", "Mio", "Gio", "Tio"];
    const EN: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let units = match crate::i18n::current() {
        crate::i18n::Lang::Fr => FR,
        crate::i18n::Lang::En => EN,
    };
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < units.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i >= 3 {
        format!("{v:.1} {}", units[i])
    } else {
        format!("{v:.0} {}", units[i])
    }
}

fn human_duration(secs: u64) -> String {
    let (d, h, m) = (secs / 86400, (secs % 86400) / 3600, (secs % 3600) / 60);
    if d > 0 {
        format!("{d} j {h} h")
    } else {
        format!("{h} h {m:02}")
    }
}

/// Relevé synchrone (à appeler dans `spawn_blocking`).
pub fn sample_system(items: &[SysItem]) -> Vec<SysLine> {
    let Some(sys) = SYS.as_ref() else {
        return vec![SysLine {
            label: crate::i18n::tr("système").into(),
            value: "indisponible".into(),
            ratio: None,
        }];
    };
    let mut sys = sys.lock().unwrap_or_else(|e| e.into_inner());
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    let need_disks = items.iter().any(|i| matches!(i, SysItem::Disk(_)));
    let disks = need_disks
        .then(Disks::new_with_refreshed_list)
        .and_then(Result::ok);
    items
        .iter()
        .map(|it| match it {
            SysItem::Cpu => {
                let u = sys.global_cpu_usage();
                SysLine {
                    label: "CPU".into(),
                    value: format!("{u:.0} %"),
                    ratio: Some(u / 100.0),
                }
            }
            SysItem::Ram => {
                let (used, total) = (sys.used_memory(), sys.total_memory().max(1));
                SysLine {
                    label: "RAM".into(),
                    value: format!("{} / {}", human_bytes(used), human_bytes(total)),
                    ratio: Some(used as f32 / total as f32),
                }
            }
            SysItem::Load => {
                let value = System::load_average()
                    .map(|l| format!("{:.2} {:.2} {:.2}", l.one, l.five, l.fifteen))
                    .unwrap_or_else(|_| "?".into());
                SysLine {
                    label: crate::i18n::tr("Charge").into(),
                    value,
                    ratio: None,
                }
            }
            SysItem::Uptime => SysLine {
                label: crate::i18n::tr("Allumé").into(),
                value: System::uptime()
                    .map(human_duration)
                    .unwrap_or_else(|_| "?".into()),
                ratio: None,
            },
            SysItem::Disk(mount) => {
                let d = disks.as_ref().and_then(|ds| {
                    ds.iter()
                        .find(|d| d.mount_point() == std::path::Path::new(mount))
                });
                match d {
                    Some(d) => {
                        let total = d.total_space().max(1);
                        let used = total.saturating_sub(d.available_space());
                        SysLine {
                            label: mount.clone(),
                            value: format!("{} / {}", human_bytes(used), human_bytes(total)),
                            ratio: Some(used as f32 / total as f32),
                        }
                    }
                    None => SysLine {
                        label: mount.clone(),
                        value: crate::i18n::tr("non monté").into(),
                        ratio: None,
                    },
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tailles_et_durees() {
        assert_eq!(human_bytes(512), "512 o");
        assert_eq!(human_bytes(3 * 1024 * 1024 * 1024 / 2), "1.5 Gio");
        assert_eq!(human_duration(90061), "1 j 1 h");
        assert_eq!(human_duration(3720), "1 h 02");
    }

    #[test]
    fn systeme_sans_planter() {
        let l = sample_system(&[SysItem::Ram, SysItem::Disk("/n/existe/pas".into())]);
        assert_eq!(l.len(), 2);
        assert_eq!(l[1].value, "non monté");
    }
}
