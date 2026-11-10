//! Écran Réglages (⚙) : un panneau par-dessus le bureau, sur l'écran actif.
//!
//! Chaque changement modifie `App::settings`, puis est enregistré dans
//! `hyprdesk/settings.lua` (Lua généré) et rechargé comme n'importe quelle
//! modification de config. Les curseurs prévisualisent pendant le glissé et
//! n'enregistrent qu'au relâchement.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use iced::widget::{
    Space, button, column, container, image, mouse_area, pick_list, radio, row, rule, scrollable, slider, text, text_input, toggler,
};
use iced::window::Id;
use iced::{Alignment, Background, Border, Color, Element, Length, Task};

use super::{App, Msg, color, list_images};
use crate::config::resolve::resolve;
use crate::config::{Fit, Mode, Origin, Rgba, RunningApps, Selector, ShowOn, Transition, WallSource};
use crate::launch::{self, AppInfo};
use crate::settings::hex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Desktop,
    Icons,
    Wallpaper,
    Effects,
}

/// Portée d'une règle de fond d'écran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    All,
    Workspace,
    Monitor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallKind {
    Image,
    Rotate,
    Color,
}

/// Mode d'un workspace : par défaut, ou imposé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WsMode {
    Default,
    Opaque,
    Glass,
}

#[derive(Debug, Clone)]
pub enum PanelMsg {
    Tab(Tab),
    Close,
    ShowOn(ShowOn),
    Mode(Mode),
    GlassOpacity(f32),
    CloseOnLaunch(bool),
    WorkspaceMode(WsMode),
    Running(RunningApps),
    IconSize(u16),
    CellSize(u16),
    Theme(String),
    AppFilter(String),
    AddToWorkspace(bool),
    AddApp(String),
    RemoveIcon(usize),
    ResetLayout,
    Scope(Scope),
    Folder(String),
    FolderSubmit,
    Images(Vec<PathBuf>),
    Thumb(PathBuf, Option<image::Handle>),
    PickImage(PathBuf),
    Kind(WallKind),
    RotInterval(u64),
    Shuffle(bool),
    Color(Rgba),
    Hex(String),
    HexSubmit,
    Fit(Fit),
    RemoveWall,
    Transition(Transition),
    FadeMs(u64),
    CrossfadeMs(u64),
    Accent(Rgba),
    Themes(Vec<String>),
    /// Fin d'un glissé de curseur : on enregistre.
    Commit,
}

pub struct Panel {
    pub win: Id,
    tab: Tab,
    scope: Scope,
    folder: String,
    images: Vec<PathBuf>,
    thumbs: HashMap<PathBuf, image::Handle>,
    app_filter: String,
    add_to_workspace: bool,
    hex: String,
    themes: Vec<String>,
}

/// Couleurs proposées (palette Dracula PRO).
const SWATCHES: [Rgba; 8] = [
    Rgba(0x95, 0x80, 0xff, 0xff),
    Rgba(0xff, 0x80, 0xbf, 0xff),
    Rgba(0x80, 0xff, 0xea, 0xff),
    Rgba(0x8a, 0xff, 0x80, 0xff),
    Rgba(0xff, 0xff, 0x80, 0xff),
    Rgba(0xff, 0x95, 0x80, 0xff),
    Rgba(0x22, 0x21, 0x2c, 0xff),
    Rgba(0x00, 0x00, 0x00, 0xff),
];

const THUMB: (u32, u32) = (192, 108);
const MAX_IMAGES: usize = 60;

fn default_folder() -> String {
    "~/.wallpapers".into()
}

fn expand(p: &str) -> PathBuf {
    crate::config::lua::expand_path(p, std::path::Path::new("/"))
}

impl Panel {
    pub fn new(win: Id) -> Self {
        Panel {
            win,
            tab: Tab::Desktop,
            scope: Scope::Workspace,
            folder: default_folder(),
            images: Vec::new(),
            thumbs: HashMap::new(),
            app_filter: String::new(),
            add_to_workspace: false,
            hex: String::new(),
            themes: Vec::new(),
        }
    }
}

fn load_images(folder: String) -> Task<Msg> {
    Task::perform(
        async move {
            let dir = expand(&folder);
            tokio::task::spawn_blocking(move || list_images(&dir).into_iter().take(MAX_IMAGES).collect::<Vec<_>>()).await.unwrap_or_default()
        },
        |l| Msg::Panel(PanelMsg::Images(l)),
    )
}

fn load_thumb(p: PathBuf) -> Task<Msg> {
    Task::perform(
        async move {
            let p2 = p.clone();
            let h = tokio::task::spawn_blocking(move || {
                let img = ::image::open(&p2).ok()?.thumbnail(THUMB.0 * 2, THUMB.1 * 2).to_rgba8();
                Some(image::Handle::from_rgba(img.width(), img.height(), img.into_raw()))
            })
            .await
            .ok()
            .flatten();
            (p, h)
        },
        |(p, h)| Msg::Panel(PanelMsg::Thumb(p, h)),
    )
}

impl App {
    /// Ouvre le panneau sur l'écran actif (le bureau doit être affiché).
    pub(super) fn open_panel(&mut self) -> Task<Msg> {
        let focused = self.monitors.iter().find(|m| m.focused).map(|m| m.name.clone());
        let win = self
            .surfaces
            .iter()
            .find(|(id, n)| Some(*n) == focused.as_ref() && self.raised.contains(id))
            .or_else(|| self.surfaces.iter().find(|(id, _)| self.raised.contains(id)))
            .map(|(id, _)| *id);
        let Some(win) = win else { return Task::none() };
        self.panel = Some(Panel::new(win));
        self.selected = None;
        Task::perform(async { tokio::task::spawn_blocking(launch::icon_themes).await.unwrap_or_default() }, |t| {
            Msg::Panel(PanelMsg::Themes(t))
        })
    }

    /// Enregistre settings.lua puis recharge la config.
    fn commit(&self) -> Task<Msg> {
        let (s, path) = (self.settings.clone(), self.config_path.clone());
        Task::perform(
            async move { tokio::task::spawn_blocking(move || s.save(&path).map_err(|e| e.to_string())).await.map_err(|e| e.to_string())? },
            |r| match r {
                Ok(_) => Msg::ConfigChanged,
                Err(e) => {
                    tracing::error!("enregistrement des réglages : {e}");
                    crate::hypr::notify(4, 6000, &format!("hyprdesk : réglages non enregistrés — {e}"));
                    Msg::Noop
                }
            },
        )
    }

    /// Aperçu immédiat d'un réglage général, avant l'enregistrement.
    fn preview(&mut self, f: impl FnOnce(&mut crate::config::General)) {
        f(&mut Arc::make_mut(&mut self.cfg).general);
    }

    fn panel_ctx(&self) -> Option<(String, String)> {
        let p = self.panel.as_ref()?;
        let m = self.monitor(p.win)?;
        Some((m.name.clone(), m.workspace.clone()))
    }

    fn scope_selector(&self, scope: Scope) -> Selector {
        let (mon, ws) = self.panel_ctx().unwrap_or_default();
        match scope {
            Scope::All => Selector::default(),
            Scope::Workspace => Selector { workspaces: Some(vec![ws]), monitors: None },
            Scope::Monitor => Selector { workspaces: None, monitors: Some(vec![mon]) },
        }
    }

    fn current_fit(&self, sel: &Selector) -> Fit {
        self.settings.wallpaper(sel).map(|w| w.fit).unwrap_or_default()
    }

    pub(super) fn panel_update(&mut self, msg: PanelMsg) -> Task<Msg> {
        let Some((scope, folder)) = self.panel.as_ref().map(|p| (p.scope, p.folder.clone())) else { return Task::none() };
        let Some(panel) = self.panel.as_mut() else { return Task::none() };
        match msg {
            PanelMsg::Tab(t) => {
                panel.tab = t;
                if t == Tab::Wallpaper && panel.images.is_empty() {
                    return load_images(panel.folder.clone());
                }
                Task::none()
            }
            PanelMsg::Close => {
                self.panel = None;
                Task::none()
            }
            PanelMsg::Themes(t) => {
                panel.themes = t;
                Task::none()
            }
            PanelMsg::ShowOn(v) => {
                self.settings.show_on = Some(v);
                self.commit()
            }
            PanelMsg::Mode(v) => {
                self.settings.mode = Some(v);
                self.commit()
            }
            PanelMsg::GlassOpacity(v) => {
                self.settings.glass_opacity = Some(v);
                self.preview(|g| g.glass_opacity = v);
                Task::none()
            }
            PanelMsg::CloseOnLaunch(v) => {
                self.settings.close_on_launch = Some(v);
                self.commit()
            }
            PanelMsg::WorkspaceMode(v) => {
                let (_, ws) = self.panel_ctx().unwrap_or_default();
                let mode = match v {
                    WsMode::Default => None,
                    WsMode::Opaque => Some(Mode::Opaque),
                    WsMode::Glass => Some(Mode::Glass),
                };
                self.settings.set_workspace_mode(&ws, mode);
                self.commit()
            }
            PanelMsg::Running(v) => {
                self.settings.running_apps = Some(v);
                self.commit()
            }
            PanelMsg::IconSize(v) => {
                self.settings.icon_size = Some(v);
                self.preview(|g| g.grid.icon_size = v);
                Task::none()
            }
            PanelMsg::CellSize(v) => {
                self.settings.cell = Some(v);
                self.preview(|g| g.grid.cell = v);
                Task::none()
            }
            PanelMsg::Theme(t) => {
                self.settings.icon_theme = Some(t);
                self.commit()
            }
            PanelMsg::AppFilter(f) => {
                panel.app_filter = f;
                Task::none()
            }
            PanelMsg::AddToWorkspace(v) => {
                panel.add_to_workspace = v;
                Task::none()
            }
            PanelMsg::AddApp(id) => {
                let ws = panel.add_to_workspace.then(|| self.panel_ctx().map(|c| c.1)).flatten();
                self.settings.add_app(&id, ws.as_deref());
                if let Some(p) = self.panel.as_mut() {
                    p.app_filter.clear();
                }
                self.commit()
            }
            PanelMsg::RemoveIcon(i) => {
                self.settings.remove_icon(i);
                self.commit()
            }
            PanelMsg::ResetLayout => {
                let (mon, ws) = self.panel_ctx().unwrap_or_default();
                self.layout.clear_desktop(&mon, &ws);
                self.layout.save();
                Task::none()
            }
            PanelMsg::Scope(s) => {
                panel.scope = s;
                Task::none()
            }
            PanelMsg::Folder(f) => {
                panel.folder = f;
                Task::none()
            }
            PanelMsg::FolderSubmit => {
                panel.images.clear();
                load_images(panel.folder.clone())
            }
            PanelMsg::Images(list) => {
                let missing: Vec<PathBuf> = list.iter().filter(|p| !panel.thumbs.contains_key(*p)).cloned().collect();
                panel.images = list;
                Task::batch(missing.into_iter().map(load_thumb))
            }
            PanelMsg::Thumb(p, h) => {
                if let Some(h) = h {
                    panel.thumbs.insert(p, h);
                }
                Task::none()
            }
            PanelMsg::PickImage(p) => {
                let sel = self.scope_selector(scope);
                let fit = self.current_fit(&sel);
                self.settings.set_wallpaper(sel, WallSource::Path(p), fit);
                self.commit()
            }
            PanelMsg::Kind(k) => {
                let typed = Rgba::parse(&panel.hex);
                let sel = self.scope_selector(scope);
                let fit = self.current_fit(&sel);
                let source = match k {
                    // Une image se choisit dans la grille : rien à enregistrer encore.
                    WallKind::Image => return Task::none(),
                    WallKind::Rotate => WallSource::Rotate {
                        dir: expand(&folder),
                        interval: Duration::from_secs(600),
                        shuffle: true,
                    },
                    WallKind::Color => WallSource::Color(typed.unwrap_or(self.cfg.general.background)),
                };
                self.settings.set_wallpaper(sel, source, fit);
                self.commit()
            }
            PanelMsg::RotInterval(secs) => {
                let sel = self.scope_selector(scope);
                if let Some(w) = self.settings.wallpapers.iter_mut().find(|w| w.sel == sel)
                    && let WallSource::Rotate { interval, .. } = &mut w.source
                {
                    *interval = Duration::from_secs(secs.max(5));
                }
                Task::none()
            }
            PanelMsg::Shuffle(v) => {
                let sel = self.scope_selector(scope);
                if let Some(w) = self.settings.wallpapers.iter_mut().find(|w| w.sel == sel)
                    && let WallSource::Rotate { shuffle, .. } = &mut w.source
                {
                    *shuffle = v;
                }
                self.commit()
            }
            PanelMsg::Color(c) => {
                panel.hex = hex(c);
                let sel = self.scope_selector(scope);
                self.settings.set_wallpaper(sel, WallSource::Color(c), Fit::Cover);
                self.commit()
            }
            PanelMsg::Hex(h) => {
                panel.hex = h;
                Task::none()
            }
            PanelMsg::HexSubmit => match Rgba::parse(&panel.hex) {
                Some(c) => self.panel_update(PanelMsg::Color(c)),
                None => Task::none(),
            },
            PanelMsg::Fit(f) => {
                let sel = self.scope_selector(scope);
                if let Some(w) = self.settings.wallpapers.iter_mut().find(|w| w.sel == sel) {
                    w.fit = f;
                }
                self.commit()
            }
            PanelMsg::RemoveWall => {
                let sel = self.scope_selector(scope);
                self.settings.remove_wallpaper(&sel);
                self.commit()
            }
            PanelMsg::Transition(t) => {
                self.settings.transition = Some(t);
                self.commit()
            }
            PanelMsg::FadeMs(v) => {
                self.settings.fade_ms = Some(v);
                self.preview(|g| g.fade_ms = v);
                Task::none()
            }
            PanelMsg::CrossfadeMs(v) => {
                self.settings.crossfade_ms = Some(v);
                self.preview(|g| g.crossfade_ms = v);
                Task::none()
            }
            PanelMsg::Accent(c) => {
                self.settings.accent = Some(c);
                self.commit()
            }
            PanelMsg::Commit => self.commit(),
        }
    }

    // -- Vue -----------------------------------------------------------------------

    pub(super) fn view_panel(&self) -> Element<'_, Msg> {
        let Some(p) = &self.panel else { return Space::new().into() };
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let tab_btn = |label: &'static str, tab: Tab| {
            let active = p.tab == tab;
            button(text(label).size(15))
                .width(Length::Fill)
                .padding([10, 14])
                .on_press(Msg::Panel(PanelMsg::Tab(tab)))
                .style(move |_, status| button::Style {
                    background: if active {
                        Some(Background::Color(Color { a: 0.35, ..accent }))
                    } else if matches!(status, button::Status::Hovered) {
                        Some(Background::Color(Color { a: 0.08, ..fg }))
                    } else {
                        None
                    },
                    text_color: fg,
                    border: Border::default().rounded(8),
                    ..Default::default()
                })
        };
        let (mon, ws) = self.panel_ctx().unwrap_or_default();
        let sidebar = column![
            text("Réglages").size(22).color(fg),
            text(format!("{mon} · workspace {ws}")).size(12).color(Color { a: 0.6, ..fg }),
            Space::new().height(12),
            tab_btn("Bureau", Tab::Desktop),
            tab_btn("Icônes", Tab::Icons),
            tab_btn("Fond d'écran", Tab::Wallpaper),
            tab_btn("Effets", Tab::Effects),
            Space::new().height(Length::Fill),
            button(text("Fermer").size(15)).width(Length::Fill).padding([10, 14]).on_press(Msg::Panel(PanelMsg::Close)),
        ]
        .spacing(6)
        .width(200);
        let content = match p.tab {
            Tab::Desktop => self.tab_desktop(&ws),
            Tab::Icons => self.tab_icons(p),
            Tab::Wallpaper => self.tab_wallpaper(p),
            Tab::Effects => self.tab_effects(),
        };
        let bg = color(th.bg, 0.97);
        let card = container(row![sidebar, rule::vertical(1), scrollable(container(content).padding([4, 24])).height(Length::Fill)].spacing(16))
            .padding(20)
            .width(Length::Fixed(960.0))
            .height(Length::Fixed(700.0))
            .style(move |_| container::Style {
                background: Some(bg.into()),
                border: Border::default().rounded(16).color(Color { a: 0.15, ..fg }).width(1),
                text_color: Some(fg),
                ..Default::default()
            });
        // Le fond assombri absorbe les clics : rien ne passe au bureau dessous.
        let dim = Color::from_rgba(0.0, 0.0, 0.0, 0.45);
        mouse_area(container(card).center(Length::Fill).style(move |_| container::Style { background: Some(dim.into()), ..Default::default() }))
            .on_press(Msg::Noop)
            .into()
    }

    /// Ligne « libellé — contrôle », avec un avertissement si hyprdesk.lua surcharge la clé.
    fn field<'a>(&self, label: &'a str, key: &str, control: impl Into<Element<'a, Msg>>) -> Element<'a, Msg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let mut left = column![text(label).size(15).color(fg)].spacing(2).width(Length::Fixed(230.0));
        if !key.is_empty() && self.cfg.keys_user.contains(key) {
            left = left.push(text("surchargé dans hyprdesk.lua").size(11).color(Color::from_rgb8(0xff, 0xb8, 0x6c)));
        }
        row![left, control.into()].spacing(16).align_y(Alignment::Center).into()
    }

    fn section<'a>(&self, title: &'a str) -> Element<'a, Msg> {
        let accent = color(self.cfg.general.theme.accent, 1.0);
        column![Space::new().height(8), text(title).size(18).color(accent)].into()
    }

    fn tab_desktop(&self, ws: &str) -> Element<'_, Msg> {
        let g = &self.cfg.general;
        let ws_mode = match self.settings.workspace_mode(ws) {
            None => WsMode::Default,
            Some(Mode::Opaque) => WsMode::Opaque,
            Some(Mode::Glass) => WsMode::Glass,
        };
        column![
            self.section("Affichage"),
            self.field(
                "Afficher le bureau sur",
                "show_on",
                column![
                    radio("Tous les écrans", ShowOn::All, Some(g.show_on), |v| Msg::Panel(PanelMsg::ShowOn(v))),
                    radio("L'écran actif seulement", ShowOn::Focused, Some(g.show_on), |v| Msg::Panel(PanelMsg::ShowOn(v))),
                ]
                .spacing(8),
            ),
            self.field(
                "Fermer après un lancement",
                "close_on_launch",
                toggler(g.close_on_launch).on_toggle(|v| Msg::Panel(PanelMsg::CloseOnLaunch(v))),
            ),
            self.section("Mode"),
            self.field(
                "Mode par défaut",
                "mode",
                column![
                    radio("Opaque — le fond d'écran couvre les fenêtres", Mode::Opaque, Some(g.mode), |v| Msg::Panel(PanelMsg::Mode(v))),
                    radio("Verre — fenêtres floutées derrière", Mode::Glass, Some(g.mode), |v| Msg::Panel(PanelMsg::Mode(v))),
                ]
                .spacing(8),
            ),
            self.field(
                "Mode de ce workspace",
                "",
                row![
                    radio("Par défaut", WsMode::Default, Some(ws_mode), |v| Msg::Panel(PanelMsg::WorkspaceMode(v))),
                    radio("Opaque", WsMode::Opaque, Some(ws_mode), |v| Msg::Panel(PanelMsg::WorkspaceMode(v))),
                    radio("Verre", WsMode::Glass, Some(ws_mode), |v| Msg::Panel(PanelMsg::WorkspaceMode(v))),
                ]
                .spacing(16),
            ),
            self.field(
                "Opacité du voile (verre)",
                "glass_opacity",
                row![
                    slider(0.0..=1.0, g.glass_opacity, |v| Msg::Panel(PanelMsg::GlassOpacity(v)))
                        .step(0.05f32)
                        .on_release(Msg::Panel(PanelMsg::Commit))
                        .width(260),
                    text(format!("{:.0} %", g.glass_opacity * 100.0)).size(14),
                ]
                .spacing(12),
            ),
            text("Le flou du mode verre demande la règle de calque de `hyprdesk snippet` dans hyprland.lua.").size(12),
        ]
        .spacing(14)
        .into()
    }

    fn tab_icons<'a>(&'a self, p: &'a Panel) -> Element<'a, Msg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let themes: Vec<String> = p.themes.clone();
        let mut shortcuts = column![].spacing(6);
        if self.settings.icons.is_empty() {
            shortcuts = shortcuts.push(text("Aucun raccourci ajouté ici.").size(13).color(dim));
        }
        for (i, ic) in self.settings.icons.iter().enumerate() {
            let name = match &ic.kind {
                crate::config::IconKind::App(a) => self.apps.iter().find(|x| &x.id == a).map(|x| x.name.clone()).unwrap_or_else(|| a.clone()),
                crate::config::IconKind::Path(p) => p.display().to_string(),
                crate::config::IconKind::Exec { cmd, .. } => cmd.clone(),
            };
            let scope = match &ic.sel.workspaces {
                None => "tous les bureaux".to_string(),
                Some(w) => format!("workspace {}", w.join(", ")),
            };
            shortcuts = shortcuts.push(
                row![
                    text(name).size(14).width(Length::Fill),
                    text(scope).size(12).color(dim),
                    button(text("Retirer").size(13)).padding([4, 10]).on_press(Msg::Panel(PanelMsg::RemoveIcon(i))),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            );
        }
        let user_icons = self.cfg.icons.iter().filter(|i| i.origin == Origin::User).count();
        let filter = p.app_filter.to_lowercase();
        let mut picker = column![].spacing(2);
        if !filter.is_empty() {
            let matches: Vec<&AppInfo> = self
                .apps
                .iter()
                .filter(|a| a.name.to_lowercase().contains(&filter) || a.id.to_lowercase().contains(&filter))
                .take(12)
                .collect();
            if matches.is_empty() {
                picker = picker.push(text("Aucune application trouvée.").size(13).color(dim));
            }
            for a in matches {
                picker = picker.push(
                    button(row![text(a.name.clone()).size(14).width(Length::Fill), text(a.id.clone()).size(11).color(dim)].spacing(8))
                        .width(Length::Fill)
                        .padding([6, 10])
                        .on_press(Msg::Panel(PanelMsg::AddApp(a.id.clone())))
                        .style(move |_, s| button::Style {
                            background: matches!(s, button::Status::Hovered).then(|| Background::Color(Color { a: 0.1, ..fg })),
                            text_color: fg,
                            border: Border::default().rounded(6),
                            ..Default::default()
                        }),
                );
            }
        }
        column![
            self.section("Applications ouvertes"),
            self.field(
                "Afficher les fenêtres ouvertes",
                "running_apps",
                column![
                    radio("Non", RunningApps::Off, Some(g.running_apps), |v| Msg::Panel(PanelMsg::Running(v))),
                    radio("Celles du workspace affiché", RunningApps::Workspace, Some(g.running_apps), |v| Msg::Panel(PanelMsg::Running(v))),
                    radio("Toutes", RunningApps::All, Some(g.running_apps), |v| Msg::Panel(PanelMsg::Running(v))),
                ]
                .spacing(8),
            ),
            self.section("Raccourcis"),
            shortcuts,
            text(format!("+ {user_icons} icône(s) déclarée(s) dans hyprdesk.lua")).size(12).color(dim),
            self.field(
                "Ajouter une application",
                "",
                column![
                    text_input("Rechercher (firefox, terminal…)", &p.app_filter).on_input(|s| Msg::Panel(PanelMsg::AppFilter(s))).padding(8),
                    toggler(p.add_to_workspace).label("Seulement sur ce workspace").on_toggle(|v| Msg::Panel(PanelMsg::AddToWorkspace(v))),
                    picker,
                ]
                .spacing(8),
            ),
            self.section("Apparence"),
            self.field(
                "Taille des icônes",
                "grid.icon_size",
                row![
                    slider(24..=128u16, g.grid.icon_size, |v| Msg::Panel(PanelMsg::IconSize(v))).on_release(Msg::Panel(PanelMsg::Commit)).width(260),
                    text(format!("{} px", g.grid.icon_size)).size(14),
                ]
                .spacing(12),
            ),
            self.field(
                "Taille des cases",
                "grid.cell",
                row![
                    slider(64..=192u16, g.grid.cell, |v| Msg::Panel(PanelMsg::CellSize(v))).on_release(Msg::Panel(PanelMsg::Commit)).width(260),
                    text(format!("{} px", g.grid.cell)).size(14),
                ]
                .spacing(12),
            ),
            self.field(
                "Thème d'icônes",
                "icon_theme",
                pick_list(themes, g.icon_theme.clone(), |t| Msg::Panel(PanelMsg::Theme(t))).placeholder("Thème du système").width(300),
            ),
            self.field(
                "Disposition de ce bureau",
                "",
                button(text("Réinitialiser les positions").size(14)).padding([6, 12]).on_press(Msg::Panel(PanelMsg::ResetLayout)),
            ),
        ]
        .spacing(14)
        .into()
    }

    fn tab_wallpaper<'a>(&'a self, p: &'a Panel) -> Element<'a, Msg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let accent = color(g.theme.accent, 1.0);
        let (mon, ws) = self.panel_ctx().unwrap_or_default();
        let sel = self.scope_selector(p.scope);
        let rule = self.settings.wallpaper(&sel);
        let kind = rule.map(|r| match r.source {
            WallSource::Path(_) => WallKind::Image,
            WallSource::Rotate { .. } => WallKind::Rotate,
            WallSource::Color(_) => WallKind::Color,
        });
        let effective = resolve(&self.cfg, &mon, &ws);
        let mut col = column![
            self.section("Portée"),
            row![
                radio("Tous les bureaux", Scope::All, Some(p.scope), |v| Msg::Panel(PanelMsg::Scope(v))),
                radio(format!("Workspace {ws}"), Scope::Workspace, Some(p.scope), |v| Msg::Panel(PanelMsg::Scope(v))),
                radio(format!("Écran {mon}"), Scope::Monitor, Some(p.scope), |v| Msg::Panel(PanelMsg::Scope(v))),
            ]
            .spacing(20),
            text(match rule {
                Some(_) => "Une règle existe pour cette portée.".to_string(),
                None => "Pas de règle pour cette portée : le fond est hérité d'une règle plus générale.".to_string(),
            })
            .size(12)
            .color(dim),
        ]
        .spacing(14);
        if effective.wallpaper.is_some_and(|w| w.origin == Origin::User) {
            col = col.push(text("Le fond affiché ici vient de hyprdesk.lua, qui passe avant ces réglages.").size(12).color(Color::from_rgb8(0xff, 0xb8, 0x6c)));
        }
        col = col.push(self.section("Source"));
        col = col.push(
            row![
                radio("Image", WallKind::Image, kind.or(Some(WallKind::Image)), |v| Msg::Panel(PanelMsg::Kind(v))),
                radio("Diaporama", WallKind::Rotate, kind, |v| Msg::Panel(PanelMsg::Kind(v))),
                radio("Couleur unie", WallKind::Color, kind, |v| Msg::Panel(PanelMsg::Kind(v))),
            ]
            .spacing(20),
        );
        col = col.push(self.field(
            "Dossier d'images",
            "",
            row![
                text_input("~/.wallpapers", &p.folder)
                    .on_input(|s| Msg::Panel(PanelMsg::Folder(s)))
                    .on_submit(Msg::Panel(PanelMsg::FolderSubmit))
                    .padding(8)
                    .width(240),
                button(text("Parcourir").size(14)).padding([6, 12]).on_press(Msg::Panel(PanelMsg::FolderSubmit)),
            ]
            .spacing(8),
        ));
        match kind {
            Some(WallKind::Rotate) => {
                if let Some(WallSource::Rotate { dir, interval, shuffle }) = rule.map(|r| &r.source) {
                    let secs = interval.as_secs();
                    col = col.push(text(format!("Diaporama de {}", dir.display())).size(13).color(dim));
                    col = col.push(self.field(
                        "Changer toutes les",
                        "",
                        row![
                            slider(10..=3600u32, secs.clamp(10, 3600) as u32, |v| Msg::Panel(PanelMsg::RotInterval(v as u64)))
                                .step(10u32)
                                .on_release(Msg::Panel(PanelMsg::Commit))
                                .width(260),
                            text(if secs >= 60 { format!("{} min {:02} s", secs / 60, secs % 60) } else { format!("{secs} s") }).size(14),
                        ]
                        .spacing(12),
                    ));
                    col = col.push(self.field("Ordre aléatoire", "", toggler(*shuffle).on_toggle(|v| Msg::Panel(PanelMsg::Shuffle(v)))));
                    col = col.push(text("Pour changer de dossier : saisir le dossier, puis choisir à nouveau « Diaporama ».").size(12).color(dim));
                }
            }
            Some(WallKind::Color) => {
                let swatches = row(SWATCHES.iter().map(|c| {
                    let col = color(*c, 1.0);
                    button(Space::new().width(32).height(32))
                        .padding(0)
                        .on_press(Msg::Panel(PanelMsg::Color(*c)))
                        .style(move |_, _| button::Style {
                            background: Some(col.into()),
                            border: Border::default().rounded(8).color(Color { a: 0.3, ..fg }).width(1),
                            ..Default::default()
                        })
                        .into()
                }))
                .spacing(8);
                col = col.push(self.field("Couleur", "", swatches));
                col = col.push(self.field(
                    "Code",
                    "",
                    text_input("#22212c", &p.hex).on_input(|s| Msg::Panel(PanelMsg::Hex(s))).on_submit(Msg::Panel(PanelMsg::HexSubmit)).padding(8).width(160),
                ));
            }
            _ => {
                let chosen = rule.and_then(|r| if let WallSource::Path(p) = &r.source { Some(p.clone()) } else { None });
                if p.images.is_empty() {
                    col = col.push(text("Aucune image (jpg, png, webp) dans ce dossier.").size(13).color(dim));
                }
                let per_row = 3;
                let mut grid = column![].spacing(10);
                for chunk in p.images.chunks(per_row) {
                    grid = grid.push(row(chunk.iter().map(|img| {
                        let selected = chosen.as_ref() == Some(img);
                        let inner: Element<'_, Msg> = match p.thumbs.get(img) {
                            Some(h) => image(h.clone()).width(THUMB.0 as f32).height(THUMB.1 as f32).content_fit(iced::ContentFit::Cover).into(),
                            None => container(text("…").size(14)).center_x(THUMB.0 as f32).center_y(THUMB.1 as f32).into(),
                        };
                        button(inner)
                            .padding(3)
                            .on_press(Msg::Panel(PanelMsg::PickImage(img.clone())))
                            .style(move |_, s| button::Style {
                                background: None,
                                border: Border::default().rounded(8).width(3).color(if selected {
                                    accent
                                } else if matches!(s, button::Status::Hovered) {
                                    Color { a: 0.5, ..fg }
                                } else {
                                    Color::TRANSPARENT
                                }),
                                ..Default::default()
                            })
                            .into()
                    }))
                    .spacing(10));
                }
                col = col.push(grid);
            }
        }
        if rule.is_some() && kind != Some(WallKind::Color) {
            let fits = [Fit::Cover, Fit::Contain, Fit::Fill, Fit::Center];
            let current = rule.map(|r| r.fit).unwrap_or_default();
            col = col.push(self.field(
                "Ajustement",
                "",
                row(fits.iter().map(|f| {
                    radio(
                        match f {
                            Fit::Cover => "Remplir",
                            Fit::Contain => "Ajuster",
                            Fit::Fill => "Étirer",
                            Fit::Center => "Centrer",
                        },
                        *f,
                        Some(current),
                        |v| Msg::Panel(PanelMsg::Fit(v)),
                    )
                    .into()
                }))
                .spacing(16),
            ));
        }
        if rule.is_some() {
            col = col.push(button(text("Supprimer la règle de cette portée").size(14)).padding([6, 12]).on_press(Msg::Panel(PanelMsg::RemoveWall)));
        }
        col.into()
    }

    fn tab_effects(&self) -> Element<'_, Msg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let swatches = row(SWATCHES[..6].iter().map(|c| {
            let col = color(*c, 1.0);
            let selected = g.theme.accent == *c;
            button(Space::new().width(32).height(32))
                .padding(0)
                .on_press(Msg::Panel(PanelMsg::Accent(*c)))
                .style(move |_, _| button::Style {
                    background: Some(col.into()),
                    border: Border::default().rounded(16).width(if selected { 3 } else { 1 }).color(if selected { fg } else { Color { a: 0.3, ..fg } }),
                    ..Default::default()
                })
                .into()
        }))
        .spacing(10);
        column![
            self.section("Apparition du bureau"),
            self.field(
                "Transition",
                "transition",
                row![
                    radio("Fondu", Transition::Fade, Some(g.transition), |v| Msg::Panel(PanelMsg::Transition(v))),
                    radio("Glissement", Transition::Slide, Some(g.transition), |v| Msg::Panel(PanelMsg::Transition(v))),
                    radio("Aucune", Transition::None, Some(g.transition), |v| Msg::Panel(PanelMsg::Transition(v))),
                ]
                .spacing(16),
            ),
            self.field(
                "Durée",
                "fade_ms",
                row![
                    slider(0..=1000u32, g.fade_ms.min(1000) as u32, |v| Msg::Panel(PanelMsg::FadeMs(v as u64)))
                        .step(10u32)
                        .on_release(Msg::Panel(PanelMsg::Commit))
                        .width(260),
                    text(format!("{} ms", g.fade_ms)).size(14),
                ]
                .spacing(12),
            ),
            self.section("Fonds d'écran"),
            self.field(
                "Fondu entre deux fonds",
                "crossfade_ms",
                row![
                    slider(0..=3000u32, g.crossfade_ms.min(3000) as u32, |v| Msg::Panel(PanelMsg::CrossfadeMs(v as u64)))
                        .step(50u32)
                        .on_release(Msg::Panel(PanelMsg::Commit))
                        .width(260),
                    text(format!("{} ms", g.crossfade_ms)).size(14),
                ]
                .spacing(12),
            ),
            self.section("Couleurs"),
            self.field("Couleur d'accent", "theme.accent", swatches),
        ]
        .spacing(14)
        .into()
    }
}
