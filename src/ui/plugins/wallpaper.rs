//! Paquet intégré « Fond d’écran » : Image, diaporama ou couleur, par bureau, workspace ou écran.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::i18n::text;
use iced::widget::{
    Space, button, column, container, image, pick_list, radio, row, slider, text_input, toggler,
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
    /// Portée réglée (`None` : le workspace où les réglages sont ouverts).
    pub scope: Option<Scope>,
    /// Règle dont la suppression attend confirmation.
    pub pending_rule: Option<Scope>,
    pub folder: String,
    pub images: Vec<PathBuf>,
    pub thumbs: HashMap<PathBuf, image::Handle>,
    /// Dimensions des images (pour signaler une résolution trop faible).
    pub dims: HashMap<PathBuf, (u32, u32)>,
    pub hex: String,
}

impl Default for State {
    fn default() -> Self {
        State {
            scope: None,
            pending_rule: None,
            folder: default_folder(),
            images: Vec::new(),
            thumbs: HashMap::new(),
            dims: HashMap::new(),
            hex: String::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    Scope(Scope),
    /// Supprime une règle (premier appui : arme ; second : confirme).
    DeleteRule(Scope),
    Folder(String),
    FolderSubmit,
    Images(Vec<PathBuf>),
    /// Vignette et dimensions d'origine.
    Thumb(PathBuf, Option<(image::Handle, (u32, u32))>),
    PickImage(PathBuf),
    Kind(WallKind),
    RotInterval(u64),
    Shuffle(bool),
    Color(Rgba),
    Hex(String),
    HexSubmit,
    Fit(Fit),
    /// Étaler l'image sur tous les écrans (découpe automatique).
    /// Étalement : 0 non, 1 bureau et verrouillage, 2 verrouillage seulement.
    Span(u8),
    RemoveWall,
}

/// Portée d'une règle de fond d'écran : un écran et/ou un workspace
/// (`None` : tous).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Scope {
    pub monitor: Option<String>,
    pub workspace: Option<String>,
}

/// Choix d'une liste de portée (`None` : tous).
#[derive(Debug, Clone, PartialEq)]
struct Pick {
    value: Option<String>,
    label: String,
}

impl std::fmt::Display for Pick {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

/// Libellé d'une portée (« Écran DP-1 · Workspace 3 »).
fn scope_label(s: &Scope) -> String {
    match (&s.monitor, &s.workspace) {
        (None, None) => crate::i18n::tr("Tous les bureaux").to_string(),
        (Some(m), None) => crate::tr!("Écran {mon}", mon = m),
        (None, Some(w)) => crate::tr!("Workspace {ws}", ws = w),
        (Some(m), Some(w)) => crate::tr!("Écran {mon} · workspace {ws}", mon = m, ws = w),
    }
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
                let full = ::image::open(&p2).ok()?;
                let dims = (full.width(), full.height());
                let img = full.thumbnail(THUMB.0 * 2, THUMB.1 * 2).to_rgba8();
                Some((
                    image::Handle::from_rgba(img.width(), img.height(), img.into_raw()),
                    dims,
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
        let scope = self.wall_scope();
        let sel = self.scope_selector(scope.clone());
        let rule = self.settings.wallpaper(&sel);
        let kind = rule.map(|r| match r.source {
            WallSource::Path(_) => WallKind::Image,
            WallSource::Rotate { .. } => WallKind::Rotate,
            WallSource::Color(_) => WallKind::Color,
        });
        let effective = resolve(&self.cfg, &mon, &ws);
        let mut col = column![
            self.section("Portée"),
            self.wall_scope_picker(&scope),
            text(match rule {
                Some(_) => crate::i18n::tr("Une règle existe pour cette portée.").to_string(),
                None => crate::i18n::tr(
                    "Pas de règle pour cette portée : le fond est hérité d'une règle plus générale."
                )
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
                    crate::i18n::tr("Image"),
                    WallKind::Image,
                    kind.or(Some(WallKind::Image)),
                    |v| wrap(Msg::Kind(v))
                )
                .font(self.font),
                radio(crate::i18n::tr("Diaporama"), WallKind::Rotate, kind, |v| {
                    wrap(Msg::Kind(v))
                })
                .font(self.font),
                radio(
                    crate::i18n::tr("Couleur unie"),
                    WallKind::Color,
                    kind,
                    |v| wrap(Msg::Kind(v))
                )
                .font(self.font),
            ]
            .spacing(20),
        );
        col = col.push(
            self.field(
                "Dossier d'images",
                "",
                row![
                    text_input(crate::i18n::tr("~/.wallpapers"), &p.wallpaper.folder)
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
                        text(crate::tr!("Diaporama de {dir}", dir = dir.display()))
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
                        text_input(crate::i18n::tr("#22212c"), &p.wallpaper.hex)
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
        let span = rule.is_some_and(|r| r.span);
        let span_mode: u8 = match rule.map(|r| (r.span, r.span_lock)) {
            Some((true, _)) => 1,
            Some((false, true)) => 2,
            _ => 0,
        };
        if rule.is_some() && kind != Some(WallKind::Color) {
            col = col.push(
                self.field(
                    "Étaler sur tous les écrans",
                    "",
                    row![
                        radio(crate::i18n::tr("Non"), 0u8, Some(span_mode), |v| wrap(
                            Msg::Span(v)
                        ))
                        .font(self.font),
                        radio(
                            crate::i18n::tr("Bureau et verrouillage"),
                            1u8,
                            Some(span_mode),
                            |v| wrap(Msg::Span(v))
                        )
                        .font(self.font),
                        radio(
                            crate::i18n::tr("Verrouillage seulement"),
                            2u8,
                            Some(span_mode),
                            |v| wrap(Msg::Span(v))
                        )
                        .font(self.font),
                    ]
                    .spacing(16),
                ),
            );
            col = col.push(
                text(crate::tr!(
                    "Une seule scène pour toute la disposition : l'image couvre l'ensemble des \
                     écrans ({p0}) et chacun en montre sa part, découpée selon sa position et sa \
                     taille dans Hyprland.",
                    p0 = self
                        .monitors
                        .iter()
                        .map(|m| format!("{} {}×{}", m.name, m.width, m.height))
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
                .font(self.font)
                .size(12)
                .color(dim),
            );
        }
        // Résolution trop faible pour les écrans visés : avertir, sans bloquer.
        if let Some(WallSource::Path(img)) = rule.map(|r| &r.source)
            && let Some(&(iw, ih)) = p.wallpaper.dims.get(img)
        {
            let screens: Vec<crate::span::Rect> = self
                .monitors
                .iter()
                .filter(|m| span || scope.monitor.as_ref().is_none_or(|t| *t == m.name))
                .map(|m| {
                    crate::span::physical(
                        crate::span::Rect {
                            x: m.x,
                            y: m.y,
                            w: m.width as f32,
                            h: m.height as f32,
                        },
                        m.scale,
                    )
                })
                .collect();
            if let Some(w) = crate::span::resolution_warning(iw, ih, &screens, span) {
                col = col.push(
                    text(crate::tr!("Attention : {w}", w = w))
                        .font(self.font)
                        .size(12)
                        .color(Color::from_rgb8(0xff, 0xb8, 0x6c)),
                );
            }
        }
        if rule.is_some() && kind != Some(WallKind::Color) && !span {
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
                                Fit::Fill => crate::i18n::tr("Étirer"),
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
    /// Portée : un écran (ou tous) × un workspace (ou tous), et les règles en place.
    fn wall_scope_picker(&self, scope: &Scope) -> Element<'_, UiMsg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let accent = color(g.theme.accent, 1.0);
        let screens: Vec<Pick> = std::iter::once(Pick {
            value: None,
            label: crate::i18n::tr("Tous les écrans").to_string(),
        })
        .chain(self.monitors.iter().map(|m| Pick {
            value: Some(m.name.clone()),
            label: m.name.clone(),
        }))
        .collect();
        // Workspaces : ceux qui existent toujours, ceux ouverts, ceux qui ont une règle.
        let mut names: Vec<String> = (1..=g.workspaces.unwrap_or(10).max(1))
            .map(|i| i.to_string())
            .collect();
        let extra = self
            .workspaces
            .iter()
            .map(|w| w.name.clone())
            .filter(|n| !n.starts_with("special"))
            .chain(
                self.settings
                    .wallpapers
                    .iter()
                    .filter_map(|w| w.sel.workspaces.as_ref()?.first().cloned()),
            );
        for n in extra {
            if !names.contains(&n) {
                names.push(n);
            }
        }
        let workspaces: Vec<Pick> = std::iter::once(Pick {
            value: None,
            label: crate::i18n::tr("Tous les workspaces").to_string(),
        })
        .chain(names.into_iter().map(|n| Pick {
            label: crate::tr!("Workspace {ws}", ws = n),
            value: Some(n),
        }))
        .collect();
        let cur_screen = screens.iter().find(|p| p.value == scope.monitor).cloned();
        let cur_ws = workspaces
            .iter()
            .find(|p| p.value == scope.workspace)
            .cloned();
        let (s1, s2) = (scope.clone(), scope.clone());
        let pickers = row![
            pick_list(screens, cur_screen, move |p: Pick| {
                wrap(Msg::Scope(Scope {
                    monitor: p.value,
                    ..s1.clone()
                }))
            })
            .font(self.font)
            .width(200),
            pick_list(workspaces, cur_ws, move |p: Pick| {
                wrap(Msg::Scope(Scope {
                    workspace: p.value,
                    ..s2.clone()
                }))
            })
            .font(self.font)
            .width(200),
        ]
        .spacing(12);
        // Règles réglées ici : un clic y revient.
        let mut rules = row![].spacing(6);
        for w in &self.settings.wallpapers {
            let s = Scope {
                monitor: w.sel.monitors.as_ref().and_then(|l| l.first().cloned()),
                workspace: w.sel.workspaces.as_ref().and_then(|l| l.first().cloned()),
            };
            let active = &s == scope;
            let armed = self
                .panel
                .as_ref()
                .is_some_and(|p| p.wallpaper.pending_rule.as_ref() == Some(&s));
            let delete = self.delete_button(armed, wrap(Msg::DeleteRule(s.clone())));
            rules = rules.push(
                row![
                    button(text(scope_label(&s)).font(self.font).size(12))
                        .padding([3, 10])
                        .style(move |_, status| button::Style {
                            background: Some(
                                if active {
                                    Color { a: 0.28, ..accent }
                                } else if matches!(status, button::Status::Hovered) {
                                    Color { a: 0.1, ..fg }
                                } else {
                                    Color { a: 0.05, ..fg }
                                }
                                .into(),
                            ),
                            text_color: fg,
                            border: Border::default().rounded(999).width(1).color(Color {
                                a: if active { 0.7 } else { 0.15 },
                                ..if active { accent } else { fg }
                            }),
                            ..Default::default()
                        })
                        .on_press(wrap(Msg::Scope(s))),
                    delete,
                ]
                .spacing(2)
                .align_y(iced::Alignment::Center),
            );
        }
        let mut col = column![pickers].spacing(10);
        if !self.settings.wallpapers.is_empty() {
            col = col.push(
                column![
                    text(crate::i18n::tr("Règles en place :"))
                        .font(self.font)
                        .size(12)
                        .color(Color { a: 0.6, ..fg }),
                    rules.wrap(),
                ]
                .spacing(6),
            );
        }
        col.into()
    }

    /// Portée affichée : celle choisie, sinon le workspace où les réglages sont ouverts.
    pub(in crate::ui) fn wall_scope(&self) -> Scope {
        self.panel
            .as_ref()
            .map(|p| self.wall_scope_of(p))
            .unwrap_or_default()
    }

    fn wall_scope_of(&self, p: &Panel) -> Scope {
        p.wallpaper.scope.clone().unwrap_or_else(|| Scope {
            monitor: None,
            workspace: Some(self.panel_ctx().unwrap_or_default().1),
        })
    }

    pub(in crate::ui) fn scope_selector(&self, scope: Scope) -> Selector {
        Selector {
            workspaces: scope.workspace.map(|w| vec![w]),
            monitors: scope.monitor.map(|m| vec![m]),
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
            .map(|p| (self.wall_scope_of(p), p.wallpaper.folder.clone()))
        else {
            return Task::none();
        };
        let Some(panel) = self.panel.as_mut() else {
            return Task::none();
        };
        match msg {
            Msg::Scope(s) => {
                panel.wallpaper.scope = Some(s);
                Task::none()
            }
            Msg::DeleteRule(s) => {
                if panel.wallpaper.pending_rule.as_ref() != Some(&s) {
                    panel.wallpaper.pending_rule = Some(s);
                    return Task::none();
                }
                panel.wallpaper.pending_rule = None;
                let sel = self.scope_selector(s);
                self.settings.remove_wallpaper(&sel);
                self.commit()
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
                if let Some((h, dims)) = h {
                    panel.wallpaper.dims.insert(p.clone(), dims);
                    panel.wallpaper.thumbs.insert(p, h);
                }
                Task::none()
            }
            Msg::PickImage(p) => {
                // En-tête seulement : de quoi avertir d'une résolution trop faible.
                if !panel.wallpaper.dims.contains_key(&p)
                    && let Ok(d) = ::image::image_dimensions(&p)
                {
                    panel.wallpaper.dims.insert(p.clone(), d);
                }
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
            Msg::Span(v) => {
                let sel = self.scope_selector(scope);
                self.settings.set_wallpaper_span(&sel, v == 1, v >= 1);
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
