//! Couche forte GTK (MLG-237, MLG-248) : un vrai thème installé,
//! `~/.local/share/themes/hyprdesk-<pack>/`, sélectionné par `gtk-theme`.
//!
//! Les thèmes GTK courants sont des feuilles compilées (Sass) : des milliers
//! de couleurs littérales, presque aucune couleur nommée — redéfinir des noms
//! ne les recolore pas. Le moteur part donc du **thème de base** (celui que
//! l'utilisateur avait choisi) et le recolore : chaque couleur est rapportée
//! à son repère (fond, surface, texte, accent, états, déclarés par le thème
//! lui-même avec `@define-color`) et transposée sur la palette du pack, en
//! OKLCH — les nuances (survol, bordures, ombres) suivent.
//!
//! Le changement de nom du thème est ce qui fait relire leurs feuilles aux
//! applications GTK 3 ouvertes (Brave, Firefox…) : le thème alterne entre
//! `hyprdesk-<pack>` et `hyprdesk-<pack>-bis` quand son contenu change.
//! `index.theme` garde le nom du thème de base (`X-Hyprdesk-Base`), qui est
//! remis à la désinstallation.

use std::path::{Path, PathBuf};

use crate::config::Rgba;

use super::color::{self, Oklch};
use super::paths::Roots;
use super::resolve::Resolved;

/// Préfixe des thèmes installés par le moteur.
pub const PREFIX: &str = "hyprdesk-";

/// Clé d'`index.theme` qui garde le thème de base.
const BASE_KEY: &str = "X-Hyprdesk-Base";

/// Taille maximale lue d'un thème de base (imports compris).
const MAX_BASE_BYTES: usize = 8 * 1024 * 1024;

#[cfg(test)]
thread_local! {
    /// `gtk-theme` vu par [`current_theme`] dans le fil d'un essai : jamais
    /// celui de la session.
    pub static TEST_CURRENT: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// `org.gnome.desktop.interface gtk-theme` en place.
pub fn current_theme() -> Option<String> {
    #[cfg(test)]
    {
        TEST_CURRENT.with(|c| c.borrow().clone())
    }
    #[cfg(not(test))]
    {
        let out = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "gtk-theme"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let v = v.trim_matches('\'').to_string();
        (!v.is_empty()).then_some(v)
    }
}

/// Dossiers où GTK cherche les thèmes, par priorité.
pub fn theme_dirs(roots: &Roots) -> Vec<PathBuf> {
    let mut dirs = vec![roots.data.join("themes")];
    if let Some(home) = roots.config.parent() {
        dirs.push(home.join(".themes"));
    }
    dirs.extend(roots.data_system.iter().map(|d| d.join("themes")));
    dirs
}

/// Dossier d'un thème par son nom.
pub fn find_theme(roots: &Roots, name: &str) -> Option<PathBuf> {
    if name.is_empty() || name.contains('/') {
        return None;
    }
    theme_dirs(roots)
        .into_iter()
        .map(|d| d.join(name))
        .find(|p| p.is_dir())
}

/// Thème de base : celui en place s'il n'est pas un thème du moteur, sinon
/// celui que le thème du moteur en place a recoloré, sinon Adwaita.
pub fn base_name(roots: &Roots, current: Option<&str>) -> String {
    match current {
        Some(c) if !c.starts_with(PREFIX) => c.to_string(),
        Some(c) => find_theme(roots, c)
            .and_then(|dir| std::fs::read_to_string(dir.join("index.theme")).ok())
            .and_then(|t| {
                t.lines()
                    .find_map(|l| l.strip_prefix(&format!("{BASE_KEY}=")))
                    .map(|v| v.trim().to_string())
            })
            .filter(|v| !v.is_empty() && !v.starts_with(PREFIX))
            .unwrap_or_else(|| "Adwaita".to_string()),
        None => "Adwaita".to_string(),
    }
}

/// Feuille que GTK chargerait pour ce thème : `gtk-<major>.<minor>` du plus
/// récent au plus ancien (comme GTK), `gtk-dark.css` en variante sombre si
/// le thème en porte une.
pub fn base_sheet(theme_dir: &Path, major: u8, dark: bool) -> Option<PathBuf> {
    let max_minor = if major == 3 { 24 } else { 20 };
    for minor in (0..=max_minor).rev().step_by(2) {
        let dir = theme_dir.join(format!("gtk-{major}.{minor}"));
        if !dir.is_dir() {
            continue;
        }
        if dark && dir.join("gtk-dark.css").is_file() {
            return Some(dir.join("gtk-dark.css"));
        }
        if dir.join("gtk.css").is_file() {
            return Some(dir.join("gtk.css"));
        }
    }
    None
}

/// Feuille de base lue, imports relatifs déroulés, `url()` relatives
/// rendues absolues (les images du thème de base restent les siennes).
pub fn read_sheet(path: &Path) -> Option<String> {
    let mut budget = MAX_BASE_BYTES;
    read_inlined(path, 0, &mut budget)
}

fn read_inlined(path: &Path, depth: usize, budget: &mut usize) -> Option<String> {
    if depth > 8 {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    if text.len() > *budget {
        return None;
    }
    *budget -= text.len();
    let dir = path.parent()?.to_path_buf();
    let mut out = String::with_capacity(text.len());
    let major = if path.to_string_lossy().contains("/gtk-4.") {
        4
    } else {
        3
    };
    for line in text.split_inclusive('\n') {
        let t = line.trim();
        // Feuille embarquée dans GTK (`resource:///…`, ainsi la base
        // Adwaita-dark) : extraite pour être recolorée elle aussi.
        if let Some(inner) = t
            .strip_prefix("@import")
            .and_then(|r| url_target(r.trim()))
            .and_then(|u| u.strip_prefix("resource://").map(str::to_string))
            .and_then(|res| gtk_resource(major, &res))
        {
            if inner.len() > *budget {
                return None;
            }
            *budget -= inner.len();
            out.push_str(&inner);
            out.push('\n');
            continue;
        }
        if let Some(target) = t
            .strip_prefix("@import")
            .and_then(|r| url_target(r.trim()))
            .filter(|u| is_relative(u))
        {
            match read_inlined(&dir.join(&target), depth + 1, budget) {
                Some(inner) => {
                    out.push_str(&inner);
                    out.push('\n');
                }
                // Introuvable : l'import reste, en chemin absolu.
                None => out.push_str(&format!(
                    "@import url(\"{}\");\n",
                    dir.join(&target).display()
                )),
            }
            continue;
        }
        out.push_str(&absolute_urls(line, &dir));
    }
    Some(out)
}

/// Bibliothèque GTK `major` (celle qui porte ses feuilles embarquées).
fn libgtk(major: u8) -> Option<PathBuf> {
    let name = if major == 4 {
        "libgtk-4.so.1"
    } else {
        "libgtk-3.so.0"
    };
    [
        "/usr/lib",
        "/usr/lib64",
        "/usr/lib/x86_64-linux-gnu",
        "/usr/lib/aarch64-linux-gnu",
    ]
    .iter()
    .map(|d| Path::new(d).join(name))
    .find(|p| p.is_file())
}

/// Feuille embarquée dans GTK (`/org/gtk/libgtk/…`), extraite par
/// `gresource` ; ses `url()` relatives pointent sur ses ressources.
pub fn gtk_resource(major: u8, path: &str) -> Option<String> {
    if !path.starts_with("/org/gtk/libgtk/") {
        return None;
    }
    let lib = libgtk(major)?;
    let out = std::process::Command::new("gresource")
        .arg("extract")
        .arg(&lib)
        .arg(path)
        .stderr(std::process::Stdio::null())
        .output()
        .ok()
        .filter(|o| o.status.success())?;
    let css = String::from_utf8(out.stdout)
        .ok()
        .filter(|c| has_rules(c))?;
    let dir = path.rsplit_once('/').map_or("", |(d, _)| d);
    Some(
        css.split_inclusive('\n')
            .map(|l| resource_urls(l, dir))
            .collect(),
    )
}

/// Adwaita de GTK 3, embarqué dans la bibliothèque : sa feuille
/// (`gtk-contained[-dark].css`), extraite.
pub fn builtin_adwaita(dark: bool) -> Option<String> {
    gtk_resource(
        3,
        &format!(
            "/org/gtk/libgtk/theme/Adwaita/gtk-contained{}.css",
            if dark { "-dark" } else { "" }
        ),
    )
}

/// Une feuille porte-t-elle des règles ? Les talons d'Adwaita dans
/// `/usr/share/themes` (« this file is no longer used ») n'en ont pas :
/// recolorés, ils donneraient un thème sans style — menus transparents.
pub fn has_rules(css: &str) -> bool {
    let mut rest = css;
    let mut code = String::new();
    while let Some(i) = rest.find("/*") {
        code.push_str(&rest[..i]);
        rest = rest[i + 2..].split_once("*/").map_or("", |(_, r)| r);
    }
    code.push_str(rest);
    code.contains('{') || code.contains("@import")
}

/// `url(relatif)` → `url("resource://<dir>/relatif")`.
fn resource_urls(line: &str, dir: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(i) = rest.find("url(") {
        out.push_str(&rest[..i]);
        let after = &rest[i..];
        match (url_target(after), after.find(')')) {
            (Some(target), Some(end)) if is_relative(&target) => {
                out.push_str(&format!("url(\"resource://{dir}/{target}\")"));
                rest = &after[end + 1..];
            }
            (_, Some(end)) => {
                out.push_str(&after[..=end]);
                rest = &after[end + 1..];
            }
            _ => {
                out.push_str(after);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// Cible d'un `url("…")`, `url('…')` ou `url(…)` en tête de `s`.
fn url_target(s: &str) -> Option<String> {
    let inner = s.strip_prefix("url(")?;
    let end = inner.find(')')?;
    Some(inner[..end].trim().trim_matches(['"', '\'']).to_string())
}

fn is_relative(u: &str) -> bool {
    !(u.starts_with('/') || u.contains(':'))
}

/// `url(relatif)` → `url("absolu")`, relatif au dossier de la feuille.
fn absolute_urls(line: &str, dir: &Path) -> String {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(i) = rest.find("url(") {
        out.push_str(&rest[..i]);
        let after = &rest[i..];
        match (url_target(after), after.find(')')) {
            (Some(target), Some(end)) if is_relative(&target) => {
                out.push_str(&format!("url(\"{}\")", dir.join(&target).display()));
                rest = &after[end + 1..];
            }
            (_, Some(end)) => {
                out.push_str(&after[..=end]);
                rest = &after[end + 1..];
            }
            _ => {
                out.push_str(after);
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// Repères d'un thème : sa couleur et celle du pack.
#[derive(Debug, Clone, Copy)]
struct Anchor {
    base: Rgba,
    target: Rgba,
}

/// Correspondance couleurs du thème de base → palette du pack.
#[derive(Debug, Clone)]
pub struct Recolor {
    bg: Anchor,
    fg: Anchor,
    /// Repères exacts (fond, surface, texte, accent, états).
    exact: Vec<Anchor>,
    /// Repères chromatiques (accent, états), par teinte.
    chromatic: Vec<Anchor>,
}

/// Noms de couleurs du thème de base, par slot du moteur.
const NAMES: &[(&str, &[&str])] = &[
    ("bg", &["theme_bg_color", "bg_color", "window_bg_color"]),
    ("fg", &["theme_fg_color", "fg_color", "window_fg_color"]),
    (
        "surface",
        &["theme_base_color", "base_color", "view_bg_color"],
    ),
    (
        "accent",
        &[
            "theme_selected_bg_color",
            "selected_bg_color",
            "accent_bg_color",
            "accent_color",
        ],
    ),
    ("surface_raised", &["card_bg_color", "popover_bg_color"]),
    ("warning", &["warning_color", "warning_bg_color"]),
    (
        "danger",
        &["error_color", "destructive_bg_color", "error_bg_color"],
    ),
    ("success", &["success_color", "success_bg_color"]),
];

impl Recolor {
    /// Repères lus dans la feuille de base (`@define-color nom littéral;`) ;
    /// `None` sans fond ni texte reconnus.
    pub fn new(sheet: &str, resolved: &Resolved) -> Option<Self> {
        let defs = defines(sheet);
        let lookup = |names: &[&str]| {
            names
                .iter()
                .find_map(|n| defs.iter().find(|(k, _)| k == n).map(|(_, c)| *c))
        };
        let mut found = Vec::new();
        for (slot, names) in NAMES {
            if let (Some(base), Some(target)) = (lookup(names), resolved.palette.get(*slot)) {
                found.push((
                    *slot,
                    Anchor {
                        base: Rgba(base.0, base.1, base.2, 255),
                        target: Rgba(target.0, target.1, target.2, 255),
                    },
                ));
            }
        }
        let get = |s: &str| found.iter().find(|(k, _)| *k == s).map(|(_, a)| *a);
        let (bg, fg) = (get("bg")?, get("fg")?);
        let chromatic = found
            .iter()
            .filter(|(k, _)| matches!(*k, "accent" | "warning" | "danger" | "success"))
            .map(|(_, a)| *a)
            .filter(|a| color::to_oklch(a.base).c >= 0.04)
            .collect();
        Some(Self {
            bg,
            fg,
            exact: found.iter().map(|(_, a)| *a).collect(),
            chromatic,
        })
    }

    /// Couleur transposée (alpha gardé).
    pub fn map(&self, c: Rgba) -> Rgba {
        let opaque = Rgba(c.0, c.1, c.2, 255);
        if let Some(a) = self.exact.iter().find(|a| a.base == opaque) {
            return Rgba(a.target.0, a.target.1, a.target.2, c.3);
        }
        // Voiles noirs (ombres) : universels, gardés.
        if opaque == Rgba(0, 0, 0, 255) && c.3 < 255 {
            return c;
        }
        // Voiles blancs (reflets) : dans le sens du texte du pack.
        if opaque == Rgba(255, 255, 255, 255) && c.3 < 255 {
            let f = self.fg.target;
            return Rgba(f.0, f.1, f.2, c.3);
        }
        let o = color::to_oklch(opaque);
        let (bb, bf) = (color::to_oklch(self.bg.base), color::to_oklch(self.fg.base));
        let (tb, tf) = (
            color::to_oklch(self.bg.target),
            color::to_oklch(self.fg.target),
        );
        // Échelle de luminosité fond → texte (s'inverse d'un thème sombre à
        // un thème clair : les survols éclaircis deviennent assombris).
        let span = bf.l - bb.l;
        let scale = if span.abs() < 1e-3 {
            1.0
        } else {
            (tf.l - tb.l) / span
        };
        let mapped = if o.c < 0.04 || self.chromatic.is_empty() {
            let t = if span.abs() < 1e-3 {
                0.0
            } else {
                (o.l - bb.l) / span
            };
            let tc = t.clamp(0.0, 1.0);
            Oklch {
                l: (tb.l + t * (tf.l - tb.l)).clamp(0.0, 1.0),
                c: tb.c + tc * (tf.c - tb.c),
                h: if tc < 0.5 { tb.h } else { tf.h },
            }
        } else {
            let écart = |h: f64| {
                let d = (o.h - h).rem_euclid(360.0);
                d.min(360.0 - d)
            };
            let a = self
                .chromatic
                .iter()
                .min_by(|x, y| {
                    écart(color::to_oklch(x.base).h).total_cmp(&écart(color::to_oklch(y.base).h))
                })
                .copied()
                .unwrap_or(self.bg);
            let (ab, at) = (color::to_oklch(a.base), color::to_oklch(a.target));
            Oklch {
                l: (at.l + (o.l - ab.l) * scale).clamp(0.0, 1.0),
                c: if ab.c > 1e-3 {
                    (o.c * at.c / ab.c).clamp(0.0, 0.37)
                } else {
                    at.c
                },
                h: (at.h + (o.h - ab.h)).rem_euclid(360.0),
            }
        };
        let r = color::from_oklch(mapped);
        Rgba(r.0, r.1, r.2, c.3)
    }

    /// Feuille recolorée : chaque couleur littérale (`#rgb`, `#rrggbb`,
    /// `#rrggbbaa`, `rgb()`, `rgba()`) transposée, le reste intact.
    pub fn sheet(&self, css: &str) -> String {
        recolor_literals(css, |c| self.map(c))
    }
}

/// `@define-color nom littéral;` de la feuille (couleurs littérales seules).
fn defines(css: &str) -> Vec<(String, Rgba)> {
    css.lines()
        .filter_map(|l| {
            let rest = l.trim().strip_prefix("@define-color")?;
            let rest = rest.trim().trim_end_matches(';').trim();
            let (name, value) = rest.split_once(char::is_whitespace)?;
            let value = value.trim();
            let (c, len) = literal_at(value)?;
            (len == value.len()).then(|| (name.to_string(), c))
        })
        .collect()
}

/// Couleur littérale en tête de `s` et sa longueur.
fn literal_at(s: &str) -> Option<(Rgba, usize)> {
    if let Some(hex) = s.strip_prefix('#') {
        let n = hex.chars().take_while(|c| c.is_ascii_hexdigit()).count();
        let h = &hex[..n];
        let byte = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok();
        let nib = |i: usize| u8::from_str_radix(&h[i..i + 1], 16).ok().map(|v| v * 17);
        let c = match n {
            3 => Rgba(nib(0)?, nib(1)?, nib(2)?, 255),
            4 => Rgba(nib(0)?, nib(1)?, nib(2)?, nib(3)?),
            6 => Rgba(byte(0)?, byte(2)?, byte(4)?, 255),
            8 => Rgba(byte(0)?, byte(2)?, byte(4)?, byte(6)?),
            _ => return None,
        };
        return Some((c, n + 1));
    }
    let inner_start = if s.starts_with("rgba(") {
        5
    } else if s.starts_with("rgb(") {
        4
    } else {
        return None;
    };
    let end = s.find(')')?;
    let parts: Vec<&str> = s[inner_start..end].split(',').map(str::trim).collect();
    if parts.len() != 3 && parts.len() != 4 {
        return None;
    }
    let chan = |p: &str| -> Option<u8> {
        match p.strip_suffix('%') {
            Some(pct) => pct
                .parse::<f64>()
                .ok()
                .map(|v| (v * 2.55).round().clamp(0.0, 255.0) as u8),
            None => p
                .parse::<f64>()
                .ok()
                .map(|v| v.round().clamp(0.0, 255.0) as u8),
        }
    };
    let a = match parts.get(3) {
        Some(p) => (p.parse::<f64>().ok()? * 255.0).round().clamp(0.0, 255.0) as u8,
        None => 255,
    };
    Some((
        Rgba(chan(parts[0])?, chan(parts[1])?, chan(parts[2])?, a),
        end + 1,
    ))
}

/// Remplace chaque couleur littérale du texte. Le format d'origine est
/// gardé (hexadécimal ou `rgb()`/`rgba()`, alpha en décimal).
fn recolor_literals(css: &str, map: impl Fn(Rgba) -> Rgba) -> String {
    let bytes = css.as_bytes();
    let mut out = String::with_capacity(css.len());
    let mut i = 0;
    let mut last = 0;
    while i < bytes.len() {
        let b = bytes[i];
        let start_ok = i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'-');
        let candidate = (b == b'#' || (start_ok && (b == b'r'))) && css.is_char_boundary(i);
        if candidate && let Some((c, len)) = literal_at(&css[i..]) {
            // `#` suivi d'un identifiant (sélecteur `#id`) : pas une couleur.
            let next = bytes.get(i + len).copied().unwrap_or(b' ');
            let fin_ok = !(next.is_ascii_alphanumeric() || next == b'-' || next == b'_');
            if fin_ok && !(b == b'#' && in_selector(css, i)) {
                out.push_str(&css[last..i]);
                let m = map(c);
                if b == b'#' {
                    if m.3 == 255 {
                        out.push_str(&format!("#{:02x}{:02x}{:02x}", m.0, m.1, m.2));
                    } else {
                        out.push_str(&format!("#{:02x}{:02x}{:02x}{:02x}", m.0, m.1, m.2, m.3));
                    }
                } else if let Some(alpha) = alpha_text(&css[i..i + len]) {
                    // Alpha d'origine gardé à l'identique (pas d'arrondi à
                    // l'octet : 0.1 reste 0.1).
                    out.push_str(&format!("rgba({}, {}, {}, {alpha})", m.0, m.1, m.2));
                } else {
                    out.push_str(&format!("rgb({}, {}, {})", m.0, m.1, m.2));
                }
                i += len;
                last = i;
                continue;
            }
        }
        i += 1;
    }
    out.push_str(&css[last..]);
    out
}

/// Quatrième composante d'un `rgba(…)`/`rgb(…)` littéral, telle qu'écrite.
fn alpha_text(lit: &str) -> Option<&str> {
    let inner = lit.split_once('(')?.1.strip_suffix(')')?;
    let parts: Vec<&str> = inner.split(',').collect();
    (parts.len() == 4).then(|| parts[3].trim())
}

/// Position dans un sélecteur (hors d'un bloc `{ … }`) : `#id`, pas une
/// couleur.
fn in_selector(css: &str, i: usize) -> bool {
    let before = &css[..i];
    let open = before.rfind('{');
    let close = before.rfind('}');
    match (open, close) {
        (Some(o), Some(c)) => c > o,
        (Some(_), None) => false,
        _ => !before.trim_start().starts_with("@define-color") && !line_is_define(before),
    }
}

fn line_is_define(before: &str) -> bool {
    before
        .rsplit('\n')
        .next()
        .is_some_and(|l| l.trim_start().starts_with("@define-color"))
}

/// Une feuille du thème installé : le thème de base recoloré, puis le bloc
/// de couleurs nommées du moteur (pour les règles qui les utilisent).
pub fn render_sheet(base: Option<&str>, resolved: &Resolved, named_block: &str) -> String {
    let head = format!(
        "/* Généré par hyprdesk-theme-engine — pack {} ({}) : thème de base recoloré. */\n",
        resolved.name, resolved.variant
    );
    match base.and_then(|css| Recolor::new(css, resolved).map(|r| (css, r))) {
        // Recoloré : ses propres couleurs nommées le sont aussi (ce sont des
        // littéraux), rien à redéfinir par-dessus.
        Some((css, r)) => format!("{head}{}", r.sheet(css)),
        // Repères introuvables : la feuille de base telle quelle, couleurs
        // nommées redéfinies par-dessus (effet partiel).
        None => format!("{head}{}\n\n{named_block}", base.unwrap_or_default()),
    }
}

/// `index.theme` du thème installé.
pub fn index_theme(name: &str, resolved: &Resolved, base: &str) -> String {
    format!(
        "[Desktop Entry]\nType=X-GNOME-Metatheme\nName={name}\nComment=Généré par \
         hyprdesk-theme-engine : {base} aux couleurs du pack {}.\nEncoding=UTF-8\n\
         {BASE_KEY}={base}\n\n[X-GNOME-Metatheme]\nGtkTheme={name}\n",
        resolved.name
    )
}

/// Thèmes du moteur installés (dossiers `hyprdesk-*` de `~/.local/share/themes`).
pub fn installed(roots: &Roots) -> Vec<PathBuf> {
    let Ok(rd) = std::fs::read_dir(roots.data.join("themes")) else {
        return Vec::new();
    };
    rd.flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with(PREFIX))
        })
        .collect()
}

/// Retire les thèmes du moteur autres que `keep`.
pub fn prune(roots: &Roots, keep: &str) {
    for dir in installed(roots) {
        if dir.file_name().and_then(|n| n.to_str()) != Some(keep) {
            let _ = std::fs::remove_dir_all(&dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack() -> Resolved {
        super::super::resolve::from_adjusted(
            Rgba(0x1f, 0x23, 0x30, 0xff),
            Rgba(0xf3, 0xf4, 0xf8, 0xff),
            Rgba(0x5b, 0x5b, 0xd6, 0xff),
            Rgba(0xe0, 0xe2, 0xea, 0xff),
        )
    }

    const BASE: &str = "@define-color theme_bg_color #17161d;\n\
        @define-color theme_fg_color #f8f8f2;\n\
        @define-color theme_base_color #22212c;\n\
        @define-color theme_selected_bg_color rgba(149, 128, 255, 0.5);\n\
        window { background-color: #17161d; color: #f8f8f2; }\n\
        button:hover { background-color: #2a2836; box-shadow: 0 1px rgba(0, 0, 0, 0.1); }\n\
        #toolbar { border-color: #9580ff; }\n\
        check { background-image: url(\"../assets/c.png\"); }\n";

    #[test]
    fn couleurs_lues_dans_tous_les_formats() {
        assert_eq!(literal_at("#666"), Some((Rgba(0x66, 0x66, 0x66, 255), 4)));
        assert_eq!(
            literal_at("#17161d;"),
            Some((Rgba(0x17, 0x16, 0x1d, 255), 7))
        );
        assert_eq!(
            literal_at("rgba(149, 128, 255, 0.5)"),
            Some((Rgba(149, 128, 255, 128), 24))
        );
        assert_eq!(
            literal_at("rgb(10%, 0%, 100%)").unwrap().0,
            Rgba(26, 0, 255, 255)
        );
    }

    #[test]
    fn thème_sombre_recoloré_en_pack_clair() {
        let r = Recolor::new(BASE, &pack()).unwrap();
        let out = r.sheet(BASE);
        // Repères exacts : fond et texte du pack.
        assert!(out.contains("background-color: #f3f4f8"), "{out}");
        assert!(out.contains("color: #1f2330"), "{out}");
        // Ombre noire gardée ; sélecteur #toolbar intact ; accent transposé.
        assert!(out.contains("rgba(0, 0, 0, 0.1)"), "{out}");
        assert!(out.contains("#toolbar {"), "{out}");
        assert!(!out.contains("#9580ff"), "{out}");
        // Survol éclairci sur fond sombre → assombri sur fond clair.
        let hover = r.map(Rgba(0x2a, 0x28, 0x36, 255));
        assert!(color::luminance(hover) < color::luminance(Rgba(0xf3, 0xf4, 0xf8, 255)));
        // Accent à 50 % : teinte du pack, alpha gardé.
        let sel = r.map(Rgba(149, 128, 255, 128));
        assert_eq!(sel.3, 128);
        assert_eq!(Rgba(sel.0, sel.1, sel.2, 255), Rgba(0x5b, 0x5b, 0xd6, 255));
    }

    #[test]
    fn feuille_de_base_imports_et_images() {
        let root = std::env::temp_dir().join(format!("hd-gtkth-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let d = root.join("Base/gtk-3.20");
        std::fs::create_dir_all(&d).unwrap();
        std::fs::create_dir_all(root.join("Base/gtk-3.0")).unwrap();
        std::fs::write(root.join("Base/gtk-3.0/gtk.css"), "/* ancien */").unwrap();
        std::fs::write(
            d.join("gtk.css"),
            "@import url(\"part.css\");\nwindow { }\n",
        )
        .unwrap();
        std::fs::write(
            d.join("part.css"),
            "check { background-image: url(\"../assets/c.png\"); }\n",
        )
        .unwrap();
        // GTK 3.24 prend gtk-3.20, pas gtk-3.0.
        let sheet = base_sheet(&root.join("Base"), 3, true).unwrap();
        assert!(sheet.ends_with("gtk-3.20/gtk.css"));
        let css = read_sheet(&sheet).unwrap();
        assert!(!css.contains("@import"), "{css}");
        assert!(
            css.contains(&format!("url(\"{}\")", d.join("../assets/c.png").display())),
            "{css}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Talon d'Adwaita (`/usr/share/themes/Adwaita/gtk-3.0/gtk.css`) : sans
    /// règle, il ne sert pas de base (le thème rendu n'aurait aucun style,
    /// menus transparents dans MarkText). Images d'une feuille embarquée :
    /// ressources de GTK.
    #[test]
    fn talon_d_adwaita_sans_regle() {
        assert!(!has_rules(
            "/* Adwaita is now part of GTK+ 3, this file is no longer used */\n"
        ));
        assert!(has_rules(
            "@import url(\"resource:///org/gtk/libgtk/theme/Adwaita/gtk-contained-dark.css\");"
        ));
        assert!(has_rules("/* a */ menu { background: #fff; }"));
        assert_eq!(
            resource_urls(
                "check { x: image(url(\"assets/c.svg\")); }",
                "/org/gtk/libgtk/theme/Adwaita"
            ),
            "check { x: image(url(\"resource:///org/gtk/libgtk/theme/Adwaita/assets/c.svg\")); }"
        );
        assert_eq!(gtk_resource(3, "/ailleurs/x.css"), None);
        // GTK 3 et gresource présents : Adwaita extrait, et recolorable.
        if let Some(css) = builtin_adwaita(true) {
            assert!(css.contains("menu"), "feuille d'Adwaita");
            assert!(!css.contains("url(\"assets/"), "images en ressources");
            let r = Recolor::new(&css, &pack()).expect("repères d'Adwaita");
            assert_ne!(r.sheet(&css), css);
        }
    }

    #[test]
    fn base_retrouvée_depuis_un_thème_du_moteur() {
        let root = std::env::temp_dir().join(format!("hd-gtkth-base-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let roots = Roots::at(&root);
        let dir = roots.data.join("themes/hyprdesk-verre");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("index.theme"),
            index_theme("hyprdesk-verre", &pack(), "Dracula-Pro"),
        )
        .unwrap();
        assert_eq!(base_name(&roots, Some("Dracula-Pro")), "Dracula-Pro");
        assert_eq!(base_name(&roots, Some("hyprdesk-verre")), "Dracula-Pro");
        assert_eq!(base_name(&roots, None), "Adwaita");
        let _ = std::fs::remove_dir_all(&root);
    }

    /// Essai sur un vrai thème : `HYPRDESK_GTK_BASE=<dossier du thème>`
    /// `HYPRDESK_GTK_OUT=<dossier>` cargo test -- --ignored essai_thème_réel
    #[test]
    #[ignore]
    fn essai_thème_réel() {
        let base = PathBuf::from(std::env::var("HYPRDESK_GTK_BASE").unwrap());
        let out = PathBuf::from(std::env::var("HYPRDESK_GTK_OUT").unwrap());
        std::fs::create_dir_all(&out).unwrap();
        let ambre = super::super::resolve::from_adjusted(
            Rgba(0xfb, 0xf3, 0xe6, 0xff),
            Rgba(0x24, 0x1a, 0x14, 0xff),
            Rgba(0xff, 0xb8, 0x6c, 0xff),
            Rgba(0x1c, 0x14, 0x0f, 0xff),
        );
        for major in [3u8, 4] {
            let sheet = base_sheet(&base, major, true).unwrap();
            let css = read_sheet(&sheet).unwrap();
            let r = Recolor::new(&css, &ambre).expect("repères");
            let rendu = r.sheet(&css);
            let n_av = css.matches('#').count();
            println!(
                "gtk-{major} : {} → {} octets, {n_av} « # »",
                css.len(),
                rendu.len()
            );
            std::fs::write(out.join(format!("gtk-{major}.css")), rendu).unwrap();
        }
    }
}
