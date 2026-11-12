//! Résolution : pour un couple (écran, workspace), le bureau effectif.
//!
//! - fond d'écran et mode : la règle la PLUS SPÉCIFIQUE l'emporte
//!   (workspace + écran > workspace > écran > `*`) ; à égalité, la dernière
//!   déclarée, comme une surcharge plus bas dans le fichier ;
//! - icônes et widgets : ils s'ADDITIONNENT, sauf `hd.desktop({ inherit = false })`
//!   qui écarte ceux déclarés pour tous les workspaces.

use super::model::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Desktop<'a> {
    pub mode: Mode,
    pub wallpaper: Option<&'a WallpaperRule>,
    /// Index dans `Config::wallpapers` (clé de l'état de rotation).
    pub wallpaper_idx: Option<usize>,
    pub icons: Vec<&'a IconDef>,
    pub widgets: Vec<&'a WidgetDef>,
}

fn best<'a, T>(
    items: impl Iterator<Item = (usize, &'a T)>,
    sel: impl Fn(&T) -> &Selector,
) -> Option<(usize, &'a T)>
where
    T: 'a,
{
    // max_by_key garde le DERNIER des maximums : la dernière déclaration gagne.
    items.max_by_key(|(_, it)| sel(it).specificity())
}

pub fn resolve<'a>(cfg: &'a Config, monitor: &str, workspace: &str) -> Desktop<'a> {
    let wall = best(
        cfg.wallpapers
            .iter()
            .enumerate()
            .filter(|(_, w)| w.sel.matches(monitor, workspace)),
        |w| &w.sel,
    );
    let desk = best(
        cfg.desktops
            .iter()
            .enumerate()
            .filter(|(_, d)| d.sel.matches(monitor, workspace)),
        |d| &d.sel,
    );
    let inherit = desk.is_none_or(|(_, d)| d.inherit);
    let mode = cfg
        .desktops
        .iter()
        .enumerate()
        .filter(|(_, d)| d.mode.is_some() && d.sel.matches(monitor, workspace))
        .max_by_key(|(_, d)| d.sel.specificity())
        .and_then(|(_, d)| d.mode)
        .unwrap_or(cfg.general.mode);
    let keep =
        |s: &Selector| s.matches(monitor, workspace) && (inherit || !s.is_workspace_wildcard());
    Desktop {
        mode,
        wallpaper: wall.map(|(_, w)| w),
        wallpaper_idx: wall.map(|(i, _)| i),
        // Icônes des Réglages : selon la portée choisie ; celles de
        // hyprdesk.lua : leur propre sélecteur.
        icons: cfg
            .icons
            .iter()
            .filter(|i| match i.origin {
                crate::config::Origin::Settings => {
                    cfg.general.icon_scope.keeps(&i.sel, monitor, workspace)
                }
                crate::config::Origin::User => keep(&i.sel),
            })
            .collect(),
        widgets: cfg
            .widgets
            .iter()
            .filter(|w| !w.in_bar && keep(&w.sel))
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sel(ws: Option<&str>, mon: Option<&str>) -> Selector {
        Selector {
            workspaces: ws.map(|w| vec![w.to_string()]),
            monitors: mon.map(|m| vec![m.to_string()]),
        }
    }

    fn wall(s: Selector, p: &str) -> WallpaperRule {
        WallpaperRule {
            origin: Origin::User,
            sel: s,
            source: WallSource::Path(PathBuf::from(p)),
            fit: Fit::Cover,
            span: false,
            span_lock: false,
        }
    }

    fn icon(s: Selector, id: &str) -> IconDef {
        IconDef {
            origin: Origin::User,
            sel: s,
            id: id.into(),
            label: None,
            kind: IconKind::App(id.into()),
            icon: None,
            pos: None,
        }
    }

    fn path_of(d: &Desktop) -> String {
        match &d.wallpaper.unwrap().source {
            WallSource::Path(p) => p.display().to_string(),
            _ => unreachable!(),
        }
    }

    #[test]
    fn le_plus_specifique_gagne() {
        let cfg = Config {
            wallpapers: vec![
                wall(sel(Some("3"), Some("DP-1")), "ws3-dp1"),
                wall(sel(None, None), "defaut"),
                wall(sel(None, Some("DP-1")), "dp1"),
                wall(sel(Some("3"), None), "ws3"),
            ],
            ..Default::default()
        };
        assert_eq!(path_of(&resolve(&cfg, "HDMI-A-2", "1")), "defaut");
        assert_eq!(path_of(&resolve(&cfg, "DP-1", "6")), "dp1");
        assert_eq!(path_of(&resolve(&cfg, "HDMI-A-2", "3")), "ws3");
        assert_eq!(path_of(&resolve(&cfg, "DP-1", "3")), "ws3-dp1");
    }

    #[test]
    fn a_egalite_la_derniere_declaree() {
        let cfg = Config {
            wallpapers: vec![wall(sel(None, None), "a"), wall(sel(None, None), "b")],
            ..Default::default()
        };
        let d = resolve(&cfg, "X", "1");
        assert_eq!(path_of(&d), "b");
        assert_eq!(d.wallpaper_idx, Some(1));
    }

    #[test]
    fn icones_additives_et_inherit() {
        let cfg = Config {
            icons: vec![
                icon(sel(None, None), "global"),
                icon(sel(Some("2"), None), "ws2"),
                icon(sel(Some("4"), None), "ws4"),
            ],
            desktops: vec![DesktopRule {
                origin: Origin::User,
                sel: sel(Some("4"), None),
                mode: Some(Mode::Glass),
                inherit: false,
            }],
            ..Default::default()
        };
        let ids = |m: &str, w: &str| {
            resolve(&cfg, m, w)
                .icons
                .iter()
                .map(|i| i.id.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids("M", "1"), vec!["global"]);
        assert_eq!(ids("M", "2"), vec!["global", "ws2"]);
        assert_eq!(ids("M", "4"), vec!["ws4"]);
        assert_eq!(resolve(&cfg, "M", "4").mode, Mode::Glass);
        assert_eq!(resolve(&cfg, "M", "1").mode, Mode::Opaque);
    }

    #[test]
    fn mode_sans_surcharge_suit_general() {
        let mut cfg = Config::default();
        cfg.general.mode = Mode::Glass;
        cfg.desktops.push(DesktopRule {
            origin: Origin::User,
            sel: sel(Some("2"), None),
            mode: None,
            inherit: true,
        });
        assert_eq!(resolve(&cfg, "M", "2").mode, Mode::Glass);
    }
}
