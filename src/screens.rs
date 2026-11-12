//! Écrans : détection, disposition (position, orientation, mode, échelle),
//! écran principal, et `monitors.lua` généré.
//!
//! hyprdesk applique un changement à chaud (`hl.monitor` par l'IPC Lua de
//! Hyprland) ; il n'est écrit dans `monitors.lua` qu'une fois confirmé. Les
//! écrans débranchés gardent leur ligne : on les retrouve en les rebranchant.

use std::path::{Path, PathBuf};

use hyprpanel_core::ipc;
use serde_json::Value;

/// Un écran tel que Hyprland le voit (y compris désactivé).
#[derive(Debug, Clone, PartialEq)]
pub struct Screen {
    /// Connecteur (`DP-1`, `HDMI-A-2`).
    pub name: String,
    /// Fabricant et modèle, pour l'affichage.
    pub description: String,
    pub enabled: bool,
    /// Mode en pixels physiques.
    pub width: u32,
    pub height: u32,
    pub refresh: f32,
    /// Modes proposés (`2560x1440@59.95`).
    pub modes: Vec<String>,
    /// Position dans l'espace global, en pixels logiques.
    pub x: i32,
    pub y: i32,
    pub scale: f32,
    /// 0..=3 : rotation par quarts de tour ; +4 : image retournée.
    pub transform: u8,
    /// Écran recopié, ou vide.
    pub mirror: String,
    pub vrr: bool,
}

/// Côté où se place un écran par rapport à un autre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Right,
    Left,
    Above,
    Below,
}

impl Side {
    pub const ALL: [Side; 4] = [Side::Right, Side::Left, Side::Above, Side::Below];

    pub fn label(self) -> &'static str {
        match self {
            Side::Right => crate::i18n::tr("À droite de"),
            Side::Left => crate::i18n::tr("À gauche de"),
            Side::Above => crate::i18n::tr("Au-dessus de"),
            Side::Below => crate::i18n::tr("En dessous de"),
        }
    }

    fn horizontal(self) -> bool {
        matches!(self, Side::Right | Side::Left)
    }
}

impl std::fmt::Display for Side {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// Alignement le long du bord commun : haut/gauche, centre, bas/droite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Start,
    Center,
    End,
}

impl Align {
    pub fn label(self, side: Side) -> &'static str {
        match (self, side.horizontal()) {
            (Align::Start, true) => crate::i18n::tr("Haut"),
            (Align::Start, false) => crate::i18n::tr("Gauche"),
            (Align::Center, _) => crate::i18n::tr("Centre"),
            (Align::End, true) => crate::i18n::tr("Bas"),
            (Align::End, false) => crate::i18n::tr("Droite"),
        }
    }
}

/// Échelles proposées.
pub const SCALES: [f32; 7] = [1.0, 1.25, 1.333_333_3, 1.5, 1.6, 1.75, 2.0];

impl Screen {
    pub fn mode(&self) -> String {
        format!("{}x{}@{:.2}", self.width, self.height, self.refresh)
    }

    pub fn rotation(&self) -> u16 {
        (self.transform % 4) as u16 * 90
    }

    pub fn flipped(&self) -> bool {
        self.transform >= 4
    }

    pub fn set_orientation(&mut self, rotation: u16, flipped: bool) {
        self.transform = ((rotation / 90) % 4) as u8 + if flipped { 4 } else { 0 };
    }

    /// Taille logique : échelle appliquée, largeur et hauteur échangées en portrait.
    pub fn logical(&self) -> (i32, i32) {
        let s = if self.scale > 0.0 { self.scale } else { 1.0 };
        let (w, h) = (
            (self.width as f32 / s).round() as i32,
            (self.height as f32 / s).round() as i32,
        );
        if self.transform % 2 == 1 {
            (h, w)
        } else {
            (w, h)
        }
    }

    /// Change de mode (`2560x1440@59.95`, `Hz` toléré).
    pub fn set_mode(&mut self, mode: &str) -> bool {
        let Some((w, h, r)) = parse_mode(mode) else {
            return false;
        };
        (self.width, self.height, self.refresh) = (w, h, r);
        true
    }

    /// Ligne `hl.monitor` de cet écran.
    pub fn lua(&self) -> String {
        let q = |s: &str| format!("\"{}\"", ipc::lua_escape(s));
        if !self.enabled {
            return format!(
                "hl.monitor({{ output = {}, disabled = true }})",
                q(&self.name)
            );
        }
        let mut f = vec![
            format!("output = {}", q(&self.name)),
            format!("mode = {}", q(&self.mode())),
            format!("position = \"{}x{}\"", self.x, self.y),
            format!("scale = {}", scale_str(self.scale)),
            format!("transform = {}", self.transform),
        ];
        if !self.mirror.is_empty() {
            f.push(format!("mirror = {}", q(&self.mirror)));
        }
        f.push(format!("vrr = {}", u8::from(self.vrr)));
        // Profil ICC choisi dans Réglages → Couleurs des écrans.
        if let Some(icc) = crate::colors::icc_for(&self.name) {
            f.push(format!("icc = {}", q(&icc)));
        }
        format!("hl.monitor({{ {} }})", f.join(", "))
    }
}

fn scale_str(s: f32) -> String {
    let v = (s * 1000.0).round() / 1000.0;
    if v.fract() == 0.0 {
        format!("{}", v as i32)
    } else {
        format!("{v}")
    }
}

/// `2560x1440@59.95` ou `2560x1440@59.95Hz` → (largeur, hauteur, fréquence).
pub fn parse_mode(s: &str) -> Option<(u32, u32, f32)> {
    let s = s.trim().trim_end_matches("Hz");
    let (size, rate) = s.split_once('@')?;
    let (w, h) = size.split_once('x')?;
    let r: f32 = rate.parse().ok()?;
    Some((w.parse().ok()?, h.parse().ok()?, r))
}

/// Écrans de `j/monitors all`.
pub fn parse(v: &Value) -> Vec<Screen> {
    let Some(list) = v.as_array() else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|m| {
            let name = m["name"].as_str()?.to_string();
            let mut modes: Vec<String> = Vec::new();
            for s in m["availableModes"].as_array().into_iter().flatten() {
                if let Some((w, h, r)) = s.as_str().and_then(parse_mode) {
                    let mode = format!("{w}x{h}@{r:.2}");
                    if !modes.contains(&mode) {
                        modes.push(mode);
                    }
                }
            }
            let mirror = m["mirrorOf"].as_str().unwrap_or("none");
            Some(Screen {
                description: m["description"].as_str().unwrap_or("").trim().to_string(),
                enabled: !m["disabled"].as_bool().unwrap_or(false),
                width: m["width"].as_u64()? as u32,
                height: m["height"].as_u64()? as u32,
                refresh: m["refreshRate"].as_f64().unwrap_or(60.0) as f32,
                modes,
                x: m["x"].as_i64().unwrap_or(0) as i32,
                y: m["y"].as_i64().unwrap_or(0) as i32,
                scale: m["scale"].as_f64().unwrap_or(1.0) as f32,
                transform: (m["transform"].as_u64().unwrap_or(0) % 8) as u8,
                mirror: if mirror == "none" {
                    String::new()
                } else {
                    mirror.to_string()
                },
                vrr: m["vrr"].as_bool().unwrap_or(false),
                name,
            })
        })
        .collect()
}

/// Écrans détectés, désactivés compris.
pub fn detect() -> Result<Vec<Screen>, ipc::IpcError> {
    Ok(parse(&ipc::query_json("monitors all")?))
}

/// Applique une disposition à chaud.
pub fn apply(list: &[Screen]) -> Result<(), String> {
    let lua: Vec<String> = list.iter().map(Screen::lua).collect();
    let r = ipc::eval(&lua.join("\n")).map_err(|e| e.to_string())?;
    if r.trim() != "ok" {
        return Err(r.trim().to_string());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Disposition

type Rect = (i32, i32, i32, i32);

fn rect(s: &Screen) -> Rect {
    let (w, h) = s.logical();
    (s.x, s.y, w, h)
}

/// Où se trouve `s` par rapport à `r` ; `None` s'ils se chevauchent.
pub fn relation(s: &Screen, r: &Screen) -> Option<(Side, Align)> {
    let (sx, sy, sw, sh) = rect(s);
    let (rx, ry, rw, rh) = rect(r);
    let side = if sx >= rx + rw {
        Side::Right
    } else if sx + sw <= rx {
        Side::Left
    } else if sy + sh <= ry {
        Side::Above
    } else if sy >= ry + rh {
        Side::Below
    } else {
        return None;
    };
    let (start, len, rstart, rlen) = if side.horizontal() {
        (sy, sh, ry, rh)
    } else {
        (sx, sw, rx, rw)
    };
    let align = if start == rstart {
        Align::Start
    } else if start + len == rstart + rlen {
        Align::End
    } else {
        Align::Center
    };
    Some((side, align))
}

/// Place `s` contre `r`.
pub fn place(s: &mut Screen, r: &Screen, side: Side, align: Align) {
    let (sw, sh) = s.logical();
    let (rx, ry, rw, rh) = rect(r);
    let along = |len: i32, rstart: i32, rlen: i32| match align {
        Align::Start => rstart,
        Align::Center => rstart + (rlen - len) / 2,
        Align::End => rstart + rlen - len,
    };
    (s.x, s.y) = match side {
        Side::Right => (rx + rw, along(sh, ry, rh)),
        Side::Left => (rx - sw, along(sh, ry, rh)),
        Side::Above => (along(sw, rx, rw), ry - sh),
        Side::Below => (along(sw, rx, rw), ry + rh),
    };
}

/// Écart entre deux rectangles (0 s'ils se touchent ou se chevauchent).
pub fn gap(a: &Screen, b: &Screen) -> i64 {
    let (ax, ay, aw, ah) = rect(a);
    let (bx, by, bw, bh) = rect(b);
    let dx = (bx - (ax + aw)).max(ax - (bx + bw)).max(0) as i64;
    let dy = (by - (ay + ah)).max(ay - (by + bh)).max(0) as i64;
    dx * dx + dy * dy
}

/// Lien d'un écran à celui contre lequel il est placé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    pub screen: usize,
    pub reference: usize,
    pub side: Side,
    pub align: Align,
}

/// Écran d'ancrage : le principal s'il est actif, sinon le plus en haut à gauche.
pub fn anchor(list: &[Screen], primary: Option<&str>) -> Option<usize> {
    list.iter()
        .position(|s| s.enabled && Some(s.name.as_str()) == primary)
        .or_else(|| {
            list.iter()
                .enumerate()
                .filter(|(_, s)| s.enabled)
                .min_by_key(|(_, s)| (s.x, s.y))
                .map(|(i, _)| i)
        })
}

/// Chaque écran actif relié, de proche en proche, à l'ancre : la disposition
/// se rejoue ainsi après un changement de taille (rotation, mode, échelle).
pub fn links(list: &[Screen], anchor: usize) -> Vec<Link> {
    let mut placed = vec![anchor];
    let mut out = Vec::new();
    loop {
        let next = list
            .iter()
            .enumerate()
            .filter(|(i, s)| s.enabled && !placed.contains(i))
            .flat_map(|(i, s)| placed.iter().map(move |&p| (i, p, s)))
            .min_by_key(|(i, p, s)| (gap(s, &list[*p]), *p != anchor, *i));
        let Some((i, p, s)) = next else { break };
        let (side, align) = relation(s, &list[p]).unwrap_or((Side::Right, Align::Start));
        out.push(Link {
            screen: i,
            reference: p,
            side,
            align,
        });
        placed.push(i);
    }
    out
}

/// Rejoue les liens dans l'ordre (chaque référence est déjà placée).
pub fn replay(list: &mut [Screen], links: &[Link]) {
    for l in links {
        let r = list[l.reference].clone();
        place(&mut list[l.screen], &r, l.side, l.align);
    }
}

/// Décale tout pour que l'ancre (l'écran principal) soit en 0×0.
pub fn normalize(list: &mut [Screen], primary: Option<&str>) {
    let Some(a) = anchor(list, primary) else {
        return;
    };
    let (dx, dy) = (list[a].x, list[a].y);
    for s in list.iter_mut() {
        s.x -= dx;
        s.y -= dy;
    }
}

/// Disposition automatique : de gauche à droite, le principal en premier,
/// centrés verticalement.
pub fn auto_arrange(list: &mut [Screen], primary: Option<&str>) {
    let mut order: Vec<usize> = (0..list.len()).filter(|&i| list[i].enabled).collect();
    order.sort_by_key(|&i| (Some(list[i].name.as_str()) != primary, list[i].x, list[i].y));
    let Some(&first) = order.first() else { return };
    (list[first].x, list[first].y) = (0, 0);
    for w in order.windows(2) {
        let r = list[w[0]].clone();
        place(&mut list[w[1]], &r, Side::Right, Align::Center);
    }
    normalize(list, primary);
}

/// Deux écrans actifs se chevauchent (disposition à revoir).
pub fn overlapping(list: &[Screen]) -> bool {
    let on: Vec<&Screen> = list
        .iter()
        .filter(|s| s.enabled && s.mirror.is_empty())
        .collect();
    on.iter().enumerate().any(|(i, a)| {
        on[i + 1..].iter().any(|b| {
            let (ax, ay, aw, ah) = rect(a);
            let (bx, by, bw, bh) = rect(b);
            ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah
        })
    })
}

// ---------------------------------------------------------------------------
// monitors.lua

/// `monitors.lua`, à côté de `hyprdesk.lua` (requis par `hyprland.lua`).
pub fn monitors_path(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("monitors.lua")
}

/// Connecteur visé par une ligne `hl.monitor({ output = "…", … })`.
fn line_output(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix("hl.monitor(")?;
    let after = &rest[rest.find("output")?..];
    let start = after.find('"')? + 1;
    let end = start + after[start..].find('"')?;
    Some(&after[start..end])
}

/// Contenu de `monitors.lua` : les écrans connus, ceux d'avant qui ne sont pas
/// branchés, et une règle d'accueil pour tout nouvel écran.
pub fn render(list: &[Screen], previous: &str) -> String {
    let mut o = String::from(
        "-- Généré par hyprdesk (Réglages → Écrans) — NE PAS ÉDITER : réécrit à chaque enregistrement.\n\
         -- Requis par hyprland.lua : require(\"monitors\").\n\n",
    );
    for s in list {
        o += &s.lua();
        o.push('\n');
    }
    let kept: Vec<&str> = previous
        .lines()
        .filter(|l| {
            line_output(l).is_some_and(|out| !out.is_empty() && !list.iter().any(|s| s.name == out))
        })
        .collect();
    if !kept.is_empty() {
        o += "\n-- Écrans non branchés au dernier enregistrement.\n";
        for l in kept {
            o += l.trim();
            o.push('\n');
        }
    }
    o += "\n-- Nouvel écran : mode préféré, placé automatiquement.\n\
          hl.monitor({ output = \"\", mode = \"preferred\", position = \"auto\", scale = \"auto\" })\n";
    // Commentaires dans la langue de l'interface (le code Lua n'est pas au catalogue).
    crate::i18n::tr_lines(&o) + "\n"
}

/// Fichier `monitors.lua` écrit par hyprdesk (en-tête français ou anglais).
fn ours(content: &str) -> bool {
    content.starts_with("-- Généré par hyprdesk") || content.starts_with("-- Generated by hyprdesk")
}

/// Enregistre la disposition. Un `monitors.lua` d'un autre outil est gardé
/// une fois en `.bak`.
pub fn save(path: &Path, list: &[Screen]) -> std::io::Result<()> {
    let previous = std::fs::read_to_string(path).unwrap_or_default();
    if !previous.is_empty() && !ours(&previous) {
        let bak = path.with_extension("lua.bak");
        if !bak.exists() {
            std::fs::write(&bak, &previous)?;
        }
    }
    crate::storage::write(
        path,
        render(list, &previous).as_bytes(),
        crate::storage::revision(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chez l'utilisateur : HDMI-A-2 paysage en 0×0, DP-1 portrait à droite, centré.
    fn bureau() -> Vec<Screen> {
        let json = serde_json::json!([
            { "name": "HDMI-A-2", "description": "ROG PG279Q", "width": 2560, "height": 1440,
              "refreshRate": 59.951, "x": 0, "y": 0, "scale": 1.0, "transform": 0,
              "disabled": false, "mirrorOf": "none", "vrr": false,
              "availableModes": ["2560x1440@59.95Hz", "1920x1080@60.00Hz", "1920x1080@60.00Hz"] },
            { "name": "DP-1", "description": "PX7 Prime", "width": 2560, "height": 1440,
              "refreshRate": 59.951, "x": 2560, "y": -560, "scale": 1.0, "transform": 1,
              "disabled": false, "mirrorOf": "none", "vrr": false,
              "availableModes": ["2560x1440@59.95Hz", "2560x1440@165.00Hz"] }
        ]);
        parse(&json)
    }

    #[test]
    fn detection() {
        let l = bureau();
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].modes, ["2560x1440@59.95", "1920x1080@60.00"]);
        assert_eq!(l[0].mode(), "2560x1440@59.95");
        assert_eq!(l[1].logical(), (1440, 2560));
        assert_eq!((l[1].rotation(), l[1].flipped()), (90, false));
        assert_eq!(
            l[1].lua(),
            r#"hl.monitor({ output = "DP-1", mode = "2560x1440@59.95", position = "2560x-560", scale = 1, transform = 1, vrr = 0 })"#
        );
    }

    #[test]
    fn relations_et_placement() {
        let l = bureau();
        assert_eq!(relation(&l[1], &l[0]), Some((Side::Right, Align::Center)));
        assert_eq!(relation(&l[0], &l[1]), Some((Side::Left, Align::Center)));
        let mut dp = l[1].clone();
        place(&mut dp, &l[0], Side::Left, Align::Start);
        assert_eq!((dp.x, dp.y), (-1440, 0));
        place(&mut dp, &l[0], Side::Below, Align::End);
        assert_eq!((dp.x, dp.y), (2560 - 1440, 1440));
    }

    #[test]
    fn rotation_rejouee_contre_le_voisin() {
        let mut l = bureau();
        let a = anchor(&l, Some("HDMI-A-2")).unwrap();
        let links = links(&l, a);
        assert_eq!(links.len(), 1);
        // DP-1 repasse en paysage : il reste à droite, centré.
        l[1].set_orientation(0, false);
        replay(&mut l, &links);
        assert_eq!((l[1].x, l[1].y), (2560, 0));
        // HDMI-A-2 passe en portrait : DP-1 le suit.
        l[0].set_orientation(90, false);
        replay(&mut l, &links);
        assert_eq!(l[1].x, 1440);
        assert!(!overlapping(&l));
    }

    #[test]
    fn principal_en_origine_et_disposition_auto() {
        let mut l = bureau();
        normalize(&mut l, Some("DP-1"));
        assert_eq!((l[1].x, l[1].y), (0, 0));
        assert_eq!((l[0].x, l[0].y), (-2560, 560));
        auto_arrange(&mut l, Some("DP-1"));
        assert_eq!((l[1].x, l[1].y), (0, 0));
        assert_eq!(l[0].x, 1440);
        assert_eq!(l[0].y, (2560 - 1440) / 2);
        assert!(!overlapping(&l));
        l[0].x = 100;
        assert!(overlapping(&l));
    }

    #[test]
    fn monitors_lua_garde_les_ecrans_debranches() {
        let previous = r#"-- Generated by hyprdmc
hl.monitor({ output = "HDMI-A-2", mode = "2560x1440@59.95", position = "0x0", scale = 1, transform = 0 })
hl.monitor({ output = "eDP-1", mode = "1920x1200@60", position = "-1920x0", scale = 1.25, transform = 0 })
hl.monitor({ output = "", mode = "preferred", position = "auto", scale = "auto" })
"#;
        let mut l = bureau();
        l[0].enabled = false;
        let out = render(&l, previous);
        assert!(out.starts_with("-- Généré par hyprdesk"));
        assert!(out.contains(r#"hl.monitor({ output = "HDMI-A-2", disabled = true })"#));
        assert!(out.contains(r#"output = "eDP-1""#));
        assert_eq!(out.matches(r#"output = """#).count(), 1);
        assert_eq!(out.matches("HDMI-A-2").count(), 1);
        assert_eq!(
            line_output(r#"  hl.monitor({ output = "DP-1" })"#),
            Some("DP-1")
        );
        assert_eq!(scale_str(1.333_333_3), "1.333");
    }
}
