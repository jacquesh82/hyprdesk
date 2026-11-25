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
    /// Extension posée sur l'image quand celle-ci est l'icône de
    /// l'application par défaut d'un fichier (HIG ICO-11).
    pub badge: Option<String>,
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
    /// Actions de l'application, dans l'ordre de `Actions=` (MLG-308).
    pub actions: Vec<DesktopAction>,
}

/// Action d'une application (`[Desktop Action <id>]`, listée par `Actions=`) :
/// « Nouvelle fenêtre », « Nouvelle fenêtre privée »… Les menus du dock, de
/// la grille du Lanceur et des icônes du bureau les proposent (MLG-308).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DesktopAction {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub exec: String,
}

/// Lit la section `[Desktop Entry]`, avec le nom localisé si la langue le
/// permet, et les actions qu'elle liste (`[Desktop Action <id>]`).
pub fn parse_desktop_entry(src: &str, lang: &str) -> Option<DesktopEntry> {
    let (lang_full, lang_short) = {
        let base = lang.split('.').next().unwrap_or("");
        (
            base.to_string(),
            base.split('_').next().unwrap_or("").to_string(),
        )
    };
    enum Section {
        Entry,
        Action(usize),
        Other,
    }
    let mut section = Section::Other;
    let mut e = DesktopEntry::default();
    let mut listed: Vec<String> = Vec::new();
    // Actions trouvées, avec leurs noms localisés (complet, court).
    let mut actions: Vec<(DesktopAction, Option<String>, Option<String>)> = Vec::new();
    let (mut name_full, mut name_short) = (None, None);
    for line in src.lines() {
        let line = line.trim();
        if let Some(head) = line.strip_prefix('[') {
            let head = head.trim_end_matches(']');
            section = if head == "Desktop Entry" {
                Section::Entry
            } else if let Some(id) = head.strip_prefix("Desktop Action ") {
                actions.push((
                    DesktopAction {
                        id: id.trim().to_string(),
                        ..Default::default()
                    },
                    None,
                    None,
                ));
                Section::Action(actions.len() - 1)
            } else {
                Section::Other
            };
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let (k, v) = (k.trim(), v.trim());
        let localized = |k: &str, key: &str| -> Option<bool> {
            if !lang_full.is_empty() && k == format!("{key}[{lang_full}]") {
                Some(true)
            } else if !lang_short.is_empty() && k == format!("{key}[{lang_short}]") {
                Some(false)
            } else {
                None
            }
        };
        match section {
            Section::Other => {}
            Section::Action(i) => {
                let (a, full, short) = &mut actions[i];
                match k {
                    "Name" => a.name = v.to_string(),
                    "Icon" => a.icon = Some(v.to_string()).filter(|s| !s.is_empty()),
                    "Exec" => a.exec = v.to_string(),
                    _ => match localized(k, "Name") {
                        Some(true) => *full = Some(v.to_string()),
                        Some(false) => *short = Some(v.to_string()),
                        None => {}
                    },
                }
            }
            Section::Entry => match k {
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
                "Actions" => {
                    listed = v
                        .split(';')
                        .map(str::trim)
                        .filter(|a| !a.is_empty())
                        .map(String::from)
                        .collect()
                }
                "Type" => e.is_app = v == "Application",
                _ => match localized(k, "Name") {
                    Some(true) => name_full = Some(v.to_string()),
                    Some(false) => name_short = Some(v.to_string()),
                    None => {}
                },
            },
        }
    }
    if let Some(n) = name_full.or(name_short) {
        e.name = n;
    }
    // Seules les actions listées par `Actions=` comptent (spécification
    // freedesktop), dans son ordre ; une action sans nom ni commande est ignorée.
    e.actions = listed
        .iter()
        .filter_map(|id| {
            let (a, full, short) = actions.iter().find(|(a, _, _)| &a.id == id)?;
            let mut a = a.clone();
            if let Some(n) = full.clone().or_else(|| short.clone()) {
                a.name = n;
            }
            (!a.name.is_empty() && !a.exec.is_empty()).then_some(a)
        })
        .collect();
    (!e.exec.is_empty()).then_some(e)
}

/// Entrée `.desktop` d'une application installée (par son identifiant), lue
/// dans la langue de l'interface.
pub fn desktop_entry(id: &str) -> Option<DesktopEntry> {
    let src = std::fs::read_to_string(find_desktop_file(id)?).ok()?;
    parse_desktop_entry(&src, &lang())
}

/// Commande d'une action d'application, prête pour `sh -c` : codes de champ
/// retirés, dans un terminal si l'application en demande un.
pub fn action_command(entry: &DesktopEntry, action: &DesktopAction, terminal: &str) -> String {
    let cmd = strip_field_codes(&action.exec);
    if entry.terminal {
        in_terminal(terminal, &cmd)
    } else {
        cmd
    }
}

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

/// Ouvre un chemin (fichier ou dossier) avec l'application par défaut,
/// détaché du lanceur (`xdg-open`). Utilisé par le mode `:f` de la
/// recherche.
pub fn open_path(p: &std::path::Path) -> Result<(), String> {
    spawn_xdg_open(p).map(|_| ())
}

fn spawn_xdg_open(p: &std::path::Path) -> Result<std::process::Child, String> {
    use std::process::{Command, Stdio};
    Command::new("xdg-open")
        .arg(p)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("xdg-open {p:?} : {e}"))
}

/// Comme [`open_path`], mais lit le code de sortie de `xdg-open` (HIG
/// DAT-03) : il rend la main vite quand il délègue (gio, kde-open) ; en
/// environnement générique il attend parfois l'application. Au plus 1,5 s,
/// puis succès présumé, et un fil récupère le processus à sa fin. À appeler
/// hors du fil de l'interface.
pub fn open_path_checked(p: &std::path::Path) -> Result<(), String> {
    use std::time::{Duration, Instant};
    let mut child = spawn_xdg_open(p)?;
    let fin = Instant::now() + Duration::from_millis(1500);
    loop {
        match child.try_wait() {
            Ok(Some(st)) => {
                return match st.code().and_then(xdg_open_error) {
                    Some(e) => Err(format!("{} : {e}", p.display())),
                    None => Ok(()),
                };
            }
            Ok(None) if Instant::now() < fin => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                std::thread::spawn(move || {
                    let _ = child.wait();
                });
                return Ok(());
            }
        }
    }
}

/// Cause d'un code de sortie de `xdg-open` (0 : aucune).
pub fn xdg_open_error(code: i32) -> Option<String> {
    use crate::i18n::tr;
    Some(match code {
        0 => return None,
        1 => tr("erreur dans la commande d'ouverture").to_string(),
        2 => tr("fichier introuvable").to_string(),
        3 => tr("aucune application pour ce type de fichier").to_string(),
        4 => tr("l'ouverture a échoué").to_string(),
        5 => tr("lecture refusée").to_string(),
        c => crate::tr!("xdg-open a rendu le code {c}", c = c),
    })
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

/// Nom d'icône (`Icon=`) de l'application d'une classe de fenêtre : entrée
/// `.desktop` du même nom, ou dont `StartupWMClass` correspond.
pub fn icon_name_for_class(class: &str, apps: &[AppInfo]) -> Option<String> {
    let lc = class.to_lowercase();
    find_desktop_file(class)
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
        })
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
    icon_name_for_class(class, apps)
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
    // SmartUI design actif : ses icônes passent avant le thème, partout.
    if let Some(p) = crate::smartui::themed(name) {
        return Some(p);
    }
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

/// Nom d'icône d'un fichier ou d'un dossier (dossier personnel, dossier, type).
pub fn path_icon_name(p: &Path) -> &'static str {
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

/// Base MIME freedesktop (shared-mime-info), réduite à ce que le bureau en
/// emploie : type d'après l'extension (`globs2`) et icône générique d'un type
/// (`generic-icons`). HIG ICO-11.
#[derive(Debug, Default)]
pub struct MimeDb {
    /// Extension en minuscules (`tar.gz`, `zip`) → (poids, type).
    globs: std::collections::HashMap<String, (u32, String)>,
    /// Motifs à casse respectée (drapeau `cs` : `*.C` est du C++, `*.c` du C).
    globs_cs: std::collections::HashMap<String, (u32, String)>,
    /// Type → icône générique (`application/zip` → `package-x-generic`).
    generic: std::collections::HashMap<String, String>,
}

impl MimeDb {
    /// Lit `globs2` et `generic-icons` dans les dossiers donnés, le plus
    /// prioritaire d'abord : à poids égal, le premier lu l'emporte.
    pub fn read(dirs: &[PathBuf]) -> Self {
        let mut db = Self::default();
        for dir in dirs {
            if let Ok(src) = std::fs::read_to_string(dir.join("globs2")) {
                db.add_globs(&src);
            }
            if let Ok(src) = std::fs::read_to_string(dir.join("generic-icons")) {
                db.add_generic(&src);
            }
        }
        db
    }

    /// Lignes `poids:type:motif[:drapeaux]` ; seuls les motifs `*.ext` sans
    /// autre joker servent. La casse est ignorée, sauf drapeau `cs`.
    fn add_globs(&mut self, src: &str) {
        for line in src.lines().filter(|l| !l.starts_with('#')) {
            let mut f = line.split(':');
            let (Some(w), Some(mime), Some(glob)) = (f.next(), f.next(), f.next()) else {
                continue;
            };
            let cs = f
                .next()
                .is_some_and(|flags| flags.split(',').any(|x| x == "cs"));
            let (Ok(w), Some(ext)) = (w.trim().parse::<u32>(), glob.strip_prefix("*.")) else {
                continue;
            };
            if ext.is_empty() || ext.contains(['*', '?', '[']) {
                continue;
            }
            let (map, ext) = if cs {
                (&mut self.globs_cs, ext.to_string())
            } else {
                (&mut self.globs, ext.to_lowercase())
            };
            match map.get(&ext) {
                Some((old, _)) if *old >= w => {}
                _ => {
                    map.insert(ext, (w, mime.to_string()));
                }
            }
        }
    }

    fn add_generic(&mut self, src: &str) {
        for line in src.lines().filter(|l| !l.starts_with('#')) {
            if let Some((mime, icon)) = line.split_once(':') {
                self.generic
                    .entry(mime.trim().to_string())
                    .or_insert_with(|| icon.trim().to_string());
            }
        }
    }

    /// Type d'un fichier d'après son nom : l'extension composée la plus
    /// longue d'abord (`.tar.gz` avant `.gz`).
    pub fn type_of(&self, path: &Path) -> Option<&str> {
        let name = path.file_name()?.to_str()?;
        let lower = name.to_lowercase();
        // `to_lowercase` garde les positions des points pour un nom ASCII ;
        // ailleurs, la casse respectée est seulement sautée.
        let same = lower.len() == name.len();
        lower
            .match_indices('.')
            .filter(|(i, _)| *i > 0)
            .find_map(|(i, _)| {
                same.then(|| self.globs_cs.get(&name[i + 1..]))
                    .flatten()
                    .or_else(|| self.globs.get(&lower[i + 1..]))
            })
            .map(|(_, m)| m.as_str())
    }

    /// Noms d'icône d'un type, dans l'ordre freedesktop : le type lui-même,
    /// son icône générique, puis `<média>-x-generic`.
    pub fn icon_names(&self, mime: &str) -> Vec<String> {
        let mut out = vec![mime.replace('/', "-")];
        if let Some(g) = self.generic.get(mime) {
            out.push(g.clone());
        }
        if let Some((media, _)) = mime.split_once('/') {
            out.push(format!("{media}-x-generic"));
        }
        let mut seen = std::collections::HashSet::new();
        out.retain(|n| seen.insert(n.clone()));
        out
    }
}

/// Base MIME du système, lue une fois.
fn mime_db() -> &'static MimeDb {
    static DB: std::sync::OnceLock<MimeDb> = std::sync::OnceLock::new();
    DB.get_or_init(|| {
        let dirs: Vec<PathBuf> = data_dirs().into_iter().map(|d| d.join("mime")).collect();
        MimeDb::read(&dirs)
    })
}

/// Nom d'icône de l'application qui ouvre `mime` par défaut (`xdg-mime`,
/// puis l'`Icon` de son `.desktop`). `xdg-mime` est un script : la réponse
/// est gardée par type pour la durée du démon. À appeler hors du fil UI.
fn default_app_icon(mime: &str) -> Option<String> {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<HashMap<String, Option<String>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    if let Some(v) = cache.lock().ok().and_then(|c| c.get(mime).cloned()) {
        return v;
    }
    let icon = std::process::Command::new("xdg-mime")
        .args(["query", "default", mime])
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|id| !id.is_empty())
        .and_then(|id| find_desktop_file(id.trim_end_matches(".desktop")))
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| parse_desktop_entry(&s, &lang()))
        .and_then(|e| e.icon);
    if let Ok(mut c) = cache.lock() {
        c.insert(mime.to_string(), icon.clone());
    }
    icon
}

/// Extension d'un fichier telle qu'elle se pose sur l'icône de l'application
/// par défaut : majuscules, quatre caractères au plus (HIG ICO-11).
pub fn extension_badge(p: &Path) -> Option<String> {
    let ext = p.extension()?.to_str()?;
    let badge: String = ext.chars().take(4).collect::<String>().to_uppercase();
    (!badge.is_empty()).then_some(badge)
}

/// Image d'un fichier ou d'un dossier du bureau, et l'extension à poser
/// dessus quand l'image est celle de l'application par défaut (HIG ICO-11) :
/// icône du type dans le thème, sinon icône de l'application qui l'ouvre,
/// sinon `text-x-generic`.
fn path_image(p: &Path, theme: Option<&str>, size: u16) -> (Option<PathBuf>, Option<String>) {
    let find = |n: &str| lookup_icon(n, theme, size);
    let known = path_icon_name(p);
    // Dossiers, et icônes que SmartUI dessine d'après `path_icon_name` :
    // inchangés.
    if matches!(known, "user-home" | "folder") || crate::smartui::themed(known).is_some() {
        return (find(known).or_else(|| find("folder")), None);
    }
    let db = mime_db();
    if let Some(mime) = db.type_of(p) {
        let mut names = db.icon_names(mime);
        if known != "text-x-generic" && !names.iter().any(|n| n == known) {
            names.push(known.to_string());
        }
        if let Some(img) = names.iter().find_map(|n| find(n)) {
            return (Some(img), None);
        }
        if let Some(img) = default_app_icon(mime).and_then(|n| find(&n)) {
            return (Some(img), extension_badge(p));
        }
    }
    (
        find(known)
            .or_else(|| find("text-x-generic"))
            .or_else(|| find("folder")),
        None,
    )
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
                        badge: None,
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
                    badge: None,
                },
            }
        }
        IconKind::Path(p) => {
            // Icône explicite d'abord ; sinon celle du type (HIG ICO-11).
            let (image, badge) = match def.icon.as_deref() {
                Some(icon) => (find(&[icon, "folder"]), None),
                None => path_image(p, theme, size),
            };
            Resolved {
                label: def.label.clone().unwrap_or_else(|| {
                    p.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_else(|| p.display().to_string())
                }),
                image,
                command: format!("xdg-open {}", sh_quote(&p.display().to_string())),
                badge,
            }
        }
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
            badge: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Codes de sortie de `xdg-open` : 0 n'est pas une erreur, les autres
    /// sont nommés (HIG DAT-03).
    #[test]
    fn codes_xdg_open() {
        assert_eq!(xdg_open_error(0), None);
        for c in 1..=5 {
            assert!(xdg_open_error(c).is_some_and(|e| !e.contains("code")));
        }
        assert!(xdg_open_error(9).is_some_and(|e| e.contains('9')));
    }

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

    /// HIG ICO-11 : le type d'un fichier se lit dans `globs2` (poids, casse,
    /// extension composée) et donne ses noms d'icône dans l'ordre freedesktop.
    #[test]
    fn type_et_icones_d_un_fichier() {
        let mut db = MimeDb::default();
        db.add_globs(
            "# commentaire\n50:application/gzip:*.gz\n50:application/x-compressed-tar:*.tar.gz\n\
             50:text/x-csrc:*.c\n10:text/x-c:*.c\n50:application/x-zip:*.ZIP\n\
             50:text/x-makefile:Makefile\n60:text/x-readme:README*\n\
             50:text/x-c++src:*.C:cs\n",
        );
        db.add_generic("application/x-zip:package-x-generic\n");
        let t = |n: &str| db.type_of(Path::new(n)).map(str::to_string);
        assert_eq!(
            t("/a/b.tar.gz").as_deref(),
            Some("application/x-compressed-tar")
        );
        assert_eq!(t("/a/b.gz").as_deref(), Some("application/gzip"));
        assert_eq!(
            t("/a/b.c").as_deref(),
            Some("text/x-csrc"),
            "poids le plus fort"
        );
        assert_eq!(
            t("/a/B.Zip").as_deref(),
            Some("application/x-zip"),
            "casse ignorée"
        );
        assert_eq!(t("/a/Makefile"), None, "motifs sans extension ignorés");
        assert_eq!(t("/a/b.C").as_deref(), Some("text/x-c++src"), "drapeau cs");
        assert_eq!(t("/a/.gz"), None, "fichier caché sans extension");
        assert_eq!(
            db.icon_names("application/x-zip"),
            [
                "application-x-zip",
                "package-x-generic",
                "application-x-generic"
            ]
        );
        assert_eq!(
            db.icon_names("text/x-csrc"),
            ["text-x-csrc", "text-x-generic"]
        );
        assert_eq!(
            extension_badge(Path::new("/a/notes.md")).as_deref(),
            Some("MD")
        );
        assert_eq!(
            extension_badge(Path::new("/a/x.markdown")).as_deref(),
            Some("MARK")
        );
        assert_eq!(extension_badge(Path::new("/a/Makefile")), None);
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

    #[test]
    fn actions_des_applications() {
        let src = "[Desktop Entry]\nName=Firefox\nExec=firefox %u\nTerminal=false\n\
                   Actions=new-window;new-private-window;absente;\n\n\
                   [Desktop Action new-private-window]\nName=New Private Window\n\
                   Name[fr]=Nouvelle fenêtre de navigation privée\nExec=firefox --private-window %u\n\n\
                   [Desktop Action new-window]\nName=New Window\nName[fr]=Nouvelle fenêtre\n\
                   Icon=window-new\nExec=firefox --new-window %u\n\n\
                   [Desktop Action non-listee]\nName=Cachée\nExec=firefox --x\n";
        let e = parse_desktop_entry(src, "fr_FR.UTF-8").unwrap();
        assert_eq!(e.name, "Firefox");
        let names: Vec<_> = e.actions.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(
            names,
            ["Nouvelle fenêtre", "Nouvelle fenêtre de navigation privée"]
        );
        assert_eq!(e.actions[0].icon.as_deref(), Some("window-new"));
        assert_eq!(
            action_command(&e, &e.actions[1], "kitty"),
            "firefox --private-window"
        );
        let en = parse_desktop_entry(src, "en_US.UTF-8").unwrap();
        assert_eq!(en.actions[0].name, "New Window");
    }
}
