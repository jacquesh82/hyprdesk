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

#[cfg(test)]
thread_local! {
    /// `PATH` vu par [`in_path`] dans le fil d'un essai : jamais le `PATH` du
    /// processus, partagé par les essais lancés en parallèle.
    pub static TEST_PATH: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Programme présent dans le `PATH` ? (doublet local : le binaire dédié ne
/// doit pas tirer `crate::apptheme`, donc le démon avec lui.)
pub fn in_path(name: &str) -> bool {
    #[cfg(test)]
    if let Some(path) = TEST_PATH.with(|p| p.borrow().clone()) {
        return path
            .split(':')
            .any(|dir| !dir.is_empty() && Path::new(dir).join(name).is_file());
    }
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
        "hyprland" => {
            // Options du pack (liste blanche) comprises : le fragment
            // persistant porte ce que le mode `hot` pose en direct.
            let content = with_hypr_options(rendu(), resolved);
            validate_hypr_fragment(&content)?;
            vec![Planned {
                path: roots.hypr_theme_dir().join("hyprland.conf"),
                content,
                op: Op::Write,
                ..Default::default()
            }]
        }
        "hyprlock" => vec![
            Planned {
                path: roots.hypr_theme_dir().join("hyprlock.conf"),
                content: {
                    validate_hyprlock(rendu())?;
                    rendu().to_string()
                },
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
            validate_kitty(rendu())?;
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
        "gtk" => {
            // Couche douce : bloc de couleurs nommées dans les gtk.css de
            // l'utilisateur (applications qui écrasent le thème, libadwaita).
            let mut out: Vec<Planned> = ["gtk-3.0", "gtk-4.0"]
                .iter()
                .map(|dir| Planned {
                    path: roots.config.join(dir).join("gtk.css"),
                    content: rendu().to_string(),
                    op: Op::UpsertBlock,
                    ..Default::default()
                })
                .collect();
            // Couche forte (MLG-237) : thème de base recoloré, installé et
            // sélectionné par gtk-theme.
            out.extend(plan_gtk_theme(rendu(), resolved, roots));
            out
        }
        "qt" => vec![Planned {
            path: roots
                .config
                .join("qt6ct")
                .join("colors")
                // Nom stable (MLG-247) : choisi une fois dans qt6ct, il suit
                // chaque pack.
                .join(QT_SCHEME),
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

/// Schéma de couleurs Qt écrit par le moteur (`~/.config/qt6ct/colors/`).
pub const QT_SCHEME: &str = "hyprdesk.conf";

/// Clé gsettings du thème GTK.
pub const GTK_THEME_KEY: (&str, &str) = ("org.gnome.desktop.interface", "gtk-theme");

/// Plan du thème GTK installé : feuilles GTK 3 et 4 du thème de base
/// recolorées, `index.theme`, puis `gtk-theme` posé. Le nom alterne entre
/// `hyprdesk-<pack>` et `hyprdesk-<pack>-bis` quand le contenu change : le
/// changement de nom fait relire leurs feuilles aux applications ouvertes.
fn plan_gtk_theme(named_block: &str, resolved: &Resolved, roots: &Roots) -> Vec<Planned> {
    use super::gtktheme as g;
    let current = g::current_theme();
    let base = g::base_name(roots, current.as_deref());
    let base_dir = g::find_theme(roots, &base);
    let dark = resolved.variant != "light";
    let mut files: Vec<(String, String)> = Vec::new();
    for (major, dir) in [(3u8, "gtk-3.0"), (4, "gtk-4.0")] {
        let sheet = base_dir
            .as_deref()
            .and_then(|d| g::base_sheet(d, major, dark))
            .and_then(|p| g::read_sheet(&p));
        let css = match sheet {
            Some(css) => g::render_sheet(Some(&css), resolved, named_block),
            // Base introuvable (Adwaita intégré à GTK 3) : sa feuille
            // embarquée, couleurs nommées redéfinies par-dessus.
            None if major == 3 => format!(
                "@import url(\"resource:///org/gtk/libgtk/theme/Adwaita/gtk-contained{}.css\");\n\n{named_block}",
                if dark { "-dark" } else { "" }
            ),
            // Pas de GTK 4 dans la base : GTK 4 garde son thème par défaut.
            None => continue,
        };
        files.push((format!("{dir}/gtk.css"), css.clone()));
        files.push((format!("{dir}/gtk-dark.css"), css));
    }
    let a = format!("{}{}", g::PREFIX, resolved.id);
    let b = format!("{a}-bis");
    let themes = roots.data.join("themes");
    let same = |name: &str| {
        files
            .iter()
            .all(|(rel, c)| read_or_empty(&themes.join(name).join(rel)) == *c)
    };
    let name = match current.as_deref() {
        Some(c) if c == a || c == b => {
            if same(c) {
                c.to_string()
            } else if c == a {
                b
            } else {
                a
            }
        }
        _ => a,
    };
    let mut out: Vec<Planned> = files
        .into_iter()
        .map(|(rel, content)| Planned {
            path: themes.join(&name).join(rel),
            content,
            op: Op::Write,
            ..Default::default()
        })
        .collect();
    out.push(Planned {
        path: themes.join(&name).join("index.theme"),
        content: g::index_theme(&name, resolved, &base),
        op: Op::Write,
        ..Default::default()
    });
    out.push(Planned {
        path: PathBuf::new(),
        content: name,
        op: Op::Setting,
        setting: Some((GTK_THEME_KEY.0.to_string(), GTK_THEME_KEY.1.to_string())),
    });
    out
}

/// Couche forte GTK retirée : le thème de base revient d'abord (les
/// applications ne restent pas sur un thème retiré), puis les thèmes du
/// moteur sont supprimés. `dry` : rien n'est fait.
pub fn remove_gtk_theme(roots: &Roots, dry: bool) -> Vec<(PathBuf, Removal)> {
    let mut done = Vec::new();
    if let Some(cur) =
        super::gtktheme::current_theme().filter(|c| c.starts_with(super::gtktheme::PREFIX))
    {
        let base = super::gtktheme::base_name(roots, Some(&cur));
        done.push((
            PathBuf::from(format!("{}/{} → {base}", GTK_THEME_KEY.0, GTK_THEME_KEY.1)),
            Removal::Reset,
        ));
        if !dry {
            let _ = Command::new("gsettings")
                .args(["set", GTK_THEME_KEY.0, GTK_THEME_KEY.1, &base])
                .output();
        }
    }
    for dir in super::gtktheme::installed(roots) {
        done.push((dir.clone(), Removal::Removed));
        if !dry {
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
    done
}

/// Après une application réussie d'une cible : ce qui ne sert plus est
/// retiré — les autres thèmes GTK du moteur (celui posé reste), les anciens
/// schémas Qt par pack.
pub fn after_commit(id: &str, planned: &[Planned], roots: &Roots) {
    match id {
        "gtk" => {
            if let Some(name) = planned
                .iter()
                .find(|p| {
                    p.setting
                        .as_ref()
                        .is_some_and(|(_, k)| k == GTK_THEME_KEY.1)
                })
                .map(|p| p.content.clone())
            {
                super::gtktheme::prune(roots, &name);
            }
        }
        "qt" => {
            if let Ok(rd) = std::fs::read_dir(roots.config.join("qt6ct").join("colors")) {
                for e in rd.flatten() {
                    let n = e.file_name().to_string_lossy().to_string();
                    if n.starts_with("hyprdesk-") && n.ends_with(".conf") {
                        let _ = std::fs::remove_file(e.path());
                    }
                }
            }
        }
        _ => {}
    }
}

/// Sortie de cible générique : `~/` développé d'après les racines XDG du
/// contexte (pas le HOME brut), confiné à `~/.config` et `~/.local/share`
/// (§13). Le périmètre est vérifié sur le chemin réel : une racine XDG qui
/// est elle-même un lien (dotfiles) reste admise, un lien qui ferait sortir
/// l'écriture du périmètre non. Les emplacements d'où une commande peut être
/// lancée (démarrage, services, scripts, configuration de Hyprland) sont
/// refusés : un pack est fait de données.
pub fn safe_output_path(roots: &Roots, declared: &str) -> Result<PathBuf> {
    let refus =
        |why: &str| Error::Template(format!("sortie hors liste blanche : {declared} ({why})"));
    let expanded = if let Some(rest) = declared.strip_prefix("~/.config/") {
        roots.config.join(rest)
    } else if let Some(rest) = declared.strip_prefix("~/.local/share/") {
        roots.data.join(rest)
    } else if declared.starts_with("~/") {
        return Err(refus("attendu : sous ~/.config ou ~/.local/share"));
    } else {
        PathBuf::from(declared)
    };
    if expanded
        .components()
        .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(refus("tronçon .. refusé"));
    }
    let réel = real_path(&expanded);
    let racines = [
        (real_path(&roots.config), false),
        (real_path(&roots.data), true),
    ];
    let Some((rel, données)) = racines
        .iter()
        .find_map(|(r, d)| réel.strip_prefix(r).ok().map(|rel| (rel.to_path_buf(), *d)))
    else {
        return Err(refus(
            "attendu : sous ~/.config ou ~/.local/share, sans lien qui en sorte",
        ));
    };
    if rel.as_os_str().is_empty() {
        return Err(refus("un fichier est attendu, pas la racine"));
    }
    if let Some(why) = emplacement_refusé(&rel, données) {
        return Err(refus(why));
    }
    Ok(expanded)
}

/// Dossiers de `~/.config` d'où une écriture peut lancer une commande.
const CONFIG_REFUSÉS: &[&str] = &[
    "autostart",
    "systemd",
    "environment.d",
    "uwsm",
    "hypr",
    "kitty",
    "fish",
    "nushell",
    "plasma-workspace",
    "xdg-desktop-portal",
    "Thunar",
    "hyprdesk",
];

/// Dossiers de `~/.local/share` refusés pour la même raison (lanceurs,
/// services D-Bus, packs et état du moteur).
const DATA_REFUSÉS: &[&str] = &[
    "applications",
    "dbus-1",
    "systemd",
    "flatpak",
    "nautilus",
    "hyprdesk",
];

/// Extensions de fichiers exécutables ou de lanceurs.
const EXTENSIONS_REFUSÉES: &[&str] = &[
    "desktop", "service", "timer", "socket", "path", "sh", "bash", "zsh", "fish", "nu", "lua",
    "py", "pl", "rb", "js", "vim", "el", "so",
];

fn emplacement_refusé(rel: &Path, données: bool) -> Option<&'static str> {
    let premier = rel
        .components()
        .next()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .unwrap_or_default();
    let dossiers = if données {
        DATA_REFUSÉS
    } else {
        CONFIG_REFUSÉS
    };
    if dossiers.contains(&premier.as_str()) {
        return Some("emplacement réservé : démarrage, services, scripts ou cible intégrée");
    }
    let ext = rel
        .extension()
        .map(|e| e.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    if EXTENSIONS_REFUSÉES.contains(&ext.as_str()) {
        return Some("type de fichier exécutable ou lanceur refusé");
    }
    None
}

/// Chemin réel d'une sortie, même inexistante : liens finaux suivis, plus
/// proche ancêtre existant rendu canonique, reste du chemin rattaché.
fn real_path(path: &Path) -> PathBuf {
    let suivi = super::state::real_target(path);
    let mut existant = suivi.as_path();
    let mut reste = Vec::new();
    loop {
        if let Ok(c) = std::fs::canonicalize(existant) {
            let mut out = c;
            for part in reste.iter().rev() {
                out.push(part);
            }
            return out;
        }
        match (existant.parent(), existant.file_name()) {
            (Some(parent), Some(name)) => {
                reste.push(name.to_os_string());
                existant = parent;
            }
            _ => return suivi,
        }
    }
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
    validate_hypr_fragment(&content)?;
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

/// Fragment Hyprland : des options `clé = valeur` et des commentaires, rien
/// d'autre. Un gabarit de pack ne peut ni lancer de commande (`exec`), ni
/// sourcer un fichier, ni poser un raccourci ou une variable d'environnement.
pub fn validate_hypr_fragment(content: &str) -> Result<()> {
    for (n, line) in content.lines().enumerate() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let Some((key, value)) = l.split_once('=') else {
            return Err(Error::Template(format!(
                "cible hyprland, ligne {} : « {l} » n'est pas une option clé = valeur",
                n + 1
            )));
        };
        let key = key.trim();
        // Couleurs des fenêtres : posées par le gabarit intégré, admises en
        // plus de la liste blanche des packs.
        let couleur_fenêtre = super::manifest::valid_option_path(key)
            && (key.starts_with("general:col.") || key.starts_with("group:col."));
        if !(couleur_fenêtre || super::manifest::allowed_hyprland_key(key)) {
            return Err(Error::Template(format!(
                "cible hyprland, ligne {} : option hors liste blanche : {key}",
                n + 1
            )));
        }
        if value.chars().any(|c| c.is_control() && c != '\t') {
            return Err(Error::Template(format!(
                "cible hyprland, ligne {} : caractère de contrôle dans la valeur",
                n + 1
            )));
        }
    }
    Ok(())
}

/// Fragment hyprlock : variables et widgets, sans commande. `cmd[…]` (texte
/// d'un label calculé par le shell), `exec` et `source` sont refusés.
pub fn validate_hyprlock(content: &str) -> Result<()> {
    for (n, line) in content.lines().enumerate() {
        let l = line.trim();
        if l.starts_with('#') {
            continue;
        }
        let key = l.split('=').next().unwrap_or_default().trim();
        if l.contains("cmd[") || key == "source" || key.starts_with("exec") {
            return Err(Error::Template(format!(
                "cible hyprlock, ligne {} : commande refusée dans un gabarit de pack (« {l} »)",
                n + 1
            )));
        }
    }
    Ok(())
}

/// Directives kitty admises : couleurs, opacité, polices. Les directives qui
/// lancent un programme (`shell`, `launch`, `map`, `watcher`,
/// `scrollback_pager`, `include`…) sont refusées.
pub fn validate_kitty(content: &str) -> Result<()> {
    const EXACTES: &[&str] = &[
        "foreground",
        "background",
        "cursor",
        "background_opacity",
        "background_blur",
        "dynamic_background_opacity",
        "background_tint",
        "font_family",
        "bold_font",
        "italic_font",
        "bold_italic_font",
        "font_size",
        "active_tab_font_style",
        "inactive_tab_font_style",
        "tab_bar_style",
    ];
    for (n, line) in content.lines().enumerate() {
        let l = line.trim();
        if l.is_empty() || l.starts_with('#') {
            continue;
        }
        let key = l.split_whitespace().next().unwrap_or_default();
        let admise = EXACTES.contains(&key)
            || key.strip_prefix("color").is_some_and(|d| {
                !d.is_empty() && d.len() <= 3 && d.chars().all(|c| c.is_ascii_digit())
            })
            || key.starts_with("selection_")
            || ["_color", "_foreground", "_background"]
                .iter()
                .any(|s| key.ends_with(s));
        if !admise || l.chars().any(|c| c.is_control() && c != '\t') {
            return Err(Error::Template(format!(
                "cible kitty, ligne {} : directive refusée : {key} (couleurs, opacité et polices seulement)",
                n + 1
            )));
        }
    }
    Ok(())
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
    /// Commande configurée sur le poste (réglage du démon, jamais un
    /// contenu de pack), passée à `sh -c`.
    Command(String),
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
    match r {
        Reload::None => Ok(None),
        Reload::Hyprctl => run("hyprctl", &["reload"]).map(Some),
        Reload::GsettingsScheme => {
            // Un appel par clé : gsettings n'en pose qu'une à la fois.
            let schéma = if variant == "light" {
                "prefer-light"
            } else {
                "prefer-dark"
            };
            run(
                "gsettings",
                &["set", "org.gnome.desktop.interface", "color-scheme", schéma],
            )?;
            // L'accent de GNOME est une couleur nommée : la plus proche de
            // celle du thème. Une clé absente (GNOME < 47) n'est pas un échec.
            let accent = gnome_accent(resolved.palette["accent"]);
            match run(
                "gsettings",
                &["set", "org.gnome.desktop.interface", "accent-color", accent],
            ) {
                Ok(_) => Ok(Some(format!("gsettings ok ({schéma}, accent {accent})"))),
                Err(e) => Ok(Some(format!(
                    "gsettings ok ({schéma}) ; accent non posé : {e}"
                ))),
            }
        }
        Reload::KittySignal => run("pkill", &["-USR1", "-x", "kitty"]).map(Some),
        Reload::Signal(name, sig) => run("pkill", &[&format!("-{sig}"), "-x", &name]).map(Some),
        Reload::KittySocket => run("kitty", &["@", "load-config"]).map(Some),
        Reload::Command(cmd) => run("sh", &["-c", &cmd]).map(Some),
    }
}

/// Lance un programme de rechargement ; texte court en cas de succès.
fn run(prog: &str, args: &[&str]) -> std::result::Result<String, String> {
    match Command::new(prog).args(args).output() {
        Ok(out) if out.status.success() => Ok(format!("{prog} ok")),
        Ok(out) => Err(format!(
            "{prog} {} : {}",
            args.join(" "),
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Err(e) => Err(format!("{prog} : {e}")),
    }
}

/// Couleurs d'accent nommées de GNOME (libadwaita 1.6), dans l'ordre de
/// l'énumération `accent-color`.
const GNOME_ACCENTS: [(&str, &str); 9] = [
    ("blue", "#3584e4"),
    ("teal", "#2190a4"),
    ("green", "#3a944a"),
    ("yellow", "#c88800"),
    ("orange", "#ed5b00"),
    ("red", "#e62d42"),
    ("pink", "#d56199"),
    ("purple", "#9141ac"),
    ("slate", "#6f8396"),
];

/// Accent GNOME le plus proche d'une couleur : `slate` pour une couleur
/// presque neutre, sinon la teinte OKLCH la plus proche.
pub fn gnome_accent(c: crate::config::Rgba) -> &'static str {
    let o = super::color::to_oklch(c);
    if o.c < 0.04 {
        return "slate";
    }
    let écart = |h: f64| {
        let d = (o.h - h).rem_euclid(360.0);
        d.min(360.0 - d)
    };
    GNOME_ACCENTS
        .iter()
        .filter(|(nom, _)| *nom != "slate")
        .filter_map(|(nom, hex)| {
            super::color::parse(hex).map(|rgba| (*nom, super::color::to_oklch(rgba).h))
        })
        .min_by(|a, b| écart(a.1).total_cmp(&écart(b.1)))
        .map(|(nom, _)| nom)
        .unwrap_or("blue")
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

/// Nettoyage d'un contenu : lignes du moteur retirées.
type Nettoyage = fn(&str) -> String;

/// Action de [`remove_all`] sur un fichier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Removal {
    /// Lignes du moteur retirées, celles de l'utilisateur gardées.
    Cleaned,
    /// Fichier qui ne portait que le thème : retiré.
    Emptied,
    /// Fichier généré par le moteur : retiré.
    Removed,
    /// Réglage remis à sa valeur d'avant le moteur.
    Reset,
}

/// Retire d'un contenu les lignes posées après [`SOURCE_MARKER`] (une
/// ligne `source =` ou `include` par marqueur) et l'ancien marqueur de
/// kitty d'avant le moteur.
pub fn remove_sources(existing: &str) -> String {
    let mut out = Vec::new();
    let mut skip_next = false;
    for line in existing.lines() {
        if skip_next {
            skip_next = false;
            continue;
        }
        let t = line.trim_start();
        if t == SOURCE_MARKER || t.starts_with("# Thème de hyprdesk (") {
            skip_next = true;
            continue;
        }
        out.push(line);
    }
    let mut text = out.join("\n");
    if existing.ends_with('\n') && !text.is_empty() {
        text.push('\n');
    }
    text
}

/// Retire l'import de Thunar posé en tête de gtk.css ([`IMPORT_MARKER`]) et
/// la ligne vide qui le suit.
pub fn remove_import(existing: &str) -> String {
    let Some(rest) = existing.strip_prefix(IMPORT_MARKER) else {
        return existing.to_string();
    };
    let rest = rest.strip_prefix('\n').unwrap_or(rest);
    let rest = match rest.split_once('\n') {
        Some((first, after)) if first.trim_start().starts_with("@import url(") => after,
        _ => rest,
    };
    rest.strip_prefix('\n').unwrap_or(rest).to_string()
}

/// Tout ce que le moteur a posé chez l'utilisateur, retiré (désinstallation,
/// MLG-230) : blocs et imports de gtk.css, lignes d'inclusion de kitty.conf
/// et hyprlock.conf, fichiers générés (kitty, Thunar, Qt, fragments
/// Hyprland). Les règles de l'utilisateur restent ; un fichier qui n'en
/// porte plus aucune est retiré. Les réglages gsettings ne sont pas touchés
/// (la restauration de l'état d'origine les remet). `dry` : rien n'est
/// écrit. Rend une action par fichier.
pub fn remove_all(
    roots: &Roots,
    target_opts: &BTreeMap<String, String>,
    dry: bool,
) -> Vec<(PathBuf, Removal)> {
    let mut done = Vec::new();
    let opt_path = |key: &str, default: PathBuf| {
        target_opt(target_opts, key)
            .map(PathBuf::from)
            .unwrap_or(default)
    };
    let mut edits: Vec<(PathBuf, Nettoyage)> = Vec::new();
    for dir in ["gtk-3.0", "gtk-4.0"] {
        edits.push((roots.config.join(dir).join("gtk.css"), |t| {
            remove_import(&remove_block(t))
        }));
    }
    edits.push((
        opt_path("thunar.conf", roots.config.join("gtk-3.0").join("gtk.css")),
        |t| remove_import(t),
    ));
    edits.push((
        opt_path("kitty.conf", roots.config.join("kitty").join("kitty.conf")),
        remove_sources,
    ));
    edits.push((
        roots.config.join("hypr").join("hyprlock.conf"),
        remove_sources,
    ));
    let mut vus = std::collections::BTreeSet::new();
    for (path, f) in edits {
        if !vus.insert(path.clone()) || !path.is_file() {
            continue;
        }
        let avant = read_or_empty(&path);
        let après = f(&avant);
        if après == avant {
            continue;
        }
        if après.trim().is_empty() {
            done.push((path.clone(), Removal::Emptied));
            if !dry {
                let _ = std::fs::remove_file(&path);
            }
        } else {
            done.push((path.clone(), Removal::Cleaned));
            if !dry {
                let _ = super::state::atomic_write(&path, &après);
            }
        }
    }
    let mut files = vec![
        opt_path(
            "kitty.file",
            roots.config.join("kitty").join("hyprdesk-theme.conf"),
        ),
        opt_path(
            "thunar.file",
            roots.config.join("gtk-3.0").join("hyprdesk-thunar.css"),
        ),
        roots.hypr_theme_dir().join("hyprland.conf"),
        roots.hypr_theme_dir().join("hyprlock.conf"),
    ];
    if let Ok(rd) = std::fs::read_dir(roots.config.join("qt6ct").join("colors")) {
        files.extend(rd.flatten().map(|e| e.path()).filter(|p| {
            p.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
                n == QT_SCHEME || (n.starts_with("hyprdesk-") && n.ends_with(".conf"))
            })
        }));
    }
    done.extend(remove_gtk_theme(roots, dry));
    for f in files {
        if f.is_file() {
            done.push((f.clone(), Removal::Removed));
            if !dry {
                let _ = std::fs::remove_file(&f);
            }
        }
    }
    if !dry {
        // Dossier des fragments vidé : retiré aussi.
        let _ = std::fs::remove_dir(roots.hypr_theme_dir());
    }
    done
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
        // gtk : les deux blocs, puis le thème installé (Adwaita intégré
        // faute de base : gtk-3.0 seul), son index.theme et gtk-theme.
        let gtk = plan(&find("gtk").unwrap(), &r, None, &roots).unwrap();
        assert_eq!(gtk.len(), 6, "{gtk:?}");
        assert!(
            gtk[2]
                .path
                .ends_with("themes/hyprdesk-regle/gtk-3.0/gtk.css")
        );
        assert!(
            gtk[2]
                .content
                .contains("resource:///org/gtk/libgtk/theme/Adwaita")
        );
        assert_eq!(gtk[5].content, "hyprdesk-regle");
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
            c.path.ends_with("qt6ct/colors/hyprdesk.conf"),
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

    #[test]
    fn fragment_hyprland_sans_commande() {
        // MLG-224 : un gabarit de pack ne pose que des options admises.
        assert!(
            validate_hypr_fragment(
                "# c\ngeneral:col.active_border = rgba(ffffffff)\ndecoration:rounding = 8\n"
            )
            .is_ok()
        );
        for mauvais in [
            "exec = touch /tmp/x",
            "exec-once = x",
            "source = ~/.config/autre.conf",
            "bind = SUPER, Q, exec, x",
            "env = A,B",
            "$var = 1",
            "general {",
        ] {
            assert!(validate_hypr_fragment(mauvais).is_err(), "{mauvais}");
        }
    }

    #[test]
    fn hot_n_envoie_pas_de_commande_glissée_dans_un_gabarit() {
        let root = std::env::temp_dir().join(format!("hd-tgt-hot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let pack = root.join("pack");
        std::fs::create_dir_all(pack.join("templates")).unwrap();
        std::fs::write(
            pack.join("templates/hyprland.tmpl"),
            "decoration:rounding = 8\nexec = touch /tmp/PWNED\n",
        )
        .unwrap();
        let r = super::super::resolve::from_theme(&crate::config::Theme::default());
        assert!(hot_keywords(&r, Some(&pack)).is_err());
        let roots = Roots::at(&root);
        assert!(plan(&find("hyprland").unwrap(), &r, Some(&pack), &roots).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn options_du_pack_dans_le_fragment_persistant() {
        // MLG-224 : les options n'étaient posées qu'en mode hot.
        let root = std::env::temp_dir().join(format!("hd-tgt-opts-{}", std::process::id()));
        let roots = Roots::at(&root);
        let mut r = super::super::resolve::from_theme(&crate::config::Theme::default());
        r.hyprland_options
            .insert("decoration:rounding".to_string(), "10".to_string());
        let planned = plan(&find("hyprland").unwrap(), &r, None, &roots).unwrap();
        assert!(
            planned[0].content.contains("decoration:rounding = 10"),
            "{}",
            planned[0].content
        );
    }

    #[test]
    fn gabarits_hyprlock_et_kitty_sans_commande() {
        assert!(validate_hyprlock("$hd_fg = rgb(1,2,3)\nlabel {\n  text = Bonjour\n}\n").is_ok());
        assert!(validate_hyprlock("label {\n  text = cmd[update:1000] curl x | sh\n}\n").is_err());
        assert!(validate_hyprlock("source = /tmp/x.conf").is_err());
        // Le gabarit intégré passe.
        assert!(validate_kitty(include_str!("templates/kitty.conf.tmpl")).is_ok());
        assert!(
            validate_kitty("foreground #ffffff\ncolor12 #000000\nbackground_opacity 0.9\n").is_ok()
        );
        for mauvais in [
            "shell /bin/sh -c 'touch /tmp/x'",
            "map ctrl+a launch touch /tmp/x",
            "include /tmp/x.conf",
            "watcher /tmp/x.py",
            "scrollback_pager sh -c x",
            "active_tab_title_template {x}",
        ] {
            assert!(validate_kitty(mauvais).is_err(), "{mauvais}");
        }
    }

    #[test]
    fn accent_gnome_le_plus_proche() {
        // MLG-222 : accent-color est une énumération, pas rgb().
        use crate::config::Rgba;
        assert_eq!(gnome_accent(Rgba(0xe6, 0x2d, 0x42, 255)), "red");
        assert_eq!(gnome_accent(Rgba(0x35, 0x84, 0xe4, 255)), "blue");
        assert_eq!(gnome_accent(Rgba(0x3a, 0x94, 0x4a, 255)), "green");
        assert_eq!(gnome_accent(Rgba(0x80, 0x80, 0x80, 255)), "slate");
        assert_eq!(gnome_accent(Rgba(0x91, 0x41, 0xac, 255)), "purple");
        for (nom, _) in GNOME_ACCENTS {
            assert!(nom.chars().all(|c| c.is_ascii_lowercase()));
        }
    }

    #[test]
    fn sorties_qui_lancent_des_commandes_refusées() {
        // MLG-224 : un pack ne pose ni lanceur, ni service, ni script.
        let root = std::env::temp_dir().join(format!("hd-tgt-deny-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        for mauvais in [
            "~/.config/autostart/x.desktop",
            "~/.config/systemd/user/x.service",
            "~/.config/hypr/hyprland.conf",
            "~/.config/kitty/kitty.conf",
            "~/.config/waybar/script.sh",
            "~/.config/nvim/init.lua",
            "~/.local/share/applications/x.desktop",
            "~/.local/share/dbus-1/services/x.service",
            "~/.local/share/hyprdesk/themes/autre/theme.toml",
            "~/.config/waybar/../autostart/x",
        ] {
            assert!(safe_output_path(&roots, mauvais).is_err(), "{mauvais}");
        }
        assert!(safe_output_path(&roots, "~/.config/rofi/hyprdesk.rasi").is_ok());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn racine_xdg_en_lien_admise() {
        // MLG-226 : ~/.config en lien (dotfiles) ne bloque plus les sorties ;
        // un lien qui sort du périmètre reste refusé.
        let root = std::env::temp_dir().join(format!("hd-tgt-lnroot-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("dotfiles/config")).unwrap();
        std::fs::create_dir_all(root.join("data")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(root.join("dotfiles/config"), root.join("config")).unwrap();
        let roots = Roots::at(&root);
        assert!(safe_output_path(&roots, "~/.config/waybar/x.css").is_ok());
        std::fs::create_dir_all(root.join("dotfiles/config/waybar")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("/etc/passwd", root.join("dotfiles/config/waybar/y.css"))
            .unwrap();
        assert!(safe_output_path(&roots, "~/.config/waybar/y.css").is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn désinstallation_retire_ce_que_le_moteur_a_posé() {
        // MLG-230 : la désinstallation laissait le bloc GTK, gtk-4.0, Qt et
        // les marqueurs du moteur.
        let root = std::env::temp_dir().join(format!("hd-tgt-rm-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        std::fs::create_dir_all(roots.config.join("gtk-3.0")).unwrap();
        std::fs::create_dir_all(roots.config.join("kitty")).unwrap();
        std::fs::write(roots.config.join("gtk-3.0/gtk.css"), "/* mes règles */\n").unwrap();
        std::fs::write(roots.config.join("kitty/kitty.conf"), "font_size 11\n").unwrap();
        let mut opts = super::super::apply::Options {
            mode: super::super::apply::Mode::Persist,
            origin: "cli".to_string(),
            ..Default::default()
        };
        opts.target_opts.insert(
            "kitty.conf".to_string(),
            roots.config.join("kitty/kitty.conf").display().to_string(),
        );
        let r = super::super::resolve::from_theme(&crate::config::Theme::default());
        let report = super::super::apply::apply(&r, None, &roots, &opts);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert!(roots.config.join("gtk-4.0/gtk.css").is_file());
        assert!(read_or_empty(&roots.config.join("kitty/kitty.conf")).contains("include"));
        // Essai à blanc : rien ne bouge.
        let plan = remove_all(&roots, &opts.target_opts, true);
        assert!(
            plan.iter()
                .any(|(p, r)| p.ends_with("gtk-4.0/gtk.css") && *r == Removal::Emptied)
        );
        assert!(!plan.is_empty());
        assert!(roots.config.join("gtk-4.0/gtk.css").is_file());
        remove_all(&roots, &opts.target_opts, false);
        assert_eq!(
            std::fs::read_to_string(roots.config.join("gtk-3.0/gtk.css")).unwrap(),
            "/* mes règles */\n"
        );
        // gtk-4.0 n'avait que le thème : retiré.
        assert!(!roots.config.join("gtk-4.0/gtk.css").exists());
        assert_eq!(
            std::fs::read_to_string(roots.config.join("kitty/kitty.conf")).unwrap(),
            "font_size 11\n"
        );
        for f in [
            "kitty/hyprdesk-theme.conf",
            "gtk-3.0/hyprdesk-thunar.css",
            "qt6ct/colors/hyprdesk.conf",
        ] {
            assert!(!roots.config.join(f).exists(), "{f}");
        }
        assert!(!roots.hypr_theme_dir().exists());
        // Rejouer ne fait plus rien.
        assert!(remove_all(&roots, &opts.target_opts, false).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn thème_gtk_installé_recoloré_et_nom_alterné() {
        // MLG-237 / MLG-248 : le thème de base recoloré est installé et
        // sélectionné ; un contenu changé change le nom (rechargement à chaud).
        use super::super::gtktheme::TEST_CURRENT;
        let root = std::env::temp_dir().join(format!("hd-tgt-gtkth-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let base = root.join(".themes/Sombre/gtk-3.20");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(
            base.join("gtk.css"),
            "@define-color theme_bg_color #17161d;\n@define-color theme_fg_color #f8f8f2;\n\
             window { background-color: #17161d; }\n",
        )
        .unwrap();
        TEST_CURRENT.with(|c| *c.borrow_mut() = Some("Sombre".to_string()));
        let r = super::super::resolve::from_theme(&crate::config::Theme::default());
        let gtk = find("gtk").unwrap();
        let p = plan(&gtk, &r, None, &roots).unwrap();
        let css = p
            .iter()
            .find(|x| x.path.ends_with("hyprdesk-regle/gtk-3.0/gtk.css"))
            .unwrap();
        assert!(
            css.content.contains("background-color: #171b27"),
            "{}",
            css.content
        );
        assert!(!css.content.contains("#17161d"));
        let index = p.iter().find(|x| x.path.ends_with("index.theme")).unwrap();
        assert!(index.content.contains("X-Hyprdesk-Base=Sombre"));
        // Thème du moteur posé, contenu identique : même nom.
        for x in &p {
            if x.op == Op::Write && x.path.starts_with(roots.data.join("themes")) {
                std::fs::create_dir_all(x.path.parent().unwrap()).unwrap();
                std::fs::write(&x.path, &x.content).unwrap();
            }
        }
        TEST_CURRENT.with(|c| *c.borrow_mut() = Some("hyprdesk-regle".to_string()));
        let p2 = plan(&gtk, &r, None, &roots).unwrap();
        assert_eq!(p2.last().unwrap().content, "hyprdesk-regle");
        // Base retrouvée par index.theme ; contenu changé : l'autre nom.
        let mut r2 = r.clone();
        r2.palette
            .insert("bg".to_string(), crate::config::Rgba(1, 2, 3, 255));
        let p3 = plan(&gtk, &r2, None, &roots).unwrap();
        assert_eq!(p3.last().unwrap().content, "hyprdesk-regle-bis");
        // Après coup : seul le thème posé reste.
        std::fs::create_dir_all(roots.data.join("themes/hyprdesk-regle-bis")).unwrap();
        after_commit("gtk", &p3, &roots);
        assert!(!roots.data.join("themes/hyprdesk-regle").exists());
        assert!(roots.data.join("themes/hyprdesk-regle-bis").exists());
        TEST_CURRENT.with(|c| *c.borrow_mut() = None);
        let _ = std::fs::remove_dir_all(&root);
    }
}
