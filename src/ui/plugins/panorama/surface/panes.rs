//! Zones du Panorama (une page, ou l'écran partagé en 2 ou 3) et widget
//! Vidéo : une fenêtre de navigateur posée exactement dans sa zone, la
//! surface y laissant un trou (image et saisie).

use std::time::Duration;

use iced::widget::{Space, button, column, container, image, row, stack, text};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Task};

use super::swipe::{Axis, Edge, swipe};
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
    pub borrowed: Option<crate::panorama::capture::Borrowed>,
}

/// Lancement abandonné (autre source choisie pendant l'ouverture) : sa
/// fenêtre est fermée dès qu'elle apparaît.
#[derive(Debug, Clone)]
pub struct Stray {
    pub prefix: String,
    pub url: String,
    pub known: Vec<String>,
    pub since: std::time::Instant,
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
    /// Sans bandeau : le lecteur des profils Vidéo et Musique garde toute
    /// sa hauteur (16:9), la colonne des services porte les commandes.
    pub bare: bool,
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
        !p.bare && (p.index.is_some() || p.widget == "video")
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
    /// Aucune vidéo, plateformes listées à côté : le mode d'emploi.
    Guide,
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
        // Zone Panorama : la bande, pas l'écran entier (la taille réelle de
        // la surface peut manquer, ou rester celle d'avant la zone).
        let zone = self
            .cfg
            .general
            .panorama
            .zone
            .as_ref()
            .filter(|z| &z.screen == mon)
            .map(|z| {
                let (_, _, w, h) = z.rect(m.width as f32, m.height as f32);
                (w, h)
            });
        let (w, h) = match (real, zone) {
            // Taille réelle cohérente avec la bande (même orientation).
            (Some(r), Some(z)) if (r.0 >= r.1) == (z.0 >= z.1) => r,
            (_, Some(z)) => z,
            (Some(r), None) => r,
            (None, None) => (m.width as f32, m.height as f32),
        };
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
        // Zones d'une composition (fractions de l'écran) en rectangles.
        let map = |tiles: &[crate::panorama::design::Tile], bare: bool| -> Vec<Pane> {
            let pad = c.design.padding;
            let gap = c.design.gap;
            tiles
                .iter()
                .map(|t| {
                    let (x, y, w, h) = if s.w >= s.h {
                        (t.x, t.y, t.w, t.h)
                    } else {
                        (t.y, t.x, t.h, t.w)
                    };
                    let mut rect = Rect {
                        x: pad + x * (s.w - 2.0 * pad) + gap / 2.0,
                        y: pad + y * (s.h - 2.0 * pad) + gap / 2.0,
                        w: (w * (s.w - 2.0 * pad) - gap).max(1.0),
                        h: (h * (s.h - 2.0 * pad) - gap).max(1.0),
                    };
                    // Lecteur des profils Vidéo et Musique : toute la hauteur
                    // de l'écran, bord à bord (sa largeur est déjà 16:9 de
                    // cette hauteur, voir `player_tiles`).
                    if bare && t.widget == "video" {
                        if h >= 0.99 {
                            (rect.y, rect.h) = (0.0, s.h);
                        } else if w >= 0.99 {
                            (rect.x, rect.w) = (0.0, s.w);
                        }
                    }
                    Pane {
                        rect,
                        widget: t.widget.clone(),
                        index: None,
                        bare,
                    }
                })
                .collect()
        };
        let full = |widget: &str| {
            vec![Pane {
                rect: Rect {
                    x: 0.0,
                    y: 0.0,
                    w: s.w,
                    h: s.h,
                },
                widget: widget.to_string(),
                index: None,
                bare: false,
            }]
        };
        if self.panorama.capture_host == Some(s.id) {
            return Geo {
                panes: full("video"),
                header: target,
                target,
            };
        }
        let panes = match self.panorama_layout_at(&s.key) {
            // Pages : leur composition ; un seul widget sur tout l'écran
            // garde l'écran entier. La veille passe à l'horloge.
            Layout::Single => match self.panorama_page_at(&s.key) {
                _ if self.panorama_asleep() => full("clock"),
                Some(p) if p.tiles.len() == 1 && p.tiles[0].w >= 0.99 && p.tiles[0].h >= 0.99 => {
                    full(&p.tiles[0].widget)
                }
                Some(p) => map(&p.tiles, false),
                None => full("clock"),
            },
            Layout::Dashboard => {
                let player = matches!(c.design.shortcut_profile.as_str(), "video" | "audio");
                let mut panes = map(&c.design.active_tiles(s.w, s.h), player);
                self.panorama_embed_media(&mut panes);
                panes
            }
            _ => layout::split_panes(&c.split().unwrap_or_default(), s.w, s.h, c.design.gap)
                .into_iter()
                .enumerate()
                .map(|(i, rect)| Pane {
                    rect,
                    widget: c.pane(i).to_string(),
                    index: Some(i),
                    bare: false,
                })
                .collect(),
        };
        Geo {
            panes,
            header: target,
            target,
        }
    }

    /// Profil Raccourcis : un média en cours (vidéo, radio, musique) prend
    /// la place des touches, sous le sélecteur de profils, en 16:9 à la
    /// largeur de la colonne. La page Fenêtres du bloc garde la colonne.
    fn panorama_embed_media(&self, panes: &mut Vec<Pane>) {
        let c = &self.cfg.general.panorama;
        if c.design.shortcut_profile != "system" || self.panorama.shortcuts_page != 0 {
            return;
        }
        let widget = if self.panorama.video.is_some() || self.panorama.radio.is_some() {
            "video"
        } else if self.panorama.media.as_ref().is_some_and(|m| m.playing) {
            "media"
        } else {
            return;
        };
        let Some(col) = panes
            .iter()
            .find(|p| p.widget == "shortcuts")
            .map(|p| p.rect)
        else {
            return;
        };
        // Sous le sélecteur, sans points de pages : la vidéo est centrée
        // dans la hauteur qui reste.
        let (top, _) = self.panorama_shortcuts_frame(col.h);
        let bottom = 8.0;
        let room_h = (col.h - top - bottom).max(40.0);
        let w = (col.w - 16.0).max(40.0).min(room_h * 16.0 / 9.0);
        let h = w * 9.0 / 16.0;
        let top = top + (room_h - h) / 2.0;
        // Une seule zone vidéo : celle de la colonne.
        panes.retain(|p| p.widget != widget);
        panes.push(Pane {
            rect: Rect {
                x: col.x + (col.w - w) / 2.0,
                y: col.y + top,
                w,
                h,
            },
            widget: widget.to_string(),
            index: None,
            bare: true,
        });
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
    pub(super) fn panorama_surfs(&self) -> Vec<Surf> {
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
                // Zone Panorama : la bande n'est pas au coin de l'écran.
                let (zx, zy) = match self
                    .cfg
                    .general
                    .panorama
                    .zone
                    .as_ref()
                    .filter(|z| z.screen == m.name)
                {
                    Some(z) => {
                        let (x, y, _, _) = z.rect(m.width as f32, m.height as f32);
                        (x, y)
                    }
                    None => (0.0, 0.0),
                };
                ((m.x + zx).round() as i32, (m.y + zy).round() as i32)
            };
            let zone = (
                origin.0 + r.x.round() as i32,
                origin.1 + r.y.round() as i32,
                r.w.round() as i32,
                r.h.round() as i32,
            );
            // Cadrage de la source : seule la vidéo reste dans la zone.
            return Some(match self.panorama_current_source() {
                Some(src) => src.framed(zone),
                None => zone,
            });
        }
        None
    }

    /// Source ouverte dans le lecteur (vidéo ou musique).
    pub(super) fn panorama_current_source(&self) -> Option<&crate::panorama::config::Source> {
        let url = &self.panorama.video.as_ref()?.url;
        let c = &self.cfg.general.panorama;
        c.sources
            .iter()
            .chain(&c.music_sources)
            .find(|s| &s.url == url)
    }

    /// Cadrage de la source ouverte : zoom et décalage, mémorisés.
    fn panorama_frame(&mut self, op: &str) -> Task<Msg> {
        let Some(url) = self.panorama.video.as_ref().map(|v| v.url.clone()) else {
            return Task::none();
        };
        if op == "done" {
            self.panorama.framing = false;
            return Task::none();
        }
        let mut c = self.panorama_cfg();
        let Some(src) = c
            .sources
            .iter_mut()
            .chain(c.music_sources.iter_mut())
            .find(|s| s.url == url)
        else {
            return Task::none();
        };
        let step = 0.02;
        match op {
            "zoom+" => src.zoom = (src.zoom + 0.05).min(4.0),
            "zoom-" => src.zoom = (src.zoom - 0.05).max(1.0),
            // La vidéo suit le doigt : « gauche » décale la fenêtre à gauche.
            "left" => src.dx = (src.dx - step).max(-2.0),
            "right" => src.dx = (src.dx + step).min(2.0),
            "up" => src.dy = (src.dy - step).max(-2.0),
            "down" => src.dy = (src.dy + step).min(2.0),
            "reset" => {
                src.zoom = 1.0;
                src.dx = 0.0;
                src.dy = 0.0;
            }
            _ => return Task::none(),
        }
        // Valeurs rondes dans la config.
        src.zoom = (src.zoom * 100.0).round() / 100.0;
        src.dx = (src.dx * 100.0).round() / 100.0;
        src.dy = (src.dy * 100.0).round() / 100.0;
        self.panorama_save_cfg(|cfg| *cfg = c)
    }

    /// Zone de saisie de chaque surface (trous de la vidéo) et place de la
    /// fenêtre vidéo ; n'envoie rien si rien n'a changé.
    pub(in crate::ui) fn sync_pip(&mut self) -> Task<Msg> {
        let mut tasks = Vec::new();
        // Mode capture (vidéo sur tout le Panorama) quitté dès qu'une zone
        // l'accueille : profil Raccourcis, page avec lecteur…
        if let Some(host) = self.panorama.capture_host {
            self.panorama.capture_host = None;
            if self.pip_target().is_none() {
                self.panorama.capture_host = Some(host);
            }
        }
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
        // Fenêtre empruntée rendue seulement quand plus aucun Panorama ne
        // s'affiche : une page sans lecteur la garde sous la surface (le son
        // continue, la pastille « en cours » le dit) et le retour à la page
        // du lecteur la remontre.
        if target.is_none()
            && surfs.is_empty()
            && self
                .panorama
                .video
                .as_ref()
                .is_some_and(|v| v.borrowed.is_some())
        {
            tasks.push(self.panorama_close_video());
        }
        if let Some(v) = self.panorama.video.as_mut()
            && let (Some(addr), Some(rect)) = (v.address.clone(), target)
            && v.placed != Some(rect)
        {
            v.placed = Some(rect);
            let borrowed = v.borrowed.clone();
            tasks.push(Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || match borrowed {
                        Some(b) => b.place(rect),
                        None => {
                            video::anchor(&addr, rect).and_then(|_| crate::hypr::place(&addr, rect))
                        }
                    })
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()))
                },
                |r| wrap(PMsg::VideoPlaced(r)),
            ));
        }
        Task::batch(tasks)
    }

    pub(super) fn panorama_save_cfg(
        &mut self,
        f: impl FnOnce(&mut crate::panorama::Config),
    ) -> Task<Msg> {
        let mut c = self.panorama_cfg();
        f(&mut c);
        self.settings.panorama = Some(c);
        self.commit()
    }

    pub(super) fn panorama_panes_update(&mut self, m: PMsg) -> Task<Msg> {
        match m {
            PMsg::Profile(id) => {
                self.panorama.last_press = Some(std::time::Instant::now());
                // Changer de profil ne ferme pas la lecture : la vidéo reste
                // dans le Panorama (lecteur du profil Musique, ou cachée sous
                // un profil sans lecteur, pastille « en cours »). Seuls
                // « Fermer » ou une autre source l'arrêtent.
                let save = self.panorama_save_cfg(|c| {
                    let _ = c.design.set("shortcut_profile", &id);
                });
                Task::batch([save, self.sync_pip()])
            }
            PMsg::Layout(l) => {
                // Nouvelle disposition : chaque écran revient à sa vue composée.
                self.panorama.page.clear();
                self.panorama.picker = None;
                self.panorama_save_cfg(|c| c.layout = l)
            }
            PMsg::Pane(i, d) => {
                let ids: Vec<&str> = crate::panorama::PAGES.iter().map(|(id, _)| *id).collect();
                self.panorama_save_cfg(|c| {
                    let cur = ids.iter().position(|x| *x == c.pane(i)).unwrap_or(0) as i64;
                    let n = ids.len() as i64;
                    let l = c.layout;
                    c.set_pane(l, i, ids[(cur + d as i64).rem_euclid(n) as usize]);
                })
            }
            PMsg::VideoOpen(i) => {
                let Some(src) = self.panorama_sources().get(i).cloned() else {
                    return Task::none();
                };
                self.panorama_open(src)
            }
            PMsg::PlayUrl(name, url) => self.panorama_open(crate::panorama::config::Source {
                name,
                url,
                ..Default::default()
            }),
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
                if self
                    .panorama
                    .video
                    .as_ref()
                    .is_some_and(|v| v.address.is_some())
                {
                    return Task::none();
                }
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
                Task::batch([Task::perform(
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
                )])
            }
            PMsg::Orphans(list) => {
                // Une seule source : la première reconnue est reprise, les
                // autres fermées.
                let mut close = Vec::new();
                for (addr, class) in list {
                    let c = &self.cfg.general.panorama;
                    let src = c
                        .sources
                        .iter()
                        .chain(&c.music_sources)
                        .find(|s| video::matches(&class, "", &s.url))
                        .cloned();
                    match src {
                        Some(src) if self.panorama.video.is_none() => {
                            let prefix = video::browser(&c.browser)
                                .map(|(_, p)| p)
                                .unwrap_or_default();
                            self.panorama.video = Some(Video {
                                borrowed: None,
                                name: src.name,
                                url: src.url,
                                prefix,
                                address: Some(addr),
                                known: Vec::new(),
                                error: None,
                                placed: None,
                                focus_back: None,
                            });
                        }
                        _ if self
                            .panorama
                            .video
                            .as_ref()
                            .and_then(|v| v.address.as_ref())
                            == Some(&addr) => {}
                        _ => close.push(addr),
                    }
                }
                Task::batch(close.into_iter().map(|a| {
                    Task::perform(
                        async move {
                            let _ = tokio::task::spawn_blocking(move || video::close(&a)).await;
                        },
                        |_| Msg::Noop,
                    )
                }))
            }
            PMsg::MusicTab(t) => {
                self.panorama.last_press = Some(std::time::Instant::now());
                self.panorama.music_tab = t;
                if t == "radio" {
                    return self.panorama_panes_update(PMsg::RadioLogos(false));
                }
                Task::none()
            }
            PMsg::RadioLogos(force) => {
                if self.panorama.radio_logos_tried && !force {
                    return Task::none();
                }
                self.panorama.radio_logos_tried = true;
                let radios: Vec<_> = self
                    .cfg
                    .general
                    .panorama
                    .radios
                    .iter()
                    .filter(|r| r.icon.is_empty() || !std::path::Path::new(&r.icon).is_file())
                    .map(|r| crate::panorama::config::Source {
                        icon: String::new(),
                        ..r.clone()
                    })
                    .collect();
                if radios.is_empty() {
                    return Task::none();
                }
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::panorama::radio::favourite_logos(&radios)
                        })
                        .await
                        .unwrap_or_default()
                    },
                    |l| wrap(PMsg::RadioLogosFound(l)),
                )
            }
            PMsg::RadioLogosFound(found) => {
                if found.is_empty() {
                    return Task::none();
                }
                let icons: Vec<Task<Msg>> = found.iter().map(|(_, i)| self.want_icon(i)).collect();
                let save = self.panorama_save_cfg(move |c| {
                    for (url, icon) in &found {
                        if let Some(r) = c.radios.iter_mut().find(|r| &r.url == url) {
                            r.icon = icon.clone();
                        }
                    }
                });
                Task::batch(icons.into_iter().chain([save]))
            }
            PMsg::RadioPoll(url, n) => Task::perform(
                async move {
                    tokio::time::sleep(std::time::Duration::from_millis(if n == 0 {
                        300
                    } else {
                        500
                    }))
                    .await;
                    let st = tokio::task::spawn_blocking(crate::panorama::radio_play::status)
                        .await
                        .unwrap_or(crate::panorama::radio_play::Status::Dead);
                    (url, st)
                },
                move |(url, st)| wrap(PMsg::RadioStatus(url, n, st)),
            ),
            PMsg::RadioStatus(url, n, st) => {
                use crate::panorama::radio_play::Status;
                // Une autre radio a pris la place : suivi abandonné.
                let Some((name, _)) = self.panorama.radio.clone().filter(|r| r.1 == url) else {
                    return Task::none();
                };
                match st {
                    Status::Playing => {
                        self.panorama.radio_connecting = false;
                        Task::none()
                    }
                    // Environ 20 s pour se connecter, puis abandon.
                    Status::Connecting if n < 40 => Task::done(wrap(PMsg::RadioPoll(url, n + 1))),
                    Status::Connecting => self.panorama_panes_update(PMsg::RadioFailed(
                        url,
                        crate::tr!("{name} ne répond pas.", name = name),
                    )),
                    Status::Dead => self.panorama_panes_update(PMsg::RadioFailed(
                        url,
                        crate::tr!("Impossible de lire {name} : flux injoignable.", name = name),
                    )),
                }
            }
            PMsg::RadioFailed(url, why) => {
                if self.panorama.radio.as_ref().is_some_and(|r| r.1 == url) {
                    self.panorama.radio = None;
                    self.panorama.radio_connecting = false;
                    crate::panorama::radio_play::stop();
                    crate::panorama::radio_play::remember(None);
                }
                tracing::warn!("Panorama : webradio : {why}");
                self.panorama.radio_error = Some(why);
                Task::none()
            }
            PMsg::RadioSearch(q, play) => {
                self.panorama.music_tab = "radio";
                self.panorama.radio_busy = true;
                self.panorama.radio_error = None;
                let query = q.clone();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || crate::panorama::radio::search(&query))
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    move |r| wrap(PMsg::RadioFound(q.clone(), play, r)),
                )
            }
            PMsg::RadioFound(q, play, r) => {
                self.panorama.radio_busy = false;
                match r {
                    Ok(l) => {
                        let first = l.first().cloned();
                        if l.is_empty() {
                            self.panorama.radio_error = Some(crate::tr!(
                                "Aucune radio pour « {q} ».",
                                q = crate::panorama::radio::query_label(&q)
                            ));
                        }
                        let icons: Vec<Task<Msg>> = l
                            .iter()
                            .filter(|s| !s.icon.is_empty())
                            .map(|s| s.icon.clone())
                            .collect::<Vec<_>>()
                            .iter()
                            .map(|i| self.want_icon(i))
                            .collect();
                        self.panorama.radio_results = Some((q, l));
                        let open = match first {
                            Some(s) if play => self.panorama_open(s),
                            _ => Task::none(),
                        };
                        Task::batch(icons.into_iter().chain([open]))
                    }
                    Err(e) => {
                        self.panorama.radio_error = Some(e);
                        Task::none()
                    }
                }
            }
            PMsg::RadioFav(name, url) => {
                self.panorama.last_press = Some(std::time::Instant::now());
                // Le logo trouvé par la recherche suit la station.
                let icon = self
                    .panorama
                    .radio_results
                    .iter()
                    .flat_map(|(_, l)| l)
                    .find(|r| r.url == url)
                    .map(|r| r.icon.clone())
                    .unwrap_or_default();
                self.panorama_save_cfg(|c| {
                    if let Some(i) = c.radios.iter().position(|r| r.url == url) {
                        c.radios.remove(i);
                    } else {
                        c.radios.push(crate::panorama::config::Source {
                            name,
                            url,
                            icon,
                            ..Default::default()
                        });
                    }
                })
            }
            PMsg::AutoFull(url, r) => {
                match r {
                    // Méthode qui a marché : mémorisée pour la source.
                    Ok(Some(m)) => {
                        let mut c = self.panorama_cfg();
                        if let Some(src) = c
                            .sources
                            .iter_mut()
                            .chain(c.music_sources.iter_mut())
                            .find(|s| s.url == url && s.fs != m)
                        {
                            src.fs = m;
                            return self.panorama_save_cfg(|cfg| *cfg = c);
                        }
                    }
                    Ok(None) => {}
                    Err(e) => tracing::info!("Panorama : vidéo seule : {e}"),
                }
                Task::none()
            }
            PMsg::Frame(op) => {
                self.panorama.last_press = Some(std::time::Instant::now());
                if op == "start" {
                    self.panorama.framing = !self.panorama.framing;
                    return Task::none();
                }
                self.panorama_frame(op)
            }
            PMsg::CaptureScan(snapshot) => {
                self.panorama.capture_busy = false;
                let Some((clients, media)) = snapshot else {
                    return Task::none();
                };
                if self.panorama_surfs().is_empty() {
                    return Task::none();
                }
                if !self.cfg.plugin_on("panorama")
                    || self
                        .panorama
                        .video
                        .as_ref()
                        .is_some_and(|v| v.address.is_none())
                {
                    return Task::none();
                }
                if let Some(b) = self
                    .panorama
                    .video
                    .as_mut()
                    .and_then(|v| v.borrowed.as_mut())
                    && self.cfg.general.panorama.restore_on_fullscreen_exit
                    && b.exited_fullscreen(&clients)
                {
                    b.return_before_fullscreen();
                    return self.panorama_close_video();
                }
                let current = self
                    .panorama
                    .video
                    .as_ref()
                    .and_then(|v| v.address.as_deref());
                let Some(b) = self.panorama.capture_watch.next(&clients, &media, current) else {
                    return Task::none();
                };
                let close = self.panorama_close_video();
                if self.panorama.radio.take().is_some() {
                    crate::panorama::radio_play::stop();
                    crate::panorama::radio_play::remember(None);
                }
                let addr = b.address.clone();
                // La vidéo est en cours avant de chercher sa place : le profil
                // Raccourcis ne lui ouvre sa zone (sous le sélecteur) qu'à
                // cette condition. Sans zone nulle part, le Panorama entier
                // l'accueille (mode capture).
                self.panorama.video = Some(Video {
                    name: b.title.clone(),
                    url: String::new(),
                    prefix: b.class.clone(),
                    address: Some(addr.clone()),
                    known: Vec::new(),
                    error: None,
                    placed: None,
                    focus_back: None,
                    borrowed: Some(b.clone()),
                });
                if self.pip_target().is_none() {
                    self.panorama.capture_host = self.panorama_surfs().first().map(|s| s.id);
                }
                let Some(rect) = self.pip_target() else {
                    self.panorama.capture_host = None;
                    self.panorama.video = None;
                    return close;
                };
                if let Some(v) = self.panorama.video.as_mut() {
                    v.placed = Some(rect);
                }
                self.panorama.capture_busy = true;
                self.panorama.last_press = Some(std::time::Instant::now());
                Task::batch([
                    close,
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || b.adopt(rect))
                                .await
                                .unwrap_or_else(|e| Err(e.to_string()))
                        },
                        move |r| wrap(PMsg::CapturePlaced(addr.clone(), r)),
                    ),
                ])
            }
            PMsg::CapturePlaced(addr, result) => {
                self.panorama.capture_busy = false;
                if self
                    .panorama
                    .video
                    .as_ref()
                    .and_then(|v| v.address.as_ref())
                    != Some(&addr)
                {
                    return Task::none();
                }
                if let Err(e) = result {
                    tracing::warn!("Panorama : capture : {e}");
                    return self.panorama_close_video();
                }
                if let Some(b) = self
                    .panorama
                    .video
                    .as_mut()
                    .and_then(|v| v.borrowed.as_mut())
                {
                    b.confirm_transfer();
                }
                Task::none()
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
                self.panorama.framing = false;
                if self.panorama.radio.take().is_some() {
                    crate::panorama::radio_play::stop();
                    crate::panorama::radio_play::remember(None);
                }
                let t = self.panorama_close_video();
                self.panorama.video = None;
                t
            }
            _ => Task::none(),
        }
    }

    /// Ouvre une source (service, page de résultats, flux de radio) dans
    /// le lecteur : une seule à la fois.
    fn panorama_open(&mut self, src: crate::panorama::config::Source) -> Task<Msg> {
        // Webradio : flux joué par mpv, sans fenêtre ; le lecteur montre la
        // barre « flux vocal ».
        let c = &self.cfg.general.panorama;
        let radio = c.radios.iter().any(|r| r.url == src.url)
            || self
                .panorama
                .radio_results
                .as_ref()
                .is_some_and(|(_, l)| l.iter().any(|r| r.url == src.url))
            || crate::panorama::radio_play::is_stream(&src.url);
        if radio {
            if self.panorama.radio.as_ref().is_some_and(|r| r.1 == src.url) {
                return Task::none();
            }
            let close = self.panorama_close_video();
            self.panorama.video = None;
            crate::panorama::radio_play::remember(Some((&src.name, &src.url)));
            self.panorama.radio = Some((src.name.clone(), src.url.clone()));
            self.panorama.radio_paused = false;
            self.panorama.radio_connecting = true;
            self.panorama.radio_error = None;
            self.panorama.level_hist.clear();
            self.panorama.last_press = Some(std::time::Instant::now());
            let url = src.url;
            let back = url.clone();
            return Task::batch([
                close,
                Task::perform(
                    async move {
                        crate::widgets::media::pause_all().await;
                        tokio::task::spawn_blocking(move || crate::panorama::radio_play::play(&url))
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    // Lancé : la connexion est suivie jusqu'au son.
                    move |r| match r {
                        Ok(()) => wrap(PMsg::RadioPoll(back.clone(), 0)),
                        Err(e) => wrap(PMsg::RadioFailed(back.clone(), e)),
                    },
                ),
            ]);
        }
        // Autre source : la radio s'arrête.
        if self.panorama.radio.take().is_some() {
            crate::panorama::radio_play::stop();
            crate::panorama::radio_play::remember(None);
        }
        let c = &self.cfg.general.panorama;
        let Some((cmd, prefix)) = video::browser(&c.browser) else {
            self.panorama.video = Some(Video {
                    borrowed: None,
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
        // Une seule source à la fois : la même, déjà ouverte ou en
        // cours d'ouverture, n'est pas relancée ; une autre la remplace.
        if self
            .panorama
            .video
            .as_ref()
            .is_some_and(|v| v.url == src.url && v.error.is_none())
        {
            return Task::none();
        }
        let close = self.panorama_close_video();
        let known: Vec<String> = self.clients.iter().map(|c| c.address.clone()).collect();
        self.panorama.video = Some(Video {
            borrowed: None,
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
                    // Ce qui joue encore (autre lecteur, ancienne fenêtre
                    // pas encore fermée) s'arrête avant le nouveau média.
                    crate::widgets::media::pause_all().await;
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

    /// Ferme la fenêtre vidéo, s'il y en a une.
    fn panorama_close_video(&mut self) -> Task<Msg> {
        self.panorama.capture_host = None;
        // Fenêtre pas encore apparue : fermée à son arrivée.
        if let Some(v) = self
            .panorama
            .video
            .as_ref()
            .filter(|v| v.address.is_none() && !v.prefix.is_empty())
        {
            self.panorama.strays.push(Stray {
                prefix: v.prefix.clone(),
                url: v.url.clone(),
                known: v.known.clone(),
                since: std::time::Instant::now(),
            });
        }
        if let Some(b) = self
            .panorama
            .video
            .as_ref()
            .and_then(|v| v.borrowed.clone())
        {
            b.cancel();
            self.panorama.video = None;
            return Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || b.restore())
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                },
                |r| {
                    if let Err(e) = r {
                        tracing::warn!("Panorama : restitution : {e}");
                    }
                    Msg::Noop
                },
            );
        }
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

    /// Ferme les fenêtres des lancements abandonnés qui viennent d'apparaître
    /// (une seule source audio ou vidéo à la fois) ; oubliées après 2 min.
    pub(super) fn panorama_close_strays(&mut self) -> Task<Msg> {
        if self.panorama.strays.is_empty() {
            return Task::none();
        }
        let current = self.panorama.video.as_ref().and_then(|v| v.address.clone());
        let mut gone = Vec::new();
        self.panorama.strays.retain(|st| {
            let found = self.clients.iter().find(|c| {
                !st.known.contains(&c.address)
                    && current.as_ref() != Some(&c.address)
                    && video::matches(&c.class, &st.prefix, &st.url)
            });
            match found {
                Some(c) => {
                    gone.push(c.address.clone());
                    false
                }
                None => st.since.elapsed() < std::time::Duration::from_secs(120),
            }
        });
        Task::batch(gone.into_iter().map(|a| {
            Task::perform(
                async move {
                    let _ = tokio::task::spawn_blocking(move || video::close(&a)).await;
                },
                |_| Msg::Noop,
            )
        }))
    }

    /// Renouvellement du bail du cadre ; les corrections sont locales au compositeur.
    pub(super) fn panorama_anchor(&self) -> Task<Msg> {
        let Some(v) = self.panorama.video.as_ref() else {
            return Task::none();
        };
        let (Some(addr), Some(rect)) = (v.address.clone(), self.pip_target()) else {
            return Task::none();
        };
        let borrowed = v.borrowed.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || match borrowed {
                    Some(b) => b.renew_anchor(rect),
                    None => video::anchor(&addr, rect),
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
            },
            |r| {
                if let Err(e) = r {
                    tracing::warn!("Panorama : cadre : {e}");
                }
                Msg::Noop
            },
        )
    }

    /// Un relevé global par seconde, sans filtrage sur l'écran actif.
    pub(super) fn panorama_capture_scan(&mut self) -> Task<Msg> {
        if self.panorama.capture_busy
            || !self.cfg.plugin_on("panorama")
            || self.panorama_surfs().is_empty()
        {
            return Task::none();
        }
        self.panorama.capture_busy = true;
        Task::perform(
            async {
                let media = tokio::time::timeout(
                    Duration::from_millis(900),
                    crate::widgets::media::statuses(),
                )
                .await
                .unwrap_or_default();
                tokio::task::spawn_blocking(|| hyprpanel_core::ipc::query_json("clients").ok())
                    .await
                    .ok()
                    .flatten()
                    .map(|v| (v, media))
            },
            |r| wrap(PMsg::CaptureScan(r)),
        )
    }

    /// Fenêtre vidéo fermée hors du Panorama (SUPER+Q…) : on l'oublie.
    pub(super) fn panorama_video_gone(&mut self) {
        let gone = self
            .panorama
            .video
            .as_ref()
            .and_then(|v| v.address.as_ref())
            .is_some_and(|a| !self.clients.iter().any(|c| &c.address == a));
        if gone {
            self.panorama.capture_host = None;
            self.panorama.video = None;
        }
    }

    // -- Vue -------------------------------------------------------------------

    pub(super) fn view_panorama_surface(&self, id: Id) -> Element<'_, Msg> {
        let inner = self.view_panorama_surface_inner(id);
        // Vue par défaut sans bloc de raccourcis (composition du profil
        // Réglages) : petite barre des profils, pour ne jamais rester coincé.
        let inner = match self.panorama_surf(id).filter(|s| {
            self.panorama.page.get(&s.key).copied().unwrap_or(0) < self.panorama_offset()
                && !self
                    .panorama_geo(s)
                    .panes
                    .iter()
                    .any(|p| p.widget == "shortcuts")
        }) {
            Some(_) => stack![
                inner,
                container(self.panorama_profile_bar())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(10)
            ]
            .into(),
            None => inner,
        };
        // Quelque chose joue hors de vue (autre page, profil sans lecteur) :
        // pastille « en cours » pour le reprendre ou l'arrêter.
        let inner = match self
            .panorama_surf(id)
            .and_then(|s| self.panorama_now_playing(&s))
        {
            Some(pill) => stack![inner, pill].into(),
            None => inner,
        };
        if let Some(s) = self.panorama_surf(id)
            && self.panorama.tasks.as_ref().is_some_and(|t| t.key == s.key)
        {
            return stack![inner, self.panorama_tasks_view(&s)].into();
        }
        match self.panorama_surf(id) {
            Some(s) if self.panorama.notifs.as_deref() == Some(s.key.as_str()) => {
                stack![inner, self.panorama_notifs(&s)].into()
            }
            Some(s) if self.panorama.picker.as_deref() == Some(s.key.as_str()) => {
                stack![inner, self.panorama_picker(&s)].into()
            }
            Some(s) => {
                let mut layers = vec![inner];
                layers.extend(self.panorama_dots_overlay(&s));
                layers.extend(self.panorama_notifs_hint());
                stack(layers).into()
            }
            None => inner,
        }
    }

    /// Pastille « en cours » : titre, lecture/pause, arrêt — quand la vidéo
    /// ou la radio ne se voit pas sur l'écran (pas de voie sans retour).
    fn panorama_now_playing(&self, s: &Surf) -> Option<Element<'_, Msg>> {
        let name = self
            .panorama
            .radio
            .as_ref()
            .map(|r| r.0.clone())
            .or_else(|| self.panorama.video.as_ref().map(|v| v.name.clone()))?;
        if self.panorama_geo(s).video_pane().is_some() || self.panorama.tasks.is_some() {
            return None;
        }
        let (bg, fg, accent) = self.panorama_palette();
        let paused = if self.panorama.radio.is_some() {
            self.panorama.radio_paused
        } else {
            !self.panorama.media.as_ref().is_some_and(|m| m.playing)
        };
        let btn = |label: &'static str, msg: Msg| {
            self.panorama_feedback(
                button(text(label).size(16).color(fg))
                    .padding([4, 12])
                    .on_press(msg)
                    .style(move |_, st| button::Style {
                        background: Some(
                            fx::alpha(
                                accent,
                                if matches!(st, button::Status::Pressed) {
                                    0.6
                                } else {
                                    0.25
                                },
                            )
                            .into(),
                        ),
                        border: Border::default().rounded(12),
                        text_color: fg,
                        ..Default::default()
                    }),
            )
        };
        let pill = container(
            row![
                text(format!("♪ {name}")).size(14).color(fg),
                btn(if paused { "▶" } else { "⏸" }, wrap(PMsg::NowPlayingToggle)),
                btn("✕", wrap(PMsg::VideoClose)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding([6, 12])
        .style(move |_| container::Style {
            background: Some(fx::alpha(bg, 0.85).into()),
            border: Border {
                radius: 18.0.into(),
                width: 1.0,
                color: fx::alpha(accent, 0.6),
            },
            ..Default::default()
        });
        Some(
            container(pill)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Start)
                .align_y(Alignment::End)
                .padding(14)
                .into(),
        )
    }

    /// Repère translucide au bord droit : les notifications s'ouvrent en
    /// glissant depuis ce bord (leur nombre, s'il y en a).
    fn panorama_notifs_hint(&self) -> Option<Element<'_, Msg>> {
        if self.panorama_asleep() {
            return None;
        }
        let (bg, fg, accent) = self.panorama_palette();
        let active = self
            .panorama
            .last_press
            .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(4));
        let n = self.toasts.history.len();
        let a = match (active, n > 0) {
            (true, _) => 0.85,
            (false, true) => 0.5,
            (false, false) => 0.28,
        };
        static BELL: std::sync::LazyLock<iced::widget::svg::Handle> = std::sync::LazyLock::new(
            || {
                iced::widget::svg::Handle::from_memory(
                    br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="#fff" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9"/><path d="M10.3 21a1.94 1.94 0 0 0 3.4 0"/></svg>"##
                        .to_vec(),
                )
            },
        );
        let tint = fx::alpha(fg, a);
        let mut col = column![
            iced::widget::svg(BELL.clone())
                .width(18)
                .height(18)
                .style(move |_, _| iced::widget::svg::Style { color: Some(tint) }),
        ]
        .spacing(4)
        .align_x(Alignment::Center);
        if n > 0 {
            col = col.push(text(n.min(99).to_string()).size(11).color(tint));
        }
        // Poignée : une languette collée au bord, le geste part de là.
        let tab = container(col)
            .padding([8, 6])
            .style(move |_| container::Style {
                background: Some(fx::alpha(bg, a * 0.5).into()),
                border: Border {
                    radius: iced::border::Radius {
                        top_left: 10.0,
                        bottom_left: 10.0,
                        top_right: 0.0,
                        bottom_right: 0.0,
                    },
                    width: 1.0,
                    color: fx::alpha(if n > 0 { accent } else { fg }, a * 0.5),
                },
                ..Default::default()
            });
        Some(
            container(tab)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::End)
                .align_y(Alignment::Center)
                .into(),
        )
    }

    /// Points du carrousel en bas au centre : la page affichée et leur
    /// nombre ; translucides sans toucher récent.
    fn panorama_dots_overlay(&self, s: &Surf) -> Option<Element<'_, Msg>> {
        let n = self.panorama_carousel();
        if n < 2 || self.panorama_asleep() {
            return None;
        }
        // Profil Raccourcis, vue composée : la bande des modules défile de
        // côté, les points des pages n'y servent pas.
        if self.panorama.page.get(&s.key).copied().unwrap_or(0) == 0
            && self.panorama_layout_at(&s.key) == Layout::Dashboard
            && self.cfg.general.panorama.design.shortcut_profile == "system"
        {
            return None;
        }
        let (_, fg, accent) = self.panorama_palette();
        let i = self.panorama.page.get(&s.key).copied().unwrap_or(0);
        let active = self
            .panorama
            .last_press
            .is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(4));
        let a = if active { 1.0 } else { 0.35 };
        let dots = self.panorama_dots(
            n,
            i,
            &s.key,
            Color { a: fg.a * a, ..fg },
            Color {
                a: accent.a * a,
                ..accent
            },
        );
        // Sous la colonne des plateformes s'il y en a une (le lecteur, bord
        // à bord, n'en est pas recouvert) ; sinon au centre.
        let geo = self.panorama_geo(s);
        let (left, width) = match geo.panes.iter().find(|p| p.widget == "playlist") {
            Some(p) if s.w >= s.h => (p.rect.x, p.rect.w),
            _ => (0.0, s.w),
        };
        Some(
            container(row![
                Space::new().width(left),
                container(dots).width(width).center_x(width)
            ])
            .width(Length::Fill)
            .height(Length::Fill)
            .align_y(Alignment::End)
            .padding(iced::Padding {
                bottom: 2.0,
                ..iced::Padding::ZERO
            })
            .into(),
        )
    }

    /// Notifications (geste depuis le bord droit) : l'historique dans un
    /// volet à droite ; toucher à côté ou le repousser vers la droite le ferme.
    fn panorama_notifs(&self, s: &Surf) -> Element<'_, Msg> {
        let (bg, fg, accent) = self.panorama_palette();
        let font = self.panorama_font();
        let size = (s.h / 32.0).clamp(12.0, 18.0);
        let dim = fx::alpha(fg, 0.65);
        let pill = move |_: &iced::Theme, st: button::Status| button::Style {
            background: Some(
                fx::alpha(
                    accent,
                    if matches!(st, button::Status::Pressed) {
                        0.6
                    } else {
                        0.18
                    },
                )
                .into(),
            ),
            border: Border::default().rounded(10),
            text_color: fg,
            ..Default::default()
        };
        let history = &self.toasts.history;
        let mut head = row![
            text(crate::i18n::tr("Notifications"))
                .size(size * 1.2)
                .font(font)
                .color(fg)
                .width(Length::Fill)
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        if !history.is_empty() {
            head = head.push(
                self.panorama_feedback(
                    button(text(crate::i18n::tr("Tout effacer")).size(size).color(fg))
                        .padding([5, 12])
                        .on_press(wrap(PMsg::NotifsClear))
                        .style(pill),
                ),
            );
        }
        head = head.push(
            self.panorama_feedback(
                button(text("✕").size(size).color(fg))
                    .padding([5, 12])
                    .on_press(wrap(PMsg::Notifs(None)))
                    .style(pill),
            ),
        );
        let mut list = column![].spacing(size * 0.7);
        if history.is_empty() {
            list = list.push(
                text(crate::i18n::tr("Aucune notification"))
                    .size(size)
                    .font(font)
                    .color(dim),
            );
        }
        for n in history.iter().take(30) {
            let mut card = column![
                text(n.app.clone()).size(size * 0.75).font(font).color(dim),
                text(n.summary.clone()).size(size).font(font).color(fg),
            ]
            .spacing(3);
            if !n.body.is_empty() {
                card = card.push(text(n.body.clone()).size(size * 0.85).font(font).color(fg));
            }
            list = list.push(
                container(card)
                    .width(Length::Fill)
                    .padding([size * 0.5, size * 0.7])
                    .style(move |_| container::Style {
                        background: Some(fx::alpha(fg, 0.06).into()),
                        border: Border::default().rounded(size * 0.6),
                        ..Default::default()
                    }),
            );
        }
        let width = (s.w * 0.34).clamp(280.0, 560.0).min(s.w);
        let panel = container(
            column![
                head,
                iced::widget::scrollable(list)
                    .height(Length::Fill)
                    .width(Length::Fill)
            ]
            .spacing(size * 0.8),
        )
        .width(width)
        .height(Length::Fill)
        .padding(size)
        .style(move |_| container::Style {
            background: Some(fx::alpha(bg, 0.94).into()),
            border: Border {
                width: 1.0,
                color: fx::alpha(accent, 0.4),
                radius: 0.0.into(),
            },
            ..Default::default()
        });
        // Le volet absorbe ses appuis ; à côté, un appui ferme.
        let panel = iced::widget::mouse_area(panel).on_press(wrap(PMsg::Pressed));
        // Repoussé vers la droite (ou vers le haut/bas), il se ferme.
        let panel =
            swipe(panel, Axis::Horizontal, |_| wrap(PMsg::Notifs(None))).motion(Duration::ZERO);
        let veil = iced::widget::mouse_area(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(fx::alpha(bg, 0.35).into()),
                    ..Default::default()
                }),
        )
        .on_press(wrap(PMsg::Notifs(None)));
        row![veil, container(panel).width(width).height(Length::Fill)]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    /// Choix de la disposition (geste de bas en haut) : une tuile par
    /// disposition, avec son schéma ; toucher à côté ou glisser vers le bas
    /// le ferme.
    fn panorama_picker(&self, s: &Surf) -> Element<'_, Msg> {
        let (bg, fg, accent) = self.panorama_palette();
        let c = &self.cfg.general.panorama;
        let n = Layout::ALL.len() as f32;
        let side = (s.h * 0.62).min((s.w * 0.9) / n - 16.0).max(60.0);
        let plan_h = side * 0.5;
        let plan_w = if s.w >= s.h {
            (plan_h * s.w / s.h).min(side * 0.86)
        } else {
            plan_h * 0.6
        };
        let mut tiles = row![].spacing(16).align_y(Alignment::Center);
        for l in Layout::ALL {
            let on = l == c.layout;
            // Schéma : les zones de la disposition, à l'échelle de l'écran.
            let rects: Vec<[f32; 4]> = match l {
                Layout::Single => vec![[0.0, 0.0, plan_w, plan_h]],
                Layout::Dashboard => c
                    .design
                    .active_tiles(plan_w, plan_h)
                    .iter()
                    .map(|t| [t.x * plan_w, t.y * plan_h, t.w * plan_w, t.h * plan_h])
                    .collect(),
                l => c
                    .split_of(l)
                    .map(|sp| sp.rects(plan_w, plan_h, 3.0))
                    .unwrap_or_default(),
            };
            let mut plan = iced::widget::Stack::new().width(plan_w).height(plan_h);
            for [x, y, w, h] in rects {
                plan = plan.push(
                    iced::widget::pin(
                        container(Space::new())
                            .width(w.max(2.0))
                            .height(h.max(2.0))
                            .style(move |_| container::Style {
                                background: Some(Color { a: 0.35, ..accent }.into()),
                                border: Border::default().rounded(3.0),
                                ..Default::default()
                            }),
                    )
                    .x(x)
                    .y(y),
                );
            }
            tiles = tiles.push(
                button(
                    column![
                        plan,
                        text(l.short())
                            .size(side * 0.16)
                            .font(self.panorama_font())
                            .color(fg),
                    ]
                    .spacing(side * 0.08)
                    .align_x(Alignment::Center),
                )
                .width(side)
                .height(side)
                .padding(side * 0.1)
                .on_press(wrap(PMsg::Layout(l)))
                .style(move |_, st| button::Style {
                    background: Some(
                        match st {
                            button::Status::Pressed => Color { a: 0.5, ..accent },
                            _ if on => Color { a: 0.22, ..accent },
                            button::Status::Hovered => Color { a: 0.16, ..fg },
                            _ => Color { a: 0.08, ..fg },
                        }
                        .into(),
                    ),
                    border: Border {
                        color: Color {
                            a: if on { 0.9 } else { 0.2 },
                            ..accent
                        },
                        width: if on { 2.0 } else { 1.0 },
                        radius: (side * 0.14).into(),
                    },
                    ..Default::default()
                }),
            );
        }
        let axis = if s.w >= s.h {
            Axis::Horizontal
        } else {
            Axis::Vertical
        };
        let panel = iced::widget::mouse_area(
            container(tiles)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(Color { a: 0.82, ..bg }.into()),
                    ..Default::default()
                }),
        )
        .on_press(wrap(PMsg::PickerClose));
        swipe(panel, axis, |_| wrap(PMsg::PickerClose))
            .on_cross(|d| {
                if d < 0 {
                    wrap(PMsg::PickerClose)
                } else {
                    wrap(PMsg::Pressed)
                }
            })
            .into()
    }

    fn view_panorama_surface_inner(&self, id: Id) -> Element<'_, Msg> {
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
        let lay = if self.panorama.capture_host == Some(s.id) {
            Layout::Single
        } else {
            self.panorama_layout_at(&s.key)
        };
        let axis = if s.w >= s.h {
            Axis::Horizontal
        } else {
            Axis::Vertical
        };
        // La vue composée est l'entrée 0 du carrousel : on glisse vers les pages.
        // Le bloc des raccourcis glisse entre ses touches et les fenêtres :
        // un geste qui y part ne change pas de page.
        let yields: Vec<iced::Rectangle> = geo
            .panes
            .iter()
            // La bande des modules défile de côté : elle aussi garde le geste.
            .filter(|p| matches!(p.widget.as_str(), "shortcuts" | "modules"))
            .map(|p| {
                iced::Rectangle::new(
                    iced::Point::new(p.rect.x, p.rect.y),
                    iced::Size::new(p.rect.w, p.rect.h),
                )
            })
            .collect();
        let paged = |content| self.panorama_paged(&s.key, axis, content, yields.clone());
        if lay == Layout::Dashboard {
            return fx::stage(paged(self.panorama_dashboard(&s, &geo, video_at)), scene);
        }
        // Page composée de plusieurs zones : rendue comme la composition.
        if lay == Layout::Single && geo.panes.len() > 1 {
            return fx::stage(paged(self.panorama_dashboard(&s, &geo, video_at)), scene);
        }
        if lay == Layout::Single {
            let p = &geo.panes[0];
            let key = s.key.clone();
            let page = self.panorama_pane_view(&s, &geo, p, true, video_at);
            let cross = key.clone();
            let paged = swipe(page, axis, move |d| wrap(PMsg::Swipe(key.clone(), d)))
                .on_edge(EDGES, edge_msg(cross))
                .yield_to(yields.clone())
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
        yields: Vec<iced::Rectangle>,
    ) -> Element<'a, Msg> {
        let key = key.to_string();
        let cross = key.clone();
        swipe(content, axis, move |d| wrap(PMsg::Swipe(key.clone(), d)))
            .on_edge(EDGES, edge_msg(cross))
            .yield_to(yields)
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
            "shortcuts" => {
                // Média logé dans la colonne : sa place est réservée.
                let inside = |q: &&Pane| {
                    q.bare
                        && matches!(q.widget.as_str(), "video" | "media")
                        && q.rect.x >= p.rect.x
                        && q.rect.x + q.rect.w <= p.rect.x + p.rect.w + 0.5
                        && q.rect.y >= p.rect.y
                        && q.rect.y + q.rect.h <= p.rect.y + p.rect.h + 0.5
                };
                let media_h = geo.panes.iter().find(inside).map(|q| q.rect.h);
                self.panorama_shortcuts(iced::Size::new(content.w, content.h), media_h)
            }
            // Tuile d'une mesure : graphique, jauge ou carte.
            w if crate::panorama::design::metric_of(w).is_some() => {
                let (m, style) = crate::panorama::design::metric_of(w).unwrap_or_default();
                let lines: Vec<crate::widgets::SysLine> =
                    self.panorama.sys_ids.get(m).cloned().into_iter().collect();
                // Un appui ouvre le gestionnaire des tâches, trié sur la mesure.
                button(super::instruments::metrics(
                    style,
                    &lines,
                    &self.panorama.history,
                    self.cfg.general.panorama.design.history_seconds as usize,
                    (fg, accent),
                    self.panorama_font(),
                    self.cfg.general.panorama.design.text_scale,
                    self.panorama_graph_phase(),
                ))
                .padding(0)
                .width(Length::Fill)
                .height(Length::Fill)
                .on_press(wrap(PMsg::TasksOpen(s.key.clone(), m.to_string())))
                .style(|_, _| button::Style::default())
                .into()
            }
            "system" if self.cfg.general.panorama.design.system_style != "cards" => {
                super::instruments::metrics(
                    &self.cfg.general.panorama.design.system_style,
                    &self.panorama.sys,
                    &self.panorama.history,
                    self.cfg.general.panorama.design.history_seconds as usize,
                    (fg, accent),
                    self.panorama_font(),
                    self.cfg.general.panorama.design.text_scale,
                    self.panorama_graph_phase(),
                )
            }
            "system" => self.panorama_system(
                &z,
                fg,
                iced::Size::new(content.w, content.h),
                p.index.is_none(),
            ),
            "clock" => self.panorama_clock(&z, fg),
            "playlist" => self.panorama_playlist(iced::Size::new(content.w, content.h)),
            "hue" => self.panorama_hue(iced::Size::new(content.w, content.h)),
            "modules" => self.panorama_modules(iced::Size::new(content.w, content.h)),
            "voice" => self.panorama_voice_module(iced::Size::new(content.w, content.h)),
            "notes" => self.panorama_notes(iced::Size::new(content.w, content.h)),
            "updates" => self.panorama_updates(iced::Size::new(content.w, content.h)),
            // Webradio en cours : barre « flux vocal ».
            "video" if self.panorama.radio.is_some() => {
                let (name, _) = self.panorama.radio.clone().unwrap_or_default();
                super::flux::view(
                    &self.cfg.general.panorama.flux_ambiance,
                    self.panorama.bg_levels,
                    &self.panorama.level_hist,
                    name,
                    if self.panorama.radio_paused {
                        crate::i18n::tr("En pause").into()
                    } else if self.panorama.radio_connecting {
                        crate::i18n::tr("Connexion…").into()
                    } else {
                        crate::i18n::tr("En direct").into()
                    },
                    self.panorama_font(),
                    self.panorama.flux_t0.elapsed().as_secs_f32(),
                    self.panorama.radio_paused,
                    // Un cran toutes les deux images (15 par seconde).
                    self.panorama
                        .levels_at
                        .map_or(0.0, |t| t.elapsed().as_secs_f32() * 15.0),
                )
            }
            "video" => {
                // La liste des plateformes est déjà à l'écran : le lecteur
                // explique comment s'en servir au lieu de la répéter.
                let listed = geo.panes.iter().any(|q| q.widget == "playlist");
                let state = match &self.panorama.video {
                    _ if video_at != Some(p.rect) => VideoState::Elsewhere,
                    Some(v) if v.address.is_some() => VideoState::Hole,
                    Some(v) if v.error.is_none() => VideoState::Opening(&v.name),
                    _ if listed => VideoState::Guide,
                    _ => VideoState::Tiles,
                };
                self.panorama_video(&z, content, state, fg)
            }
            // Musique ou vidéo d'un onglet, logée sous le sélecteur du profil
            // Raccourcis : les commandes, et comment en voir l'image ici.
            "media" if p.bare && self.cfg.general.panorama.design.shortcut_profile == "system" => {
                let hint = 34.0;
                column![
                    self.panorama_compact_media(iced::Size::new(
                        content.w,
                        (content.h - hint).max(40.0)
                    )),
                    container(
                        text(crate::i18n::tr(
                            "Pour voir l'image ici : Picture-in-Picture ou plein écran dans le navigateur.",
                        ))
                        .size(12)
                        .color(Color { a: 0.65, ..fg })
                    )
                    .height(hint)
                    .padding([0, 8])
                    .center_y(hint)
                ]
                .into()
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
                                crate::ui::design::on_accent(accent, fg, self.panorama_palette().0)
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
        // Les points du carrousel sont en bas au centre (`panorama_dots_overlay`).
        let _ = accent;
        let r = row![seg].spacing(8).align_y(Alignment::Center);
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
            VideoState::Guide => {
                let error = self.panorama.video.as_ref().and_then(|v| v.error.clone());
                let size = (z.unit * 0.05).clamp(13.0, 22.0);
                let step = |n: &'static str, t: &'static str| -> Element<'_, Msg> {
                    row![
                        container(text(n).size(size * 0.9).color(fg))
                            .center_x(size * 1.7)
                            .center_y(size * 1.7)
                            .style(move |_| container::Style {
                                background: Some(fx::alpha(accent, 0.3).into()),
                                border: Border::default().rounded(size),
                                ..Default::default()
                            }),
                        text(crate::i18n::tr(t))
                            .size(size)
                            .font(self.panorama_font())
                            .color(dim)
                            .width(Length::Fill),
                    ]
                    .spacing(size * 0.7)
                    .align_y(Alignment::Center)
                    .into()
                };
                let mut col = column![
                    text(crate::i18n::tr("Lecteur vidéo"))
                        .size(size * 1.4)
                        .font(iced::Font {
                            weight: iced::font::Weight::Semibold,
                            ..self.panorama_font()
                        })
                        .color(fg),
                    step(
                        "1",
                        "Touchez une plateforme dans la liste Plateformes : elle s'ouvre ici.",
                    ),
                    step(
                        "2",
                        "Ou, depuis un navigateur, lancez une vidéo en Picture-in-Picture ou en \
                         plein écran : elle vient se placer ici.",
                    ),
                ]
                .spacing(size * 0.9)
                .max_width((area.w * 0.8).max(200.0));
                if let Some(e) = error {
                    col = col.push(
                        text(e)
                            .size(size * 0.85)
                            .color(Color::from_rgb8(0xff, 0xb8, 0x6c))
                            .font(self.panorama_font()),
                    );
                }
                container(col)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .padding(z.pad)
                    .into()
            }
            VideoState::Elsewhere => centered(
                text(crate::i18n::tr("La vidéo est dans l'autre zone."))
                    .size(z.unit * 0.06)
                    .color(dim)
                    .into(),
            ),
            VideoState::Tiles => {
                let mut col = column![].spacing(z.pad / 2.0).align_x(Alignment::Center);
                if let Some(e) = self.panorama.video.as_ref().and_then(|v| v.error.as_ref()) {
                    col = col.push(
                        text(e.clone())
                            .size(z.unit * 0.045)
                            .color(Color::from_rgb8(0xff, 0xb8, 0x6c))
                            .font(self.panorama_font()),
                    );
                }
                // Tuiles carrées, les plus grandes qui tiennent dans la zone ;
                // au-delà d'un minimum tactile, la zone défile.
                let gap = z.pad / 2.0;
                let (aw, ah) = ((area.w - z.pad).max(1.0), (area.h - z.pad).max(1.0));
                let shown = self.panorama_sources();
                let n = shown.len().max(1);
                let (mut cols, mut side) = (1, 0.0_f32);
                for k in 1..=n {
                    let rows = n.div_ceil(k) as f32;
                    let sd = ((aw - gap * (k as f32 - 1.0)) / k as f32)
                        .min((ah - gap * (rows - 1.0)) / rows);
                    if sd > side {
                        (cols, side) = (k, sd);
                    }
                }
                let side = side.clamp(z.target * 1.5, z.target * 3.4);
                let cols = cols
                    .max(1)
                    .min(((aw + gap) / (side + gap)).floor().max(1.0) as usize);
                let mut line = row![].spacing(gap);
                for (i, src) in shown.iter().enumerate() {
                    let icon_side = side * 0.48;
                    let icon: Element<'_, Msg> = (!src.icon.is_empty())
                        .then(|| self.icon_named(&src.icon, icon_side, fg))
                        .flatten()
                        .unwrap_or_else(|| {
                            // Initiale, en attendant une icône (SmartUI).
                            let letter = src
                                .name
                                .chars()
                                .find(|ch| ch.is_alphanumeric())
                                .map(|ch| ch.to_uppercase().to_string())
                                .unwrap_or_else(|| "•".into());
                            container(
                                text(letter)
                                    .size(icon_side * 0.55)
                                    .font(iced::Font {
                                        weight: iced::font::Weight::Bold,
                                        ..self.panorama_font()
                                    })
                                    .color(fg),
                            )
                            .width(icon_side)
                            .height(icon_side)
                            .center_x(icon_side)
                            .center_y(icon_side)
                            .style(move |_| container::Style {
                                background: Some(Color { a: 0.3, ..accent }.into()),
                                border: Border::default().rounded(icon_side * 0.24),
                                ..Default::default()
                            })
                            .into()
                        });
                    line = line.push(
                        self.panorama_feedback(
                            button(
                                // Icône et nom au centre de la tuile.
                                container(
                                    column![
                                        icon,
                                        text(src.name.clone())
                                            .size((side * 0.11).max(11.0))
                                            .font(iced::Font {
                                                weight: iced::font::Weight::Semibold,
                                                ..self.panorama_font()
                                            })
                                            .color(fg)
                                            .wrapping(text::Wrapping::None),
                                    ]
                                    .spacing(side * 0.07)
                                    .align_x(Alignment::Center),
                                )
                                .center_x(Length::Fill)
                                .center_y(Length::Fill),
                            )
                            .width(side)
                            .height(side)
                            .padding(side * 0.1)
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
                                    radius: (side * 0.16).into(),
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
                        line = row![].spacing(gap);
                    }
                }
                if !shown.len().is_multiple_of(cols) {
                    col = col.push(line);
                }
                if shown.is_empty() {
                    col = col.push(
                        text(crate::i18n::tr(
                            "Aucune source : ajoutez-en dans Réglages → Panorama.",
                        ))
                        .size(z.unit * 0.05)
                        .color(dim),
                    );
                }
                let col = container(col)
                    .width(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Shrink);
                container(iced::widget::scrollable(col).height(Length::Fill))
                    .padding(z.pad / 2.0)
                    .width(area.w)
                    .height(area.h)
                    .into()
            }
        }
    }
}

#[cfg(test)]
mod capture_tests {
    use super::*;
    fn app() -> App {
        let mut app = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-capture-test/config.lua"),
            true,
        )
        .0;
        std::sync::Arc::make_mut(&mut app.cfg)
            .general
            .panorama
            .layout = Layout::Single;
        std::sync::Arc::make_mut(&mut app.cfg)
            .general
            .panorama
            .pages = vec![crate::panorama::design::Page::single("clock")];
        app.panorama.preview = Some(super::super::Preview {
            win: Id::unique(),
            virtual_size: (1600, 450),
            scale: 1.0,
        });
        app.panorama.preview_at = Some((100, 100));
        app
    }
    fn snapshot() -> PMsg {
        PMsg::CaptureScan(Some((
            serde_json::json!([{
                "address":"0x42", "class":"firefox", "title":"Picture-in-Picture",
                "workspace":{"name":"12"}, "at":[2560,100], "size":[600,338],
                "floating":true, "monitor":2
            }]),
            Vec::new(),
        )))
    }
    #[test]
    fn capture_lodges_under_profile_tabs_in_shortcuts_profile() {
        let mut app = app();
        let id = app.panorama.preview.unwrap().win;
        {
            let c = &mut std::sync::Arc::make_mut(&mut app.cfg).general.panorama;
            c.layout = Layout::Dashboard;
            c.design.set("shortcut_profile", "system").unwrap();
        }
        let _ = app.panorama_update(snapshot());
        assert!(app.panorama.video.is_some());
        assert!(
            app.panorama.capture_host.is_none(),
            "pas de plein écran : la colonne l'accueille"
        );
        let geo = app.panorama_geo(&app.panorama_surf(id).unwrap());
        let video = geo.video_pane().expect("zone vidéo");
        let col = geo
            .panes
            .iter()
            .find(|p| p.widget == "shortcuts")
            .expect("colonne des raccourcis");
        assert!(video.bare);
        assert!(video.rect.y > col.rect.y && video.rect.x >= col.rect.x);
        assert!(video.rect.x + video.rect.w <= col.rect.x + col.rect.w + 0.5);
        assert!((video.rect.w / video.rect.h - 16.0 / 9.0).abs() < 0.01);
        // Page Fenêtres du bloc : la vidéo reste en cours, cachée sous le
        // Panorama (pas rendue) ; retour aux touches, retour à la colonne.
        let _ = app.panorama_update(PMsg::ShortcutsPage(1));
        assert!(app.panorama.video.is_some());
        assert!(app.pip_target().is_none());
        let _ = app.panorama_update(PMsg::ShortcutsPage(0));
        assert!(app.pip_target().is_some());
        assert!(app.panorama.capture_host.is_none());
    }
    #[test]
    fn capture_temporarily_replaces_clock_then_restores_page() {
        let mut app = app();
        let id = app.panorama.preview.unwrap().win;
        let before = app.cfg.general.panorama.clone();
        let _ = app.panorama_update(snapshot());
        assert!(app.panorama.video.as_ref().unwrap().borrowed.is_some());
        assert_eq!(app.panorama.capture_host, Some(id));
        assert!(
            app.panorama_geo(&app.panorama_surf(id).unwrap())
                .video_pane()
                .is_some()
        );
        let _ = app.panorama_update(PMsg::VideoClose);
        assert!(app.panorama.video.is_none());
        assert!(app.panorama.capture_host.is_none());
        assert_eq!(app.cfg.general.panorama, before);
        assert!(
            app.panorama_geo(&app.panorama_surf(id).unwrap())
                .video_pane()
                .is_none()
        );
        let _ = app.panorama_update(snapshot());
        assert!(
            app.panorama.video.is_none(),
            "pas de reprise après restitution"
        );
    }
    #[test]
    fn fullscreen_exit_only_returns_window_after_transfer_confirmation() {
        let mut app = app();
        let mut c = serde_json::json!({
            "address":"0x77", "class":"mpv", "title":"Film",
            "workspace":{"name":"12"}, "at":[2560,100], "size":[800,450],
            "floating":false, "monitor":2, "fullscreen":2, "fullscreenClient":2
        });
        let scan =
            |c: &serde_json::Value| PMsg::CaptureScan(Some((serde_json::json!([c]), Vec::new())));
        let _ = app.panorama_update(scan(&c));
        assert_eq!(
            app.panorama.video.as_ref().unwrap().address.as_deref(),
            Some("0x77")
        );
        c["fullscreen"] = 0.into();
        c["fullscreenClient"] = 0.into();
        // Relevés successifs après une transition ou une sortie de plein écran.
        for _ in 0..30 {
            let _ = app.panorama_update(scan(&c));
            assert_eq!(
                app.panorama.video.as_ref().unwrap().address.as_deref(),
                Some("0x77")
            );
        }
        let _ = app.panorama_update(PMsg::CapturePlaced("0x77".into(), Ok(())));
        assert!(app.cfg.general.panorama.restore_on_fullscreen_exit);
        std::sync::Arc::make_mut(&mut app.cfg)
            .general
            .panorama
            .restore_on_fullscreen_exit = false;
        let _ = app.panorama_update(scan(&c));
        assert_eq!(
            app.panorama.video.as_ref().unwrap().address.as_deref(),
            Some("0x77"),
            "retour automatique désactivé : la fenêtre reste dans Panorama"
        );
        assert!(app.panorama.capture_host.is_some());
        std::sync::Arc::make_mut(&mut app.cfg)
            .general
            .panorama
            .restore_on_fullscreen_exit = true;
        let _ = app.panorama_update(scan(&c));
        assert!(
            app.panorama.video.is_none(),
            "sortie de plein écran après transfert confirmé"
        );
        assert!(app.panorama.capture_host.is_none());
    }

    #[test]
    fn failed_capture_and_lost_surface_release_borrowed_window() {
        let mut app = app();
        let _ = app.panorama_update(snapshot());
        let _ = app.panorama_update(PMsg::CapturePlaced("0x42".into(), Err("échec".into())));
        assert!(app.panorama.video.is_none());
        assert!(app.panorama.capture_host.is_none());
        let mut app = self::app();
        let _ = app.panorama_update(snapshot());
        app.panorama.preview = None;
        let _ = app.sync_pip();
        assert!(app.panorama.video.is_none());
    }
    #[test]
    fn stale_capture_failure_does_not_close_current_window() {
        let mut app = app();
        let _ = app.panorama_update(snapshot());
        let _ = app.panorama_update(PMsg::CapturePlaced("0x99".into(), Err("retard".into())));
        assert_eq!(
            app.panorama.video.as_ref().unwrap().address.as_deref(),
            Some("0x42")
        );
    }
}

/// Bords suivis sur les pages du Panorama.
const EDGES: &[Edge] = &[Edge::Bottom, Edge::Top, Edge::Right];

/// Bas : choix de la disposition ; haut : fermer ou masquer ; droite :
/// notifications.
fn edge_msg(key: String) -> impl Fn(Edge) -> Msg {
    move |e| match e {
        Edge::Bottom => wrap(PMsg::Cross(key.clone(), 1)),
        Edge::Top => wrap(PMsg::Cross(key.clone(), -1)),
        Edge::Right | Edge::Left => wrap(PMsg::Notifs(Some(key.clone()))),
    }
}
