//! Paquet intégré « Fond d’écran » : Image, diaporama ou couleur, par bureau, workspace ou écran.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use iced::widget::{
    Space, button, column, container, image, radio, row, slider, text, text_input, toggler,
};
use iced::{Border, Color, Element, Task};

use super::super::panel::{Panel, PanelMsg, SWATCHES};
use super::super::{App, Msg as UiMsg, color, list_images};
use super::{PluginInfo, PluginMsg};
use crate::config::resolve::resolve;
use crate::config::{Fit, Origin, Rgba, Selector, WallSource};
use crate::settings::hex;

pub const INFO: PluginInfo = PluginInfo {
    id: "wallpaper",
    label: "Fond d’écran",
    icon: "image-x-generic",
    summary: "Image, diaporama ou couleur, par bureau, workspace ou écran.",
    builtin: true,
};

/// Message de ce paquet, enveloppé pour l'application.
pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Wallpaper(m)))
}

/// État propre à ce paquet, porté par le tiroir des réglages.
pub struct State {
    pub scope: Scope,
    pub folder: String,
    pub images: Vec<PathBuf>,
    pub thumbs: HashMap<PathBuf, image::Handle>,
    pub hex: String,
}

impl Default for State {
    fn default() -> Self {
        State {
            scope: Scope::Workspace,
            folder: default_folder(),
            images: Vec::new(),
            thumbs: HashMap::new(),
            hex: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
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
pub(in crate::ui) const THUMB: (u32, u32) = (160, 90);
pub(in crate::ui) const MAX_IMAGES: usize = 60;
pub(in crate::ui) fn default_folder() -> String {
    "~/.wallpapers".into()
}
pub(in crate::ui) fn expand(p: &str) -> PathBuf {
    crate::config::lua::expand_path(p, std::path::Path::new("/"))
}
pub(in crate::ui) fn load_images(folder: String) -> Task<UiMsg> {
    Task::perform(
        async move {
            let dir = expand(&folder);
            tokio::task::spawn_blocking(move || {
                list_images(&dir)
                    .into_iter()
                    .take(MAX_IMAGES)
                    .collect::<Vec<_>>()
            })
            .await
            .unwrap_or_default()
        },
        |l| wrap(Msg::Images(l)),
    )
}
pub(in crate::ui) fn load_thumb(p: PathBuf) -> Task<UiMsg> {
    Task::perform(
        async move {
            let p2 = p.clone();
            static DECODERS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(4);
            let _permit = DECODERS.acquire().await;
            let h = tokio::task::spawn_blocking(move || {
                let img = ::image::open(&p2)
                    .ok()?
                    .thumbnail(THUMB.0 * 2, THUMB.1 * 2)
                    .to_rgba8();
                Some(image::Handle::from_rgba(
                    img.width(),
                    img.height(),
                    img.into_raw(),
                ))
            })
            .await
            .ok()
            .flatten();
            (p, h)
        },
        |(p, h)| wrap(Msg::Thumb(p, h)),
    )
}

impl App {
    pub(in crate::ui) fn tab_wallpaper<'a>(&'a self, p: &'a Panel) -> Element<'a, UiMsg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let accent = color(g.theme.accent, 1.0);
        let (mon, ws) = self.panel_ctx().unwrap_or_default();
        let sel = self.scope_selector(p.wallpaper.scope);
        let rule = self.settings.wallpaper(&sel);
        let kind = rule.map(|r| match r.source {
            WallSource::Path(_) => WallKind::Image,
            WallSource::Rotate { .. } => WallKind::Rotate,
            WallSource::Color(_) => WallKind::Color,
        });
        let effective = resolve(&self.cfg, &mon, &ws);
        let mut col =
            column![
            self.section("Portée"),
            row![
                radio("Tous les bureaux", Scope::All, Some(p.wallpaper.scope), |v| {
                    wrap(Msg::Scope(v))
                }).font(self.font),
                radio(
                    format!("Workspace {ws}"),
                    Scope::Workspace,
                    Some(p.wallpaper.scope),
                    |v| wrap(Msg::Scope(v))
                ).font(self.font),
                radio(
                    format!("Écran {mon}"),
                    Scope::Monitor,
                    Some(p.wallpaper.scope),
                    |v| wrap(Msg::Scope(v))
                ).font(self.font),
            ]
            .spacing(20),
            text(match rule {
                Some(_) => "Une règle existe pour cette portée.".to_string(),
                None =>
                    "Pas de règle pour cette portée : le fond est hérité d'une règle plus générale."
                        .to_string(),
            })
            .font(self.font)
            .size(12)
            .color(dim),
        ]
            .spacing(14);
        if effective
            .wallpaper
            .is_some_and(|w| w.origin == Origin::User)
        {
            col = col.push(
                text("Le fond affiché ici vient de hyprdesk.lua, qui passe avant ces réglages.")
                    .font(self.font)
                    .size(12)
                    .color(Color::from_rgb8(0xff, 0xb8, 0x6c)),
            );
        }
        col = col.push(self.section("Source"));
        col = col.push(
            row![
                radio(
                    "Image",
                    WallKind::Image,
                    kind.or(Some(WallKind::Image)),
                    |v| wrap(Msg::Kind(v))
                )
                .font(self.font),
                radio("Diaporama", WallKind::Rotate, kind, |v| wrap(Msg::Kind(v))).font(self.font),
                radio("Couleur unie", WallKind::Color, kind, |v| wrap(Msg::Kind(
                    v
                )))
                .font(self.font),
            ]
            .spacing(20),
        );
        col = col.push(
            self.field(
                "Dossier d'images",
                "",
                row![
                    text_input("~/.wallpapers", &p.wallpaper.folder)
                        .font(self.font)
                        .on_input(|s| wrap(Msg::Folder(s)))
                        .on_submit(wrap(Msg::FolderSubmit))
                        .padding(8)
                        .width(240),
                    button(text("Parcourir").font(self.font).size(14))
                        .padding([6, 12])
                        .on_press(wrap(Msg::FolderSubmit)),
                ]
                .spacing(8),
            ),
        );
        match kind {
            Some(WallKind::Rotate) => {
                if let Some(WallSource::Rotate {
                    dir,
                    interval,
                    shuffle,
                }) = rule.map(|r| &r.source)
                {
                    let secs = interval.as_secs();
                    col = col.push(
                        text(format!("Diaporama de {}", dir.display()))
                            .font(self.font)
                            .size(13)
                            .color(dim),
                    );
                    col = col.push(
                        self.field(
                            "Changer toutes les",
                            "",
                            row![
                                slider(10..=3600u32, secs.clamp(10, 3600) as u32, |v| wrap(
                                    Msg::RotInterval(v as u64)
                                ))
                                .step(10u32)
                                .on_release(UiMsg::Panel(PanelMsg::Commit))
                                .width(260),
                                text(if secs >= 60 {
                                    format!("{} min {:02} s", secs / 60, secs % 60)
                                } else {
                                    format!("{secs} s")
                                })
                                .font(self.font)
                                .size(14),
                            ]
                            .spacing(12),
                        ),
                    );
                    col = col.push(self.field(
                        "Ordre aléatoire",
                        "",
                        toggler(*shuffle).on_toggle(|v| wrap(Msg::Shuffle(v))),
                    ));
                    col = col.push(text("Pour changer de dossier : saisir le dossier, puis choisir à nouveau « Diaporama ».").font(self.font).size(12).color(dim));
                }
            }
            Some(WallKind::Color) => {
                let swatches = row(SWATCHES.iter().map(|c| {
                    let col = color(*c, 1.0);
                    button(Space::new().width(32).height(32))
                        .padding(0)
                        .on_press(wrap(Msg::Color(*c)))
                        .style(move |_, _| button::Style {
                            background: Some(col.into()),
                            border: Border::default()
                                .rounded(8)
                                .color(Color { a: 0.3, ..fg })
                                .width(1),
                            ..Default::default()
                        })
                        .into()
                }))
                .spacing(8);
                col = col.push(self.field("Couleur", "", swatches));
                col = col.push(
                    self.field(
                        "Code",
                        "",
                        text_input("#22212c", &p.wallpaper.hex)
                            .font(self.font)
                            .on_input(|s| wrap(Msg::Hex(s)))
                            .on_submit(wrap(Msg::HexSubmit))
                            .padding(8)
                            .width(160),
                    ),
                );
            }
            _ => {
                let chosen = rule.and_then(|r| {
                    if let WallSource::Path(p) = &r.source {
                        Some(p.clone())
                    } else {
                        None
                    }
                });
                if p.wallpaper.images.is_empty() {
                    col = col.push(
                        text("Aucune image (jpg, png, webp) dans ce dossier.")
                            .font(self.font)
                            .size(13)
                            .color(dim),
                    );
                }
                let available = self.panel_width()
                    - if self.panel_width() >= 640.0 {
                        156.0
                    } else {
                        46.0
                    }
                    - 96.0;
                let per_row = ((available / 176.0).floor() as usize).max(1);
                let mut grid = column![].spacing(10);
                for chunk in p.wallpaper.images.chunks(per_row) {
                    grid = grid.push(
                        row(chunk.iter().map(|img| {
                            let selected = chosen.as_ref() == Some(img);
                            let inner: Element<'_, UiMsg> = match p.wallpaper.thumbs.get(img) {
                                Some(h) => image(h.clone())
                                    .width(THUMB.0 as f32)
                                    .height(THUMB.1 as f32)
                                    .content_fit(iced::ContentFit::Cover)
                                    .into(),
                                None => container(text("…").font(self.font).size(14))
                                    .center_x(THUMB.0 as f32)
                                    .center_y(THUMB.1 as f32)
                                    .into(),
                            };
                            button(inner)
                                .padding(3)
                                .on_press(wrap(Msg::PickImage(img.clone())))
                                .style(move |_, s| button::Style {
                                    background: None,
                                    border: Border::default().rounded(8).width(3).color(
                                        if selected {
                                            accent
                                        } else if matches!(s, button::Status::Hovered) {
                                            Color { a: 0.5, ..fg }
                                        } else {
                                            Color::TRANSPARENT
                                        },
                                    ),
                                    ..Default::default()
                                })
                                .into()
                        }))
                        .spacing(10),
                    );
                }
                col = col.push(grid);
            }
        }
        if rule.is_some() && kind != Some(WallKind::Color) {
            let fits = [Fit::Cover, Fit::Contain, Fit::Fill, Fit::Center];
            let current = rule.map(|r| r.fit).unwrap_or_default();
            col = col.push(
                self.field(
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
                            |v| wrap(Msg::Fit(v)),
                        )
                        .font(self.font)
                        .into()
                    }))
                    .spacing(16),
                ),
            );
        }
        if rule.is_some() {
            col = col.push(
                button(
                    text("Supprimer la règle de cette portée")
                        .font(self.font)
                        .size(14),
                )
                .padding([6, 12])
                .on_press(wrap(Msg::RemoveWall)),
            );
        }
        col.into()
    }
    pub(in crate::ui) fn scope_selector(&self, scope: Scope) -> Selector {
        let (mon, ws) = self.panel_ctx().unwrap_or_default();
        match scope {
            Scope::All => Selector::default(),
            Scope::Workspace => Selector {
                workspaces: Some(vec![ws]),
                monitors: None,
            },
            Scope::Monitor => Selector {
                workspaces: None,
                monitors: Some(vec![mon]),
            },
        }
    }
    pub(in crate::ui) fn current_fit(&self, sel: &Selector) -> Fit {
        self.settings
            .wallpaper(sel)
            .map(|w| w.fit)
            .unwrap_or_default()
    }

    pub(in crate::ui) fn wallpaper_update(&mut self, msg: Msg) -> Task<UiMsg> {
        let Some((scope, folder)) = self
            .panel
            .as_ref()
            .map(|p| (p.wallpaper.scope, p.wallpaper.folder.clone()))
        else {
            return Task::none();
        };
        let Some(panel) = self.panel.as_mut() else {
            return Task::none();
        };
        match msg {
            Msg::Scope(s) => {
                panel.wallpaper.scope = s;
                Task::none()
            }
            Msg::Folder(f) => {
                panel.wallpaper.folder = f;
                Task::none()
            }
            Msg::FolderSubmit => {
                panel.wallpaper.images.clear();
                load_images(panel.wallpaper.folder.clone())
            }
            Msg::Images(list) => {
                let missing: Vec<PathBuf> = list
                    .iter()
                    .filter(|p| !panel.wallpaper.thumbs.contains_key(*p))
                    .cloned()
                    .collect();
                panel.wallpaper.images = list;
                Task::batch(missing.into_iter().map(load_thumb))
            }
            Msg::Thumb(p, h) => {
                if let Some(h) = h {
                    panel.wallpaper.thumbs.insert(p, h);
                }
                Task::none()
            }
            Msg::PickImage(p) => {
                let sel = self.scope_selector(scope);
                let fit = self.current_fit(&sel);
                self.settings.set_wallpaper(sel, WallSource::Path(p), fit);
                self.commit()
            }
            Msg::Kind(k) => {
                let typed = Rgba::parse(&panel.wallpaper.hex);
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
                    WallKind::Color => {
                        WallSource::Color(typed.unwrap_or(self.cfg.general.background))
                    }
                };
                self.settings.set_wallpaper(sel, source, fit);
                self.commit()
            }
            Msg::RotInterval(secs) => {
                let sel = self.scope_selector(scope);
                if let Some(w) = self.settings.wallpapers.iter_mut().find(|w| w.sel == sel)
                    && let WallSource::Rotate { interval, .. } = &mut w.source
                {
                    *interval = Duration::from_secs(secs.max(5));
                }
                Task::none()
            }
            Msg::Shuffle(v) => {
                let sel = self.scope_selector(scope);
                if let Some(w) = self.settings.wallpapers.iter_mut().find(|w| w.sel == sel)
                    && let WallSource::Rotate { shuffle, .. } = &mut w.source
                {
                    *shuffle = v;
                }
                self.commit()
            }
            Msg::Color(c) => {
                panel.wallpaper.hex = hex(c);
                let sel = self.scope_selector(scope);
                self.settings
                    .set_wallpaper(sel, WallSource::Color(c), Fit::Cover);
                self.commit()
            }
            Msg::Hex(h) => {
                panel.wallpaper.hex = h;
                Task::none()
            }
            Msg::HexSubmit => match Rgba::parse(&panel.wallpaper.hex) {
                Some(c) => self.wallpaper_update(Msg::Color(c)),
                None => Task::none(),
            },
            Msg::Fit(f) => {
                let sel = self.scope_selector(scope);
                if let Some(w) = self.settings.wallpapers.iter_mut().find(|w| w.sel == sel) {
                    w.fit = f;
                }
                self.commit()
            }
            Msg::RemoveWall => {
                let sel = self.scope_selector(scope);
                self.settings.remove_wallpaper(&sel);
                self.commit()
            }
        }
    }
}
