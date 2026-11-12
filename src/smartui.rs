//! SmartUI design : les icônes des applications redessinées dans un même
//! style par le fournisseur IA choisi pour cette fonction
//! (`crate::ai_services`, LYRA par défaut).
//!
//! Les icônes validées forment un vrai thème d'icônes freedesktop,
//! `$XDG_DATA_HOME/icons/hyprdesk-smartui/` (hérite du thème choisi), rangé
//! par nom d'icône (`Icon=` du .desktop). Quand l'option est active, il passe
//! avant le thème dans toute la recherche d'icônes de hyprdesk
//! (`launch::lookup_icon`) : lanceur, bureau, SUPER+R, barre, notifications.
//! Une proposition faite à la main attend dans
//! `$XDG_CACHE_HOME/hyprdesk/smartui/propositions/` ; la génération
//! automatique valide d'office. Le SVG rendu par le modèle est vérifié : rien
//! d'actif (script, image ou référence externe), au plus 256 Kio.

use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

/// Nom du thème d'icônes (dossier dans `$XDG_DATA_HOME/icons`).
pub const THEME_NAME: &str = "hyprdesk-smartui";

/// Option active : le thème passe avant celui de l'utilisateur.
static ENABLED: AtomicBool = AtomicBool::new(false);

thread_local! {
    /// Recherche de l'icône d'origine (envoyée au modèle) : thème ignoré.
    static BYPASS: Cell<bool> = const { Cell::new(false) };
}

#[cfg(test)]
thread_local! {
    /// Tests : dossiers et option propres au fil (les variables XDG sont
    /// partagées par les tests en parallèle).
    static TEST: std::cell::RefCell<Option<(PathBuf, bool)>> = const { std::cell::RefCell::new(None) };
}

fn home_dir(var: &str, fallback: &str, sub: &str) -> PathBuf {
    #[cfg(test)]
    if let Some((root, _)) = TEST.with(|t| t.borrow().clone()) {
        return root.join(sub);
    }
    std::env::var_os(var)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(fallback)
        })
        .join(sub)
}

/// Style « HyprDesk », proposé tant que l'utilisateur n'a pas écrit le sien
/// (façon icônes iOS / macOS). Adapté à une icône par application : le nom
/// de l'application est dans la demande, et le logo d'origine est
/// réinterprété, pas recopié.
pub const DEFAULT_STYLE: &str = r#"A modern app icon, in the style of Apple's iOS / macOS app icons.

SHAPE
- Square canvas 1024×1024 px, filled edge to edge, with a continuous-curvature rounded square (squircle, superellipse), not a plain rounded rectangle with circular corners. Corner radius ≈ 22.37% of the width.
- A single, bold, centered glyph with generous breathing room (about 15–20% margin around the symbol). Simple silhouette, readable at 29 px.
- Geometry built on a strict grid: balanced proportions, optical centering, consistent stroke widths, softly rounded terminals.

STYLE
- Minimalist, polished, premium, friendly. One idea, one symbol, no text, no letters.
- Smooth, subtle gradient background (top lighter, bottom more saturated), with a vibrant but harmonious color palette (max 2–3 hues).
- Glyph in a clean flat or semi-flat style, with a very light inner highlight and a soft, diffuse drop shadow to create gentle depth, without heavy skeuomorphism.
- A hint of translucency / frosted glass layering is allowed (Liquid Glass look): soft specular highlight along the top edge, subtle light refraction, delicate inner glow.
- Soft, even, studio-like lighting coming from above. No harsh outlines, no noise, no texture, no bevel overload.

RENDER
- Vector-crisp, ultra-clean edges, high resolution, flat front view, centered, isolated on a transparent background.
- Mood: calm, elegant, precise, "designed by Apple".

NEGATIVE PROMPT
text, letters, watermark, busy background, photo-realism, thick black outlines, harsh shadows, clutter, multiple objects, perspective/3D tilt, drop-shadow outside the icon, Apple logo, verbatim copies of existing app logos"#;

/// Taille maximale d'un SVG accepté (proposition ou original envoyé).
const MAX_SVG: usize = 256 * 1024;

pub fn dir() -> PathBuf {
    home_dir("XDG_CACHE_HOME", ".cache", "hyprdesk/smartui")
}

/// Nom de fichier d'une classe de fenêtre (ou d'un id .desktop).
pub fn slug(class: &str) -> String {
    let s: String = class
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let s = s.trim_start_matches('.').to_string();
    if s.is_empty() { "_".into() } else { s }
}

/// Dossier du thème d'icônes SmartUI.
pub fn theme_dir() -> PathBuf {
    home_dir(
        "XDG_DATA_HOME",
        ".local/share",
        &format!("icons/{THEME_NAME}"),
    )
}

fn apps_dir() -> PathBuf {
    theme_dir().join("scalable/apps")
}

/// Clé d'une icône : son nom (`Icon=`), ou le nom du fichier d'une icône
/// donnée par chemin.
fn key_of(name: &str) -> String {
    let p = Path::new(name);
    let stem = if p.is_absolute() {
        p.file_stem().and_then(|s| s.to_str()).unwrap_or(name)
    } else {
        name
    };
    slug(stem)
}

/// Clé de l'icône d'une application du lanceur (classe ou id .desktop).
pub fn key_for(class: &str, apps: &[crate::launch::AppInfo]) -> String {
    key_of(&crate::launch::icon_name_for_class(class, apps).unwrap_or_else(|| class.to_lowercase()))
}

fn valid_path(key: &str) -> PathBuf {
    apps_dir().join(format!("{}.svg", slug(key)))
}

fn proposal_path(key: &str) -> PathBuf {
    dir()
        .join("propositions")
        .join(format!("{}.svg", slug(key)))
}

/// Option et thème hérité, à chaque configuration appliquée.
pub fn configure(enabled: bool, inherits: Option<&str>) {
    #[cfg(test)]
    if TEST.with(|t| t.borrow().is_some()) {
        TEST.with(|t| {
            if let Some((_, on)) = t.borrow_mut().as_mut() {
                *on = enabled;
            }
        });
        migrate(dir().join("validees"));
        if apps_dir().is_dir() {
            let _ = write_index(inherits);
        }
        return;
    }
    ENABLED.store(enabled, Ordering::Relaxed);
    migrate_old_cache();
    if apps_dir().is_dir() {
        let _ = write_index(inherits);
    }
}

pub fn enabled() -> bool {
    #[cfg(test)]
    if let Some((_, on)) = TEST.with(|t| t.borrow().clone()) {
        return on;
    }
    ENABLED.load(Ordering::Relaxed)
}

/// `index.theme` : un thème freedesktop complet, utilisable aussi par GTK/Qt.
fn write_index(inherits: Option<&str>) -> std::io::Result<()> {
    let inherits = match inherits.map(str::trim).filter(|t| !t.is_empty()) {
        Some(t) if t != THEME_NAME => format!("{t},hicolor"),
        _ => "hicolor".into(),
    };
    let index = format!(
        "[Icon Theme]\n\
         Name=HyprDesk SmartUI\n\
         Comment=Icônes redessinées par SmartUI design (hyprdesk)\n\
         Inherits={inherits}\n\
         Directories=scalable/apps\n\
         \n\
         [scalable/apps]\n\
         Size=128\n\
         MinSize=8\n\
         MaxSize=1024\n\
         Type=Scalable\n\
         Context=Applications\n"
    );
    let path = theme_dir().join("index.theme");
    if std::fs::read_to_string(&path).ok().as_deref() == Some(index.as_str()) {
        return Ok(());
    }
    std::fs::write(path, index)
}

/// Icônes validées avant le thème (`…/smartui/validees/`, par classe) : rangées dans le thème.
fn migrate_old_cache() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| migrate(dir().join("validees")));
}

fn migrate(old: PathBuf) {
    let Ok(list) = std::fs::read_dir(&old) else {
        return;
    };
    let _ = std::fs::create_dir_all(apps_dir());
    for e in list.flatten() {
        let to = apps_dir().join(e.file_name());
        if !to.exists() {
            let _ = std::fs::rename(e.path(), to);
        }
    }
    let _ = std::fs::remove_dir_all(old);
}

/// Icône SmartUI d'un nom d'icône, quand l'option est active (recherche de
/// hyprdesk, `launch::lookup_icon`).
pub fn themed(name: &str) -> Option<PathBuf> {
    if !enabled() || BYPASS.with(Cell::get) || name.is_empty() {
        return None;
    }
    let p = valid_path(&key_of(name));
    p.is_file().then_some(p)
}

/// Icône d'origine d'une application (celle du thème choisi, jamais SmartUI).
pub fn original_icon(
    class: &str,
    apps: &[crate::launch::AppInfo],
    theme: Option<&str>,
) -> Option<PathBuf> {
    BYPASS.with(|b| b.set(true));
    let p = crate::launch::icon_for_class(class, apps, theme, 128);
    BYPASS.with(|b| b.set(false));
    p
}

/// Le fichier vient de SmartUI (relu en mémoire par l'interface : une icône
/// refaite garde le même chemin).
pub fn owns(p: &Path) -> bool {
    p.starts_with(theme_dir()) || p.starts_with(dir())
}

/// Icône validée, si elle existe.
pub fn validated(key: &str) -> Option<PathBuf> {
    let p = valid_path(key);
    p.is_file().then_some(p)
}

/// Proposition en attente de validation.
pub fn proposal(key: &str) -> Option<PathBuf> {
    let p = proposal_path(key);
    p.is_file().then_some(p)
}

/// La proposition entre dans le thème.
pub fn validate(key: &str, inherits: Option<&str>) -> Result<PathBuf, String> {
    let from = proposal_path(key);
    let to = valid_path(key);
    std::fs::create_dir_all(apps_dir()).map_err(|e| e.to_string())?;
    let _ = write_index(inherits);
    std::fs::rename(&from, &to)
        .or_else(|_| std::fs::copy(&from, &to).and_then(|_| std::fs::remove_file(&from)))
        .map_err(|e| e.to_string())?;
    Ok(to)
}

pub fn reject(key: &str) {
    let _ = std::fs::remove_file(proposal_path(key));
}

/// Retire l'icône du thème : l'application reprend celle du thème choisi.
pub fn forget(key: &str) {
    let _ = std::fs::remove_file(valid_path(key));
}

/// Vide le cache et le thème.
pub fn clear() -> std::io::Result<()> {
    for d in [dir(), theme_dir()] {
        match std::fs::remove_dir_all(d) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => return Err(e),
            _ => {}
        }
    }
    Ok(())
}

/// Génération automatique : demandée explicitement, ou par défaut quand
/// LYRA est là.
pub fn auto_on(cfg: &crate::config::Config) -> bool {
    cfg.general
        .launcher
        .smartui_auto
        .unwrap_or_else(crate::assistant::lyra_installed)
}

/// File de la génération automatique (une à la fois ; une application
/// essayée ne l'est qu'une fois par session ; arrêt à la première erreur).
#[derive(Default)]
pub struct Auto {
    pub queue: std::collections::VecDeque<String>,
    pub tried: std::collections::HashSet<String>,
    pub busy: bool,
    pub stopped: bool,
    pub added: usize,
}

pub fn auto() -> std::sync::MutexGuard<'static, Auto> {
    static AUTO: std::sync::LazyLock<std::sync::Mutex<Auto>> =
        std::sync::LazyLock::new(Default::default);
    AUTO.lock().unwrap_or_else(|e| e.into_inner())
}

/// Demande au modèle (système, message), selon l'icône d'origine.
fn request(
    style: &str,
    class: &str,
    name: &str,
    original: Option<&Path>,
) -> crate::ai_services::Request {
    use crate::ai_services::{Attachment, Request};
    let style = if style.trim().is_empty() {
        DEFAULT_STYLE
    } else {
        style.trim()
    };
    let mut attachment = None;
    let source = match original {
        Some(p) if p.extension().is_some_and(|e| e == "svg") => std::fs::read_to_string(p)
            .ok()
            .filter(|s| s.len() <= MAX_SVG)
            .map(|s| format!("\n\nOriginal icon (SVG):\n```svg\n{}\n```", s.trim())),
        Some(p) if p.extension().is_some_and(|e| e == "png") => std::fs::read(p)
            .ok()
            .filter(|b| b.len() <= 2 * 1024 * 1024)
            .map(|bytes| {
                attachment = Some(Attachment::Png {
                    path: p.to_path_buf(),
                    bytes,
                });
                "\n\nThe original icon is attached as an image.".to_string()
            }),
        _ => None,
    }
    .unwrap_or_else(|| {
        "\n\nNo original icon is available: design one from the application name.".into()
    });
    Request {
        system: "You are an icon designer for a Linux desktop. You redraw application icons \
                 so that every icon of the dock shares one consistent visual style. You answer \
                 with one standalone SVG document and nothing else."
            .into(),
        prompt: format!(
            "Redraw the icon of the application \"{name}\" (window class \"{class}\") in this style:\n\
             {style}\n\n\
             Requirements:\n\
             - Keep it instantly recognisable: reinterpret the original's emblematic symbol and dominant colour as one simplified glyph, without copying the logo verbatim.\n\
             - Root element <svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 1024 1024\" width=\"1024\" height=\"1024\">.\n\
             - Shadows, highlights and glass effects with SVG gradients, opacity and filters (feGaussianBlur, feOffset) only.\n\
             - Vector shapes and gradients only: no <text>, no <image>, no external reference, no script, no CSS @import.\n\
             - Answer with the SVG document only: no explanation, no Markdown fence.{source}"
        ),
        attachment,
    }
}

/// Extrait le SVG de la réponse du modèle et le vérifie.
pub fn extract_svg(reply: &str) -> Result<String, String> {
    // Minuscules ASCII : mêmes positions d'octets que la réponse.
    let lower = reply.to_ascii_lowercase();
    let start = lower
        .find("<svg")
        .ok_or_else(|| crate::i18n::tr("la réponse ne contient pas de SVG").to_string())?;
    let end = lower
        .rfind("</svg>")
        .filter(|e| *e > start)
        .ok_or_else(|| crate::i18n::tr("SVG incomplet (réponse coupée ?)").to_string())?
        + "</svg>".len();
    let mut svg = reply[start..end].trim().to_string();
    if svg.len() > MAX_SVG {
        return Err(crate::i18n::tr("SVG trop lourd").into());
    }
    let l = svg.to_ascii_lowercase();
    let active = [
        "<script",
        "<foreignobject",
        "<image",
        "javascript:",
        "@import",
        "<iframe",
    ]
    .iter()
    .any(|m| l.contains(m))
        || l.contains("href=\"http")
        || l.contains("href='http")
        || l.contains("url(http")
        || has_event_attribute(&l);
    if active {
        return Err(crate::i18n::tr("SVG refusé : contenu actif ou externe").into());
    }
    // usvg exige l'espace de noms SVG.
    let head_end = l.find('>').unwrap_or(l.len());
    if !l[..head_end].contains("xmlns=") {
        svg.insert_str(4, " xmlns=\"http://www.w3.org/2000/svg\"");
    }
    Ok(svg)
}

/// Un attribut d'événement (`onload=`, `onclick =`…).
fn has_event_attribute(l: &str) -> bool {
    let b = l.as_bytes();
    l.match_indices(" on").any(|(i, _)| {
        let rest = &b[i + 3..];
        let n = rest.iter().take_while(|c| c.is_ascii_alphabetic()).count();
        n > 0
            && rest[n..]
                .iter()
                .find(|c| !c.is_ascii_whitespace())
                .is_some_and(|c| *c == b'=')
    })
}

/// Génère une proposition pour la classe (bloquant) ; rend son chemin.
pub fn generate(
    cfg: &crate::config::Config,
    key: &str,
    class: &str,
    name: &str,
    original: Option<&Path>,
) -> Result<PathBuf, String> {
    let req = request(&cfg.general.launcher.smartui_style, class, name, original);
    let reply = crate::ai_services::ask(cfg, "smartui", &req)?;
    let svg = extract_svg(&reply)?;
    let path = proposal_path(key);
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, svg).map_err(|e| e.to_string())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Thème : index, recherche par nom ou chemin, option coupée, migration.
    #[test]
    fn theme_d_icones() {
        let d = std::env::temp_dir().join(format!("hyprdesk-smartui-theme-{}", std::process::id()));
        TEST.with(|t| *t.borrow_mut() = Some((d.clone(), false)));
        std::fs::create_dir_all(dir().join("validees")).unwrap();
        std::fs::write(dir().join("validees/kitty.svg"), "<svg/>").unwrap();
        configure(true, Some("Papirus-Dark"));
        assert!(validated("kitty").is_some(), "migré dans le thème");
        let index = std::fs::read_to_string(theme_dir().join("index.theme")).unwrap();
        assert!(
            index.contains("Inherits=Papirus-Dark,hicolor") && index.contains("[scalable/apps]")
        );
        assert_eq!(themed("kitty"), validated("kitty"));
        assert_eq!(themed("/opt/kitty/kitty.png"), validated("kitty"));
        assert!(themed("firefox").is_none());
        configure(false, None);
        assert!(themed("kitty").is_none(), "option coupée");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn noms_de_fichier() {
        assert_eq!(slug("org.mozilla.Firefox"), "org.mozilla.firefox");
        assert_eq!(slug("../etc/passwd"), "_etc_passwd");
        assert_eq!(slug("  "), "_");
        assert_eq!(slug("Code - OSS"), "code_-_oss");
    }

    #[test]
    fn svg_extrait_de_la_reponse() {
        let r = "Voici l'icône :\n```svg\n<svg viewBox=\"0 0 128 128\"><rect width=\"128\" height=\"128\"/></svg>\n```";
        let s = extract_svg(r).unwrap();
        assert!(
            s.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox"),
            "{s}"
        );
        assert!(s.ends_with("</svg>"));
        let ok = "<svg xmlns=\"http://www.w3.org/2000/svg\"><defs><linearGradient id=\"g\"/></defs>\
                  <rect fill=\"url(#g)\"/></svg>";
        assert_eq!(extract_svg(ok).unwrap(), ok);
        assert!(extract_svg("pas d'icône").is_err());
        assert!(extract_svg("<svg><rect/>").is_err(), "coupé");
    }

    #[test]
    fn svg_actif_refuse() {
        for bad in [
            "<svg><script>alert(1)</script></svg>",
            "<svg><image href=\"http://x/a.png\"/></svg>",
            "<svg onload=\"x()\"></svg>",
            "<svg><rect onclick = \"x()\"/></svg>",
            "<svg><style>@import url(http://x);</style></svg>",
            "<svg><a href=\"javascript:x()\"/></svg>",
        ] {
            assert!(extract_svg(bad).is_err(), "{bad}");
        }
        // « one », « opacity » : pas des événements.
        assert!(extract_svg("<svg><g data-one=\"1\" opacity=\"0.5\"/></svg>").is_ok());
    }

    /// Génération réelle par le service par défaut (LYRA, avec l'abonnement
    /// de `HYPRDESK_SMARTUI_AI`, défaut claude) : cache dans un dossier temporaire.
    /// `cargo test generation_reelle -- --ignored --nocapture`
    #[test]
    #[ignore = "appelle le service IA (abonnement ou clé de l'utilisateur)"]
    fn generation_reelle() {
        let d = std::env::temp_dir().join(format!("hyprdesk-smartui-reel-{}", std::process::id()));
        // SAFETY : test ignoré, lancé seul.
        unsafe { std::env::set_var("XDG_CACHE_HOME", &d) };
        let mut cfg = crate::config::Config::default();
        let ai = std::env::var("HYPRDESK_SMARTUI_AI").unwrap_or_else(|_| "claude".into());
        cfg.general.apps.insert("ai".into(), ai);
        let original = crate::launch::icon_for_class("kitty", &[], None, 128);
        eprintln!("original : {original:?}");
        let started = std::time::Instant::now();
        let p = generate(&cfg, "kitty", "kitty", "kitty", original.as_deref()).expect("génération");
        let svg = std::fs::read_to_string(&p).unwrap();
        eprintln!("{} octets en {:?} :\n{svg}", svg.len(), started.elapsed());
        assert!(svg.starts_with("<svg") && svg.ends_with("</svg>"));
        assert!(proposal("kitty").is_some());
        unsafe { std::env::set_var("XDG_DATA_HOME", &d) };
        validate("kitty", None).unwrap();
        assert!(validated("kitty").is_some() && proposal("kitty").is_none());
        configure(true, Some("Papirus"));
        assert!(themed("kitty").is_some() && themed("/usr/share/pixmaps/kitty.png").is_some());
        assert!(original_icon("kitty", &[], None).is_none_or(|p| !owns(&p)));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn demande_selon_l_original() {
        let r = request("", "firefox", "Firefox", None);
        assert!(r.prompt.contains(DEFAULT_STYLE) && r.prompt.contains("No original icon"));
        assert!(r.attachment.is_none());
        let d = std::env::temp_dir().join(format!("hyprdesk-smartui-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let svg = d.join("a.svg");
        std::fs::write(&svg, "<svg><circle r=\"4\"/></svg>").unwrap();
        let r = request("plat", "a", "A", Some(&svg));
        assert!(r.prompt.contains("<circle r=\"4\"/>") && r.prompt.contains("plat"));
        let png = d.join("a.png");
        std::fs::write(&png, [137u8, 80, 78, 71]).unwrap();
        let r = request("plat", "a", "A", Some(&png));
        assert!(r.attachment.is_some() && r.prompt.contains("attached"));
        let _ = std::fs::remove_dir_all(&d);
    }
}
