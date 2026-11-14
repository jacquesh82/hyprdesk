//! Persistance de l'index : snapshot binaire compressé.
//!
//! Format « HDI1 » : volumes, blob des chemins, métadonnées par entrée —
//! rien de dérivé (les postings sont reconstruites au chargement, une seule
//! représentation du même invariant). Le tout passe à la moulinette gzip et
//! s'écrit de façon atomique (fichier temporaire puis renommage) : un crash
//! laisse l'ancien snapshot intact, un snapshot corrompu renvoie à un
//! simple rescan.

use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use thiserror::Error;

use super::index::{Index, Volume};

/// Étiquette de format ; un changement cassant passera en « HDI2 ».
const MAGIC: &[u8; 4] = b"HDI1";
const VERSION: u32 = 1;

/// Erreurs de chargement du snapshot.
#[derive(Debug, Error)]
pub enum StoreError {
    /// Pas encore de snapshot (premier lancement).
    #[error("aucun snapshot")]
    Absent,
    /// Snapshot illisible ou d'une autre époque.
    #[error("snapshot corrompu : {0}")]
    Corrupt(String),
    /// Erreur d'entrée/sortie.
    #[error("E/S : {0}")]
    Io(#[from] std::io::Error),
}

impl StoreError {
    /// Vrai si c'est juste « pas de snapshot encore ».
    pub fn is_absent(&self) -> bool {
        matches!(self, StoreError::Absent)
    }
}

/// `$XDG_CACHE_HOME/hyprdesk/index/` (défaut `~/.cache`).
pub fn snapshot_dir() -> PathBuf {
    let base = match std::env::var_os("XDG_CACHE_HOME") {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => {
            let mut p = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/tmp"));
            p.push(".cache");
            p
        }
    };
    base.join("hyprdesk").join("index")
}

/// Chemin du snapshot.
pub fn snapshot_path() -> PathBuf {
    snapshot_dir().join("snapshot.bin")
}

/// Écrit le snapshot au chemin par défaut (écriture atomique).
pub fn save(index: &Index) -> std::io::Result<()> {
    let path = snapshot_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    save_to(&path, index)
}

/// Écrit le snapshot dans un fichier précis (utile aux tests).
pub fn save_to(path: &std::path::Path, index: &Index) -> std::io::Result<()> {
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    gz.write_all(&encoder(index))?;
    let data = gz.finish()?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, &data)?;
    // renommage atomique : un crash laisse l'ancien snapshot intact
    fs::rename(&tmp, path)
}

/// Charge le snapshot par défaut.
pub fn load() -> Result<Index, StoreError> {
    load_from(&snapshot_path())
}

/// Charge un snapshot depuis un fichier précis.
pub fn load_from(path: &std::path::Path) -> Result<Index, StoreError> {
    let bytes = fs::read(path).map_err(|e| match e.kind() {
        std::io::ErrorKind::NotFound => StoreError::Absent,
        _ => StoreError::Io(e),
    })?;
    let mut gz = GzDecoder::new(&bytes[..]);
    let mut brut = Vec::new();
    gz.read_to_end(&mut brut)
        .map_err(|e| StoreError::Corrupt(e.to_string()))?;
    decoder(&brut).map_err(StoreError::Corrupt)
}

// ——————————————————————— encodage ———————————————————————

fn encoder(index: &Index) -> Vec<u8> {
    // entrées vivantes seulement : le snapshot ne conserve pas les pierres
    // tombales, un chargement repart propre
    let vivantes: Vec<usize> = (0..index.starts.len())
        .filter(|&i| index.alive[i])
        .collect();
    let mut out = Vec::with_capacity(index.blob.len() + vivantes.len() * 24 + 64);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());

    out.extend_from_slice(&(index.volumes.len() as u32).to_le_bytes());
    for v in &index.volumes {
        chaine(&mut out, &v.root);
        chaine(&mut out, &v.fs);
    }

    out.extend_from_slice(&(vivantes.len() as u32).to_le_bytes());

    let mut blob = Vec::with_capacity(index.blob.len());
    let mut starts = Vec::with_capacity(vivantes.len());
    let mut kinds = Vec::with_capacity(vivantes.len());
    let mut vols = Vec::with_capacity(vivantes.len());
    for &i in &vivantes {
        let (s, e) = bornes(index, i);
        starts.push(blob.len() as u32);
        blob.extend_from_slice(&index.blob[s..e]);
        kinds.push(index.kinds[i]);
        vols.push(index.vols[i]);
    }
    out.extend_from_slice(&(blob.len() as u32).to_le_bytes());
    out.extend_from_slice(&blob);
    for s in &starts {
        out.extend_from_slice(&s.to_le_bytes());
    }
    out.extend_from_slice(&kinds);
    out.extend_from_slice(&vols);
    for &i in &vivantes {
        out.extend_from_slice(&index.sizes[i].to_le_bytes());
    }
    for &i in &vivantes {
        out.extend_from_slice(&(index.mtimes[i] as u64).to_le_bytes());
    }
    out
}

fn chaine(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

fn bornes(index: &Index, i: usize) -> (usize, usize) {
    let s = index.starts[i] as usize;
    let e = if i + 1 < index.starts.len() {
        index.starts[i + 1] as usize
    } else {
        index.blob.len()
    };
    (s, e)
}

// ——————————————————————— décodage ———————————————————————

fn decoder(brut: &[u8]) -> Result<Index, String> {
    let mut p = 0usize;
    lire(brut, &mut p)
}

fn lire(brut: &[u8], p: &mut usize) -> Result<Index, String> {
    struct Lecteur<'a> {
        b: &'a [u8],
        p: usize,
    }
    impl Lecteur<'_> {
        fn u32(&mut self) -> Result<u32, String> {
            if self.p + 4 > self.b.len() {
                return Err("fin de données inattendue (u32)".into());
            }
            let o: [u8; 4] = self.b[self.p..self.p + 4].try_into().expect("4 octets");
            self.p += 4;
            Ok(u32::from_le_bytes(o))
        }

        fn u64(&mut self) -> Result<u64, String> {
            if self.p + 8 > self.b.len() {
                return Err("fin de données inattendue (u64)".into());
            }
            let o: [u8; 8] = self.b[self.p..self.p + 8].try_into().expect("8 octets");
            self.p += 8;
            Ok(u64::from_le_bytes(o))
        }

        fn octets(&mut self, n: usize) -> Result<&[u8], String> {
            if self.p + n > self.b.len() {
                return Err(format!("fin de données inattendue ({n} octets)"));
            }
            let s = &self.b[self.p..self.p + n];
            self.p += n;
            Ok(s)
        }

        fn chaine(&mut self) -> Result<String, String> {
            let n = self.u32()? as usize;
            let o = self.octets(n)?;
            Ok(String::from_utf8_lossy(o).into_owned())
        }
    }

    let mut r = Lecteur { b: brut, p: *p };
    if r.octets(4)? != MAGIC {
        return Err("étiquette inconnue".into());
    }
    let version = r.u32()?;
    if version != VERSION {
        return Err(format!("version {version} non gérée"));
    }

    let n_vols = r.u32()? as usize;
    let mut volumes = Vec::with_capacity(n_vols);
    for _ in 0..n_vols {
        let root = r.chaine()?;
        let fs = r.chaine()?;
        volumes.push(Volume { root, fs });
    }

    let n = r.u32()? as usize;
    let blob_len = r.u32()? as usize;
    let blob = r.octets(blob_len)?.to_vec();
    let mut starts = Vec::with_capacity(n);
    for _ in 0..n {
        starts.push(r.u32()?);
    }
    let kinds = r.octets(n)?.to_vec();
    let vols = r.octets(n)?.to_vec();
    let mut sizes = Vec::with_capacity(n);
    let mut mtimes = Vec::with_capacity(n);
    for _ in 0..n {
        sizes.push(r.u64()?);
    }
    for _ in 0..n {
        mtimes.push(r.u64()? as i64);
    }
    *p = r.p;
    Ok(Index::from_parts(
        blob, starts, kinds, sizes, mtimes, vols, volumes,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fileindex::index::{EntryMeta, KIND_DIR, KIND_FILE};

    fn index_exemple() -> Index {
        let mut idx = Index::new();
        idx.volumes = vec![Volume {
            root: "/home/test".into(),
            fs: "ext4".into(),
        }];
        for (p, k) in [
            ("/home/test/hyprdesk.lua", KIND_FILE),
            ("/home/test/HyprDesk.rs", KIND_FILE),
            ("/home/test/docs", KIND_DIR),
            ("/home/test/docs/lisez-moi.md", KIND_FILE),
        ] {
            idx.push(
                p,
                EntryMeta {
                    kind: k,
                    size: 1024,
                    mtime: 1_700_000_123,
                    vol: 0,
                },
            );
        }
        idx
    }

    fn chemin_test(nom: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "hyprdesk-index-test-{nom}-{}.bin",
            std::process::id()
        ));
        p
    }

    #[test]
    fn roundtrip_snapshot() {
        let idx = index_exemple();
        let path = chemin_test("roundtrip");
        save_to(&path, &idx).expect("sauvegarde");
        let charge = load_from(&path).expect("chargement");
        let _ = fs::remove_file(&path);

        assert_eq!(charge.len(), 4);
        assert_eq!(charge.volumes().len(), 1);
        assert_eq!(charge.volumes()[0].root, "/home/test");
        assert_eq!(charge.volumes()[0].fs, "ext4");
        // mêmes résultats qu'avant la sauvegarde
        let avant = idx.search("hyprdesk", 10, None, None);
        let apres = charge.search("hyprdesk", 10, None, None);
        assert_eq!(avant, apres);
        let hit = apres
            .iter()
            .find(|h| h.name == "hyprdesk.lua")
            .expect("hyprdesk.lua");
        assert_eq!(hit.path, "/home/test/hyprdesk.lua");
        assert_eq!(hit.size, 1024);
    }

    #[test]
    fn snapshot_oublie_les_entrees_mortes() {
        let mut idx = index_exemple();
        idx.remove_path("/home/test/HyprDesk.rs");
        let path = chemin_test("mortes");
        save_to(&path, &idx).expect("sauvegarde");
        let charge = load_from(&path).expect("chargement");
        let _ = fs::remove_file(&path);
        assert_eq!(charge.len(), 3);
        assert!(charge.search("HyprDesk.rs", 10, None, None).is_empty());
        assert_eq!(charge.search("lisez", 10, None, None).len(), 1);
    }

    #[test]
    fn snapshot_absent_et_corrompu() {
        assert!(matches!(
            load_from(std::path::Path::new("/tmp/hyprdesk-index-inexistant.bin")),
            Err(StoreError::Absent)
        ));
        let path = chemin_test("corrompu");
        fs::write(&path, b"pas du gzip").expect("écriture");
        assert!(matches!(load_from(&path), Err(StoreError::Corrupt(_))));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn chemin_par_defaut_dans_le_cache() {
        let p = snapshot_path();
        assert!(
            p.ends_with(
                std::path::Path::new("hyprdesk")
                    .join("index")
                    .join("snapshot.bin")
            )
        );
    }
}
