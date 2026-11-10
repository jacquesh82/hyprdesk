//! Écran Réglages (⚙) : un panneau par-dessus le bureau, sur l'écran actif.
//!
//! Chaque changement modifie `App::settings`, puis est enregistré dans
//! `hyprdesk/settings.lua` (Lua généré) et rechargé comme n'importe quelle
//! modification de config. Les curseurs prévisualisent pendant le glissé et
//! n'enregistrent qu'au relâchement.

use std::sync::Arc;

use iced::widget::{
    Space, button, column, container, mouse_area, row, rule, scrollable, text, tooltip,
};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Task};

use super::plugins::{self, PluginMsg};
use super::{App, Msg, color};
use crate::config::Rgba;
use crate::launch::{self};

/// Onglet du tiroir : l'identifiant d'un paquet de réglages (`plugins::BUILTIN`).
pub type Tab = &'static str;

/// Messages du tiroir : ceux de la coquille, et ceux des paquets (enveloppés).
#[derive(Debug, Clone)]
pub enum PanelMsg {
    /// Affiche le paquet d'identifiant donné.
    Tab(Tab),
    Close,
    /// Fin d'un glissé de curseur : on enregistre.
    Commit,
    /// Message d'un paquet de réglages.
    Plugin(PluginMsg),
}

pub struct Panel {
    /// Surface du bureau de l'écran visé (contexte : écran, workspace, aperçu).
    pub win: Id,
    /// Surface propre du tiroir, au-dessus des fenêtres.
    pub surface: Id,
    pub tab: Tab,
    // État de chaque paquet intégré.
    pub desktop: plugins::desktop::State,
    pub icons: plugins::icons::State,
    pub wallpaper: plugins::wallpaper::State,
    pub widgets: plugins::widgets::State,
    pub screens: plugins::screens::State,
    pub workspaces: plugins::workspaces::State,
    pub apps: plugins::apps::State,
    pub plugins: plugins::plugins::State,
    pub assistant: plugins::assistant::State,
    pub launcher: plugins::launcher::State,
    pub share: plugins::share::State,
    pub bar: plugins::bar::State,
}

/// Couleurs proposées (palette Dracula PRO), partagées par les paquets.
pub(in crate::ui) const SWATCHES: [Rgba; 8] = [
    Rgba(0x95, 0x80, 0xff, 0xff),
    Rgba(0xff, 0x80, 0xbf, 0xff),
    Rgba(0x80, 0xff, 0xea, 0xff),
    Rgba(0x8a, 0xff, 0x80, 0xff),
    Rgba(0xff, 0xff, 0x80, 0xff),
    Rgba(0xff, 0x95, 0x80, 0xff),
    Rgba(0x22, 0x21, 0x2c, 0xff),
    Rgba(0x00, 0x00, 0x00, 0xff),
];

/// Espace de noms de la surface du tiroir des réglages.
pub const SETTINGS_NAMESPACE: &str = "hyprdesk-settings";

/// Largeur du tiroir des réglages, marge comprise.
pub const DRAWER_W: f32 = 780.0;
const DRAWER_GAP: f32 = 12.0;

impl Panel {
    pub fn new(win: Id) -> Self {
        Panel {
            win,
            surface: Id::unique(),
            tab: plugins::desktop::INFO.id,
            desktop: Default::default(),
            icons: Default::default(),
            wallpaper: Default::default(),
            widgets: Default::default(),
            screens: Default::default(),
            workspaces: Default::default(),
            apps: Default::default(),
            plugins: Default::default(),
            assistant: Default::default(),
            launcher: Default::default(),
            share: Default::default(),
            bar: Default::default(),
        }
    }

    /// Sélectionne un widget et affiche son onglet.
    pub fn select_widget(&mut self, id: &str) {
        self.widgets.widget_sel = Some(id.to_string());
        self.tab = plugins::widgets::INFO.id;
    }
}

impl App {
    /// Ouvre le tiroir des réglages sur l'écran principal, dans sa propre
    /// surface au-dessus des fenêtres : le mode du bureau ne change pas.
    pub(super) fn open_panel(&mut self) -> Task<Msg> {
        // Déjà ouvert : rien à refaire (un nouvel appel ne doit pas le dupliquer).
        if self.panel.is_some() && self.panel_fade.is_none_or(|f| f.to >= 1.0) {
            return Task::none();
        }
        let primary = self.primary_monitor();
        let win = self
            .surfaces
            .iter()
            .find(|(_, n)| Some(*n) == primary.as_ref())
            .or_else(|| self.surfaces.iter().next())
            .map(|(id, _)| *id);
        let Some(win) = win else { return Task::none() };
        // Un seul tiroir à la fois : l'assistant laisse la place aux réglages.
        let chat = self.close_chat();
        Task::batch([chat, self.open_panel_at(win)])
    }

    fn open_panel_at(&mut self, win: Id) -> Task<Msg> {
        // Tiroir en cours de fermeture : on rouvre le même.
        let reuse = self
            .panel
            .as_ref()
            .filter(|p| p.win == win)
            .map(|p| p.surface);
        if let Some(old) = self
            .panel
            .as_ref()
            .filter(|p| p.win != win)
            .map(|p| p.surface)
        {
            self.panel = None;
            self.panel_fade = None;
            return Task::batch([iced::window::close(old), self.open_panel_at(win)]);
        }
        let mut p = Panel::new(win);
        let mut create = Task::none();
        p.surface = match reuse {
            Some(id) => id,
            None => {
                let id = Id::unique();
                let monitor = self.surfaces.get(&win).cloned().unwrap_or_default();
                create = Task::done(Msg::NewLayerShell {
                    settings: iced_layershell::reexport::NewLayerShellSettings {
                        size: Some((self.panel_width() as u32, 0)),
                        layer: iced_layershell::reexport::Layer::Top,
                        anchor: iced_layershell::reexport::Anchor::Top
                            | iced_layershell::reexport::Anchor::Bottom
                            | iced_layershell::reexport::Anchor::Right,
                        exclusive_zone: Some(0),
                        keyboard_interactivity:
                            iced_layershell::reexport::KeyboardInteractivity::OnDemand,
                        output_option: iced_layershell::reexport::OutputOption::OutputName(monitor),
                        namespace: Some(SETTINGS_NAMESPACE.into()),
                        ..Default::default()
                    },
                    id,
                });
                id
            }
        };
        let k = &self.cfg.general.keys;
        p.desktop.keys = [
            k.toggle.clone(),
            k.settings.clone(),
            k.leave.clone(),
            k.launcher.clone(),
            k.hide.clone(),
        ];
        self.panel = Some(p);
        let from = self
            .panel_fade
            .map(|f| f.value(std::time::Instant::now()))
            .unwrap_or(0.0);
        self.panel_fade = Some(super::Fade {
            from,
            to: 1.0,
            start: std::time::Instant::now(),
            dur: self.motion_duration(),
        });
        self.selected = None;
        let themes = Task::perform(
            async {
                tokio::task::spawn_blocking(launch::icon_themes)
                    .await
                    .unwrap_or_default()
            },
            |t| plugins::icons::wrap(plugins::icons::Msg::Themes(t)),
        );
        Task::batch([
            create,
            self.sync_window_gaps(),
            themes,
            Task::perform(
                async {
                    tokio::task::spawn_blocking(launch::font_families)
                        .await
                        .unwrap_or_default()
                },
                |f| plugins::desktop::wrap(plugins::desktop::Msg::Fonts(f)),
            ),
        ])
    }

    /// Applique les réglages à la config effective, tout de suite, sans
    /// repasser par Lua : le bureau à côté du tiroir change en direct.
    pub(in crate::ui) fn apply(&mut self) -> Task<Msg> {
        let next = self.settings.apply_to(&self.cfg);
        self.install_config(Arc::new(next))
    }

    /// Applique puis enregistre settings.lua (sans rechargement : la mémoire
    /// est déjà à jour ; un rechargement déclenché par la surveillance du
    /// fichier retombe sur la même config et ne change rien).
    pub(in crate::ui) fn commit(&mut self) -> Task<Msg> {
        let applied = self.apply();
        Task::batch([applied, self.save()])
    }

    /// Enregistre puis recharge : activer ou retirer un widget demande de
    /// relancer sa configuration (fonctions Lua des plugins).
    pub(in crate::ui) fn commit_reload(&self) -> Task<Msg> {
        self.save().chain(Task::done(Msg::ConfigChanged))
    }

    pub(in crate::ui) fn save(&self) -> Task<Msg> {
        let (s, path) = (self.settings.clone(), self.config_path.clone());
        let revision = crate::storage::revision();
        self.settings_revision
            .store(revision, std::sync::atomic::Ordering::Release);
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    s.save_versioned(&path, revision).map_err(|e| e.to_string())
                })
                .await
                .map_err(|e| e.to_string())?
            },
            |r| match r {
                Ok(_) => Msg::Noop,
                Err(e) => {
                    tracing::error!("enregistrement des réglages : {e}");
                    crate::hypr::notify(
                        4,
                        6000,
                        &format!("hyprdesk : réglages non enregistrés — {e}"),
                    );
                    Msg::Noop
                }
            },
        )
    }

    pub(in crate::ui) fn panel_ctx(&self) -> Option<(String, String)> {
        let p = self.panel.as_ref()?;
        let m = self.monitor(p.win)?;
        Some((m.name.clone(), m.workspace.clone()))
    }

    pub(super) fn panel_update(&mut self, msg: PanelMsg) -> Task<Msg> {
        match msg {
            PanelMsg::Tab(t) => {
                let Some(panel) = self.panel.as_mut() else {
                    return Task::none();
                };
                panel.tab = t;
                plugins::on_show(self, t)
            }
            PanelMsg::Close => self.close_panel(),
            PanelMsg::Commit => self.commit(),
            PanelMsg::Plugin(m) => plugins::update(self, m),
        }
    }

    // -- Vue -----------------------------------------------------------------------

    pub(super) fn panel_width(&self) -> f32 {
        self.panel
            .as_ref()
            .map(|p| {
                let width = self.size_of(p.win).width;
                if width < 1200.0 { width } else { DRAWER_W }
            })
            .unwrap_or(DRAWER_W)
    }

    pub(super) fn view_panel(&self) -> Element<'_, Msg> {
        let Some(p) = &self.panel else {
            return Space::new().into();
        };
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let wide = self.panel_width() >= 640.0;
        // Paquets coupés : absents de la navigation (rallumables dans Plugins).
        let entries = plugins::BUILTIN
            .iter()
            .filter(|i| self.cfg.plugin_on(i.id))
            .map(|i| (i.label, i.icon, i.id));
        let nav = column(entries.map(|(label, icon, tab)| {
            let active = p.tab == tab;
            let glyph: Element<'_, Msg> = super::icons::get(icon)
                .map(|handle| {
                    iced::widget::svg(handle)
                        .width(18)
                        .height(18)
                        .style(move |_, _| iced::widget::svg::Style {
                            color: Some(if active { accent } else { fg }),
                        })
                        .into()
                })
                .unwrap_or_else(|| Space::new().into());
            let mut content = row![glyph].spacing(10).align_y(Alignment::Center);
            if wide {
                content = content.push(
                    text(label)
                        .font(self.font)
                        .size(13)
                        .font(self.ui_font(iced::font::Weight::Medium)),
                );
            }
            button(content)
                .width(Length::Fill)
                .padding([12, 12])
                .on_press(Msg::Panel(PanelMsg::Tab(tab)))
                .style(move |_, status| button::Style {
                    background: if active {
                        Some(Color { a: 0.16, ..accent }.into())
                    } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                        Some(Color { a: 0.07, ..fg }.into())
                    } else {
                        None
                    },
                    text_color: if active { accent } else { fg },
                    border: Border::default().rounded(11),
                    ..Default::default()
                })
                .into()
        }))
        .spacing(4)
        .width(if wide { 156 } else { 46 });
        let (mon, ws) = self.panel_ctx().unwrap_or_default();
        let close_icon = super::icons::get("window-close").unwrap();
        let close = tooltip(
            button(
                iced::widget::svg(close_icon)
                    .width(16)
                    .height(16)
                    .style(move |_, _| iced::widget::svg::Style { color: Some(fg) }),
            )
            .padding(9)
            .on_press(Msg::Panel(PanelMsg::Close))
            .style(button::text),
            text("Fermer · Échap").font(self.font),
            iced::widget::tooltip::Position::Left,
        );
        let header = row![
            column![
                text("HYPRDESK")
                    .font(self.font)
                    .size(10)
                    .color(Color { a: 0.55, ..fg })
                    .font(self.title_font()),
                text("Réglages")
                    .font(self.font)
                    .size(27)
                    .font(self.title_font())
                    .color(fg),
                text(format!("{mon}  ·  Bureau {ws}"))
                    .font(self.font)
                    .size(12)
                    .color(Color { a: 0.65, ..fg }),
            ]
            .spacing(5)
            .width(Length::Fill),
            close,
        ]
        .align_y(Alignment::Center);
        let content = plugins::view(self, p, p.tab);
        let detail = column![
            row![
                container(Space::new().width(5).height(5)).style(move |_| container::Style {
                    background: Some(accent.into()),
                    border: Border::default().rounded(5),
                    ..Default::default()
                }),
                text("Modifications enregistrées automatiquement")
                    .font(self.font)
                    .size(11)
                    .color(Color { a: 0.6, ..fg })
            ]
            .spacing(7)
            .align_y(Alignment::Center),
            scrollable(container(content).padding(iced::Padding {
                right: 14.0,
                ..iced::Padding::ZERO
            }))
            .height(Length::Fill),
        ]
        .spacing(18)
        .width(Length::Fill);
        let card = container(
            column![
                header,
                rule::horizontal(1),
                row![nav, detail].spacing(24).height(Length::Fill)
            ]
            .spacing(22),
        )
        .padding(24)
        .width(self.panel_width() - 2.0 * DRAWER_GAP)
        .height(Length::Fill)
        .style(|theme| super::design::surface(theme, super::design::Surface::Panel));
        let progress = self
            .panel_fade
            .map(|f| f.value(std::time::Instant::now()))
            .unwrap_or(1.0);
        container(
            iced::widget::float(mouse_area(card).on_press(Msg::Noop))
                .translate(move |_, _| iced::Vector::new((1.0 - progress) * 28.0, 0.0)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::End)
        .padding(DRAWER_GAP)
        .into()
    }

    /// Champ : libellé (et avertissement si hyprdesk.lua surcharge la clé), puis le contrôle.
    pub(in crate::ui) fn field<'a>(
        &self,
        label: &'a str,
        key: &str,
        control: impl Into<Element<'a, Msg>>,
    ) -> Element<'a, Msg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let mut head = row![
            text(label)
                .font(self.font)
                .size(14)
                .color(Color { a: 0.85, ..fg })
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        if !key.is_empty() && self.cfg.keys_user.contains(key) {
            head = head.push(
                text("surchargé dans hyprdesk.lua — sans effet ici")
                    .font(self.font)
                    .size(11)
                    .color(Color::from_rgb8(0xff, 0xb8, 0x6c)),
            );
        }
        column![head, control.into()].spacing(6).into()
    }

    pub(in crate::ui) fn section<'a>(&self, title: &'a str) -> Element<'a, Msg> {
        let accent = color(self.cfg.general.theme.accent, 1.0);
        column![
            Space::new().height(8),
            text(title)
                .font(self.font)
                .size(16)
                .font(self.title_font())
                .color(accent)
        ]
        .into()
    }
}
