//! Historique du presse-papiers (MLG-13) : hyprdesk orchestre cliphist plutôt
//! que de le refaire.
//!
//! Réglage activé (`clipboard_history`, par défaut) et cliphist installé,
//! hyprdesk lance `wl-paste --watch cliphist store` comme composant de
//! session supervisé — sauf si un tel veilleur tourne déjà (lancé par
//! hyprland.lua, par exemple). Dans le lanceur (`hyprdesk search`), une
//! requête qui commence par `:clip` cherche dans l'historique ; Entrée remet
//! l'élément choisi dans le presse-papiers (`cliphist decode | wl-copy`).

use std::io::Write;
use std::process::{Command, Stdio};

use crate::config::ComponentDef;

/// Nom du composant de session.
pub const COMPONENT: &str = "cliphist";
/// Préfixe de la recherche dans l'historique, dans le lanceur.
pub const PREFIX: &str = ":clip";
/// Éléments lus au plus dans l'historique.
const LIMIT: usize = 200;

/// Un élément de l'historique, tel que `cliphist list` le donne.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// Ligne complète (« id<TAB>aperçu »), à rendre à `cliphist decode`.
    pub line: String,
    /// Aperçu sur une ligne.
    pub preview: String,
}

/// Requête de l'historique si `q` commence par le préfixe (`:clip mot`).
pub fn query(q: &str) -> Option<&str> {
    let rest = q.trim_start().strip_prefix(PREFIX)?;
    (rest.is_empty() || rest.starts_with(' ')).then(|| rest.trim())
}

/// Sortie de `cliphist list` (du plus récent au plus ancien).
pub fn parse(list: &str) -> Vec<Entry> {
    list.lines()
        .filter_map(|l| {
            let (id, preview) = l.split_once('\t')?;
            id.trim().parse::<u64>().ok()?;
            Some(Entry {
                line: l.to_string(),
                preview: preview.trim().to_string(),
            })
        })
        .take(LIMIT)
        .collect()
}

/// cliphist et les outils wl-clipboard sont-ils là ?
pub fn available() -> bool {
    ["cliphist", "wl-paste", "wl-copy"]
        .iter()
        .all(|p| crate::launch::executable(p))
}

/// Commande du veilleur.
pub fn watcher_exec() -> String {
    "wl-paste --watch cliphist store".into()
}

/// Composant à superviser : réglage activé, cliphist installé, pas d'autre
/// veilleur déjà en route.
pub fn component(enabled: bool) -> Option<ComponentDef> {
    (enabled && available() && foreign_watcher().is_none()).then(|| ComponentDef {
        name: COMPONENT.into(),
        exec: watcher_exec(),
        restart: true,
    })
}

/// `wl-paste … cliphist …` dans une ligne de commande (arguments séparés par NUL).
fn is_watcher(cmdline: &str) -> bool {
    let mut args = cmdline.split('\0').filter(|a| !a.is_empty());
    let Some(arg0) = args.next() else {
        return false;
    };
    arg0.rsplit('/').next() == Some("wl-paste") && args.any(|a| a.contains("cliphist"))
}

/// PID d'un veilleur cliphist qui n'est pas celui de hyprdesk : deux
/// veilleurs enregistreraient chaque copie deux fois.
pub fn foreign_watcher() -> Option<u32> {
    let ours = crate::session::snapshot()
        .into_iter()
        .find(|(d, _)| d.name == COMPONENT)
        .and_then(|(_, s)| s.pid);
    std::fs::read_dir("/proc")
        .ok()?
        .flatten()
        .filter_map(|e| e.file_name().to_str()?.parse::<u32>().ok())
        .filter(|pid| Some(*pid) != ours)
        .find(|pid| {
            std::fs::read_to_string(format!("/proc/{pid}/cmdline")).is_ok_and(|c| is_watcher(&c))
        })
}

/// Historique (bloquant : à appeler hors du fil de l'interface).
pub fn history() -> Vec<Entry> {
    Command::new("cliphist")
        .arg("list")
        .stderr(Stdio::null())
        .output()
        .map(|o| parse(&String::from_utf8_lossy(&o.stdout)))
        .unwrap_or_default()
}

fn pipe(prog: &str, args: &[&str], input: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut child = Command::new(prog)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(input)?;
    }
    let out = child.wait_with_output()?;
    if !out.status.success() {
        return Err(std::io::Error::other(format!("{prog} : {}", out.status)));
    }
    Ok(out.stdout)
}

/// Remet un élément de l'historique dans le presse-papiers (bloquant).
pub fn copy(e: &Entry) -> std::io::Result<()> {
    let data = pipe("cliphist", &["decode"], format!("{}\n", e.line).as_bytes())?;
    pipe("wl-copy", &[], &data).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixe_de_recherche() {
        assert_eq!(query(":clip"), Some(""));
        assert_eq!(query("  :clip  mot de passe "), Some("mot de passe"));
        assert_eq!(query(":clipboard"), None);
        assert_eq!(query("firefox"), None);
    }

    #[test]
    fn sortie_de_cliphist() {
        let e = parse(
            "12\tbonjour  \n11\t[[ binary data 3 KiB png 64x64 ]]\nsans tabulation\nx\tid invalide\n",
        );
        assert_eq!(e.len(), 2);
        assert_eq!(e[0].line, "12\tbonjour  ");
        assert_eq!(e[0].preview, "bonjour");
        assert!(e[1].preview.starts_with("[[ binary"));
    }

    #[test]
    fn veilleur_reconnu() {
        assert!(is_watcher("wl-paste\0--watch\0cliphist\0store\0"));
        assert!(is_watcher(
            "/usr/bin/wl-paste\0--type\0text\0--watch\0cliphist\0store\0"
        ));
        assert!(!is_watcher("wl-paste\0--watch\0other\0"));
        assert!(!is_watcher("cliphist\0store\0"));
        assert!(!is_watcher(""));
    }
}
