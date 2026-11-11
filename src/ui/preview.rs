//! Aperçu interactif isolé : mêmes vues, données locales, aucun IPC ni lancement.
use super::plugins::{PluginMsg, desktop, icons};
use super::*;
use crate::config::{Anchor, BarDef, BarItem, BarPosition, Origin, Selector, WidgetDef};
use iced::widget::{Stack, container, pin};

#[derive(Debug, Clone)]
enum PreviewMsg {
    Ui(Box<Msg>),
    Capture,
    Captured(iced::window::Screenshot),
    Inspect,
    Inspected(iced::window::Screenshot),
}

struct Preview {
    app: App,
    desktop: Id,
    bar: Id,
    output: Option<PathBuf>,
    scene: usize,
    remaining_scenes: std::collections::VecDeque<usize>,
    size: Size,
}

const SCENES: [&str; 54] = [
    "desktop",
    "typography",
    "icons",
    "wallpaper",
    "widgets",
    "bar",
    "effects",
    "volume",
    "calendar",
    "workspaces",
    "screens",
    "launcher",
    "apps",
    "assistant",
    "plugins",
    "share",
    "launcher-idle",
    "launcher-hover",
    "launcher-top",
    "launcher-left",
    "launcher-right",
    "launcher-popup",
    "chat",
    "theme",
    "bureau",
    "launcher-opening",
    "launcher-menu-opening",
    "launcher-launching",
    "launcher-bright",
    "launcher-dark",
    "launcher-menu-long",
    "layouts",
    "tour",
    "veille",
    "mode-launcher",
    "mode-hidden",
    "osd",
    "animations",
    "animations-effets",
    "polkit",
    "notifications",
    "probleme",
    "recherche",
    "presse-papiers",
    "son",
    "energie",
    "date-heure",
    "reseau",
    "bluetooth",
    "menu-icone",
    "fenetres",
    "connexion",
    "luminosite",
    "plugin-config",
];

pub fn run(output: Option<PathBuf>) -> iced::Result {
    let scale = std::env::var("HYPRDESK_PREVIEW_SCALE")
        .ok()
        .and_then(|s| s.parse::<f32>().ok())
        .filter(|v| v.is_finite())
        .unwrap_or(1.0)
        .clamp(1.0, 3.0);
    let width = std::env::var("HYPRDESK_PREVIEW_WIDTH")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(1440.0)
        .clamp(640.0, 2560.0);
    let height = std::env::var("HYPRDESK_PREVIEW_HEIGHT")
        .ok()
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(900.0)
        .clamp(480.0, 2560.0);
    iced::application(
        move || Preview::boot(output.clone(), Size::new(width, height)),
        Preview::update,
        Preview::view,
    )
    .settings(iced::Settings {
        fonts: crate::fonts::bundled(),
        default_font: crate::fonts::DEFAULT,
        ..Default::default()
    })
    .title(crate::i18n::tr("hyprdesk · aperçu isolé"))
    .window_size(Size::new(width, height))
    .scale_factor(move |_: &Preview| scale)
    .theme(|p: &Preview| p.app.theme.clone())
    .subscription(|p: &Preview| {
        let frames = if p.app.launcher_animating() {
            iced::window::frames().map(|_| PreviewMsg::Ui(Box::new(Msg::Frame)))
        } else if p.app.animating() {
            iced::time::every(Duration::from_millis(16))
                .map(|_| PreviewMsg::Ui(Box::new(Msg::Frame)))
        } else {
            Subscription::none()
        };
        let capture = iced::event::listen_with(|event, _, _| match event {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::F12),
                ..
            }) => Some(PreviewMsg::Inspect),
            _ => None,
        });
        Subscription::batch([frames, capture])
    })
    .run()
}

fn widget(id: &str, kind: WidgetKind, anchor: Anchor) -> WidgetDef {
    WidgetDef {
        origin: Origin::User,
        sel: Selector::default(),
        id: id.into(),
        type_id: id.into(),
        kind,
        anchor,
        x: 0.0,
        y: 0.0,
        size: 0.0,
        interval: Duration::from_secs(1),
        card: true,
        in_bar: false,
    }
}

impl Preview {
    fn boot(output: Option<PathBuf>, size: Size) -> (Self, Task<PreviewMsg>) {
        let (mut app, _) = App::boot_with(PathBuf::from("/tmp/hyprdesk-preview/config.lua"), true);
        let (desktop, bar) = (Id::unique(), Id::unique());
        app.font = crate::fonts::DEFAULT;
        app.monitors = vec![Monitor {
            name: crate::i18n::tr("APERÇU").into(),
            width: size.width as u32,
            height: size.height as u32,
            workspace: "1".into(),
            focused: true,
            reserved: (0.0, 44.0, 0.0, 0.0),
            x: 0.0,
            y: 0.0,
            scale: 1.0,
        }];
        app.surfaces
            .insert(desktop, crate::i18n::tr("APERÇU").into());
        app.sizes.insert(desktop, size);
        app.raised.insert(desktop);
        app.visible = true;
        app.volume = Some((0.64, false));
        let dev = |id: u32, name: &str, default: bool| bar::AudioDevice {
            id,
            name: name.into(),
            default,
        };
        app.audio = bar::Audio {
            mic: Some((0.9, false)),
            outputs: vec![
                dev(61, "Arctis Nova Pro Wireless Analog Stereo", true),
                dev(71, "Built-in Audio Digital Stereo (HDMI)", false),
            ],
            inputs: vec![
                dev(54, "Arctis Nova Pro Wireless Mono", true),
                dev(59, "HD Webcam eMeet C960 Analog Stereo", false),
                dev(60, "Built-in Audio Analog Stereo", false),
            ],
        };
        app.screens = crate::screens::parse(&serde_json::json!([
            { "name": "HDMI-A-2", "description": "ROG PG279Q", "width": 2560, "height": 1440,
              "refreshRate": 59.951, "x": 0, "y": 0, "scale": 1.0, "transform": 0,
              "availableModes": ["2560x1440@59.95Hz", "1920x1080@60.00Hz"] },
            { "name": "DP-1", "description": "PX7 Prime", "width": 2560, "height": 1440,
              "refreshRate": 59.951, "x": 2560, "y": -560, "scale": 1.0, "transform": 1,
              "availableModes": ["2560x1440@59.95Hz", "2560x1440@165.00Hz"] },
            { "name": "HDMI-A-1", "description": crate::i18n::tr("Vidéoprojecteur"), "width": 1920, "height": 1080,
              "refreshRate": 60.0, "x": 0, "y": 0, "scale": 1.0, "transform": 0, "disabled": true }
        ]));
        app.settings.primary = Some("HDMI-A-2".into());
        app.settings.workspaces = Some(7);
        app.settings
            .set_assign(config::AppMatch::App("firefox".into()), "2", false);
        app.settings
            .set_assign(config::AppMatch::Class("^(discord)$".into()), "6", true);
        app.active_window = Some((
            "hyprdesk".into(),
            crate::i18n::tr("Un espace à vous").into(),
        ));
        app.bar_surfaces.insert(
            bar,
            bar::BarSurface {
                monitor: crate::i18n::tr("APERÇU").into(),
                position: BarPosition::Top,
                height: 44,
            },
        );
        let mut cfg = Config::default();
        cfg.general.running_apps = RunningApps::All;
        cfg.catalog = widgets::builtin_types();
        for (index, (label, id)) in [
            ("Navigateur", "firefox"),
            ("Fichiers", "org.gnome.Nautilus"),
            ("Terminal", "kitty"),
            (crate::i18n::tr("Éditeur"), "code"),
        ]
        .iter()
        .enumerate()
        {
            cfg.icons.push(IconDef {
                origin: Origin::User,
                sel: Selector::default(),
                id: id.to_string(),
                label: Some(label.to_string()),
                kind: config::IconKind::App(id.to_string()),
                icon: None,
                pos: Some((0, index as u16)),
            });
        }
        let mut clock = widget(
            "clock",
            WidgetKind::Clock {
                format: "%H:%M".into(),
            },
            Anchor::TopRight,
        );
        clock.size = 80.0;
        clock.card = false;
        let mut calendar = widget("calendar", WidgetKind::Calendar, Anchor::Right);
        calendar.y = -20.0;
        let system = widget(
            "system",
            WidgetKind::System {
                show: vec![config::SysItem::Cpu, config::SysItem::Ram],
            },
            Anchor::BottomRight,
        );
        cfg.widgets = vec![clock, calendar, system];
        let mut barclock = widget(
            "barclock",
            WidgetKind::Clock {
                format: "%a %-d %b  ·  %H:%M".into(),
            },
            Anchor::Top,
        );
        barclock.in_bar = true;
        barclock.type_id = "clock".into();
        cfg.widgets.push(barclock);
        cfg.bar = Some(BarDef {
            monitors: None,
            position: BarPosition::Top,
            height: 44,
            zones: [
                vec![BarItem::Workspaces, BarItem::Window],
                vec![BarItem::Widget("barclock".into())],
                vec![BarItem::Brightness, BarItem::Volume, BarItem::Desktop],
            ],
            portrait: None,
            compact: Default::default(),
            colors: Default::default(),
        });
        app.icons = cfg
            .icons
            .iter()
            .map(|d| launch::resolve(d, None, 48, "kitty"))
            .collect();
        let paths: Vec<_> = app.icons.iter().map(|r| r.image.clone()).collect();
        app.icon_imgs = paths.iter().map(|p| app.handle_for(p.as_deref())).collect();
        app.ui_imgs = UI_ICONS.iter().map(|n| app.builtin_icon(n)).collect();
        for (index, icon) in cfg.icons.iter().enumerate() {
            app.clients.push(Client {
                address: format!("preview-{index}"),
                class: icon.id.clone(),
                title: icon.label.clone().unwrap_or_default(),
                workspace: "1".into(),
            });
            app.class_imgs.insert(icon.id.clone(), app.icon_imgs[index]);
        }
        for name in bar::STATIC_ICONS {
            let icon = app.builtin_icon(name);
            app.named_icons.insert(name.to_string(), icon);
        }
        app.values.insert(
            "system".into(),
            widgets::Value::Sys(vec![
                widgets::SysLine {
                    label: crate::i18n::tr("CPU").into(),
                    value: "12 %".into(),
                    ratio: Some(0.12),
                },
                widgets::SysLine {
                    label: crate::i18n::tr("RAM").into(),
                    value: "4,2 / 16 Gio".into(),
                    ratio: Some(0.26),
                },
            ]),
        );
        app.workspaces = (1..=4)
            .map(|id| hypr::Workspace {
                id,
                name: id.to_string(),
                monitor: crate::i18n::tr("APERÇU").into(),
                windows: if id == 1 { 2 } else { 0 },
            })
            .collect();
        let cfg = app.settings.apply_to(&cfg);
        app.theme = make_theme(&cfg);
        app.cfg = Arc::new(cfg);
        // Permet de vérifier une surface sans recapturer tous les réglages.
        let mut scenes: std::collections::VecDeque<usize> =
            std::env::var("HYPRDESK_PREVIEW_SCENES")
                .ok()
                .map(|s| {
                    s.split(',')
                        .filter_map(|name| SCENES.iter().position(|scene| *scene == name.trim()))
                        .collect()
                })
                .unwrap_or_default();
        if scenes.is_empty() {
            scenes.extend(0..SCENES.len());
        }
        let scene = scenes.pop_front().unwrap_or(0);
        let mut preview = Self {
            app,
            desktop,
            bar,
            output,
            scene,
            remaining_scenes: scenes,
            size,
        };
        if scene != 0 {
            preview.set_scene();
        }
        let task = preview.next_capture();
        (preview, task)
    }

    fn next_capture(&self) -> Task<PreviewMsg> {
        if self.output.is_some() {
            Task::perform(tokio::time::sleep(Duration::from_millis(500)), |_| {
                PreviewMsg::Capture
            })
        } else {
            Task::none()
        }
    }

    fn set_scene(&mut self) {
        self.app.panel = None;
        self.app.chat = None;
        self.app.popover = None;
        self.app.launcher_popup = None;
        self.app.launcher_runtime.docks.clear();
        self.app.launcher_runtime.launches.clear();
        if self.scene >= 16 {
            use crate::config::{LauncherGroup, LauncherPosition};
            let cfg = Arc::make_mut(&mut self.app.cfg);
            cfg.general.launcher.position = match self.scene {
                18 => LauncherPosition::Top,
                19 => LauncherPosition::Left,
                20 => LauncherPosition::Right,
                _ => LauncherPosition::Bottom,
            };
            cfg.general.launcher.pinned = vec!["kitty".into(), "firefox".into()];
            cfg.general.launcher_group = LauncherGroup::App;
            if self.scene == 27 {
                cfg.general.launcher.pinned.push("preview-launch".into());
            }
            if matches!(self.scene, 28 | 29) {
                cfg.general.background = if self.scene == 28 {
                    config::Rgba(255, 255, 255, 255)
                } else {
                    config::Rgba(0, 0, 0, 255)
                };
                let mut effect = cfg.general.theme.effect(config::Place::Launcher);
                effect.opacity = 0.25;
                cfg.general
                    .theme
                    .effects
                    .insert(config::Place::Launcher, effect);
            }
            let thickness = plugins::launcher::thickness(cfg.general.launcher.size) as f32;
            self.app.monitors[0].reserved = match cfg.general.launcher.position {
                LauncherPosition::Bottom => (0.0, 44.0, 0.0, thickness),
                LauncherPosition::Top => (0.0, 44.0 + thickness, 0.0, 0.0),
                LauncherPosition::Left => (thickness, 44.0, 0.0, 0.0),
                LauncherPosition::Right => (0.0, 44.0, thickness, 0.0),
            };
            self.app.active_window = Some(("firefox".into(), "Navigateur".into()));
            if !self
                .app
                .clients
                .iter()
                .any(|c| c.address == "preview-browser-2")
            {
                self.app.clients.push(Client {
                    address: "preview-browser-2".into(),
                    class: "firefox".into(),
                    title: crate::i18n::tr("Mindlog · Notes du jour").into(),
                    workspace: "2".into(),
                });
            }
            self.app.hover_fades.clear();
            if self.scene == 17 {
                let _ = self
                    .app
                    .launcher_pointer(crate::i18n::tr("APERÇU"), Some(1.15));
            }
            if matches!(self.scene, 21 | 26 | 30) {
                self.app.launcher_popup = Some(plugins::launcher::Popup {
                    id: Id::unique(),
                    monitor: crate::i18n::tr("APERÇU").into(),
                    key: "dock-pin:firefox".into(),
                    content: plugins::launcher::PopupContent::Windows("firefox".into()),
                    fade: self.app.launcher_transition(
                        if self.scene == 26 { 0.5 } else { 1.0 },
                        if self.scene == 26 { 0.5 } else { 1.0 },
                        0,
                    ),
                    max_height: self.size.height - 160.0,
                    placement: plugins::launcher::Placement {
                        bounds: iced::Rectangle::new(
                            Point::new((self.size.width - 416.0) / 2.0, self.size.height - 204.0),
                            Size::new(416.0, 102.0),
                        ),
                        pivot: Point::new(0.5, 1.0),
                    },
                });
            }
            if self.scene == 25 {
                self.app.launcher_runtime.docks.insert(
                    crate::i18n::tr("APERÇU").into(),
                    self.app.launcher_transition(0.5, 0.5, 0),
                );
            }
            if self.scene == 27 {
                let _ = self.app.launcher_launch_started("preview-launch".into());
            }
            if self.scene == 30 {
                for index in 0..40 {
                    self.app.clients.push(Client {
                        address: format!("preview-many-{index}"),
                        class: "firefox".into(),
                        title: format!("Document {} — Notes de travail", index + 1),
                        workspace: "2".into(),
                    });
                }
            }
        }
        // Scènes des modes du bureau (captures) : fenêtres visibles, bureau masqué.
        // Les champs sont posés directement : les transitions réelles font de l'IPC.
        match SCENES[self.scene] {
            "mode-launcher" => self.app.with_windows = true,
            "mode-hidden" => {
                self.app.visible = false;
                self.app.with_windows = false;
                self.app.raised.remove(&self.desktop);
            }
            _ => {}
        }
        let tab = match self.scene {
            1 => Some("theme"),
            2 => Some("icons"),
            3 => Some("wallpaper"),
            4 => Some("widgets"),
            5 => Some("bar"),
            6 => Some("effects"),
            9 => Some("workspaces"),
            10 => Some("screens"),
            11 => Some("launcher"),
            12 => Some("apps"),
            13 => Some("assistant"),
            14 => Some("plugins"),
            15 => Some("share"),
            23 => Some("theme"),
            24 => Some("desktop"),
            31 => Some("layouts"),
            32 => Some("wallpaper"),
            33 => Some("lock"),
            _ => None,
        };
        // Scènes repérées par leur nom (et non leur rang).
        let tab = tab.or(match SCENES[self.scene] {
            "animations" | "animations-effets" => Some("animations"),
            "son" => Some("sound"),
            "energie" => Some("energy"),
            "date-heure" => Some("datetime"),
            "reseau" => Some("network"),
            "bluetooth" => Some("bt"),
            "plugin-config" => Some("plugins"),
            _ => None,
        });
        // Onglet de chaque paquet : le premier, sauf « typography » (Thème → Texte)
        // et HYPRDESK_PREVIEW_SUBTAB (ex. `bar=modules,theme=effets`) pour les essais.
        self.app.subtabs.clear();
        // Configuration avancée par défaut (toutes les vues) ; HYPRDESK_PREVIEW_BASIC : l'essentiel.
        Arc::make_mut(&mut self.app.cfg).general.advanced_settings =
            std::env::var_os("HYPRDESK_PREVIEW_BASIC").is_none();
        // Visite guidée : proposée seulement dans sa scène.
        Arc::make_mut(&mut self.app.cfg).general.tour_seen = SCENES[self.scene] != "tour";
        if self.scene == 1 {
            self.app.subtabs.insert("theme", "texte");
        }
        // Verrouillage → Veille, hyprdesk gérant la veille (délais par défaut).
        // Agent polkit : la carte de saisie, une tentative refusée.
        if SCENES[self.scene] == "polkit" {
            self.app.polkit = Some(super::polkit::Dialog {
                win: iced::window::Id::unique(),
                req: crate::polkit::Request {
                    action: "org.freedesktop.udisks2.filesystem-mount".into(),
                    message: crate::i18n::tr("Authentification requise pour monter un disque")
                        .into(),
                    user: "jacques".into(),
                    cookie: String::new(),
                    attempt: 2,
                },
                text: "Mot de passe :".into(),
                echo: false,
                input: "secret".into(),
                message: Some((
                    crate::i18n::tr("Mot de passe refusé, réessayez.").into(),
                    true,
                )),
                reply: Some(super::polkit::Responder::detached()),
            });
        }
        // Les bulles n'appartiennent qu'à leur scène.
        if SCENES[self.scene] != "notifications" {
            self.app.toasts.list.clear();
        }
        // Bulles de notifications : une ordinaire, une critique avec actions.
        if SCENES[self.scene] == "notifications" {
            let mut crit = std::collections::HashMap::new();
            crit.insert("urgency".to_string(), zbus::zvariant::OwnedValue::from(2u8));
            let acts: Vec<String> = ["default", "Ouvrir", "later", "Plus tard"]
                .map(String::from)
                .to_vec();
            let _ = self
                .app
                .want_icon("preferences-system-notifications-symbolic");
            self.app.toasts.list = vec![
                super::toasts::Toast {
                    n: crate::notifications::build(
                        2,
                        "Batterie",
                        "",
                        "Batterie faible : 9 %",
                        "Branchez l'ordinateur pour continuer à travailler.",
                        &acts,
                        &crit,
                        -1,
                    ),
                    since: std::time::Instant::now(),
                },
                super::toasts::Toast {
                    n: crate::notifications::build(
                        1,
                        "Firefox",
                        "",
                        "Téléchargement terminé",
                        "rapport-annuel.pdf (2,4 Mo)",
                        &[],
                        &Default::default(),
                        -1,
                    ),
                    since: std::time::Instant::now(),
                },
            ];
        }
        // Fenêtre de détail d'un problème (mise à jour échouée, sans assistant).
        if SCENES[self.scene] == "probleme" {
            self.app.problem = Some(super::problem::Problem {
                win: iced::window::Id::unique(),
                text: crate::updates::report(&crate::updates::Step {
                    source: "aur",
                    command: "yay -Syu --noconfirm --sudo pkexec".into(),
                    ok: false,
                    code: Some(1),
                    tail: ":: Processing package changes...\nerror: failed to commit transaction (conflicting files)\npython-foo: /usr/lib/python3.13/site-packages/foo/__init__.py exists in filesystem\nErrors occurred, no packages were upgraded.".into(),
                }),
                copied: false,
            });
        } else {
            self.app.problem = None;
        }
        // Lanceur avec recherche : « fi » parmi quelques applications.
        if SCENES[self.scene] == "recherche" {
            let app = |id: &str, name: &str, icon: &str| crate::launch::AppInfo {
                id: id.into(),
                name: name.into(),
                icon: Some(icon.into()),
                wm_class: None,
                mime_types: Vec::new(),
                categories: Vec::new(),
                available: true,
            };
            self.app.apps = Arc::new(vec![
                app("firefox", "Firefox", "firefox"),
                app(
                    "org.gnome.Nautilus",
                    "Fichiers",
                    "system-file-manager-symbolic",
                ),
                app("kitty", "Terminal", "utilities-terminal"),
                app(
                    "libreoffice-writer",
                    "LibreOffice Writer",
                    "libreoffice-writer",
                ),
                app(
                    "org.gnome.Settings",
                    "Paramètres du profil",
                    "preferences-system-symbolic",
                ),
                app("filezilla", "FileZilla", "filezilla"),
            ]);
            self.app.search = None;
            let _ = self.app.toggle_search();
            let _ = self
                .app
                .search_update(super::search::SMsg::Query("fi".into()));
        } else if SCENES[self.scene] == "presse-papiers" {
            // Recherche dans l'historique du presse-papiers : « :clip ».
            self.app.search = None;
            let _ = self.app.toggle_search();
            if let Some(s) = self.app.search.as_mut() {
                s.query = ":clip".into();
                s.clips = Some(Vec::new());
            }
            let entry = |id: u32, preview: &str| crate::clipboard::Entry {
                line: format!("{id}\t{preview}"),
                preview: preview.into(),
            };
            let _ = self.app.search_update(super::search::SMsg::Clips(vec![
                entry(42, "https://plane.le-lab.net/mlg/issues/13"),
                entry(41, "cargo test --release -- --nocapture"),
                entry(40, "Rendez-vous jeudi 14 h, salle B"),
                entry(39, "[[ binary data 48 KiB png 640x480 ]]"),
                entry(38, "fn main() { println!(\"bonjour\"); }"),
            ]));
        } else {
            self.app.search = None;
        }
        // La carte polkit n'appartient qu'à sa scène.
        if SCENES[self.scene] != "polkit" {
            self.app.polkit = None;
        }
        // L'affichage à l'écran n'appartient qu'à sa scène.
        if SCENES[self.scene] != "osd" {
            self.app.osd = None;
        }
        if SCENES[self.scene] == "animations-effets" {
            self.app.subtabs.insert("animations", "effets");
        }
        // Animations : quelques éléments personnalisés, pour voir les contrôles.
        if SCENES[self.scene].starts_with("animations") {
            let fx = &mut Arc::make_mut(&mut self.app.cfg).general.fx;
            fx.anims.insert(
                "windowsIn".into(),
                crate::fx::Anim {
                    enabled: true,
                    speed: 4.0,
                    curve: "spring".into(),
                    style: Some("popin".into()),
                },
            );
            fx.anims.insert(
                "workspaces".into(),
                crate::fx::Anim {
                    enabled: true,
                    speed: 6.0,
                    curve: "ease-out".into(),
                    style: Some("slidefade".into()),
                },
            );
            fx.options.insert(
                "decoration.blur.variant".into(),
                crate::tiling::OptValue::Num(9.0),
            );
            fx.options.insert(
                "decoration.rounding".into(),
                crate::tiling::OptValue::Num(12.0),
            );
        }
        if SCENES[self.scene] == "veille" {
            self.app.subtabs.insert("lock", "veille");
            Arc::make_mut(&mut self.app.cfg).general.idle.manage = true;
        }
        if let Ok(spec) = std::env::var("HYPRDESK_PREVIEW_SUBTAB") {
            for pair in spec.split(',') {
                if let Some((pkg, sub)) = pair.split_once('=')
                    && let Some(info) = plugins::find(pkg.trim())
                    && let Some(sub) = plugins::find_tab(info.id, sub)
                {
                    self.app.subtabs.insert(info.id, sub);
                }
            }
        }
        if let Some(tab) = tab {
            self.app.panel = Some(Panel::new(self.desktop));
            let _ = self.app.panel_update(PanelMsg::Tab(tab));
            let _ =
                self.app
                    .panel_update(PanelMsg::Plugin(PluginMsg::Desktop(desktop::Msg::Fonts(
                        launch::font_families(),
                    ))));
            let _ = self
                .app
                .panel_update(PanelMsg::Plugin(PluginMsg::Icons(icons::Msg::Themes(
                    launch::icon_themes(),
                ))));
        }
        // Plugins → Mises à jour → Configurer : options et commandes d'exemple.
        if SCENES[self.scene] == "plugin-config" {
            use crate::tiling::OptValue;
            Arc::make_mut(&mut self.app.cfg)
                .catalog
                .push(crate::config::WidgetType {
                    id: "majs".into(),
                    name: crate::i18n::tr("Mises à jour").into(),
                    description: crate::i18n::tr("Paquets à mettre à jour ; mise à jour en un clic. Tout à jour : une coche discrète (option masquer_a_jour pour la cacher).").into(),
                    source: crate::config::WidgetSource::Bundled,
                    anchor: crate::config::Anchor::TopRight,
                    toggleable: true,
                    error: None,
                    options: vec![
                        ("masquer_a_jour".into(), OptValue::Bool(false)),
                        ("texte".into(), OptValue::Bool(true)),
                    ],
                });
        }
        if SCENES[self.scene] == "plugin-config"
            && let Some(p) = self.app.panel.as_mut()
        {
            p.plugins.tab = super::plugins::plugins::Tab::Widgets;
            p.plugins.configuring = Some("majs".into());
            p.plugins.update_sources = vec![
                (
                    "pacman".into(),
                    "Dépôts Arch".into(),
                    "checkupdates".into(),
                    String::new(),
                ),
                (
                    "aur".into(),
                    "AUR".into(),
                    "yay -Qua".into(),
                    "yay -Syu --noconfirm --sudo pkexec --answerclean None --answerdiff None"
                        .into(),
                ),
            ];
        }
        // Son : appareils et applications d'exemple (l'aperçu ne lit pas PipeWire).
        if SCENES[self.scene] == "son"
            && let Some(p) = self.app.panel.as_mut()
        {
            let n = |id: u32, name: &str, default: bool, volume: f32, muted: bool| {
                super::plugins::sound::Node {
                    id,
                    name: name.into(),
                    default,
                    volume,
                    muted,
                }
            };
            p.sound = super::plugins::sound::State {
                outputs: vec![
                    n(46, "Audio intégré (IEC958)", false, 1.0, false),
                    n(55, "Arctis Nova Pro Wireless", true, 0.45, false),
                ],
                inputs: vec![n(56, "Arctis Nova Pro Wireless Mono", true, 0.9, false)],
                streams: vec![
                    n(78, "Firefox", false, 0.8, false),
                    n(91, "Spotify", false, 0.6, true),
                ],
                loaded: true,
            };
        }
        // Énergie et Date et heure : états d'exemple (l'aperçu ne lit pas le système).
        if SCENES[self.scene] == "energie"
            && let Some(p) = self.app.panel.as_mut()
        {
            p.energy = super::plugins::energy::State {
                profiles: vec![
                    ("performance".into(), false),
                    ("balanced".into(), true),
                    ("power-saver".into(), false),
                ],
                batteries: vec![("BAT0".into(), 76, "Discharging".into())],
                loaded: true,
            };
            Arc::make_mut(&mut self.app.cfg).general.idle.manage = true;
        }
        if SCENES[self.scene] == "date-heure"
            && let Some(p) = self.app.panel.as_mut()
        {
            p.datetime = super::plugins::datetime::State {
                timezone: "Europe/Paris".into(),
                ntp: true,
                can_ntp: true,
                synced: true,
                local_rtc: false,
                zones: vec!["Europe/London".into(), "Europe/Paris".into()],
                loaded: true,
                error: None,
            };
        }
        // Réseau et Bluetooth : états d'exemple.
        if SCENES[self.scene] == "reseau"
            && let Some(p) = self.app.panel.as_mut()
        {
            use super::plugins::network::{Conn, Net, State};
            let c = |name: &str, kind: &str, active: bool| Conn {
                name: name.into(),
                kind: kind.into(),
                active,
            };
            let n = |ssid: &str, signal: u8, secure: bool, in_use: bool| Net {
                ssid: ssid.into(),
                signal,
                secure,
                in_use,
            };
            p.network = State {
                wifi: true,
                airplane: false,
                conns: vec![
                    c("Maison", "802-11-wireless", true),
                    c("Bureau VPN", "vpn", false),
                    c("Filaire", "802-3-ethernet", false),
                ],
                nets: vec![
                    n("Maison", 82, true, true),
                    n("Gare-Libre", 61, false, false),
                    n("Voisin 5G", 44, true, false),
                ],
                loaded: true,
                confirm: Some("Filaire".into()),
                error: None,
            };
        }
        if SCENES[self.scene] == "bluetooth"
            && let Some(p) = self.app.panel.as_mut()
        {
            use super::plugins::bluetooth::{Device, State};
            let d = |mac: &str, name: &str, connected: bool, battery: Option<u8>| Device {
                mac: mac.into(),
                name: name.into(),
                connected,
                battery,
            };
            p.bluetooth = State {
                powered: Some(true),
                paired: vec![
                    d("AA:00:00:00:00:01", "MX Vertical", true, Some(85)),
                    d("AA:00:00:00:00:02", "Air75", false, None),
                ],
                nearby: vec![d("AA:00:00:00:00:03", "JBL Flip 5", false, None)],
                loaded: true,
                ..Default::default()
            };
        }
        // Menu contextuel d'une icône (fichier du Bureau), corbeille à confirmer.
        if SCENES[self.scene] == "menu-icone" {
            self.app.icon_menu = Some(super::context::IconMenu {
                win: iced::window::Id::unique(),
                catcher: iced::window::Id::unique(),
                icon: 1,
                path: Some("/home/u/Bureau/Rapport.pdf".into()),
                rename: None,
                confirm: true,
                error: None,
            });
        } else {
            self.app.icon_menu = None;
        }
        // Sélecteur de fenêtres (Alt+Tab), la deuxième choisie.
        if SCENES[self.scene] == "fenetres" {
            let w = |a: &str, class: &str, title: &str, ws: &str| crate::switcher::Win {
                address: a.into(),
                class: class.into(),
                title: title.into(),
                workspace: ws.into(),
                focus: 0,
            };
            self.app.switcher = Some(super::switcher::Switcher {
                win: iced::window::Id::unique(),
                wins: vec![
                    w("0x1", "kitty", "~/projets/hyprdesk — cargo test", "1"),
                    w("0x2", "firefox", "Plane — MLG-22 Fenêtres", "2"),
                    w("0x3", "org.gnome.Nautilus", "Téléchargements", "2"),
                    w(
                        "0x4",
                        "libreoffice-writer",
                        "Rapport.odt — LibreOffice Writer",
                        "3",
                    ),
                ],
                selected: 1,
                loading: false,
                back: false,
            });
        } else {
            self.app.switcher = None;
        }
        if SCENES[self.scene] == "tour"
            && let Some(p) = self.app.panel.as_mut()
        {
            p.tour = Some(2);
        }
        // Layouts : raccourcis d'un hyprland.lua d'exemple (l'aperçu ne lit aucun fichier).
        if SCENES[self.scene] == "layouts"
            && let Some(p) = self.app.panel.as_mut()
        {
            p.layouts.binds = crate::tiling::parse_binds(
                "local mainMod = \"SUPER\"\n\
                 hl.bind(mainMod .. \" + h\", hl.dsp.focus({ direction = \"left\" }))\n\
                 hl.bind(mainMod .. \" + l\", hl.dsp.focus({ direction = \"right\" }))\n\
                 hl.bind(mainMod .. \" + SHIFT + left\", hl.dsp.window.move({ direction = \"left\" }))\n\
                 hl.bind(mainMod .. \" + SHIFT + right\", hl.dsp.window.move({ direction = \"right\" }))\n\
                 hl.bind(mainMod .. \" + F\", hl.dsp.window.fullscreen())\n\
                 hl.bind(mainMod .. \" + S\", hl.dsp.layout(\"togglesplit\"))\n\
                 hl.bind(\"SUPER + ALT + F\", hl.dsp.exec_cmd(\"firefox\"))\n",
            );
        }
        if SCENES[self.scene] == "chat" {
            self.app.chat_demo(self.desktop);
        }
        if self.scene == 8 {
            let _ = self
                .app
                .open_popover(self.bar, "calendar".into(), popover::Kind::Calendar);
        }
        if self.scene == 7 {
            let _ = self
                .app
                .open_popover(self.bar, "volume".into(), popover::Kind::Volume);
        }
        // Module Luminosité : un portable (écran, clavier) et un écran externe.
        if SCENES[self.scene] == "luminosite" {
            use crate::brightness::{Device, Display};
            self.app.light = super::bar::Light {
                screen: Some(Device {
                    subsystem: "backlight",
                    name: "intel_backlight".into(),
                    current: 720,
                    max: 1200,
                }),
                kbd: Some(Device {
                    subsystem: "leds",
                    name: "kbd_backlight".into(),
                    current: 1,
                    max: 3,
                }),
                displays: vec![Display {
                    connector: "DP-1".into(),
                    bus: 12,
                    current: 40,
                    max: 100,
                }],
            };
            let _ = self
                .app
                .open_popover(self.bar, "brightness".into(), popover::Kind::Brightness);
        }
        // Touche de volume : la carte d'affichage à l'écran.
        if SCENES[self.scene] == "osd" {
            let _ = self.app.show_osd(crate::osd::Kind::Volume, 0.62, false);
        }
    }

    fn update(&mut self, msg: PreviewMsg) -> Task<PreviewMsg> {
        match msg {
            PreviewMsg::Inspect => iced::window::oldest()
                .and_then(iced::window::screenshot)
                .map(PreviewMsg::Inspected),
            PreviewMsg::Inspected(shot) => {
                if let Err(e) = ::image::save_buffer(
                    "/tmp/hyprdesk-preview.png",
                    &shot.rgba,
                    shot.size.width,
                    shot.size.height,
                    ::image::ColorType::Rgba8,
                ) {
                    eprintln!("capture : {e}");
                }
                Task::none()
            }
            PreviewMsg::Capture => iced::window::oldest()
                .and_then(iced::window::screenshot)
                .map(PreviewMsg::Captured),
            PreviewMsg::Captured(shot) => {
                if let Some(dir) = &self.output {
                    let result = std::fs::create_dir_all(dir).and_then(|()| {
                        ::image::save_buffer(
                            dir.join(format!("{}.png", SCENES[self.scene])),
                            &shot.rgba,
                            shot.size.width,
                            shot.size.height,
                            ::image::ColorType::Rgba8,
                        )
                        .map_err(std::io::Error::other)
                    });
                    if let Err(e) = result {
                        eprintln!("capture : {e}");
                        return iced::exit();
                    }
                }
                let Some(next) = self.remaining_scenes.pop_front() else {
                    return iced::exit();
                };
                self.scene = next;
                self.set_scene();
                self.next_capture()
            }
            PreviewMsg::Ui(message) => self.update_ui(*message),
        }
    }

    fn update_ui(&mut self, message: Msg) -> Task<PreviewMsg> {
        match message {
            Msg::OpenPanel => {
                self.scene = 1;
                self.set_scene();
                Task::none()
            }
            Msg::Panel(PanelMsg::Tab(t)) => {
                let _ = self.app.panel_update(PanelMsg::Tab(t));
                Task::none()
            }
            Msg::Panel(PanelMsg::Close) => {
                self.app.panel = None;
                Task::none()
            }
            Msg::Panel(PanelMsg::Plugin(PluginMsg::Desktop(desktop::Msg::Font(f)))) => {
                self.app.font = Font::with_name(intern_font(&f));
                self.app.ui_family = intern_font(&f);
                Arc::make_mut(&mut self.app.cfg).general.font = Some(f);
                Task::none()
            }
            Msg::Panel(PanelMsg::Plugin(PluginMsg::Desktop(desktop::Msg::FontSize(v)))) => {
                Arc::make_mut(&mut self.app.cfg).general.font_size = v;
                Task::none()
            }
            Msg::OpenPopover(_, _, popover::Request::Volume) => {
                self.scene = 7;
                self.set_scene();
                Task::none()
            }
            Msg::IconHover(h) => self
                .app
                .update_inner(Msg::IconHover(h))
                .map(|m| PreviewMsg::Ui(Box::new(m))),
            Msg::Hover(k, on) => self
                .app
                .update_inner(Msg::Hover(k, on))
                .map(|m| PreviewMsg::Ui(Box::new(m))),
            Msg::LauncherPointer(monitor, position) => {
                let _ = self.app.launcher_pointer(&monitor, position);
                Task::none()
            }
            Msg::LauncherAnchor(monitor, key, bounds) => {
                let m = &self.app.monitors[0];
                let l = &self.app.cfg.general.launcher;
                let origin = plugins::launcher::screen_anchor(
                    m,
                    l.position,
                    l.size,
                    iced::Rectangle::default(),
                )
                .position();
                self.app
                    .launcher_anchor(monitor, key, bounds - iced::Vector::new(origin.x, origin.y))
                    .map(|m| PreviewMsg::Ui(Box::new(m)))
            }
            m @ (Msg::LauncherHover(_)
            | Msg::LauncherPopupOpen(_)
            | Msg::LauncherPopupClose(_)
            | Msg::LauncherPopupHover(_)
            | Msg::LauncherPopupMeasured(..)
            | Msg::LauncherShown(_)
            | Msg::LauncherLaunchExpired(..)) => self
                .app
                .update_inner(m)
                .map(|m| PreviewMsg::Ui(Box::new(m))),
            Msg::LauncherLaunch(id) => self
                .app
                .launcher_launch_started(id)
                .map(|m| PreviewMsg::Ui(Box::new(m))),
            Msg::PopoverMeasured(win, height) => self
                .app
                .fit_popover(win, height)
                .map(|m| PreviewMsg::Ui(Box::new(m))),
            Msg::Frame => self
                .app
                .update_inner(Msg::Frame)
                .map(|m| PreviewMsg::Ui(Box::new(m))),
            _ => Task::none(),
        }
    }

    fn view(&self) -> Element<'_, PreviewMsg> {
        let mut layers: Vec<Element<'_, Msg>> = vec![
            self.app.view(self.desktop),
            container(self.app.view_bar(self.bar))
                .height(44)
                .width(Length::Fill)
                .into(),
        ];
        // Le lanceur a sa propre surface : par-dessus le bureau, sous la barre.
        // Bureau masqué : le dock n'existe plus.
        if let Some(m) = self.app.monitors.first()
            && SCENES[self.scene] != "mode-hidden"
        {
            // Le compositeur tient compte de la barre avant de placer le dock.
            layers.insert(
                1,
                container(self.app.view_launcher(m, 1.0))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(Padding {
                        top: 44.0,
                        ..Padding::ZERO
                    })
                    .into(),
            );
        }
        // Les scènes de modes posent deux fenêtres fictives entre le bureau et le dock.
        if matches!(SCENES[self.scene], "mode-launcher" | "mode-hidden") {
            layers.insert(1, self.demo_windows());
        }
        // Les tiroirs ont leur propre surface : à droite, sous la barre.
        if self.app.chat.is_some() {
            layers.push(
                container(
                    container(self.app.view_chat())
                        .width(self.app.panel_width())
                        .height(Length::Fill),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::Alignment::End)
                .padding(iced::Padding {
                    top: 44.0,
                    ..iced::Padding::ZERO
                })
                .into(),
            );
        }
        if self.app.panel.is_some() {
            layers.push(
                container(
                    container(self.app.view_panel())
                        .width(self.app.panel_width())
                        .height(Length::Fill),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(iced::Alignment::End)
                .padding(iced::Padding {
                    top: 44.0,
                    ..iced::Padding::ZERO
                })
                .into(),
            );
        }
        if let Some(p) = &self.app.popover {
            layers.push(
                pin(container(self.app.view_popover())
                    .width(p.size.width)
                    .height(p.size.height))
                .x(self.size.width - p.size.width - 18.0)
                .y(44.0)
                .into(),
            );
        }
        // Écran de connexion (greetd), par-dessus tout : application à part.
        if SCENES[self.scene] == "connexion" {
            layers.push(crate::greeter_ui::preview_view().map(|_| Msg::Noop));
        }
        if self.app.switcher.is_some() {
            layers.push(
                container(self.app.view_switcher())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_y(Length::Fill)
                    .into(),
            );
        }
        if self.app.icon_menu.is_some() {
            layers.push(
                container(self.app.view_icon_menu())
                    .padding(iced::Padding {
                        top: 150.0,
                        left: 120.0,
                        ..Default::default()
                    })
                    .into(),
            );
        }
        if self.app.search.is_some() {
            layers.push(
                container(self.app.view_search())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into(),
            );
        }
        if self.app.problem.is_some() {
            layers.push(
                container(self.app.view_problem())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into(),
            );
        }
        if !self.app.toasts.list.is_empty() {
            layers.push(
                container(self.app.view_toasts())
                    .width(Length::Fill)
                    .align_x(iced::Alignment::End)
                    .padding(Padding {
                        top: 44.0,
                        ..Padding::ZERO
                    })
                    .into(),
            );
        }
        if self.app.polkit.is_some() {
            layers.push(
                container(self.app.view_polkit())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into(),
            );
        }
        if self.app.osd.is_some() {
            layers.push(
                container(self.app.view_osd())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(iced::Alignment::Center)
                    .align_y(iced::Alignment::End)
                    .padding(Padding {
                        bottom: 140.0,
                        ..Padding::ZERO
                    })
                    .into(),
            );
        }
        if let Some(p) = &self.app.launcher_popup {
            layers.push(
                pin(container(self.app.view_launcher_popup())
                    .width(p.placement.bounds.width)
                    .height(p.placement.bounds.height))
                .x(p.placement.bounds.x)
                .y(p.placement.bounds.y)
                .into(),
            );
        }
        let view: Element<'_, Msg> = Stack::with_children(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
        view.map(|m| PreviewMsg::Ui(Box::new(m)))
    }

    /// Deux fenêtres fictives en mosaïque : illustrer les modes du bureau
    /// (lanceur seul, bureau masqué) dans les captures de l'aperçu.
    /// L'aperçu étant une seule surface, les vraies fenêtres ne peuvent pas
    /// y apparaître ; celles-ci occupent la place qu'Hyprland leur laisse.
    fn demo_windows(&self) -> Element<'_, Msg> {
        let g = &self.app.cfg.general;
        let fg = g.theme.fg;
        let title_bg = color(fg, 0.06);
        let placeholder = color(fg, 0.08);
        let outline = color(fg, 0.14);
        let card_bg = color(g.theme.bg, 1.0);
        // La mosaïque laisse au dock la place que le compositeur lui réserve.
        let bottom = plugins::launcher::thickness(g.launcher.size) as f32 + 16.0;
        let card = move |title: &'static str| -> Element<'_, Msg> {
            let mut body = column([]).spacing(10).width(Length::Fill);
            for portion in [92u16, 68, 45] {
                body = body.push(
                    container(
                        Space::new()
                            .width(Length::FillPortion(portion))
                            .height(Length::Fixed(10.0)),
                    )
                    .style(move |_| container::Style {
                        background: Some(Background::Color(placeholder)),
                        border: Border {
                            color: Color::TRANSPARENT,
                            width: 0.0,
                            radius: 5.0.into(),
                        },
                        ..Default::default()
                    }),
                );
            }
            container(column(vec![
                container(text(title).size(13))
                    .padding(Padding {
                        top: 9.0,
                        left: 12.0,
                        right: 12.0,
                        bottom: 9.0,
                    })
                    .style(move |_| container::Style {
                        background: Some(Background::Color(title_bg)),
                        ..Default::default()
                    })
                    .into(),
                body.padding(16).into(),
            ]))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(Background::Color(card_bg)),
                border: Border {
                    color: outline,
                    width: 1.0,
                    radius: 12.0.into(),
                },
                ..Default::default()
            })
            .into()
        };
        container(iced::widget::row(vec![
            card(crate::i18n::tr("Mindlog · Notes du jour")),
            card("hyprdesk.lua — kitty"),
        ]))
        .padding(Padding {
            top: 44.0 + 12.0,
            left: 12.0,
            right: 12.0,
            bottom,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}
