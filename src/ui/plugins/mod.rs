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

pub mod about;
pub mod animations;
pub mod apps;
pub mod apptheme;
pub mod assistant;
pub mod autostart;
pub mod bar;
pub mod bluetooth;
pub mod colors;
pub mod datetime;
pub mod desktop;
pub mod effects;
pub mod energy;
pub mod history;
pub mod hyprconfig;
pub mod icons;
pub mod index;
pub mod input;
pub mod keys;
pub mod launcher;
pub mod layouts;
pub mod lock;
pub mod login;
pub mod modules;
pub mod network;
pub mod panorama;
pub mod passwords;
pub mod performance;
#[allow(clippy::module_inception)]
pub mod plugins;
pub mod screens;
pub mod share;
pub mod sound;
pub mod theme;
pub mod wallpaper;
pub mod widgets;
pub mod windows;
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

/// Paquets intégrés (l'ordre de la navigation vient de [`GROUPS`]).
pub const BUILTIN: &[PluginInfo] = &[
    desktop::INFO,
    keys::INFO,
    icons::INFO,
    launcher::INFO,
    index::INFO,
    apps::INFO,
    autostart::INFO,
    assistant::INFO,
    theme::INFO,
    apptheme::INFO,
    wallpaper::INFO,
    widgets::INFO,
    modules::INFO,
    bar::INFO,
    panorama::INFO,
    screens::INFO,
    colors::INFO,
    workspaces::INFO,
    windows::INFO,
    layouts::INFO,
    animations::INFO,
    performance::INFO,
    sound::INFO,
    energy::INFO,
    network::INFO,
    bluetooth::INFO,
    datetime::INFO,
    input::INFO,
    lock::INFO,
    login::INFO,
    passwords::INFO,
    share::INFO,
    plugins::INFO,
    history::INFO,
    hyprconfig::INFO,
    about::INFO,
];

/// Groupe de la navigation des Réglages : titre, paquets dans l'ordre.
pub struct Group {
    pub label: &'static str,
    pub ids: &'static [&'static str],
}

/// Navigation des Réglages, par groupes (chaque paquet dans un seul groupe,
/// libellés uniques) : d'abord ce qui se voit (bureau, apparence, barre),
/// puis le matériel, le système, les applications, et enfin la configuration
/// de hyprdesk elle-même.
pub const GROUPS: &[Group] = &[
    Group {
        label: "Bureau",
        ids: &["desktop", "wallpaper", "icons"],
    },
    // Les plugins (Barre, Lanceur, Panorama), puis ce qui s'y ajoute :
    // modules du Panorama, widgets, plugins installés.
    Group {
        label: "Plugins",
        ids: &[
            "bar", "launcher", "panorama", "modules", "widgets", "plugins",
        ],
    },
    Group {
        label: "Apparence",
        ids: &["theme", "apptheme", "animations"],
    },
    Group {
        label: "Fenêtres",
        ids: &["windows", "workspaces", "layouts"],
    },
    Group {
        label: "Écrans",
        ids: &["screens", "colors"],
    },
    Group {
        label: "Clavier et souris",
        ids: &["input", "keys"],
    },
    Group {
        label: "Son et réseau",
        ids: &["sound", "network", "bt"],
    },
    Group {
        label: "Système",
        ids: &[
            "energy",
            "performance",
            "lock",
            "login",
            "datetime",
            "passwords",
        ],
    },
    Group {
        label: "Applications",
        ids: &["apps", "autostart", "assistant", "index"],
    },
    Group {
        label: "Configuration",
        ids: &["history", "hyprconfig", "share", "about"],
    },
];

/// Paquets affichés seulement en configuration avancée.
pub const ADVANCED_ONLY: &[&str] = &["share"];

/// Onglets affichés seulement en configuration avancée : (paquet, onglet).
///
/// Règle : un onglet fait seulement d'options brutes de Hyprland, de
/// fichiers ou de réglages d'expert est avancé ; sur une page de base, les
/// options brutes de Hyprland sont encadrées (`App::hyprset_tab`).
pub const ADVANCED_TABS: &[(&str, &str)] = &[
    ("desktop", "modes"),
    ("desktop", "session"),
    ("desktop", "dependances"),
    ("assistant", "services"),
    ("performance", "hyprdesk"),
    ("theme", "transparence"),
    ("animations", "courbes"),
    ("animations", "bureau"),
    ("layouts", "custom"),
    ("windows", "comportement"),
    ("windows", "groupes"),
    ("input", "peripheriques"),
    ("hyprconfig", "environnement"),
    ("hyprconfig", "plugins"),
    ("hyprconfig", "permissions"),
    ("hyprconfig", "avance"),
    // Carte graphique de rendu (Hyprland, hyprdesk) : MLG-211.
    ("performance", "gpu"),
];

/// Options avancées de Hyprland (registre `hyprset`) rangées sous une page
/// existante, là où on les cherche : (paquet, onglet — vide : la page sans
/// onglet —, clé de groupe de `hyprset::GROUPS`). Affichées en configuration
/// avancée seulement.
pub const ADVANCED_PLACES: &[(&str, &str, &str)] = &[
    ("input", "souris", "souris"),
    ("input", "pave", "pave"),
    ("workspaces", "ecrans", "workspaces"),
    ("keys", "ensemble", "raccourcis"),
    ("lock", "veille", "veille"),
    ("wallpaper", "", "fond"),
    ("animations", "effets", "flou"),
];

/// Couleur des réglages de la configuration avancée (liseré, pastille,
/// point dans le menu) : distincte de l'accent et de l'orange des
/// avertissements.
pub const ADV_COLOR: iced::Color = iced::Color::from_rgb(0.545, 0.914, 0.992);

/// Réglage avancé : cadre cyan, fond à peine teinté.
pub fn adv_frame<'a>(e: impl Into<Element<'a, Msg>>) -> Element<'a, Msg> {
    use iced::widget::container;
    container(e)
        .width(iced::Length::Fill)
        .padding([10, 12])
        .style(|_| container::Style {
            background: Some(
                iced::Color {
                    a: 0.04,
                    ..ADV_COLOR
                }
                .into(),
            ),
            border: iced::Border::default()
                .rounded(10)
                .width(1)
                .color(iced::Color {
                    a: 0.45,
                    ..ADV_COLOR
                }),
            ..Default::default()
        })
        .into()
}

/// Icône de la configuration avancée (fiole), dans la couleur donnée.
pub fn adv_glyph<'a>(size: f32, color: iced::Color) -> Element<'a, Msg> {
    match super::icons::get("hyprdesk-advanced") {
        Some(handle) => iced::widget::svg(handle)
            .width(size)
            .height(size)
            .style(move |_, _| iced::widget::svg::Style { color: Some(color) })
            .into(),
        None => iced::widget::Space::new().width(0).height(0).into(),
    }
}

/// Repère d'un paquet ou d'un onglet avancé : la fiole en cyan.
pub fn adv_icon<'a>(size: f32) -> Element<'a, Msg> {
    adv_glyph(size, ADV_COLOR)
}

/// Sources des paquets, pour compter leurs sections avancées
/// (`self.adv(…)`) ; un fichier qui en contient doit y figurer (test).
const ADV_SOURCES: &[(&str, &str)] = &[
    ("animations", include_str!("animations.rs")),
    ("assistant", include_str!("assistant.rs")),
    ("bar", include_str!("bar.rs")),
    ("colors", include_str!("colors.rs")),
    ("desktop", include_str!("desktop.rs")),
    ("index", include_str!("index.rs")),
    ("input", include_str!("input.rs")),
    ("keys", include_str!("keys.rs")),
    ("layouts", include_str!("layouts.rs")),
    ("lock", include_str!("lock.rs")),
    ("panorama", include_str!("panorama.rs")),
    ("panorama", include_str!("panorama/design.rs")),
    ("modules", include_str!("panorama/modules.rs")),
    ("plugins", include_str!("plugins.rs")),
    ("screens", include_str!("screens.rs")),
    ("windows", include_str!("windows.rs")),
    ("windows", include_str!("../hyprlists.rs")),
    ("workspaces", include_str!("workspaces.rs")),
];

/// Sections avancées écrites dans un source (hors tests).
fn adv_calls(src: &str) -> usize {
    src.split("#[cfg(test)]")
        .next()
        .unwrap_or("")
        .matches("self.adv(")
        .count()
}

/// Nombre de réglages avancés d'un paquet : onglets avancés, sections
/// d'options de Hyprland ajoutées à ses pages, sections encadrées.
pub fn adv_count(id: &str) -> usize {
    let tabs = tabs(id)
        .iter()
        .filter(|(t, _)| ADVANCED_TABS.contains(&(id, *t)))
        .count();
    let places = ADVANCED_PLACES.iter().filter(|(p, _, _)| *p == id).count();
    let sections: usize = ADV_SOURCES
        .iter()
        .filter(|(p, _)| *p == id)
        .map(|(_, s)| adv_calls(s))
        .sum();
    tabs + places + sections
}

/// Pastille « Avancé » en tête d'une page ou d'un onglet avancé.
pub fn adv_badge<'a>(font: iced::Font) -> Element<'a, Msg> {
    use iced::widget::container;
    let ink = iced::Color::from_rgb8(0x14, 0x14, 0x1c);
    container(
        iced::widget::row![
            adv_glyph(12.0, ink),
            crate::i18n::text("Avancé").font(font).size(11).color(ink),
        ]
        .spacing(4)
        .align_y(iced::Alignment::Center),
    )
    .padding([2, 8])
    .style(|_| container::Style {
        background: Some(ADV_COLOR.into()),
        border: iced::Border::default().rounded(999),
        ..Default::default()
    })
    .into()
}

/// Paquet ou onglet réservé à la configuration avancée.
pub fn is_advanced(id: &str, tab: Option<&str>) -> bool {
    ADVANCED_ONLY.contains(&id) || tab.is_some_and(|t| ADVANCED_TABS.contains(&(id, t)))
}

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
        "keys" => keys::TABS,
        "theme" => theme::TABS,
        "screens" => screens::TABS,
        "launcher" => launcher::TABS,
        "bar" => bar::TABS,
        "panorama" => panorama::TABS,
        "workspaces" => workspaces::TABS,
        "layouts" => layouts::TABS,
        "animations" => animations::TABS,
        "windows" => windows::TABS,
        "performance" => performance::TABS,
        "widgets" => widgets::TABS,
        "lock" => lock::TABS,
        "input" => input::TABS,
        "autostart" => autostart::TABS,
        "assistant" => assistant::TABS,
        "hyprconfig" => hyprconfig::TABS,
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
    // Onglets renommés : l'ancien nom mène au nouveau.
    let want = match (pkg, want.as_str()) {
        ("theme", "effets" | "effects") => "transparence".to_string(),
        _ => want,
    };
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
            adv_frame(e)
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
        "raccourcis" | "shortcuts" | "touches" | "binds" | "keybinds" | "keys" => "keys",
        "icones" | "icônes" => "icons",
        "fond" => "wallpaper",
        "barre" => "bar",
        "bandeau" | "edge" | "tactile" | "touch" | "ecran-tactile" | "écran-tactile" => "panorama",
        // Ancien paquet Effets : onglet Animations → Bureau.
        "effets" | "effects" | "transitions" => "animations",
        "thème" | "themes" | "thèmes" | "couleurs" | "apparence" => "theme",
        "kitty" | "theme-apps" | "thème-applications" => "apptheme",
        "ecrans" | "écrans" => "screens",
        "couleurs-ecrans" | "couleurs-écrans" | "calibration" | "gamma" | "icc" | "mire" => {
            "colors"
        }
        "lanceur" | "dock" => "launcher",
        "layout" | "tiling" | "agencement" | "agencements" => "layouts",
        "fx" | "animation" | "anim" => "animations",
        "perf" | "performances" | "rendu" | "gpu" | "cpu" => "performance",
        "applications" | "defaults" => "apps",
        "hyprland" | "hyprland.lua" | "menage" | "ménage" | "conflits" => "hyprconfig",
        "historique" | "history" | "rollback" | "retour" => "history",
        "demarrage"
        | "démarrage"
        | "demarrage-automatique"
        | "démarrage-automatique"
        | "exec-once"
        | "startup" => "autostart",
        "extensions" | "paquets" => "plugins",
        "ia" | "ai" | "lyra" | "smartui" | "services-ia" => "assistant",
        "partage" | "export" | "import" => "share",
        "a-propos" | "à-propos" | "apropos" | "version" | "info" => "about",
        "verrouillage" | "verrou" => "lock",
        "son" | "audio" | "volume" => "sound",
        "reseau" | "réseau" | "wifi" | "wi-fi" | "vpn" => "network",
        "bluetooth" => "bt",
        "energie" | "énergie" | "power" | "batterie" => "energy",
        "hue" | "lumieres" | "lumières" | "lights" | "philips-hue" | "notes" | "module" => {
            "modules"
        }
        "installes" | "installés" | "installed" => "plugins",
        "date" | "heure" | "date-heure" | "time" | "fuseau" => "datetime",
        "clavier" | "souris" | "keyboard" | "mouse" | "touchpad" | "pave" | "pavé" | "entrees"
        | "entrées" => "input",
        "connexion" | "ecran-connexion" | "écran-connexion" | "greeter" | "greetd" | "gdm"
        | "display-manager" => "login",
        "mots-de-passe" | "mdp" | "mymfa" | "trousseau" | "keyring" | "2fa" => "passwords",
        other => other,
    };
    BUILTIN.iter().find(|p| p.id == id)
}

/// Message d'un paquet, enveloppé dans `PanelMsg::Plugin`.
#[derive(Debug, Clone)]
pub enum PluginMsg {
    Desktop(desktop::Msg),
    Keys(keys::Msg),
    Icons(icons::Msg),
    Launcher(launcher::Msg),
    Index(index::Msg),
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
    Performance(performance::Msg),
    Sound(sound::Msg),
    Energy(energy::Msg),
    Network(network::Msg),
    Bluetooth(bluetooth::Msg),
    DateTime(datetime::Msg),
    Input(input::Msg),
    Autostart(autostart::Msg),
    HyprConfig(hyprconfig::Msg),
    Colors(colors::Msg),
    Login(login::Msg),
    Passwords(passwords::Msg),
    Effects(effects::Msg),
    Theme(theme::Msg),
    AppTheme(apptheme::Msg),
    About(about::Msg),
    Panorama(panorama::Msg),
}

pub(in crate::ui) fn update(app: &mut App, m: PluginMsg) -> Task<Msg> {
    match m {
        PluginMsg::Desktop(m) => app.desktop_update(m),
        PluginMsg::Keys(m) => app.keys_update(m),
        PluginMsg::Icons(m) => app.icons_update(m),
        PluginMsg::Launcher(m) => app.launcher_update(m),
        PluginMsg::Index(m) => app.index_update(m),
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
        PluginMsg::Performance(m) => app.performance_update(m),
        PluginMsg::Sound(m) => app.sound_update(m),
        PluginMsg::Energy(m) => app.energy_update(m),
        PluginMsg::Network(m) => app.network_update(m),
        PluginMsg::Bluetooth(m) => app.bluetooth_update(m),
        PluginMsg::DateTime(m) => app.datetime_update(m),
        PluginMsg::Input(m) => app.input_update(m),
        PluginMsg::Autostart(m) => app.autostart_update(m),
        PluginMsg::HyprConfig(m) => app.hyprconfig_update(m),
        PluginMsg::Colors(m) => app.colors_update(m),
        PluginMsg::Login(m) => app.login_update(m),
        PluginMsg::Passwords(m) => app.passwords_update(m),
        PluginMsg::Effects(m) => app.effects_update(m),
        PluginMsg::Theme(m) => app.theme_update(m),
        PluginMsg::AppTheme(m) => app.apptheme_update(m),
        PluginMsg::About(m) => app.about_update(m),
        PluginMsg::Panorama(m) => app.panorama_settings_update(m),
    }
}

pub(in crate::ui) fn view<'a>(app: &'a App, p: &'a Panel, id: &str) -> Element<'a, Msg> {
    let tab = app.subtab(id);
    let mut page = page(app, p, id);
    // Page ou onglet avancé : pastille en tête.
    if is_advanced(id, Some(tab)) {
        page = iced::widget::column![adv_badge(app.font), page]
            .spacing(10)
            .into();
    }
    if !app.advanced() {
        return page;
    }
    let extra: Vec<&str> = ADVANCED_PLACES
        .iter()
        .filter(|(pkg, t, _)| *pkg == id && (t.is_empty() || *t == tab))
        .map(|(_, _, k)| *k)
        .collect();
    if extra.is_empty() {
        return page;
    }
    let mut col = iced::widget::column![page].spacing(14);
    for k in extra {
        col = col.push(adv_frame(app.hyprset_groups(k)));
    }
    col.into()
}

fn page<'a>(app: &'a App, p: &'a Panel, id: &str) -> Element<'a, Msg> {
    match id {
        "icons" => app.tab_icons(p),
        "keys" => app.keys_page(),
        "launcher" => app.launcher_page(),
        "index" => app.index_page(),
        "apps" => app.apps_page(),
        "assistant" => app.assistant_page(),
        "plugins" => app.plugins_page(),
        "about" => app.about_page(),
        "share" => app.share_page(),
        "lock" => app.lock_page(),
        "wallpaper" => app.tab_wallpaper(p),
        "widgets" => app.tab_widgets(p),
        "modules" => app.modules_page(),
        "bar" => app.tab_bar(),
        "panorama" => app.panorama_page(),
        "screens" => app.tab_screens(),
        "workspaces" => app.tab_workspaces(p),
        "layouts" => app.layouts_page(),
        "windows" => app.windows_page(),
        "animations" => app.animations_page(),
        "performance" => app.performance_page(),
        "sound" => app.sound_page(),
        "energy" => app.energy_page(),
        "network" => app.network_page(),
        "bt" => app.bluetooth_page(),
        "datetime" => app.datetime_page(),
        "input" => app.input_page(),
        "autostart" => app.autostart_page(),
        "hyprconfig" => app.hyprconfig_page(),
        "history" => app.hyprconfig_history(),
        "colors" => app.colors_page(),
        "login" => app.login_page(),
        "passwords" => app.passwords_page(),
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
        "desktop" => Task::batch([
            desktop::load_places(),
            desktop::load_hyprbars(app.cfg.general.hyprbars_path.clone()),
        ]),
        "screens" => App::detect_screens(),
        "panorama" => Task::batch([panorama::load(), panorama::modules::load()]),
        "modules" => panorama::modules::load(),
        "keys" => app.keys_load(),
        "index" => {
            index::init_fields(app);
            index::load()
        }
        "layouts" => Task::batch([
            layouts::load_current(),
            layouts::load_binds(app.config_path.clone()),
        ]),
        "apps" => apps::load_system(),
        "animations" => animations::load_current(),
        "performance" => Task::batch([animations::load_current(), performance::load()]),
        "sound" => sound::load(),
        "energy" => energy::load(),
        "network" => network::load(),
        "bt" => bluetooth::load(),
        "datetime" => datetime::load(),
        "input" => input::load(),
        "autostart" => autostart::load(app.config_path.clone()),
        "hyprconfig" => Task::batch([hyprconfig::load(app), hyprconfig::load_hyprpm(app)]),
        "history" => hyprconfig::load_history(app),
        "colors" => colors::load(),
        "workspaces" => workspaces::load_foreign(app.config_path.clone()),
        "login" => login::load(),
        "lock" => lock::load(),
        "passwords" => passwords::load(),
        "assistant" => assistant::on_show(app),
        "about" => about::load(app.config_path.clone()),
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

    #[test]
    fn placements_avances_coherents() {
        // Chaque groupe du registre a sa place : un onglet propre ou une page.
        let own = [
            "fenetres",
            "fenetres-details",
            "comportement-fenetres",
            "groupes",
            "curseur",
            "gestes",
            "avance",
            "permissions",
        ];
        for (k, title, _) in crate::hyprset::GROUPS {
            assert!(
                own.contains(k) || ADVANCED_PLACES.iter().any(|(_, _, p)| p == k),
                "groupe « {title} » ({k}) sans place"
            );
        }
        for (pkg, tab, _) in ADVANCED_PLACES {
            assert!(BUILTIN.iter().any(|p| p.id == *pkg), "paquet {pkg}");
            assert!(
                tab.is_empty() || tabs(pkg).iter().any(|(t, _)| t == tab),
                "onglet {pkg}/{tab}"
            );
        }
    }

    /// Options brutes de Hyprland : sur un onglet avancé, ou encadrées sur
    /// un onglet de base ; seuls les groupes de base restent visibles sans
    /// la configuration avancée.
    #[test]
    fn options_brutes_marquees_avancees() {
        // Groupe du registre → onglet qui l'affiche, et s'il y est encadré.
        let places: &[(&str, &str, &str, bool)] = &[
            ("fenetres", "windows", "apparence", false),
            ("fenetres-details", "windows", "apparence", true),
            ("comportement-fenetres", "windows", "comportement", false),
            ("groupes", "windows", "groupes", false),
            ("curseur", "input", "curseur", true),
            ("gestes", "input", "gestes", true),
            ("avance", "hyprconfig", "avance", false),
            ("permissions", "hyprconfig", "permissions", false),
        ];
        for (k, title, _) in crate::hyprset::GROUPS {
            if ADVANCED_PLACES.iter().any(|(_, _, p)| p == k) {
                continue;
            }
            let (_, pkg, tab, framed) = places
                .iter()
                .find(|(g, ..)| g == k)
                .unwrap_or_else(|| panic!("groupe « {title} » ({k}) sans onglet"));
            assert!(tabs(pkg).iter().any(|(t, _)| t == tab), "{pkg}/{tab}");
            let basic = crate::hyprset::BASIC_GROUPS.contains(k);
            assert!(
                basic || *framed || is_advanced(pkg, Some(tab)),
                "{k} : options brutes sur {pkg}/{tab}, ni avancé ni encadré"
            );
            assert!(!(basic && *framed), "{k} : groupe de base encadré");
        }
        // Onglets nouveaux de Hyprland : marqués.
        for (p, t) in [
            ("input", "peripheriques"),
            ("hyprconfig", "plugins"),
            ("hyprconfig", "permissions"),
            ("hyprconfig", "environnement"),
            ("animations", "courbes"),
            ("animations", "bureau"),
            ("theme", "transparence"),
        ] {
            assert!(is_advanced(p, Some(t)), "{p}/{t}");
        }
        // Ancien paquet Effets : Animations → Bureau ; ancien onglet Thème → Effets.
        assert_eq!(find("effets").map(|p| p.id), Some("animations"));
        assert_eq!(find_tab("theme", "effets"), Some("transparence"));
        assert!(BUILTIN.iter().all(|p| p.id != "effects"));
    }

    /// Chaque fichier de paquet qui encadre une section avancée est compté.
    #[test]
    fn sections_avancees_comptees() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ui");
        let mut files = Vec::new();
        fn walk(d: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(d).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().is_some_and(|x| x == "rs") {
                    out.push(p);
                }
            }
        }
        walk(&dir, &mut files);
        let counted: usize = ADV_SOURCES.iter().map(|(_, s)| adv_calls(s)).sum();
        let total: usize = files
            .iter()
            // mod.rs : `adv` lui-même ; ui/hyprset.rs : le repli de
            // `hyprset_tab`, dont les appelants encadrent eux-mêmes.
            .filter(|f| !f.ends_with("plugins/mod.rs") && !f.ends_with("ui/hyprset.rs"))
            .map(|f| adv_calls(&std::fs::read_to_string(f).unwrap()))
            .sum();
        assert_eq!(
            counted, total,
            "un fichier avec self.adv( manque à ADV_SOURCES"
        );
        assert!(adv_count("input") >= 2);
        assert!(icons_ok());
    }

    fn icons_ok() -> bool {
        crate::ui::icons::get("hyprdesk-advanced").is_some()
    }

    #[test]
    fn chaque_paquet_a_son_icone() {
        let missing: Vec<&str> = BUILTIN
            .iter()
            .filter(|p| crate::ui::icons::get(p.icon).is_none())
            .map(|p| p.id)
            .collect();
        assert!(missing.is_empty(), "paquets sans glyphe : {missing:?}");
    }

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
                .flat_map(|s| [s.title, s.intro]),
        );
        all.extend(
            crate::ui::tour::STEPS
                .iter()
                .flat_map(|s| s.items.iter().flat_map(|i| [i.title, i.body])),
        );
        for p in BUILTIN {
            all.extend([p.label, p.summary]);
            all.extend(tabs(p.id).iter().map(|(_, l)| *l));
        }
        for w in crate::widgets::BUILTINS {
            all.extend([w.name, w.description]);
        }
        all.extend(crate::ui::hyprlists::texts());
        all.extend(crate::config::BAR_ITEMS.iter().map(|(_, l)| *l));
        all.extend(crate::config::BAR_COLOR_SLOTS.iter().map(|(_, l)| *l));
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

    /// Chaque paquet est dans un seul groupe de la navigation ; les libellés
    /// de groupes sont uniques (deux « Système » se confondraient) ; les
    /// paquets et onglets avancés existent.
    #[test]
    fn groupes() {
        let mut labels: Vec<&str> = GROUPS.iter().map(|g| g.label).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), GROUPS.len(), "libellés de groupes uniques");
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
        assert!(!visible("share", false) && visible("share", true));
        // Groupe Plugins : les plugins, leurs modules, les widgets et les
        // plugins installés, toujours visibles.
        let g = GROUPS.iter().find(|g| g.label == "Plugins").unwrap();
        assert_eq!(
            g.ids,
            &[
                "bar", "launcher", "panorama", "modules", "widgets", "plugins"
            ]
        );
        assert!(g.ids.iter().all(|id| visible(id, false)));
        assert_eq!(find("hue").map(|p| p.id), Some("modules"));
        assert_eq!(visible_tabs("desktop", false).len(), 1);
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
        assert_eq!(
            find_tab("keys", "Fenetres et workspaces"),
            Some("essentiels")
        );
        assert_eq!(find_tab("launcher", "épinglées"), Some("epinglees"));
        assert_eq!(find_tab("screens", "ÉCRAN"), Some("ecran"));
        assert_eq!(find_tab("theme", "texte"), Some("texte"));
        assert_eq!(find_tab("keys", "overview"), Some("ensemble"));
        assert_eq!(find_tab("desktop", "inconnu"), None);
        assert_eq!(find_tab("apps", "x"), None);
    }
}
