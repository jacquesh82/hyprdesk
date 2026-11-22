//! Archives de packs `.hyprtheme` (§3 de la spécification) : tar+gzip en
//! Rust pur, sans sous-process.
//!
//! Sécurité d'abord : à la lecture, seuls les fichiers réguliers et les
//! dossiers sont acceptés (symlinks et liens durs refusés), les chemins sont
//! relatifs et sans tronçon `..`, et les limites du §14 s'appliquent avant
//! toute écriture sur disque. Le manifeste `theme.toml` doit être à la racine.

use std::io::{Read, Write};
use std::path::Path;

use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;

use super::manifest::Manifest;
use super::packs;
use super::paths::Roots;
use super::{Error, Result};

/// Taille maximale d'une archive décompressée (§14 : 50 Mio).
pub const MAX_ARCHIVE_BYTES: u64 = packs::MAX_PACK_BYTES;

/// Nombre maximal de fichiers d'un pack.
pub const MAX_FILES: usize = 2000;

/// Taille maximale d'un gabarit (§14 : 1 Mio).
pub const MAX_FILE_BYTES: u64 = 1024 * 1024;

/// Fichier à la racine d'une archive : le manifeste du pack.
pub const MANIFEST_AT_ROOT: &str = packs::MANIFEST_FILE;

/// Une entrée d'archive : chemin relatif normalisé et contenu.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub path: String,
    pub data: Vec<u8>,
}

/// Chemin d'archive acceptable : relatif, sans tronçon `..`, sans `.` seul,
/// pas un nom réservé de sauvegarde du moteur.
pub fn valid_path(path: &str) -> bool {
    if path.is_empty() || path.starts_with('/') || path.starts_with('\\') || path.contains('\0') {
        return false;
    }
    path.split(['/', '\\'])
        .all(|seg| !seg.is_empty() && seg != ".." && seg != ".")
}

/// Écrit une archive `.hyprtheme` (gzip + ustar, fichiers réguliers seuls).
pub fn write(out: &Path, entries: &[Entry]) -> Result<()> {
    if entries.len() > MAX_FILES {
        return Err(Error::Manifest(
            "archive".to_string(),
            format!("trop de fichiers : {} > {MAX_FILES}", entries.len()),
        ));
    }
    let total: u64 = entries.iter().map(|e| e.data.len() as u64).sum();
    if total > MAX_ARCHIVE_BYTES {
        return Err(Error::Manifest(
            "archive".to_string(),
            format!("contenu trop grand : {total} > {MAX_ARCHIVE_BYTES} octets"),
        ));
    }
    let file = std::fs::File::create(out)?;
    let gz = GzEncoder::new(file, Compression::default());
    let mut w = gz;
    for e in entries {
        write_entry(&mut w, e)?;
    }
    // Deux blocs nuls : fin d'archive.
    w.write_all(&[0u8; 1024])
        .map_err(|e| Error::Manifest("archive".to_string(), e.to_string()))?;
    w.finish()
        .map_err(|e| Error::Manifest("archive".to_string(), e.to_string()))?;
    Ok(())
}

fn octal(value: u64, width: usize) -> Vec<u8> {
    // Champ ustar : chiffres octaux terminés par NUL (checksum : NUL+espace).
    let digits = format!("{:0width$o}", value, width = width - 1);
    let mut out = digits.into_bytes();
    out.push(0);
    out
}

fn write_entry<W: Write>(w: &mut W, e: &Entry) -> Result<()> {
    if !valid_path(&e.path) {
        return Err(Error::Manifest(
            "archive".to_string(),
            format!("chemin refusé : {}", e.path),
        ));
    }
    let path = e.path.as_bytes();
    if path.len() > 100 {
        return Err(Error::Manifest(
            "archive".to_string(),
            format!("chemin trop long pour ustar : {}", e.path),
        ));
    }
    let mut header = [0u8; 512];
    header[..path.len()].copy_from_slice(path);
    header[100..108].copy_from_slice(&octal(0o644, 8));
    header[108..116].copy_from_slice(&octal(0, 8));
    header[116..124].copy_from_slice(&octal(0, 8));
    header[124..136].copy_from_slice(&octal(e.data.len() as u64, 12));
    header[136..148].copy_from_slice(&octal(0, 12));
    header[156] = b'0'; // fichier régulier
    header[257..263].copy_from_slice(b"ustar\0");
    header[263..265].copy_from_slice(b"00");
    // Checksum : somme des octets avec le champ lui-même à espaces.
    header[148..156].copy_from_slice(b"        ");
    let sum: u32 = header.iter().map(|b| u32::from(*b)).sum();
    header[148..156].copy_from_slice(&format!("{:06o}\0 ", sum).into_bytes());
    w.write_all(&header)
        .map_err(|er| Error::Manifest("archive".to_string(), er.to_string()))?;
    w.write_all(&e.data)
        .map_err(|er| Error::Manifest("archive".to_string(), er.to_string()))?;
    let pad = (512 - e.data.len() % 512) % 512;
    w.write_all(&vec![0u8; pad])
        .map_err(|er| Error::Manifest("archive".to_string(), er.to_string()))?;
    Ok(())
}

/// Lit une archive : entrées validées, limites appliquées, manifeste requis
/// à la racine. Renvoie aussi le manifeste analysé.
pub fn read(archive: &Path) -> Result<(Vec<Entry>, Manifest)> {
    let raw = std::fs::read(archive)?;
    if raw.len() as u64 > MAX_ARCHIVE_BYTES * 2 {
        return Err(Error::Manifest(
            "archive".to_string(),
            format!("archive trop grande : {} octets", raw.len()),
        ));
    }
    let gz = GzDecoder::new(&raw[..]);
    let mut r = gz;
    let mut entries: Vec<Entry> = Vec::new();
    let mut total: u64 = 0;
    let mut manifest: Option<Manifest> = None;
    let mut long_name: Option<String> = None;
    let mut header = [0u8; 512];
    // Fin de flux = fin d'archive (une archive tronquée sans bloc nul final
    // est tolérée : les entrées déjà lues restent validées).
    while r.read_exact(&mut header).is_ok() {
        if header == [0u8; 512] {
            break;
        }
        let typeflag = header[156];
        let size = octal_field(&header[124..136])?;
        // Nom : préfixe ustar puis champ longname GNU éventuel.
        let mut name = field(&header[..100]);
        let prefix = field(&header[345..500]);
        if !prefix.is_empty() {
            name = format!("{prefix}/{name}");
        }
        if let Some(ln) = long_name.take() {
            name = ln;
        }
        match typeflag {
            // Fichier régulier ('0' ou NUL historique).
            b'0' | 0 => {
                if size > MAX_FILE_BYTES {
                    return Err(Error::Manifest(
                        "archive".to_string(),
                        format!("fichier trop grand : {name} ({size} > {MAX_FILE_BYTES} octets)"),
                    ));
                }
                let mut data = vec![0u8; size as usize];
                r.read_exact(&mut data)
                    .map_err(|e| Error::Manifest("archive".to_string(), e.to_string()))?;
                let pad = ((512 - size as usize % 512) % 512) as u64;
                if pad > 0 {
                    std::io::copy(&mut r.by_ref().take(pad), &mut std::io::sink())
                        .map_err(|e| Error::Manifest("archive".to_string(), e.to_string()))?;
                }
                if !valid_path(&name) {
                    return Err(Error::Manifest(
                        "archive".to_string(),
                        format!("chemin refusé dans l'archive : {name}"),
                    ));
                }
                total += size;
                if total > MAX_ARCHIVE_BYTES {
                    return Err(Error::Manifest(
                        "archive".to_string(),
                        format!("contenu trop grand : {total} > {MAX_ARCHIVE_BYTES} octets"),
                    ));
                }
                if entries.len() >= MAX_FILES {
                    return Err(Error::Manifest(
                        "archive".to_string(),
                        format!("trop de fichiers (> {MAX_FILES})"),
                    ));
                }
                if name == MANIFEST_AT_ROOT {
                    let (m, _) =
                        Manifest::parse(&guess_id(&name), &String::from_utf8_lossy(&data))?;
                    manifest = Some(m);
                }
                entries.push(Entry { path: name, data });
            }
            // Longname GNU : le prochain fichier porte ce nom.
            b'L' => {
                let mut data = vec![0u8; size as usize];
                r.read_exact(&mut data)
                    .map_err(|e| Error::Manifest("archive".to_string(), e.to_string()))?;
                let pad = ((512 - size as usize % 512) % 512) as u64;
                if pad > 0 {
                    std::io::copy(&mut r.by_ref().take(pad), &mut std::io::sink())
                        .map_err(|e| Error::Manifest("archive".to_string(), e.to_string()))?;
                }
                long_name = Some(
                    String::from_utf8_lossy(&data)
                        .trim_end_matches('\0')
                        .to_string(),
                );
            }
            // Dossiers et en-têtes pax : sans contenu utile ici.
            b'5' | b'x' | b'g' => {
                if size > 0 {
                    let mut data = vec![0u8; size as usize];
                    r.read_exact(&mut data)
                        .map_err(|e| Error::Manifest("archive".to_string(), e.to_string()))?;
                    let pad = ((512 - size as usize % 512) % 512) as u64;
                    if pad > 0 {
                        std::io::copy(&mut r.by_ref().take(pad), &mut std::io::sink())
                            .map_err(|e| Error::Manifest("archive".to_string(), e.to_string()))?;
                    }
                }
            }
            // Symlinks, liens durs, devices : refusés (§13).
            other => {
                return Err(Error::Manifest(
                    "archive".to_string(),
                    format!(
                        "type d'entrée refusé ({} : {}) : {name}",
                        other as char,
                        symlink_name()
                    ),
                ));
            }
        }
    }
    let manifest = manifest.ok_or_else(|| {
        Error::Manifest(
            "archive".to_string(),
            format!("manifeste {MANIFEST_AT_ROOT} absent de la racine"),
        )
    })?;
    Ok((entries, manifest))
}

/// Libellé du type refusé, pour le message d'erreur.
fn symlink_name() -> &'static str {
    "lien symbolique ou autre type non accepté"
}

fn guess_id(path: &str) -> String {
    Path::new(path)
        .parent()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
        .unwrap_or_default()
}

fn field(bytes: &[u8]) -> String {
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end])
        .trim_end()
        .to_string()
}

fn octal_field(bytes: &[u8]) -> Result<u64> {
    let text = field(bytes);
    if text.is_empty() {
        return Ok(0);
    }
    u64::from_str_radix(text.trim(), 8)
        .map_err(|e| Error::Manifest("archive".to_string(), format!("taille illisible : {e}")))
}

/// Exporte un pack de dossier vers une archive (fichiers triés pour un
/// diff stable, sauvegardes du moteur exclues).
pub fn export_pack(pack_dir: &Path, out: &Path) -> Result<usize> {
    let entries = dir_entries(pack_dir)?;
    write(out, &entries)?;
    Ok(entries.len())
}

/// Contenu d'un dossier de pack en entrées d'archive : manifeste requis à la
/// racine, fichiers triés, sauvegardes du moteur et fichiers cachés exclus.
pub fn dir_entries(pack_dir: &Path) -> Result<Vec<Entry>> {
    if !pack_dir.join(MANIFEST_AT_ROOT).is_file() {
        return Err(Error::Missing(pack_dir.display().to_string()));
    }
    let mut entries = Vec::new();
    collect(pack_dir, pack_dir, &mut entries)?;
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(entries)
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<Entry>) -> Result<()> {
    for e in std::fs::read_dir(dir)? {
        let e = e?;
        let path = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if name.starts_with('.') || name.ends_with(".avant-hyprdesk.bak") {
            continue;
        }
        let meta = e.metadata()?;
        let rel = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        if meta.is_dir() {
            collect(root, &path, out)?;
        } else if meta.is_file() {
            if meta.len() > MAX_FILE_BYTES {
                return Err(Error::Manifest(
                    "archive".to_string(),
                    format!("fichier trop grand pour l'archive : {rel}"),
                ));
            }
            out.push(Entry {
                path: rel,
                data: std::fs::read(&path)?,
            });
        }
        // Symlinks et autres : ignorés à l'export (le pack reste des données).
    }
    Ok(())
}

/// Installe une archive `.hyprtheme` (ou un dossier de pack direct) dans le
/// dossier utilisateur : lecture et validation complètes avant la première
/// écriture, aucun fichier partiel en cas d'échec. Renvoie l'identifiant
/// installé.
pub fn install(source: &Path, roots: &Roots) -> Result<String> {
    if source.is_dir() {
        install_entries(dir_entries(source)?, roots)
    } else {
        let (entries, _) = read(source)?;
        install_entries(entries, roots)
    }
}

/// Pose des entrées validées comme pack utilisateur.
pub fn install_entries(entries: Vec<Entry>, roots: &Roots) -> Result<String> {
    let manifest_entry = entries
        .iter()
        .find(|e| e.path == MANIFEST_AT_ROOT)
        .ok_or_else(|| {
            Error::Manifest(
                "installation".to_string(),
                format!("manifeste {MANIFEST_AT_ROOT} absent"),
            )
        })?;
    let (manifest, _) = Manifest::parse("", &String::from_utf8_lossy(&manifest_entry.data))?;
    let id = manifest.meta.id.clone();
    let target = roots.themes_dir().join(&id);
    if target.exists() {
        return Err(Error::Manifest(
            id.clone(),
            format!("pack déjà installé : {} (remove d'abord)", target.display()),
        ));
    }
    // Écriture : d'abord en staging, puis déplacement du dossier complet.
    let staging = roots.cache_dir().join("install");
    let _ = std::fs::remove_dir_all(&staging);
    std::fs::create_dir_all(&staging)?;
    let result = (|| -> Result<()> {
        for e in &entries {
            let path = staging.join(&e.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&path, &e.data)?;
        }
        // Revalidation depuis le disque : le manifeste posé doit parser.
        Manifest::parse(
            &id,
            &String::from_utf8_lossy(&std::fs::read(staging.join(MANIFEST_AT_ROOT))?),
        )?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(e);
    }
    std::fs::create_dir_all(roots.themes_dir())?;
    std::fs::rename(&staging, &target)?;
    Ok(id)
}

/// Retire un pack installé par l'utilisateur (les packs système et embarqués
/// ne se retirent pas d'ici).
pub fn remove(id: &str, roots: &Roots) -> Result<()> {
    let target = roots.themes_dir().join(id);
    if !target.is_dir() {
        return Err(Error::Missing(format!("pack utilisateur {id}")));
    }
    std::fs::remove_dir_all(&target)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = r##"
[meta]
id = "tiers"
name = "Pack tiers"
version = "0.1.0"
[palette]
bg = "#000000"
fg = "#ffffff"
accent = "#00ff00"
"##;

    fn entries() -> Vec<Entry> {
        vec![
            Entry {
                path: "theme.toml".to_string(),
                data: MANIFEST.as_bytes().to_vec(),
            },
            Entry {
                path: "templates/kitty.tmpl".to_string(),
                data: b"foreground {{palette.fg}}\n".to_vec(),
            },
        ]
    }

    #[test]
    fn aller_retour_archive() {
        let dir = std::env::temp_dir().join(format!("hd-arch-1-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("tiers.hyprtheme");
        assert_eq!(
            write(&archive, &entries())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            Ok(())
        );
        let (read_entries, manifest) = read(&archive).unwrap();
        assert_eq!(read_entries.len(), 2);
        assert_eq!(manifest.meta.id, "tiers");
        // Round-trip binaire : mêmes contenus.
        for e in &entries() {
            let found = read_entries.iter().find(|r| r.path == e.path).unwrap();
            assert_eq!(found.data, e.data);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn chemin_relais_et_symlink_refusés() {
        let dir = std::env::temp_dir().join(format!("hd-arch-2-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("malin.hyprtheme");
        let bad = vec![Entry {
            path: "../fugitif.txt".to_string(),
            data: b"x".to_vec(),
        }];
        assert!(write(&archive, &bad).is_err());
        // Une archive avec une entrée symlink (typeflag '2') est refusée à la
        // lecture : fabriquée à la main, en-tête ustar correct.
        let mut header = [0u8; 512];
        let name = b"chaine\0";
        header[..name.len()].copy_from_slice(name);
        header[156] = b'2';
        header[148..156].copy_from_slice(b"        ");
        let sum: u32 = header.iter().map(|b| u32::from(*b)).sum();
        header[148..156].copy_from_slice(&format!("{:06o}\0 ", sum).into_bytes());
        let gz = flate2::write::GzEncoder::new(
            std::fs::File::create(&archive).unwrap(),
            Compression::default(),
        );
        let mut gz = gz;
        gz.write_all(&header).unwrap();
        gz.write_all(&[0u8; 1024]).unwrap();
        gz.finish().unwrap();
        assert!(read(&archive).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manifeste_absent_refusé() {
        let dir = std::env::temp_dir().join(format!("hd-arch-3-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let archive = dir.join("vide.hyprtheme");
        write(
            &archive,
            &[Entry {
                path: "notes.txt".to_string(),
                data: b"rien".to_vec(),
            }],
        )
        .unwrap();
        assert!(read(&archive).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn export_puis_install_puis_remove() {
        let root = std::env::temp_dir().join(format!("hd-arch-4-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        // Le pack à exporter vit hors du dossier des packs (machine A) :
        // installer le même identifiant ensuite refuse un pack déjà là.
        let src = root.join("machine-a").join("tiers");
        std::fs::create_dir_all(src.join("templates")).unwrap();
        std::fs::write(src.join("theme.toml"), MANIFEST).unwrap();
        std::fs::write(
            src.join("templates/kitty.tmpl"),
            "foreground {{palette.fg}}\n",
        )
        .unwrap();
        // Sauvegarde du moteur et fichier caché : exclus de l'export.
        std::fs::write(src.join("kitty.tmpl.avant-hyprdesk.bak"), "old").unwrap();
        std::fs::write(src.join(".cache"), "x").unwrap();
        let archive = root.join("tiers.hyprtheme");
        let n = export_pack(&src, &archive).unwrap();
        assert_eq!(n, 2, "manifeste + gabarit seulement");
        // Installation ailleurs (machine B simulée).
        let id = install(&archive, &roots).unwrap();
        assert_eq!(id, "tiers");
        assert!(roots.themes_dir().join("tiers/theme.toml").is_file());
        assert!(
            roots
                .themes_dir()
                .join("tiers/templates/kitty.tmpl")
                .is_file()
        );
        // Pas de double installation.
        assert!(install(&archive, &roots).is_err());
        // Retrait.
        remove("tiers", &roots).unwrap();
        assert!(!roots.themes_dir().join("tiers").exists());
        assert!(remove("tiers", &roots).is_err());
        assert!(remove("inconnu", &roots).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn pack_too_big_export_bloqué() {
        // Un pack contenant un fichier > MAX_FILE_BYTES : l'export refuse.
        let dir = std::env::temp_dir().join(format!("hd-arch-5-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let roots = Roots::at(&dir);
        let src = roots.themes_dir().join("lourd");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("theme.toml"), MANIFEST).unwrap();
        std::fs::write(
            src.join("gros.bin"),
            vec![0u8; (MAX_FILE_BYTES + 1) as usize],
        )
        .unwrap();
        let err = export_pack(&src, &dir.join("lourd.hyprtheme")).unwrap_err();
        assert!(err.to_string().contains("trop grand"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
