//! Icônes du bureau : entrées `.desktop`, recherche d'images dans le thème
//! d'icônes, et commande à lancer.

use std::path::{Path, PathBuf};

use crate::config::{IconDef, IconKind};

/// Ce qu'une icône de bureau affiche et lance, une fois résolue.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub label: String,
    pub image: Option<PathBuf>,
    pub command: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DesktopEntry {
    pub name: String,
    pub icon: Option<String>,
    pub exec: String,
    pub terminal: bool,
}

/// Lit la section `[Desktop Entry]`, avec le nom localisé si la langue le permet.
pub fn parse_desktop_entry(src: &str, lang: &str) -> Option<DesktopEntry> {
    let (lang_full, lang_short) = {
        let base = lang.split('.').next().unwrap_or("");
        (base.to_string(), base.split('_').next().unwrap_or("").to_string())
    };
    let mut in_section = false;
    let mut e = DesktopEntry::default();
    let (mut name_full, mut name_short) = (None, None);
    for line in src.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_section = line == "[Desktop Entry]";
            continue;
        }
        if !in_section || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim());
        match k {
            "Name" => e.name = v.to_string(),
            "Icon" => e.icon = Some(v.to_string()),
            "Exec" => e.exec = v.to_string(),
            "Terminal" => e.terminal = v == "true",
            _ if !lang_full.is_empty() && k == format!("Name[{lang_full}]") => name_full = Some(v.to_string()),
            _ if !lang_short.is_empty() && k == format!("Name[{lang_short}]") => name_short = Some(v.to_string()),
            _ => {}
        }
    }
    if let Some(n) = name_full.or(name_short) {
        e.name = n;
    }
    (!e.exec.is_empty()).then_some(e)
}

/// Retire les codes de champ (`%u`, `%F`…) d'une ligne Exec ; `%%` devient `%`.
pub fn strip_field_codes(exec: &str) -> String {
    let mut out = String::with_capacity(exec.len());
    let mut chars = exec.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            match chars.next() {
                Some('%') => out.push('%'),
                Some(_) | None => {}
            }
        } else {
            out.push(c);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn data_dirs() -> Vec<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let mut dirs = vec![std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".local/share"))];
    let sys = std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    dirs.extend(sys.split(':').filter(|s| !s.is_empty()).map(PathBuf::from));
    dirs
}

/// Cherche `<id>.desktop`, puis `*.<id>.desktop` (ex. `org.mozilla.firefox` pour `firefox`).
pub fn find_desktop_file(id: &str) -> Option<PathBuf> {
    let apps: Vec<PathBuf> = data_dirs().into_iter().map(|d| d.join("applications")).collect();
    for dir in &apps {
        let p = dir.join(format!("{id}.desktop"));
        if p.is_file() {
            return Some(p);
        }
    }
    let suffix = format!(".{}.desktop", id.to_lowercase());
    for dir in &apps {
        let Ok(rd) = std::fs::read_dir(dir) else { continue };
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.ends_with(&suffix) {
                return Some(entry.path());
            }
        }
    }
    None
}

fn lang() -> String {
    std::env::var("LC_MESSAGES").or_else(|_| std::env::var("LANG")).unwrap_or_default()
}

/// Encadre un argument pour `sh` avec des apostrophes.
pub fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Commande dans un terminal : `{cmd}` dans le gabarit, sinon `<terminal> -e <cmd>`.
pub fn in_terminal(terminal: &str, cmd: &str) -> String {
    if terminal.contains("{cmd}") { terminal.replace("{cmd}", cmd) } else { format!("{terminal} -e {cmd}") }
}

/// Image d'icône : chemin absolu existant, sinon recherche dans le thème.
pub fn lookup_icon(name: &str, theme: Option<&str>, size: u16) -> Option<PathBuf> {
    let p = Path::new(name);
    if p.is_absolute() {
        return p.is_file().then(|| p.to_path_buf());
    }
    let mut q = freedesktop_icons::lookup(name).with_size(size).with_cache();
    if let Some(t) = theme {
        q = q.with_theme(t);
    }
    q.find()
}

fn path_icon_name(p: &Path) -> &'static str {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if home.as_deref() == Some(p) {
        return "user-home";
    }
    if p.is_dir() {
        return "folder";
    }
    match p.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref() {
        Some("png" | "jpg" | "jpeg" | "webp" | "gif" | "svg") => "image-x-generic",
        Some("pdf") => "application-pdf",
        Some("mp3" | "flac" | "ogg" | "wav") => "audio-x-generic",
        Some("mp4" | "mkv" | "webm") => "video-x-generic",
        _ => "text-x-generic",
    }
}

/// Résout une icône déclarée : libellé, image et commande. Les fichiers sont
/// lus ici : à appeler hors du thread UI.
pub fn resolve(def: &IconDef, theme: Option<&str>, size: u16, terminal: &str) -> Resolved {
    let find = |names: &[&str]| names.iter().find_map(|n| lookup_icon(n, theme, size));
    match &def.kind {
        IconKind::App(id) => {
            let entry = find_desktop_file(id).and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| parse_desktop_entry(&s, &lang()));
            match entry {
                Some(e) => {
                    let cmd = strip_field_codes(&e.exec);
                    let icon = def.icon.clone().or(e.icon.clone()).unwrap_or_else(|| id.clone());
                    Resolved {
                        label: def.label.clone().unwrap_or(e.name),
                        image: find(&[icon.as_str(), id.as_str(), "application-x-executable"]),
                        command: if e.terminal { in_terminal(terminal, &cmd) } else { cmd },
                    }
                }
                // Application introuvable : on tente quand même l'identifiant comme commande.
                None => Resolved {
                    label: def.label.clone().unwrap_or_else(|| id.clone()),
                    image: find(&[def.icon.as_deref().unwrap_or(id), "application-x-executable"]),
                    command: id.clone(),
                },
            }
        }
        IconKind::Path(p) => Resolved {
            label: def.label.clone().unwrap_or_else(|| {
                p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
            }),
            image: find(&[def.icon.as_deref().unwrap_or(path_icon_name(p)), "folder"]),
            command: format!("xdg-open {}", sh_quote(&p.display().to_string())),
        },
        IconKind::Exec { cmd, terminal: in_term } => Resolved {
            label: def.label.clone().unwrap_or_else(|| cmd.split_whitespace().next().unwrap_or(cmd).to_string()),
            image: find(&[def.icon.as_deref().unwrap_or("utilities-terminal"), "application-x-executable"]),
            command: if *in_term { in_terminal(terminal, cmd) } else { cmd.clone() },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIREFOX: &str = "[Desktop Entry]\nName=Firefox\nName[fr]=Navigateur Firefox\nExec=/usr/lib/firefox/firefox %u\nIcon=firefox\nType=Application\n\n[Desktop Action new-window]\nName=New Window\nExec=firefox --new-window %u\n";

    #[test]
    fn entree_desktop_localisee() {
        let e = parse_desktop_entry(FIREFOX, "fr_FR.UTF-8").unwrap();
        assert_eq!(e.name, "Navigateur Firefox");
        assert_eq!(e.exec, "/usr/lib/firefox/firefox %u");
        assert_eq!(e.icon.as_deref(), Some("firefox"));
        assert!(!e.terminal);
        // Les actions ne remplacent pas l'entrée principale.
        assert_eq!(parse_desktop_entry(FIREFOX, "en_US.UTF-8").unwrap().name, "Firefox");
    }

    #[test]
    fn codes_de_champ() {
        assert_eq!(strip_field_codes("firefox %u"), "firefox");
        assert_eq!(strip_field_codes("app --x=%% %F --y"), "app --x=% --y");
    }

    #[test]
    fn terminal_et_guillemets() {
        assert_eq!(in_terminal("kitty", "htop"), "kitty -e htop");
        assert_eq!(in_terminal("foot {cmd}", "htop"), "foot htop");
        assert_eq!(sh_quote("l'été"), r"'l'\''été'");
    }
}
