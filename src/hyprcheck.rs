//! Ménage de la configuration de Hyprland (Réglages → Configuration de
//! Hyprland) : incohérences et conflits de `hyprland.lua`, chacun avec sa
//! correction quand elle est sûre.
//!
//! - erreurs signalées par Hyprland (`hyprctl configerrors`) ;
//! - raccourcis en double (même combinaison sur deux lignes actives) ;
//! - démarrages en double, ou d'un programme absent ;
//! - programmes en doublon avec ce que hyprdesk fait lui-même (fond d'écran,
//!   barre, notifications, veille, agent polkit, presse-papiers) ;
//! - `require` d'un fichier absent ;
//! - lignes déjà commentées par hyprdesk (« géré par hyprdesk ») ;
//! - copies de sauvegarde accumulées (la plus récente de chaque sorte reste).
//!
//! Les corrections commentent ou suppriment une ligne (après une copie de
//! hyprland.lua), ou suppriment des copies ; une ligne qui a changé entre
//! l'analyse et la correction n'est pas touchée.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Error,
    DuplicateBind,
    DuplicateStart,
    MissingProgram,
    MissingRequire,
    Managed,
    StaleComment,
    Backups,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Error => "Erreur de configuration",
            Kind::DuplicateBind => "Raccourci en double",
            Kind::DuplicateStart => "Démarrage en double",
            Kind::MissingProgram => "Programme absent",
            Kind::MissingRequire => "Fichier requis absent",
            Kind::Managed => "Fait par hyprdesk",
            Kind::StaleComment => "Ligne commentée par hyprdesk",
            Kind::Backups => "Copies de sauvegarde",
        }
    }
}

/// Correction proposée.
#[derive(Debug, Clone, PartialEq)]
pub enum Fix {
    /// Mettre en commentaire la ligne (0 = première), si elle vaut toujours ce texte.
    Comment(usize, String),
    /// Supprimer la ligne, si elle vaut toujours ce texte.
    Delete(usize, String),
    /// Supprimer ces fichiers.
    DeleteFiles(Vec<PathBuf>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Finding {
    pub kind: Kind,
    /// Ligne dans hyprland.lua (à partir de 1).
    pub line: Option<usize>,
    pub excerpt: String,
    pub detail: String,
    pub fix: Option<Fix>,
    /// Correction sûre, comprise dans « Tout ranger ».
    pub safe: bool,
}

/// Ce que hyprdesk fait lui-même : programmes en doublon, et pourquoi.
pub type Managed = Vec<(&'static [&'static str], &'static str)>;

/// Programmes que hyprdesk remplace, selon ce qui est actif.
pub fn managed(
    wallpaper: bool,
    bar: bool,
    notifications: bool,
    idle: bool,
    clipboard: bool,
) -> Managed {
    let mut v: Managed = vec![(
        &[
            "polkit-kde-authentication-agent-1",
            "polkit-gnome-authentication-agent-1",
            "hyprpolkitagent",
            "lxpolkit",
            "polkit-mate-authentication-agent-1",
        ],
        "hyprdesk est l'agent polkit de la session",
    )];
    if wallpaper {
        v.push((&["hyprpaper", "swaybg", "swww-daemon", "wpaperd"], "hyprdesk dessine le fond d'écran"));
    }
    if bar {
        v.push((&["waybar", "ags", "eww"], "la barre de hyprdesk est active"));
    }
    if notifications {
        v.push((&["mako", "dunst", "swaync", "fnott", "wayle"], "hyprdesk affiche les notifications"));
    }
    if idle {
        v.push((&["hypridle", "swayidle"], "hyprdesk lance et règle la veille (hypridle)"));
    }
    if clipboard {
        v.push((&["cliphist"], "hyprdesk tient l'historique du presse-papiers"));
    }
    v
}

/// Programme lancé par une commande (`sh -c '…'` compris, variables
/// d'environnement et `env` écartés).
pub fn program_of(cmd: &str) -> Option<String> {
    let mut c = cmd.trim();
    for sh in ["sh -c ", "bash -c ", "zsh -c "] {
        if let Some(rest) = c.strip_prefix(sh) {
            c = rest.trim().trim_matches(|ch| ch == '\'' || ch == '"');
        }
    }
    let first = c.split([';', '&', '|']).next()?.trim();
    first
        .split_whitespace()
        .find(|t| *t != "env" && !t.contains('=') && *t != "exec")
        .map(|t| t.trim_matches(|ch| ch == '\'' || ch == '"').to_string())
}

/// Ligne de code mise en commentaire par hyprdesk (takeover, installation) ;
/// les commentaires explicatifs (« … géré par hyprdesk ; anciens réglages »)
/// restent : ce ne sont pas des lignes mortes.
fn hyprdesk_comment(line: &str) -> bool {
    let t = line.trim_start();
    let code = t
        .strip_prefix("--")
        .map(str::trim_start)
        .is_some_and(|c| c.starts_with("hl.") || c.starts_with("exec") || c.starts_with("require("));
    code
        && ["géré par hyprdesk", "gérée par hyprdesk", "gérées par hyprdesk", "retiré : hyprdesk"]
            .iter()
            .any(|m| t.contains(m))
}

/// Analyse de hyprland.lua : `dir` son dossier (require), `exists` : le
/// programme est-il installé.
pub fn analyse(
    src: &str,
    dir: &Path,
    managed: &Managed,
    exists: impl Fn(&str) -> bool,
) -> Vec<Finding> {
    let lines: Vec<&str> = src.lines().collect();
    let mut out = Vec::new();
    // Raccourcis en double.
    let mut seen: HashMap<(u32, String), usize> = HashMap::new();
    for b in crate::keymap::parse_source(src) {
        match seen.get(&b.combo) {
            Some(first) => {
                let text = lines.get(b.line - 1).copied().unwrap_or_default();
                out.push(Finding {
                    kind: Kind::DuplicateBind,
                    line: Some(b.line),
                    excerpt: text.trim().to_string(),
                    detail: crate::tr!(
                        "même combinaison qu'à la ligne {l} : seule la dernière agit",
                        l = first
                    ),
                    fix: Some(Fix::Comment(b.line - 1, text.to_string())),
                    safe: false,
                });
            }
            None => {
                seen.insert(b.combo.clone(), b.line);
            }
        }
    }
    // Démarrages.
    let mut started: BTreeMap<String, usize> = BTreeMap::new();
    for it in crate::session_start::scan_hyprland(src)
        .into_iter()
        .filter(|i| i.enabled && !i.locked)
    {
        let text = lines.get(it.line).copied().unwrap_or_default().to_string();
        if let Some(first) = started.get(&it.command) {
            out.push(Finding {
                kind: Kind::DuplicateStart,
                line: Some(it.line + 1),
                excerpt: it.command.clone(),
                detail: crate::tr!("déjà lancé à la ligne {l}", l = first + 1),
                fix: Some(Fix::Comment(it.line, text)),
                safe: true,
            });
            continue;
        }
        started.insert(it.command.clone(), it.line);
        let Some(prog) = program_of(&it.command) else {
            continue;
        };
        // Tous les mots de la commande (`sh -c 'sleep 5 && hyprpaper'`).
        let words: Vec<&str> = it
            .command
            .split(|c: char| c.is_whitespace() || "'\";&|".contains(c))
            .filter(|w| !w.is_empty())
            .map(|w| w.rsplit('/').next().unwrap_or(w))
            .collect();
        if let Some((_, why)) = managed
            .iter()
            .find(|(names, _)| names.iter().any(|n| words.contains(n)))
        {
            out.push(Finding {
                kind: Kind::Managed,
                line: Some(it.line + 1),
                excerpt: it.command.clone(),
                detail: crate::i18n::tr(why).to_string(),
                fix: Some(Fix::Comment(it.line, text.clone())),
                safe: true,
            });
        } else if !exists(&prog) {
            out.push(Finding {
                kind: Kind::MissingProgram,
                line: Some(it.line + 1),
                excerpt: it.command.clone(),
                detail: crate::tr!("« {p} » n'est pas installé", p = prog),
                fix: Some(Fix::Comment(it.line, text)),
                safe: true,
            });
        }
    }
    for (n, l) in lines.iter().enumerate() {
        let t = l.trim_start();
        // require d'un fichier absent.
        if !t.starts_with("--")
            && let Some(rest) = t.strip_prefix("require(")
            && let Some(name) = rest
                .trim_start_matches(['"', '\''])
                .split(['"', '\''])
                .next()
            && !name.is_empty()
        {
            let file = dir.join(format!("{}.lua", name.replace('.', "/")));
            if !file.is_file() {
                out.push(Finding {
                    kind: Kind::MissingRequire,
                    line: Some(n + 1),
                    excerpt: t.to_string(),
                    detail: crate::tr!("{f} n'existe pas : Hyprland s'arrête sur cette ligne", f = file.display()),
                    fix: Some(Fix::Comment(n, l.to_string())),
                    safe: false,
                });
            }
        }
        if hyprdesk_comment(l) {
            out.push(Finding {
                kind: Kind::StaleComment,
                line: Some(n + 1),
                excerpt: t.to_string(),
                detail: crate::i18n::tr("déjà coupée : la ligne peut partir").to_string(),
                fix: Some(Fix::Delete(n, l.to_string())),
                safe: true,
            });
        }
    }
    out.sort_by_key(|f| (f.kind, f.line));
    out
}

/// Copies de sauvegarde en trop du dossier : par sorte (`hyprland.lua.avant-hyprdesk-lock.bak`…),
/// la plus récente reste.
pub fn old_backups(dir: &Path) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut by_kind: BTreeMap<String, Vec<(std::time::SystemTime, PathBuf)>> = BTreeMap::new();
    for e in rd.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        if !(name.ends_with(".bak") || name.contains(".bak-") || name.contains(".bak.")) {
            continue;
        }
        // Sorte : le nom sans horodatage ni chiffres.
        let kind: String = name.chars().filter(|c| !c.is_ascii_digit()).collect();
        let t = e.metadata().and_then(|m| m.modified()).unwrap_or(std::time::UNIX_EPOCH);
        by_kind.entry(kind).or_default().push((t, e.path()));
    }
    let mut out = Vec::new();
    for (_, mut v) in by_kind {
        v.sort();
        v.pop();
        out.extend(v.into_iter().map(|(_, p)| p));
    }
    out.sort();
    out
}

/// Erreurs de configuration en cours, d'après Hyprland.
pub fn config_errors() -> Vec<String> {
    std::process::Command::new("hyprctl")
        .arg("configerrors")
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Relevé complet : erreurs, analyse de hyprland.lua, copies en trop.
pub fn full(config_path: &Path, managed: &Managed) -> Vec<Finding> {
    let hypr = crate::session_start::hyprland_path(config_path);
    let dir = hypr.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut out: Vec<Finding> = config_errors()
        .into_iter()
        .map(|e| Finding {
            kind: Kind::Error,
            line: None,
            excerpt: e,
            detail: String::new(),
            fix: None,
            safe: false,
        })
        .collect();
    if let Ok(src) = std::fs::read_to_string(&hypr) {
        out.extend(analyse(&src, &dir, managed, crate::launch::executable));
    }
    let old = old_backups(&dir);
    if !old.is_empty() {
        out.push(Finding {
            kind: Kind::Backups,
            line: None,
            excerpt: crate::tr!("{n} copie(s) dans {d}", n = old.len(), d = dir.display()),
            detail: crate::i18n::tr("la plus récente de chaque sorte est gardée").to_string(),
            fix: Some(Fix::DeleteFiles(old)),
            safe: true,
        });
    }
    out
}

/// Applique des corrections ; hyprland.lua est copié avant (une copie par
/// passage). Rend le nombre de corrections faites.
pub fn apply(config_path: &Path, fixes: &[Fix]) -> Result<usize, String> {
    let hypr = crate::session_start::hyprland_path(config_path);
    let mut done = 0;
    let line_fixes: Vec<&Fix> = fixes
        .iter()
        .filter(|f| matches!(f, Fix::Comment(..) | Fix::Delete(..)))
        .collect();
    if !line_fixes.is_empty() {
        let src = std::fs::read_to_string(&hypr).map_err(|e| e.to_string())?;
        let mut lines: Vec<Option<String>> = src.lines().map(|l| Some(l.to_string())).collect();
        for f in &line_fixes {
            match f {
                Fix::Comment(i, text) => {
                    if let Some(Some(l)) = lines.get_mut(*i)
                        && l == text
                    {
                        let t = l.trim_start();
                        let indent = &l[..l.len() - t.len()];
                        *l = format!("{indent}-- {t} -- hyprdesk : ménage");
                        done += 1;
                    }
                }
                Fix::Delete(i, text) => {
                    if lines.get(*i).is_some_and(|l| l.as_deref() == Some(text.as_str())) {
                        lines[*i] = None;
                        done += 1;
                    }
                }
                Fix::DeleteFiles(_) => {}
            }
        }
        if done > 0 {
            let bak = hypr.with_extension("lua.avant-hyprdesk-menage.bak");
            std::fs::write(&bak, &src).map_err(|e| e.to_string())?;
            let mut out: String = lines.into_iter().flatten().collect::<Vec<_>>().join("\n");
            if src.ends_with('\n') {
                out.push('\n');
            }
            std::fs::write(&hypr, out).map_err(|e| e.to_string())?;
        }
    }
    for f in fixes {
        if let Fix::DeleteFiles(files) = f {
            for p in files {
                if std::fs::remove_file(p).is_ok() {
                    done += 1;
                }
            }
        }
    }
    Ok(done)
}

/// Relevé en texte, pour LYRA.
pub fn report(findings: &[Finding]) -> String {
    let mut s = String::new();
    for f in findings {
        let line = f.line.map(|l| crate::tr!("ligne {l} : ", l = l)).unwrap_or_default();
        s += &format!("- {} — {line}{}", crate::i18n::tr(f.kind.label()), f.excerpt);
        if !f.detail.is_empty() {
            s += &format!(" ({})", f.detail);
        }
        s.push('\n');
    }
    crate::tr!(
        "Voici le relevé de ma configuration Hyprland (~/.config/hypr/hyprland.lua) fait par hyprdesk :\n{r}\nExplique-moi chaque point, puis propose les corrections les plus sûres, une par une. Demande-moi avant de modifier quoi que ce soit, et ne touche jamais à inputs.lua, monitors.lua ni autostart.lua (tenus par hyprdesk).",
        r = s
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = r#"local mainMod = "SUPER"
require("monitors")
require("absent")
hl.bind(mainMod .. " + R", hl.dsp.exec_cmd("rofi"))
hl.bind("SUPER + R", hl.dsp.exec_cmd("wofi"))
hl.on("hyprland.start", function()
    hl.exec_cmd("waybar")
    hl.exec_cmd("nm-applet")
    hl.exec_cmd("nm-applet")
    hl.exec_cmd("programme-inconnu --x")
    hl.exec_cmd("sh -c 'sleep 5 && hyprpaper'")
    hl.exec_cmd("sh -c 'wl-paste --type text --watch cliphist store'")
    -- hl.exec_cmd("hypridle") -- veille gérée par hyprdesk (idle_manage)
    -- éclairage nocturne géré par hyprdesk : anciens réglages 5200 K
end)
"#;

    #[test]
    fn releve_de_hyprland_lua() {
        let dir = std::env::temp_dir().join(format!("hyprcheck-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("monitors.lua"), "").unwrap();
        let m = managed(true, true, true, true, true);
        let f = analyse(SRC, &dir, &m, |p| p != "programme-inconnu");
        let kinds: Vec<(Kind, Option<usize>)> = f.iter().map(|f| (f.kind, f.line)).collect();
        assert!(kinds.contains(&(Kind::DuplicateBind, Some(5))), "{kinds:?}");
        assert!(kinds.contains(&(Kind::DuplicateStart, Some(9))), "{kinds:?}");
        assert!(kinds.contains(&(Kind::MissingProgram, Some(10))), "{kinds:?}");
        assert!(kinds.contains(&(Kind::Managed, Some(7))), "waybar : {kinds:?}");
        assert!(kinds.contains(&(Kind::Managed, Some(11))), "hyprpaper : {kinds:?}");
        assert!(kinds.contains(&(Kind::Managed, Some(12))), "cliphist : {kinds:?}");
        assert!(kinds.contains(&(Kind::MissingRequire, Some(3))), "{kinds:?}");
        assert!(!kinds.contains(&(Kind::MissingRequire, Some(2))));
        assert!(kinds.contains(&(Kind::StaleComment, Some(13))), "{kinds:?}");
        // Commentaire explicatif : gardé.
        assert!(!kinds.contains(&(Kind::StaleComment, Some(14))), "{kinds:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn corrections_sur_le_fichier() {
        let dir = std::env::temp_dir().join(format!("hyprcheck-fix-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let cfg = dir.join("hyprdesk.lua");
        std::fs::write(dir.join("hyprland.lua"), SRC).unwrap();
        let lines: Vec<&str> = SRC.lines().collect();
        let n = apply(
            &cfg,
            &[
                Fix::Comment(8, lines[8].to_string()),
                Fix::Delete(12, lines[12].to_string()),
                // Ligne qui a changé : pas touchée.
                Fix::Comment(6, "autre chose".into()),
            ],
        )
        .unwrap();
        assert_eq!(n, 2);
        let out = std::fs::read_to_string(dir.join("hyprland.lua")).unwrap();
        assert!(out.contains("    -- hl.exec_cmd(\"nm-applet\") -- hyprdesk : ménage"));
        assert!(!out.contains("idle_manage"));
        assert!(out.contains("    hl.exec_cmd(\"waybar\")"));
        assert!(dir.join("hyprland.lua.avant-hyprdesk-menage.bak").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn programme_d_une_commande() {
        assert_eq!(program_of("nm-applet --indicator").as_deref(), Some("nm-applet"));
        assert_eq!(
            program_of("sh -c 'sleep 5 && hyprpaper'").as_deref(),
            Some("sleep")
        );
        assert_eq!(program_of("env GDK_BACKEND=x11 slack").as_deref(), Some("slack"));
    }
}
