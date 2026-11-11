//! Luminosité de l'écran et du clavier, sans droits particuliers : lecture
//! dans sysfs (`/sys/class/backlight`, `/sys/class/leds/*kbd_backlight*`),
//! écriture par logind (`org.freedesktop.login1.Session.SetBrightness`), qui
//! l'autorise à la session active.
//!
//! Écrans externes : DDC/CI par `ddcutil` (code VCP 0x10, luminosité), un
//! bus I²C par écran. La détection est lente (quelques secondes) : faite
//! une fois, puis seules les valeurs sont relues.

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

/// Écran externe réglable par DDC/CI.
#[derive(Debug, Clone, PartialEq)]
pub struct Display {
    /// Connecteur, au nom de Hyprland (`DP-1`, `HDMI-A-2`).
    pub connector: String,
    pub bus: u32,
    pub current: u32,
    pub max: u32,
}

impl Display {
    pub fn level(&self) -> f32 {
        if self.max == 0 {
            0.0
        } else {
            self.current as f32 / self.max as f32
        }
    }
}

/// Écrans de `ddcutil detect --brief` : (bus I²C, connecteur sans `cardN-`).
pub fn parse_detect(out: &str) -> Vec<(u32, String)> {
    let mut list = Vec::new();
    let mut bus = None;
    for line in out.lines() {
        let l = line.trim();
        if l.starts_with("Display ") || l.starts_with("Invalid display") {
            bus = None;
            if l.starts_with("Invalid") {
                continue;
            }
        }
        if let Some(v) = l.strip_prefix("I2C bus:") {
            bus = v.trim().rsplit('-').next().and_then(|n| n.parse().ok());
        } else if let Some(v) = l.strip_prefix("DRM connector:")
            && let Some(b) = bus.take()
        {
            let c = v.trim();
            let c = c.split_once('-').map(|(_, r)| r).unwrap_or(c);
            list.push((b, c.to_string()));
        }
    }
    list
}

/// Valeur et maximum de `ddcutil getvcp 10 --brief` (`VCP 10 C 50 100`).
pub fn parse_vcp(out: &str) -> Option<(u32, u32)> {
    let mut it = out.split_whitespace();
    while let Some(w) = it.next() {
        if w == "VCP" {
            let _code = it.next()?;
            if it.next()? != "C" {
                return None;
            }
            let cur = it.next()?.parse().ok()?;
            let max: u32 = it.next()?.parse().ok()?;
            return (max > 0).then_some((cur, max));
        }
    }
    None
}

async fn ddcutil(args: &[String], timeout: u64) -> Option<String> {
    let o = crate::processes::output(
        tokio::process::Command::new("ddcutil").args(args),
        std::time::Duration::from_secs(timeout),
    )
    .await
    .ok()?;
    o.status
        .success()
        .then(|| String::from_utf8_lossy(&o.stdout).into_owned())
}

/// Écrans DDC/CI détectés (une fois par démon ; vide sans ddcutil).
pub async fn ddc_buses() -> Vec<(u32, String)> {
    static DETECTED: tokio::sync::OnceCell<Vec<(u32, String)>> = tokio::sync::OnceCell::const_new();
    DETECTED
        .get_or_init(|| async {
            if !crate::launch::executable("ddcutil") {
                return Vec::new();
            }
            ddcutil(&["detect".into(), "--brief".into()], 20)
                .await
                .map(|o| parse_detect(&o))
                .unwrap_or_default()
        })
        .await
        .clone()
}

/// Luminosité des écrans DDC/CI.
pub async fn ddc_displays() -> Vec<Display> {
    let mut out = Vec::new();
    for (bus, connector) in ddc_buses().await {
        let args = vec![
            "--bus".into(),
            bus.to_string(),
            "getvcp".into(),
            "10".into(),
            "--brief".into(),
        ];
        if let Some((current, max)) = ddcutil(&args, 5).await.as_deref().and_then(parse_vcp) {
            out.push(Display {
                connector,
                bus,
                current,
                max,
            });
        }
    }
    out
}

/// Règle la luminosité d'un écran DDC/CI (0..max).
pub async fn ddc_set(bus: u32, value: u32) -> Result<(), String> {
    let args = vec![
        "--bus".into(),
        bus.to_string(),
        "--noverify".into(),
        "setvcp".into(),
        "10".into(),
        value.to_string(),
    ];
    ddcutil(&args, 5)
        .await
        .map(|_| ())
        .ok_or_else(|| crate::tr!("ddcutil : écran du bus {b} injoignable", b = bus))
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

    #[test]
    fn ecrans_ddc() {
        let detect = "Display 1\n   I2C bus:          /dev/i2c-7\n   DRM connector:    card1-HDMI-A-2\n   Monitor:          HYC:PX7 Prime:\n\nInvalid display\n   I2C bus:          /dev/i2c-3\n\nDisplay 2\n   I2C bus:          /dev/i2c-12\n   DRM connector:    card1-DP-1\n";
        assert_eq!(
            parse_detect(detect),
            vec![(7, "HDMI-A-2".to_string()), (12, "DP-1".to_string())]
        );
        assert_eq!(parse_vcp("VCP 10 C 50 100"), Some((50, 100)));
        assert_eq!(parse_vcp("VCP 10 ERR"), None);
        assert_eq!(parse_vcp("VCP 10 C 0 0"), None);
    }
}
