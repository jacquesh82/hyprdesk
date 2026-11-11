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
    /// `TryExec` : programme dont la présence dit si l'application est installée.
    pub try_exec: Option<String>,
    pub terminal: bool,
    /// `NoDisplay` ou `Hidden` : à ne pas proposer dans une liste d'applications.
    pub hidden: bool,
    pub wm_class: Option<String>,
    pub is_app: bool,
    /// Types de fichiers ouverts (`MimeType`).
    pub mime_types: Vec<String>,
    pub categories: Vec<String>,
}

/// Lit la section `[Desktop Entry]`, avec le nom localisé si la langue le permet.
pub fn parse_desktop_entry(src: &str, lang: &str) -> Option<DesktopEntry> {
    let (lang_full, lang_short) = {
        let base = lang.split('.').next().unwrap_or("");
        (
            base.to_string(),
            base.split('_').next().unwrap_or("").to_string(),
        )
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
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let (k, v) = (k.trim(), v.trim());
        match k {
            "Name" => e.name = v.to_string(),
            "Icon" => e.icon = Some(v.to_string()),
            "Exec" => e.exec = v.to_string(),
            "TryExec" => e.try_exec = Some(v.to_string()),
            "Terminal" => e.terminal = v == "true",
            "NoDisplay" | "Hidden" => e.hidden |= v == "true",
            "StartupWMClass" => e.wm_class = Some(v.to_string()),
            "MimeType" => {
                e.mime_types = v
                    .split(';')
                    .filter(|m| !m.is_empty())
                    .map(String::from)
                    .collect()
            }
            "Categories" => {
                e.categories = v
                    .split(';')
                    .filter(|c| !c.is_empty())
                    .map(String::from)
                    .collect()
            }
            "Type" => e.is_app = v == "Application",
            _ if !lang_full.is_empty() && k == format!("Name[{lang_full}]") => {
                name_full = Some(v.to_string())
            }
            _ if !lang_short.is_empty() && k == format!("Name[{lang_short}]") => {
                name_short = Some(v.to_string())
            }
            _ => {}
        }
    }
    if let Some(n) = name_full.or(name_short) {
        e.name = n;
    }
    (!e.exec.is_empty()).then_some(e)
}

/// Programme exécutable : chemin absolu exécutable, ou nom trouvé dans le PATH.
pub fn executable(prog: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let runnable = |p: &std::path::Path| {
        p.metadata()
            .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    };
    if prog.contains('/') {
        return runnable(std::path::Path::new(prog));
    }
    std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| runnable(&d.join(prog))))
}

/// Premier mot d'une ligne Exec (guillemets compris), après `env VAR=…`.
pub fn exec_program(exec: &str) -> Option<String> {
    let mut words = Vec::new();
    let mut rest = exec.trim();
    while !rest.is_empty() {
        let (word, tail) = if let Some(q) = rest.strip_prefix('"') {
            match q.find('"') {
                Some(end) => (&q[..end], &q[end + 1..]),
                None => (q, ""),
            }
        } else {
            match rest.find(char::is_whitespace) {
                Some(end) => (&rest[..end], &rest[end..]),
                None => (rest, ""),
            }
        };
        words.push(word.to_string());
        rest = tail.trim_start();
        let first = &words[0];
        let skip = first == "env" || first.ends_with("/env");
        // `env A=1 B=2 prog …` : le programme est le premier mot sans `=`.
        if !(skip && (words.len() == 1 || words.last().is_some_and(|w| w.contains('=')))) {
            break;
        }
    }
    words.pop().filter(|w| !w.is_empty())
}

impl DesktopEntry {
    /// Le programme de l'entrée est présent : `TryExec` s'il est donné, sinon
    /// le premier mot de `Exec`. Une entrée restée après une désinstallation
    /// (fréquent dans `~/.local/share/applications`) ne l'est pas.
    pub fn available(&self) -> bool {
        if let Some(t) = &self.try_exec
            && let Some(p) = exec_program(t)
            && !executable(&p)
        {
            return false;
        }
        exec_program(&self.exec).is_some_and(|p| executable(&p))
    }
}

/// Retire les codes de champ (`%u`, `%F`…) d'une ligne Exec ; `%%` devient `%`.
pub fn strip_field_codes(exec: &str) -> String {
    let mut out = String::with_capacity(exec.len());
    let mut chars = exec.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            if let Some('%') = chars.next() {
                out.push('%');
            }
        } else {
            out.push(c);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn data_dirs() -> Vec<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let mut dirs = vec![
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share")),
    ];
    let sys =
        std::env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());
    dirs.extend(sys.split(':').filter(|s| !s.is_empty()).map(PathBuf::from));
    dirs
}

/// Cherche `<id>.desktop`, puis `*.<id>.desktop` (ex. `org.mozilla.firefox` pour `firefox`).
pub fn find_desktop_file(id: &str) -> Option<PathBuf> {
    let apps: Vec<PathBuf> = data_dirs()
        .into_iter()
        .map(|d| d.join("applications"))
        .collect();
    for dir in &apps {
        let p = dir.join(format!("{id}.desktop"));
        if p.is_file() {
            return Some(p);
        }
    }
    let suffix = format!(".{}.desktop", id.to_lowercase());
    for dir in &apps {
        let Ok(rd) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.ends_with(&suffix) {
                return Some(entry.path());
            }
        }
    }
    None
}

/// Application installée, pour la liste « ajouter un raccourci ».
#[derive(Debug, Clone, PartialEq)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub wm_class: Option<String>,
    /// Types de fichiers ouverts et catégories, pour les applications par défaut.
    pub mime_types: Vec<String>,
    pub categories: Vec<String>,
    /// Programme présent sur la machine (`DesktopEntry::available`).
    pub available: bool,
}

/// Applications visibles des dossiers XDG ; le dossier utilisateur masque le système.
pub fn installed_apps() -> Vec<AppInfo> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    let lang = lang();
    for dir in data_dirs().into_iter().map(|d| d.join("applications")) {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            let Some(id) = path
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|n| n.strip_suffix(".desktop"))
            else {
                continue;
            };
            if !seen.insert(id.to_string()) {
                continue;
            }
            let Some(e) = std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| parse_desktop_entry(&s, &lang))
            else {
                continue;
            };
            if e.hidden || !e.is_app {
                continue;
            }
            let available = e.available();
            out.push(AppInfo {
                available,
                id: id.to_string(),
                name: e.name,
                icon: e.icon,
                wm_class: e.wm_class,
                mime_types: e.mime_types,
                categories: e.categories,
            });
        }
    }
    out.sort_by_key(|a| a.name.to_lowercase());
    out
}

/// Image d'icône d'une fenêtre d'après sa classe : entrée `.desktop` du même
/// nom, ou dont `StartupWMClass` correspond, sinon le nom de classe lui-même.
pub fn icon_for_class(
    class: &str,
    apps: &[AppInfo],
    theme: Option<&str>,
    size: u16,
) -> Option<PathBuf> {
    let lc = class.to_lowercase();
    let from_entry = find_desktop_file(class)
        .or_else(|| find_desktop_file(&lc))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| parse_desktop_entry(&s, ""))
        .and_then(|e| e.icon)
        .or_else(|| {
            apps.iter()
                .find(|a| {
                    a.wm_class
                        .as_deref()
                        .is_some_and(|w| w.eq_ignore_ascii_case(class))
                        || a.id.to_lowercase() == lc
                })
                .and_then(|a| a.icon.clone())
        });
    from_entry
        .and_then(|i| lookup_icon(&i, theme, size))
        .or_else(|| lookup_icon(&lc, theme, size))
        .or_else(|| lookup_icon("application-x-executable", theme, size))
}

/// Thèmes d'icônes installés (hors thèmes de curseurs seuls).
pub fn icon_themes() -> Vec<String> {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let mut dirs: Vec<PathBuf> = data_dirs().into_iter().map(|d| d.join("icons")).collect();
    dirs.push(home.join(".icons"));
    let mut out: Vec<String> = Vec::new();
    for d in dirs {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let idx = e.path().join("index.theme");
            let Ok(s) = std::fs::read_to_string(&idx) else {
                continue;
            };
            if !s
                .lines()
                .any(|l| l.trim_start().starts_with("Directories="))
            {
                continue;
            }
            let name = e.file_name().to_string_lossy().into_owned();
            if !out.contains(&name) {
                out.push(name);
            }
        }
    }
    out.sort_by_key(|s| s.to_lowercase());
    out
}

fn lang() -> String {
    std::env::var("LC_MESSAGES")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default()
}

/// Familles installées et police embarquée : aucun téléchargement implicite.
pub fn font_families() -> Vec<String> {
    let mut families = std::process::Command::new("fc-list")
        .args(["--format", "%{family[0]}\n"])
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    families.extend([
        crate::fonts::FAMILY.into(),
        "Noto Sans".into(),
        "sans-serif".into(),
    ]);
    families.sort_by_key(|f| f.to_lowercase());
    families.dedup();
    families
}

/// Encadre un argument pour `sh` avec des apostrophes.
pub fn sh_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

/// Commande dans un terminal : `{cmd}` dans le gabarit, sinon `<terminal> -e <cmd>`.
pub fn in_terminal(terminal: &str, cmd: &str) -> String {
    if terminal.contains("{cmd}") {
        terminal.replace("{cmd}", cmd)
    } else {
        format!("{terminal} -e {cmd}")
    }
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
    match p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .as_deref()
    {
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
            let entry = find_desktop_file(id)
                .and_then(|p| std::fs::read_to_string(p).ok())
                .and_then(|s| parse_desktop_entry(&s, &lang()));
            match entry {
                Some(e) => {
                    let cmd = strip_field_codes(&e.exec);
                    let icon = def
                        .icon
                        .clone()
                        .or(e.icon.clone())
                        .unwrap_or_else(|| id.clone());
                    Resolved {
                        label: def.label.clone().unwrap_or(e.name),
                        image: find(&[icon.as_str(), id.as_str(), "application-x-executable"]),
                        command: if e.terminal {
                            in_terminal(terminal, &cmd)
                        } else {
                            cmd
                        },
                    }
                }
                // Application introuvable : on tente quand même l'identifiant comme commande.
                None => Resolved {
                    label: def.label.clone().unwrap_or_else(|| id.clone()),
                    image: find(&[
                        def.icon.as_deref().unwrap_or(id),
                        "application-x-executable",
                    ]),
                    command: id.clone(),
                },
            }
        }
        IconKind::Path(p) => Resolved {
            label: def.label.clone().unwrap_or_else(|| {
                p.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| p.display().to_string())
            }),
            image: find(&[def.icon.as_deref().unwrap_or(path_icon_name(p)), "folder"]),
            command: format!("xdg-open {}", sh_quote(&p.display().to_string())),
        },
        IconKind::Exec {
            cmd,
            terminal: in_term,
        } => Resolved {
            label: def
                .label
                .clone()
                .unwrap_or_else(|| cmd.split_whitespace().next().unwrap_or(cmd).to_string()),
            image: find(&[
                def.icon.as_deref().unwrap_or("utilities-terminal"),
                "application-x-executable",
            ]),
            command: if *in_term {
                in_terminal(terminal, cmd)
            } else {
                cmd.clone()
            },
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
        assert!(e.is_app && !e.hidden);
        // Les actions ne remplacent pas l'entrée principale.
        assert_eq!(
            parse_desktop_entry(FIREFOX, "en_US.UTF-8").unwrap().name,
            "Firefox"
        );
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

    #[test]
    fn programme_d_une_entree() {
        assert_eq!(exec_program("firefox %u").as_deref(), Some("firefox"));
        assert_eq!(
            exec_program("\"/opt/My App/run\" --x").as_deref(),
            Some("/opt/My App/run")
        );
        assert_eq!(
            exec_program("env GDK_BACKEND=x11 A=1 slack %U").as_deref(),
            Some("slack")
        );
        let e = |exec: &str, try_exec: Option<&str>| DesktopEntry {
            exec: exec.into(),
            try_exec: try_exec.map(String::from),
            ..Default::default()
        };
        assert!(e("sh -c true", None).available());
        assert!(!e("/usr/bin/hyprdesk-absent-xyz %u", None).available());
        assert!(!e("sh", Some("hyprdesk-absent-xyz")).available());
    }
}
