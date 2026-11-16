//! Politique d'administration du poste et dossier d'exécution privé.
//!
//! `/etc/hyprdesk/policy.toml` (propriété de root, non modifiable par
//! l'utilisateur) impose des réglages que la configuration Lua, les Réglages
//! et une archive importée ne peuvent pas contourner. Fichier absent ou
//! illisible : aucune contrainte. Fichier présent mais appartenant à un autre
//! compte que root, ou modifiable par le groupe ou les autres : ignoré et
//! signalé (une politique que l'utilisateur peut écrire n'en est pas une).
//!
//! ```toml
//! fingerprint = false            # empreinte interdite au verrouillage
//! idle_lock_max = 600            # verrouillage sur inactivité imposé (s, ≤)
//! third_party_plugins = false    # installation de plugins tiers interdite
//! index_removable = false        # supports amovibles exclus de l'index
//! clipboard_history = false      # historique du presse-papiers interdit
//! ```
//!
//! [`runtime_dir`] rend le dossier d'exécution de l'utilisateur
//! (`$XDG_RUNTIME_DIR`, sinon `/run/user/<uid>`), après avoir vérifié qu'il
//! lui appartient et n'est ouvert qu'à lui (0700). Jamais de repli sur `/tmp`,
//! partagé entre comptes.

// Fonctions branchées au fil des lots MLG-150 à MLG-155.
#![allow(dead_code)]

use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const PATH: &str = "/etc/hyprdesk/policy.toml";

#[derive(Debug, Default, Clone, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Policy {
    /// `Some(false)` : empreinte interdite ; `Some(true)` : autorisée (le
    /// réglage de l'utilisateur décide) ; absent : réglage de l'utilisateur.
    pub fingerprint: Option<bool>,
    /// Délai maximal (secondes) avant verrouillage sur inactivité : la gestion
    /// de l'inactivité est alors imposée et « jamais » refusé.
    pub idle_lock_max: Option<u32>,
    /// `Some(false)` : installer un plugin tiers (git, dossier) est refusé.
    pub third_party_plugins: Option<bool>,
    /// `Some(false)` : les supports amovibles ne sont pas indexés.
    pub index_removable: Option<bool>,
    /// `Some(false)` : l'historique du presse-papiers ne démarre pas.
    pub clipboard_history: Option<bool>,
}

impl Policy {
    pub fn fingerprint_allowed(&self) -> bool {
        self.fingerprint != Some(false)
    }
    pub fn plugins_allowed(&self) -> bool {
        self.third_party_plugins != Some(false)
    }
    pub fn removable_allowed(&self) -> bool {
        self.index_removable != Some(false)
    }
    pub fn clipboard_allowed(&self) -> bool {
        self.clipboard_history != Some(false)
    }
    /// Délai de verrouillage effectif : celui de l'utilisateur, borné par la
    /// politique (`0` = jamais, remplacé par le maximum imposé).
    pub fn clamp_idle_lock(&self, user: u32) -> u32 {
        match self.idle_lock_max {
            Some(max) if user == 0 || user > max => max,
            _ => user,
        }
    }
}

/// Politique du poste, lue une fois par processus.
pub fn get() -> &'static Policy {
    static P: OnceLock<Policy> = OnceLock::new();
    P.get_or_init(|| load(Path::new(PATH)))
}

fn load(path: &Path) -> Policy {
    let Ok(meta) = std::fs::metadata(path) else {
        return Policy::default();
    };
    if let Err(e) = trusted(&meta) {
        eprintln!("hyprdesk : {} ignoré ({e})", path.display());
        return Policy::default();
    }
    match std::fs::read_to_string(path).map(|s| parse(&s)) {
        Ok(Ok(p)) => p,
        Ok(Err(e)) => {
            eprintln!("hyprdesk : {} invalide, ignoré : {e}", path.display());
            Policy::default()
        }
        Err(_) => Policy::default(),
    }
}

fn parse(s: &str) -> Result<Policy, String> {
    toml::from_str(s).map_err(|e| e.to_string())
}

/// Le fichier de politique doit appartenir à root et n'être modifiable que
/// par lui.
fn trusted(meta: &std::fs::Metadata) -> Result<(), &'static str> {
    use std::os::unix::fs::MetadataExt;
    if meta.uid() != 0 {
        return Err("n'appartient pas à root");
    }
    if meta.mode() & 0o022 != 0 {
        return Err("modifiable par le groupe ou les autres");
    }
    Ok(())
}

/// Dossier d'exécution privé de l'utilisateur, vérifié (propriétaire, 0700).
pub fn runtime_dir() -> Result<PathBuf, String> {
    // SAFETY : getuid ne peut pas échouer.
    let uid = unsafe { libc::getuid() };
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|d| !d.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("/run/user/{uid}")));
    check_private(&dir, uid)?;
    Ok(dir)
}

fn check_private(dir: &Path, uid: u32) -> Result<(), String> {
    use std::os::unix::fs::MetadataExt;
    // symlink_metadata : un lien posé à la place du dossier est refusé.
    let meta = std::fs::symlink_metadata(dir).map_err(|e| {
        format!(
            "dossier d'exécution {} indisponible ({e}) : XDG_RUNTIME_DIR requis",
            dir.display()
        )
    })?;
    if !meta.is_dir() {
        return Err(format!("{} n'est pas un dossier", dir.display()));
    }
    if meta.uid() != uid {
        return Err(format!("{} n'appartient pas à l'utilisateur", dir.display()));
    }
    if meta.mode() & 0o077 != 0 {
        return Err(format!("{} est ouvert à d'autres comptes", dir.display()));
    }
    Ok(())
}

/// Sous-dossier privé (0700) du dossier d'exécution, créé au besoin.
pub fn runtime_subdir(name: &str) -> Result<PathBuf, String> {
    use std::os::unix::fs::DirBuilderExt;
    let dir = runtime_dir()?.join(name);
    match std::fs::DirBuilder::new().mode(0o700).create(&dir) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(format!("{} : {e}", dir.display())),
    }
    // SAFETY : getuid ne peut pas échouer.
    check_private(&dir, unsafe { libc::getuid() })?;
    Ok(dir)
}

/// Crée un fichier privé (0600) de façon exclusive, sans suivre de lien.
pub fn create_private(path: &Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
}

/// Écrit `data` dans `path` par un fichier temporaire 0600 du même dossier,
/// puis renommage atomique.
pub fn write_private(path: &Path, data: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    let _ = std::fs::remove_file(&tmp);
    let mut f = create_private(&tmp)?;
    f.write_all(data)?;
    f.sync_all()?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn politique_lue() {
        let p = parse("fingerprint = false\nidle_lock_max = 600\n").unwrap();
        assert!(!p.fingerprint_allowed());
        assert!(p.plugins_allowed());
        assert_eq!(p.clamp_idle_lock(0), 600);
        assert_eq!(p.clamp_idle_lock(1200), 600);
        assert_eq!(p.clamp_idle_lock(300), 300);
        assert!(parse("inconnue = 1").is_err());
        assert_eq!(Policy::default().clamp_idle_lock(0), 0);
    }

    #[test]
    fn dossier_prive_verifie() {
        let base = std::env::temp_dir().join(format!("hd-policy-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir(&base).unwrap();
        use std::os::unix::fs::PermissionsExt;
        let uid = unsafe { libc::getuid() };
        std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(check_private(&base, uid).is_err());
        std::fs::set_permissions(&base, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(check_private(&base, uid).is_ok());
        assert!(check_private(&base, uid + 1).is_err());
        let f = base.join("secret");
        create_private(&f).unwrap();
        assert!(create_private(&f).is_err(), "création exclusive");
        let mode = std::fs::metadata(&f).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        write_private(&f, b"x").unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"x");
        let _ = std::fs::remove_dir_all(&base);
    }
}
