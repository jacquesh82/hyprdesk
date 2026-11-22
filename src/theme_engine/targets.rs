//! Cibles (§8 de la spécification) : chaque application qui reçoit le thème.
//!
//! Le contrat reprend celui de `crate::apptheme` (une cible = fichiers,
//! rechargement, présence) avec des opérations déclaratives : chaque cible
//! planifie des [`Planned`] (chemin, contenu, opération), l'application
//! transactionnelle de `apply.rs` les exécute avec retour arrière.
//!
//! Note d'intégration hyprdesk : dans une session hyprdesk, les couleurs des
//! fenêtres (bordures, ombre) sont déjà portées par le démon via
//! `appearance.lua` (API `hl.config`). Le fragment `hyprland` du moteur reste
//! un fragment hyprlang pour les configurations standard (`source=`), et le
//! mode `hot` passe par `hyprctl keyword` dans les deux mondes.

use std::path::{Path, PathBuf};
use std::process::Command;

use super::color::Fmt;
use super::paths::Roots;
use super::resolve::Resolved;
use super::template::{self, Context};
use super::{Error, Result};

/// Une cible : identifiant stable, gabarit intégré, sortie par défaut.
pub struct Target {
    pub id: &'static str,
    pub label: &'static str,
    pub summary: &'static str,
    pub default_fmt: Fmt,
    pub builtin_template: &'static str,
    /// Présence du logiciel dans le `PATH`.
    pub installed: fn() -> bool,
}

/// Registre des cibles, dans l'ordre d'application.
pub const TARGETS: &[Target] = &[
    Target {
        id: "hyprland",
        label: "Hyprland",
        summary: "Fragment hyprlang : bordures, ombre, options du pack ; inclus par source=.",
        default_fmt: Fmt::Hypr,
        builtin_template: include_str!("templates/hyprland.conf.tmpl"),
        installed: || in_path("hyprctl"),
    },
    Target {
        id: "hyprlock",
        label: "Hyprlock",
        summary: "Variables $hd_* pour hyprlock.conf, inclus par source=.",
        default_fmt: Fmt::Hypr,
        builtin_template: include_str!("templates/hyprlock.conf.tmpl"),
        installed: || in_path("hyprlock"),
    },
    Target {
        id: "kitty",
        label: "Kitty",
        summary: "Fichier de couleurs inclus en fin de kitty.conf, relu par SIGUSR1.",
        default_fmt: Fmt::Hex,
        builtin_template: include_str!("templates/kitty.conf.tmpl"),
        installed: || in_path("kitty"),
    },
    Target {
        id: "gtk",
        label: "GTK",
        summary: "Couleurs nommées gtk-3.0 et gtk-4.0 (bloc marqué) + schéma clair/sombre.",
        default_fmt: Fmt::Rgb,
        builtin_template: include_str!("templates/gtk.css.tmpl"),
        installed: || in_path("gsettings"),
    },
    Target {
        id: "thunar",
        label: "Thunar",
        summary: "Feuille limitée à .thunar, importée en tête de gtk.css.",
        default_fmt: Fmt::Rgb,
        builtin_template: include_str!("templates/thunar.css.tmpl"),
        installed: || in_path("thunar"),
    },
];

/// Cible par identifiant.
pub fn find(id: &str) -> Option<&'static Target> {
    TARGETS.iter().find(|t| t.id == id)
}

/// Programme présent dans le `PATH` ? (doublet local : le binaire dédié ne
/// doit pas tirer `crate::apptheme`, donc le démon avec lui.)
pub fn in_path(name: &str) -> bool {
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };
    path.split(':')
        .any(|dir| !dir.is_empty() && Path::new(dir).join(name).is_file())
}

/// Opération prévue sur un fichier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    /// Écriture complète du fichier (rendu du gabarit).
    Write,
    /// Bloc marqué remplacé en fin de fichier (les règles de l'utilisateur
    /// restent devant).
    UpsertBlock,
    /// Ligne ajoutée en tête (import CSS : GTK ignore un import tardif).
    ImportFirst,
    /// Ligne ajoutée une fois en fin de fichier (`source =` hyprlang).
    AppendSource,
}

/// Opération planifiée pour une cible.
#[derive(Debug, Clone)]
pub struct Planned {
    pub path: PathBuf,
    pub content: String,
    pub op: Op,
}

/// Marqueurs du bloc GTK dans gtk.css.
pub const BLOCK_BEGIN: &str =
    "/* [hyprdesk-theme-engine] début du bloc thème — ne pas modifier entre ces marques */";
pub const BLOCK_END: &str = "/* [hyprdesk-theme-engine] fin du bloc thème */";

/// Marqueur de l'import Thunar en tête de gtk.css.
pub const IMPORT_MARKER: &str = "/* [hyprdesk-theme-engine] import du thème de Thunar */";

/// Marqueur des lignes `source =` ajoutées aux configurations hyprlang.
pub const SOURCE_MARKER: &str = "# [hyprdesk-theme-engine] thème de hyprdesk";

/// Rendu du gabarit de la cible : celui du pack (`templates/<id>.tmpl`) s'il
/// existe, sinon le gabarit intégré.
pub fn render_for(target: &Target, resolved: &Resolved, pack_dir: Option<&Path>) -> Result<String> {
    let pack_template = pack_dir
        .and_then(|dir| {
            let p = dir.join("templates").join(format!("{}.tmpl", target.id));
            std::fs::read_to_string(p).ok()
        })
        .unwrap_or_else(|| target.builtin_template.to_string());
    let ctx = Context {
        resolved,
        default_fmt: target.default_fmt,
    };
    template::render(&pack_template, &ctx)
}

/// Planifie les opérations d'une cible (rendus + chemins), sans rien écrire.
pub fn plan(
    target: &Target,
    resolved: &Resolved,
    pack_dir: Option<&Path>,
    roots: &Roots,
) -> Result<Vec<Planned>> {
    let content = render_for(target, resolved, pack_dir)?;
    validate_css(target.id, &content)?;
    let out = match target.id {
        "hyprland" => vec![Planned {
            path: roots.hypr_theme_dir().join("hyprland.conf"),
            content: with_hypr_options(&content, resolved),
            op: Op::Write,
        }],
        "hyprlock" => vec![
            Planned {
                path: roots.hypr_theme_dir().join("hyprlock.conf"),
                content,
                op: Op::Write,
            },
            Planned {
                path: roots.config.join("hypr").join("hyprlock.conf"),
                content: format!(
                    "source = {}",
                    roots.hypr_theme_dir().join("hyprlock.conf").display()
                ),
                op: Op::AppendSource,
            },
        ],
        "kitty" => vec![Planned {
            path: roots.config.join("kitty").join("hyprdesk-theme.conf"),
            content,
            op: Op::Write,
        }],
        "gtk" => ["gtk-3.0", "gtk-4.0"]
            .iter()
            .map(|dir| Planned {
                path: roots.config.join(dir).join("gtk.css"),
                content: content.clone(),
                op: Op::UpsertBlock,
            })
            .collect(),
        "thunar" => {
            let thunar_css = roots.config.join("gtk-3.0").join("hyprdesk-thunar.css");
            let gtk_css = roots.config.join("gtk-3.0").join("gtk.css");
            // Chemin relatif quand la feuille est à côté de gtk.css.
            let target = if thunar_css.parent() == gtk_css.parent() {
                thunar_css
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            } else {
                thunar_css.display().to_string()
            };
            vec![
                Planned {
                    path: thunar_css,
                    content,
                    op: Op::Write,
                },
                Planned {
                    path: gtk_css,
                    content: format!("@import url(\"{target}\");"),
                    op: Op::ImportFirst,
                },
            ]
        }
        other => return Err(Error::Template(format!("cible inconnue : {other}"))),
    };
    Ok(out)
}

/// Ajoute au fragment hyprland les options de liste blanche du pack,
/// triées pour un diff stable.
fn with_hypr_options(content: &str, resolved: &Resolved) -> String {
    let mut out = content.to_string();
    if resolved.hyprland_options.is_empty() {
        return out;
    }
    out.push_str("\n# Options du pack (liste blanche).\n");
    for (key, value) in &resolved.hyprland_options {
        out.push_str(&format!("{key} = {value}\n"));
    }
    out
}

/// Options Hyprland prêtes pour le mode `hot` : lignes du fragment rendu
/// (`clé = valeur`) puis options du pack.
pub fn hot_keywords(resolved: &Resolved, pack_dir: Option<&Path>) -> Result<Vec<(String, String)>> {
    let target = find("hyprland").ok_or_else(|| Error::Missing("hyprland".to_string()))?;
    let content = render_for(target, resolved, pack_dir)?;
    let content = with_hypr_options(&content, resolved);
    let mut out = Vec::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            out.push((key.trim().to_string(), value.trim().to_string()));
        }
    }
    Ok(out)
}

/// Validation légère d'un rendu CSS : accolades équilibrées (les gabarits
/// hyprlang sont de simples `clé = valeur` vérifiés à la planification).
fn validate_css(id: &str, content: &str) -> Result<()> {
    if id != "gtk" && id != "thunar" {
        return Ok(());
    }
    let (open, close) = (content.matches('{').count(), content.matches('}').count());
    if open != close {
        return Err(Error::Template(format!(
            "cible {id} : accolades déséquilibrées ({open} ouvrantes, {close} fermantes)"
        )));
    }
    Ok(())
}

/// Rechargement prévu par cible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reload {
    None,
    Hyprctl,
    GsettingsScheme,
    KittySignal,
}

pub fn reload_of(id: &str) -> Reload {
    match id {
        "hyprland" => Reload::Hyprctl,
        "gtk" => Reload::GsettingsScheme,
        "kitty" => Reload::KittySignal,
        _ => Reload::None,
    }
}

/// Exécute le rechargement prévu ; `Ok(Some(texte))` = fait, `Ok(None)` =
/// rien à faire, `Err` = échec signalé dans le rapport (l'écriture reste
/// valable).
pub fn run_reload(r: Reload, variant: &str) -> std::result::Result<Option<String>, String> {
    let (prog, args) = match r {
        Reload::None => return Ok(None),
        Reload::Hyprctl => ("hyprctl", vec!["reload"]),
        Reload::GsettingsScheme => (
            "gsettings",
            vec![
                "set",
                "org.gnome.desktop.interface",
                "color-scheme",
                if variant == "light" {
                    "prefer-light"
                } else {
                    "prefer-dark"
                },
            ],
        ),
        Reload::KittySignal => ("pkill", vec!["-USR1", "-x", "kitty"]),
    };
    match Command::new(prog).args(&args).output() {
        Ok(out) if out.status.success() => Ok(Some(format!("{prog} ok"))),
        Ok(out) => Err(format!(
            "{prog} {} : {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Err(e) => Err(format!("{prog} : {e}")),
    }
}

/// Contenu du fichier, ou vide s'il n'existe pas.
pub fn read_or_empty(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

/// Copie de sauvegarde une seule fois (`.bak`) : jamais écrasée par le
/// moteur, elle garde la première version trouvée hors du moteur.
pub fn backup_once(path: &Path) -> Result<bool> {
    let bak = backup_path(path);
    if !path.is_file() || bak.is_file() {
        return Ok(false);
    }
    std::fs::copy(path, &bak)?;
    Ok(true)
}

/// Chemin de la copie de sauvegarde d'un fichier.
pub fn backup_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".avant-hyprdesk.bak");
    path.with_file_name(name)
}

/// Bloc marqué remplacé dans un contenu : l'ancien bloc saute, le nouveau
/// s'ajoute à la fin (les règles de l'utilisateur restent devant).
pub fn upsert_block(existing: &str, block: &str) -> String {
    let head = match existing.find(BLOCK_BEGIN) {
        Some(i) => {
            let after = &existing[i..];
            let end = after
                .find(BLOCK_END)
                .map(|j| i + j + BLOCK_END.len())
                .unwrap_or(existing.len());
            format!(
                "{}{}",
                &existing[..i],
                existing[end..].trim_start_matches('\n')
            )
        }
        None => existing.to_string(),
    };
    let mut out = head;
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(BLOCK_BEGIN);
    out.push('\n');
    out.push_str(block.trim());
    out.push('\n');
    out.push_str(BLOCK_END);
    out.push('\n');
    out
}

/// Import en tête d'une feuille CSS (une seule fois).
pub fn include_first(existing: &str, line: &str) -> String {
    if existing.contains(line) {
        return existing.to_string();
    }
    format!("{IMPORT_MARKER}\n{line}\n\n{existing}")
}

/// Ligne ajoutée une fois en fin de contenu (`source =` hyprlang).
pub fn append_source_once(existing: &str, needle: &str, line: &str) -> String {
    if existing.contains(needle) {
        return existing.to_string();
    }
    let mut out = existing.to_string();
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(SOURCE_MARKER);
    out.push('\n');
    out.push_str(line);
    out.push('\n');
    out
}

/// Avertissement d'inclusion manquante : la config principale existe mais ne
/// référence pas le fragment généré (§8 : le moteur n'édite jamais
/// hyprland.conf / hyprland.lua lui-même).
pub fn include_missing_warning(roots: &Roots, fragment: &str) -> Option<String> {
    for name in ["hyprland.conf", "hyprland.lua"] {
        let path = roots.config.join("hypr").join(name);
        if path.is_file() {
            let content = read_or_empty(&path);
            if !content.contains(fragment) {
                return Some(format!(
                    "hyprland : {name} existe mais n'inclut pas « theme/{fragment} » (include_missing)"
                ));
            }
            return None;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bloc_remplacé_et_règles_utilisateur_préservées() {
        let first = upsert_block(
            "@define-color ancien #fff;\n",
            "@define-color accent rgb(1, 2, 3);\n",
        );
        assert!(first.starts_with("@define-color ancien #fff;"), "{first}");
        assert!(first.contains(BLOCK_BEGIN) && first.contains(BLOCK_END));
        let second = upsert_block(&first, "@define-color accent rgb(9, 9, 9);\n");
        assert!(second.contains("rgb(9, 9, 9)"), "{second}");
        assert!(!second.contains("rgb(1, 2, 3)"), "{second}");
        assert!(second.contains("@define-color ancien"));
        // Idempotent.
        let third = upsert_block(&second, "@define-color accent rgb(9, 9, 9);\n");
        assert_eq!(second, third);
    }

    #[test]
    fn import_unique_en_tête() {
        let once = include_first("@define-color x #fff;\n", "@import url(\"a.css\");");
        assert!(once.starts_with(IMPORT_MARKER), "{once}");
        assert!(once.contains("@import url(\"a.css\");"));
        let twice = include_first(&once, "@import url(\"a.css\");");
        assert_eq!(once, twice);
    }

    #[test]
    fn source_unique_en_fin() {
        let line = "source = /a/b/theme/hyprlock.conf";
        let once = append_source_once("bind = …\n", "theme/hyprlock.conf", line);
        assert!(
            once.ends_with("source = /a/b/theme/hyprlock.conf\n"),
            "{once}"
        );
        let twice = append_source_once(&once, "theme/hyprlock.conf", line);
        assert_eq!(once, twice);
    }

    #[test]
    fn sauvegarde_une_seule_fois() {
        let dir = std::env::temp_dir().join(format!("hd-tgt-bak-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("gtk.css");
        std::fs::write(&f, "/* de l'utilisateur */\n").unwrap();
        assert!(backup_once(&f).unwrap());
        assert_eq!(
            std::fs::read_to_string(backup_path(&f)).unwrap(),
            "/* de l'utilisateur */\n"
        );
        std::fs::write(&f, "/* modifié par le moteur */\n").unwrap();
        // Deuxième appel : la première version est conservée.
        assert!(!backup_once(&f).unwrap());
        assert_eq!(
            std::fs::read_to_string(backup_path(&f)).unwrap(),
            "/* de l'utilisateur */\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn planification_de_chaque_cible() {
        let root = std::env::temp_dir().join(format!("hd-tgt-plan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let r = super::super::resolve::from_theme(&crate::config::Theme::default());
        for t in TARGETS {
            let planned = plan(t, &r, None, &roots).unwrap();
            assert!(!planned.is_empty(), "{} : rien de planifié", t.id);
            for p in &planned {
                assert!(!p.content.is_empty(), "{} : rendu vide", t.id);
            }
        }
        // hyprlock ajoute la ligne source ; gtk touche les deux versions.
        let lock = plan(find("hyprlock").unwrap(), &r, None, &roots).unwrap();
        assert_eq!(lock.len(), 2);
        assert!(matches!(lock[1].op, Op::AppendSource));
        let gtk = plan(find("gtk").unwrap(), &r, None, &roots).unwrap();
        assert_eq!(gtk.len(), 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn mots_clés_hot_extraits() {
        let mut r = super::super::resolve::from_theme(&crate::config::Theme::default());
        r.hyprland_options
            .insert("decoration:rounding".to_string(), "10".to_string());
        let kw = hot_keywords(&r, None).unwrap();
        assert!(
            kw.iter()
                .any(|(k, v)| k == "general:col.active_border" && v.contains("rgba(")),
            "{kw:?}"
        );
        assert!(
            kw.contains(&("decoration:rounding".to_string(), "10".to_string())),
            "{kw:?}"
        );
    }

    #[test]
    fn chemin_present_dans_le_path() {
        assert!(in_path("sh") || in_path("ls"));
        assert!(!in_path("programme-inexistant-xyz"));
    }
}
