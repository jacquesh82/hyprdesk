//! Aperçu interactif isolé : mêmes vues, données locales, aucun IPC ni lancement.
use super::panel::SliderEdit;
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
    BarGesture,
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

const SCENES: [&str; 170] = [
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
    "reglages",
    "son",
    "energie",
    "date-heure",
    "reseau",
    "bluetooth",
    "menu-icone",
    "proprietes",
    "menu-dock",
    "menu-dock-long",
    "menu-edition",
    "fenetres",
    "connexion",
    "luminosite",
    "plugin-config",
    "applications",
    "lanceur-central",
    "clavier-souris",
    "ecran-connexion",
    "mots-de-passe",
    "coffre-creation",
    "coffre-sauvegarde",
    "coffre-pret",
    "launcher-keyboard",
    "mymfa-fenetre",
    "mymfa-mots-de-passe",
    "mymfa-activite",
    "ia-services",
    "smartui",
    "a-propos",
    "chat-welcome",
    "chat-writing",
    "chat-waiting",
    "chat-offline",
    "chat-still",
    "raccourcis",
    "raccourcis-essentiels",
    "bar-idle",
    "bar-hover",
    "bar-pressed",
    "bar-workspace-switch",
    "bar-controls",
    "bar-menu-switch",
    "bar-menu-closing",
    "bar-menu-shell",
    "bar-menu-workspaces",
    "bar-menu-window",
    "bar-menu-help",
    "bar-bottom",
    "bar-overflow",
    "bar-majs-progression",
    "bar-still",
    "bar-graphite-idle",
    "bar-graphite-pressed",
    "bar-graphite-switch",
    "bar-graphite-bottom",
    "bar-graphite-urgent",
    "controls-desktop",
    "overview",
    "menu-fenetre",
    "bar-selector-relief",
    "bar-selector-rail",
    "bar-selector-miniatures",
    "bar-selector-icones",
    "bar-selector-notif",
    "bar-graphite-edit",
    "bar-island-idle",
    "bar-island-hover",
    "bar-island-bottom",
    "bar-island-still",
    "bar-island-pressed",
    "bar-island-switch",
    "bar-island-notif",
    "bar-island-activity",
    "bar-island-urgent",
    "bar-convex-idle",
    "bar-convex-pressed",
    "bar-convex-switch",
    "bar-convex-urgent",
    "bar-convex-controls",
    "bar-convex-calendar",
    "bar-convex-opening",
    "bar-convex-bottom",
    "bar-convex-still",
    "bar-convex-merge",
    "bar-convex-settings",
    "bar-convex-launcher",
    "bar-convex-sides",
    "bar-convex-dock-shown",
    "bar-convex-notification",
    "wizard-1",
    "wizard-2",
    "wizard-3",
    "wizard-4",
    "wizard-5",
    "wizard-6",
    "wizard-7",
    "wizard-8",
    "wizard-9",
    "wizard-10",
    "performance",
    "fenetres-apparence",
    "fenetres-regles",
    "curseur",
    "environnement",
    "panorama-media",
    "panorama-system",
    "panorama-clock",
    "panorama-split",
    "panorama-video",
    "panorama-video-veil",
    "panorama-still",
    "panorama-forest",
    "panorama-telemetry",
    "panorama-studio",
    "panorama-applications",
    "panorama-webradio",
    "panorama-config-style",
    "panorama-config-composition",
    "panorama-config-shortcuts",
    "panorama-config-lecteur",
    "panorama-config-video",
    "bar-notifications",
    "bar-unread",
    "bar-island-unread",
    "reglages-valeur-saisie",
];

pub fn run(output: Option<PathBuf>) -> iced::Result {
    let panorama_only = std::env::var("HYPRDESK_PREVIEW_SCENES").is_ok_and(|s| {
        !s.is_empty()
            && s.split(',')
                .all(|name| name.trim().starts_with("panorama-"))
    });
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
        .clamp(if panorama_only { 240.0 } else { 480.0 }, 2560.0);
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
        let frames = if p.app.launcher_animating() && p.app.convex_options().is_none() {
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
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                Some(PreviewMsg::Ui(Box::new(Msg::Released)))
            }
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: keyboard::Key::Named(keyboard::key::Named::Escape),
                ..
            }) => Some(PreviewMsg::Ui(Box::new(Msg::BarDragCancel))),
            Event::Keyboard(keyboard::Event::KeyPressed { key, .. }) => {
                Some(PreviewMsg::Ui(Box::new(Msg::Key(Id::unique(), key))))
            }
            Event::Keyboard(keyboard::Event::ModifiersChanged(modifiers)) => {
                Some(PreviewMsg::Ui(Box::new(Msg::Modifiers(modifiers))))
            }
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
        screen: Default::default(),
        if_missing: Default::default(),
    }
}

impl Preview {
    fn boot(output: Option<PathBuf>, size: Size) -> (Self, Task<PreviewMsg>) {
        let (mut app, _) = App::boot_with(PathBuf::from("/tmp/hyprdesk-preview/config.lua"), true);
        let (desktop, bar) = (Id::unique(), Id::unique());
        app.font = crate::fonts::DEFAULT;
        app.monitors = vec![Monitor {
            dpms_off: false,
            name: crate::i18n::tr("APERÇU").into(),
            width: size.width as u32,
            height: size.height as u32,
            workspace: "1".into(),
            base: String::new(),
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
        // Plein bureau sur l'écran d'aperçu (workspace 1).
        app.ws_modes.insert("1".into(), DesktopMode::FullDesktop);
        app.applied.insert(
            crate::i18n::tr("APERÇU").into(),
            (DesktopMode::FullDesktop, "1".into()),
        );
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
                margin: 0,
                ..Default::default()
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
        let mut controls = widget("barcontrols", WidgetKind::Controls, Anchor::TopRight);
        controls.type_id = "controls".into();
        controls.in_bar = true;
        cfg.widgets.push(controls);
        cfg.bar = Some(BarDef {
            shape: Default::default(),
            monitors: None,
            position: BarPosition::Top,
            height: 44,
            margin: 0,
            zones: [
                vec![BarItem::Workspaces, BarItem::Window],
                vec![BarItem::Widget("barclock".into())],
                vec![
                    BarItem::Brightness,
                    BarItem::Volume,
                    BarItem::Desktop,
                    BarItem::Widget("barcontrols".into()),
                ],
            ],
            portrait: None,
            compact: Default::default(),
            style: Default::default(),
            colors: Default::default(),
            convex: Default::default(),
        });
        for (type_id, icon, detail) in [
            (
                "network",
                "network-wireless-signal-excellent-symbolic",
                "Maison",
            ),
            ("bluetooth", "bluetooth-active-symbolic", "Activé"),
        ] {
            let mut source = widget(
                &format!("bar:controls:{type_id}"),
                WidgetKind::Lua {
                    func: 0,
                    click: None,
                    scroll: None,
                    menu: Some(0),
                    toggle: Some(0),
                },
                Anchor::TopRight,
            );
            source.type_id = type_id.into();
            source.in_bar = true;
            app.values.insert(
                source.id.clone(),
                widgets::Value::Rich(config::Rendered {
                    icon: Some(icon.into()),
                    tooltip: Some(detail.into()),
                    toggled: Some(true),
                    ..Default::default()
                }),
            );
            cfg.widgets.push(source);
            let img = app.builtin_icon(icon);
            app.named_icons.insert(icon.into(), img);
        }
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
                monitor: crate::i18n::tr("APERÇU").into(),
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
                fullscreen: false,
            })
            .collect();
        if let Ok(name) = std::env::var("HYPRDESK_PREVIEW_THEME") {
            let name = if name == "light" { "Clair" } else { &name };
            if let Some(preset) = crate::theme::PRESETS
                .iter()
                .find(|preset| preset.name.eq_ignore_ascii_case(name))
            {
                app.settings.apply_preset(preset);
            }
        }
        // `HYPRDESK_PREVIEW_PACK=verre [HYPRDESK_PREVIEW_VARIANT=light]` : le
        // pack du moteur (palette, effets, jetons, verre) habille les scènes.
        if let Ok(id) = std::env::var("HYPRDESK_PREVIEW_PACK") {
            let roots = crate::theme_engine::paths::Roots::from_env();
            let variant =
                std::env::var("HYPRDESK_PREVIEW_VARIANT").unwrap_or_else(|_| "dark".into());
            match crate::theme_engine::bridge::resolve_pack(&roots, &id, &variant) {
                Ok((r, _)) => {
                    app.settings
                        .apply_theme(&crate::theme_engine::bridge::to_theme(&r));
                    // Forme du pack (MLG-276).
                    for (k, v) in &r.shape {
                        if let Err(e) =
                            crate::cli_config::apply_pack_value(&mut app.settings, k, v, &cfg)
                        {
                            eprintln!("{k} = {v} : {e}");
                        }
                    }
                    super::tokens::set_palette(r.palette);
                    super::tokens::set(r.design);
                }
                Err(e) => eprintln!("HYPRDESK_PREVIEW_PACK={id} : {e}"),
            }
        }
        let mut cfg = app.settings.apply_to(&cfg);
        // La barre de l'aperçu est posée en dur : sa forme vient des réglages
        // (pack essayé, MLG-277).
        if let (Some(b), Some(sb)) = (cfg.bar.as_mut(), app.settings.bar.as_ref()) {
            b.shape = sb.shape;
        }
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
        if self.output.is_none() {
            return Task::none();
        }
        let gesture = matches!(
            SCENES[self.scene],
            "bar-hover"
                | "bar-pressed"
                | "bar-workspace-switch"
                | "bar-menu-switch"
                | "bar-menu-closing"
                | "bar-graphite-pressed"
                | "bar-graphite-switch"
                | "bar-island-pressed"
                | "bar-island-switch"
                | "bar-convex-pressed"
                | "bar-convex-switch"
        );
        let capture = Task::perform(
            tokio::time::sleep(Duration::from_millis(
                if SCENES[self.scene].starts_with("panorama-") {
                    1100
                } else if gesture {
                    150
                } else {
                    500
                },
            )),
            |_| PreviewMsg::Capture,
        );
        if gesture {
            Task::batch([
                capture,
                Task::perform(tokio::time::sleep(Duration::from_millis(65)), |_| {
                    PreviewMsg::BarGesture
                }),
            ])
        } else {
            capture
        }
    }

    fn set_scene(&mut self) {
        self.app.panorama = Default::default();
        self.app.panel = None;
        self.app.chat = None;
        // La vue d'ensemble restait ouverte sur les scènes suivantes.
        self.app.overview = None;
        self.app.bar_preview = None;
        self.app.convex_runtime = Default::default();
        self.app
            .toasts
            .history
            .retain(|notification| notification.id < u32::MAX - 1);
        self.app.workspaces.retain(|w| w.id <= 4);
        let cfg = Arc::make_mut(&mut self.app.cfg);
        cfg.widgets
            .retain(|w| !w.id.starts_with("preview-module-") && w.id != "barlauncher");
        cfg.general.clipboard_history = SCENES[self.scene] == "bar-controls";
        if let Some(w) = cfg.widgets.iter_mut().find(|w| w.id == "barclock") {
            w.kind = WidgetKind::Clock {
                format: "%a %-d %b  ·  %H:%M".into(),
            };
        }
        if let Some(w) = cfg.widgets.iter_mut().find(|w| w.id == "barcontrols") {
            w.in_bar = true;
            w.y = 0.0;
        }
        if let Some(bar) = cfg.bar.as_mut() {
            bar.zones = [
                vec![BarItem::Workspaces, BarItem::Window],
                vec![BarItem::Widget("barclock".into())],
                vec![
                    BarItem::Brightness,
                    BarItem::Volume,
                    BarItem::Desktop,
                    BarItem::Widget("barcontrols".into()),
                ],
            ];
            // Retour à la pose de référence : les scènes Graphite se règlent
            // ensuite elles-mêmes (style, pose en bas, marge).
            bar.style = config::BarStyle::Continuite;
            bar.position = BarPosition::Top;
            bar.height = 44;
            bar.margin = 0;
        }
        self.app.urgent.clear();
        self.app.clients.retain(|c| c.address != "preview-urgent");
        if let Some(bar) = self.app.bar_surfaces.get_mut(&self.bar) {
            bar.position = BarPosition::Top;
            bar.height = 44;
            bar.margin = 0;
        }
        self.app.monitors[0].reserved = (0.0, 44.0, 0.0, 0.0);
        self.app.monitors[0].workspace = "1".into();
        self.app.popover = None;
        self.app.launcher_popup = None;
        self.app.launcher_runtime.docks.clear();
        self.app.launcher_runtime.launches.clear();
        self.app.launcher_runtime.focus = None;
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
            } else {
                // Les scènes suivantes repartent du fond par défaut : sinon le
                // noir de launcher-dark reste sous toutes les captures d'après.
                cfg.general.background = config::General::default().background;
                cfg.general.theme.effects.remove(&config::Place::Launcher);
            }
            let thickness = plugins::dock::thickness(cfg.general.launcher.size) as f32;
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
                    monitor: crate::i18n::tr("APERÇU").into(),
                });
            }
            self.app.hover_fades.clear();
            if self.scene == 17 {
                let _ = self
                    .app
                    .launcher_pointer(crate::i18n::tr("APERÇU"), Some(1.15));
            }
            if matches!(self.scene, 21 | 26 | 30) {
                self.app.launcher_popup = Some(plugins::dock::Popup {
                    shown: true,
                    id: Id::unique(),
                    monitor: crate::i18n::tr("APERÇU").into(),
                    key: "dock-pin:firefox".into(),
                    content: plugins::dock::PopupContent::Windows("firefox".into()),
                    fade: self.app.launcher_transition(
                        if self.scene == 26 { 0.5 } else { 1.0 },
                        if self.scene == 26 { 0.5 } else { 1.0 },
                        0,
                    ),
                    max_height: self.size.height - 160.0,
                    placement: plugins::dock::Placement {
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
            // Dock tenu au clavier : deuxième case choisie (anneau + vague).
            if SCENES[self.scene] == "launcher-keyboard" {
                let monitor = crate::i18n::tr("APERÇU").to_string();
                self.app.launcher_runtime.focus = Some((monitor.clone(), 1));
                let _ = self.app.launcher_pointer(&monitor, Some(1.0));
            }
            if self.scene == 30 {
                for index in 0..40 {
                    self.app.clients.push(Client {
                        address: format!("preview-many-{index}"),
                        class: "firefox".into(),
                        title: format!("Document {} — Notes de travail", index + 1),
                        workspace: "2".into(),
                        monitor: crate::i18n::tr("APERÇU").into(),
                    });
                }
            }
        }
        // Scènes des modes du bureau (captures) : fenêtres visibles, bureau masqué.
        // Les champs sont posés directement : les transitions réelles font de l'IPC.
        let apercu = crate::i18n::tr("APERÇU").to_string();
        let mode = match SCENES[self.scene] {
            "mode-launcher" => DesktopMode::LauncherOnly,
            "mode-hidden" => {
                self.app.visible = false;
                self.app.raised.remove(&self.desktop);
                DesktopMode::NoDesktop
            }
            _ => {
                // Bureau remis après mode-hidden : sans cela, les scènes
                // suivantes perdent icônes et widgets.
                self.app.visible = true;
                self.app.raised.insert(self.desktop);
                DesktopMode::FullDesktop
            }
        };
        self.app.ws_modes.insert("1".into(), mode);
        self.app.applied.insert(apercu, (mode, "1".into()));
        let tab = match self.scene {
            1 => Some("theme"),
            2 => Some("icons"),
            3 => Some("wallpaper"),
            4 => Some("widgets"),
            5 => Some("bar"),
            6 => Some("animations"),
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
            name if name.starts_with("panorama-config-") => Some("panorama"),
            "animations" | "animations-effets" => Some("animations"),
            "son" => Some("sound"),
            "energie" => Some("energy"),
            "performance" => Some("performance"),
            "fenetres-apparence" | "fenetres-regles" => Some("windows"),
            "curseur" => Some("input"),
            "environnement" => Some("hyprconfig"),
            "date-heure" => Some("datetime"),
            "reseau" => Some("network"),
            "bluetooth" => Some("bt"),
            "plugin-config" => Some("plugins"),
            "clavier-souris" => Some("input"),
            "ecran-connexion" => Some("login"),
            "mots-de-passe" => Some("passwords"),
            "ia-services" | "smartui" => Some("assistant"),
            "a-propos" => Some("about"),
            "raccourcis" | "raccourcis-essentiels" => Some("keys"),
            "reglages-valeur-saisie" => Some("dock"),
            "bar-graphite-edit" => Some("bar"),
            _ => None,
        });
        // Onglet de chaque paquet : le premier, sauf « typography » (Thème → Texte)
        // et HYPRDESK_PREVIEW_SUBTAB (ex. `bar=modules,theme=transparence`) pour les essais.
        self.app.subtabs.clear();
        // Configuration avancée par défaut (toutes les vues) ; HYPRDESK_PREVIEW_BASIC : l'essentiel.
        Arc::make_mut(&mut self.app.cfg).general.advanced_settings =
            std::env::var_os("HYPRDESK_PREVIEW_BASIC").is_none();
        // Visite guidée : proposée seulement dans sa scène.
        Arc::make_mut(&mut self.app.cfg).general.tour_seen = SCENES[self.scene] != "tour";
        if self.scene == 1 {
            self.app.subtabs.insert("theme", "texte");
        }
        // Ancien paquet Effets : Animations → Bureau.
        if SCENES[self.scene] == "effects" {
            self.app.subtabs.insert("animations", "bureau");
        }
        // Bureau → Raccourcis : des conflits d'exemple, et SUPER + R pris à rofi.
        self.app.key_report = if SCENES[self.scene] == "bureau" {
            let source = crate::keymap::parse_source(
                "local mainMod = \"SUPER\"\nlocal menu = \"rofi -show drun\"\n\
                 hl.bind(mainMod .. \" + R\", hl.dsp.exec_cmd(menu))\n\
                 hl.bind(mainMod .. \" + Tab\", hl.dsp.workspace.toggle_special(\"magic\"))\n\
                 hl.bind(\"SUPER + SHIFT + D\", hl.dsp.exec_cmd(\"discord\"), { submap_universal = true })\n",
            );
            let binds = serde_json::json!([
                { "modmask": 64, "key": "R", "submap": "", "submap_universal": "false", "description": "", "dispatcher": "HL.Dispatcher(exec_cmd)" },
                { "modmask": 64, "key": "Tab", "submap": "", "submap_universal": "false", "description": "", "dispatcher": "HL.Dispatcher(workspace)" },
                { "modmask": 65, "key": "D", "submap": "", "submap_universal": "true", "description": "", "dispatcher": "HL.Dispatcher(exec_cmd)" },
            ]);
            crate::keymap::analyse(&self.app.cfg.general.keys, true, &binds, &source).0
        } else if SCENES[self.scene].starts_with("raccourcis") {
            // Raccourcis : un hyprland.lua d'exemple (AZERTY, workspaces 1 à 7,
            // molette du layout scrolling en dwindle) et la submap de hyprdesk.
            let lua = "local mainMod = \"SUPER\"\n\
                 hl.bind(\"Print\", hl.dsp.exec_cmd(\"hyprshot -m region\"), { submap_universal = true })\n\
                 hl.bind(mainMod .. \" + Return\", hl.dsp.exec_cmd(\"kitty\"))\n\
                 hl.bind(mainMod .. \" + Q\", hl.dsp.window.close())\n\
                 hl.bind(mainMod .. \" + F\", hl.dsp.window.fullscreen())\n\
                 hl.bind(mainMod .. \" + h\", hl.dsp.focus({ direction = \"left\" }))\n\
                 hl.bind(mainMod .. \" + m\", hl.dsp.focus({ direction = \"right\" }))\n\
                 hl.bind(mainMod .. \" + ampersand\", hl.dsp.focus({ workspace = 1 }))\n\
                 hl.bind(mainMod .. \" + eacute\", hl.dsp.focus({ workspace = 2 }))\n\
                 hl.bind(mainMod .. \" + ALT + ampersand\", hl.dsp.window.move({ workspace = 1 }))\n\
                 hl.bind(mainMod .. \" + SHIFT + 1\", hl.dsp.window.move({ monitor = \"DP-1\" }))\n\
                 hl.bind(mainMod .. \" + mouse_down\", hl.dsp.layout(\"move +col\"))\n\
                 hl.bind(\"XF86AudioMute\", hl.dsp.exec_cmd(\"wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle\"))\n";
            let source = crate::keymap::parse_source(lua);
            let binds = serde_json::json!([
                { "modmask": 0, "key": "Print", "submap": "", "submap_universal": "true", "description": "", "dispatcher": "HL.Dispatcher(exec_cmd)" },
                { "modmask": 64, "key": "Return", "submap": "", "description": "", "dispatcher": "HL.Dispatcher(exec_cmd)" },
                { "modmask": 64, "key": "Q", "submap": "", "description": "", "dispatcher": "HL.Dispatcher(close)" },
                { "modmask": 64, "key": "F", "submap": "", "description": "", "dispatcher": "HL.Dispatcher(fullscreen)" },
                { "modmask": 64, "key": "h", "submap": "", "description": "", "dispatcher": "HL.Dispatcher(focus)" },
                { "modmask": 64, "key": "m", "submap": "", "description": "", "dispatcher": "HL.Dispatcher(focus)" },
                { "modmask": 64, "key": "ampersand", "submap": "", "description": "", "dispatcher": "HL.Dispatcher(focus)" },
                { "modmask": 64, "key": "eacute", "submap": "", "description": "", "dispatcher": "HL.Dispatcher(focus)" },
                { "modmask": 72, "key": "ampersand", "submap": "", "description": "", "dispatcher": "HL.Dispatcher(move)" },
                { "modmask": 65, "key": "1", "submap": "", "description": "", "dispatcher": "HL.Dispatcher(move)" },
                { "modmask": 64, "key": "mouse_down", "submap": "", "description": "", "dispatcher": "HL.Dispatcher(layout)" },
                { "modmask": 0, "key": "XF86AudioMute", "submap": "", "description": "", "dispatcher": "HL.Dispatcher(exec_cmd)" },
                { "modmask": 64, "key": "D", "submap": "", "description": "hyprdesk : changer de mode", "dispatcher": "HL.Dispatcher(exec_cmd)" },
                { "modmask": 64, "key": "R", "submap": "", "description": "hyprdesk : lanceur d'applications", "dispatcher": "HL.Dispatcher(exec_cmd)" },
                { "modmask": 64, "key": "", "keycode": 49, "submap": "", "description": "hyprdesk : Workspace précédent (aller-retour)", "dispatcher": "HL.Dispatcher(focus)" },
                { "modmask": 68, "key": "h", "submap": "", "description": "hyprdesk : Échanger avec la fenêtre voisine (gauche)", "dispatcher": "HL.Dispatcher(swap)" },
                { "modmask": 64, "key": "D", "submap": "hyprdesk", "description": "hyprdesk : no-desktop", "dispatcher": "HL.Dispatcher(exec_cmd)" },
                { "modmask": 64, "key": "", "keycode": 10, "submap": "hyprdesk", "description": "hyprdesk : workspace 1", "dispatcher": "HL.Dispatcher(focus)" },
                { "modmask": 64, "key": "", "keycode": 49, "submap": "hyprdesk", "description": "hyprdesk : Workspace précédent (aller-retour)", "dispatcher": "HL.Dispatcher(focus)" },
                { "modmask": 0, "key": "XF86AudioMute", "submap": "hyprdesk", "description": "hyprdesk : touche multimédia", "dispatcher": "HL.Dispatcher(exec_cmd)" },
            ]);
            let keys = &self.app.cfg.general.keys;
            let mut r = crate::keymap::analyse(keys, true, &binds, &source).0;
            r.essentials = crate::essentials::plan(keys, &binds, &source, &["dwindle".into()]);
            r.hyprland = crate::essentials::inventory(&binds, &source);
            if SCENES[self.scene] == "raccourcis-essentiels" {
                self.app.subtabs.insert("keys", "essentiels");
            }
            r
        } else {
            Default::default()
        };
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
                    details: vec![("program".into(), "/usr/bin/udisksctl".into())],
                },
                text: "Mot de passe :".into(),
                echo: false,
                input: zeroize::Zeroizing::new("secret".into()),
                message: Some((
                    crate::i18n::tr("Mot de passe refusé, réessayez.").into(),
                    true,
                )),
                reply: Some(super::polkit::Responder::detached()),
                show_details: false,
            });
        }
        // Les bulles n'appartiennent qu'à leur scène.
        if SCENES[self.scene] != "notifications" {
            self.app.toasts.list.clear();
            self.app.toasts.explain.clear();
        }
        // Bulles de notifications : une ordinaire, une critique avec actions.
        if SCENES[self.scene] == "notifications" {
            let mut crit = std::collections::HashMap::new();
            crit.insert("urgency".to_string(), zbus::zvariant::OwnedValue::from(2u8));
            // La seconde action est destructrice : bouton en rouge (HIG SYS-17).
            let acts: Vec<String> = [
                "default",
                "Ouvrir",
                "later",
                "Plus tard",
                "destructive:dismiss",
                "Ne plus m'avertir",
            ]
            .map(String::from)
            .to_vec();
            let _ = self
                .app
                .want_icon("preferences-system-notifications-symbolic");
            // Repère de la bulle critique : un dessin de hyprdesk, posé tout de suite.
            let _ = self
                .app
                .want_icon(crate::ui::plugins::bar::surface::URGENT_ICON);
            // Icône de l'application de la bulle, résolue tout de suite
            // (pas de tâche dans l'aperçu).
            for app in ["firefox", "thunderbird"] {
                if !self.app.named_icons.contains_key(app) {
                    let path = launch::lookup_icon(app, None, 32)
                        .or_else(|| launch::lookup_icon(app, Some("Adwaita"), 32));
                    let gen_ = self.app.icons_gen;
                    let _ = self
                        .app
                        .update_inner(Msg::IconResolved(gen_, app.into(), path));
                }
            }
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
                    request: None,
                    alert: false,
                },
                super::toasts::Toast {
                    n: crate::notifications::build(
                        1,
                        "Firefox",
                        "firefox",
                        "Téléchargement terminé",
                        "rapport-annuel.pdf (2,4 Mo)",
                        &[],
                        &Default::default(),
                        -1,
                    ),
                    since: std::time::Instant::now(),
                    request: None,
                    alert: false,
                },
                // Demande d'autorisation (HIG PAT-02) : la bulle de la première
                // notification d'une application, « Refuser » et « Autoriser »
                // au-dessus de son contenu.
                super::toasts::Toast {
                    n: crate::notifications::build(
                        3,
                        "Thunderbird",
                        "thunderbird",
                        "Nouveau message",
                        "Alice : on déjeune ?",
                        &[],
                        &Default::default(),
                        -1,
                    ),
                    since: std::time::Instant::now(),
                    request: Some("thunderbird".into()),
                    alert: false,
                },
            ];
            // Première critique de son application : sa bulle explique le
            // passage du Silence et permet de le refuser (HIG PAT-05).
            self.app.toasts.explain = vec![super::toasts::Explain {
                key: "Batterie".into(),
                id: 2,
            }];
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
                origin: crate::i18n::tr("Mises à jour").to_string(),
                copied: false,
            });
        } else {
            self.app.problem = None;
        }
        // Assistant de configuration, étape N.
        match SCENES[self.scene]
            .strip_prefix("wizard-")
            .and_then(|n| n.parse::<usize>().ok())
        {
            Some(n) => self.wizard_scene(n - 1),
            None => self.app.wizard = None,
        }
        // Lanceur : grande feuille (« applications ») ou carte centrée
        // (« lanceur-central ») ; quelques applications, par catégorie, et
        // des suggestions fixes (l'aperçu ne lit pas l'usage réel).
        if matches!(
            SCENES[self.scene],
            "applications" | "lanceur-central" | "bar-convex-launcher"
        ) {
            let app = |id: &str, name: &str, icon: &str, cat: &str| crate::launch::AppInfo {
                id: id.into(),
                name: name.into(),
                icon: Some(icon.into()),
                wm_class: None,
                mime_types: Vec::new(),
                categories: vec![cat.into()],
                available: true,
            };
            self.app.apps = Arc::new(vec![
                app("firefox", "Firefox", "firefox", "Network"),
                app(
                    "org.gnome.Nautilus",
                    "Fichiers",
                    "system-file-manager-symbolic",
                    "FileManager",
                ),
                app(
                    "kitty",
                    "Terminal",
                    "utilities-terminal",
                    "TerminalEmulator",
                ),
                app(
                    "libreoffice-writer",
                    "LibreOffice Writer",
                    "libreoffice-writer",
                    "Office",
                ),
                app(
                    "libreoffice-calc",
                    "LibreOffice Calc",
                    "libreoffice-calc",
                    "Office",
                ),
                app("gimp", "GIMP", "gimp", "Graphics"),
                app("inkscape", "Inkscape", "inkscape", "Graphics"),
                app("vlc", "VLC", "vlc", "AudioVideo"),
                app("code", "Visual Studio Code", "vscode", "Development"),
                app("steam", "Steam", "steam", "Game"),
                app(
                    "org.gnome.Settings",
                    "Paramètres",
                    "preferences-system-symbolic",
                    "Settings",
                ),
                app("filezilla", "FileZilla", "filezilla", "Network"),
                app("thunderbird", "Thunderbird", "thunderbird", "Email"),
                app("obs", "OBS Studio", "com.obsproject.Studio", "AudioVideo"),
            ]);
            Arc::make_mut(&mut self.app.cfg).general.apps_launcher.style =
                if SCENES[self.scene] == "applications" {
                    crate::config::AppsStyle::Sheet
                } else {
                    crate::config::AppsStyle::Spotlight
                };
            self.app.search = None;
            let _ = self.app.toggle_apps();
            if let Some(s) = self.app.search.as_mut() {
                s.suggested = vec![0, 2, 8, 4, 5, 7, 12, 1];
            }
        } else if SCENES[self.scene] == "recherche" {
            // Lanceur avec recherche : « fi » parmi quelques applications.
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
            Arc::make_mut(&mut self.app.cfg).general.apps_launcher.style =
                crate::config::AppsStyle::Spotlight;
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
                entry(42, "https://example.com/lists/13"),
                entry(41, "cargo test --release -- --nocapture"),
                entry(40, "Rendez-vous jeudi 14 h, salle B"),
                entry(39, "[[ binary data 48 KiB png 640x480 ]]"),
                entry(38, "fn main() { println!(\"bonjour\"); }"),
            ]));
        } else if SCENES[self.scene] == "reglages" {
            // Lanceur, mode réglages : « :r fond » dans l'index du tiroir.
            self.app.search = None;
            let _ = self.app.toggle_search();
            let _ = self
                .app
                .search_update(super::search::SMsg::Query(":r fond".into()));
        } else {
            self.app.search = None;
        }
        // Coffre myMFA : création (saisie en cours, confirmation fausse),
        // reprise de la sauvegarde du web, coffre prêt, fenêtre myMFA.
        self.app.mfa_dialog = None;
        self.app.mfa_fenetre = None;
        if SCENES[self.scene].starts_with("mymfa-") {
            self.app.mfa = crate::mymfa::State {
                installed: true,
                vault: true,
                sealed: true,
                ..Default::default()
            };
            let mut fenetre = super::mymfa_fenetre::Fenetre::apercu(iced::window::Id::unique());
            if SCENES[self.scene] == "mymfa-mots-de-passe" {
                fenetre.onglet = super::mymfa_fenetre::Onglet::MotsDePasse;
                fenetre.detail = Some("l1".to_string());
            }
            if SCENES[self.scene] == "mymfa-activite" {
                fenetre.onglet = super::mymfa_fenetre::Onglet::Activite;
            }
            self.app.mfa_fenetre = Some(fenetre);
        }
        if let Some(kind) = match SCENES[self.scene] {
            "coffre-creation" => Some(super::mymfa::Kind::Create),
            "coffre-sauvegarde" => Some(super::mymfa::Kind::Install(PathBuf::from(
                "/home/jacques/Téléchargements/mymfa-vault-2026-11-11.json",
            ))),
            "coffre-pret" => Some(super::mymfa::Kind::Ready { sealed: true }),
            _ => None,
        } {
            let creating = kind == super::mymfa::Kind::Create;
            self.app.mfa = crate::mymfa::State {
                installed: true,
                session_key: true,
                ..Default::default()
            };
            self.app.mfa_dialog = Some(super::mymfa::Dialog {
                win: iced::window::Id::unique(),
                kind,
                app: String::new(),
                input: if creating {
                    "correct horse".into()
                } else {
                    String::new()
                },
                confirm: if creating {
                    "correct hor".into()
                } else {
                    String::new()
                },
                backup: None,
                sealing: false,
                message: None,
                busy: false,
            });
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
        // IA → Assistant : l'abonnement Claude Code choisi.
        if SCENES[self.scene] == "assistant" {
            Arc::make_mut(&mut self.app.cfg)
                .general
                .apps
                .insert("ai".into(), "claude".into());
        }
        // IA : deux fournisseurs ajoutés, SmartUI par LYRA (par défaut).
        if matches!(SCENES[self.scene], "ia-services" | "smartui") {
            use crate::ai_services::{Provider, Service};
            let g = &mut Arc::make_mut(&mut self.app.cfg).general;
            g.ai.services = vec![
                Service {
                    id: "anthropic".into(),
                    name: "Claude (perso)".into(),
                    kind: Provider::Anthropic,
                    endpoint: String::new(),
                    model: String::new(),
                },
                Service {
                    id: "compatible".into(),
                    name: "Ollama (ce poste)".into(),
                    kind: Provider::Compatible,
                    endpoint: String::new(),
                    model: "qwen2.5-vl".into(),
                },
            ];
            g.launcher.smartui = true;
            g.launcher.smartui_auto = Some(true);
            self.app.subtabs.insert(
                "assistant",
                if SCENES[self.scene] == "smartui" {
                    "smartui"
                } else {
                    "services"
                },
            );
        }
        // Fenêtres, curseur, environnement (MLG-141, 142, 144) : valeurs
        // d'exemple, l'aperçu ne lit pas Hyprland.
        if matches!(
            SCENES[self.scene],
            "fenetres-apparence" | "fenetres-regles" | "curseur" | "environnement"
        ) {
            use crate::tiling::OptValue;
            let h = &mut Arc::make_mut(&mut self.app.cfg).general.hypr;
            h.options
                .insert("general.gaps_in".into(), OptValue::Sides([5; 4]));
            h.options
                .insert("general.border_size".into(), OptValue::Num(2.0));
            h.options.insert(
                "general.col.active_border".into(),
                OptValue::Gradient(
                    vec![
                        crate::config::Rgba(0x45, 0x41, 0x58, 0xff),
                        crate::config::Rgba(0x95, 0x80, 0xff, 0xff),
                    ],
                    90,
                ),
            );
            h.env = vec![
                ("XCURSOR_THEME".into(), "Adwaita".into()),
                ("XCURSOR_SIZE".into(), "24".into()),
                ("QT_QPA_PLATFORM".into(), "wayland;xcb".into()),
            ];
            h.rules = vec![crate::hyprset::Rule {
                kind: crate::hyprset::RuleKind::Window,
                body: serde_json::json!({
                    "name": "float-pavucontrol",
                    "match": { "class": "^(pavucontrol)$" },
                    "float": true
                }),
            }];
            let tab = match SCENES[self.scene] {
                "fenetres-regles" => ("windows", "regles"),
                "curseur" => ("input", "curseur"),
                "environnement" => ("hyprconfig", "environnement"),
                _ => ("windows", "apparence"),
            };
            self.app.subtabs.insert(tab.0, tab.1);
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
            // Valeur d'un curseur saisie (HIG SEL-08) : « Taille des icônes »,
            // Dock → Apparence ; l'aperçu ne peut pas cliquer la valeur.
            if SCENES[self.scene] == "reglages-valeur-saisie"
                && let Some(p) = self.app.panel.as_mut()
            {
                self.app.subtabs.insert("dock", "apparence");
                p.slider = Some(SliderEdit::new(
                    plugins::dock::size_spec(),
                    f64::from(self.app.cfg.general.launcher.size),
                ));
            }
        }
        // Clavier et souris : la configuration de l'utilisateur (l'aperçu ne
        // lit pas Hyprland) et les dispositions de xkeyboard-config.
        if SCENES[self.scene] == "clavier-souris"
            && let Some(p) = self.app.panel.as_mut()
        {
            let (layouts, variants) = crate::inputs::layouts();
            p.input = super::plugins::input::State {
                loaded: true,
                cur: Some(crate::inputs::Inputs {
                    kb_layout: "fr".into(),
                    kb_variant: "azerty".into(),
                    tp_natural_scroll: true,
                    ..Default::default()
                }),
                layouts,
                variants,
                ..Default::default()
            };
        }
        // IA → Assistant : LYRA installée, service actif (l'aperçu ne sonde rien).
        if SCENES[self.scene] == "assistant"
            && let Some(p) = self.app.panel.as_mut()
        {
            p.assistant.status = Some(super::plugins::assistant::Status {
                installed: true,
                service: true,
                provider_flag: true,
                context_dir: true,
            });
        }
        // Autres services IA : clé Anthropic au trousseau, Ollama sans clé.
        if SCENES[self.scene] == "ia-services"
            && let Some(p) = self.app.panel.as_mut()
        {
            use crate::ai_services::KeySource;
            let keys = &mut p.assistant.services.providers.keys;
            keys.insert("anthropic".into(), KeySource::Keyring);
            keys.insert("compatible".into(), KeySource::Missing);
        }
        // SmartUI design : icônes du dépôt (assets/preview/smartui), une
        // proposition à valider, une génération en cours.
        if SCENES[self.scene] == "smartui"
            && let Some(p) = self.app.panel.as_mut()
        {
            use super::plugins::assistant::services::Row;
            let demo = |key: &str| {
                std::fs::read(format!(
                    "{}/assets/preview/smartui/{key}.svg",
                    env!("CARGO_MANIFEST_DIR")
                ))
                .ok()
                .map(iced::widget::svg::Handle::from_memory)
            };
            let row = |key: &str, class: &str, name: &str, state: u8| {
                let icon = demo(key);
                Row {
                    key: key.into(),
                    class: class.into(),
                    name: name.into(),
                    original: crate::smartui::original_icon(class, &[], None),
                    proposal: if state == 1 { icon.clone() } else { None },
                    validated: if state == 0 { icon } else { None },
                }
            };
            let s = &mut p.assistant.services;
            s.rows = vec![
                row("firefox", "firefox", "Firefox", 0),
                row("kitty", "kitty", "kitty", 0),
                row("org.xfce.thunar", "thunar", "Thunar", 0),
                row("zcode", "zcode", "Zcode", 0),
                row("brave-desktop", "brave-browser", "Brave", 1),
                row("marktext", "marktext", "MarkText", 2),
            ];
            s.busy = Some("marktext".into());
        }
        // À propos : un poste de démonstration.
        if SCENES[self.scene] == "a-propos"
            && let Some(p) = self.app.panel.as_mut()
        {
            p.about.facts = Some(super::plugins::about::Facts {
                os: "Arch Linux".into(),
                kernel: "7.2.15-arch1-1".into(),
                hyprland: "v0.57.0 (3f1a2c4)".into(),
                lyra: Some("~/.local/bin/hyprpanel-chat".into()),
                deps: (21, 23),
                smartui_icons: 6,
                files: [
                    ("autostart.lua", false),
                    ("binds.lua", false),
                    ("hyprland.lua", false),
                    ("animations.lua", true),
                    ("appearance.lua", true),
                    ("env.lua", true),
                    ("inputs.lua", true),
                    ("layouts.lua", true),
                    ("monitors.lua", true),
                    ("rules.lua", true),
                    ("system.lua", true),
                ]
                .into_iter()
                .map(|(n, g)| (n.to_string(), g))
                .collect(),
                adopted: 99,
                history: (14, Some("16/11 09:42 · Réglages".into())),
            });
        }
        // Mots de passe : coffre ouvert, gnome-keyring encore là, reprise à confirmer.
        if SCENES[self.scene] == "mots-de-passe"
            && let Some(p) = self.app.panel.as_mut()
        {
            p.passwords = super::plugins::passwords::State {
                loaded: true,
                gnome_keyring: true,
                holder: Some(("gnome-keyring-daemon".into(), 1834)),
                pam_mymfa: true,
                mymfa_module: true,
                confirm: true,
                ..Default::default()
            };
            self.app.mfa = crate::mymfa::State {
                installed: true,
                vault: true,
                locked: false,
                sealed: true,
                session_key: false,
                secret_service: false,
            };
        }
        // Écran de connexion : GDM en service, greetd installé, configuration
        // écrite, bascule à confirmer.
        if SCENES[self.scene] == "ecran-connexion"
            && let Some(p) = self.app.panel.as_mut()
        {
            p.login = super::plugins::login::State {
                status: crate::greeter_setup::Status {
                    dm: Some("gdm.service".into()),
                    dm_active: true,
                    greetd: true,
                    cage: true,
                    greeter_user: true,
                    binary: Some(crate::greeter_setup::SYSTEM_BIN.into()),
                    command: Some(crate::greeter_setup::greeter_command(
                        crate::greeter_setup::SYSTEM_BIN,
                        &crate::greeter_setup::Profile {
                            layout: "fr".into(),
                            variant: "azerty".into(),
                            ..Default::default()
                        },
                        crate::greeter_setup::Compositor::Hyprland("start-hyprland"),
                    )),
                    hyprland: true,
                    fprintd: true,
                    ours: true,
                    loaded: true,
                    ..Default::default()
                },
                confirm: true,
                keyboard: ("fr".into(), "azerty".into(), String::new()),
                ..Default::default()
            };
        }
        // Plugins → Mises à jour → Configurer : options et commandes d'exemple.
        if SCENES[self.scene] == "plugin-config" {
            use crate::tiling::OptValue;
            Arc::make_mut(&mut self.app.cfg)
                .catalog
                .push(crate::config::WidgetType {
                    id: "updates".into(),
                    name: crate::i18n::tr("Mises à jour").into(),
                    description: crate::i18n::tr("Paquets à mettre à jour ; mise à jour en un clic. Tout à jour : une coche discrète (option masquer_a_jour pour la cacher).").into(),
                    source: crate::config::WidgetSource::Bundled,
                    anchor: crate::config::Anchor::TopRight,
                    toggleable: true,
                    error: None,
                    options: vec![
                        ("detaille".into(), OptValue::Bool(true)),
                        ("masquer_a_jour".into(), OptValue::Bool(false)),
                        ("texte".into(), OptValue::Bool(true)),
                    ],
                });
        }
        if SCENES[self.scene] == "plugin-config"
            && let Some(p) = self.app.panel.as_mut()
        {
            p.plugins.tab = super::plugins::plugins::Tab::Widgets;
            p.plugins.configuring = Some("updates".into());
            p.plugins.polkit_keep = true;
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
        // Performance : profil Équilibré posé (l'aperçu ne lit pas Hyprland).
        if SCENES[self.scene] == "performance" {
            crate::perf::Profile::Balanced
                .apply(&mut Arc::make_mut(&mut self.app.cfg).general.fx.options);
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
        if SCENES[self.scene] == "proprietes" {
            // Propriétés d'un vrai fichier (le binaire lui-même).
            let exe = std::env::current_exe().unwrap_or_else(|_| "/etc/hostname".into());
            let mut m = super::context::IconMenu::new(
                super::context::MenuTarget::Icon,
                iced::window::Id::unique(),
            );
            m.icon = 1;
            m.path = Some(exe.clone());
            m.props = Some(super::context::path_properties(&exe));
            self.app.icon_menu = Some(m);
        } else if SCENES[self.scene] == "menu-icone" {
            let mut m = super::context::IconMenu::new(
                super::context::MenuTarget::Icon,
                iced::window::Id::unique(),
            );
            m.icon = 1;
            m.path = Some("/home/u/Bureau/Rapport.pdf".into());
            m.confirm = true;
            m.hover = Some(3);
            self.app.icon_menu = Some(m);
        } else if SCENES[self.scene] == "menu-dock" {
            // Menu d'une application du dock (MLG-307) : ses fenêtres, ses
            // actions `.desktop`, l'épingler, la fermer.
            // Dock en bas dans l'aperçu : la carte s'ouvre vers le haut et ses
            // groupes s'inversent (HIG MENU-09).
            let mut m = Self::menu_dock_app();
            m.upward = true;
            let launch = crate::i18n::tr("Nouvelle fenêtre").to_string();
            m.hover = self
                .app
                .icon_menu_lines(&m)
                .iter()
                .position(|l| matches!(l, super::menu::Line::Item(i) if i.label == launch));
            self.app.icon_menu = Some(m);
        } else if SCENES[self.scene] == "menu-dock-long" {
            // Le même menu, plus haut que l'écran (HIG MENU-02) : la carte est
            // plafonnée à l'écran et ses lignes défilent à l'intérieur.
            let monitor = self.app.monitors[0].name.clone();
            self.app.clients = (1..=60)
                .map(|k| crate::hypr::Client {
                    address: format!("0x{k:x}"),
                    class: "firefox".into(),
                    title: format!("Fenêtre {k}"),
                    workspace: "1".into(),
                    monitor: monitor.clone(),
                })
                .collect();
            let mut m = Self::menu_dock_app();
            m.monitor = monitor;
            let full = self.app.icon_menu_height(&m) + 2.0 * super::context::SHADOW;
            let screen = self.app.monitors[0].height as f32;
            m.cap(full, Some(screen));
            self.app.icon_menu = Some(m);
        } else if SCENES[self.scene] == "menu-edition" {
            // Menu d'édition d'un champ (MLG-309), sélection faite.
            let mut m = super::context::IconMenu::new(
                super::context::MenuTarget::Edit(super::context::EditRef {
                    surface: iced::window::Id::unique(),
                    can_copy: true,
                    editable: true,
                    can_paste: true,
                }),
                iced::window::Id::unique(),
            );
            m.hover = Some(1);
            self.app.icon_menu = Some(m);
        } else {
            self.app.icon_menu = None;
        }
        // Icônes des menus, résolues tout de suite (pas de tâche dans l'aperçu).
        if let Some(m) = &self.app.icon_menu {
            for name in super::menu::icons(&self.app.icon_menu_lines(m)) {
                if !self.app.named_icons.contains_key(&name) {
                    let path = launch::lookup_icon(&name, None, 32)
                        .or_else(|| launch::lookup_icon(&name, Some("Adwaita"), 32));
                    let gen_ = self.app.icons_gen;
                    let _ = self.app.update_inner(Msg::IconResolved(gen_, name, path));
                }
            }
        }
        // Sélecteur de fenêtres (Alt+Tab), la deuxième choisie.
        if SCENES[self.scene] == "fenetres" {
            let w = |a: &str, class: &str, title: &str, ws: &str| crate::switcher::Win {
                address: a.into(),
                class: class.into(),
                title: title.into(),
                workspace: ws.into(),
                focus: 0,
                stashed: ws == crate::hypr::STASH,
            };
            self.app.switcher = Some(super::switcher::Switcher {
                win: iced::window::Id::unique(),
                wins: vec![
                    w("0x1", "kitty", "~/projets/hyprdesk — cargo test", "1"),
                    w(
                        "0x2",
                        "firefox",
                        "Plane — MLG-22 Fenêtres",
                        crate::hypr::STASH,
                    ),
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
        if SCENES[self.scene] == "chat" || SCENES[self.scene].starts_with("chat-") {
            self.app.chat_demo(self.desktop);
            if let Some(chat) = self.app.chat.as_mut() {
                let c = chat.cur_mut();
                match SCENES[self.scene] {
                    "chat-welcome" => {
                        c.entries.clear();
                        c.input.clear();
                        c.busy = false;
                        c.thinking = false;
                    }
                    "chat-writing" => {
                        c.entries.truncate(4);
                        c.thinking = false;
                        let raw = crate::i18n::tr("Je prépare un bureau plus aéré, avec des marges et des couleurs qui vous ressemblent.").to_string();
                        c.entries.push(chat::Entry::Assistant {
                            md: iced::widget::markdown::Content::parse(&raw),
                            raw,
                        });
                    }
                    "chat-waiting" => {
                        c.busy = false;
                        c.thinking = false;
                    }
                    "chat-offline" => {
                        c.busy = false;
                        c.thinking = false;
                        c.ready = false;
                        c.ended = Some(
                            crate::i18n::tr("La connexion avec LYRA a été interrompue.").into(),
                        );
                    }
                    _ => {}
                }
            }
            if SCENES[self.scene] == "chat-still" {
                Arc::make_mut(&mut self.app.cfg).general.transition = Transition::None;
            }
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
            self.app.light = crate::ui::plugins::bar::surface::Light {
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
        if SCENES[self.scene] == "controls-desktop" {
            let controls = Arc::make_mut(&mut self.app.cfg)
                .widgets
                .iter_mut()
                .find(|w| w.id == "barcontrols")
                .unwrap();
            controls.in_bar = false;
            controls.anchor = Anchor::TopRight;
            controls.y = 40.0;
            self.app.light.screen = Some(crate::brightness::Device {
                subsystem: "backlight",
                name: "preview".into(),
                current: 72,
                max: 100,
            });
        }
        if SCENES[self.scene].starts_with("bar-") {
            // Une scène isolée et une capture après le parcours complet doivent
            // partir du même bureau, indépendamment des scènes noir/blanc ou masqué.
            self.app.visible = true;
            self.app.raised.insert(self.desktop);
            self.app.fade = None;
            let general = &mut Arc::make_mut(&mut self.app.cfg).general;
            general.background = config::General::default().background;
            general.theme.effects.remove(&config::Place::Launcher);
            Arc::make_mut(&mut self.app.cfg).general.transition =
                if SCENES[self.scene] == "bar-still" {
                    Transition::None
                } else {
                    Transition::Fade
                };
            self.app.light.screen = Some(crate::brightness::Device {
                subsystem: "backlight",
                name: "preview".into(),
                current: 72,
                max: 100,
            });
            if SCENES[self.scene] == "bar-bottom" {
                self.app.bar_surfaces.get_mut(&self.bar).unwrap().position = BarPosition::Bottom;
                self.app.monitors[0].reserved = (0.0, 0.0, 0.0, 44.0);
            }
            if SCENES[self.scene].starts_with("bar-graphite-") {
                // La composition du préréglage : le titre de l'application au
                // premier plan à gauche, les espaces centrés, et à droite les
                // indicateurs du quotidien, l'horloge courte et le seul
                // panneau de réglages rapides.
                let cfg = Arc::make_mut(&mut self.app.cfg);
                for (id, type_id, icon) in [
                    (
                        "preview-module-network",
                        "network",
                        "network-wireless-signal-excellent-symbolic",
                    ),
                    (
                        "preview-module-battery",
                        "battery",
                        "battery-level-good-symbolic",
                    ),
                ] {
                    let mut w = widget(
                        id,
                        WidgetKind::Lua {
                            func: 0,
                            click: None,
                            scroll: None,
                            menu: None,
                            toggle: None,
                        },
                        Anchor::TopRight,
                    );
                    w.type_id = type_id.into();
                    w.in_bar = true;
                    self.app.values.insert(
                        id.to_string(),
                        widgets::Value::Rich(config::Rendered {
                            icon: Some(icon.into()),
                            ..Default::default()
                        }),
                    );
                    cfg.widgets.push(w);
                }
                let bar = cfg.bar.as_mut().unwrap();
                bar.style = config::BarStyle::Graphite;
                if let Some(w) = cfg.widgets.iter_mut().find(|w| w.id == "barclock") {
                    w.kind = WidgetKind::Clock {
                        format: "%H:%M".into(),
                    };
                }
                let mut launcher = widget("barlauncher", WidgetKind::Launcher, Anchor::Bottom);
                launcher.type_id = "launcher".into();
                launcher.in_bar = true;
                cfg.widgets.retain(|w| w.id != "barlauncher");
                cfg.widgets.push(launcher);
                bar.zones = [
                    vec![],
                    vec![BarItem::Widget("barlauncher".into())],
                    vec![
                        BarItem::Widget("preview-module-network".into()),
                        BarItem::Volume,
                        BarItem::Widget("preview-module-battery".into()),
                        BarItem::Widget("barclock".into()),
                    ],
                ];
                bar.position = BarPosition::Bottom;
                bar.height = 48;
                bar.margin = 0;
                let surface = self.app.bar_surfaces.get_mut(&self.bar).unwrap();
                surface.position = BarPosition::Bottom;
                surface.height = 48;
                surface.margin = 0;
                self.app.monitors[0].reserved = (0.0, 0.0, 0.0, 48.0);
            }

            if SCENES[self.scene].starts_with("bar-island-") {
                // Capsule de référence : vide au repos, commandes révélées.
                let cfg = Arc::make_mut(&mut self.app.cfg);
                let bar = cfg.bar.as_mut().unwrap();
                bar.style = config::BarStyle::Island;
                bar.height = 36;
                bar.margin = 8;
                bar.position = if SCENES[self.scene] == "bar-island-bottom" {
                    BarPosition::Bottom
                } else {
                    BarPosition::Top
                };
                let surface = self.app.bar_surfaces.get_mut(&self.bar).unwrap();
                surface.position = bar.position;
                surface.height = 36;
                surface.margin = 8;
                self.app.monitors[0].reserved = (0.0, 44.0, 0.0, 0.0);
                // L'ovale des non-lues (HIG SYS-02) se pose sur l'horloge :
                // la pilule la montre, commandes révélées.
                if !matches!(
                    SCENES[self.scene],
                    "bar-island-idle"
                        | "bar-island-notif"
                        | "bar-island-activity"
                        | "bar-island-bottom"
                ) {
                    self.app.bar_preview = Some((6, false));
                }
                if SCENES[self.scene] == "bar-island-bottom" {
                    self.app.monitors[0].reserved = (0.0, 0.0, 0.0, 44.0);
                }
                if SCENES[self.scene] == "bar-island-still" {
                    Arc::make_mut(&mut self.app.cfg).general.transition = config::Transition::None;
                }
                if SCENES[self.scene] == "bar-island-urgent" {
                    self.app.clients.push(Client {
                        address: "preview-urgent".into(),
                        class: "firefox".into(),
                        title: crate::i18n::tr("Mindlog · Notes du jour").into(),
                        workspace: "2".into(),
                        monitor: crate::i18n::tr("APERÇU").into(),
                    });
                    self.app.urgent.insert("preview-urgent".into());
                }
                if matches!(
                    SCENES[self.scene],
                    "bar-island-notif" | "bar-island-activity"
                ) {
                    // Prise de main : une bulle vivante remplace l'heure de
                    // la pilule centrale. Sa vue étendue (HIG SYS-13) montre
                    // un corps et des actions.
                    let activity = SCENES[self.scene] == "bar-island-activity";
                    let _ = self.app.want_icon("firefox");
                    self.app.toasts.list = vec![super::toasts::Toast {
                        n: crate::notifications::build(
                            1,
                            "Firefox",
                            "firefox",
                            "Téléchargement terminé",
                            if activity {
                                "rapport-annuel.pdf (2,4 Mo) est dans le dossier Téléchargements."
                            } else {
                                "rapport-annuel.pdf (2,4 Mo)"
                            },
                            &if activity {
                                [
                                    "default",
                                    "Ouvrir",
                                    "show",
                                    "Afficher le dossier",
                                    "share",
                                    "Partager",
                                ]
                                .map(String::from)
                                .to_vec()
                            } else {
                                vec![]
                            },
                            &Default::default(),
                            -1,
                        ),
                        since: std::time::Instant::now(),
                        request: None,
                        alert: false,
                    }];
                }
            }

            if SCENES[self.scene].starts_with("bar-convex-") {
                // Lame à niches : bord à bord en haut, 48 d'air (44 de
                // silhouette + 4 sous la lame).
                let cfg = Arc::make_mut(&mut self.app.cfg);
                cfg.bar.as_mut().unwrap().style = config::BarStyle::Convexe;
                cfg.bar.as_mut().unwrap().height = 48;
                cfg.bar.as_mut().unwrap().margin = 0;
                if SCENES[self.scene] == "bar-convex-bottom" {
                    cfg.bar.as_mut().unwrap().position = BarPosition::Bottom;
                }
                if SCENES[self.scene] == "bar-convex-still" {
                    cfg.general.transition = config::Transition::None;
                }

                let surface = self.app.bar_surfaces.get_mut(&self.bar).unwrap();
                surface.position = BarPosition::Top;
                surface.height = 48;
                surface.margin = 0;
                self.app.monitors[0].reserved = (0.0, 48.0, 0.0, 0.0);
                if SCENES[self.scene] == "bar-convex-urgent" {
                    self.app.clients.push(Client {
                        address: "preview-urgent".into(),
                        class: "firefox".into(),
                        title: crate::i18n::tr("Mindlog · Notes du jour").into(),
                        workspace: "2".into(),
                        monitor: crate::i18n::tr("APERÇU").into(),
                    });
                    self.app.urgent.insert("preview-urgent".into());
                }
            }

            if SCENES[self.scene] == "bar-convex-bottom" {
                self.app.bar_surfaces.get_mut(&self.bar).unwrap().position = BarPosition::Bottom;
                self.app.monitors[0].reserved = (0.0, 0.0, 0.0, 48.0);
            }
            if matches!(
                SCENES[self.scene],
                "bar-convex-controls" | "bar-convex-opening" | "bar-convex-still"
            ) {
                self.app.bar_anchors.insert(
                    (self.bar, "barcontrols".into()),
                    iced::Rectangle::new(
                        iced::Point::new(self.size.width - 40.0, 0.0),
                        Size::new(32.0, 48.0),
                    ),
                );
                let _ = self.app.open_popover(
                    self.bar,
                    "barcontrols".into(),
                    popover::Kind::Widget("barcontrols".into()),
                );
            }
            if SCENES[self.scene] == "bar-convex-calendar" {
                let _ = self
                    .app
                    .open_popover(self.bar, "barclock".into(), popover::Kind::Calendar);
            }
            if SCENES[self.scene] == "bar-convex-sides" {
                let m = self.app.monitors[0].name.clone();
                let _ = self
                    .app
                    .convex_hover(m.clone(), super::convex::Part::Left, true);
                let _ = self.app.convex_hover(m, super::convex::Part::Right, true);
            }
            if SCENES[self.scene] == "bar-convex-dock-shown" {
                let _ = self.app.convex_hover(
                    self.app.monitors[0].name.clone(),
                    super::convex::Part::Dock,
                    true,
                );
            }
            if SCENES[self.scene] == "bar-convex-notification" {
                let n = crate::notifications::build(
                    42,
                    "Hyprdesk",
                    "",
                    "Téléchargement terminé",
                    "Votre fichier est prêt.",
                    &[],
                    &Default::default(),
                    -1,
                );
                let _ = self.app.toasts_update(super::toasts::TMsg::Event(
                    crate::notifications::Event::Shown(n),
                ));
            }
            if SCENES[self.scene] == "bar-convex-launcher" {
                self.app.search = None;
                let _ = self.app.toggle_search();
                if let Some(s) = self.app.search.as_mut() {
                    s.visibility = Some(Fade {
                        from: 1.0,
                        to: 1.0,
                        start: Instant::now(),
                        dur: Duration::ZERO,
                    });
                }
            }
            if SCENES[self.scene] == "bar-convex-settings" {
                self.app.settings.bar =
                    Some(crate::settings::BarSettings::from_config(&self.app.cfg));
                self.app.panel = Some(Panel::new(self.desktop));
                let _ = self.app.panel_update(PanelMsg::Tab("bar"));
            }

            if SCENES[self.scene] == "bar-graphite-urgent" {
                // Une fenêtre marquée urgente sur un espace non actif : le
                // losange se pose à côté de son numéro.
                self.app.clients.push(Client {
                    address: "preview-urgent".into(),
                    class: "firefox".into(),
                    title: crate::i18n::tr("Mindlog · Notes du jour").into(),
                    workspace: "2".into(),
                    monitor: crate::i18n::tr("APERÇU").into(),
                });
                self.app.urgent.insert("preview-urgent".into());
            }
            if matches!(
                SCENES[self.scene],
                "bar-controls"
                    | "bar-menu-switch"
                    | "bar-menu-closing"
                    | "bar-bottom"
                    | "bar-still"
            ) {
                let _ = self.app.open_popover(
                    self.bar,
                    "barcontrols".into(),
                    popover::Kind::Widget("barcontrols".into()),
                );
                self.demo_controls();
            }
            // Menu « hyprdesk » ouvert (HIG MENU-01) : son rendu se vérifie sans
            // session. Les scènes d'aperçu n'exécutent pas les tâches : les
            // icônes du menu sont résolues tout de suite. L'ancre est
            // provisoire ; le rendu du titre pose la sienne.
            if SCENES[self.scene] == "bar-menu-shell" {
                // Une icône de fichier du bureau est sélectionnée : « Mettre à
                // la corbeille » y est active (HIG MENU-04).
                let file = crate::config::IconDef {
                    origin: Default::default(),
                    sel: Default::default(),
                    id: "apercu-notes".into(),
                    label: Some("notes.txt".into()),
                    kind: crate::config::IconKind::Path(std::path::PathBuf::from("/tmp/notes.txt")),
                    icon: None,
                    pos: None,
                };
                for entry in super::plugins::bar::menus::entries(Some(&file)) {
                    if let crate::config::MenuEntry::Item {
                        icon: Some(icon), ..
                    } = entry
                    {
                        let image = self.app.builtin_icon(&icon);
                        self.app.named_icons.insert(icon, image);
                    }
                }
                self.app.bar_anchors.insert(
                    (self.bar, "menus:hyprdesk".into()),
                    iced::Rectangle::new(iced::Point::new(8.0, 0.0), Size::new(76.0, 44.0)),
                );
                let _ = self.app.open_popover(
                    self.bar,
                    "menus:hyprdesk".into(),
                    popover::Kind::Menu {
                        entries: super::plugins::bar::menus::entries(Some(&file)),
                        source: popover::MenuSource::Shell,
                    },
                );
            }
            // Carte des espaces ouverte du titre « Espaces » (HIG MENU-19) : la
            // rangée « Organiser les fenêtres… » est celle du composant commun,
            // après un séparateur. Même idiome : icône résolue tout de suite,
            // ancre provisoire.
            if SCENES[self.scene] == "bar-menu-workspaces" {
                let image = self.app.builtin_icon("workspace-switcher");
                self.app
                    .named_icons
                    .insert("workspace-switcher".into(), image);
                self.app.bar_anchors.insert(
                    (self.bar, "menus:workspaces".into()),
                    iced::Rectangle::new(iced::Point::new(104.0, 0.0), Size::new(96.0, 44.0)),
                );
                let _ = self.app.open_popover(
                    self.bar,
                    "menus:workspaces".into(),
                    popover::Kind::Workspaces,
                );
            }
            // Menu « Fenêtre », « Déplacer vers l'espace » ouvert (HIG
            // MENU-03) : la colonne liste les autres espaces, à côté d'une
            // liste qui garde sa largeur et sa place. Même idiome : icône
            // résolue tout de suite, ancre provisoire.
            if SCENES[self.scene] == "bar-menu-window" {
                let image = self.app.builtin_icon(crate::ui::menu::CHEVRON);
                self.app
                    .named_icons
                    .insert(crate::ui::menu::CHEVRON.into(), image);
                self.app.bar_anchors.insert(
                    (self.bar, "menus:window".into()),
                    iced::Rectangle::new(iced::Point::new(104.0, 0.0), Size::new(96.0, 44.0)),
                );
                let window: serde_json::Value =
                    serde_json::from_str(r#"{"address":"0xabc","workspace":{"id":2,"name":"2"}}"#)
                        .unwrap();
                let spaces: Vec<crate::hypr::Workspace> = (1..=5)
                    .map(|i| crate::hypr::Workspace {
                        id: i,
                        name: i.to_string(),
                        monitor: crate::i18n::tr("APERÇU").into(),
                        windows: 1,
                        fullscreen: false,
                    })
                    .collect();
                if let Some((_, entries)) =
                    super::plugins::bar::window::entries(&window, &spaces, 2)
                {
                    let sub = entries
                        .iter()
                        .position(|e| matches!(e, crate::config::MenuEntry::Submenu { .. }));
                    let _ = self.app.open_popover(
                        self.bar,
                        "menus:window".into(),
                        popover::Kind::Menu {
                            entries,
                            source: popover::MenuSource::Window {
                                address: "0xabc".into(),
                            },
                        },
                    );
                    if let Some(p) = self.app.popover.as_mut()
                        && let Some(sub) = sub
                    {
                        p.sub = Some(sub);
                    }
                    let _ = self.app.refresh_popover_anchor();
                }
            }
            // Menu « Aide » ouvert (HIG MENU-05) : le champ de recherche en
            // tête, l'aide du shell, « À propos de hyprdesk ». Même idiome :
            // icône résolue tout de suite, ancre provisoire (quatrième titre).
            if SCENES[self.scene] == "bar-menu-help" {
                let image = self.app.builtin_icon("help-about");
                self.app.named_icons.insert("help-about".into(), image);
                self.app.bar_anchors.insert(
                    (self.bar, "menus:help".into()),
                    iced::Rectangle::new(iced::Point::new(208.0, 0.0), Size::new(52.0, 44.0)),
                );
                let _ = self.app.open_popover(
                    self.bar,
                    "menus:help".into(),
                    popover::Kind::Menu {
                        entries: super::plugins::bar::menus::help_entries(),
                        source: popover::MenuSource::Shell,
                    },
                );
            }
            // Vue étendue d'une activité de la pilule Island (HIG SYS-13) :
            // le corps et les actions de la bulle vivante, puis la carte des
            // notifications et le calendrier en pied. Même idiome : l'icône
            // de l'application résolue tout de suite ; sans ancre relevée,
            // la carte se centre sous la pilule.
            if SCENES[self.scene] == "bar-island-activity" {
                let gen_ = self.app.icons_gen;
                if !self.app.named_icons.contains_key("firefox") {
                    let path = launch::lookup_icon("firefox", None, 32)
                        .or_else(|| launch::lookup_icon("firefox", Some("Adwaita"), 32));
                    let _ = self
                        .app
                        .update_inner(Msg::IconResolved(gen_, "firefox".into(), path));
                }
                let _ = self
                    .app
                    .open_popover(self.bar, "clock".into(), popover::Kind::Activity(1));
            }
            // Carte Notifications de la barre (HIG LAY-02) : l'historique est
            // groupé par application, une boîte par application. Même idiome :
            // icônes résolues tout de suite (pas de tâche dans l'aperçu). La
            // carte se passe d'ancre : elle se centre.
            if SCENES[self.scene] == "bar-notifications" {
                let gen_ = self.app.icons_gen;
                for icon in ["firefox", "signal"] {
                    if !self.app.named_icons.contains_key(icon) {
                        let path = launch::lookup_icon(icon, None, 32)
                            .or_else(|| launch::lookup_icon(icon, Some("Adwaita"), 32));
                        let _ = self
                            .app
                            .update_inner(Msg::IconResolved(gen_, icon.into(), path));
                    }
                }
                self.app.toasts.history = [
                    (
                        1,
                        "Firefox",
                        "firefox",
                        "Téléchargement terminé",
                        "rustbook.pdf — 4,2 Mo",
                    ),
                    (
                        2,
                        "Signal",
                        "signal",
                        "Nouveau message",
                        "Anna : on se voit à 19 h ?",
                    ),
                    (
                        3,
                        "Firefox",
                        "firefox",
                        "Mise à jour disponible",
                        "Redémarrez pour installer la version 143.",
                    ),
                    (
                        4,
                        "Hyprdesk",
                        "",
                        "Capture enregistrée",
                        "Copiée dans le presse-papiers",
                    ),
                    (5, "Signal", "signal", "Marc a rejoint le groupe", ""),
                ]
                .into_iter()
                .map(|(id, app, icon, title, body)| {
                    crate::notifications::build(
                        id,
                        app,
                        icon,
                        crate::i18n::tr(title),
                        crate::i18n::tr(body),
                        &[],
                        &Default::default(),
                        -1,
                    )
                })
                .collect();
                let _ = self.app.open_popover(
                    self.bar,
                    "notifications".into(),
                    popover::Kind::Notifications,
                );
            }
            // Ovale des non-lues sur l'horloge (HIG SYS-02) : trois non lues,
            // aucune liste ouverte.
            if matches!(SCENES[self.scene], "bar-unread" | "bar-island-unread") {
                self.app.toasts.history = ["Firefox", "Signal", "Firefox"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, app)| {
                        crate::notifications::build(
                            i as u32 + 1,
                            app,
                            "",
                            "Nouveau message",
                            "",
                            &[],
                            &Default::default(),
                            -1,
                        )
                    })
                    .collect();
            }
            // Mise à jour en cours (MLG-161) : mode compact (« 42 % ») puis
            // mode détaillé (jauge et %), côte à côte.
            if SCENES[self.scene] == "bar-majs-progression" {
                for (id, progress) in [
                    ("preview-majs-compact", None),
                    ("preview-majs-detaille", Some(0.42)),
                ] {
                    let mut w = widget(
                        id,
                        WidgetKind::Lua {
                            func: 0,
                            click: None,
                            scroll: None,
                            menu: Some(0),
                            toggle: None,
                        },
                        Anchor::TopRight,
                    );
                    w.in_bar = true;
                    self.app.values.insert(
                        id.to_string(),
                        widgets::Value::Rich(config::Rendered {
                            icon: Some("software-update-available-symbolic".into()),
                            text: "42 %".into(),
                            state: config::RenderState::Active,
                            progress,
                            ..Default::default()
                        }),
                    );
                    let cfg = Arc::make_mut(&mut self.app.cfg);
                    cfg.widgets.push(w);
                    let bar = cfg.bar.as_mut().unwrap();
                    // Seuls ces deux modules à droite : rien ne part dans « … ».
                    bar.zones[2].retain(
                        |i| matches!(i, BarItem::Widget(w) if w.starts_with("preview-majs")),
                    );
                    bar.zones[2].push(BarItem::Widget(id.into()));
                    if let Some(p) = bar.portrait.as_mut() {
                        p[2].retain(
                            |i| matches!(i, BarItem::Widget(w) if w.starts_with("preview-majs")),
                        );
                        p[2].push(BarItem::Widget(id.into()));
                    }
                }
            }
            if SCENES[self.scene] == "bar-overflow" {
                self.app
                    .workspaces
                    .extend((5..=40).map(|id| hypr::Workspace {
                        id,
                        name: id.to_string(),
                        monitor: self.app.monitors[0].name.clone(),
                        windows: 0,
                        fullscreen: false,
                    }));
                for i in 0..10 {
                    let id = format!("preview-module-{i}");
                    let mut w = widget(
                        &id,
                        WidgetKind::Lua {
                            func: 0,
                            click: None,
                            scroll: None,
                            menu: Some(0),
                            toggle: None,
                        },
                        Anchor::TopRight,
                    );
                    w.in_bar = true;
                    self.app.values.insert(
                        id.clone(),
                        widgets::Value::Rich(config::Rendered {
                            text: format!("Module {}", i + 1),
                            ..Default::default()
                        }),
                    );
                    let cfg = Arc::make_mut(&mut self.app.cfg);
                    cfg.widgets.push(w);
                    cfg.bar.as_mut().unwrap().zones[2].push(BarItem::Widget(id));
                }
            }
        }
        if matches!(
            SCENES[self.scene],
            "overview"
                | "menu-fenetre"
                | "bar-selector-relief"
                | "bar-selector-rail"
                | "bar-selector-miniatures"
                | "bar-selector-icones"
                | "bar-selector-notif"
        ) {
            self.demo_window_data();
        }
        if matches!(SCENES[self.scene], "overview" | "menu-fenetre") {
            let win = Id::unique();
            self.app.overview = Some(overview::Overview::new(
                win,
                self.app.monitors[0].name.clone(),
                Fade {
                    from: 1.0,
                    to: 1.0,
                    start: Instant::now(),
                    dur: Duration::ZERO,
                },
            ));
            self.app.sizes.insert(win, self.size);
            self.app.overview.as_mut().unwrap().selected = 1;
            // Une fenêtre en cours de glissement, saisie dans la carte 0.
            self.app.overview.as_mut().unwrap().press = Some(overview::Press {
                card: 0,
                window: Some(overview::Win {
                    address: "0x6".into(),
                    class: "kitty".into(),
                    title: "cargo run — zsh".into(),
                }),
                start: Point::new(420.0, 300.0),
                title: false,
            });
            self.app.cursor.insert(win, Point::new(560.0, 560.0));
        }
        if SCENES[self.scene] == "menu-fenetre" {
            // Clic droit sur une fenêtre de l'aperçu des espaces : son menu,
            // celui de la barre (HIG MENU-06).
            if let Some(o) = self.app.overview.as_mut() {
                o.press = None;
            }
            self.app.workspaces = (1..=3)
                .map(|id| crate::hypr::Workspace {
                    id,
                    name: format!("Projet {id}"),
                    monitor: self.app.monitors[0].name.clone(),
                    windows: 1,
                    fullscreen: false,
                })
                .collect();
            let mut m = super::context::IconMenu::new(
                super::context::MenuTarget::Window(super::context::WindowRef {
                    address: "0x6".into(),
                    state: serde_json::json!({
                        "address": "0x6",
                        "class": "kitty",
                        "floating": false,
                        "pinned": false,
                        "fullscreen": 0,
                        "workspace": { "id": 1, "name": "1" }
                    }),
                    same_app: 1,
                }),
                Id::unique(),
            );
            m.monitor = self.app.monitors[0].name.clone();
            m.hover = Some(0);
            // Icônes résolues tout de suite (pas de tâche dans l'aperçu).
            for name in super::menu::icons(&self.app.icon_menu_lines(&m)) {
                if !self.app.named_icons.contains_key(&name) {
                    let path = launch::lookup_icon(&name, None, 32)
                        .or_else(|| launch::lookup_icon(&name, Some("Adwaita"), 32));
                    let gen_ = self.app.icons_gen;
                    let _ = self.app.update_inner(Msg::IconResolved(gen_, name, path));
                }
            }
            self.app.icon_menu = Some(m);
        }
        if matches!(
            SCENES[self.scene],
            "bar-selector-relief"
                | "bar-selector-rail"
                | "bar-selector-miniatures"
                | "bar-selector-icones"
                | "bar-selector-notif"
        ) {
            let mode = match SCENES[self.scene] {
                "bar-selector-relief" => crate::config::WsSelector::Relief,
                "bar-selector-miniatures" | "bar-selector-notif" => {
                    crate::config::WsSelector::Miniatures
                }
                "bar-selector-icones" => crate::config::WsSelector::Icones,
                _ => crate::config::WsSelector::Rail,
            };
            Arc::make_mut(&mut self.app.cfg).general.workspace_selector = mode;
            // Une notification d'une application de l'espace 2 : le bord de la
            // miniature passe à la couleur d'alerte (comme sur la maquette).
            // L'icône « Éditeur » du bureau (posée en fausse fenêtre sur
            // l'espace 1, même classe `code`) ferait tousser l'espace 1.
            if SCENES[self.scene] == "bar-selector-notif" {
                self.app
                    .clients
                    .retain(|c| !(c.workspace == "1" && c.class == "code"));
                self.app.toasts.list.push(toasts::Toast {
                    n: crate::notifications::Notification {
                        action_icons: Vec::new(),
                        id: 1,
                        app: "Visual Studio Code".into(),
                        icon: String::new(),
                        summary: "Compilation terminée".into(),
                        body: "0 erreur, 2 avertissements (2,4 s).".into(),
                        actions: Vec::new(),
                        urgency: crate::notifications::Urgency::Normal,
                        timeout_ms: -1,
                        resident: false,
                        transient: false,
                        desktop_entry: Some("code".into()),
                        category: None,
                        // Entre par la liste des bulles, pas par l'historique.
                        unread: false,
                    },
                    since: Instant::now(),
                    request: None,
                    alert: false,
                });
            }
        }

        // Panorama isolé : les vraies vues, sans surface Wayland ni navigateur.
        if SCENES[self.scene].starts_with("panorama-") {
            use crate::panorama::config::Layout;
            for name in [
                "media-skip-backward-symbolic",
                "media-skip-forward-symbolic",
                "media-playback-start-symbolic",
                "media-playback-pause-symbolic",
                "go-down-symbolic",
                "window-close-symbolic",
            ] {
                let icon = self.app.builtin_icon(name);
                self.app.named_icons.insert(name.into(), icon);
            }
            let cfg = Arc::make_mut(&mut self.app.cfg);
            cfg.general.panorama = Default::default();
            cfg.general.panorama.sleep_minutes = 0;
            let veil = SCENES[self.scene] == "panorama-video-veil";
            cfg.general.panorama.layout = if SCENES[self.scene] == "panorama-split" || veil {
                Layout::Split3
            } else {
                Layout::Single
            };
            cfg.general.panorama.set_panes(
                Layout::Split3,
                if veil {
                    vec!["clock".into(), "video".into(), "system".into()]
                } else {
                    vec!["media".into(), "clock".into(), "system".into()]
                },
            );
            cfg.general.transition = if SCENES[self.scene] == "panorama-still" {
                config::Transition::None
            } else {
                config::Transition::Fade
            };
            self.app.panorama.preview = Some(panorama::Preview {
                win: self.desktop,
                virtual_size: (self.size.width as u32, self.size.height as u32),
                scale: 1.0,
            });
            self.app.sizes.insert(self.desktop, self.size);
            let page = match SCENES[self.scene] {
                "panorama-system" => 1,
                "panorama-clock" => 2,
                "panorama-video" => 3,
                _ => 0,
            };
            self.app
                .panorama
                .page
                .insert(panorama::PREVIEW.into(), page);
            self.app.panorama.media = Some(crate::widgets::media::Status {
                player: "Spotify".into(),
                playing: true,
                title: "Midnight City".into(),
                artist: "M83".into(),
                album: "Hurry Up, We're Dreaming".into(),
                position: Some(127),
                length: Some(244),
                ..Default::default()
            });
            // Vidéo posée et lecture en cours : le voile entoure sa zone.
            self.app.panorama.video = veil.then(|| panorama::preview_video("Arte"));
            // Mesures fictives, écrites comme le relevé réel (widgets::system) :
            // unités et libellés dans la langue de l'interface.
            let gib = match crate::i18n::current() {
                crate::i18n::Lang::Fr => "Gio",
                crate::i18n::Lang::En => "GiB",
            };
            self.app.panorama.sys = [
                ("CPU".to_string(), "12 %".to_string(), Some(0.12)),
                ("RAM".into(), format!("6.4 {gib} / 32.0 {gib}"), Some(0.2)),
                ("/".into(), format!("128.0 {gib} / 512.0 {gib}"), Some(0.25)),
                (
                    crate::i18n::tr("Charge").into(),
                    "0.42 0.38 0.35".into(),
                    None,
                ),
                (crate::i18n::tr("Allumé").into(), "3 h 24".into(), None),
            ]
            .into_iter()
            .map(|(label, value, ratio)| crate::widgets::SysLine {
                label,
                value,
                ratio,
            })
            .collect();
            let preset = SCENES[self.scene].trim_start_matches("panorama-");
            if crate::panorama::design::PRESETS.contains(&preset) {
                let c = &mut Arc::make_mut(&mut self.app.cfg).general.panorama;
                c.design.apply_preset(preset).unwrap();
                c.layout = Layout::Dashboard;
                for line in &self.app.panorama.sys {
                    if let Some(v) = line.ratio {
                        self.app.panorama.history.insert(
                            line.label.clone(),
                            (0..60)
                                .map(|n| (v + ((n as f32 * 0.31).sin() * 0.08)).clamp(0.0, 1.0))
                                .collect(),
                        );
                    }
                }
            }
            for shortcut in &self.app.cfg.general.panorama.design.shortcuts.clone() {
                let icon = self.app.builtin_icon(&shortcut.icon);
                self.app.named_icons.insert(shortcut.icon.clone(), icon);
            }
            // Webradio : profil Musique, onglet Webradio, une radio qui se
            // connecte et une recherche par genre (Panorama et Réglages).
            if matches!(
                SCENES[self.scene],
                "panorama-webradio" | "panorama-config-lecteur"
            ) {
                let c = &mut Arc::make_mut(&mut self.app.cfg).general.panorama;
                c.layout = Layout::Dashboard;
                c.design.shortcut_profile = "audio".into();
                c.radios = crate::panorama::radio::default_radios();
                c.webradio = "tunein".into();
                let p = &mut self.app.panorama;
                p.music_tab = "radio";
                p.sources_kind = "radio";
                p.radio_query = "jazz".into();
                p.radio = Some((c.radios[1].name.clone(), c.radios[1].url.clone()));
                p.radio_connecting = true;
                p.radio_results = Some((
                    "#jazz".into(),
                    [
                        "101 Smooth Jazz",
                        "Jazz Radio",
                        "BBC Radio 6 Music",
                        "Radio Swiss Jazz",
                    ]
                    .iter()
                    .enumerate()
                    .map(|(i, n)| crate::panorama::config::Source {
                        name: n.to_string(),
                        url: format!("https://stream.example/jazz-{i}.mp3"),
                        ..Default::default()
                    })
                    .collect(),
                ));
            }
            if let Some(tab) = SCENES[self.scene].strip_prefix("panorama-config-") {
                self.app.subtabs.insert(
                    "panorama",
                    match tab {
                        "shortcuts" => "raccourcis",
                        "composition" => "pages",
                        // Lecteur, sous-vue Vidéo : sortie du plein écran, voile.
                        "video" => "lecteur",
                        t => t,
                    },
                );
            }
        }
        // Touche de volume : la carte d'affichage à l'écran.
        if SCENES[self.scene] == "osd" {
            let _ = self.app.show_osd(crate::osd::Kind::Volume, 0.62, false);
        }
        // Les scènes statiques sont des poses finales. Un rendu logiciel lent
        // ne doit pas transformer leur capture en une image d'apparition.
        // Les gestes suivants partent de cette pose puis jouent leurs vrais fades.
        if self.output.is_some()
            && let Some(p) = self.app.popover.as_mut()
        {
            let progress = if SCENES[self.scene] == "bar-convex-opening" {
                0.5
            } else {
                1.0
            };
            p.visibility = Fade {
                from: progress,
                to: progress,
                start: Instant::now(),
                dur: Duration::ZERO,
            };
        }
    }

    /// Menu d'une application du dock (MLG-307) : ses fenêtres, ses actions
    /// `.desktop`, l'épingler, la fermer.
    fn menu_dock_app() -> super::context::IconMenu {
        let mut m = super::context::IconMenu::new(
            super::context::MenuTarget::App(super::context::AppRef {
                id: Some("firefox".into()),
                class: Some("firefox".into()),
                name: "Firefox".into(),
            }),
            iced::window::Id::unique(),
        );
        m.entry = crate::launch::parse_desktop_entry(
            "[Desktop Entry]\nName=Firefox\nExec=firefox %u\n\
             Actions=new-window;new-private-window;\n\
             [Desktop Action new-window]\nName=Nouvelle fenêtre\nExec=firefox --new-window %u\n\
             [Desktop Action new-private-window]\nName=Nouvelle fenêtre de navigation privée\n\
             Exec=firefox --private-window %u\n",
            "fr_FR",
        );
        m.hover = Some(3);
        m
    }

    fn demo_controls(&mut self) {
        let Some(dashboard) = self.app.popover.as_mut().and_then(|p| p.dashboard.as_mut()) else {
            return;
        };
        for i in 0..24 {
            let wave = (i as f32 * 1.15).sin();
            let ratio = (i as f32 * 0.47).sin() * 0.10 + wave * 0.035;
            dashboard.update(popover::continuity::Sample {
                media: Some(crate::widgets::media::Status {
                    title: "Night Walk".into(),
                    artist: "Kudasai".into(),
                    playing: true,
                    position: Some(84),
                    length: Some(237),
                    ..Default::default()
                }),
                system: [
                    ("CPU", "28 %", 0.28 + ratio),
                    ("GPU", "41 %", 0.41 - ratio * 0.5),
                    ("RAM", "6,7 / 16 Gio", 0.42),
                ]
                .into_iter()
                .map(|(label, value, ratio)| widgets::SysLine {
                    label: label.into(),
                    value: value.into(),
                    ratio: Some(ratio),
                })
                .collect(),
                network: Some((12_400_000.0 + 2_400_000.0 * wave, 2_100_000.0)),
            });
        }
        if SCENES[self.scene] != "bar-controls" {
            return;
        }
        let cfg = Arc::make_mut(&mut self.app.cfg);
        for (type_id, value) in [
            ("meteo", widgets::Value::Text("Paris · 8 °C".into())),
            (
                "battery",
                widgets::Value::Rich(config::Rendered {
                    text: "76 %".into(),
                    ..Default::default()
                }),
            ),
        ] {
            let id = format!("preview-module-controls-{type_id}");
            let mut source = widget(&id, WidgetKind::Calendar, Anchor::TopLeft);
            source.type_id = type_id.into();
            source.in_bar = true;
            cfg.widgets.push(source);
            self.app.values.insert(id, value);
        }
        self.app.toasts.history = [
            (u32::MAX - 1, "Mise à jour disponible", "Hyprdesk"),
            (u32::MAX, "Capture enregistrée", "Hyprland"),
        ]
        .into_iter()
        .map(|(id, title, app)| {
            crate::notifications::build(
                id,
                app,
                "",
                crate::i18n::tr(title),
                "",
                &[],
                &Default::default(),
                -1,
            )
        })
        .collect();
    }

    /// Fenêtres de démonstration sur les espaces 1 à 3 : clients (icônes
    /// d'activité) et géométries (miniatures du sélecteur, écran d'organisation).
    #[allow(clippy::type_complexity)]
    fn demo_window_data(&mut self) {
        let mon = self.app.monitors[0].name.clone();
        let wins: [(&str, &str, &str, &str, (i32, i32, i32, i32), bool); 6] = [
            (
                "0x1",
                "firefox",
                "The Rust Programming Language — Mozilla Firefox",
                "1",
                (0, 44, 700, 856),
                false,
            ),
            (
                "0x2",
                "discord",
                "Hyprdesk · #général — Discord",
                "1",
                (700, 44, 740, 856),
                false,
            ),
            (
                "0x3",
                "code",
                "main.rs — hyprdesk — Visual Studio Code",
                "2",
                (0, 44, 960, 856),
                false,
            ),
            (
                "0x4",
                "kitty",
                "hyprdesk on  main — zsh",
                "2",
                (1010, 180, 400, 260),
                true,
            ),
            (
                "0x5",
                "gimp",
                "Affiche — GIMP",
                "3",
                (0, 44, 860, 856),
                false,
            ),
            (
                "0x6",
                "kitty",
                "cargo run — zsh",
                "3",
                (880, 44, 560, 420),
                true,
            ),
        ];
        for (i, (addr, class, title, ws, rect, floating)) in wins.into_iter().enumerate() {
            self.app.clients.push(Client {
                address: addr.into(),
                class: class.into(),
                title: title.into(),
                workspace: ws.into(),
                monitor: mon.clone(),
            });
            self.app.ws_geos.push(hypr::WindowGeo {
                address: addr.into(),
                class: class.into(),
                title: title.into(),
                workspace: ws.into(),
                floating,
                rect,
            });
            // Une icône plausible pour chaque classe (les images de bureau
            // résolues de l'aperçu, prises en rond).
            if !self.app.icon_imgs.is_empty() {
                let img = self.app.icon_imgs[i % self.app.icon_imgs.len()];
                self.app.class_imgs.entry(class.into()).or_insert(img);
            }
        }
        for w in self.app.workspaces.iter_mut() {
            w.windows = self
                .app
                .ws_geos
                .iter()
                .filter(|g| g.workspace == w.name)
                .count() as u32;
        }
    }

    fn update(&mut self, msg: PreviewMsg) -> Task<PreviewMsg> {
        match msg {
            PreviewMsg::BarGesture => {
                match SCENES[self.scene] {
                    "bar-hover" => self.app.bar_preview = Some((6, false)),
                    "bar-pressed" => self.app.bar_preview = Some((6, true)),
                    "bar-workspace-switch" => self.app.monitors[0].workspace = "3".into(),
                    "bar-graphite-pressed" => self.app.bar_preview = Some((6, true)),
                    "bar-graphite-switch" => self.app.monitors[0].workspace = "3".into(),
                    "bar-island-pressed" | "bar-convex-pressed" => {
                        self.app.bar_preview = Some((6, true))
                    }
                    "bar-island-switch" | "bar-convex-switch" => {
                        self.app.monitors[0].workspace = "3".into()
                    }
                    "bar-menu-switch" => {
                        let _ =
                            self.app
                                .open_popover(self.bar, "volume".into(), popover::Kind::Volume);
                    }
                    "bar-menu-closing" => {
                        let _ = self.app.close_popover();
                    }
                    _ => {}
                }
                Task::none()
            }
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
            Msg::Key(_, key)
                if self
                    .app
                    .popover
                    .as_ref()
                    .is_some_and(|p| p.dashboard.is_some()) =>
            {
                popover::continuity::handle_key(&key, self.app.mods.shift())
                    .map(|m| PreviewMsg::Ui(Box::new(m)))
            }
            Msg::Modifiers(modifiers) => {
                self.app.mods = modifiers;
                Task::none()
            }
            Msg::Panorama(m) => {
                use panorama::PMsg;
                match m {
                    m @ (PMsg::Swipe(..) | PMsg::Goto(..) | PMsg::Pressed) => {
                        return self
                            .app
                            .panorama_update(m)
                            .map(|m| PreviewMsg::Ui(Box::new(m)));
                    }
                    PMsg::Layout(layout) => {
                        Arc::make_mut(&mut self.app.cfg).general.panorama.layout = layout
                    }
                    PMsg::Pane(i, d) => {
                        let c = &mut Arc::make_mut(&mut self.app.cfg).general.panorama;
                        let ids = crate::panorama::PAGES;
                        let cur =
                            ids.iter().position(|(id, _)| *id == c.pane(i)).unwrap_or(0) as i32;
                        let next = ids[(cur + d).rem_euclid(ids.len() as i32) as usize].0;
                        let l = c.layout;
                        c.set_pane(l, i, next);
                    }
                    PMsg::Control("PlayPause") => {
                        if let Some(m) = self.app.panorama.media.as_mut() {
                            m.playing = !m.playing;
                        }
                    }
                    _ => {}
                }
                Task::none()
            }
            Msg::BarDragEnd | Msg::Released if self.app.bar_drag.is_some() => {
                let drag = self.app.bar_drag;
                if self.app.finish_bar_drag()
                    && let Some(((fz, fi), Some((tz, ti)))) = drag
                    && let Some(bar) = Arc::make_mut(&mut self.app.cfg).bar.as_mut()
                {
                    let zones = if fz < 3 {
                        &mut bar.zones
                    } else {
                        bar.portrait.get_or_insert_with(|| bar.zones.clone())
                    };
                    let item = zones[fz % 3].remove(fi);
                    let i = if fz == tz && ti > fi { ti - 1 } else { ti };
                    zones[tz % 3].insert(i, item);
                }
                Task::none()
            }
            m @ (Msg::BarDragStart(..) | Msg::BarDragOver(..) | Msg::BarDragLeave(..)) => self
                .app
                .update_inner(m)
                .map(|m| PreviewMsg::Ui(Box::new(m))),
            Msg::BarDragCancel => {
                if self.app.bar_drag.take().is_none() {
                    let _ = self.app.close_popover();
                }
                Task::none()
            }
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
            Msg::OpenPopover(_, key, request) => {
                if self
                    .app
                    .popover
                    .as_ref()
                    .is_some_and(|p| p.key == key && !p.closing())
                {
                    let _ = self.app.close_popover();
                    return Task::none();
                }
                let kind = match request {
                    popover::Request::Volume => popover::Kind::Volume,
                    popover::Request::Brightness => popover::Kind::Brightness,
                    popover::Request::Calendar => popover::Kind::Calendar,
                    popover::Request::Widget(id) => popover::Kind::Widget(id),
                    popover::Request::Workspaces => popover::Kind::Workspaces,
                    popover::Request::Overflow => {
                        popover::Kind::Overflow(self.app.bar_overflow_items(self.bar))
                    }
                    popover::Request::LuaMenu { widget, .. } => popover::Kind::Menu {
                        entries: vec![config::MenuEntry::Header(widget)],
                        source: popover::MenuSource::Lua {
                            widget: "preview".into(),
                        },
                    },
                    _ => return Task::none(),
                };
                let _ = self.app.open_popover(self.bar, key, kind);
                Task::none()
            }
            Msg::On(_, inner) => self.update_ui(*inner),
            Msg::Control(command) => {
                let mode = match command {
                    control::Command::Show => DesktopMode::FullDesktop,
                    control::Command::Hide => DesktopMode::NoDesktop,
                    control::Command::LauncherOnly => DesktopMode::LauncherOnly,
                    control::Command::Notifications(action) => {
                        self.app.toasts.command(&action);
                        return Task::none();
                    }
                    _ => return Task::none(),
                };
                self.app
                    .ws_modes
                    .insert(self.app.monitors[0].workspace.clone(), mode);
                Task::none()
            }
            Msg::BarWorkspace(_, name) => {
                self.app.monitors[0].workspace = name;
                let _ = self.app.close_popover();
                Task::none()
            }
            Msg::VolumeSet(value) => {
                self.app.volume = Some((value, false));
                Task::none()
            }
            Msg::VolumeMute => {
                if let Some((_, muted)) = self.app.volume.as_mut() {
                    *muted = !*muted;
                }
                Task::none()
            }
            Msg::MicSet(value) => {
                if let Some((volume, _)) = self.app.audio.mic.as_mut() {
                    *volume = value;
                }
                Task::none()
            }
            Msg::MicMute => {
                if let Some((_, muted)) = self.app.audio.mic.as_mut() {
                    *muted = !*muted;
                }
                Task::none()
            }
            Msg::Media(_, "PlayPause") => {
                if let Some(media) = self
                    .app
                    .popover
                    .as_mut()
                    .and_then(|p| p.dashboard.as_mut())
                    .and_then(|dashboard| dashboard.media.as_mut())
                {
                    media.playing = !media.playing;
                }
                Task::none()
            }
            Msg::LightSet(target, value) => {
                self.app.light.set_level(target, value);
                Task::none()
            }
            Msg::AudioDefault(id) => {
                for device in &mut self.app.audio.outputs {
                    device.default = device.id == id;
                }
                Task::none()
            }
            Msg::ControlToggle(id) => {
                if let Some(widgets::Value::Rich(r)) = self.app.values.get_mut(&id) {
                    r.toggled = r.toggled.map(|v| !v);
                    r.tooltip = Some(
                        if r.toggled == Some(true) {
                            "Activé"
                        } else {
                            "Désactivé"
                        }
                        .into(),
                    );
                }
                Task::none()
            }
            Msg::Search(super::search::SMsg::Close) => {
                let _ = self.app.close_search();
                Task::none()
            }
            Msg::Search(super::search::SMsg::Query(q)) if !q.starts_with(':') => self
                .app
                .search_update(super::search::SMsg::Query(q))
                .map(|m| PreviewMsg::Ui(Box::new(m))),
            m @ (Msg::ConvexHover(..)
            | Msg::ConvexHide(..)
            | Msg::ConvexWake(..)
            | Msg::ConvexClose(..)
            | Msg::PopoverShown(..)
            | Msg::MotionSettled
            | Msg::Cursor(..)) => self
                .app
                .update_inner(m)
                .map(|m| PreviewMsg::Ui(Box::new(m))),
            Msg::Search(super::search::SMsg::Shown) => self
                .app
                .search_update(super::search::SMsg::Shown)
                .map(|m| PreviewMsg::Ui(Box::new(m))),
            Msg::Toasts(m) => self
                .app
                .toasts_update(m)
                .map(|m| PreviewMsg::Ui(Box::new(m))),
            Msg::ClosePopover => {
                let _ = self.app.close_popover();
                Task::none()
            }
            m @ (Msg::BarAnchor(..) | Msg::BarLayout(..) | Msg::CalendarMonth(_)) => self
                .app
                .update_inner(m)
                .map(|m| PreviewMsg::Ui(Box::new(m))),
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
                let origin = self
                    .app
                    .launcher_screen_anchor(m, iced::Rectangle::default())
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
            | Msg::LauncherPopupShown(..)
            | Msg::LauncherShown(_)
            | Msg::LauncherLaunchExpired(..)) => self
                .app
                .update_inner(m)
                .map(|m| PreviewMsg::Ui(Box::new(m))),
            Msg::LauncherActivate(inner) => self.update_ui(*inner),
            Msg::LauncherLaunch(id) => self
                .app
                .launcher_launch_started(id)
                .map(|m| PreviewMsg::Ui(Box::new(m))),
            Msg::PopoverMeasured(win, height) => self
                .app
                .fit_popover(win, height)
                .map(|m| PreviewMsg::Ui(Box::new(m))),
            Msg::Chat(chat::ChatMsg::NewConversation) => {
                if let Some(panel) = self.app.chat.as_mut() {
                    let mut fresh = chat::Conversation::new(
                        panel.convs.iter().map(|c| c.session).max().unwrap_or(0) + 1,
                        Vec::new(),
                        crate::tr!("Conversation {n}", n = panel.convs.len() + 1),
                    );
                    fresh.ready = true;
                    fresh.provider = panel.cur().provider.clone();
                    fresh.providers = panel.cur().providers.clone();
                    let i = panel.push(fresh);
                    panel.active = i;
                }
                Task::none()
            }
            Msg::Chat(chat::ChatMsg::Close) => {
                self.app.chat = None;
                Task::none()
            }
            Msg::Chat(
                m @ (chat::ChatMsg::Input(_)
                | chat::ChatMsg::Suggest(_)
                | chat::ChatMsg::Switch(_)
                | chat::ChatMsg::ToggleSteps(_)
                | chat::ChatMsg::Select(_)
                | chat::ChatMsg::EditorAction(_)
                | chat::ChatMsg::Provider(_)),
            ) => self.app.chat_update(m).map(|m| PreviewMsg::Ui(Box::new(m))),
            Msg::Frame => self
                .app
                .update_inner(Msg::Frame)
                .map(|m| PreviewMsg::Ui(Box::new(m))),
            _ => Task::none(),
        }
    }

    fn view(&self) -> Element<'_, PreviewMsg> {
        if SCENES[self.scene].starts_with("panorama-")
            && !SCENES[self.scene].starts_with("panorama-config-")
        {
            return self
                .app
                .view_panorama(self.desktop)
                .map(|m| PreviewMsg::Ui(Box::new(m)));
        }
        // La barre prend sa hauteur et sa marge réelles : une lame flottante
        // (Graphite) montre son jour au-dessus du bord.
        let bar_def = self.app.cfg.bar.as_ref();
        let bh = bar_def.map(|b| b.height as f32).unwrap_or(44.0);
        let bm = bar_def.map(|b| b.margin as f32).unwrap_or(0.0);
        let bottom = self.app.bar_surfaces[&self.bar].position == BarPosition::Bottom;
        let mut layers: Vec<Element<'_, Msg>> = vec![
            self.app.view(self.desktop),
            pin(container(self.app.view_bar(self.bar))
                .height(bh)
                .width(self.size.width))
            .y(if bottom {
                self.size.height - bh - bm
            } else {
                bm
            })
            .into(),
        ];
        // Le lanceur a sa propre surface : par-dessus le bureau, sous la barre.
        // Bureau masqué : le dock n'existe plus.
        if let Some(m) = self.app.monitors.first()
            && SCENES[self.scene] != "mode-hidden"
            && !self.app.launcher_in_bar(&m.name)
        {
            // Le compositeur tient compte de la barre avant de placer le dock.
            layers.insert(
                1,
                container(self.app.view_launcher(m, 1.0))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(Padding {
                        top: if self.app.bar_surfaces[&self.bar].position == BarPosition::Top {
                            44.0
                        } else {
                            0.0
                        },
                        bottom: if self.app.bar_surfaces[&self.bar].position == BarPosition::Bottom
                        {
                            44.0
                        } else {
                            0.0
                        },
                        ..Padding::ZERO
                    })
                    .into(),
            );
        }
        if self.app.convex_options().is_some() {
            layers.push(self.app.view_convex_frame());
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
                    top: if bottom { 0.0 } else { bh },
                    bottom: if bottom { bh } else { 0.0 },
                    ..iced::Padding::ZERO
                })
                .into(),
            );
        }
        if let Some(p) = &self.app.popover {
            layers.push(
                pin(container(self.app.view_catcher())
                    .width(self.size.width)
                    .height(self.size.height - 44.0))
                .y(if p.position == BarPosition::Top {
                    44.0
                } else {
                    0.0
                })
                .into(),
            );
            layers.push(
                pin(container(self.app.view_popover())
                    .width(p.size.width)
                    .height(p.size.height))
                .x(p.left)
                .y(if let Some(a) = p.attached {
                    a.top
                } else if p.position == BarPosition::Top {
                    bh + popover::GAP - popover::SHADOW
                } else {
                    self.size.height - 44.0 - popover::GAP + popover::SHADOW - p.size.height
                })
                .into(),
            );
        }
        // Écran de connexion (greetd), par-dessus tout : application à part.
        if SCENES[self.scene] == "connexion" {
            layers.push(crate::greeter_ui::preview_view().map(|_| Msg::Noop));
        }
        if self.app.overview.is_some() {
            layers.push(
                container(self.app.view_overview())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .into(),
            );
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
        if self.app.wizard.is_some() {
            layers.push(
                container(self.app.view_wizard())
                    .width(Length::Fill)
                    .height(Length::Fill)
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
        if !self.app.toasts.list.is_empty() || self.app.toasts.win.is_some() {
            if self.app.convex_options().is_some() {
                layers.push(
                    container(container(self.app.view_toasts()).width(452).height(196))
                        .width(Length::Fill)
                        .align_x(iced::Alignment::Center)
                        .into(),
                );
            } else {
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
        }
        if self.app.mfa_dialog.is_some() {
            layers.push(
                container(self.app.view_mfa_dialog())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .into(),
            );
        }
        if self.app.mfa_fenetre.is_some() {
            layers.push(
                container(self.app.view_mfa_fenetre())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
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

    /// Assistant de configuration à l'étape `step` : clavier, fonds, applications
    /// et veille fictifs (rien n'est lu ni écrit sur la machine, sauf les
    /// images livrées et les icônes du thème).
    fn wizard_scene(&mut self, step: usize) {
        let win = Id::unique();
        self.app.sizes.insert(win, self.size);
        let app = |id: &str, name: &str, icon: &str, cat: &str| crate::launch::AppInfo {
            id: id.into(),
            name: name.into(),
            icon: Some(icon.into()),
            wm_class: None,
            mime_types: Vec::new(),
            categories: vec![cat.into()],
            available: true,
        };
        self.app.apps = Arc::new(vec![
            app("kitty", "kitty", "kitty", "TerminalEmulator"),
            app("Alacritty", "Alacritty", "Alacritty", "TerminalEmulator"),
            app(
                "org.gnome.Console",
                "Console",
                "org.gnome.Console",
                "TerminalEmulator",
            ),
            app("firefox", "Firefox", "firefox", "WebBrowser"),
            app("chromium", "Chromium", "chromium", "WebBrowser"),
            app("brave-browser", "Brave", "brave-browser", "WebBrowser"),
            app(
                "org.gnome.Nautilus",
                "Fichiers",
                "org.gnome.Nautilus",
                "FileManager",
            ),
            app("thunar", "Thunar", "org.xfce.thunar", "FileManager"),
        ]);
        for a in self.app.apps.clone().iter() {
            let p = crate::launch::icon_for_class(&a.id, &self.app.apps, None, 48);
            let img = self.app.handle_for(p.as_deref());
            self.app.class_imgs.insert(a.id.clone(), img);
        }
        // État d'ouverture identique pour chaque scène (les choix d'une scène
        // précédente ne doivent pas passer pour l'état d'avant).
        let cfg = Arc::make_mut(&mut self.app.cfg);
        cfg.general.apps.remove("terminal");
        cfg.general.idle = Default::default();
        let mut w = wizard::Wizard::new(win, self.app.settings.clone(), self.app.cfg.clone());
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/screenshots");
        w.walls = [
            "Turquoise Twilight Over a Futuristic City.png",
            "Turquoise Anime Girl at Sunset Overlook.png",
            "Teal Siren Over a Neon Spring City.png",
        ]
        .iter()
        .map(|n| root.join(n))
        .collect();
        for p in &w.walls {
            if let Some(h) = wizard::scene_thumb(p) {
                w.thumbs.insert(p.clone(), h);
            }
        }
        let first = w.walls[0].clone();
        let fit = self.app.current_fit(&Selector::default());
        self.app
            .settings
            .set_wallpaper(Selector::default(), WallSource::Path(first), fit);
        let inputs = crate::inputs::Inputs {
            kb_layout: "fr,us".into(),
            ..Default::default()
        };
        w.kb = wizard::Keyboard {
            layouts: [
                ("fr", "French"),
                ("be", "Belgian"),
                ("us", "English (US)"),
                ("gb", "English (UK)"),
                ("de", "German"),
                ("ch", "Switzerland"),
                ("es", "Spanish"),
                ("it", "Italian"),
            ]
            .iter()
            .map(|(c, d)| (c.to_string(), d.to_string()))
            .collect(),
            before: Some(inputs.clone()),
            cur: Some(inputs),
            try_text: "Bonjour, à bientôt !".into(),
        };
        for i in 0..step {
            w.go(i);
        }
        w.go(step);
        self.app.wizard = Some(w);
        // Quelques choix déjà faits : le récapitulatif les marque, la veille
        // montre l'avertissement (verrouillage après l'extinction).
        let cfg = Arc::make_mut(&mut self.app.cfg);
        cfg.general.apps.insert("terminal".into(), "kitty".into());
        cfg.general.idle = config::IdleCfg {
            manage: true,
            dim: 240,
            lock: 480,
            dpms: 360,
            suspend: 1800,
            ..Default::default()
        };
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
        let bottom = plugins::dock::thickness(g.launcher.size) as f32 + 16.0;
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
                container(text(title).size(crate::ui::tokens::ts(13.0)))
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
