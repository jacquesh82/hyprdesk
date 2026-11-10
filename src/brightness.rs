//! Luminosité de l'écran et du clavier, sans droits particuliers : lecture
//! dans sysfs (`/sys/class/backlight`, `/sys/class/leds/*kbd_backlight*`),
//! écriture par logind (`org.freedesktop.login1.Session.SetBrightness`), qui
//! l'autorise à la session active.

use std::path::{Path, PathBuf};

/// Un rétroéclairage réglable.
#[derive(Debug, Clone, PartialEq)]
pub struct Device {
    /// Sous-système logind : `backlight` (écran) ou `leds` (clavier).
    pub subsystem: &'static str,
    pub name: String,
    pub current: u32,
    pub max: u32,
}

impl Device {
    /// Niveau 0..1.
    pub fn level(&self) -> f32 {
        if self.max == 0 {
            0.0
        } else {
            self.current as f32 / self.max as f32
        }
    }
}

fn read_u32(p: &Path) -> Option<u32> {
    std::fs::read_to_string(p).ok()?.trim().parse().ok()
}

fn read_device(dir: &Path, subsystem: &'static str) -> Option<Device> {
    Some(Device {
        subsystem,
        name: dir.file_name()?.to_str()?.to_string(),
        current: read_u32(&dir.join("brightness"))?,
        max: read_u32(&dir.join("max_brightness")).filter(|m| *m > 0)?,
    })
}

fn devices_in(root: &Path, subsystem: &'static str, keep: impl Fn(&str) -> bool) -> Vec<Device> {
    let mut dirs: Vec<PathBuf> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(&keep))
        .collect();
    dirs.sort();
    dirs.iter()
        .filter_map(|d| read_device(d, subsystem))
        .collect()
}

/// Rétroéclairage de l'écran : le plus fin (le plus de paliers), en général
/// celui du pilote graphique plutôt qu'`acpi_video`.
pub fn screen() -> Option<Device> {
    devices_in(Path::new("/sys/class/backlight"), "backlight", |_| true)
        .into_iter()
        .max_by_key(|d| d.max)
}

/// Rétroéclairage du clavier.
pub fn keyboard() -> Option<Device> {
    devices_in(Path::new("/sys/class/leds"), "leds", |n| {
        n.contains("kbd_backlight")
    })
    .into_iter()
    .next()
}

/// Valeur suivante pour un pas en pour cent (`+5`, `-5`) ; l'écran ne
/// descend jamais à 0 (noir complet), le clavier si.
pub fn next_value(d: &Device, percent: i32) -> u32 {
    let step = (d.max as f64 * percent.unsigned_abs() as f64 / 100.0)
        .round()
        .max(1.0) as i64;
    let v = if percent >= 0 {
        d.current as i64 + step
    } else {
        d.current as i64 - step
    };
    let floor = if d.subsystem == "backlight" { 1 } else { 0 };
    v.clamp(floor, d.max as i64) as u32
}

/// Applique une valeur par logind.
pub async fn set(d: &Device, value: u32) -> Result<(), String> {
    let conn = zbus::Connection::system()
        .await
        .map_err(|e| e.to_string())?;
    conn.call_method(
        Some("org.freedesktop.login1"),
        "/org/freedesktop/login1/session/auto",
        Some("org.freedesktop.login1.Session"),
        "SetBrightness",
        &(d.subsystem, d.name.as_str(), value),
    )
    .await
    .map(|_| ())
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dev(subsystem: &'static str, current: u32, max: u32) -> Device {
        Device {
            subsystem,
            name: "x".into(),
            current,
            max,
        }
    }

    #[test]
    fn pas_et_bornes() {
        let d = dev("backlight", 500, 1000);
        assert_eq!(next_value(&d, 5), 550);
        assert_eq!(next_value(&d, -5), 450);
        assert_eq!(
            next_value(&dev("backlight", 20, 1000), -5),
            1,
            "jamais noir"
        );
        assert_eq!(next_value(&dev("backlight", 990, 1000), 5), 1000);
        // Clavier à 3 paliers : un pas fait au moins un palier, 0 permis.
        assert_eq!(next_value(&dev("leds", 1, 3), -10), 0);
        assert_eq!(next_value(&dev("leds", 1, 3), 10), 2);
        assert!((dev("leds", 1, 4).level() - 0.25).abs() < 1e-6);
    }

    #[test]
    fn lecture_sysfs() {
        let root = std::env::temp_dir().join(format!("hyprdesk-bl-{}", std::process::id()));
        for (name, cur, max) in [
            ("acpi_video0", "5", "10"),
            ("intel_backlight", "300", "1200"),
        ] {
            let d = root.join(name);
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(d.join("brightness"), cur).unwrap();
            std::fs::write(d.join("max_brightness"), max).unwrap();
        }
        let all = devices_in(&root, "backlight", |_| true);
        assert_eq!(all.len(), 2);
        let best = all.into_iter().max_by_key(|d| d.max).unwrap();
        assert_eq!(
            (best.name.as_str(), best.current, best.max),
            ("intel_backlight", 300, 1200)
        );
        std::fs::remove_dir_all(&root).unwrap();
    }
}
