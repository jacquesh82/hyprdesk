//! Binaires dédiés, installés à côté de `hyprdesk` : l'écran de
//! verrouillage, l'écran de connexion et l'agent polkit tournent chacun dans
//! un exécutable qui ne lie que leur code (ni démon, ni Lua, ni index).
//!
//! `hyprdesk lock`, `hyprdesk greeter` et `hyprdesk polkit-ui` restent des
//! commandes valides (raccourcis et configurations déjà posés) : elles passent
//! la main au binaire dédié s'il est là, sinon s'exécutent comme avant.

use std::path::{Path, PathBuf};

/// Écran de verrouillage (`hyprdesk lock`).
pub const LOCK: &str = "hyprdesk-lock";
/// Écran de connexion pour greetd (`hyprdesk greeter`).
pub const GREETER: &str = "hyprdesk-greeter";
/// Agent polkit de la session (`hyprdesk polkit-ui`, `polkit-agent`).
pub const POLKIT: &str = "hyprdesk-polkit";

/// Le binaire dédié `name` posé à côté de `exe`, s'il est exécutable.
pub fn beside(exe: &Path, name: &str) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;
    let p = exe.with_file_name(name);
    let m = std::fs::metadata(&p).ok()?;
    (m.is_file() && m.permissions().mode() & 0o111 != 0).then_some(p)
}

/// Le binaire dédié `name` à côté de l'exécutable courant.
pub fn path(name: &str) -> Option<PathBuf> {
    beside(&std::env::current_exe().ok()?, name)
}

/// Commande shell lançant `name` (binaire dédié), sinon `hyprdesk <fallback>`
/// avec l'exécutable courant : pour hypridle, Hyprland, le démon.
pub fn command(name: &str, fallback: &str) -> String {
    match path(name) {
        Some(p) => crate::launch::sh_quote(&p.display().to_string()),
        None => {
            let exe = std::env::current_exe()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "hyprdesk".into());
            format!("{} {fallback}", crate::launch::sh_quote(&exe))
        }
    }
}

/// Remplace le processus par le binaire dédié `name` avec `args`. Ne revient
/// que s'il est absent ou que l'`exec` échoue : l'appelant fait alors le
/// travail lui-même.
pub fn exec(name: &str, args: &[String]) {
    use std::os::unix::process::CommandExt;
    let Some(p) = path(name) else {
        return;
    };
    let e = std::process::Command::new(&p).args(args).exec();
    eprintln!(
        "{}",
        crate::tr!(
            "hyprdesk : {p} impossible à lancer ({e}), repli sur le binaire principal",
            p = p.display(),
            e = e
        )
    );
}

/// Début commun des binaires dédiés : `--config <fichier>` retiré des
/// arguments, `--version` traité ; langue et moteur de rendu préparés comme
/// le fait `hyprdesk`. `Err` : code de sortie à rendre aussitôt.
pub fn start(name: &str, args: &mut Vec<String>) -> Result<PathBuf, std::process::ExitCode> {
    let mut config = crate::config::default_path();
    if let Some(i) = args.iter().position(|a| a == "--config" || a == "-c") {
        let Some(p) = args.get(i + 1).cloned() else {
            eprintln!("{}", crate::tr!("--config attend un fichier"));
            return Err(std::process::ExitCode::from(2));
        };
        config = PathBuf::from(p);
        args.drain(i..=i + 1);
    }
    if args.first().is_some_and(|a| a == "--version" || a == "-V") {
        println!("{name} {}", env!("CARGO_PKG_VERSION"));
        return Err(std::process::ExitCode::SUCCESS);
    }
    crate::i18n::init_cli(&config);
    crate::history::init(&config);
    crate::render::init(&config, false);
    Ok(config)
}

/// Arguments à passer au binaire dédié : `--config` seulement s'il a été
/// donné à `hyprdesk`, puis `rest`.
pub fn forward(config: &Path, rest: &[&str]) -> Vec<String> {
    let mut v = Vec::new();
    if config != crate::config::default_path() {
        v.push("--config".to_string());
        v.push(config.display().to_string());
    }
    v.extend(rest.iter().map(|s| s.to_string()));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binaire_voisin() {
        let d = std::env::temp_dir().join(format!("hyprdesk-companion-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let exe = d.join("hyprdesk");
        assert_eq!(beside(&exe, LOCK), None, "absent");
        let lock = d.join(LOCK);
        std::fs::write(&lock, "#!/bin/sh\n").unwrap();
        assert_eq!(beside(&exe, LOCK), None, "pas exécutable");
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&lock, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(beside(&exe, LOCK), Some(lock));
        std::fs::remove_dir_all(&d).unwrap();
    }
}
