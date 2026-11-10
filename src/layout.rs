//! Disposition des icônes : placement sur la grille, et positions déplacées à
//! la souris, mémorisées dans `$XDG_STATE_HOME/hyprdesk/layout.toml`. Le Lua
//! de l'utilisateur n'est jamais réécrit.

use std::collections::{BTreeMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub type Cell = (u16, u16);

/// Dimensions de grille pour une surface : (colonnes, lignes).
pub fn grid_dims(width: f32, height: f32, cell: u16, gap: u16, margin: u16) -> (u16, u16) {
    let fit = |len: f32| -> u16 {
        let usable = len - 2.0 * margin as f32 + gap as f32;
        (usable / (cell + gap) as f32).floor().max(1.0) as u16
    };
    (fit(width), fit(height))
}

/// Place les icônes : position mémorisée, sinon imposée par la config, sinon
/// première cellule libre en remplissant colonne par colonne (comme un
/// bureau classique). Rend `None` pour une icône qui ne tient plus.
pub fn place(wanted: &[Option<Cell>], cols: u16, rows: u16) -> Vec<Option<Cell>> {
    let inside = |c: Cell| c.0 < cols && c.1 < rows;
    let mut taken: HashSet<Cell> = HashSet::new();
    let mut out: Vec<Option<Cell>> = vec![None; wanted.len()];
    // D'abord les positions explicites, dans l'ordre de déclaration.
    for (i, w) in wanted.iter().enumerate() {
        if let Some(c) = w.filter(|c| inside(*c))
            && taken.insert(c)
        {
            out[i] = Some(c);
        }
    }
    let mut free = (0..cols).flat_map(|c| (0..rows).map(move |r| (c, r)));
    for slot in out.iter_mut().filter(|s| s.is_none()) {
        *slot = free.find(|c| !taken.contains(c));
        if let Some(c) = *slot {
            taken.insert(c);
        }
    }
    out
}

/// Cellule sous un point (coordonnées de surface), si dans la grille.
pub fn cell_at(
    x: f32,
    y: f32,
    cell: u16,
    gap: u16,
    margin: u16,
    cols: u16,
    rows: u16,
) -> Option<Cell> {
    let step = (cell + gap) as f32;
    let (cx, cy) = ((x - margin as f32) / step, (y - margin as f32) / step);
    if cx < 0.0 || cy < 0.0 {
        return None;
    }
    let c = (cx.floor() as u16, cy.floor() as u16);
    (c.0 < cols && c.1 < rows).then_some(c)
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Layout {
    /// Clé `écran|workspace|id d'icône` → [colonne, ligne].
    #[serde(default)]
    positions: BTreeMap<String, [u16; 2]>,
}

pub fn key(monitor: &str, workspace: &str, icon: &str) -> String {
    format!("{monitor}|{workspace}|{icon}")
}

fn path() -> PathBuf {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state")
        });
    base.join("hyprdesk").join("layout.toml")
}

impl Layout {
    pub fn load() -> Self {
        let p = path();
        match std::fs::read_to_string(&p) {
            Ok(s) => toml::from_str(&s).unwrap_or_else(|e| {
                tracing::warn!(
                    "{}",
                    crate::tr!(
                        "{p0} illisible, disposition ignorée : {e}",
                        p0 = p.display(),
                        e = e
                    )
                );
                Layout::default()
            }),
            Err(_) => Layout::default(),
        }
    }

    pub fn save(&self) {
        let p = path();
        let write = || -> anyhow::Result<()> {
            if let Some(d) = p.parent() {
                std::fs::create_dir_all(d)?;
            }
            crate::storage::write(
                &p,
                toml::to_string_pretty(self)?.as_bytes(),
                crate::storage::revision(),
            )?;
            Ok(())
        };
        if let Err(e) = write() {
            tracing::warn!(
                "{}",
                crate::tr!("enregistrement de {p0} : {e}", p0 = p.display(), e = e)
            );
        }
    }

    pub fn get(&self, key: &str) -> Option<Cell> {
        self.positions.get(key).map(|[c, r]| (*c, *r))
    }

    /// Oublie les positions mémorisées d'un bureau (écran + workspace).
    pub fn clear_desktop(&mut self, monitor: &str, workspace: &str) {
        let prefix = format!("{monitor}|{workspace}|");
        self.positions.retain(|k, _| !k.starts_with(&prefix));
    }

    pub fn set(&mut self, key: String, c: Cell) {
        self.positions.insert(key, [c.0, c.1]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimensions() {
        // 2560x1395 (sous la barre), cellules 96 + 16, marge 32.
        assert_eq!(grid_dims(2560.0, 1395.0, 96, 16, 32), (22, 12));
        assert_eq!(grid_dims(10.0, 10.0, 96, 16, 32), (1, 1));
    }

    #[test]
    fn placement_colonne_par_colonne_et_explicite() {
        let p = place(
            &[None, Some((0, 0)), None, Some((5, 5)), Some((9, 9))],
            3,
            2,
        );
        // (0,0) est réservée par la 2e icône ; la 1re prend (0,1), la 3e (1,0).
        assert_eq!(
            p,
            vec![
                Some((0, 1)),
                Some((0, 0)),
                Some((1, 0)),
                Some((1, 1)),
                Some((2, 0))
            ]
        );
    }

    #[test]
    fn conflit_et_debordement() {
        let p = place(&[Some((0, 0)), Some((0, 0)), None], 1, 2);
        assert_eq!(p, vec![Some((0, 0)), Some((0, 1)), None]);
    }

    #[test]
    fn cellule_sous_le_curseur() {
        assert_eq!(cell_at(40.0, 40.0, 96, 16, 32, 5, 5), Some((0, 0)));
        assert_eq!(
            cell_at(
                32.0 + 112.0 * 2.0 + 5.0,
                32.0 + 112.0 + 5.0,
                96,
                16,
                32,
                5,
                5
            ),
            Some((2, 1))
        );
        assert_eq!(cell_at(10.0, 40.0, 96, 16, 32, 5, 5), None);
        assert_eq!(cell_at(5000.0, 40.0, 96, 16, 32, 5, 5), None);
    }

    #[test]
    fn toml_aller_retour() {
        let mut l = Layout::default();
        l.set(key("DP-1", "6", "app:kitty"), (2, 3));
        let s = toml::to_string_pretty(&l).unwrap();
        let back: Layout = toml::from_str(&s).unwrap();
        assert_eq!(back.get("DP-1|6|app:kitty"), Some((2, 3)));
    }
}
