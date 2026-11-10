//! État transmis par le démon à l'écran de verrouillage : valeurs déjà
//! résolues (fond, couleurs, police, options), dans un JSON de la session.
//!
//! Le verrou n'exécute ainsi aucun Lua : un plugin ne peut ni le bloquer ni y
//! injecter du code. Sans ce fichier (démon arrêté), des valeurs par défaut
//! sûres s'appliquent.

use std::path::{Path, PathBuf};

use crate::config::{Config, Rgba, WallSource};

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct LockState {
    /// Image de fond (déjà choisie : celle du verrouillage, sinon celle du bureau).
    pub image: Option<PathBuf>,
    /// Couleur de fond, sans image ou sous l'image.
    pub background: [u8; 4],
    pub fg: [u8; 4],
    pub accent: [u8; 4],
    pub font: Option<String>,
    pub blur: bool,
    pub dim: f32,
    pub show_user: bool,
    pub show_battery: bool,
    pub show_media: bool,
    /// Scène étalée sur les écrans au verrouillage (règle `span` ou `span = "lock"`).
    pub span: bool,
    /// Fond de chaque écran, préparé par le démon (ordre de Hyprland) : sa part
    /// de la scène étalée, ou son propre fond d'écran.
    pub outputs: Vec<LockOutput>,
    /// Image propre au verrouillage (`lock_image`) : la même sur tous les écrans.
    #[serde(default)]
    pub own_image: bool,
}

/// Part d'une scène étalée pour un écran : sa taille logique et son image.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct LockOutput {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// Image de cet écran (sa part de la scène, ou son propre fond) ; sans
    /// image, l'image commune ou la couleur de fond.
    #[serde(default)]
    pub image: Option<PathBuf>,
}

impl Default for LockState {
    fn default() -> Self {
        let g = crate::config::General::default();
        let rgba = |c: Rgba| [c.0, c.1, c.2, c.3];
        LockState {
            image: None,
            background: rgba(g.background),
            fg: rgba(g.theme.fg),
            accent: rgba(g.theme.accent),
            font: None,
            blur: g.lock.blur,
            dim: g.lock.dim,
            show_user: g.lock.show_user,
            show_battery: g.lock.show_battery,
            show_media: g.lock.show_media,
            span: false,
            outputs: Vec::new(),
            own_image: false,
        }
    }
}

/// `$XDG_RUNTIME_DIR/hyprdesk/lock.json` (propre à l'utilisateur et à la session).
pub fn path() -> PathBuf {
    let base = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::temp_dir().join(format!("hyprdesk-{}", unsafe { libc::getuid() }))
        });
    base.join("hyprdesk").join("lock.json")
}

/// État du verrouillage d'après la config, pour l'écran principal et le
/// workspace qu'il affiche.
pub fn from_config(cfg: &Config, monitor: &str, workspace: &str) -> LockState {
    let g = &cfg.general;
    let l = &g.lock;
    let rgba = |c: Rgba| [c.0, c.1, c.2, c.3];
    let mut background = rgba(g.background);
    let desktop = crate::config::resolve::resolve(cfg, monitor, workspace);
    let own = l.image.clone().filter(|p| p.is_file());
    let own_is_set = own.is_some();
    // Étalée au verrouillage : la scène du bureau (pas une image propre au verrouillage).
    let span = own.is_none() && desktop.wallpaper.is_some_and(|w| w.span_lock);
    let image = own.or_else(|| match desktop.wallpaper.map(|w| &w.source) {
        Some(WallSource::Path(p)) => Some(p.clone()),
        Some(WallSource::Rotate { dir, .. }) => first_image(dir),
        Some(WallSource::Color(c)) => {
            background = rgba(*c);
            None
        }
        None => None,
    });
    LockState {
        image,
        background,
        fg: rgba(g.theme.fg),
        accent: rgba(g.theme.accent),
        font: g.font.clone(),
        blur: l.blur,
        dim: l.dim,
        show_user: l.show_user,
        show_battery: l.show_battery,
        show_media: l.show_media,
        span,
        outputs: Vec::new(),
        own_image: own_is_set,
    }
}

/// Scène étalée : découpe la part de chaque écran (pixels physiques) dans le
/// dossier de la session, une fois par image et disposition ; rien sinon.
///
/// Sans scène étalée : chaque écran garde son propre fond (`own`, dans
/// l'ordre de `screens`) ; une image propre au verrouillage vaut pour tous.
pub fn prepare_span(s: &mut LockState, screens: &[crate::span::Screen], own: &[Option<PathBuf>]) {
    prepare_span_in(s, screens, own, &path().with_file_name("lock-span"));
}

fn prepare_span_in(
    s: &mut LockState,
    screens: &[crate::span::Screen],
    own: &[Option<PathBuf>],
    dir: &Path,
) {
    s.outputs.clear();
    if s.own_image {
        return;
    }
    let Some(img_path) = s.image.clone().filter(|_| s.span) else {
        // Le fond de chaque écran.
        s.outputs = screens
            .iter()
            .enumerate()
            .map(|(i, sc)| LockOutput {
                name: sc.name.clone(),
                width: sc.rect.w.round() as u32,
                height: sc.rect.h.round() as u32,
                image: own.get(i).cloned().flatten(),
            })
            .collect();
        return;
    };
    let stamp = std::fs::metadata(&img_path)
        .and_then(|m| m.modified())
        .map(|t| format!("{t:?}"))
        .unwrap_or_default();
    let key = format!(
        "{}|{stamp}|{}",
        img_path.display(),
        screens
            .iter()
            .map(|sc| format!(
                "{}:{}:{}:{}:{}:{}",
                sc.name, sc.rect.x, sc.rect.y, sc.rect.w, sc.rect.h, sc.scale
            ))
            .collect::<Vec<_>>()
            .join(",")
    );
    let file = |name: &str| dir.join(format!("{name}.png"));
    let fresh = std::fs::read_to_string(dir.join("key")).ok().as_deref() == Some(key.as_str())
        && screens.iter().all(|sc| file(&sc.name).is_file());
    if !fresh {
        let Ok(img) = ::image::open(&img_path) else {
            return;
        };
        let rects: Vec<crate::span::Rect> = screens.iter().map(|sc| sc.rect).collect();
        let Some(canvas) = crate::span::canvas(&rects) else {
            return;
        };
        if std::fs::create_dir_all(dir).is_err() {
            return;
        }
        for sc in screens {
            let phys = crate::span::physical(sc.rect, sc.scale);
            let part = crate::span::cut(
                &img,
                phys.w.round() as u32,
                phys.h.round() as u32,
                crate::span::placement(canvas, sc.rect),
            );
            if part.save(file(&sc.name)).is_err() {
                return;
            }
        }
        let _ = std::fs::write(dir.join("key"), &key);
    }
    s.outputs = screens
        .iter()
        .map(|sc| LockOutput {
            name: sc.name.clone(),
            width: sc.rect.w.round() as u32,
            height: sc.rect.h.round() as u32,
            image: Some(file(&sc.name)),
        })
        .collect();
}

pub fn first_image(dir: &Path) -> Option<PathBuf> {
    let mut l: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                matches!(e.to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "webp")
            })
        })
        .collect();
    l.sort();
    l.into_iter().next()
}

/// Écrit l'état (atomique, lisible par l'utilisateur seul).
pub fn write(s: &LockState) -> std::io::Result<()> {
    let p = path();
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700));
    }
    let json = serde_json::to_vec_pretty(s).map_err(std::io::Error::other)?;
    if std::fs::read(&p).ok().as_deref() == Some(json.as_slice()) {
        return Ok(());
    }
    crate::storage::write(&p, &json, crate::storage::revision())
}

/// Lit l'état ; valeurs par défaut s'il manque ou s'il est illisible.
pub fn read() -> LockState {
    std::fs::read(path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scène étalée au verrouillage : la part de chaque écran, à la taille
    /// physique de l'écran, refaite seulement si l'image ou la disposition change.
    #[test]
    fn parts_des_ecrans() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-span-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let img = dir.join("scene.png");
        ::image::RgbImage::from_fn(400, 100, |x, _| ::image::Rgb([(x / 2) as u8, 0, 0]))
            .save(&img)
            .unwrap();
        let screens = [
            crate::span::Screen {
                name: "A".into(),
                rect: crate::span::Rect {
                    x: 0.0,
                    y: 0.0,
                    w: 200.0,
                    h: 100.0,
                },
                scale: 1.0,
            },
            crate::span::Screen {
                name: "B".into(),
                rect: crate::span::Rect {
                    x: 200.0,
                    y: 0.0,
                    w: 100.0,
                    h: 50.0,
                },
                scale: 2.0,
            },
        ];
        let mut s = LockState {
            image: Some(img.clone()),
            span: true,
            ..Default::default()
        };
        let out = dir.join("parts");
        prepare_span_in(&mut s, &screens, &[], &out);
        assert_eq!(s.outputs.len(), 2);
        assert_eq!((s.outputs[1].width, s.outputs[1].height), (100, 50));
        let b = ::image::open(s.outputs[1].image.as_ref().unwrap()).unwrap();
        assert_eq!((b.width(), b.height()), (200, 100), "pixels physiques");
        // Sans étalement : rien.
        let mut s = LockState {
            image: Some(img.clone()),
            ..Default::default()
        };
        // Sans étalement : le fond propre de chaque écran (aucun pour B).
        prepare_span_in(&mut s, &screens, &[Some(img.clone()), None], &out);
        assert_eq!(s.outputs.len(), 2);
        assert_eq!(s.outputs[0].image.as_ref(), Some(&img));
        assert_eq!(s.outputs[1].image, None);
        // Image propre au verrouillage : la même partout.
        s.own_image = true;
        prepare_span_in(&mut s, &screens, &[Some(img.clone()), None], &out);
        assert!(s.outputs.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn etat_du_verrou() {
        let mut cfg = Config::default();
        cfg.general.lock.dim = 0.6;
        cfg.wallpapers.push(crate::config::WallpaperRule {
            origin: Default::default(),
            sel: Default::default(),
            source: WallSource::Color(Rgba(1, 2, 3, 255)),
            fit: Default::default(),
            span: false,
            span_lock: false,
        });
        let s = from_config(&cfg, "DP-1", "1");
        assert_eq!(s.image, None);
        assert_eq!(s.background, [1, 2, 3, 255]);
        assert_eq!(s.dim, 0.6);
        // Aller-retour JSON, champs manquants complétés.
        let json = serde_json::to_string(&s).unwrap();
        assert_eq!(serde_json::from_str::<LockState>(&json).unwrap(), s);
        let partial: LockState = serde_json::from_str(r#"{ "blur": false }"#).unwrap();
        assert!(!partial.blur && partial.show_user);
    }
}
