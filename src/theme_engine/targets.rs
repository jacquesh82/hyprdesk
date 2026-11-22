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

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::color::Fmt;
use super::paths::Roots;
use super::resolve::Resolved;
use super::template::{self, Context};
use super::{Error, Result};

/// Une cible à exécuter : intégrée (le plan code ses opérations) ou
/// générique, déclarée par le pack (gabarit + sortie + rechargement borné).
#[derive(Debug, Clone)]
pub struct Target {
    pub id: String,
    pub label: String,
    pub summary: String,
    pub default_fmt: Fmt,
    /// Gabarit intégré (cibles intégrées seulement).
    pub builtin_template: Option<&'static str>,
    /// Gabarit déclaré par le pack, chemin relatif au dossier du pack.
    pub pack_template: Option<String>,
    /// Sortie déclarée (cibles génériques seulement, en liste blanche).
    pub output: Option<String>,
    /// Rechargement borné déclaré par le pack (cibles génériques).
    pub reload: Option<String>,
    /// Présence du logiciel dans le `PATH`.
    pub installed: fn() -> bool,
}

fn builtin() -> Vec<Target> {
    vec![
        Target {
            id: "hyprland".to_string(),
            label: "Hyprland".to_string(),
            summary: "Fragment hyprlang : bordures, ombre, options du pack ; inclus par source=."
                .to_string(),
            default_fmt: Fmt::Hypr,
            builtin_template: Some(include_str!("templates/hyprland.conf.tmpl")),
            pack_template: None,
            output: None,
            reload: None,
            installed: || in_path("hyprctl"),
        },
        Target {
            id: "hyprlock".to_string(),
            label: "Hyprlock".to_string(),
            summary: "Variables $hd_* pour hyprlock.conf, inclus par source=.".to_string(),
            default_fmt: Fmt::Hypr,
            builtin_template: Some(include_str!("templates/hyprlock.conf.tmpl")),
            pack_template: None,
            output: None,
            reload: None,
            installed: || in_path("hyprlock"),
        },
        Target {
            id: "kitty".to_string(),
            label: "Kitty".to_string(),
            summary: "Fichier de couleurs inclus en fin de kitty.conf, relu par SIGUSR1."
                .to_string(),
            default_fmt: Fmt::Hex,
            builtin_template: Some(include_str!("templates/kitty.conf.tmpl")),
            pack_template: None,
            output: None,
            reload: None,
            installed: || in_path("kitty"),
        },
        Target {
            id: "gtk".to_string(),
            label: "GTK".to_string(),
            summary: "Couleurs nommées gtk-3.0 et gtk-4.0 (bloc marqué) + schéma clair/sombre."
                .to_string(),
            default_fmt: Fmt::Rgb,
            builtin_template: Some(include_str!("templates/gtk.css.tmpl")),
            pack_template: None,
            output: None,
            reload: None,
            installed: || in_path("gsettings"),
        },
        Target {
            id: "thunar".to_string(),
            label: "Thunar".to_string(),
            summary: "Feuille limitée à .thunar, importée en tête de gtk.css.".to_string(),
            default_fmt: Fmt::Rgb,
            builtin_template: Some(include_str!("templates/thunar.css.tmpl")),
            pack_template: None,
            output: None,
            reload: None,
            installed: || in_path("thunar"),
        },
        Target {
            id: "fonts".to_string(),
            label: "Polices".to_string(),
            summary: "Police d'interface déclarée par le pack (gsettings).".to_string(),
            default_fmt: Fmt::Hex,
            builtin_template: None,
            pack_template: None,
            output: None,
            reload: None,
            installed: || in_path("gsettings"),
        },
        Target {
            id: "icons".to_string(),
            label: "Icônes".to_string(),
            summary: "Thème d'icônes déclaré par le pack (gsettings).".to_string(),
            default_fmt: Fmt::Hex,
            builtin_template: None,
            pack_template: None,
            output: None,
            reload: None,
            installed: || in_path("gsettings"),
        },
        Target {
            id: "cursors".to_string(),
            label: "Curseurs".to_string(),
            summary: "Thème et taille de curseur déclarés par le pack (gsettings).".to_string(),
            default_fmt: Fmt::Hex,
            builtin_template: None,
            pack_template: None,
            output: None,
            reload: None,
            installed: || in_path("gsettings"),
        },
        Target {
            id: "sounds".to_string(),
            label: "Sons".to_string(),
            summary: "Thème de sons déclaré par le pack (gsettings).".to_string(),
            default_fmt: Fmt::Hex,
            builtin_template: None,
            pack_template: None,
            output: None,
            reload: None,
            installed: || in_path("gsettings"),
        },
        Target {
            id: "qt".to_string(),
            label: "Qt".to_string(),
            summary: "Schéma de couleurs qt5/6ct (QPalette en #aarrggbb) ; vaut au relancement."
                .to_string(),
            default_fmt: Fmt::Argb,
            builtin_template: Some(include_str!("templates/qt6ct.conf.tmpl")),
            pack_template: None,
            output: None,
            reload: None,
            installed: || {
                in_path("qt6ct")
                    || std::env::var("HOME")
                        .map(|h| Path::new(&h).join(".config/qt6ct").is_dir())
                        .unwrap_or(false)
            },
        },
    ]
}

/// Identifiants des cibles intégrées.
const BUILTIN_IDS: [&str; 10] = [
    "hyprland", "hyprlock", "kitty", "gtk", "thunar", "qt", "fonts", "icons", "cursors", "sounds",
];

/// Toutes les cibles à exécuter pour cette palette : les intégrées (avec
/// leurs surcharges de pack), puis les génériques déclarées par le pack.
pub fn all(resolved: &Resolved) -> Vec<Target> {
    let mut out = builtin();
    for t in &mut out {
        if let Some(decl) = resolved.pack_targets.get(&t.id) {
            t.pack_template = decl.template.clone();
            t.reload = decl.reload.clone();
        }
    }
    out.retain(|t| {
        resolved
            .pack_targets
            .get(&t.id)
            .and_then(|d| d.enabled)
            .unwrap_or(true)
    });
    for (id, decl) in &resolved.pack_targets {
        if BUILTIN_IDS.contains(&id.as_str()) {
            continue;
        }
        out.push(Target {
            id: id.clone(),
            label: id.clone(),
            summary: format!(
                "cible générique du pack {} (rechargement borné)",
                resolved.id
            ),
            default_fmt: Fmt::Hex,
            builtin_template: None,
            pack_template: decl.template.clone(),
            output: decl.output.clone(),
            reload: decl.reload.clone(),
            installed: || true,
        });
    }
    out
}

/// Cible intégrée par identifiant.
pub fn find(id: &str) -> Option<Target> {
    builtin().into_iter().find(|t| t.id == id)
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Op {
    /// Écriture complète du fichier (rendu du gabarit).
    #[default]
    Write,
    /// Bloc marqué remplacé en fin de fichier (les règles de l'utilisateur
    /// restent devant).
    UpsertBlock,
    /// Ligne ajoutée en tête (import CSS : GTK ignore un import tardif).
    ImportFirst,
    /// Ligne ajoutée une fois en fin de fichier (`source =` hyprlang).
    AppendSource,
    /// Réglage gsettings posé (ancienne valeur relevée pour le retour
    /// arrière) ; `content` porte la valeur, `setting` le couple (schema, clé).
    Setting,
}

/// Opération planifiée pour une cible.
#[derive(Debug, Clone, Default)]
pub struct Planned {
    pub path: PathBuf,
    pub content: String,
    pub op: Op,
    /// Réglage d'environnement posé en plus (`schema`, `clé`) — les cibles
    /// fonts/icons/cursors/sounds ne touchent pas au disque.
    pub setting: Option<(String, String)>,
}

/// Marqueurs du bloc GTK dans gtk.css.
pub const BLOCK_BEGIN: &str =
    "/* [hyprdesk-theme-engine] début du bloc thème — ne pas modifier entre ces marques */";
pub const BLOCK_END: &str = "/* [hyprdesk-theme-engine] fin du bloc thème */";

/// Marqueur de l'import Thunar en tête de gtk.css.
pub const IMPORT_MARKER: &str = "/* [hyprdesk-theme-engine] import du thème de Thunar */";

/// Marqueur des lignes `source =` ajoutées aux configurations hyprlang.
pub const SOURCE_MARKER: &str = "# [hyprdesk-theme-engine] thème de hyprdesk";

/// Rendu du gabarit de la cible, par priorité : gabarit déclaré dans le
/// manifeste (`[targets.<id>].template`), fichier de convention
/// `templates/<id>.tmpl`, puis gabarit intégré.
pub fn render_for(target: &Target, resolved: &Resolved, pack_dir: Option<&Path>) -> Result<String> {
    let source = pack_dir
        .and_then(|dir| {
            if let Some(rel) = &target.pack_template {
                let p = dir.join(rel);
                return Some(std::fs::read_to_string(p).map_err(|e| {
                    Error::Template(format!(
                        "cible {} : gabarit du pack illisible ({e})",
                        target.id
                    ))
                }));
            }
            // Convention : templates/<id>.tmpl à côté du manifeste.
            let p = dir.join("templates").join(format!("{}.tmpl", target.id));
            std::fs::read_to_string(p).ok().map(Ok)
        })
        .transpose()?
        .or_else(|| target.builtin_template.map(str::to_string))
        .ok_or_else(|| {
            Error::Template(format!(
                "cible {} : aucun gabarit (le pack doit déclarer [targets.{}].template)",
                target.id, target.id
            ))
        })?;
    let ctx = Context {
        resolved,
        default_fmt: target.default_fmt,
    };
    template::render(&source, &ctx)
}

/// Planifie les opérations d'une cible (rendus + chemins), sans rien écrire.
pub fn plan(
    target: &Target,
    resolved: &Resolved,
    pack_dir: Option<&Path>,
    roots: &Roots,
) -> Result<Vec<Planned>> {
    plan_opts(target, resolved, pack_dir, roots, &BTreeMap::new())
}

/// Valeur d'option de cible (`kitty.file`…), réglée ou absente.
pub fn target_opt<'a>(target_opts: &'a BTreeMap<String, String>, key: &str) -> Option<&'a str> {
    target_opts
        .get(key)
        .map(|v| v.trim())
        .filter(|v| !v.is_empty())
}

/// Plan complet, avec les options par cible (chemins de fichiers réglés par
/// l'utilisateur dans le démon, clés conservées).
pub fn plan_opts(
    target: &Target,
    resolved: &Resolved,
    pack_dir: Option<&Path>,
    roots: &Roots,
    target_opts: &BTreeMap<String, String>,
) -> Result<Vec<Planned>> {
    if let Some(id) = builtin_id(&target.id) {
        // Cibles à réglages seuls (fonts, icons, cursors, sounds) : aucun
        // gabarit à rendre, leur plan dérive de la palette résolue.
        let à_gabarits = !matches!(id, "fonts" | "icons" | "cursors" | "sounds");
        let content = if à_gabarits {
            Some(render_for(target, resolved, pack_dir)?)
        } else {
            None
        };
        let out = plan_builtin(id, content.as_deref(), resolved, roots, target_opts)?;
        return Ok(out);
    }
    let content = render_for(target, resolved, pack_dir)?;
    validate_css(&target.id, &content)?;
    // Cible générique : sortie déclarée en liste blanche, écriture simple.
    let out_str = target.output.as_deref().ok_or_else(|| {
        Error::Template(format!(
            "cible {} : sortie requise ([targets.{}].output)",
            target.id, target.id
        ))
    })?;
    let path = safe_output_path(roots, out_str)?;
    Ok(vec![Planned {
        path,
        content,
        op: Op::Write,
        ..Default::default()
    }])
}

/// Identifiant de cible intégrée, pour router le plan.
fn builtin_id(id: &str) -> Option<&'static str> {
    BUILTIN_IDS.iter().copied().find(|b| *b == id)
}

/// Plan des cibles intégrées (opérations déclaratives en dur).
fn plan_builtin(
    id: &'static str,
    content: Option<&str>,
    resolved: &Resolved,
    roots: &Roots,
    target_opts: &BTreeMap<String, String>,
) -> Result<Vec<Planned>> {
    let rendu = || content.expect("gabarit manquant pour une cible à fichier");
    let out = match id {
        "hyprland" => vec![Planned {
            path: roots.hypr_theme_dir().join("hyprland.conf"),
            content: rendu().to_string(),
            op: Op::Write,
            ..Default::default()
        }],
        "hyprlock" => vec![
            Planned {
                path: roots.hypr_theme_dir().join("hyprlock.conf"),
                content: rendu().to_string(),
                op: Op::Write,
                ..Default::default()
            },
            Planned {
                path: roots.config.join("hypr").join("hyprlock.conf"),
                content: format!(
                    "source = {}",
                    roots.hypr_theme_dir().join("hyprlock.conf").display()
                ),
                op: Op::AppendSource,
                setting: None,
            },
        ],
        "kitty" => {
            let file = match target_opt(target_opts, "kitty.file") {
                Some(f) => PathBuf::from(f),
                None => roots.config.join("kitty").join("hyprdesk-theme.conf"),
            };
            let mut out = vec![Planned {
                path: file.clone(),
                content: rendu().to_string(),
                op: Op::Write,
                ..Default::default()
            }];
            // Inclusion dans kitty.conf (option `kitty.conf`, vide : à faire
            // soi-même) — même mécanique que le démon historique.
            if let Some(conf) = target_opt(target_opts, "kitty.conf") {
                let conf = PathBuf::from(conf);
                let line = match (file.parent(), conf.parent(), file.file_name()) {
                    (Some(a), Some(b), Some(name)) if a == b => {
                        format!("include {}", name.to_string_lossy())
                    }
                    _ => format!("include {}", file.display()),
                };
                out.push(Planned {
                    path: conf,
                    content: line,
                    op: Op::AppendSource,
                    ..Default::default()
                });
            }
            out
        }
        "gtk" => ["gtk-3.0", "gtk-4.0"]
            .iter()
            .map(|dir| Planned {
                path: roots.config.join(dir).join("gtk.css"),
                content: rendu().to_string(),
                op: Op::UpsertBlock,
                ..Default::default()
            })
            .collect(),
        "qt" => vec![Planned {
            path: roots
                .config
                .join("qt6ct")
                .join("colors")
                .join(format!("hyprdesk-{}.conf", resolved.id)),
            content: rendu().to_string(),
            op: Op::Write,
            setting: None,
        }],
        "fonts" => {
            let Some(fonts) = &resolved.fonts else {
                // Le pack ne déclare pas [fonts] : la cible n\'a rien à faire.
                return Ok(Vec::new());
            };
            let Some(family) = &fonts.family else {
                // Pas de police déclarée : rien à faire.
                return Ok(Vec::new());
            };
            let value = match fonts.size {
                Some(size) => format!("{family} {size}"),
                None => family.clone(),
            };
            vec![Planned {
                path: PathBuf::new(),
                content: value,
                op: Op::Setting,
                setting: Some((
                    "org.gnome.desktop.interface".to_string(),
                    "font-name".to_string(),
                )),
            }]
        }
        "icons" => {
            let Some(icons) = &resolved.icons else {
                // Le pack ne déclare pas [icons] : la cible n\'a rien à faire.
                return Ok(Vec::new());
            };
            let Some(theme) = &icons.theme else {
                return Ok(Vec::new());
            };
            vec![Planned {
                path: PathBuf::new(),
                content: theme.clone(),
                op: Op::Setting,
                setting: Some((
                    "org.gnome.desktop.interface".to_string(),
                    "icon-theme".to_string(),
                )),
            }]
        }
        "cursors" => {
            let Some(cursors) = &resolved.cursors else {
                // Le pack ne déclare pas [cursors] : la cible n\'a rien à faire.
                return Ok(Vec::new());
            };
            let mut out = Vec::new();
            if let Some(theme) = &cursors.theme {
                out.push(Planned {
                    path: PathBuf::new(),
                    content: theme.clone(),
                    op: Op::Setting,
                    setting: Some((
                        "org.gnome.desktop.interface".to_string(),
                        "cursor-theme".to_string(),
                    )),
                });
            }
            if let Some(size) = cursors.size {
                out.push(Planned {
                    path: PathBuf::new(),
                    content: size.to_string(),
                    op: Op::Setting,
                    setting: Some((
                        "org.gnome.desktop.interface".to_string(),
                        "cursor-size".to_string(),
                    )),
                });
            }
            if out.is_empty() {
                return Ok(Vec::new());
            }
            out
        }
        "sounds" => {
            let Some(sounds) = &resolved.sounds else {
                // Le pack ne déclare pas [sounds] : la cible n\'a rien à faire.
                return Ok(Vec::new());
            };
            let Some(theme) = &sounds.theme else {
                return Ok(Vec::new());
            };
            vec![Planned {
                path: PathBuf::new(),
                content: theme.clone(),
                op: Op::Setting,
                setting: Some((
                    "org.gnome.desktop.sound".to_string(),
                    "theme-name".to_string(),
                )),
            }]
        }
        "thunar" => {
            let thunar_css = match target_opt(target_opts, "thunar.file") {
                Some(f) => PathBuf::from(f),
                None => roots.config.join("gtk-3.0").join("hyprdesk-thunar.css"),
            };
            let gtk_css = match target_opt(target_opts, "thunar.conf") {
                Some(c) if c != thunar_css.display().to_string() => PathBuf::from(c),
                _ => roots.config.join("gtk-3.0").join("gtk.css"),
            };
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
                    content: rendu().to_string(),
                    op: Op::Write,
                    ..Default::default()
                },
                Planned {
                    path: gtk_css,
                    content: format!("@import url(\"{target}\");"),
                    op: Op::ImportFirst,
                    setting: None,
                },
            ]
        }
        other => return Err(Error::Template(format!("cible inconnue : {other}"))),
    };
    Ok(out)
}

/// Sortie de cible générique : `~/` développé d'après les racines XDG du
/// contexte (pas le HOME brut), confiné à `~/.config` et `~/.local/share`
/// (§13), sans traverser de lien symbolique.
pub fn safe_output_path(roots: &Roots, declared: &str) -> Result<PathBuf> {
    let expanded = if let Some(rest) = declared.strip_prefix("~/.config/") {
        roots.config.join(rest)
    } else if let Some(rest) = declared.strip_prefix("~/.local/share/") {
        roots.data.join(rest)
    } else if declared.starts_with("~/") {
        return Err(Error::Template(format!(
            "sortie hors liste blanche : {declared} (attendu : sous ~/.config ou ~/.local/share)"
        )));
    } else {
        PathBuf::from(declared)
    };
    if !expanded.starts_with(&roots.config) && !expanded.starts_with(&roots.data) {
        return Err(Error::Template(format!(
            "sortie hors liste blanche : {declared} (attendu : sous ~/.config ou ~/.local/share)"
        )));
    }
    // Aucun parent ne peut être un lien symbolique qui sortirait du périmètre.
    let mut ancêtre: Option<&Path> = Some(expanded.as_path());
    while let Some(a) = ancêtre {
        if let Ok(meta) = std::fs::symlink_metadata(a)
            && meta.is_symlink()
        {
            return Err(Error::Template(format!(
                "sortie refusée : traverserait le lien symbolique {}",
                a.display()
            )));
        }
        ancêtre = a.parent();
    }
    Ok(expanded)
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
    let content = render_for(&target, resolved, pack_dir)?;
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reload {
    None,
    Hyprctl,
    GsettingsScheme,
    KittySignal,
    /// Cible générique : signal borné à un processus (§8).
    Signal(String, String),
    /// Cible générique : kitty rechargé par son socket de contrôle distant.
    KittySocket,
}

pub fn reload_of(id: &str) -> Reload {
    match id {
        "hyprland" => Reload::Hyprctl,
        "gtk" => Reload::GsettingsScheme,
        "kitty" => Reload::KittySignal,
        _ => Reload::None,
    }
}

/// Rechargement d'une cible générique, d'après sa déclaration déjà validée.
pub fn reload_generic(declared: &str) -> Result<Reload> {
    let decl = super::manifest::PackTarget {
        reload: Some(declared.to_string()),
        ..Default::default()
    };
    match decl.reload_kind().map_err(Error::Template)? {
        Some(super::manifest::ReloadSpec::None) => Ok(Reload::None),
        Some(super::manifest::ReloadSpec::Signal(p, s)) => Ok(Reload::Signal(p, s)),
        Some(super::manifest::ReloadSpec::Gsettings) => Ok(Reload::GsettingsScheme),
        Some(super::manifest::ReloadSpec::Hyprctl) => Ok(Reload::Hyprctl),
        Some(super::manifest::ReloadSpec::KittySocket) => Ok(Reload::KittySocket),
        Some(super::manifest::ReloadSpec::Daemon) => Ok(Reload::None),
        None => Ok(Reload::None),
    }
}

/// Exécute le rechargement prévu ; `Ok(Some(texte))` = fait, `Ok(None)` =
/// rien à faire, `Err` = échec signalé dans le rapport (l'écriture reste
/// valable). Le schéma GTK s'accompagne de la couleur d'accent
/// (`org.gnome.desktop.interface accent-color`), lue par libadwaita récent
/// et les applications GNOME — le canal standard sans service de portail.
pub fn run_reload(r: Reload, resolved: &Resolved) -> std::result::Result<Option<String>, String> {
    let variant = resolved.variant.as_str();
    let (prog, args): (&str, Vec<String>) = match r {
        Reload::None => return Ok(None),
        Reload::Hyprctl => ("hyprctl", vec!["reload".to_string()]),
        Reload::GsettingsScheme => {
            let accent = super::color::fmt(resolved.palette["accent"], Fmt::Rgb);
            (
                "gsettings",
                vec![
                    "set".to_string(),
                    "org.gnome.desktop.interface".to_string(),
                    "color-scheme".to_string(),
                    if variant == "light" {
                        "prefer-light".to_string()
                    } else {
                        "prefer-dark".to_string()
                    },
                ]
                .into_iter()
                .chain([
                    "set".to_string(),
                    "org.gnome.desktop.interface".to_string(),
                    "accent-color".to_string(),
                    format!("'{accent}'"),
                ])
                .collect(),
            )
        }
        Reload::KittySignal => (
            "pkill",
            vec!["-USR1".to_string(), "-x".to_string(), "kitty".to_string()],
        ),
        Reload::Signal(name, sig) => (
            "pkill",
            vec![format!("-{sig}"), "-x".to_string(), name.clone()],
        ),
        Reload::KittySocket => (
            "kitty",
            vec!["@@".replace("@@", "@"), "load-config".to_string()],
        ),
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

/// Retire le bloc marqué d'un contenu (clear GTK) : tout ce qui précède
/// reste, ce qui suit (règles de l'utilisateur postérieures) aussi.
pub fn remove_block(existing: &str) -> String {
    match existing.find(BLOCK_BEGIN) {
        Some(i) => {
            let after = &existing[i..];
            let end = after
                .find(BLOCK_END)
                .map(|j| i + j + BLOCK_END.len())
                .unwrap_or(existing.len());
            let mut head = existing[..i].to_string();
            let tail = existing[end..].trim_start_matches('\n');
            head.push_str(tail);
            head
        }
        None => existing.to_string(),
    }
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
        let à_réglages = ["fonts", "icons", "cursors", "sounds"];
        let toutes = all(&r);
        for t in &toutes {
            let planned = plan(t, &r, None, &roots).unwrap();
            if !à_réglages.contains(&t.id.as_str()) {
                assert!(!planned.is_empty(), "{} : rien de planifié", t.id);
            }
            for p in &planned {
                assert!(!p.content.is_empty(), "{} : rendu vide", t.id);
            }
        }
        // hyprlock ajoute la ligne source ; gtk touche les deux versions.
        let lock = plan(&find("hyprlock").unwrap(), &r, None, &roots).unwrap();
        assert_eq!(lock.len(), 2);
        assert!(matches!(lock[1].op, Op::AppendSource));
        let gtk = plan(&find("gtk").unwrap(), &r, None, &roots).unwrap();
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
    fn cible_générique_planifiée_avec_sortie_et_signal() {
        let root = std::env::temp_dir().join(format!("hd-tgt-gen-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let pack = root.join("pack");
        std::fs::create_dir_all(pack.join("templates")).unwrap();
        std::fs::write(
            pack.join("templates/waybar.tmpl"),
            "/* {{meta.name}} */\n@define-color accent {{palette.accent}};\n",
        )
        .unwrap();
        let mut r = super::super::resolve::from_theme(&crate::config::Theme::default());
        r.pack_targets.insert(
            "waybar".to_string(),
            super::super::manifest::PackTarget {
                template: Some("templates/waybar.tmpl".to_string()),
                output: Some("~/.config/waybar/hyprdesk.css".to_string()),
                reload: Some("signal:waybar:USR1".to_string()),
                enabled: None,
            },
        );
        let cibles = all(&r);
        let waybar = cibles.iter().find(|t| t.id == "waybar").unwrap();
        let planned = plan(waybar, &r, Some(&pack), &roots).unwrap();
        assert_eq!(
            planned[0].path,
            roots.config.join("waybar/hyprdesk.css"),
            "~ développé"
        );
        assert!(
            planned[0].content.contains("@define-color accent"),
            "{}",
            planned[0].content
        );
        // Rechargement borné reconnu.
        assert_eq!(
            reload_generic("signal:waybar:USR1").unwrap(),
            Reload::Signal("waybar".to_string(), "USR1".to_string())
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn kitty_inclusion_et_fichier_réglés_par_options() {
        let root = std::env::temp_dir().join(format!("hd-tgt-kopt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let r = super::super::resolve::from_theme(&crate::config::Theme::default());
        let mut target_opts = BTreeMap::new();
        target_opts.insert(
            "kitty.file".to_string(),
            root.join("kitty-perso/couleurs.conf").display().to_string(),
        );
        target_opts.insert(
            "kitty.conf".to_string(),
            root.join("kitty-perso/kitty.conf").display().to_string(),
        );
        let kitty = find("kitty").unwrap();
        let planned = plan_opts(&kitty, &r, None, &roots, &target_opts).unwrap();
        assert_eq!(planned.len(), 2, "{planned:?}");
        assert!(planned[0].path.ends_with("couleurs.conf"), "{planned:?}");
        // Include : même dossier → nom seul (mécanique du démon historique).
        assert_eq!(
            planned[1].content, "include couleurs.conf",
            "{}",
            planned[1].content
        );
        assert!(matches!(planned[1].op, Op::AppendSource));
        // Options absentes : chemins par défaut, pas d'inclusion.
        let defaut = plan(&kitty, &r, None, &roots).unwrap();
        assert_eq!(defaut.len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn remove_block_renvoie_aux_règles_utilisateur() {
        let bloc = format!("avant\n{}\ncontenu\n{}\naprès\n", BLOCK_BEGIN, BLOCK_END);
        let sans = remove_block(&bloc);
        assert!(sans.contains("avant") && sans.contains("après"), "{sans}");
        assert!(!sans.contains(BLOCK_BEGIN));
        // Idempotent.
        assert_eq!(remove_block(&sans), sans);
    }

    #[test]
    fn sortie_hors_liste_blanche_et_liens_symboliques_refusés() {
        let root = std::env::temp_dir().join(format!("hd-tgt-safe-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        assert!(safe_output_path(&roots, "~/.ssh/evil").is_err());
        assert!(safe_output_path(&roots, "/etc/hypr/evil").is_err());
        assert!(safe_output_path(&roots, "~/.config/waybar/x.css").is_ok());
        assert!(safe_output_path(&roots, "~/.local/share/app/x").is_ok());
        // Un parent symlink fait refuser la sortie.
        std::fs::create_dir_all(roots.config.join("sortie")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc", roots.config.join("sortie/lien")).unwrap();
        assert!(safe_output_path(&roots, "~/.config/sortie/lien/x.css").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn cible_désactivée_par_le_pack() {
        let mut r = super::super::resolve::from_theme(&crate::config::Theme::default());
        r.pack_targets.insert(
            "kitty".to_string(),
            super::super::manifest::PackTarget {
                enabled: Some(false),
                ..Default::default()
            },
        );
        assert!(all(&r).iter().all(|t| t.id != "kitty")); // all(&r) vit assez pour l'itération
        // (itération directe sur la valeur retournée : E0716 évité ci-après)
        r.pack_targets.get_mut("kitty").unwrap().enabled = Some(true);
        let cibles = all(&r);
        assert!(cibles.iter().any(|t| t.id == "kitty"));
    }

    #[test]
    fn gabarit_qt_en_argb() {
        let root = std::env::temp_dir().join(format!("hd-tgt-qt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let r = super::super::resolve::from_theme(&crate::config::Theme::default());
        let cibles = all(&r);
        let qt = cibles.iter().find(|t| t.id == "qt").unwrap();
        let planned = plan(qt, &r, None, &roots).unwrap();
        let c = &planned[0];
        assert!(
            c.path.ends_with("qt6ct/colors/hyprdesk-regle.conf"),
            "{}",
            c.path.display()
        );
        assert!(c.content.contains("[ColorScheme]"), "{}", c.content);
        // Le fond en #aarrggbb opaque.
        assert!(c.content.contains("#ff171b27"), "{}", c.content);
        assert!(c.content.matches(',').count() >= 3 * 19);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn chemin_present_dans_le_path() {
        assert!(in_path("sh") || in_path("ls"));
        assert!(!in_path("programme-inexistant-xyz"));
    }
}
