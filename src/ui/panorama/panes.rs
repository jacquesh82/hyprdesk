//! Zones du Panorama (une page, ou l'écran partagé en 2 ou 3) et widget
//! Vidéo : une fenêtre de navigateur posée exactement dans sa zone, la
//! surface y laissant un trou (image et saisie).

use std::time::Duration;

use iced::widget::{Space, button, column, container, image, row, stack, text};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Task};

use super::swipe::{Axis, swipe};
use super::{App, Msg, PMsg, PREVIEW, PREVIEW_PX_PER_MM, fx, wrap};
use crate::panorama::config::Layout;
use crate::panorama::layout::{self, Rect};
use crate::panorama::video;

/// Recherche de la fenêtre lancée : un essai toutes les 400 ms, 25 s au plus.
const POLL: Duration = Duration::from_millis(400);
const POLL_MAX: u32 = 62;

/// Vidéo en cours du widget Vidéo.
#[derive(Debug, Clone)]
pub struct Video {
    pub name: String,
    pub url: String,
    /// Préfixe d'app_id des fenêtres d'application du navigateur.
    pub prefix: String,
    /// Fenêtre retrouvée (absente pendant l'ouverture).
    pub address: Option<String>,
    /// Fenêtres déjà là au lancement.
    pub known: Vec<String>,
    pub error: Option<String>,
    /// Rectangle global où la fenêtre a été posée en dernier.
    pub placed: Option<(i32, i32, i32, i32)>,
    /// Fenêtre active avant le lancement (le clavier lui est rendu).
    pub focus_back: Option<String>,
}

/// Une surface du Panorama (écran ou fenêtre d'aperçu).
#[derive(Debug, Clone)]
pub(super) struct Surf {
    pub id: Id,
    /// Écran, ou `PREVIEW`.
    pub key: String,
    pub w: f32,
    pub h: f32,
    pub ppm: Option<f32>,
}

#[derive(Debug, Clone)]
pub(super) struct Pane {
    pub rect: Rect,
    pub widget: String,
    /// Rang de la zone (écran partagé) ; `None` : page unique.
    pub index: Option<usize>,
}

#[derive(Debug, Clone)]
pub(super) struct Geo {
    pub panes: Vec<Pane>,
    /// Hauteur du bandeau d'une zone.
    pub header: f32,
    pub target: f32,
}

impl Geo {
    /// La zone a un bandeau (écran partagé, ou page Vidéo).
    pub fn has_header(p: &Pane) -> bool {
        p.index.is_some() || p.widget == "video"
    }

    /// Contenu d'une zone, sous son bandeau.
    pub fn content(&self, p: &Pane) -> Rect {
        if Self::has_header(p) {
            Rect {
                x: p.rect.x,
                y: p.rect.y + self.header,
                w: p.rect.w,
                h: (p.rect.h - self.header).max(1.0),
            }
        } else {
            p.rect
        }
    }

    /// Première zone Vidéo : c'est elle qui accueille la fenêtre.
    pub fn video_pane(&self) -> Option<&Pane> {
        self.panes.iter().find(|p| p.widget == "video")
    }
}

/// Ce que la surface dessine dans la zone Vidéo.
enum VideoState<'a> {
    /// Aucune vidéo : les tuiles des sources.
    Tiles,
    /// Ouverture en cours.
    Opening(&'a str),
    /// Fenêtre posée : trou transparent.
    Hole,
    /// La vidéo est dans une autre zone.
    Elsewhere,
}

impl App {
    /// Surface du Panorama (écran ou aperçu) et sa taille réelle.
    pub(super) fn panorama_surf(&self, id: Id) -> Option<Surf> {
        // Taille réelle de la surface d'abord : après une rotation, la liste
        // des écrans peut avoir un temps de retard ; la fenêtre d'aperçu,
        // elle, se redimensionne à la main.
        let real = self
            .sizes
            .get(&id)
            .map(|s| (s.width, s.height))
            .filter(|(w, h)| *w > 0.0 && *h > 0.0);
        if let Some(p) = self.panorama.preview.filter(|p| p.win == id) {
            let (w, h) = real.unwrap_or((
                p.virtual_size.0 as f32 * p.scale,
                p.virtual_size.1 as f32 * p.scale,
            ));
            return Some(Surf {
                id,
                key: PREVIEW.to_string(),
                w,
                h,
                ppm: Some(PREVIEW_PX_PER_MM * p.scale),
            });
        }
        let mon = self.panorama.surfaces.get(&id)?;
        let m = self.monitors.iter().find(|x| &x.name == mon)?;
        let ppm = self
            .panorama
            .screens
            .iter()
            .find(|s| &s.name == mon)
            .and_then(crate::panorama::detect::Screen::px_per_mm);
        let (w, h) = real.unwrap_or((m.width as f32, m.height as f32));
        Some(Surf {
            id,
            key: mon.clone(),
            w,
            h,
            ppm,
        })
    }

    /// Zones de la surface et leurs widgets.
    pub(super) fn panorama_geo(&self, s: &Surf) -> Geo {
        let c = &self.cfg.general.panorama;
        let target = layout::target(s.ppm);
        let panes = match self.panorama_layout_at(&s.key) {
            Layout::Single => {
                let widget = if self.panorama_asleep() {
                    "clock"
                } else {
                    self.panorama_page_id(&s.key)
                };
                vec![Pane {
                    rect: Rect {
                        x: 0.0,
                        y: 0.0,
                        w: s.w,
                        h: s.h,
                    },
                    widget: widget.to_string(),
                    index: None,
                }]
            }
            Layout::Dashboard => c
                .design
                .tiles
                .iter()
                .map(|t| {
                    let pad = c.design.padding;
                    let gap = c.design.gap;
                    let (x, y, w, h) = if s.w >= s.h {
                        (t.x, t.y, t.w, t.h)
                    } else {
                        (t.y, t.x, t.h, t.w)
                    };
                    Pane {
                        rect: Rect {
                            x: pad + x * (s.w - 2.0 * pad) + gap / 2.0,
                            y: pad + y * (s.h - 2.0 * pad) + gap / 2.0,
                            w: (w * (s.w - 2.0 * pad) - gap).max(1.0),
                            h: (h * (s.h - 2.0 * pad) - gap).max(1.0),
                        },
                        widget: t.widget.clone(),
                        index: None,
                    }
                })
                .collect(),
            _ => layout::split_panes(&c.split().unwrap_or_default(), s.w, s.h, c.design.gap)
                .into_iter()
                .enumerate()
                .map(|(i, rect)| Pane {
                    rect,
                    widget: c.pane(i).to_string(),
                    index: Some(i),
                })
                .collect(),
        };
        Geo {
            panes,
            header: target,
            target,
        }
    }

    /// Trous de la surface (zone de la fenêtre vidéo posée).
    fn panorama_holes(&self, geo: &Geo) -> Vec<Rect> {
        if self
            .panorama
            .video
            .as_ref()
            .is_none_or(|v| v.address.is_none())
        {
            return Vec::new();
        }
        geo.video_pane()
            .map(|p| vec![geo.content(p)])
            .unwrap_or_default()
    }

    /// Une seule surface accueille la vidéo : les autres gardent leur matière.
    fn panorama_video_host(&self) -> Option<Id> {
        self.panorama_surfs()
            .into_iter()
            .find(|s| self.panorama_geo(s).video_pane().is_some())
            .map(|s| s.id)
    }

    fn panorama_surface_holes(&self, s: &Surf, geo: &Geo) -> Vec<Rect> {
        if self.panorama_video_host() == Some(s.id) {
            self.panorama_holes(geo)
        } else {
            Vec::new()
        }
    }

    /// Surfaces du Panorama : écrans d'abord, puis l'aperçu.
    fn panorama_surfs(&self) -> Vec<Surf> {
        let mut ids: Vec<Id> = self.panorama.surfaces.keys().copied().collect();
        ids.sort_by_key(|id| self.panorama.surfaces.get(id).cloned());
        ids.extend(self.panorama.preview.map(|p| p.win));
        ids.into_iter()
            .filter_map(|id| self.panorama_surf(id))
            .collect()
    }

    /// Rectangle global où poser la fenêtre vidéo : la zone Vidéo du premier
    /// écran Panorama qui en montre une, sinon celle de l'aperçu.
    fn pip_target(&self) -> Option<(i32, i32, i32, i32)> {
        for s in self.panorama_surfs() {
            let geo = self.panorama_geo(&s);
            let Some(p) = geo.video_pane() else { continue };
            let r = geo.content(p);
            let origin = if s.key == PREVIEW {
                self.panorama.preview_at?
            } else {
                let m = self.monitors.iter().find(|m| m.name == s.key)?;
                (m.x.round() as i32, m.y.round() as i32)
            };
            return Some((
                origin.0 + r.x.round() as i32,
                origin.1 + r.y.round() as i32,
                r.w.round() as i32,
                r.h.round() as i32,
            ));
        }
        None
    }

    /// Zone de saisie de chaque surface (trous de la vidéo) et place de la
    /// fenêtre vidéo ; n'envoie rien si rien n'a changé.
    pub(in crate::ui) fn sync_pip(&mut self) -> Task<Msg> {
        let mut tasks = Vec::new();
        let surfs = self.panorama_surfs();
        self.panorama
            .regions
            .retain(|id, _| surfs.iter().any(|s| s.id == *id));
        for s in &surfs {
            let holes = self.panorama_surface_holes(s, &self.panorama_geo(s));
            if self.panorama.regions.get(&s.id) == Some(&holes) {
                continue;
            }
            self.panorama.regions.insert(s.id, holes.clone());
            let (w, h) = (s.w.ceil() as i32, s.h.ceil() as i32);
            tasks.push(Task::done(Msg::SetInputRegion {
                id: s.id,
                callback: iced_layershell::actions::ActionCallback::new(move |region| {
                    region.add(0, 0, w, h);
                    for r in &holes {
                        region.subtract(
                            r.x.floor() as i32,
                            r.y.floor() as i32,
                            r.w.ceil() as i32,
                            r.h.ceil() as i32,
                        );
                    }
                }),
            }));
        }
        let target = self.pip_target();
        if let Some(v) = self.panorama.video.as_mut()
            && let (Some(addr), Some(rect)) = (v.address.clone(), target)
            && v.placed != Some(rect)
        {
            v.placed = Some(rect);
            tasks.push(Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || crate::hypr::place(&addr, rect))
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                },
                |r| wrap(PMsg::VideoPlaced(r)),
            ));
        }
        Task::batch(tasks)
    }

    fn panorama_save_cfg(&mut self, f: impl FnOnce(&mut crate::panorama::Config)) -> Task<Msg> {
        let mut c = self.panorama_cfg();
        f(&mut c);
        self.settings.panorama = Some(c);
        self.commit()
    }

    pub(super) fn panorama_panes_update(&mut self, m: PMsg) -> Task<Msg> {
        match m {
            PMsg::Layout(l) => {
                // Nouvelle disposition : chaque écran revient à sa vue composée.
                self.panorama.page.clear();
                self.panorama_save_cfg(|c| c.layout = l)
            }
            PMsg::Pane(i, d) => {
                let ids: Vec<&str> = crate::panorama::PAGES.iter().map(|(id, _)| *id).collect();
                self.panorama_save_cfg(|c| {
                    while c.panes.len() <= i {
                        let d = c.pane(c.panes.len()).to_string();
                        c.panes.push(d);
                    }
                    let cur = ids.iter().position(|x| *x == c.panes[i]).unwrap_or(0) as i64;
                    let n = ids.len() as i64;
                    c.panes[i] = ids[(cur + d as i64).rem_euclid(n) as usize].to_string();
                })
            }
            PMsg::VideoOpen(i) => {
                let c = &self.cfg.general.panorama;
                let Some(src) = c.sources.get(i).cloned() else {
                    return Task::none();
                };
                let Some((cmd, prefix)) = video::browser(&c.browser) else {
                    self.panorama.video = Some(Video {
                        name: src.name,
                        url: src.url,
                        prefix: String::new(),
                        address: None,
                        known: Vec::new(),
                        error: Some(
                            crate::i18n::tr(
                                "Aucun navigateur Chromium trouvé (brave, chromium, google-chrome…) : Réglages → Panorama.",
                            )
                            .to_string(),
                        ),
                        placed: None,
                        focus_back: None,
                    });
                    return Task::none();
                };
                let close = self.panorama_close_video();
                let known: Vec<String> = self.clients.iter().map(|c| c.address.clone()).collect();
                self.panorama.video = Some(Video {
                    name: src.name.clone(),
                    url: src.url.clone(),
                    prefix,
                    address: None,
                    known,
                    error: None,
                    placed: None,
                    focus_back: self.active_window.as_ref().map(|(a, _)| a.clone()),
                });
                self.panorama.last_press = Some(std::time::Instant::now());
                let url = src.url;
                Task::batch([
                    close,
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || video::launch(&cmd, &url))
                                .await
                                .unwrap_or_else(|e| Err(e.to_string()))
                        },
                        |r| match r {
                            Ok(()) => wrap(PMsg::VideoPoll(0)),
                            Err(e) => wrap(PMsg::VideoPlaced(Err(e))),
                        },
                    ),
                ])
            }
            PMsg::VideoPoll(n) => {
                let Some(v) = self.panorama.video.as_ref().filter(|v| v.address.is_none()) else {
                    return Task::none();
                };
                if n >= POLL_MAX {
                    let name = v.name.clone();
                    if let Some(v) = self.panorama.video.as_mut() {
                        v.error = Some(crate::tr!(
                            "La fenêtre de {name} n'est pas apparue.",
                            name = name
                        ));
                    }
                    return Task::none();
                }
                let (known, prefix, url) = (v.known.clone(), v.prefix.clone(), v.url.clone());
                Task::perform(
                    async move {
                        tokio::time::sleep(POLL).await;
                        tokio::task::spawn_blocking(move || {
                            hyprpanel_core::ipc::query_json("clients")
                                .ok()
                                .and_then(|c| video::find_new(&c, &known, &prefix, &url))
                        })
                        .await
                        .ok()
                        .flatten()
                    },
                    move |found| match found {
                        Some(a) => wrap(PMsg::VideoFound(Some(a))),
                        None => wrap(PMsg::VideoPoll(n + 1)),
                    },
                )
            }
            PMsg::VideoFound(Some(addr)) => {
                let Some(v) = self.panorama.video.as_mut() else {
                    // Fermée entre-temps : la fenêtre part aussi.
                    return Task::perform(
                        async move {
                            let _ = tokio::task::spawn_blocking(move || video::close(&addr)).await;
                        },
                        |_| Msg::Noop,
                    );
                };
                v.address = Some(addr.clone());
                v.placed = None;
                let back = v.focus_back.clone();
                // Posée là où la zone l'attend (ou sur place si la zone n'est
                // pas visible), puis le clavier rendu à la fenêtre d'avant.
                let rect = self.pip_target();
                if let Some(v) = self.panorama.video.as_mut() {
                    v.placed = rect;
                }
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let r = video::adopt(&addr, rect);
                            if let Some(b) = back {
                                let _ = crate::hypr::focus_window(&b);
                            }
                            r
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| wrap(PMsg::VideoPlaced(r)),
                )
            }
            PMsg::VideoFound(None) => Task::none(),
            PMsg::VideoPlaced(r) => {
                if let Err(e) = r {
                    tracing::warn!("{}", crate::tr!("Panorama : vidéo : {e}", e = e.clone()));
                    if let Some(v) = self.panorama.video.as_mut() {
                        v.error = Some(e);
                    }
                }
                Task::none()
            }
            PMsg::VideoClose => {
                let t = self.panorama_close_video();
                self.panorama.video = None;
                t
            }
            _ => Task::none(),
        }
    }

    /// Ferme la fenêtre vidéo, s'il y en a une.
    fn panorama_close_video(&mut self) -> Task<Msg> {
        match self.panorama.video.take().and_then(|v| v.address) {
            Some(a) => Task::perform(
                async move {
                    let _ = tokio::task::spawn_blocking(move || video::close(&a)).await;
                },
                |_| Msg::Noop,
            ),
            None => Task::none(),
        }
    }

    /// Fenêtre vidéo fermée hors du Panorama (SUPER+Q…) : on l'oublie.
    pub(super) fn panorama_video_gone(&mut self) {
        let gone = self
            .panorama
            .video
            .as_ref()
            .and_then(|v| v.address.as_ref())
            .is_some_and(|a| {
                !self.clients.is_empty() && !self.clients.iter().any(|c| &c.address == a)
            });
        if gone {
            self.panorama.video = None;
        }
    }

    // -- Vue -------------------------------------------------------------------

    pub(super) fn view_panorama_surface(&self, id: Id) -> Element<'_, Msg> {
        let Some(s) = self.panorama_surf(id) else {
            return Space::new().into();
        };
        let geo = self.panorama_geo(&s);
        let (bg, fg, accent) = self.panorama_palette();
        let video_at = (self.panorama_video_host() == Some(s.id))
            .then(|| geo.video_pane().map(|p| p.rect))
            .flatten();
        let scene = fx::Scene {
            key: format!(
                "{}:{}",
                self.cfg.general.panorama.layout.id(),
                geo.panes
                    .iter()
                    .map(|p| p.widget.as_str())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            bg,
            fg,
            accent,
            animate: !self.panorama_motion().is_zero(),
            playing: self.cfg.general.panorama.design.effects
                && !self.panorama_asleep()
                && self.panorama.media.as_ref().is_some_and(|m| m.playing),
            holes: self.panorama_surface_holes(&s, &geo),
            design: self.cfg.general.panorama.design.clone(),
            wallpaper: self
                .panorama
                .wallpaper
                .as_ref()
                .map(|(_, h, size)| (h.clone(), *size)),
        };
        let lay = self.panorama_layout_at(&s.key);
        let axis = if s.w >= s.h {
            Axis::Horizontal
        } else {
            Axis::Vertical
        };
        // La vue composée est l'entrée 0 du carrousel : on glisse vers les pages.
        let paged = |content| self.panorama_paged(&s.key, axis, content);
        if lay == Layout::Dashboard {
            return fx::stage(paged(self.panorama_dashboard(&s, &geo, video_at)), scene);
        }
        if lay == Layout::Single {
            let p = &geo.panes[0];
            let key = s.key.clone();
            let page = self.panorama_pane_view(&s, &geo, p, true, video_at);
            let paged = swipe(page, axis, move |d| wrap(PMsg::Swipe(key.clone(), d)))
                .motion(self.panorama_motion())
                .on_press(wrap(PMsg::Pressed));
            if p.widget == "video" {
                // Bandeau et contenu portent leur fond : le trou reste transparent.
                return fx::stage(paged, scene);
            }
            let mut layers: Vec<Element<'_, Msg>> = Vec::new();
            if let Some((_, h)) = &self.panorama.cover
                && p.widget == "media"
            {
                // Pochette en fond, très atténuée.
                layers.push(
                    image(h.clone())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .content_fit(iced::ContentFit::Cover)
                        .opacity(0.065_f32)
                        .into(),
                );
            }
            layers.push(paged.into());
            // Commandes en bas à droite : rien du lecteur n'y est posé.
            if self.cfg.general.panorama.design.controls {
                layers.push(
                    container(self.panorama_controls(&s, &geo, fg, s.w < geo.target * 8.0))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .padding(geo.target * 0.15)
                        .align_x(iced::alignment::Horizontal::Right)
                        .align_y(iced::alignment::Vertical::Bottom)
                        .into(),
                );
            }
            return fx::stage(
                stack(layers).width(Length::Fill).height(Length::Fill),
                scene,
            );
        }
        // Écran découpé : zones rangée par rangée (même `y`), séparées par
        // des bandes du fond (les écarts ne laissent rien voir dessous).
        let band = |w: f32, h: f32| -> Element<'_, Msg> {
            container(Space::new())
                .width(w)
                .height(h)
                .style(move |_| container::Style {
                    background: Some(bg.into()),
                    ..Default::default()
                })
                .into()
        };
        let gap = self.cfg.general.panorama.design.gap;
        let mut rows: Vec<Vec<(usize, &Pane)>> = Vec::new();
        for (i, p) in geo.panes.iter().enumerate() {
            match rows.last_mut() {
                Some(r) if (r[0].1.rect.y - p.rect.y).abs() < 0.5 => r.push((i, p)),
                _ => rows.push(vec![(i, p)]),
            }
        }
        let n = geo.panes.len();
        let mut lines: Vec<Element<'_, Msg>> = Vec::new();
        for (ri, r) in rows.iter().enumerate() {
            if ri > 0 {
                lines.push(band(s.w, gap));
            }
            let h = r[0].1.rect.h;
            let mut items: Vec<Element<'_, Msg>> = Vec::new();
            for (k, (i, p)) in r.iter().enumerate() {
                if k > 0 {
                    items.push(band(gap, h));
                }
                items.push(self.panorama_pane_view(&s, &geo, p, i + 1 == n, video_at));
            }
            lines.push(row(items).into());
        }
        let content: Element<'_, Msg> = column(lines).into();
        fx::stage(paged(content), scene)
    }

    /// Contenu qu'on fait glisser vers l'entrée voisine du carrousel.
    fn panorama_paged<'a>(
        &'a self,
        key: &str,
        axis: Axis,
        content: Element<'a, Msg>,
    ) -> Element<'a, Msg> {
        let key = key.to_string();
        swipe(content, axis, move |d| wrap(PMsg::Swipe(key.clone(), d)))
            .motion(self.panorama_motion())
            .on_press(wrap(PMsg::Pressed))
            .into()
    }

    /// Une zone : son bandeau (s'il en a un) et son widget.
    pub(super) fn panorama_pane_view(
        &self,
        s: &Surf,
        geo: &Geo,
        p: &Pane,
        last: bool,
        video_at: Option<Rect>,
    ) -> Element<'_, Msg> {
        let (bg, fg, accent) = self.panorama_palette();
        let content = geo.content(p);
        let mut z = layout::zones(content.w, content.h, s.ppm);
        z.unit *= self.cfg.general.panorama.design.text_scale;
        let body: Element<'_, Msg> = match p.widget.as_str() {
            "calendar" => self.panorama_calendar(iced::Size::new(content.w, content.h)),
            "shortcuts" => self.panorama_shortcuts(iced::Size::new(content.w, content.h)),
            "system" if self.cfg.general.panorama.design.system_style != "cards" => {
                super::instruments::metrics(
                    &self.cfg.general.panorama.design.system_style,
                    &self.panorama.sys,
                    &self.panorama.history,
                    self.cfg.general.panorama.design.history_seconds as usize,
                    (fg, accent),
                    self.panorama_font(),
                    self.cfg.general.panorama.design.text_scale,
                )
            }
            "system" => self.panorama_system(
                &z,
                fg,
                iced::Size::new(content.w, content.h),
                p.index.is_none(),
            ),
            "clock" => self.panorama_clock(&z, fg),
            "video" => {
                let state = match &self.panorama.video {
                    _ if video_at != Some(p.rect) => VideoState::Elsewhere,
                    Some(v) if v.address.is_some() => VideoState::Hole,
                    Some(v) if v.error.is_none() => VideoState::Opening(&v.name),
                    _ => VideoState::Tiles,
                };
                self.panorama_video(&z, content, state, fg)
            }
            "media" if self.cfg.general.panorama.design.media_style == "vinyl" => {
                self.panorama_vinyl(iced::Size::new(content.w, content.h))
            }
            "media" if self.cfg.general.panorama.design.media_style == "compact" => {
                self.panorama_compact_media(iced::Size::new(content.w, content.h))
            }
            id if id.starts_with("widget:") => self
                .cfg
                .widgets
                .iter()
                .find(|w| w.id == id.trim_start_matches("widget:"))
                .map(|w| self.view_widget(s.id, w, 1.0).1)
                .unwrap_or_else(|| {
                    text(crate::i18n::tr("Widget indisponible"))
                        .color(fg)
                        .into()
                }),
            _ => self.panorama_media(&z, fg),
        };
        let hole = p.widget == "video"
            && video_at == Some(p.rect)
            && self
                .panorama
                .video
                .as_ref()
                .is_some_and(|v| v.address.is_some());
        let body: Element<'_, Msg> = if hole || !Geo::has_header(p) {
            body
        } else {
            container(body)
                .width(content.w)
                .height(content.h)
                .style(move |_| fx::glass(bg, fg, self.panorama_palette().2, 18.0))
                .into()
        };
        if !Geo::has_header(p) {
            return container(body).width(p.rect.w).height(p.rect.h).into();
        }
        column![
            self.panorama_pane_header(s, geo, p, last, fg, bg),
            container(body).width(content.w).height(content.h)
        ]
        .width(p.rect.w)
        .height(p.rect.h)
        .into()
    }

    /// Bandeau d'une zone : ◀ widget ▶ (écran partagé), commandes de la
    /// vidéo, et sur la dernière zone (ou la page Vidéo) celles du Panorama.
    fn panorama_pane_header(
        &self,
        s: &Surf,
        geo: &Geo,
        p: &Pane,
        last: bool,
        fg: Color,
        bg: Color,
    ) -> Element<'_, Msg> {
        let t = geo.target;
        let small = t * 0.8;
        let label = crate::i18n::tr(crate::panorama::page_label(&p.widget));
        let mut r = row![].spacing(8).align_y(Alignment::Center);
        if let Some(i) = p.index {
            r = r
                .push(self.panorama_text_button("◀", small, fg, wrap(PMsg::Pane(i, -1))))
                .push(
                    text(label)
                        .size(t * 0.36)
                        .color(fg)
                        .font(self.panorama_font())
                        .wrapping(text::Wrapping::None),
                )
                .push(self.panorama_text_button("▶", small, fg, wrap(PMsg::Pane(i, 1))));
        } else {
            r = r.push(
                text(label)
                    .size(t * 0.36)
                    .color(fg)
                    .font(self.panorama_font())
                    .wrapping(text::Wrapping::None),
            );
        }
        if p.widget == "video"
            && let Some(v) = &self.panorama.video
        {
            r = r.push(
                text(format!("· {}", v.name))
                    .size(t * 0.32)
                    .color(Color { a: 0.7, ..fg })
                    .font(self.panorama_font())
                    .wrapping(text::Wrapping::None),
            );
            r = r.push(Space::new().width(Length::Fill));
            r = r.push(self.panorama_text_button(
                crate::i18n::tr("Sources"),
                small,
                fg,
                wrap(PMsg::VideoClose),
            ));
        } else {
            r = r.push(Space::new().width(Length::Fill));
        }
        if last || p.index.is_none() {
            r = r.push(self.panorama_controls(s, geo, fg, p.rect.w < geo.target * 8.0));
        }
        container(r)
            .padding([0.0, t * 0.15])
            .width(p.rect.w)
            .height(geo.header)
            .align_y(iced::alignment::Vertical::Center)
            .style(move |_| container::Style {
                background: Some(bg.into()),
                ..Default::default()
            })
            .clip(true)
            .into()
    }

    /// Commandes du Panorama : disposition 1 | 1×2 | 1×3 | 2×2 | 3×3 | ✦, points des pages (une
    /// page à la fois), escamotage (ou fermeture de l'aperçu).
    pub(super) fn panorama_controls(
        &self,
        s: &Surf,
        geo: &Geo,
        fg: Color,
        compact: bool,
    ) -> Element<'_, Msg> {
        let accent = self.panorama_palette().2;
        let t = geo.target;
        let cur = self.cfg.general.panorama.layout;
        let mut seg = row![].spacing(2);
        for l in Layout::ALL.into_iter().filter(|l| !compact || *l == cur) {
            let on = l == cur;
            seg = seg.push(
                button(
                    container(
                        text(l.short())
                            .size(
                                t * if l.short().chars().count() > 1 {
                                    0.26
                                } else {
                                    0.32
                                },
                            )
                            .font(self.panorama_font())
                            .color(if on {
                                super::super::design::on_accent(
                                    accent,
                                    fg,
                                    self.panorama_palette().0,
                                )
                            } else {
                                fg
                            }),
                    )
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
                )
                .width(
                    t * if l.short().chars().count() > 1 {
                        0.95
                    } else {
                        0.7
                    },
                )
                .height(t * 0.7)
                .on_press(wrap(PMsg::Layout(if compact { cur.next() } else { l })))
                .style(move |_, st| button::Style {
                    background: Some(
                        if on {
                            accent
                        } else {
                            Color {
                                a: if matches!(st, button::Status::Pressed) {
                                    0.25
                                } else {
                                    0.08
                                },
                                ..fg
                            }
                        }
                        .into(),
                    ),
                    border: Border::default().rounded(t * 0.2),
                    ..Default::default()
                }),
            );
        }
        let seg: Element<'_, Msg> = if compact {
            iced::widget::tooltip(
                seg,
                text(crate::i18n::tr("Disposition suivante")),
                iced::widget::tooltip::Position::Top,
            )
            .into()
        } else {
            seg.into()
        };
        let mut r = row![seg].spacing(8).align_y(Alignment::Center);
        // Points du carrousel : vue composée (s'il y en a une), puis les pages.
        let n = self.panorama_carousel();
        if n > 1 {
            let i = self.panorama.page.get(&s.key).copied().unwrap_or(0);
            r = r.push(self.panorama_dots(n, i, &s.key, fg, accent));
        }
        let hide = button(
            // Aperçu : le bouton ferme la fenêtre ; sinon il escamote.
            container(
                self.icon_named(
                    if s.key == PREVIEW {
                        "window-close-symbolic"
                    } else {
                        "go-down-symbolic"
                    },
                    t * 0.3,
                    fg,
                )
                .unwrap_or_else(|| text("▾").size(t * 0.4).color(fg).into()),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill),
        )
        .width(t * 0.8)
        .height(t * 0.8)
        .on_press(wrap(PMsg::Hide(s.key.clone())))
        .style(move |_, st| button::Style {
            background: Some(
                Color {
                    a: if matches!(st, button::Status::Pressed | button::Status::Hovered) {
                        0.18
                    } else {
                        0.08
                    },
                    ..fg
                }
                .into(),
            ),
            border: Border::default().rounded(t),
            ..Default::default()
        });
        r.push(self.panorama_feedback(hide)).into()
    }

    fn panorama_text_button<'a>(
        &self,
        label: impl text::IntoFragment<'a>,
        h: f32,
        fg: Color,
        msg: Msg,
    ) -> Element<'a, Msg> {
        self.panorama_feedback(
            button(
                container(
                    text(label)
                        .size(h * 0.42)
                        .font(self.panorama_font())
                        .color(fg),
                )
                .center_y(Length::Fill)
                .padding([0.0, h * 0.25]),
            )
            .height(h)
            .on_press(msg)
            .style(move |_, st| button::Style {
                background: Some(
                    Color {
                        a: match st {
                            button::Status::Pressed => 0.28,
                            button::Status::Hovered => 0.16,
                            _ => 0.08,
                        },
                        ..fg
                    }
                    .into(),
                ),
                border: Border::default().rounded(h / 2.0),
                ..Default::default()
            }),
        )
    }

    /// Widget Vidéo : tuiles des sources, ouverture, ou trou de la fenêtre.
    fn panorama_video(
        &self,
        z: &layout::Zones,
        area: Rect,
        state: VideoState<'_>,
        fg: Color,
    ) -> Element<'_, Msg> {
        let dim = Color { a: 0.65, ..fg };
        let accent = self.panorama_palette().2;
        let centered = |e: Element<'static, Msg>| -> Element<'_, Msg> {
            container(e)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        };
        match state {
            VideoState::Hole => Space::new().width(area.w).height(area.h).into(),
            VideoState::Opening(name) => centered(
                text(crate::tr!("Ouverture de {name}…", name = name.to_string()))
                    .size(z.unit * 0.07)
                    .color(dim)
                    .into(),
            ),
            VideoState::Elsewhere => centered(
                text(crate::i18n::tr("La vidéo est dans l'autre zone."))
                    .size(z.unit * 0.06)
                    .color(dim)
                    .into(),
            ),
            VideoState::Tiles => {
                let c = &self.cfg.general.panorama;
                let mut col = column![].spacing(z.pad / 2.0);
                if let Some(e) = self.panorama.video.as_ref().and_then(|v| v.error.as_ref()) {
                    col = col.push(
                        text(e.clone())
                            .size(z.unit * 0.045)
                            .color(Color::from_rgb8(0xff, 0xb8, 0x6c))
                            .font(self.panorama_font()),
                    );
                }
                let tile_w = (z.target * 3.6).max(160.0);
                let cols = ((area.w - z.pad) / (tile_w + z.pad / 2.0)).floor().max(1.0) as usize;
                let th = z.target * 1.3;
                let mut line = row![].spacing(z.pad / 2.0);
                for (i, src) in c.sources.iter().enumerate() {
                    line = line.push(
                        self.panorama_feedback(
                            button(
                                container(
                                    column![
                                        text(src.name.clone())
                                            .size(th * 0.3)
                                            .font(iced::Font {
                                                weight: iced::font::Weight::Semibold,
                                                ..self.panorama_font()
                                            })
                                            .color(fg)
                                            .wrapping(text::Wrapping::None),
                                        text(video::host(&src.url))
                                            .size(th * 0.17)
                                            .font(self.panorama_font())
                                            .color(dim)
                                    ]
                                    .spacing(6),
                                )
                                .center_x(Length::Fill)
                                .center_y(Length::Fill),
                            )
                            .width(Length::Fill)
                            .height(th)
                            .on_press(wrap(PMsg::VideoOpen(i)))
                            .style(move |_, st| button::Style {
                                background: Some(
                                    match st {
                                        button::Status::Pressed => Color { a: 0.5, ..accent },
                                        button::Status::Hovered => Color { a: 0.16, ..fg },
                                        _ => Color { a: 0.08, ..fg },
                                    }
                                    .into(),
                                ),
                                border: Border {
                                    color: fx::alpha(
                                        accent,
                                        if matches!(st, button::Status::Hovered) {
                                            0.65
                                        } else {
                                            0.18
                                        },
                                    ),
                                    width: 1.0,
                                    radius: 16.0.into(),
                                },
                                shadow: iced::Shadow {
                                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.15),
                                    offset: iced::Vector::new(0.0, 5.0),
                                    blur_radius: 12.0,
                                },
                                ..Default::default()
                            }),
                        ),
                    );
                    if (i + 1) % cols == 0 {
                        col = col.push(line);
                        line = row![].spacing(z.pad / 2.0);
                    }
                }
                if !c.sources.len().is_multiple_of(cols) {
                    // Dernière ligne : cases vides pour garder la largeur des tuiles.
                    for _ in 0..(cols - c.sources.len() % cols) {
                        line = line.push(Space::new().width(Length::Fill));
                    }
                    col = col.push(line);
                }
                if c.sources.is_empty() {
                    col = col.push(
                        text(crate::i18n::tr(
                            "Aucune source : ajoutez-en dans Réglages → Panorama.",
                        ))
                        .size(z.unit * 0.05)
                        .color(dim),
                    );
                }
                container(iced::widget::scrollable(col).height(Length::Fill))
                    .padding(z.pad / 2.0)
                    .width(area.w)
                    .height(area.h)
                    .into()
            }
        }
    }
}
