//! Panorama (MLG-164) : surface plein écran tactile des écrans bandeau.
//!
//! - une surface layer-shell par écran au rôle Panorama (calque Top, jamais
//!   de focus clavier : toucher le Panorama ne prend pas le clavier à la
//!   fenêtre de l'écran principal) ;
//! - des pages qu'on fait glisser (`swipe`), au doigt ou à la souris ;
//! - escamotable : une poignée sur le bord du bas la fait revenir ;
//! - proposée d'elle-même à la détection d'un écran candidat
//!   (`crate::panorama::detect`), une fois par écran.

mod swipe;

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use iced::widget::{Space, button, column, container, image, row, stack, text};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Task};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::{App, Msg, color};
use crate::panorama::detect::{self, Offer, Screen};
use crate::panorama::layout::{self, Orientation, Zones};
use crate::widgets::media::Status;
use swipe::{Axis, swipe};

/// Poignée de l'écran escamoté.
const HANDLE_W: u32 = 180;
const HANDLE_H: u32 = 26;
/// Relevés (lecteur, système) tant qu'un Panorama est affiché.
pub const TICK: Duration = Duration::from_secs(1);

const ICONS: &[&str] = &[
    "media-playback-start-symbolic",
    "media-playback-pause-symbolic",
    "media-skip-forward-symbolic",
    "media-skip-backward-symbolic",
    "audio-x-generic-symbolic",
    "go-down-symbolic",
    "go-up-symbolic",
];

#[derive(Debug, Default)]
pub struct State {
    /// Surface → écran.
    pub surfaces: HashMap<Id, String>,
    /// Poignée (Panorama escamoté) → écran.
    pub handles: HashMap<Id, String>,
    /// Carte de proposition → offre.
    pub offers: HashMap<Id, Offer>,
    /// Écrans escamotés (cette session).
    pub hidden: HashSet<String>,
    /// Page affichée, par écran.
    pub page: HashMap<String, usize>,
    /// Écrans tels que Hyprland les décrit (EDID, taille physique).
    pub screens: Vec<Screen>,
    /// Périphériques tactiles.
    pub touch: Vec<String>,
    /// Écrans (identifiant EDID) déjà proposés cette session.
    pub offered: HashSet<String>,
    pub media: Option<Status>,
    pub sys: Vec<crate::widgets::SysLine>,
    /// Dernier toucher (la veille attend `sleep_minutes` sans toucher ni lecture).
    pub last_press: Option<Instant>,
    pub cover: Option<(String, image::Handle)>,
}

#[derive(Debug, Clone)]
pub enum PMsg {
    /// Écrans et tactiles relus.
    Detected(Vec<Screen>, Vec<String>),
    Swipe(String, i32),
    Goto(String, usize),
    Hide(String),
    Show(String),
    /// Basculer (ligne de commande : sans écran, le premier Panorama).
    Toggle(Option<String>),
    Accept(Id),
    Decline(Id),
    Later(Id),
    Tick,
    Media(Option<Status>),
    Sys(Vec<crate::widgets::SysLine>),
    /// Commande MPRIS (`PlayPause`, `Next`, `Previous`).
    Control(&'static str),
    Pressed,
    Touch(Result<(), String>),
}

fn wrap(m: PMsg) -> Msg {
    Msg::Panorama(m)
}

/// Écrans et tactiles décrits par Hyprland (bloquant).
fn detect_now() -> (Vec<Screen>, Vec<String>) {
    use hyprpanel_core::ipc::query_json;
    let screens = query_json("monitors")
        .map(|v| detect::parse_screens(&v))
        .unwrap_or_default();
    let touch = query_json("devices")
        .map(|v| detect::parse_touch(&v))
        .unwrap_or_default();
    (screens, touch)
}

pub fn detect_task() -> Task<Msg> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(detect_now)
                .await
                .unwrap_or_default()
        },
        |(s, t)| wrap(PMsg::Detected(s, t)),
    )
}

fn fmt_time(secs: i64) -> String {
    let s = secs.max(0);
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

impl App {
    /// Configuration du Panorama en vigueur (réglages en cours, sinon chargée).
    pub(super) fn panorama_cfg(&self) -> crate::panorama::Config {
        self.settings
            .panorama
            .clone()
            .unwrap_or_else(|| self.cfg.general.panorama.clone())
    }

    /// Écrans branchés au rôle Panorama (connecteurs).
    pub(super) fn panorama_monitors(&self) -> Vec<String> {
        if !self.cfg.plugin_on("panorama") {
            return Vec::new();
        }
        let c = &self.cfg.general.panorama;
        self.monitors
            .iter()
            .filter(|m| {
                c.screens.iter().any(|e| e == &m.name)
                    || self
                        .panorama
                        .screens
                        .iter()
                        .find(|s| s.name == m.name)
                        .is_some_and(|s| c.has(s))
            })
            .map(|m| m.name.clone())
            .collect()
    }

    pub(super) fn is_panorama(&self, monitor: &str) -> bool {
        self.panorama_monitors().iter().any(|m| m == monitor)
    }

    /// Crée ou ferme les surfaces et poignées selon la config et les écrans.
    pub(super) fn sync_panorama(&mut self) -> Task<Msg> {
        let want = self.panorama_monitors();
        let mut tasks: Vec<Task<Msg>> = Vec::new();
        let shown: Vec<String> = want
            .iter()
            .filter(|m| !self.panorama.hidden.contains(*m))
            .cloned()
            .collect();
        let hidden: Vec<String> = want
            .iter()
            .filter(|m| self.panorama.hidden.contains(*m))
            .cloned()
            .collect();
        for (id, m) in self.panorama.surfaces.clone() {
            if !shown.contains(&m) {
                self.panorama.surfaces.remove(&id);
                tasks.push(iced::window::close(id));
            }
        }
        for (id, m) in self.panorama.handles.clone() {
            if !hidden.contains(&m) {
                self.panorama.handles.remove(&id);
                tasks.push(iced::window::close(id));
            }
        }
        // Rôle donné autrement (Réglages, ligne de commande) ou écran parti :
        // la proposition n'a plus lieu d'être.
        for (id, o) in self.panorama.offers.clone() {
            if want.contains(&o.screen) || !self.monitors.iter().any(|m| m.name == o.screen) {
                self.panorama.offers.remove(&id);
                tasks.push(iced::window::close(id));
            }
        }
        for m in shown {
            if self.panorama.surfaces.values().any(|s| s == &m) {
                continue;
            }
            let id = Id::unique();
            self.panorama.surfaces.insert(id, m.clone());
            tracing::info!("{}", crate::tr!("Panorama sur {p0}", p0 = m.clone()));
            tasks.push(Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some((0, 0)),
                    layer: Layer::Top,
                    anchor: LsAnchor::Top | LsAnchor::Bottom | LsAnchor::Left | LsAnchor::Right,
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    output_option: OutputOption::OutputName(m),
                    namespace: Some(crate::panorama::NAMESPACE.into()),
                    ..Default::default()
                },
                id,
            }));
        }
        for m in hidden {
            if self.panorama.handles.values().any(|s| s == &m) {
                continue;
            }
            let id = Id::unique();
            self.panorama.handles.insert(id, m.clone());
            tasks.push(Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some((HANDLE_W, HANDLE_H)),
                    layer: Layer::Top,
                    anchor: LsAnchor::Bottom,
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    output_option: OutputOption::OutputName(m),
                    namespace: Some(crate::panorama::NAMESPACE.into()),
                    ..Default::default()
                },
                id,
            }));
        }
        if !self.panorama.surfaces.is_empty() {
            for n in ICONS {
                tasks.push(self.want_icon(n));
            }
        }
        Task::batch(tasks)
    }

    /// Propositions à afficher (écrans candidats nouveaux).
    fn panorama_offers(&mut self) -> Task<Msg> {
        if !self.cfg.plugin_on("panorama") {
            return Task::none();
        }
        let c = self.panorama_cfg();
        let list = detect::offers(
            &self.panorama.screens,
            &self.panorama.touch,
            &c.screens,
            &c.declined,
        );
        let mut tasks = Vec::new();
        for o in list {
            let key = if o.edid.is_empty() {
                o.screen.clone()
            } else {
                o.edid.clone()
            };
            if !self.panorama.offered.insert(key) {
                continue;
            }
            let Some(m) = self.monitors.iter().find(|m| m.name == o.screen) else {
                continue;
            };
            let w = (m.width.saturating_sub(32)).min(900);
            let h = (m.height.saturating_sub(32)).min(260);
            // Un tactile sûr : il est rattaché tout de suite, pour qu'on
            // puisse répondre au doigt.
            if let Some(_t) = &o.touch {
                let (path, out) = (self.config_path.clone(), o.screen.clone());
                tasks.push(Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::panorama::map_touch(&path, Some(&out))
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| wrap(PMsg::Touch(r)),
                ));
            }
            let id = Id::unique();
            tasks.push(Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some((w, h)),
                    layer: Layer::Overlay,
                    anchor: LsAnchor::empty(),
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    output_option: OutputOption::OutputName(o.screen.clone()),
                    namespace: Some(crate::panorama::NAMESPACE.into()),
                    ..Default::default()
                },
                id,
            }));
            self.panorama.offers.insert(id, o);
        }
        Task::batch(tasks)
    }

    /// Enregistre une nouvelle configuration du Panorama.
    fn panorama_save(&mut self, c: crate::panorama::Config) -> Task<Msg> {
        self.settings.panorama = Some(c);
        self.commit()
    }

    pub(super) fn panorama_update(&mut self, m: PMsg) -> Task<Msg> {
        match m {
            PMsg::Detected(screens, touch) => {
                self.panorama.screens = screens;
                self.panorama.touch = touch;
                let offers = self.panorama_offers();
                Task::batch([self.sync_bars(), self.sync_panorama(), offers])
            }
            PMsg::Swipe(mon, dir) => {
                let n = self.cfg.general.panorama.pages.len().max(1) as i64;
                let p = self.panorama.page.entry(mon).or_insert(0);
                *p = (*p as i64 + dir as i64).rem_euclid(n) as usize;
                Task::none()
            }
            PMsg::Goto(mon, i) => {
                self.panorama.page.insert(mon, i);
                Task::none()
            }
            PMsg::Hide(mon) => {
                self.panorama.hidden.insert(mon);
                self.sync_panorama()
            }
            PMsg::Show(mon) => {
                self.panorama.hidden.remove(&mon);
                self.panorama.last_press = Some(Instant::now());
                self.sync_panorama()
            }
            PMsg::Toggle(mon) => {
                let list = self.panorama_monitors();
                let Some(mon) = mon.or_else(|| list.first().cloned()) else {
                    return Task::none();
                };
                if self.panorama.hidden.contains(&mon) {
                    self.panorama_update(PMsg::Show(mon))
                } else {
                    self.panorama_update(PMsg::Hide(mon))
                }
            }
            PMsg::Accept(id) => {
                let Some(o) = self.panorama.offers.remove(&id) else {
                    return Task::none();
                };
                let mut c = self.panorama_cfg();
                let key = if o.edid.is_empty() {
                    o.screen.clone()
                } else {
                    o.edid.clone()
                };
                c.declined.retain(|d| d != &key);
                if !c.screens.contains(&key) {
                    c.screens.push(key);
                }
                // Tactile présent : il suit l'écran choisi (« Oui » touché ici).
                let touch = if self.panorama.touch.is_empty() {
                    Task::none()
                } else {
                    let (path, out) = (self.config_path.clone(), o.screen.clone());
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                crate::panorama::map_touch(&path, Some(&out))
                            })
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                        },
                        |r| wrap(PMsg::Touch(r)),
                    )
                };
                Task::batch([iced::window::close(id), self.panorama_save(c), touch])
            }
            PMsg::Decline(id) => {
                let Some(o) = self.panorama.offers.remove(&id) else {
                    return Task::none();
                };
                let mut c = self.panorama_cfg();
                let key = if o.edid.is_empty() { o.screen } else { o.edid };
                if !c.declined.contains(&key) {
                    c.declined.push(key);
                }
                Task::batch([iced::window::close(id), self.panorama_save(c)])
            }
            PMsg::Later(id) => {
                self.panorama.offers.remove(&id);
                iced::window::close(id)
            }
            PMsg::Tick => {
                if self.panorama.surfaces.is_empty() {
                    return Task::none();
                }
                let media =
                    Task::perform(crate::widgets::media::status(), |s| wrap(PMsg::Media(s)));
                let wants_sys = self
                    .panorama
                    .surfaces
                    .values()
                    .any(|m| self.panorama_page_id(m) == "system");
                if !wants_sys {
                    return media;
                }
                let sys = Task::perform(
                    async {
                        tokio::task::spawn_blocking(|| {
                            use crate::config::SysItem;
                            crate::widgets::sample_system(&[
                                SysItem::Cpu,
                                SysItem::Ram,
                                SysItem::Disk("/".into()),
                                SysItem::Load,
                                SysItem::Uptime,
                            ])
                        })
                        .await
                        .unwrap_or_default()
                    },
                    |l| wrap(PMsg::Sys(l)),
                );
                Task::batch([media, sys])
            }
            PMsg::Media(s) => {
                // Pochette locale (file://) : chargée une fois par adresse.
                let art = s
                    .as_ref()
                    .and_then(|s| s.art.strip_prefix("file://"))
                    .map(str::to_string);
                match art {
                    Some(path) if self.panorama.cover.as_ref().map(|c| &c.0) != Some(&path) => {
                        let h = image::Handle::from_path(&path);
                        self.panorama.cover = Some((path, h));
                    }
                    None => self.panorama.cover = None,
                    _ => {}
                }
                self.panorama.media = s;
                Task::none()
            }
            PMsg::Sys(l) => {
                self.panorama.sys = l;
                Task::none()
            }
            PMsg::Control(method) => {
                let Some(bus) = self.panorama.media.as_ref().map(|s| s.bus.clone()) else {
                    return Task::none();
                };
                if method == "PlayPause"
                    && let Some(s) = self.panorama.media.as_mut()
                {
                    s.playing = !s.playing;
                }
                Task::perform(crate::widgets::media::media_control(bus, method), |_| {
                    wrap(PMsg::Tick)
                })
            }
            PMsg::Pressed => {
                self.panorama.last_press = Some(Instant::now());
                Task::none()
            }
            PMsg::Touch(r) => {
                if let Err(e) = r {
                    tracing::warn!(
                        "{}",
                        crate::tr!("Panorama : tactile non rattaché : {e}", e = e)
                    );
                }
                Task::none()
            }
        }
    }

    /// `hyprdesk panorama <toggle|show|hide|page <n|id>|detect>`.
    pub(super) fn panorama_command(&mut self, a: &str) -> Task<Msg> {
        let mut it = a.split_whitespace();
        let first = self.panorama_monitors().into_iter().next();
        match (it.next().unwrap_or("toggle"), first) {
            ("detect", _) => {
                // Nouvelle passe : les écrans déjà proposés peuvent l'être encore.
                self.panorama.offered.clear();
                detect_task()
            }
            ("toggle", m) => self.panorama_update(PMsg::Toggle(m)),
            ("show", Some(m)) => self.panorama_update(PMsg::Show(m)),
            ("hide", Some(m)) => self.panorama_update(PMsg::Hide(m)),
            ("page", Some(m)) => {
                let want = it.next().unwrap_or("");
                let pages = &self.cfg.general.panorama.pages;
                let i = want
                    .parse::<usize>()
                    .ok()
                    .map(|n| n.saturating_sub(1))
                    .or_else(|| pages.iter().position(|p| p == want));
                match i {
                    Some(i) if i < pages.len() => {
                        self.panorama.hidden.remove(&m);
                        self.panorama.page.insert(m, i);
                        self.sync_panorama()
                    }
                    _ => Task::none(),
                }
            }
            _ => Task::none(),
        }
    }

    /// Surface fermée par le compositeur (écran débranché…).
    pub(super) fn panorama_closed(&mut self, id: Id) -> bool {
        self.panorama.surfaces.remove(&id).is_some()
            || self.panorama.handles.remove(&id).is_some()
            || self.panorama.offers.remove(&id).is_some()
    }

    pub(super) fn panorama_owns(&self, id: Id) -> bool {
        self.panorama.surfaces.contains_key(&id)
            || self.panorama.handles.contains_key(&id)
            || self.panorama.offers.contains_key(&id)
    }

    /// Page affichée sur un écran (la veille passe d'elle-même à l'horloge).
    fn panorama_page_id(&self, monitor: &str) -> &str {
        let pages = &self.cfg.general.panorama.pages;
        let i = self.panorama.page.get(monitor).copied().unwrap_or(0);
        pages
            .get(i.min(pages.len().saturating_sub(1)))
            .map(String::as_str)
            .unwrap_or("media")
    }

    /// Veille : rien ne joue et personne n'a touché depuis `sleep_minutes`.
    fn panorama_asleep(&self) -> bool {
        let mins = self.cfg.general.panorama.sleep_minutes;
        if mins == 0 || self.panorama.media.as_ref().is_some_and(|m| m.playing) {
            return false;
        }
        self.panorama
            .last_press
            .is_none_or(|t| t.elapsed() >= Duration::from_secs(mins as u64 * 60))
            && self.cfg.general.panorama.pages.iter().any(|p| p == "clock")
    }

    pub(super) fn view_panorama(&self, id: Id) -> Element<'_, Msg> {
        if let Some(m) = self.panorama.handles.get(&id) {
            return self.view_panorama_handle(m);
        }
        if let Some(o) = self.panorama.offers.get(&id) {
            return self.view_panorama_offer(id, o);
        }
        let Some(mon) = self.panorama.surfaces.get(&id) else {
            return Space::new().into();
        };
        let Some(m) = self.monitors.iter().find(|x| &x.name == mon) else {
            return Space::new().into();
        };
        let ppm = self
            .panorama
            .screens
            .iter()
            .find(|s| &s.name == mon)
            .and_then(Screen::px_per_mm);
        // Taille réelle de la surface d'abord : après une rotation, la liste
        // des écrans peut avoir un temps de retard.
        let (w, h) = self
            .sizes
            .get(&id)
            .map(|s| (s.width, s.height))
            .filter(|(w, h)| *w > 0.0 && *h > 0.0)
            .unwrap_or((m.width as f32, m.height as f32));
        let z = layout::zones(w, h, ppm);
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let bg = color(th.bg, 1.0);
        let pages = &self.cfg.general.panorama.pages;
        let page_id = if self.panorama_asleep() {
            "clock"
        } else {
            self.panorama_page_id(mon)
        };
        let body: Element<'_, Msg> = match page_id {
            "system" => self.panorama_system(&z, fg),
            "clock" => self.panorama_clock(&z, fg),
            _ => self.panorama_media(&z, fg),
        };
        let axis = match z.orientation {
            Orientation::Landscape => Axis::Horizontal,
            Orientation::Portrait => Axis::Vertical,
        };
        let mon_s = mon.clone();
        let paged = swipe(body, axis, move |d| wrap(PMsg::Swipe(mon_s.clone(), d)))
            .on_press(wrap(PMsg::Pressed));
        // Points des pages (touchables) et bouton d'escamotage.
        let cur = self.panorama.page.get(mon).copied().unwrap_or(0);
        let accent = color(th.accent, 1.0);
        let tgt = z.target;
        let hide = button(
            self.icon_named("go-down-symbolic", tgt * 0.3, fg)
                .unwrap_or_else(|| text("▾").size(tgt * 0.4).color(fg).into()),
        )
        .width(tgt)
        .height(tgt)
        .on_press(wrap(PMsg::Hide(mon.clone())))
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
            border: Border::default().rounded(tgt / 2.0),
            ..Default::default()
        });
        let chrome = container(
            // En bas à droite : rien du lecteur n'y est posé.
            row![
                Space::new().width(Length::Fill),
                self.panorama_dots(pages.len(), cur, mon, fg, accent),
                hide
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(z.pad / 2.0)
        .align_y(iced::alignment::Vertical::Bottom);
        let backdrop = container(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(bg.into()),
                ..Default::default()
            });
        let mut layers: Vec<Element<'_, Msg>> = vec![backdrop.into()];
        if let Some((_, h)) = &self.panorama.cover
            && page_id == "media"
        {
            // Pochette en fond, très atténuée.
            layers.push(
                container(
                    image(h.clone())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .content_fit(iced::ContentFit::Cover)
                        .opacity(0.18_f32),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            );
        }
        layers.push(paged.into());
        layers.push(chrome.into());
        stack(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn panorama_dots<'a>(
        &self,
        n: usize,
        cur: usize,
        mon: &str,
        fg: Color,
        accent: Color,
    ) -> Element<'a, Msg> {
        let items: Vec<Element<'a, Msg>> = (0..n)
            .map(|i| {
                let on = i == cur;
                button(
                    container(Space::new().width(if on { 26 } else { 10 }).height(10)).style(
                        move |_| container::Style {
                            background: Some(
                                if on { accent } else { Color { a: 0.35, ..fg } }.into(),
                            ),
                            border: Border::default().rounded(999),
                            ..Default::default()
                        },
                    ),
                )
                .padding(10)
                .on_press(wrap(PMsg::Goto(mon.to_string(), i)))
                .style(|_, _| button::Style::default())
                .into()
            })
            .collect();
        row(items).spacing(2).into()
    }

    fn panorama_button<'a>(&self, icon: &str, size: f32, fg: Color, msg: Msg) -> Element<'a, Msg> {
        let glyph = self
            .icon_named(icon, size * 0.45, fg)
            .unwrap_or_else(|| text("•").size(size * 0.4).color(fg).into());
        button(
            container(glyph)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
        )
        .width(size)
        .height(size)
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
            border: Border::default().rounded(size / 2.0),
            ..Default::default()
        })
        .into()
    }

    fn panorama_media(&self, z: &Zones, fg: Color) -> Element<'_, Msg> {
        let th = &self.cfg.general.theme;
        let accent = color(th.accent, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let art_side = z.art.w;
        let art: Element<'_, Msg> = match &self.panorama.cover {
            Some((_, h)) => image(h.clone())
                .width(art_side)
                .height(art_side)
                .content_fit(iced::ContentFit::Cover)
                .border_radius(18.0)
                .into(),
            None => container(
                self.icon_named("audio-x-generic-symbolic", art_side * 0.35, dim)
                    .unwrap_or_else(|| Space::new().into()),
            )
            .width(art_side)
            .height(art_side)
            .center_x(art_side)
            .center_y(art_side)
            .style(move |_| container::Style {
                background: Some(Color { a: 0.08, ..fg }.into()),
                border: Border::default().rounded(18),
                ..Default::default()
            })
            .into(),
        };
        let tgt = z.target;
        let short = z.art.w + 2.0 * z.pad;
        let (title, sub, playing, pos, len) = match &self.panorama.media {
            Some(s) => (
                if s.title.is_empty() {
                    s.player.clone()
                } else {
                    s.title.clone()
                },
                [s.artist.as_str(), s.album.as_str()]
                    .iter()
                    .filter(|x| !x.is_empty())
                    .copied()
                    .collect::<Vec<_>>()
                    .join(" · "),
                s.playing,
                s.position,
                s.length,
            ),
            None => (
                crate::i18n::tr("Rien ne joue").to_string(),
                crate::i18n::tr("Lancez un lecteur (Spotify, Firefox, mpv…)").to_string(),
                false,
                None,
                None,
            ),
        };
        let progress: Element<'_, Msg> = match (pos, len) {
            (Some(p), Some(l)) if l > 0 => {
                let r = (p as f32 / l as f32).clamp(0.0, 1.0);
                let filled = (r * 1000.0) as u16;
                let bar = |portion: u16, c: Option<Color>| -> Element<'_, Msg> {
                    container(Space::new())
                        .width(Length::FillPortion(portion.max(1)))
                        .height(8)
                        .style(move |_| container::Style {
                            background: c.map(Into::into),
                            border: Border::default().rounded(4),
                            ..Default::default()
                        })
                        .into()
                };
                column![
                    row![
                        bar(filled, Some(accent)),
                        bar(1000 - filled, Some(Color { a: 0.15, ..fg }))
                    ],
                    row![
                        text(fmt_time(p)).size(short * 0.045).color(dim),
                        Space::new().width(Length::Fill),
                        text(fmt_time(l)).size(short * 0.045).color(dim)
                    ]
                ]
                .spacing(6)
                .into()
            }
            _ => Space::new().into(),
        };
        let controls = row![
            self.panorama_button(
                "media-skip-backward-symbolic",
                tgt,
                fg,
                wrap(PMsg::Control("Previous"))
            ),
            self.panorama_button(
                if playing {
                    "media-playback-pause-symbolic"
                } else {
                    "media-playback-start-symbolic"
                },
                tgt * 1.35,
                fg,
                wrap(PMsg::Control("PlayPause"))
            ),
            self.panorama_button(
                "media-skip-forward-symbolic",
                tgt,
                fg,
                wrap(PMsg::Control("Next"))
            ),
        ]
        .spacing(tgt * 0.4)
        .align_y(Alignment::Center);
        let info = column![
            text(title)
                .size(short * 0.1)
                .color(fg)
                .font(self.font)
                .wrapping(text::Wrapping::None),
            text(sub)
                .size(short * 0.055)
                .color(dim)
                .font(self.font)
                .wrapping(text::Wrapping::None),
            Space::new().height(Length::Fill),
            progress,
            Space::new().height(z.pad / 2.0),
            controls,
        ]
        .spacing(8)
        .width(z.main.w)
        .height(z.main.h)
        .clip(true);
        let side: Element<'_, Msg> = match (z.side, &self.panorama.media) {
            (Some(r), Some(s)) => column![
                text(crate::i18n::tr("Lecteur"))
                    .size(short * 0.04)
                    .color(dim)
                    .font(self.font),
                text(s.player.split('.').next().unwrap_or(&s.player).to_string())
                    .size(short * 0.06)
                    .color(fg)
                    .font(self.font),
            ]
            .spacing(6)
            .width(r.w)
            .height(r.h)
            .into(),
            (Some(r), None) => Space::new().width(r.w).height(r.h).into(),
            (None, _) => Space::new().width(0).into(),
        };
        let pad = z.pad;
        match z.orientation {
            Orientation::Landscape => row![art, info, side]
                .spacing(pad)
                .padding(pad)
                .align_y(Alignment::Start)
                .into(),
            Orientation::Portrait => column![art, info, side]
                .spacing(pad)
                .padding(pad)
                .align_x(Alignment::Center)
                .into(),
        }
    }

    fn panorama_system(&self, z: &Zones, fg: Color) -> Element<'_, Msg> {
        let th = &self.cfg.general.theme;
        let accent = color(th.accent, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let short = z.art.w + 2.0 * z.pad;
        let cards: Vec<Element<'_, Msg>> = self
            .panorama
            .sys
            .iter()
            .map(|l| {
                let gauge: Element<'_, Msg> = match l.ratio {
                    Some(r) => {
                        let f = (r.clamp(0.0, 1.0) * 1000.0) as u16;
                        row![
                            container(Space::new())
                                .width(Length::FillPortion(f.max(1)))
                                .height(10)
                                .style(move |_| container::Style {
                                    background: Some(accent.into()),
                                    border: Border::default().rounded(5),
                                    ..Default::default()
                                }),
                            container(Space::new())
                                .width(Length::FillPortion((1000 - f).max(1)))
                                .height(10)
                                .style(move |_| container::Style {
                                    background: Some(Color { a: 0.15, ..fg }.into()),
                                    border: Border::default().rounded(5),
                                    ..Default::default()
                                }),
                        ]
                        .into()
                    }
                    None => Space::new().into(),
                };
                container(
                    column![
                        text(l.label.clone())
                            .size(short * 0.05)
                            .color(dim)
                            .font(self.font),
                        text(l.value.clone())
                            .size(short * 0.09)
                            .color(fg)
                            .font(self.font),
                        Space::new().height(Length::Fill),
                        gauge,
                    ]
                    .spacing(6),
                )
                .padding(z.pad)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(Color { a: 0.06, ..fg }.into()),
                    border: Border::default().rounded(18),
                    ..Default::default()
                })
                .into()
            })
            .collect();
        let pad = z.pad;
        let body: Element<'_, Msg> = match z.orientation {
            Orientation::Landscape => row(cards).spacing(pad).into(),
            Orientation::Portrait => column(cards).spacing(pad).into(),
        };
        container(body)
            .padding(iced::Padding {
                top: pad,
                left: pad,
                right: pad,
                bottom: pad + z.target,
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn panorama_clock(&self, z: &Zones, fg: Color) -> Element<'_, Msg> {
        let dim = Color { a: 0.6, ..fg };
        let short = z.art.w + 2.0 * z.pad;
        let now = chrono::Local::now();
        let date = now
            .format_localized("%A %e %B", crate::i18n::chrono_locale())
            .to_string();
        let mut col = column![
            text(now.format("%H:%M").to_string())
                .size(short * 0.42)
                .color(fg)
                .font(self.font),
            text(date).size(short * 0.08).color(dim).font(self.font),
        ]
        .align_x(Alignment::Center)
        .spacing(4);
        if let Some(s) = self.panorama.media.as_ref().filter(|s| !s.title.is_empty()) {
            col = col.push(
                text(if s.artist.is_empty() {
                    s.title.clone()
                } else {
                    format!("{} — {}", s.title, s.artist)
                })
                .size(short * 0.05)
                .color(dim)
                .font(self.font),
            );
        }
        container(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn view_panorama_handle(&self, mon: &str) -> Element<'_, Msg> {
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let bg = color(th.bg, 0.92);
        button(
            container(
                row![
                    self.icon_named("go-up-symbolic", 14.0, fg)
                        .unwrap_or_else(|| text("▴").size(14).color(fg).into()),
                    text("Panorama").size(13).color(fg).font(self.font)
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .on_press(wrap(PMsg::Show(mon.to_string())))
        .style(move |_, _| button::Style {
            background: Some(bg.into()),
            border: Border {
                radius: iced::border::Radius {
                    top_left: 13.0,
                    top_right: 13.0,
                    bottom_right: 0.0,
                    bottom_left: 0.0,
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
    }

    fn view_panorama_offer(&self, id: Id, o: &Offer) -> Element<'_, Msg> {
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let accent = color(th.accent, 1.0);
        let bg = color(th.bg, 0.97);
        let dim = Color { a: 0.7, ..fg };
        let what = if o.touch.is_some() {
            crate::i18n::tr(
                "Écran tactile allongé détecté. En faire un Panorama : lecteur multimédia, système et horloge, pilotés au doigt ?",
            )
        } else if o.ambiguous {
            crate::i18n::tr(
                "Écran allongé détecté. En faire un Panorama ? Touchez « Oui » sur l'écran tactile : le tactile y sera rattaché.",
            )
        } else {
            crate::i18n::tr(
                "Écran allongé détecté. En faire un Panorama : lecteur multimédia, système et horloge, en plein écran ?",
            )
        };
        let btn = |label: &'static str, primary: bool, msg: Msg| -> Element<'_, Msg> {
            button(
                text(crate::i18n::tr(label))
                    .size(18)
                    .font(self.font)
                    .color(if primary { Color::BLACK } else { fg }),
            )
            .padding([14, 26])
            .on_press(msg)
            .style(move |_, st| button::Style {
                background: Some(
                    if primary {
                        accent
                    } else {
                        Color {
                            a: if matches!(st, button::Status::Hovered | button::Status::Pressed) {
                                0.18
                            } else {
                                0.08
                            },
                            ..fg
                        }
                    }
                    .into(),
                ),
                border: Border::default().rounded(999),
                ..Default::default()
            })
            .into()
        };
        container(
            column![
                text(crate::i18n::tr("Panorama"))
                    .size(26)
                    .font(self.font)
                    .color(fg),
                text(what).size(16).font(self.font).color(dim),
                Space::new().height(Length::Fill),
                row![
                    btn("Oui, en faire un Panorama", true, wrap(PMsg::Accept(id))),
                    btn("Plus tard", false, wrap(PMsg::Later(id))),
                    btn("Non merci", false, wrap(PMsg::Decline(id))),
                ]
                .spacing(12),
            ]
            .spacing(10),
        )
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(bg.into()),
            border: Border::default()
                .rounded(22)
                .width(1)
                .color(Color { a: 0.15, ..fg }),
            ..Default::default()
        })
        .into()
    }
}
