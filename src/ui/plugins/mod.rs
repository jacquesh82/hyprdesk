//! Paquets de réglages intégrés (built-in packages).
//!
//! Chaque entrée de l'écran Réglages est un paquet autonome : un manifeste
//! ([`PluginInfo`]), ses messages, son état, sa vue et sa mise à jour. La
//! coquille du tiroir (`panel.rs`) ne connaît que ce registre : la navigation
//! se construit depuis [`BUILTIN`], chaque message est routé vers son paquet.
//! Ajouter un paquet : un module ici, une ligne dans `BUILTIN`, une variante
//! dans [`PluginMsg`] et un bras dans `update` / `view`. Conception complète :
//! `docs/PLUGINS.md`.

use iced::{Element, Task};

use super::panel::Panel;
use super::{App, Msg};

pub mod animations;
pub mod apps;
pub mod apptheme;
pub mod assistant;
pub mod bar;
pub mod datetime;
pub mod desktop;
pub mod effects;
pub mod energy;
pub mod icons;
pub mod launcher;
pub mod layouts;
pub mod lock;
#[allow(clippy::module_inception)]
pub mod plugins;
pub mod screens;
pub mod share;
pub mod sound;
pub mod theme;
pub mod wallpaper;
pub mod widgets;
pub mod workspaces;

/// Manifeste d'un paquet de réglages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PluginInfo {
    /// Identifiant stable (onglet, `hyprdesk settings <id>`).
    pub id: &'static str,
    /// Libellé de la navigation.
    pub label: &'static str,
    /// Icône (nom freedesktop ; glyphe intégré dans `ui/icons.rs`).
    pub icon: &'static str,
    /// Une phrase : ce que le paquet règle.
    pub summary: &'static str,
    /// Livré avec hyprdesk (statut « build-in package »).
    pub builtin: bool,
}

/// Paquets intégrés, dans l'ordre de la navigation.
pub const BUILTIN: &[PluginInfo] = &[
    desktop::INFO,
    icons::INFO,
    launcher::INFO,
    apps::INFO,
    assistant::INFO,
    theme::INFO,
    apptheme::INFO,
    wallpaper::INFO,
    widgets::INFO,
    bar::INFO,
    screens::INFO,
    workspaces::INFO,
    layouts::INFO,
    effects::INFO,
    animations::INFO,
    sound::INFO,
    energy::INFO,
    datetime::INFO,
    lock::INFO,
    share::INFO,
    plugins::INFO,
];

/// Groupe de la navigation des Réglages : titre, paquets dans l'ordre.
pub struct Group {
    pub label: &'static str,
    pub ids: &'static [&'static str],
}

/// Navigation des Réglages, par groupes (chaque paquet dans un seul groupe).
pub const GROUPS: &[Group] = &[
    Group {
        label: "Bureau",
        ids: &["desktop", "wallpaper", "icons", "widgets"],
    },
    Group {
        label: "Apparence",
        ids: &["theme", "apptheme", "effects", "animations"],
    },
    Group {
        label: "Barre et lanceur",
        ids: &["bar", "launcher"],
    },
    Group {
        label: "Fenêtres et écrans",
        ids: &["screens", "workspaces", "layouts"],
    },
    Group {
        label: "Système",
        ids: &["sound", "energy", "datetime"],
    },
    Group {
        label: "Applications",
        ids: &["apps", "assistant"],
    },
    Group {
        label: "Système",
        ids: &["lock", "share", "plugins"],
    },
];

/// Paquets affichés seulement en configuration avancée.
pub const ADVANCED_ONLY: &[&str] = &["effects", "share", "plugins"];

/// Onglets affichés seulement en configuration avancée : (paquet, onglet).
pub const ADVANCED_TABS: &[(&str, &str)] = &[
    ("desktop", "session"),
    ("desktop", "dependances"),
    ("theme", "effets"),
    ("layouts", "custom"),
];

/// Le paquet est affiché (configuration de base ou avancée).
pub fn visible(id: &str, advanced: bool) -> bool {
    advanced || !ADVANCED_ONLY.contains(&id)
}

/// Onglets affichés d'un paquet.
pub fn visible_tabs(id: &str, advanced: bool) -> Vec<SubTab> {
    tabs(id)
        .iter()
        .filter(|(t, _)| advanced || !ADVANCED_TABS.contains(&(id, *t)))
        .copied()
        .collect()
}

/// Onglet d'un paquet : (identifiant stable, libellé).
pub type SubTab = (&'static str, &'static str);

/// Onglets d'un paquet chargé (aucun : une seule page). Chaque paquet
/// concerné déclare les siens dans sa constante `TABS`.
pub fn tabs(id: &str) -> &'static [SubTab] {
    match id {
        "desktop" => desktop::TABS,
        "theme" => theme::TABS,
        "screens" => screens::TABS,
        "launcher" => launcher::TABS,
        "bar" => bar::TABS,
        "workspaces" => workspaces::TABS,
        "layouts" => layouts::TABS,
        "animations" => animations::TABS,
        "widgets" => widgets::TABS,
        "lock" => lock::TABS,
        _ => &[],
    }
}

/// Onglet d'un paquet désigné en ligne de commande (`bureau/raccourcis`) :
/// identifiant ou libellé, sans casse ni accents.
pub fn find_tab(pkg: &str, name: &str) -> Option<&'static str> {
    let fold = |s: &str| {
        s.to_lowercase()
            .chars()
            .map(|c| match c {
                'é' | 'è' | 'ê' | 'ë' => 'e',
                'à' | 'â' => 'a',
                'î' | 'ï' => 'i',
                'ô' => 'o',
                'û' | 'ù' => 'u',
                'ç' => 'c',
                other => other,
            })
            .collect::<String>()
    };
    let want = fold(name.trim());
    tabs(pkg)
        .iter()
        .find(|(id, label)| {
            // Identifiant, libellé français ou anglais (`bureau/raccourcis`, `desktop/shortcuts`).
            fold(id) == want
                || fold(label) == want
                || fold(crate::i18n::tr_in(crate::i18n::Lang::En, label)) == want
        })
        .map(|(id, _)| *id)
}

impl App {
    /// Onglet affiché d'un paquet : le dernier choisi, sinon le premier.
    pub(in crate::ui) fn subtab(&self, pkg: &str) -> &'static str {
        let t = visible_tabs(pkg, self.advanced());
        self.subtabs
            .get(pkg)
            .copied()
            .filter(|s| t.iter().any(|(id, _)| id == s))
            .or_else(|| t.first().map(|(id, _)| *id))
            .unwrap_or("")
    }

    /// Configuration avancée : tous les détails de l'écran Réglages.
    pub(in crate::ui) fn advanced(&self) -> bool {
        self.cfg.general.advanced_settings
    }

    /// Élément affiché seulement en configuration avancée (sinon rien, sans
    /// laisser d'espace dans la colonne).
    pub(in crate::ui) fn adv<'a>(&self, e: impl Into<Element<'a, Msg>>) -> Element<'a, Msg> {
        if self.advanced() {
            e.into()
        } else {
            iced::widget::Space::new().width(0).height(0).into()
        }
    }

    /// Suppression en deux temps : « × », puis « Supprimer ? » (armé) qui
    /// confirme. Le même message arme puis confirme ; le paquet retient
    /// l'élément armé.
    pub(in crate::ui) fn delete_button<'a>(&self, armed: bool, msg: Msg) -> Element<'a, Msg> {
        use iced::widget::button;
        let warn = iced::Color::from_rgb8(0xff, 0x95, 0x80);
        let fg = super::color(self.cfg.general.theme.fg, 1.0);
        button(
            crate::i18n::text(if armed {
                crate::i18n::tr("Supprimer ?")
            } else {
                "×"
            })
            .font(self.font)
            .size(if armed { 12.0 } else { 15.0 }),
        )
        .padding(if armed { [3, 10] } else { [1, 8] })
        .on_press(msg)
        .style(move |_, status| button::Style {
            background: Some(
                if armed {
                    iced::Color { a: 0.85, ..warn }
                } else if matches!(status, button::Status::Hovered) {
                    iced::Color { a: 0.25, ..warn }
                } else {
                    iced::Color::TRANSPARENT
                }
                .into(),
            ),
            text_color: if armed { iced::Color::BLACK } else { fg },
            border: iced::Border::default().rounded(999),
            ..Default::default()
        })
        .into()
    }

    /// L'onglet `tab` du paquet `pkg` est affiché.
    pub(in crate::ui) fn on_subtab(&self, pkg: &str, tab: &str) -> bool {
        self.subtab(pkg) == tab
    }
}

/// Paquet d'identifiant donné (ou d'alias de ligne de commande).
pub fn find(id: &str) -> Option<&'static PluginInfo> {
    let id = match id {
        "bureau" => "desktop",
        "icones" | "icônes" => "icons",
        "fond" => "wallpaper",
        "barre" => "bar",
        "effets" => "effects",
        "thème" | "themes" | "thèmes" | "couleurs" | "apparence" => "theme",
        "kitty" | "theme-apps" | "thème-applications" => "apptheme",
        "ecrans" | "écrans" => "screens",
        "lanceur" | "dock" => "launcher",
        "layout" | "tiling" | "agencement" | "agencements" => "layouts",
        "fx" | "animation" | "anim" => "animations",
        "applications" | "defaults" => "apps",
        "extensions" | "paquets" => "plugins",
        "ia" | "ai" | "lyra" => "assistant",
        "partage" | "export" | "import" => "share",
        "verrouillage" | "verrou" => "lock",
        other => other,
    };
    BUILTIN.iter().find(|p| p.id == id)
}

/// Message d'un paquet, enveloppé dans `PanelMsg::Plugin`.
#[derive(Debug, Clone)]
pub enum PluginMsg {
    Desktop(desktop::Msg),
    Icons(icons::Msg),
    Launcher(launcher::Msg),
    Apps(apps::Msg),
    Assistant(assistant::Msg),
    Plugins(plugins::Msg),
    Share(share::Msg),
    Lock(lock::Msg),
    Wallpaper(wallpaper::Msg),
    Widgets(widgets::Msg),
    Bar(bar::Msg),
    Screens(screens::Msg),
    Workspaces(workspaces::Msg),
    Layouts(layouts::Msg),
    Animations(animations::Msg),
    Sound(sound::Msg),
    Energy(energy::Msg),
    DateTime(datetime::Msg),
    Effects(effects::Msg),
    Theme(theme::Msg),
    AppTheme(apptheme::Msg),
}

pub(in crate::ui) fn update(app: &mut App, m: PluginMsg) -> Task<Msg> {
    match m {
        PluginMsg::Desktop(m) => app.desktop_update(m),
        PluginMsg::Icons(m) => app.icons_update(m),
        PluginMsg::Launcher(m) => app.launcher_update(m),
        PluginMsg::Apps(m) => app.apps_update(m),
        PluginMsg::Assistant(m) => app.assistant_update(m),
        PluginMsg::Plugins(m) => app.plugins_update(m),
        PluginMsg::Share(m) => app.share_update(m),
        PluginMsg::Lock(m) => app.lock_update(m),
        PluginMsg::Wallpaper(m) => app.wallpaper_update(m),
        PluginMsg::Widgets(m) => app.widgets_update(m),
        PluginMsg::Bar(m) => app.bar_update(m),
        PluginMsg::Screens(m) => app.screens_update(m),
        PluginMsg::Workspaces(m) => app.workspaces_update(m),
        PluginMsg::Layouts(m) => app.layouts_update(m),
        PluginMsg::Animations(m) => app.animations_update(m),
        PluginMsg::Sound(m) => app.sound_update(m),
        PluginMsg::Energy(m) => app.energy_update(m),
        PluginMsg::DateTime(m) => app.datetime_update(m),
        PluginMsg::Effects(m) => app.effects_update(m),
        PluginMsg::Theme(m) => app.theme_update(m),
        PluginMsg::AppTheme(m) => app.apptheme_update(m),
    }
}

pub(in crate::ui) fn view<'a>(app: &'a App, p: &'a Panel, id: &str) -> Element<'a, Msg> {
    match id {
        "icons" => app.tab_icons(p),
        "launcher" => app.launcher_page(),
        "apps" => app.apps_page(),
        "assistant" => app.assistant_page(),
        "plugins" => app.plugins_page(),
        "share" => app.share_page(),
        "lock" => app.lock_page(),
        "wallpaper" => app.tab_wallpaper(p),
        "widgets" => app.tab_widgets(p),
        "bar" => app.tab_bar(),
        "screens" => app.tab_screens(),
        "workspaces" => app.tab_workspaces(p),
        "layouts" => app.layouts_page(),
        "animations" => app.animations_page(),
        "sound" => app.sound_page(),
        "energy" => app.energy_page(),
        "datetime" => app.datetime_page(),
        "effects" => app.tab_effects(),
        "theme" => app.theme_page(),
        "apptheme" => app.apptheme_page(),
        _ => {
            let (_, ws) = app.panel_ctx().unwrap_or_default();
            app.tab_desktop(&ws)
        }
    }
}

/// Un paquet devient visible : de quoi se remplir (détection, vignettes…).
pub(in crate::ui) fn on_show(app: &mut App, id: &str) -> Task<Msg> {
    match id {
        "screens" => App::detect_screens(),
        "layouts" => Task::batch([
            layouts::load_current(),
            layouts::load_binds(app.config_path.clone()),
        ]),
        "apps" => apps::load_system(),
        "animations" => animations::load_current(),
        "sound" => sound::load(),
        "energy" => energy::load(),
        "datetime" => datetime::load(),
        "assistant" => assistant::load_status(),
        "theme" => theme::load_library(app.config_path.clone()),
        // Relu à chaque arrivée sur l'onglet : les images ajoutées ou retirées
        // apparaissent (vignettes déjà faites gardées, seules les nouvelles se chargent).
        "wallpaper" => match &app.panel {
            Some(p) => wallpaper::load_images(p.wallpaper.folder.clone()),
            None => Task::none(),
        },
        _ => Task::none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registre() {
        let ids: Vec<&str> = BUILTIN.iter().map(|p| p.id).collect();
        let mut unique = ids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), ids.len(), "identifiants uniques");
        assert!(BUILTIN.iter().all(|p| p.builtin && !p.label.is_empty()));
        assert_eq!(find("fond").map(|p| p.id), Some("wallpaper"));
        assert_eq!(find("thème").map(|p| p.id), Some("theme"));
        assert_eq!(find("screens").map(|p| p.id), Some("screens"));
        assert!(find("inconnu").is_none());
    }

    /// Textes des registres (constantes, traduites à l'affichage) : chacun a
    /// sa traduction anglaise. HYPRDESK_I18N_TODO2=fichier écrit les manquants.
    #[test]
    fn registres_traduits() {
        let mut all: Vec<&str> = Vec::new();
        all.extend(GROUPS.iter().map(|g| g.label));
        all.extend(
            crate::ui::tour::STEPS
                .iter()
                .flat_map(|s| [s.title, s.body]),
        );
        for p in BUILTIN {
            all.extend([p.label, p.summary]);
            all.extend(tabs(p.id).iter().map(|(_, l)| *l));
        }
        for w in crate::widgets::BUILTINS {
            all.extend([w.name, w.description]);
        }
        all.extend(crate::config::BAR_ITEMS.iter().map(|(_, l)| *l));
        all.extend(crate::tiling::ACTIONS.iter().map(|a| a.label));
        all.extend(
            crate::tiling::CUSTOM
                .iter()
                .flat_map(|c| [c.label, c.summary]),
        );
        all.extend(crate::tiling::OPTIONS.iter().map(|o| o.label));
        for o in crate::tiling::OPTIONS {
            match &o.kind {
                crate::tiling::Kind::IntChoice(l) => all.extend(l.iter().map(|(_, s)| *s)),
                crate::tiling::Kind::Choice(l) => all.extend(l.iter().map(|(_, s)| *s)),
                _ => {}
            }
        }
        let missing = crate::i18n::untranslated("registres", all);
        assert!(missing.is_empty(), "sans traduction : {missing:?}");
    }

    /// Chaque paquet est dans un seul groupe de la navigation ; les paquets
    /// et onglets avancés existent.
    #[test]
    fn groupes() {
        for p in BUILTIN {
            let n = GROUPS.iter().filter(|g| g.ids.contains(&p.id)).count();
            assert_eq!(n, 1, "{} : dans {n} groupe(s)", p.id);
        }
        for g in GROUPS {
            assert!(g.ids.iter().all(|id| find(id).is_some()), "{}", g.label);
        }
        assert!(ADVANCED_ONLY.iter().all(|id| find(id).is_some()));
        for (p, t) in ADVANCED_TABS {
            assert!(tabs(p).iter().any(|(id, _)| id == t), "{p}/{t}");
        }
        assert!(!visible("plugins", false) && visible("plugins", true));
        assert_eq!(visible_tabs("desktop", false).len(), 2);
        assert_eq!(visible_tabs("desktop", true).len(), 4);
    }

    /// Onglets : identifiants uniques, désignables sans casse ni accents.
    #[test]
    fn onglets() {
        for p in BUILTIN {
            let t = tabs(p.id);
            let mut ids: Vec<&str> = t.iter().map(|(id, _)| *id).collect();
            ids.sort();
            ids.dedup();
            assert_eq!(ids.len(), t.len(), "{} : onglets uniques", p.id);
            assert!(
                t.is_empty() || t.len() >= 2,
                "{} : au moins deux onglets",
                p.id
            );
        }
        assert_eq!(find_tab("desktop", "Raccourcis"), Some("raccourcis"));
        assert_eq!(find_tab("launcher", "épinglées"), Some("epinglees"));
        assert_eq!(find_tab("screens", "ÉCRAN"), Some("ecran"));
        assert_eq!(find_tab("theme", "texte"), Some("texte"));
        assert_eq!(find_tab("desktop", "shortcuts"), Some("raccourcis"));
        assert_eq!(find_tab("desktop", "inconnu"), None);
        assert_eq!(find_tab("apps", "x"), None);
    }
}
