//! Carte graphique du rendu (Réglages → Performance → Carte graphique).
//!
//! - **Hyprland** dessine tout sur sa carte « principale » : la première de
//!   `AQ_DRM_DEVICES`, sinon celle qu'aquamarine trouve en premier. Si les
//!   écrans sont branchés sur une autre carte, chaque image y est recopiée :
//!   la carte de rendu travaille pour rien (une iGPU à fond pour deux écrans
//!   1440p à 144 Hz branchés sur la carte dédiée, par exemple).
//!   `hyprland_gpu` désigne la carte par son **adresse PCI** (stable) ; le
//!   bloc Lua de `env.lua` la retrouve à chaque démarrage de Hyprland
//!   (`/sys/class/drm/cardN/device/uevent` : les numéros `cardN` changent
//!   d'un démarrage à l'autre, et les chemins `by-path` contiennent des « : »,
//!   le séparateur d'`AQ_DRM_DEVICES`) et la pose en tête, les autres cartes
//!   à la suite (leurs écrans restent utilisables). Absente au démarrage
//!   (réservée aux VM par vfio-pci), Hyprland retombe sur les autres.
//!   Pris en compte au démarrage suivant de Hyprland.
//! - **hyprdesk** suit par défaut la carte de rendu de Hyprland (pas de
//!   recopie entre cartes pour ses surfaces) : `HYPRDESK_WGPU_PCI` désigne
//!   l'adaptateur à wgpu (copie locale d'iced_wgpu).

use std::path::Path;

/// Carte graphique vue par le noyau (nœud DRM).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    /// `card0`.
    pub node: String,
    /// Adresse PCI (`0000:01:00.0`).
    pub slot: String,
    /// Identifiants PCI (fabricant, modèle).
    pub vendor: u32,
    pub device: u32,
    pub driver: String,
    /// Nom lisible (`lspci`, sinon le fabricant).
    pub name: String,
    /// Sorties branchées (`DP-2`…).
    pub outputs: Vec<String>,
}

impl Card {
    /// `10de:2488`, pour wgpu.
    pub fn pci_id(&self) -> String {
        format!("{:04x}:{:04x}", self.vendor, self.device)
    }
}

/// Nom d'un fabricant de cartes graphiques.
pub fn vendor_name(vendor: u32) -> &'static str {
    match vendor {
        0x8086 => "Intel",
        0x10de => "NVIDIA",
        0x1002 => "AMD",
        0x1af4 => "virtio",
        0x15ad => "VMware",
        _ => "?",
    }
}

/// Champs utiles d'un `uevent` de périphérique PCI.
pub fn parse_uevent(s: &str) -> Option<(String, u32, u32, String)> {
    let get = |k: &str| {
        s.lines()
            .find_map(|l| l.strip_prefix(k).and_then(|r| r.strip_prefix('=')))
            .map(str::trim)
    };
    let slot = get("PCI_SLOT_NAME")?.to_string();
    let (v, d) = get("PCI_ID")?.split_once(':')?;
    let vendor = u32::from_str_radix(v, 16).ok()?;
    let device = u32::from_str_radix(d, 16).ok()?;
    Some((
        slot,
        vendor,
        device,
        get("DRIVER").unwrap_or("").to_string(),
    ))
}

fn lspci_name(slot: &str) -> Option<String> {
    let out = std::process::Command::new("lspci")
        .args(["-mm", "-s", slot])
        .output()
        .ok()?;
    // « 01:00.0 "VGA compatible controller" "NVIDIA Corporation" "GA104 [GeForce RTX 3070]" … »
    let line = String::from_utf8_lossy(&out.stdout).to_string();
    let fields: Vec<&str> = line.split('"').filter(|f| !f.trim().is_empty()).collect();
    let model = fields.get(3)?.trim();
    let model = model
        .rsplit_once('[')
        .map(|(_, m)| m.trim_end_matches(']').trim())
        .unwrap_or(model);
    Some(model.to_string())
}

/// Cartes graphiques à nœud DRM (celles que Hyprland peut utiliser).
pub fn cards() -> Vec<Card> {
    cards_in(Path::new("/sys/class/drm"))
}

fn cards_in(root: &Path) -> Vec<Card> {
    let Ok(dir) = std::fs::read_dir(root) else {
        return Vec::new();
    };
    let mut names: Vec<String> = dir
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .collect();
    names.sort();
    let mut out = Vec::new();
    for node in names
        .iter()
        .filter(|n| n.starts_with("card") && !n.contains('-'))
    {
        let Ok(u) = std::fs::read_to_string(root.join(node).join("device/uevent")) else {
            continue;
        };
        let Some((slot, vendor, device, driver)) = parse_uevent(&u) else {
            continue;
        };
        let outputs = names
            .iter()
            .filter_map(|n| n.strip_prefix(&format!("{node}-")))
            .filter(|o| {
                std::fs::read_to_string(root.join(format!("{node}-{o}")).join("status"))
                    .is_ok_and(|s| s.trim() == "connected")
            })
            .map(str::to_string)
            .collect();
        let name = lspci_name(&slot)
            .map(|m| format!("{} {m}", vendor_name(vendor)))
            .unwrap_or_else(|| vendor_name(vendor).to_string());
        out.push(Card {
            node: node.clone(),
            slot,
            vendor,
            device,
            driver,
            name,
            outputs,
        });
    }
    out
}

/// Nœud de rendu (`renderD128`) → adresse PCI.
fn slot_of_node(node: &str) -> Option<String> {
    let u = std::fs::read_to_string(format!("/sys/class/drm/{node}/device/uevent")).ok()?;
    parse_uevent(&u).map(|(s, ..)| s)
}

/// Nœud de la carte de rendu d'après le journal de Hyprland
/// (« Creating CDRMRenderer on gpu /dev/dri/renderD128 »).
pub fn renderer_node_in_log(log: &str) -> Option<String> {
    log.lines().find_map(|l| {
        let i = l.find("Creating CDRMRenderer on gpu ")?;
        let p = l[i..].split_whitespace().last()?;
        p.rsplit('/').next().map(str::to_string)
    })
}

/// Carte de rendu de la session Hyprland en cours (adresse PCI).
pub fn hyprland_current() -> Option<String> {
    let sig = std::env::var("HYPRLAND_INSTANCE_SIGNATURE").ok()?;
    let run = std::env::var("XDG_RUNTIME_DIR").ok()?;
    let log = std::fs::read_to_string(Path::new(&run).join("hypr").join(sig).join("hyprland.log"))
        .ok()?;
    slot_of_node(&renderer_node_in_log(&log)?)
}

/// Bloc Lua de `env.lua` : retrouve les cartes par leur adresse PCI au
/// démarrage de Hyprland et pose `AQ_DRM_DEVICES`, la carte choisie en tête.
pub fn env_lua(slot: &str) -> String {
    let slot = hyprpanel_core::ipc::lua_escape(slot);
    format!(
        r#"do
  local want, first, rest = "{slot}", nil, {{}}
  for n = 0, 31 do
    local f = io.open("/sys/class/drm/card" .. n .. "/device/uevent")
    if f then
      local u = f:read("a") or ""
      f:close()
      local dev = "/dev/dri/card" .. n
      if u:find("PCI_SLOT_NAME=" .. want, 1, true) then first = dev else rest[#rest + 1] = dev end
    end
  end
  if first then
    table.insert(rest, 1, first)
    hl.env("AQ_DRM_DEVICES", table.concat(rest, ":"))
  end
end
"#
    )
}

/// Adresse PCI bien formée (`0000:01:00.0`).
pub fn valid_slot(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 12
        && b[4] == b':'
        && b[7] == b':'
        && b[10] == b'.'
        && s.chars()
            .enumerate()
            .all(|(i, c)| matches!(i, 4 | 7 | 10) || c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uevent_et_journal() {
        let u = "DRIVER=nvidia\nPCI_CLASS=30000\nPCI_ID=10DE:2488\nPCI_SLOT_NAME=0000:01:00.0\n";
        assert_eq!(
            parse_uevent(u),
            Some(("0000:01:00.0".into(), 0x10de, 0x2488, "nvidia".into()))
        );
        let log = "DEBUG from aquamarine ]: CDRMRenderer(drm): Using device /dev/dri/card1\n\
                   DEBUG from aquamarine ]: Creating CDRMRenderer on gpu /dev/dri/renderD128\n";
        assert_eq!(renderer_node_in_log(log), Some("renderD128".into()));
        assert_eq!(renderer_node_in_log("rien"), None);
    }

    #[test]
    fn cartes_lues_dans_sys() {
        let d = std::env::temp_dir().join(format!("hyprdesk-gpus-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        for (n, u) in [
            (
                "card0",
                "DRIVER=nvidia\nPCI_ID=10DE:2488\nPCI_SLOT_NAME=0000:01:00.0\n",
            ),
            (
                "card1",
                "DRIVER=i915\nPCI_ID=8086:A780\nPCI_SLOT_NAME=0000:00:02.0\n",
            ),
        ] {
            std::fs::create_dir_all(d.join(n).join("device")).unwrap();
            std::fs::write(d.join(n).join("device/uevent"), u).unwrap();
        }
        for (o, s) in [
            ("card0-DP-2", "connected"),
            ("card1-HDMI-A-1", "disconnected"),
        ] {
            std::fs::create_dir_all(d.join(o)).unwrap();
            std::fs::write(d.join(o).join("status"), s).unwrap();
        }
        let c = cards_in(&d);
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].outputs, vec!["DP-2".to_string()]);
        assert!(c[1].outputs.is_empty());
        assert_eq!(c[0].pci_id(), "10de:2488");
        assert!(c[1].name.starts_with("Intel"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn adresse_pci() {
        assert!(valid_slot("0000:01:00.0"));
        assert!(!valid_slot("01:00.0"));
        assert!(!valid_slot("0000:01:00.0\"; os.exit() --"));
    }

    #[test]
    fn bloc_lua() {
        let l = env_lua("0000:01:00.0");
        assert!(l.contains(r#"local want, first, rest = "0000:01:00.0""#));
        assert!(l.contains(r#"hl.env("AQ_DRM_DEVICES", table.concat(rest, ":"))"#));
        // Le bloc est du Lua valide.
        mlua::Lua::new().load(&l).into_function().unwrap();
    }
}
