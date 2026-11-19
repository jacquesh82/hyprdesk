//! Écran découpé en zones (MLG-172) : 1, 1×2, 1×3, 2×2, 3×3.
//!
//! Wayland ne sait pas découper une sortie en moniteurs logiques ; hyprdesk
//! s'en approche de deux façons, sur la même géométrie :
//! - le Panorama dessine ses widgets dans les zones de sa surface ;
//! - un écran ordinaire reçoit un layout Lua de Hyprland
//!   (`lua:hyprdesk-zones-<nom>`, voir `assets/zones.lua`) : chaque fenêtre
//!   garde sa zone, et maximiser ou mettre en plein écran ne couvre que sa
//!   zone (l'application se croit en plein écran).
//!
//! En 1×2 et 1×3, la zone 1 prend 1/2 ou 1/3 du grand côté ; le reste est
//! partagé à parts égales. Un écran portrait empile les zones.

/// Découpage de base.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Grid {
    #[default]
    One,
    Cols2,
    Cols3,
    Grid2,
    Grid3,
}

/// Part de la zone 1 en 1×2 et 1×3.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum First {
    Half,
    Third,
}

impl First {
    pub fn id(self) -> &'static str {
        match self {
            First::Half => "1/2",
            First::Third => "1/3",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim() {
            "1/2" | "half" | "0.5" => Some(First::Half),
            "1/3" | "third" => Some(First::Third),
            _ => None,
        }
    }
}

/// Découpage d'un écran : grille et part de la zone 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Split {
    pub grid: Grid,
    /// `None` : part par défaut de la grille (1/2 en 1×2, 1/3 en 1×3).
    pub first: Option<First>,
}

impl Grid {
    pub const ALL: [Grid; 5] = [
        Grid::One,
        Grid::Cols2,
        Grid::Cols3,
        Grid::Grid2,
        Grid::Grid3,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Grid::One => "1",
            Grid::Cols2 => "1x2",
            Grid::Cols3 => "1x3",
            Grid::Grid2 => "2x2",
            Grid::Grid3 => "3x3",
        }
    }

    /// Libellé court (boutons) : 1, 1×2…
    pub fn short(self) -> &'static str {
        match self {
            Grid::One => "1",
            Grid::Cols2 => "1×2",
            Grid::Cols3 => "1×3",
            Grid::Grid2 => "2×2",
            Grid::Grid3 => "3×3",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim().to_lowercase().replace('×', "x").as_str() {
            "1" | "1x1" | "one" | "single" => Grid::One,
            "1x2" | "2" | "split2" => Grid::Cols2,
            "1x3" | "3" | "split3" => Grid::Cols3,
            "2x2" | "4" | "grid4" => Grid::Grid2,
            "3x3" | "9" | "grid9" => Grid::Grid3,
            _ => return None,
        })
    }

    pub fn count(self) -> usize {
        match self {
            Grid::One => 1,
            Grid::Cols2 => 2,
            Grid::Cols3 => 3,
            Grid::Grid2 => 4,
            Grid::Grid3 => 9,
        }
    }

    /// La part de la zone 1 se choisit (1×2 et 1×3).
    pub fn has_first(self) -> bool {
        matches!(self, Grid::Cols2 | Grid::Cols3)
    }

    fn default_first(self) -> First {
        match self {
            Grid::Cols3 => First::Third,
            _ => First::Half,
        }
    }
}

impl Split {
    pub const fn new(grid: Grid) -> Self {
        Split { grid, first: None }
    }

    /// Part effective de la zone 1.
    pub fn first(&self) -> First {
        self.first.unwrap_or(self.grid.default_first())
    }

    /// Identifiant de config : `1`, `1x2`, `1x2:1/3`, `1x3`, `1x3:1/2`, `2x2`, `3x3`
    /// (la part n'est écrite que si elle diffère du défaut de la grille).
    pub fn id(&self) -> String {
        if self.grid.has_first() && self.first() != self.grid.default_first() {
            format!("{}:{}", self.grid.id(), self.first().id())
        } else {
            self.grid.id().to_string()
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        let (g, f) = match s.split_once(':') {
            Some((g, f)) => (g, Some(f)),
            None => (s, None),
        };
        let grid = Grid::parse(g)?;
        let first = match f {
            Some(f) if grid.has_first() => Some(First::parse(f)?),
            Some(_) => return None,
            None => None,
        };
        let mut s = Split { grid, first };
        // Une part égale au défaut n'est pas retenue (id stable).
        if s.first == Some(grid.default_first()) {
            s.first = None;
        }
        Some(s)
    }

    pub fn count(&self) -> usize {
        self.grid.count()
    }

    /// Libellé : « 1×2 (1/3) ».
    pub fn label(&self) -> String {
        if self.grid.has_first() {
            format!("{} ({})", self.grid.short(), self.first().id())
        } else {
            self.grid.short().to_string()
        }
    }

    /// Parts des colonnes et des lignes (écran paysage).
    pub fn fractions(&self) -> (Vec<f32>, Vec<f32>) {
        let first = match self.first() {
            First::Half => 0.5,
            First::Third => 1.0 / 3.0,
        };
        match self.grid {
            Grid::One => (vec![1.0], vec![1.0]),
            Grid::Cols2 => (vec![first, 1.0 - first], vec![1.0]),
            Grid::Cols3 => {
                let r = (1.0 - first) / 2.0;
                (vec![first, r, r], vec![1.0])
            }
            Grid::Grid2 => (vec![0.5; 2], vec![0.5; 2]),
            Grid::Grid3 => (vec![1.0 / 3.0; 3], vec![1.0 / 3.0; 3]),
        }
    }

    /// Zones d'une surface `w`×`h` (ligne par ligne), séparées de `gap` ;
    /// en portrait, colonnes et lignes s'échangent.
    pub fn rects(&self, w: f32, h: f32, gap: f32) -> Vec<[f32; 4]> {
        let (mut cols, mut rows) = self.fractions();
        if h > w {
            std::mem::swap(&mut cols, &mut rows);
        }
        let xs = spans(w, &cols, gap);
        let ys = spans(h, &rows, gap);
        ys.iter()
            .flat_map(|&(y, ch)| xs.iter().map(move |&(x, cw)| [x, y, cw, ch]))
            .collect()
    }

    /// Nom du layout Lua (sans préfixe) : `1x2`, `1x2t`, `1x3`, `1x3h`, `2x2`, `3x3`.
    pub fn slug(&self) -> String {
        let mut s = self.grid.id().to_string();
        if self.grid.has_first() && self.first() != self.grid.default_first() {
            s.push(match self.first() {
                First::Half => 'h',
                First::Third => 't',
            });
        }
        s
    }

    /// Layout pour Hyprland ; `None` pour un écran non découpé.
    pub fn hypr_layout(&self) -> Option<String> {
        (self.grid != Grid::One).then(|| format!("lua:hyprdesk-zones-{}", self.slug()))
    }

    /// Lua qui définit le layout (après [`runtime_lua`]).
    pub fn define_lua(&self) -> String {
        let (c, r) = self.fractions();
        let list = |v: &[f32]| {
            v.iter()
                .map(|f| format!("{f:.6}"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        format!(
            "__hyprdesk_zones.define(\"{}\", {{ {} }}, {{ {} }})\n",
            self.slug(),
            list(&c),
            list(&r)
        )
    }
}

/// Découpe `len` selon `fr`, en retirant les espaces `gap` : (début, longueur).
fn spans(len: f32, fr: &[f32], gap: f32) -> Vec<(f32, f32)> {
    let n = fr.len().max(1);
    let usable = (len - gap * (n - 1) as f32).max(n as f32);
    let mut out = Vec::with_capacity(n);
    let mut at = 0.0;
    for (i, f) in fr.iter().enumerate() {
        let l = if i + 1 == n {
            (len - at).max(1.0)
        } else {
            (usable * f).round().max(1.0)
        };
        out.push((at, l));
        at += l + gap;
    }
    out
}

/// Runtime Lua commun (layouts de zones, plein écran confiné) ; rejouable.
pub fn runtime_lua() -> &'static str {
    include_str!("../assets/zones.lua")
}

/// Règle de workspace de l'écran `monitor`, nom de layout compris.
pub fn rule_selector(monitor: &str) -> String {
    format!("m[{monitor}]")
}

/// Lua des écrans découpés, pour `layouts.lua` comme à chaud : runtime,
/// layouts employés, une règle par écran (gardée dans `__hyprdesk_zones.rules`
/// pour être retirée ou réaffirmée quand un workspace change d'écran).
pub fn screens_lua(screens: &std::collections::BTreeMap<String, Split>) -> String {
    let used: Vec<(&String, &Split)> = screens
        .iter()
        .filter(|(_, s)| s.grid != Grid::One)
        .collect();
    let mut lua = String::new();
    lua += runtime_lua();
    lua += "\nif __hyprdesk_zones.rules then for _, r in ipairs(__hyprdesk_zones.rules) do \
            pcall(function() r:set_enabled(false) end) end end\n\
            __hyprdesk_zones.rules = {}\n";
    let mut defined: Vec<String> = Vec::new();
    for (_, s) in &used {
        if !defined.contains(&s.slug()) {
            defined.push(s.slug());
            lua += &s.define_lua();
        }
    }
    for (mon, s) in &used {
        lua += &format!(
            "table.insert(__hyprdesk_zones.rules, hl.workspace_rule({{ workspace = \"{}\", layout = \"{}\" }}))\n",
            hyprpanel_core::ipc::lua_escape(&rule_selector(mon)),
            s.hypr_layout().unwrap_or_default()
        );
    }
    lua
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiants() {
        for s in ["1", "1x2", "1x2:1/3", "1x3", "1x3:1/2", "2x2", "3x3"] {
            assert_eq!(Split::parse(s).unwrap().id(), s);
        }
        // Part égale au défaut : non retenue.
        assert_eq!(Split::parse("1x2:1/2").unwrap().id(), "1x2");
        assert_eq!(Split::parse("1×3").unwrap().grid, Grid::Cols3);
        assert!(Split::parse("2x2:1/3").is_none());
        assert!(Split::parse("4x4").is_none());
        assert_eq!(Split::parse("1x3:1/2").unwrap().slug(), "1x3h");
        assert_eq!(Split::parse("1x2:1/3").unwrap().slug(), "1x2t");
        assert_eq!(Split::new(Grid::One).hypr_layout(), None);
        assert_eq!(
            Split::new(Grid::Grid2).hypr_layout().as_deref(),
            Some("lua:hyprdesk-zones-2x2")
        );
    }

    #[test]
    fn geometrie() {
        let close = |a: f32, b: f32| (a - b).abs() < 1.0;
        // 1×2 au tiers sur 3440×1440.
        let r = Split::parse("1x2:1/3").unwrap().rects(3440.0, 1440.0, 0.0);
        assert_eq!(r.len(), 2);
        assert!(close(r[0][2], 3440.0 / 3.0) && close(r[1][0], r[0][2]));
        assert!(close(r[1][0] + r[1][2], 3440.0));
        // 1×3 à la moitié : 1/2, 1/4, 1/4.
        let r = Split::parse("1x3:1/2").unwrap().rects(2560.0, 720.0, 0.0);
        assert!(close(r[0][2], 1280.0) && close(r[1][2], 640.0) && close(r[2][2], 640.0));
        // 3×3 avec espaces : neuf zones dans la surface.
        let r = Split::new(Grid::Grid3).rects(2560.0, 1440.0, 12.0);
        assert_eq!(r.len(), 9);
        for z in &r {
            assert!(z[0] >= 0.0 && z[1] >= 0.0 && z[0] + z[2] <= 2560.5 && z[1] + z[3] <= 1440.5);
        }
        assert!(close(r[8][0] + r[8][2], 2560.0) && close(r[8][1] + r[8][3], 1440.0));
        // Portrait : 1×2 empilé.
        let r = Split::new(Grid::Cols2).rects(720.0, 2560.0, 0.0);
        assert_eq!((r[1][0], r[1][1], r[1][2]), (0.0, 1280.0, 720.0));
    }

    #[test]
    fn lua_des_ecrans() {
        let mut m = std::collections::BTreeMap::new();
        m.insert("DP-2".to_string(), Split::parse("1x3:1/2").unwrap());
        m.insert("DP-3".to_string(), Split::new(Grid::One));
        let lua = screens_lua(&m);
        assert!(lua.contains(
            "__hyprdesk_zones.define(\"1x3h\", { 0.500000, 0.250000, 0.250000 }, { 1.000000 })"
        ));
        assert!(lua.contains("workspace = \"m[DP-2]\", layout = \"lua:hyprdesk-zones-1x3h\""));
        assert!(!lua.contains("m[DP-3]"));
    }
}
