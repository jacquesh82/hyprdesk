//! Thème : tout ce qui se voit, et rien d'autre.
//!
//! Un thème réunit le nom, les couleurs (texte, surfaces, accent, fond sans
//! image), les fonds d'écran, l'effet de chaque endroit de l'interface
//! (opacité et flou de la barre, des menus, du lanceur, des réglages,
//! opacité des widgets), le mode du bureau et son verre, le voile du
//! verrouillage et les transitions.
//!
//! Il s'écrit comme le reste des réglages, dans `settings.lua`
//! (`hd.config({ theme = { … } })` et `hd.wallpaper`). Il voyage seul dans une
//! archive de thème (`crate::share`, portée `Scope::Theme`) ; « Mes thèmes »
//! sont ces archives, rangées dans `hyprdesk/themes/`.

use std::path::{Path, PathBuf};

use crate::config::{Config, Effect, Origin, Place, Rgba, Theme};
use crate::settings::Settings;

/// Thème livré avec hyprdesk : couleurs et effets (les fonds restent ceux en place).
#[derive(Debug, Clone, PartialEq)]
pub struct Preset {
    pub name: &'static str,
    pub fg: Rgba,
    pub bg: Rgba,
    pub accent: Rgba,
    /// Opacités de la barre, des menus, du lanceur, des réglages et des widgets.
    pub opacity: [f32; 5],
    pub blur: bool,
}

impl Preset {
    pub fn effects(&self) -> std::collections::BTreeMap<Place, Effect> {
        Place::ALL
            .iter()
            .zip(self.opacity)
            .map(|(p, opacity)| {
                (
                    *p,
                    Effect {
                        opacity,
                        blur: self.blur && p.blurrable(),
                    },
                )
            })
            .collect()
    }
}

pub const PRESETS: &[Preset] = &[
    Preset {
        name: "Ardoise",
        fg: Rgba(0xf2, 0xf4, 0xfc, 0xff),
        bg: Rgba(0x17, 0x1b, 0x27, 0xff),
        accent: Rgba(0xa6, 0x9c, 0xff, 0xff),
        opacity: [0.72, 0.90, 0.88, 0.97, 0.84],
        blur: true,
    },
    Preset {
        name: "Verre",
        fg: Rgba(0xf6, 0xf7, 0xff, 0xff),
        bg: Rgba(0x1b, 0x1f, 0x33, 0xff),
        accent: Rgba(0xff, 0x80, 0xbf, 0xff),
        opacity: [0.42, 0.66, 0.55, 0.82, 0.55],
        blur: true,
    },
    Preset {
        name: "Encre",
        fg: Rgba(0xe8, 0xe8, 0xec, 0xff),
        bg: Rgba(0x00, 0x00, 0x00, 0xff),
        accent: Rgba(0x80, 0xff, 0xea, 0xff),
        opacity: [0.94, 0.97, 0.95, 1.0, 0.92],
        blur: false,
    },
    Preset {
        name: "Forêt",
        fg: Rgba(0xee, 0xf3, 0xea, 0xff),
        bg: Rgba(0x16, 0x22, 0x1b, 0xff),
        accent: Rgba(0x8a, 0xff, 0x80, 0xff),
        opacity: [0.74, 0.92, 0.88, 0.97, 0.84],
        blur: true,
    },
    Preset {
        name: "Ambre",
        fg: Rgba(0xfb, 0xf3, 0xe6, 0xff),
        bg: Rgba(0x24, 0x1a, 0x14, 0xff),
        accent: Rgba(0xff, 0xb8, 0x6c, 0xff),
        opacity: [0.76, 0.92, 0.88, 0.97, 0.86],
        blur: true,
    },
    Preset {
        name: "Clair",
        fg: Rgba(0x1f, 0x23, 0x30, 0xff),
        bg: Rgba(0xf3, 0xf4, 0xf8, 0xff),
        accent: Rgba(0x5b, 0x5b, 0xd6, 0xff),
        opacity: [0.82, 0.95, 0.92, 0.98, 0.90],
        blur: true,
    },
];

/// Résumé d'un thème, lisible sans charger son Lua (manifeste d'archive).
#[derive(Debug, Default, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Summary {
    pub name: String,
    pub fg: String,
    pub bg: String,
    pub accent: String,
}

impl Summary {
    pub fn of(t: &Theme) -> Self {
        let h = crate::settings::hex;
        Summary {
            name: t.name.clone(),
            fg: h(t.fg),
            bg: h(t.bg),
            accent: h(t.accent),
        }
    }

    /// Couleurs (texte, surfaces, accent), celles par défaut si illisibles.
    pub fn colors(&self) -> (Rgba, Rgba, Rgba) {
        let d = Theme::default();
        let c = |s: &str, d: Rgba| Rgba::parse(s).unwrap_or(d);
        (
            c(&self.fg, d.fg),
            c(&self.bg, d.bg),
            c(&self.accent, d.accent),
        )
    }
}

/// Config effective : `settings.lua` puis `hyprdesk.lua` s'il existe.
pub fn effective(config_path: &Path) -> Result<Config, String> {
    if config_path.is_file() {
        crate::config::lua::load_file(config_path).map(|l| l.config)
    } else {
        crate::config::lua::load_settings_only(config_path)
    }
}

impl Settings {
    /// Le thème effectif de `cfg`, champ par champ explicite : de quoi le
    /// reproduire à l'identique ailleurs. Les fonds de hyprdesk.lua en font
    /// partie (ils deviennent des fonds des réglages).
    pub fn theme_of(cfg: &Config) -> Settings {
        let g = &cfg.general;
        Settings {
            theme_name: Some(g.theme.name.clone()),
            fg: Some(g.theme.fg),
            bg: Some(g.theme.bg),
            accent: Some(g.theme.accent),
            background: Some(g.background),
            effects: Place::ALL
                .iter()
                .map(|p| (*p, g.theme.effect(*p)))
                .collect(),
            mode: Some(g.mode),
            glass_opacity: Some(g.glass_opacity),
            transition: Some(g.transition),
            fade_ms: Some(g.fade_ms),
            crossfade_ms: Some(g.crossfade_ms),
            lock_blur: Some(g.lock.blur),
            lock_dim: Some(g.lock.dim),
            wallpapers: cfg
                .wallpapers
                .iter()
                .cloned()
                .map(|mut w| {
                    w.origin = Origin::Settings;
                    w
                })
                .collect(),
            ..Default::default()
        }
    }

    /// Ne garde que le thème de ces réglages.
    pub fn theme_part(&self) -> Settings {
        Settings {
            theme_name: self.theme_name.clone(),
            fg: self.fg,
            bg: self.bg,
            accent: self.accent,
            background: self.background,
            effects: self.effects.clone(),
            mode: self.mode,
            glass_opacity: self.glass_opacity,
            transition: self.transition,
            fade_ms: self.fade_ms,
            crossfade_ms: self.crossfade_ms,
            lock_blur: self.lock_blur,
            lock_dim: self.lock_dim,
            wallpapers: self.wallpapers.clone(),
            ..Default::default()
        }
    }

    /// Remplace le thème de ces réglages par celui de `t` (fonds compris) ;
    /// le reste (barre, raccourcis, widgets…) ne bouge pas.
    pub fn set_theme(&mut self, t: &Settings) {
        let t = t.theme_part();
        self.theme_name = t.theme_name;
        self.fg = t.fg;
        self.bg = t.bg;
        self.accent = t.accent;
        self.background = t.background;
        self.effects = t.effects;
        self.mode = t.mode;
        self.glass_opacity = t.glass_opacity;
        self.transition = t.transition;
        self.fade_ms = t.fade_ms;
        self.crossfade_ms = t.crossfade_ms;
        self.lock_blur = t.lock_blur;
        self.lock_dim = t.lock_dim;
        self.wallpapers = t.wallpapers;
    }

    /// Thème livré : ses couleurs et ses effets ; fonds et mode restent en place.
    pub fn apply_preset(&mut self, p: &Preset) {
        self.theme_name = Some(p.name.into());
        self.fg = Some(p.fg);
        self.bg = Some(p.bg);
        self.accent = Some(p.accent);
        self.effects = p.effects();
    }
}

/// Dossier de « Mes thèmes ».
pub fn themes_dir(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .unwrap_or(Path::new("."))
        .join("hyprdesk/themes")
}

/// Retire un thème de « Mes thèmes » : déplacé dans `themes/.retired/`
/// (récupérable), jamais effacé.
pub fn retire(path: &Path, config_path: &Path) -> Result<PathBuf, String> {
    let dir = themes_dir(config_path);
    if path.parent() != Some(dir.as_path()) {
        return Err(crate::i18n::tr("seuls les thèmes de « Mes thèmes » se retirent").into());
    }
    let bin = dir.join(".retired");
    std::fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let mut target = bin.join(&name);
    if target.exists() {
        target = bin.join(format!("{}.{name}", crate::storage::revision()));
    }
    std::fs::rename(path, &target).map_err(|e| e.to_string())?;
    Ok(target)
}

/// Nom de fichier d'un thème : minuscules, chiffres et tirets.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    for c in name.trim().to_lowercase().chars() {
        let c = match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            c => c,
        };
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() { "theme".into() } else { out }
}

/// Un thème enregistré : son archive et son résumé.
#[derive(Debug, Clone, PartialEq)]
pub struct Saved {
    pub path: PathBuf,
    pub summary: Summary,
}

/// « Mes thèmes », triés par nom.
pub fn library(config_path: &Path) -> Vec<Saved> {
    let Ok(rd) = std::fs::read_dir(themes_dir(config_path)) else {
        return Vec::new();
    };
    let mut out: Vec<Saved> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().ends_with(".tar.gz"))
        .filter_map(|path| {
            let m = crate::share::read_manifest(&path).ok()?;
            let summary = m.theme.unwrap_or_else(|| Summary {
                name: path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(crate::i18n::tr("thème"))
                    .trim_end_matches(".tar.gz")
                    .to_string(),
                ..Summary::of(&Theme::default())
            });
            Some(Saved { path, summary })
        })
        .collect();
    out.sort_by(|a, b| {
        a.summary
            .name
            .to_lowercase()
            .cmp(&b.summary.name.to_lowercase())
    });
    out
}

/// Enregistre le thème en place dans « Mes thèmes » (remplace un thème du même nom).
pub fn save_current(config_path: &Path, name: &str) -> Result<PathBuf, String> {
    let out = themes_dir(config_path).join(format!("{}.tar.gz", slug(name)));
    crate::share::export_theme(config_path, &out, Some(name))?;
    Ok(out)
}

// ---------------------------------------------------------------------------
// Thème tiré d'une scène

/// Une scène : l'image d'où naît un thème, et son nom.
pub struct Scene {
    pub image: PathBuf,
    pub name: String,
    /// Dossier temporaire (pack `.zip` extrait), retiré avec la scène.
    tmp: Option<PathBuf>,
}

impl Drop for Scene {
    fn drop(&mut self) {
        if let Some(d) = &self.tmp {
            let _ = std::fs::remove_dir_all(d);
        }
    }
}

/// Nom lisible d'un fichier : « Cozy Twilight Study with Cat ».
fn stem_name(p: &Path) -> String {
    p.file_stem()
        .and_then(|n| n.to_str())
        .unwrap_or(crate::i18n::tr("Scène"))
        .replace(['-', '_'], " ")
        .trim()
        .to_string()
}

/// Scène d'un pack (dossier avec `manifest.json` : `master`, `theme`), cherchée
/// à la racine puis un niveau plus bas.
fn scene_in_dir(dir: &Path) -> Option<(PathBuf, String)> {
    let candidates = std::iter::once(dir.to_path_buf()).chain(
        std::fs::read_dir(dir)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.is_dir()),
    );
    for d in candidates {
        let Ok(src) = std::fs::read_to_string(d.join("manifest.json")) else {
            continue;
        };
        let Ok(m) = serde_json::from_str::<serde_json::Value>(&src) else {
            continue;
        };
        let Some(master) = m["master"].as_str() else {
            continue;
        };
        // Le manifeste ne doit pas sortir du pack (chemins résolus, `..` compris).
        let (Ok(image), Ok(root)) = (d.join(master).canonicalize(), d.canonicalize()) else {
            continue;
        };
        if !image.starts_with(&root) || !image.is_file() {
            continue;
        }
        let name = m["theme"]
            .as_str()
            .map(str::to_string)
            .unwrap_or_else(|| stem_name(&d));
        return Some((image, name));
    }
    None
}

/// Scène d'une source : une image, ou un pack de fonds multi-écrans (dossier
/// ou `.zip` avec un `manifest.json` qui désigne sa scène maîtresse).
pub fn scene(source: &Path) -> Result<Scene, String> {
    let lower = source.to_string_lossy().to_lowercase();
    if source.is_dir() {
        let (image, name) = scene_in_dir(source).ok_or_else(|| {
            crate::tr!(
                "{p0} : pas de manifest.json avec une scène (« master »)",
                p0 = source.display()
            )
        })?;
        return Ok(Scene {
            image,
            name,
            tmp: None,
        });
    }
    if lower.ends_with(".zip") {
        let tmp = std::env::temp_dir().join(format!(
            "hyprdesk-scene-{}-{}",
            std::process::id(),
            crate::storage::revision()
        ));
        std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
        let st = std::process::Command::new("unzip")
            .arg("-qq")
            .arg(source)
            .arg("-d")
            .arg(&tmp)
            .status()
            .map_err(|e| format!("unzip : {e}"))?;
        let found = st.success().then(|| scene_in_dir(&tmp)).flatten();
        let Some((image, name)) = found else {
            let _ = std::fs::remove_dir_all(&tmp);
            return Err(crate::tr!(
                "{p0} : pas de manifest.json avec une scène (« master »)",
                p0 = source.display()
            ));
        };
        return Ok(Scene {
            image,
            name,
            tmp: Some(tmp),
        });
    }
    if !source.is_file() {
        return Err(crate::tr!("{p0} : introuvable", p0 = source.display()));
    }
    Ok(Scene {
        image: source.to_path_buf(),
        name: stem_name(source),
        tmp: None,
    })
}

fn to_hsv(c: [u8; 3]) -> (f32, f32, f32) {
    let [r, g, b] = c.map(|v| v as f32 / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, if max == 0.0 { 0.0 } else { d / max }, max)
}

fn from_hsv(h: f32, s: f32, v: f32) -> Rgba {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0).rem_euclid(2.0) - 1.0).abs());
    let m = v - c;
    let (r, g, b) = match (h.rem_euclid(360.0) / 60.0) as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let b8 = |v: f32| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgba(b8(r), b8(g), b8(b), 0xff)
}

/// Couleurs d'un thème tirées d'une scène : surfaces sombres dans la teinte
/// des ombres, accent dans la teinte vive la plus présente, texte clair.
pub fn palette(img: &image::DynamicImage) -> (Rgba, Rgba, Rgba) {
    let small = img.thumbnail(96, 96).to_rgb8();
    let mut px: Vec<(f32, f32, f32, [u8; 3])> = small
        .pixels()
        .map(|p| {
            let (h, s, v) = to_hsv(p.0);
            (h, s, v, p.0)
        })
        .collect();
    let d = Theme::default();
    if px.is_empty() {
        return (d.fg, d.bg, d.accent);
    }
    // Ombres : le quart le plus sombre, moyenné.
    px.sort_by(|a, b| a.2.total_cmp(&b.2));
    let dark = &px[..(px.len() / 4).max(1)];
    let n = dark.len() as f32;
    let avg = |i: usize| (dark.iter().map(|p| p.3[i] as f32).sum::<f32>() / n) as u8;
    let (bh, bs, _) = to_hsv([avg(0), avg(1), avg(2)]);
    let bg = from_hsv(bh, bs.min(0.45), 0.11);
    // Accent : la teinte vive qui pèse le plus (saturation × luminosité).
    let mut bins = [(0.0f32, 0.0f32, 0.0f32); 24];
    for &(h, s, v, _) in &px {
        if s > 0.3 && v > 0.45 {
            let b = &mut bins[((h / 15.0) as usize).min(23)];
            b.0 += s * v;
            b.1 += h * s * v;
            b.2 += s * s * v;
        }
    }
    let best = bins
        .iter()
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .copied()
        .unwrap_or_default();
    let accent = if best.0 > 0.0 {
        from_hsv(best.1 / best.0, (best.2 / best.0).clamp(0.45, 0.7), 0.97)
    } else {
        d.accent
    };
    let fg = from_hsv(bh, 0.05, 0.96);
    (fg, bg, accent)
}

/// Thème tiré d'une scène : ses couleurs, la scène étalée sur tous les écrans
/// (bureau et verrouillage), des surfaces translucides et floutées.
pub fn from_scene(image: &Path, name: &str) -> Result<Settings, String> {
    let img = image::open(image).map_err(|e| format!("{} : {e}", image.display()))?;
    let (fg, bg, accent) = palette(&img);
    let mut t = Settings::theme_of(&Config::default());
    t.theme_name = Some(name.to_string());
    t.fg = Some(fg);
    t.bg = Some(bg);
    t.accent = Some(accent);
    t.background = Some(bg);
    t.effects = Place::ALL
        .iter()
        .zip([0.62, 0.86, 0.78, 0.94, 0.72])
        .map(|(p, opacity)| {
            (
                *p,
                Effect {
                    opacity,
                    blur: p.blurrable(),
                },
            )
        })
        .collect();
    t.wallpapers = vec![crate::config::WallpaperRule {
        origin: Origin::Settings,
        sel: Default::default(),
        source: crate::config::WallSource::Path(image.to_path_buf()),
        fit: Default::default(),
        span: true,
        span_lock: true,
    }];
    Ok(t)
}

/// Crée un thème depuis une image ou un pack, et l'enregistre dans « Mes
/// thèmes ». Rend l'archive et, si l'image est trop petite pour les écrans
/// actuels, un avertissement.
pub fn create(
    config_path: &Path,
    source: &Path,
    name: Option<&str>,
) -> Result<(PathBuf, Option<String>), String> {
    let sc = scene(source)?;
    let name = name
        .map(str::trim)
        .filter(|n| !n.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| sc.name.clone());
    let theme = from_scene(&sc.image, &name)?;
    let out = themes_dir(config_path).join(format!("{}.tar.gz", slug(&name)));
    crate::share::pack_theme(&theme, &out)?;
    let warning = image::image_dimensions(&sc.image).ok().and_then(|(w, h)| {
        let rects: Vec<crate::span::Rect> = crate::span::screens()
            .ok()?
            .into_iter()
            .map(|s| crate::span::physical(s.rect, s.scale))
            .collect();
        crate::span::resolution_warning(w, h, &rects, true)
    });
    Ok((out, warning))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Retirer un thème le met de côté (récupérable) ; hors de « Mes thèmes », refusé.
    #[test]
    fn retirer_un_theme() {
        let root = std::env::temp_dir().join(format!("hyprdesk-retire-{}", std::process::id()));
        let cfg = root.join("hyprdesk.lua");
        let dir = themes_dir(&cfg);
        std::fs::create_dir_all(&dir).unwrap();
        let t = dir.join("ardoise.tar.gz");
        std::fs::write(&t, b"x").unwrap();
        let to = retire(&t, &cfg).unwrap();
        assert!(!t.exists() && to.is_file());
        assert!(to.starts_with(dir.join(".retired")));
        // Même nom une seconde fois : gardé à côté, rien d'écrasé.
        std::fs::write(&t, b"y").unwrap();
        let to2 = retire(&t, &cfg).unwrap();
        assert!(to2 != to && to.is_file() && to2.is_file());
        assert!(retire(&root.join("ailleurs.tar.gz"), &cfg).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn noms_de_fichier() {
        assert_eq!(slug("Forêt d'été"), "foret-d-ete");
        assert_eq!(slug("  "), "theme");
        assert_eq!(slug("Mon thème #2"), "mon-theme-2");
    }

    #[test]
    fn theme_seul_puis_applique() {
        let mut cfg = Config::default();
        cfg.general.theme.accent = Rgba(1, 2, 3, 255);
        cfg.general.theme.effects.insert(
            Place::Bar,
            Effect {
                opacity: 0.5,
                blur: false,
            },
        );
        let t = Settings::theme_of(&cfg);
        // Le Lua du thème se relit à l'identique.
        let mut s = Settings {
            workspaces: Some(4),
            accent: Some(Rgba(9, 9, 9, 255)),
            ..Default::default()
        };
        s.set_theme(&t);
        assert_eq!(s.workspaces, Some(4), "le reste ne bouge pas");
        assert_eq!(s.accent, Some(Rgba(1, 2, 3, 255)));
        let back = s.apply_to(&Config::default());
        assert_eq!(back.general.theme.effect(Place::Bar).opacity, 0.5);
        assert!(
            !back
                .general
                .theme
                .blurred_namespaces()
                .contains(&"hyprdesk-bar")
        );
        assert!(
            back.general
                .theme
                .blurred_namespaces()
                .contains(&"hyprdesk-popover")
        );
        // Un thème livré garde les fonds et le mode.
        s.mode = Some(crate::config::Mode::Glass);
        s.apply_preset(&PRESETS[2]);
        assert_eq!(s.mode, Some(crate::config::Mode::Glass));
        assert_eq!(s.theme_name.as_deref(), Some("Encre"));
        assert!(!s.effects[&Place::Menus].blur);
    }

    #[test]
    fn palette_d_une_scene() {
        // Ciel orangé au-dessus, ombres violettes en dessous.
        let img = image::RgbImage::from_fn(40, 40, |_, y| {
            if y < 20 {
                image::Rgb([0xf0, 0x90, 0x40])
            } else {
                image::Rgb([0x30, 0x20, 0x50])
            }
        });
        let (fg, bg, accent) = palette(&image::DynamicImage::ImageRgb8(img));
        let (bh, _, bv) = to_hsv([bg.0, bg.1, bg.2]);
        assert!(
            bv < 0.15 && (240.0..300.0).contains(&bh),
            "surfaces sombres et violettes : {bg:?}"
        );
        let (ah, _, av) = to_hsv([accent.0, accent.1, accent.2]);
        assert!(
            av > 0.9 && (15.0..45.0).contains(&ah),
            "accent orangé : {accent:?}"
        );
        assert!(to_hsv([fg.0, fg.1, fg.2]).2 > 0.9);
    }

    #[test]
    fn scene_d_un_pack() {
        let d = std::env::temp_dir().join(format!("hyprdesk-pack-{}", std::process::id()));
        let inner = d.join("mon-pack");
        std::fs::create_dir_all(&inner).unwrap();
        image::RgbImage::new(8, 8)
            .save(inner.join("scene.png"))
            .unwrap();
        std::fs::write(
            inner.join("manifest.json"),
            r#"{ "master": "scene.png", "theme": "Lac au couchant" }"#,
        )
        .unwrap();
        let sc = scene(&d).unwrap();
        assert_eq!(sc.name, "Lac au couchant");
        assert_eq!(sc.image, inner.join("scene.png").canonicalize().unwrap());
        // Un manifeste qui pointe hors du pack est ignoré.
        std::fs::write(
            inner.join("manifest.json"),
            r#"{ "master": "../../etc/passwd" }"#,
        )
        .unwrap();
        assert!(scene(&d).is_err());
        let t = from_scene(&inner.join("scene.png"), "X").unwrap();
        assert!(t.wallpapers[0].span);
        let _ = std::fs::remove_dir_all(&d);
    }
}
