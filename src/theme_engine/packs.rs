//! Découverte et chaîne d'héritage des packs (§3 et §6 de la spécification) :
//! dossiers utilisateur puis système puis packs embarqués (l'utilisateur
//! gagne), chargement d'un pack par identifiant, linéarisation de l'héritage
//! (profondeur bornée, cycles détectés, parents manquants en avertissement).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::manifest::Manifest;
use super::{Error, Result, paths::Roots};

/// Profondeur maximale d'héritage (§14) : 4 niveaux de parents.
pub const MAX_DEPTH: usize = 4;

/// Nom du fichier manifeste d'un pack.
pub const MANIFEST_FILE: &str = "theme.toml";

/// Provenance d'un pack : priorité à l'utilisateur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Origin {
    /// Ressource du binaire (`include_str!`).
    Bundled,
    /// `$XDG_DATA_DIRS/hyprdesk/themes/<id>/`.
    System,
    /// `~/.local/share/hyprdesk/themes/<id>/`.
    User,
}

impl Origin {
    /// Nom court affiché par la CLI.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bundled => "bundled",
            Self::System => "system",
            Self::User => "user",
        }
    }
}

/// Un pack chargé : manifeste validé, dossier (absent s'il est embarqué).
#[derive(Debug, Clone)]
pub struct Pack {
    pub origin: Origin,
    /// Dossier du pack (`None` : embarqué, ressources du binaire).
    pub dir: Option<PathBuf>,
    pub manifest: Manifest,
}

/// Lit un pack depuis un dossier (`theme.toml` à la racine).
pub fn load_dir(dir: &Path, origin: Origin) -> Result<Pack> {
    let id = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let path = dir.join(MANIFEST_FILE);
    let text = std::fs::read_to_string(&path)?;
    let (manifest, _) = Manifest::parse(&id, &text)?;
    Ok(Pack {
        origin,
        dir: Some(dir.to_path_buf()),
        manifest,
    })
}

/// Découverte : tous les packs visibles, triés par identifiant. Les dossiers
/// invalides n'arrêtent pas la découverte, ils produisent des avertissements.
pub fn discover(roots: &Roots, bundled: &[Pack]) -> Result<(Vec<Pack>, Vec<String>)> {
    let mut warnings = Vec::new();
    let mut by_id: BTreeMap<String, Pack> = BTreeMap::new();
    // Priorité croissante : embarqué, système, utilisateur (BTreeMap tenu à jour
    // en écrasant à chaque passe plus prioritaire).
    for pack in bundled {
        by_id.insert(pack.manifest.meta.id.clone(), pack.clone());
    }
    for (origin, dirs) in [
        (Origin::System, roots.system_themes_dirs()),
        (Origin::User, vec![roots.themes_dir()]),
    ] {
        for dir in dirs {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() || !path.join(MANIFEST_FILE).exists() {
                    continue;
                }
                match load_dir(&path, origin) {
                    Ok(pack) => {
                        by_id.insert(pack.manifest.meta.id.clone(), pack);
                    }
                    Err(e) => warnings.push(format!(
                        "pack ignoré {} : {e}",
                        path.file_name().unwrap_or_default().to_string_lossy()
                    )),
                }
            }
        }
    }
    Ok((by_id.into_values().collect(), warnings))
}

/// Pack par identifiant dans une liste découverte.
pub fn find<'a>(packs: &'a [Pack], id: &str) -> Option<&'a Pack> {
    packs.iter().find(|p| p.manifest.meta.id == id)
}

/// Chaîne d'héritage linéarisée, ancêtres d'abord, le pack lui-même en
/// dernier : `[inherits] themes = ["a", "b"]` donne `chain(a)`, `chain(b)`,
/// puis le pack (le dernier de la liste gagne pour chaque slot ou gabarit).
/// Un losange est linéarisé une seule fois ; un cycle est une erreur.
pub fn chain(packs: &[Pack], id: &str) -> Result<Vec<Pack>> {
    let mut path = Vec::new();
    let mut done = std::collections::HashSet::new();
    let mut out = Vec::new();
    load_chain(packs, id, 1, &mut path, &mut done, &mut out)?;
    Ok(out)
}

/// Profondeur totale maximale d'une chaîne : le pack + 4 niveaux de parents.
const MAX_CHAIN_LEN: usize = MAX_DEPTH + 1;

fn load_chain(
    packs: &[Pack],
    id: &str,
    depth: usize,
    path: &mut Vec<String>,
    done: &mut std::collections::HashSet<String>,
    out: &mut Vec<Pack>,
) -> Result<()> {
    if depth > MAX_CHAIN_LEN {
        return Err(Error::Chain(id.to_string(), MAX_DEPTH));
    }
    if path.iter().any(|v| v == id) {
        return Err(Error::Chain(id.to_string(), MAX_DEPTH));
    }
    if done.contains(id) {
        return Ok(());
    }
    let pack = find(packs, id)
        .ok_or_else(|| Error::Missing(id.to_string()))?
        .clone();
    path.push(id.to_string());
    for parent in &pack.manifest.inherits.themes {
        // Parent manquant : avertissement, la résolution continue (§6).
        if find(packs, parent).is_none() {
            continue;
        }
        load_chain(packs, parent, depth + 1, path, done, out)?;
    }
    path.pop();
    done.insert(id.to_string());
    out.push(pack);
    Ok(())
}

/// Limite de taille d'un dossier de pack, pour l'installation d'archive.
pub fn pack_dir_too_big(dir: &Path) -> bool {
    fn walk(dir: &Path, total: &mut u64) -> std::io::Result<()> {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let meta = entry.metadata()?;
            if meta.is_dir() {
                walk(&entry.path(), total)?;
            } else {
                *total += meta.len();
            }
        }
        Ok(())
    }
    let mut total = 0;
    walk(dir, &mut total).is_ok() && total > MAX_PACK_BYTES
}

/// Taille maximale d'un pack installé (§14 : 50 Mio).
pub const MAX_PACK_BYTES: u64 = 50 * 1024 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    fn pack(id: &str, inherits: &[&str]) -> Pack {
        let inherits = if inherits.is_empty() {
            String::new()
        } else {
            format!(
                "[inherits]\nthemes = [{}]\n",
                inherits
                    .iter()
                    .map(|i| format!("\"{i}\""))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let text = format!(
            "[meta]\nid = \"{id}\"\nname = \"{id}\"\nversion = \"1.0.0\"\n\n{inherits}\n[palette]\nbg = \"#000000\"\nfg = \"#ffffff\"\naccent = \"#ff0000\"\n"
        );
        let (manifest, _) = Manifest::parse(id, &text).unwrap();
        Pack {
            origin: Origin::Bundled,
            dir: None,
            manifest,
        }
    }

    #[test]
    fn chaine_linéarisée_ancêtres_dabord() {
        let packs = vec![
            pack("base", &[]),
            pack("milieu", &["base"]),
            pack("haut", &["base", "milieu"]),
        ];
        let ids: Vec<String> = chain(&packs, "haut")
            .unwrap()
            .into_iter()
            .map(|p| p.manifest.meta.id)
            .collect();
        assert_eq!(ids, vec!["base", "milieu", "haut"]);
    }

    #[test]
    fn cycle_détecté() {
        let packs = vec![pack("pa", &["pb"]), pack("pb", &["pa"])];
        assert!(chain(&packs, "pa").is_err());
    }

    #[test]
    fn profondeur_bornée() {
        let packs: Vec<Pack> = (0..6)
            .map(|i| {
                if i == 0 {
                    pack("p0", &[])
                } else {
                    pack(&format!("p{i}"), &[&format!("p{}", i - 1)])
                }
            })
            .collect();
        assert!(chain(&packs, "p5").is_err());
        let four: Vec<Pack> = (0..5)
            .map(|i| {
                if i == 0 {
                    pack("q0", &[])
                } else {
                    pack(&format!("q{i}"), &[&format!("q{}", i - 1)])
                }
            })
            .collect();
        assert!(chain(&four, "q4").is_ok());
    }

    #[test]
    fn parent_manquant_en_avertissement_pas_en_erreur() {
        let packs = vec![pack("seul", &["fantôme"])];
        let c = chain(&packs, "seul").unwrap();
        assert_eq!(c.len(), 1);
    }

    #[test]
    fn découverte_priorité_à_lutilisateur() {
        let root = std::env::temp_dir().join(format!("hd-packs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let user_dir = roots.themes_dir().join("zeta");
        std::fs::create_dir_all(&user_dir).unwrap();
        std::fs::write(
            user_dir.join(MANIFEST_FILE),
            "[meta]\nid = \"zeta\"\nname = \"Zeta locale\"\nversion = \"2.0.0\"\n[palette]\nbg = \"#111111\"\nfg = \"#222222\"\naccent = \"#333333\"\n",
        )
        .unwrap();
        let (packs, warnings) = discover(&roots, &[pack("zeta", &[])]).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        let zeta = find(&packs, "zeta").unwrap();
        assert_eq!(zeta.origin, Origin::User);
        assert_eq!(zeta.manifest.meta.version, "2.0.0");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dossier_invalide_en_avertissement() {
        let root = std::env::temp_dir().join(format!("hd-packs-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let dir = roots.themes_dir().join("cassé");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(MANIFEST_FILE), "pas du toml ===").unwrap();
        let (_, warnings) = discover(&roots, &[]).unwrap();
        assert_eq!(warnings.len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }
}
