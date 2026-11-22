//! Côte à côte (profils Vidéo et Musique) : l'écran du Panorama partagé en
//! deux, le média en cours d'un côté, de l'autre une zone d'accueil où l'on
//! ajoute (« + ») un ou plusieurs modules, ou une fenêtre ouverte ramenée
//! par Hyprland (`crate::panorama::guest`). Le bandeau de la zone inverse
//! les deux côtés ou revient sans partage (la fenêtre est alors rendue).
//!
//! Le contenu de la zone (modules, classe de la fenêtre) est retenu
//! (`plugin_options.panorama` : `side.last`) et revient au prochain côte à
//! côte. Cinq préréglages (`side.preset1` à `side.preset5`) : un appui
//! rappelle, un appui long enregistre le contenu affiché (et remplace).

use std::time::{Duration, Instant};

use iced::widget::{Space, button, column, container, mouse_area, row, scrollable, svg, text};
use iced::{Alignment, Border, Color, Element, Length, Size, Task};

use super::panes::{Geo, Pane, Surf};
use super::swipe::{Axis, swipe};
use super::{App, Msg, PMsg, fx, wrap};
use crate::panorama::guest::Guest;
use crate::panorama::layout::Rect;

/// Modules au plus dans la zone d'accueil.
const MAX_MODULES: usize = 4;
/// Préréglages de la zone d'accueil.
pub const PRESETS: usize = 5;
/// Durée d'un appui long (enregistrement d'un préréglage).
const HOLD: Duration = Duration::from_millis(650);
/// Durée d'affichage d'un avis dans le bandeau.
const NOTICE: Duration = Duration::from_millis(4000);
/// Clé des options du Panorama (`side.<clé>`).
const OPTION: &str = "side";

/// Contenu de la zone d'accueil, tel qu'il est retenu : ses modules et la
/// classe de la fenêtre ramenée (une adresse ne survit pas à la fenêtre).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Content {
    pub modules: Vec<String>,
    pub window: Option<String>,
}

impl Content {
    /// `notes,hue|kitty` ; jamais vide (un préréglage vide reste enregistré).
    pub fn encode(&self) -> String {
        format!(
            "{}|{}",
            self.modules.join(","),
            self.window.as_deref().unwrap_or("")
        )
    }

    pub fn decode(v: &str) -> Option<Content> {
        if v.trim().is_empty() {
            return None;
        }
        let (m, w) = v.split_once('|').unwrap_or((v, ""));
        let mut modules: Vec<String> = Vec::new();
        for id in m.split(',').map(str::trim) {
            if crate::panorama::modules::CATALOGUE
                .iter()
                .any(|(c, _, _)| *c == id)
                && !modules.iter().any(|x| x == id)
                && modules.len() < MAX_MODULES
            {
                modules.push(id.to_string());
            }
        }
        let w = w.trim();
        Some(Content {
            modules,
            window: (!w.is_empty()).then(|| w.to_string()),
        })
    }
}

/// État du côte à côte (pour la session).
#[derive(Debug, Clone, Default)]
pub struct SideBySide {
    /// Le média passe à droite (en bas sur un écran portrait).
    pub swapped: bool,
    /// Modules de la zone d'accueil, dans l'ordre.
    pub modules: Vec<String>,
    /// Fenêtre ramenée dans la zone d'accueil.
    pub guest: Option<Guest>,
    /// Rectangle global où elle a été posée en dernier.
    pub placed: Option<(i32, i32, i32, i32)>,
    /// Choix « + » ouvert (modules, fenêtres).
    pub picking: bool,
    /// Fenêtre en cours de transfert.
    pub busy: bool,
    pub error: Option<String>,
    /// Préréglage tenu : (rang, appui) ; l'appui long l'enregistre.
    pub hold: Option<(usize, u64)>,
    pub presses: u64,
    /// Avis du bandeau (préréglage enregistré…) et son heure.
    pub notice: Option<(String, Instant)>,
    /// Préréglage affiché : les changements de la zone y sont enregistrés.
    pub active: Option<usize>,
    /// Module en édition (appui long sur sa tuile) : déplacer, retirer.
    pub editing: Option<usize>,
}

impl App {
    /// Zones du côte à côte sur la surface `s` : le média (`video`, ou
    /// `media` pour une musique sans fenêtre) et la zone d'accueil (`aside`).
    pub(super) fn panorama_side_panes(&self, s: &Surf, sp: &SideBySide) -> Vec<Pane> {
        let d = &self.cfg.general.panorama.design;
        let (pad, gap) = (d.padding, d.gap);
        let (iw, ih) = ((s.w - 2.0 * pad).max(2.0), (s.h - 2.0 * pad).max(2.0));
        let (a, b) = if s.w >= s.h {
            let w = ((iw - gap) / 2.0).max(1.0);
            (
                Rect {
                    x: pad,
                    y: pad,
                    w,
                    h: ih,
                },
                Rect {
                    x: pad + w + gap,
                    y: pad,
                    w,
                    h: ih,
                },
            )
        } else {
            let h = ((ih - gap) / 2.0).max(1.0);
            (
                Rect {
                    x: pad,
                    y: pad,
                    w: iw,
                    h,
                },
                Rect {
                    x: pad,
                    y: pad + h + gap,
                    w: iw,
                    h,
                },
            )
        };
        let (media, aside) = if sp.swapped { (b, a) } else { (a, b) };
        // Musique d'un lecteur sans fenêtre (MPRIS) : ses commandes ; sinon
        // le lecteur du Panorama (fenêtre, radio, ou choix des sources).
        let player = self.panorama.video.is_some() || self.panorama.radio.is_some();
        let playing = self.panorama.media.as_ref().is_some_and(|m| m.playing);
        let widget = if !player && playing { "media" } else { "video" };
        vec![
            Pane {
                rect: media,
                widget: widget.into(),
                index: None,
                bare: false,
            },
            Pane {
                rect: aside,
                widget: "aside".into(),
                index: None,
                bare: false,
            },
        ]
    }

    /// Une fenêtre occupe la zone d'accueil : elle y est visible (trou).
    pub(super) fn panorama_side_hosting(&self) -> bool {
        self.panorama
            .split
            .as_ref()
            .is_some_and(|sp| sp.guest.is_some())
    }

    /// Rectangle global de la zone d'accueil (première surface qui en montre une).
    pub(super) fn panorama_aside_target(&self) -> Option<(i32, i32, i32, i32)> {
        self.panorama_surfs().into_iter().find_map(|s| {
            let geo = self.panorama_geo(&s);
            let p = geo.panes.iter().find(|p| p.widget == "aside")?;
            self.panorama_global(&s, geo.content(p))
        })
    }

    /// Pose (ou rend) la fenêtre invitée selon la zone d'accueil affichée.
    pub(super) fn panorama_side_sync(&mut self, no_surface: bool) -> Task<Msg> {
        if no_surface {
            // Plus aucun Panorama : la fenêtre retrouve sa place.
            let guest = self.panorama.split.as_mut().and_then(|sp| {
                sp.placed = None;
                sp.guest.take()
            });
            return restore(guest);
        }
        let target = self.panorama_aside_target();
        let Some(sp) = self.panorama.split.as_mut() else {
            return Task::none();
        };
        match (sp.guest.clone(), target) {
            (Some(g), Some(rect)) if sp.placed != Some(rect) => {
                sp.placed = Some(rect);
                blocking(move || g.place(rect))
            }
            _ => Task::none(),
        }
    }

    /// Renouvelle le bail du cadre de la fenêtre invitée.
    pub(super) fn panorama_side_anchor(&self) -> Task<Msg> {
        match (
            self.panorama.split.as_ref().and_then(|sp| sp.guest.clone()),
            self.panorama_aside_target(),
        ) {
            (Some(g), Some(rect)) => blocking(move || g.renew(rect)),
            _ => Task::none(),
        }
    }

    /// Fenêtre invitée fermée hors du Panorama : on l'oublie.
    pub(super) fn panorama_side_gone(&mut self) {
        let clients = &self.clients;
        if let Some(sp) = self.panorama.split.as_mut()
            && sp
                .guest
                .as_ref()
                .is_some_and(|g| !clients.iter().any(|c| c.address == g.address))
        {
            sp.guest = None;
            sp.placed = None;
        }
    }

    pub(super) fn panorama_side_update(&mut self, m: PMsg) -> Task<Msg> {
        self.panorama.last_press = Some(std::time::Instant::now());
        match m {
            PMsg::Side(op) => {
                let op = match op {
                    "toggle" if self.panorama.split.is_some() => "off",
                    "toggle" => "on",
                    o => o,
                };
                match op {
                    "on" => {
                        let mut tasks = Vec::new();
                        if self.panorama.split.is_none() {
                            self.panorama.split = Some(SideBySide::default());
                            // La zone revient comme la fois d'avant.
                            if let Some(c) = self.side_saved("last") {
                                tasks.push(self.side_apply(c, true));
                            }
                        }
                        // Les profils Vidéo et Musique portent le côte à côte.
                        let profile = &self.cfg.general.panorama.design.shortcut_profile;
                        if !matches!(profile.as_str(), "video" | "audio") {
                            tasks.insert(0, self.panorama_update(PMsg::Profile("video".into())));
                        } else {
                            tasks.insert(0, self.sync_pip());
                        }
                        Task::batch(tasks)
                    }
                    "off" => {
                        let guest = self.panorama.split.take().and_then(|sp| sp.guest);
                        Task::batch([restore(guest), self.sync_pip()])
                    }
                    "swap" => {
                        if let Some(sp) = self.panorama.split.as_mut() {
                            sp.swapped = !sp.swapped;
                        }
                        self.sync_pip()
                    }
                    "pick" | "done" => {
                        if let Some(sp) = self.panorama.split.as_mut() {
                            sp.picking = op == "pick";
                            sp.editing = None;
                            sp.error = None;
                        }
                        Task::none()
                    }
                    "release" => {
                        let guest = self.panorama.split.as_mut().and_then(|sp| {
                            sp.placed = None;
                            sp.guest.take()
                        });
                        Task::batch([restore(guest), self.sync_pip(), self.side_changed()])
                    }
                    _ => Task::none(),
                }
            }
            PMsg::SideModule(id) => {
                if let Some(sp) = self.panorama.split.as_mut() {
                    if let Some(i) = sp.modules.iter().position(|m| *m == id) {
                        sp.modules.remove(i);
                    } else if sp.modules.len() < MAX_MODULES
                        && crate::panorama::modules::CATALOGUE
                            .iter()
                            .any(|(m, _, _)| *m == id)
                    {
                        sp.modules.push(id.clone());
                    }
                }
                let remember = self.side_changed();
                // Tuile Hue : relevé aussitôt.
                if id == "hue" {
                    return Task::batch([remember, self.panorama_hue_refresh()]);
                }
                remember
            }
            PMsg::SidePreset(i, true) => {
                let Some(sp) = self.panorama.split.as_mut() else {
                    return Task::none();
                };
                sp.presses += 1;
                let stamp = sp.presses;
                sp.hold = Some((i, stamp));
                Task::perform(tokio::time::sleep(HOLD), move |_| {
                    wrap(PMsg::SideHold(i, stamp))
                })
            }
            PMsg::SidePreset(i, false) => {
                let Some(sp) = self.panorama.split.as_mut() else {
                    return Task::none();
                };
                // Appui long déjà servi (enregistré) : le relâcher ne rappelle pas.
                if sp.hold.take().is_none_or(|(h, _)| h != i) {
                    return Task::none();
                }
                self.side_load(i)
            }
            // Glisser dans la zone : préréglage enregistré suivant (+1) ou
            // précédent (-1), en boucle.
            PMsg::SideSwipe(d) => {
                let defined: Vec<usize> = self
                    .side_presets()
                    .iter()
                    .enumerate()
                    .filter(|(_, on)| **on)
                    .map(|(i, _)| i)
                    .collect();
                let active = self.panorama.split.as_ref().and_then(|sp| sp.active);
                match next_preset(&defined, active, d) {
                    Some(i) => {
                        let load = self.side_load(i);
                        self.side_notice(crate::tr!("Préréglage {n}", n = i + 1));
                        load
                    }
                    None => {
                        self.side_notice(
                            crate::i18n::tr(
                                "Aucun préréglage : appui long sur 1 à 5 pour en enregistrer",
                            )
                            .to_string(),
                        );
                        Task::none()
                    }
                }
            }
            PMsg::SideHold(i, stamp) => {
                let Some(sp) = self.panorama.split.as_mut() else {
                    return Task::none();
                };
                if sp.hold != Some((i, stamp)) {
                    return Task::none();
                }
                sp.hold = None;
                sp.active = Some(i);
                let c = self.side_content();
                self.side_notice(crate::tr!("Préréglage {n} enregistré", n = i + 1));
                self.side_store(&format!("preset{}", i + 1), &c)
            }
            PMsg::SideEdit(e) => {
                if let Some(sp) = self.panorama.split.as_mut() {
                    sp.editing = e.filter(|i| *i < sp.modules.len());
                }
                Task::none()
            }
            PMsg::SideMove(i, d) => {
                let Some(sp) = self.panorama.split.as_mut() else {
                    return Task::none();
                };
                let j = i as i64 + i64::from(d);
                if i >= sp.modules.len() || j < 0 || j as usize >= sp.modules.len() {
                    return Task::none();
                }
                sp.modules.swap(i, j as usize);
                // L'édition suit le module déplacé.
                sp.editing = Some(j as usize);
                self.side_changed()
            }
            PMsg::SideRemove(i) => {
                let Some(sp) = self.panorama.split.as_mut() else {
                    return Task::none();
                };
                if i < sp.modules.len() {
                    sp.modules.remove(i);
                }
                sp.editing = None;
                self.side_changed()
            }
            PMsg::SideWindow(addr) => {
                let Some(rect) = self.panorama_aside_target() else {
                    return Task::none();
                };
                let Some(sp) = self.panorama.split.as_mut() else {
                    return Task::none();
                };
                // Une seule fenêtre : la précédente est d'abord rendue.
                let old = sp.guest.take();
                sp.placed = None;
                sp.picking = false;
                sp.busy = true;
                sp.error = None;
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            if let Some(o) = old {
                                let _ = o.restore();
                            }
                            Guest::take(&addr, rect)
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| wrap(PMsg::SideTaken(r)),
                )
            }
            PMsg::SideTaken(r) => match (r, self.panorama.split.as_mut()) {
                (Ok(g), Some(sp)) => {
                    sp.busy = false;
                    sp.guest = Some(g);
                    Task::batch([self.sync_pip(), self.side_changed()])
                }
                // Côte à côte quitté pendant le transfert : la fenêtre repart.
                (Ok(g), None) => restore(Some(g)),
                (Err(e), Some(sp)) => {
                    sp.busy = false;
                    sp.error = Some(crate::tr!("Fenêtre non ramenée : {e}", e = e));
                    Task::none()
                }
                (Err(_), None) => Task::none(),
            },
            PMsg::SideDone(r) => {
                if let Err(e) = r {
                    tracing::warn!("Panorama : côte à côte : {e}");
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    /// Contenu affiché de la zone d'accueil.
    fn side_content(&self) -> Content {
        let sp = self.panorama.split.clone().unwrap_or_default();
        Content {
            modules: sp.modules,
            window: sp.guest.map(|g| g.class).filter(|c| !c.is_empty()),
        }
    }

    /// Contenu retenu sous `key` (`last`, `preset1`…).
    fn side_saved(&self, key: &str) -> Option<Content> {
        Content::decode(&crate::panorama::modules::option(&self.cfg, OPTION, key))
    }

    /// Préréglages enregistrés (pour le bandeau).
    fn side_presets(&self) -> [bool; PRESETS] {
        std::array::from_fn(|i| self.side_saved(&format!("preset{}", i + 1)).is_some())
    }

    fn side_store(&mut self, key: &str, c: &Content) -> Task<Msg> {
        let all = self
            .settings
            .plugin_options
            .clone()
            .unwrap_or_else(|| self.cfg.general.plugin_options.clone());
        self.settings.plugin_options = Some(crate::panorama::modules::with_option(
            all,
            OPTION,
            key,
            &c.encode(),
        ));
        self.commit()
    }

    /// Retient le contenu affiché pour le prochain côte à côte.
    fn side_remember(&mut self) -> Task<Msg> {
        let c = self.side_content();
        if self.side_saved("last").as_ref() == Some(&c) {
            return Task::none();
        }
        self.side_store("last", &c)
    }

    /// Affiche le préréglage `i` (rang 0 à 4) et en fait le préréglage actif.
    fn side_load(&mut self, i: usize) -> Task<Msg> {
        if let Some(sp) = self.panorama.split.as_mut() {
            sp.active = Some(i);
            sp.editing = None;
        }
        match self.side_saved(&format!("preset{}", i + 1)) {
            Some(c) => self.side_apply(c, false),
            // Préréglage libre : la zone se vide (fenêtre rendue) pour
            // qu'on la compose ; elle s'y enregistre au fil des changements.
            None => {
                let clear = self.side_apply(Content::default(), true);
                self.side_notice(crate::tr!(
                    "Préréglage {n} vide : composez la zone, puis appui long pour l'enregistrer",
                    n = i + 1
                ));
                clear
            }
        }
    }

    /// La zone a changé : retenue pour la prochaine fois, et enregistrée
    /// dans le préréglage affiché s'il y en a un.
    fn side_changed(&mut self) -> Task<Msg> {
        let last = self.side_remember();
        let Some(i) = self.panorama.split.as_ref().and_then(|sp| sp.active) else {
            return last;
        };
        let c = self.side_content();
        let key = format!("preset{}", i + 1);
        if self.side_saved(&key).as_ref() == Some(&c) {
            return last;
        }
        Task::batch([last, self.side_store(&key, &c)])
    }

    fn side_notice(&mut self, msg: String) {
        if let Some(sp) = self.panorama.split.as_mut() {
            sp.notice = Some((msg, Instant::now()));
        }
    }

    /// Affiche un contenu retenu : ses modules, et sa fenêtre si une fenêtre
    /// de cette classe est ouverte (`quiet` : sans avis si elle ne l'est pas).
    fn side_apply(&mut self, c: Content, quiet: bool) -> Task<Msg> {
        let video = self.panorama.video.as_ref().and_then(|v| v.address.clone());
        let found = c.window.as_ref().and_then(|class| {
            self.clients
                .iter()
                .find(|w| {
                    &w.class == class
                        && !w.workspace.starts_with("special:")
                        && Some(&w.address) != video.as_ref()
                })
                .map(|w| w.address.clone())
        });
        let Some(sp) = self.panorama.split.as_mut() else {
            return Task::none();
        };
        sp.modules = c.modules.clone();
        sp.picking = false;
        let current = sp.guest.as_ref().map(|g| g.class.clone());
        let mut tasks = vec![self.side_remember()];
        match (&c.window, found) {
            // Déjà là : rien à ramener.
            (Some(class), _) if current.as_ref() == Some(class) => {}
            (Some(_), Some(addr)) => tasks.push(Task::done(wrap(PMsg::SideWindow(addr)))),
            (window, _) => {
                if let Some(class) = window
                    && !quiet
                {
                    self.side_notice(crate::tr!(
                        "Aucune fenêtre {class} ouverte",
                        class = class.clone()
                    ));
                }
                // Le préréglage n'a pas de fenêtre : celle affichée est rendue.
                if window.is_none() || !quiet {
                    let guest = self.panorama.split.as_mut().and_then(|sp| {
                        sp.placed = None;
                        sp.guest.take()
                    });
                    if guest.is_some() {
                        tasks.push(restore(guest));
                        tasks.push(self.sync_pip());
                    }
                }
            }
        }
        if c.modules.iter().any(|m| m == "hue") {
            tasks.push(self.panorama_hue_refresh());
        }
        Task::batch(tasks)
    }

    // -- Vue -------------------------------------------------------------------

    /// Bouton du bandeau : icône intégrée et libellé.
    fn side_button(
        &self,
        icon: &str,
        label: &str,
        h: f32,
        fg: Color,
        msg: Msg,
    ) -> Element<'_, Msg> {
        let g = h * 0.42;
        let glyph: Element<'_, Msg> = match crate::ui::icons::get(icon) {
            Some(handle) => svg(handle)
                .width(g)
                .height(g)
                .style(move |_, _| svg::Style { color: Some(fg) })
                .into(),
            None => Space::new().width(0).into(),
        };
        self.panorama_feedback(
            button(
                container(
                    row![
                        glyph,
                        text(crate::i18n::tr(label).to_string())
                            .size(h * 0.36)
                            .font(self.panorama_font())
                            .color(fg)
                            .wrapping(text::Wrapping::None)
                    ]
                    .spacing(h * 0.15)
                    .align_y(Alignment::Center),
                )
                .center_y(Length::Fill)
                .padding([0.0, h * 0.3]),
            )
            .height(h)
            .on_press(msg)
            .style(move |_, st| button::Style {
                background: Some(
                    fx::alpha(
                        fg,
                        match st {
                            button::Status::Pressed => 0.28,
                            button::Status::Hovered => 0.16,
                            _ => 0.08,
                        },
                    )
                    .into(),
                ),
                border: Border::default().rounded(h / 2.0),
                ..Default::default()
            }),
        )
    }

    /// Bandeau de la zone d'accueil : ajouter, rendre la fenêtre, inverser
    /// les côtés, revenir sans partage.
    pub(super) fn panorama_aside_header(
        &self,
        geo: &Geo,
        p: &Pane,
        fg: Color,
        bg: Color,
    ) -> Element<'_, Msg> {
        let t = geo.target;
        let h = t * 0.8;
        let compact = p.rect.w < t * 9.0;
        let sp = self.panorama.split.clone().unwrap_or_default();
        let lbl = |l: &'static str| if compact { "" } else { l };
        let mut r = row![].spacing(8).align_y(Alignment::Center);
        let title = match (&sp.notice, &sp.guest) {
            (Some((n, at)), _) if at.elapsed() < NOTICE => n.clone(),
            (_, Some(g)) if !g.title.is_empty() => g.title.clone(),
            (_, Some(g)) => g.class.clone(),
            _ => crate::i18n::tr("Zone d'accueil").to_string(),
        };
        let accent = self.panorama_palette().2;
        r = r.push(
            container(
                text(title)
                    .size(t * 0.34)
                    .color(fg)
                    .font(self.panorama_font())
                    .wrapping(text::Wrapping::None),
            )
            .width(Length::Fill)
            .clip(true),
        );
        // Préréglages 1 à 5 : appui = rappel, appui long = enregistrement.
        let saved = self.side_presets();
        let side = h * 0.85;
        let mut presets = row![].spacing(4).align_y(Alignment::Center);
        for (i, on) in saved.into_iter().enumerate() {
            let held = sp.hold.is_some_and(|(k, _)| k == i);
            let active = sp.active == Some(i);
            presets = presets.push(
                mouse_area(
                    container(
                        text((i + 1).to_string())
                            .size(side * 0.45)
                            .font(self.panorama_font())
                            .color(fg),
                    )
                    .width(side)
                    .height(side)
                    .center_x(side)
                    .center_y(side)
                    .style(move |_| container::Style {
                        background: Some(
                            fx::alpha(
                                if on || held { accent } else { fg },
                                if held {
                                    0.7
                                } else if on {
                                    0.32
                                } else {
                                    0.06
                                },
                            )
                            .into(),
                        ),
                        border: Border {
                            radius: (side / 2.0).into(),
                            width: if active { 2.5 } else { 1.0 },
                            color: fx::alpha(
                                if on || active { accent } else { fg },
                                if active {
                                    1.0
                                } else if on {
                                    0.8
                                } else {
                                    0.25
                                },
                            ),
                        },
                        ..Default::default()
                    }),
                )
                .on_press(wrap(PMsg::SidePreset(i, true)))
                .on_release(wrap(PMsg::SidePreset(i, false)))
                .interaction(iced::mouse::Interaction::Pointer),
            );
        }
        r = r.push(presets);
        if sp.guest.is_some() {
            r = r.push(self.side_button(
                "window-close",
                lbl("Rendre la fenêtre"),
                h,
                fg,
                wrap(PMsg::Side("release")),
            ));
        } else if sp.picking {
            r = r.push(self.side_button(
                "object-select",
                lbl("Terminé"),
                h,
                fg,
                wrap(PMsg::Side("done")),
            ));
        } else {
            r = r.push(self.side_button(
                "list-add",
                lbl("Ajouter"),
                h,
                fg,
                wrap(PMsg::Side("pick")),
            ));
        }
        r = r
            .push(self.side_button(
                "hyprdesk-split-swap",
                lbl("Inverser"),
                h,
                fg,
                wrap(PMsg::Side("swap")),
            ))
            .push(self.side_button(
                "hyprdesk-split-off",
                lbl("Sans partage"),
                h,
                fg,
                wrap(PMsg::Side("off")),
            ));
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

    /// Contenu de la zone d'accueil : la fenêtre (trou), le choix « + »,
    /// ou les modules ajoutés.
    pub(super) fn panorama_aside(&self, size: Size) -> Element<'_, Msg> {
        let d = &self.cfg.general.panorama.design;
        let sp = self.panorama.split.clone().unwrap_or_default();
        if sp.guest.is_some() {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        }
        if sp.picking {
            return self.panorama_aside_picker(&sp, size);
        }
        // Glisser dans la zone : préréglage suivant ou précédent. Les tuiles
        // à curseurs (Lumières, Musique) gardent leur glisser.
        let axis = if size.width >= size.height {
            Axis::Horizontal
        } else {
            Axis::Vertical
        };
        let (cols, w, h) = side_grid(sp.modules.len(), size, d.gap.min(16.0));
        let gap = d.gap.min(16.0);
        let yields = sp
            .modules
            .iter()
            .enumerate()
            .filter(|(_, m)| matches!(m.as_str(), "hue" | "media"))
            .map(|(i, _)| {
                let (r, c) = ((i / cols) as f32, (i % cols) as f32);
                iced::Rectangle::new(
                    iced::Point::new(8.0 + c * (w + gap), 8.0 + r * (h + gap)),
                    Size::new(w, h),
                )
            })
            .collect();
        swipe(self.panorama_aside_body(&sp, size), axis, |d| {
            wrap(PMsg::SideSwipe(d))
        })
        .yield_to(yields)
        .motion(self.panorama_motion())
        .into()
    }

    /// Zone d'accueil sans fenêtre ni choix ouvert : modules, ou le « + ».
    fn panorama_aside_body(&self, sp: &SideBySide, size: Size) -> Element<'_, Msg> {
        let (bg, fg, accent) = self.panorama_palette();
        let d = &self.cfg.general.panorama.design;
        let dim = fx::alpha(fg, 0.65);
        if sp.busy {
            return container(
                text(crate::i18n::tr("Fenêtre en route…"))
                    .size(crate::ui::tokens::ts(15.0))
                    .color(dim),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
        }
        if sp.modules.is_empty() {
            // Zone vide : un grand « + ».
            let side = (size.width.min(size.height) * 0.3).clamp(64.0, 180.0);
            let plus: Element<'_, Msg> = match crate::ui::icons::get("list-add") {
                Some(h) => svg(h)
                    .width(side * 0.4)
                    .height(side * 0.4)
                    .style(move |_, _| svg::Style { color: Some(fg) })
                    .into(),
                None => text("+").size(side * 0.5).color(fg).into(),
            };
            let mut col = column![
                self.panorama_feedback(
                    button(
                        container(plus)
                            .center_x(Length::Fill)
                            .center_y(Length::Fill)
                    )
                    .width(side)
                    .height(side)
                    .on_press(wrap(PMsg::Side("pick")))
                    .style(move |_, st| button::Style {
                        background: Some(
                            fx::alpha(
                                accent,
                                if matches!(st, button::Status::Pressed) {
                                    0.5
                                } else {
                                    0.15
                                },
                            )
                            .into(),
                        ),
                        border: Border {
                            radius: (side * 0.22).into(),
                            width: 1.0,
                            color: fx::alpha(accent, 0.5),
                        },
                        ..Default::default()
                    })
                ),
                text(crate::i18n::tr("Ajoutez un module ou ramenez une fenêtre"))
                    .size(15.0 * d.text_scale)
                    .color(dim),
            ]
            .spacing(14)
            .align_x(Alignment::Center);
            if let Some(e) = &sp.error {
                col = col.push(text(e.clone()).size(crate::ui::tokens::ts(13.0)).color(dim));
            }
            return container(col)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        }
        // Modules : une ou deux colonnes selon la forme de la zone.
        let gap = d.gap.min(16.0);
        let n = sp.modules.len();
        let (cols, w, h) = side_grid(n, size, gap);
        let mut grid = column![].spacing(gap);
        for (r, chunk) in sp.modules.chunks(cols).enumerate() {
            let mut line = row![].spacing(gap);
            for (k, m) in chunk.iter().enumerate() {
                let i = r * cols + k;
                let editing = sp.editing == Some(i);
                let card = container(self.panorama_module_body(m, Size::new(w, h)))
                    .width(w)
                    .height(h)
                    .clip(true)
                    .style(move |_| container::Style {
                        background: Some(fx::alpha(bg, d.card_opacity.max(0.06)).into()),
                        border: Border {
                            radius: d.radius.min(18.0).into(),
                            width: if editing { 2.0 } else { 1.0 },
                            color: if editing {
                                accent
                            } else {
                                fx::alpha(fg, d.border_opacity.max(0.12))
                            },
                        },
                        ..Default::default()
                    });
                // Appui long : la tuile passe en édition (ou en sort).
                let tile: Element<'_, Msg> =
                    super::hold::hold(card, wrap(PMsg::SideEdit((!editing).then_some(i)))).into();
                line = line.push(if editing {
                    iced::widget::stack![tile, self.side_edit_bar(i, n, w, h)].into()
                } else {
                    tile
                });
            }
            grid = grid.push(line);
        }
        container(grid).padding(8).into()
    }

    /// Commandes d'une tuile en édition : avant, après, retirer, terminé.
    fn side_edit_bar(&self, i: usize, n: usize, w: f32, h: f32) -> Element<'_, Msg> {
        let (bg, fg, _) = self.panorama_palette();
        let b = (w.min(h) * 0.22).clamp(36.0, 56.0);
        let key = |icon: &str, msg: Option<Msg>| -> Element<'_, Msg> {
            let tint = if msg.is_some() {
                fg
            } else {
                fx::alpha(fg, 0.3)
            };
            let glyph: Element<'_, Msg> = match crate::ui::icons::get(icon) {
                Some(handle) => svg(handle)
                    .width(b * 0.45)
                    .height(b * 0.45)
                    .style(move |_, _| svg::Style { color: Some(tint) })
                    .into(),
                None => Space::new().into(),
            };
            let mut bt = button(
                container(glyph)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            )
            .width(b)
            .height(b)
            .style(move |_, st| button::Style {
                background: Some(
                    fx::alpha(
                        fg,
                        match st {
                            button::Status::Pressed => 0.32,
                            button::Status::Hovered => 0.2,
                            _ => 0.12,
                        },
                    )
                    .into(),
                ),
                border: Border::default().rounded(b / 2.0),
                ..Default::default()
            });
            if let Some(m) = msg {
                bt = bt.on_press(m);
            }
            self.panorama_feedback(bt)
        };
        let bar = row![
            key("go-previous", (i > 0).then(|| wrap(PMsg::SideMove(i, -1)))),
            key("go-next", (i + 1 < n).then(|| wrap(PMsg::SideMove(i, 1)))),
            key("user-trash", Some(wrap(PMsg::SideRemove(i)))),
            key("object-select", Some(wrap(PMsg::SideEdit(None)))),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        container(container(bar).padding(10).style(move |_| container::Style {
            background: Some(fx::alpha(bg, 0.85).into()),
            border: Border::default().rounded(b * 0.6),
            ..Default::default()
        }))
        .width(w)
        .height(h)
        .center_x(w)
        .center_y(h)
        .style(move |_| container::Style {
            background: Some(fx::alpha(bg, 0.45).into()),
            border: Border::default().rounded(d_radius(self)),
            ..Default::default()
        })
        .into()
    }

    /// Choix « + » : modules à cocher, puis les fenêtres ouvertes à ramener.
    fn panorama_aside_picker(&self, sp: &SideBySide, size: Size) -> Element<'_, Msg> {
        let (bg, fg, accent) = self.panorama_palette();
        let dim = fx::alpha(fg, 0.65);
        let heading = |s: &str| {
            text(crate::i18n::tr(s).to_string())
                .size(crate::ui::tokens::ts(15.0))
                .font(iced::Font {
                    weight: iced::font::Weight::Semibold,
                    ..self.panorama_font()
                })
                .color(fg)
        };
        let chip = move |on: bool| {
            move |_: &iced::Theme, st: button::Status| button::Style {
                background: Some(
                    fx::alpha(
                        accent,
                        if matches!(st, button::Status::Pressed) {
                            0.55
                        } else if on {
                            0.35
                        } else {
                            0.1
                        },
                    )
                    .into(),
                ),
                border: Border {
                    radius: 12.0.into(),
                    width: if on { 2.0 } else { 1.0 },
                    color: fx::alpha(if on { accent } else { fg }, if on { 0.9 } else { 0.2 }),
                },
                text_color: fg,
                ..Default::default()
            }
        };
        let mut chips: Vec<Element<'_, Msg>> = Vec::new();
        for (id, label, icon) in crate::panorama::modules::CATALOGUE {
            let on = sp.modules.iter().any(|m| m == id);
            let glyph: Element<'_, Msg> = match crate::ui::icons::get(icon) {
                Some(h) => svg(h)
                    .width(22)
                    .height(22)
                    .style(move |_, _| svg::Style { color: Some(fg) })
                    .into(),
                None => Space::new().width(0).into(),
            };
            chips.push(
                self.panorama_feedback(
                    button(
                        column![
                            glyph,
                            text(crate::i18n::tr(label).to_string())
                                .size(crate::ui::tokens::ts(12.0))
                                .color(fg)
                                .wrapping(text::Wrapping::None)
                        ]
                        .spacing(6)
                        .align_x(Alignment::Center),
                    )
                    .width(104)
                    .padding([10, 6])
                    .on_press(wrap(PMsg::SideModule(id.to_string())))
                    .style(chip(on)),
                ),
            );
        }
        // Modules en grille : tous visibles d'un coup, sans défiler de côté.
        let cols = (((size.width - 24.0 + 8.0) / 112.0).floor() as usize).max(1);
        let mut mods = column![].spacing(8);
        let mut it = chips.into_iter().peekable();
        while it.peek().is_some() {
            let mut line = row![].spacing(8);
            for c in it.by_ref().take(cols) {
                line = line.push(c);
            }
            mods = mods.push(line);
        }
        let video = self
            .panorama
            .video
            .as_ref()
            .and_then(|v| v.address.as_deref());
        let mut wins = column![].spacing(6);
        let mut any = false;
        for c in self
            .clients
            .iter()
            .filter(|c| !c.workspace.starts_with("special:") && Some(c.address.as_str()) != video)
        {
            any = true;
            let img = self
                .class_imgs
                .get(&c.class)
                .copied()
                .unwrap_or(crate::ui::IconImg::None);
            let icon: Element<'_, Msg> = self
                .img_element(img, 26.0, 1.0)
                .unwrap_or_else(|| Space::new().width(26).height(26).into());
            let title = if c.title.is_empty() {
                c.class.clone()
            } else {
                c.title.clone()
            };
            wins = wins.push(
                self.panorama_feedback(
                    button(
                        row![
                            icon,
                            container(
                                column![
                                    text(title)
                                        .size(crate::ui::tokens::ts(13.0))
                                        .color(fg)
                                        .wrapping(text::Wrapping::None),
                                    text(format!("{} · {}", c.class, c.monitor))
                                        .size(crate::ui::tokens::ts(11.0))
                                        .color(dim)
                                        .wrapping(text::Wrapping::None),
                                ]
                                .spacing(1)
                            )
                            .width(Length::Fill)
                            .clip(true),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    )
                    .width(Length::Fill)
                    .padding([6, 10])
                    .on_press(wrap(PMsg::SideWindow(c.address.clone())))
                    .style(move |_, st| button::Style {
                        background: Some(
                            fx::alpha(
                                if matches!(st, button::Status::Pressed | button::Status::Hovered) {
                                    accent
                                } else {
                                    bg
                                },
                                0.5,
                            )
                            .into(),
                        ),
                        border: Border {
                            radius: 10.0.into(),
                            width: 1.0,
                            color: fx::alpha(fg, 0.15),
                        },
                        text_color: fg,
                        ..Default::default()
                    }),
                ),
            );
        }
        if !any {
            wins = wins.push(
                text(crate::i18n::tr("Aucune fenêtre ouverte"))
                    .size(crate::ui::tokens::ts(13.0))
                    .color(dim),
            );
        }
        let done = self.panorama_feedback(
            button(
                text(crate::i18n::tr("Terminé").to_string())
                    .size(crate::ui::tokens::ts(14.0))
                    .color(fg),
            )
            .padding([8, 18])
            .on_press(wrap(PMsg::Side("done")))
            .style(chip(true)),
        );
        // « Terminé » toujours en haut, hors de ce qui défile.
        let top = row![heading("Modules"), Space::new().width(Length::Fill), done]
            .align_y(Alignment::Center)
            .padding([8, 12]);
        let mut col = column![mods, heading("Ramener une fenêtre"), wins].spacing(10);
        if let Some(e) = &sp.error {
            col = col.push(text(e.clone()).size(crate::ui::tokens::ts(13.0)).color(dim));
        }
        // Le tout défile au doigt, n'importe où dans la zone.
        column![
            top,
            super::drag::drag_scroll(
                scrollable(col.padding([4, 12]))
                    .width(Length::Fill)
                    .height(Length::Fill)
            )
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }
}

/// Grille des modules de la zone d'accueil (marge de 8) : (colonnes,
/// largeur, hauteur d'une tuile). Deux colonnes sur une zone large.
fn side_grid(n: usize, size: Size, gap: f32) -> (usize, f32, f32) {
    let cols = if n > 1 && size.width > size.height * 1.3 {
        2
    } else {
        1
    };
    let rows = n.div_ceil(cols).max(1);
    let inner = 16.0;
    let w = ((size.width - inner - gap * (cols - 1) as f32) / cols as f32).max(40.0);
    let h = ((size.height - inner - gap * (rows - 1) as f32) / rows as f32).max(40.0);
    (cols, w, h)
}

/// Préréglage suivant (`d` > 0) ou précédent parmi les rangs `defined`
/// (croissants), en boucle ; depuis aucun : le premier ou le dernier.
fn next_preset(defined: &[usize], active: Option<usize>, d: i32) -> Option<usize> {
    if defined.is_empty() {
        return None;
    }
    let next = match active.and_then(|a| defined.iter().position(|x| *x == a)) {
        Some(p) => (p as i64 + i64::from(d.signum())).rem_euclid(defined.len() as i64) as usize,
        // Actif vide (ou aucun) : le suivant ou le précédent selon son rang.
        None => match (active, d > 0) {
            (Some(a), true) => defined.iter().position(|x| *x > a).unwrap_or(0),
            (Some(a), false) => defined
                .iter()
                .rposition(|x| *x < a)
                .unwrap_or(defined.len() - 1),
            (None, true) => 0,
            (None, false) => defined.len() - 1,
        },
    };
    Some(defined[next])
}

/// Arrondi des tuiles de la zone d'accueil.
fn d_radius(app: &App) -> f32 {
    app.cfg.general.panorama.design.radius.min(18.0)
}

/// Tâche bloquante (appels à Hyprland) hors du fil de l'interface.
fn blocking(f: impl FnOnce() -> Result<(), String> + Send + 'static) -> Task<Msg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(f)
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
        },
        |r| wrap(PMsg::SideDone(r)),
    )
}

/// Rend la fenêtre invitée, s'il y en a une.
fn restore(guest: Option<Guest>) -> Task<Msg> {
    match guest {
        Some(g) => blocking(move || g.restore()),
        None => Task::none(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contenu_retenu() {
        let c = Content {
            modules: vec!["notes".into(), "hue".into()],
            window: Some("kitty".into()),
        };
        assert_eq!(c.encode(), "notes,hue|kitty");
        assert_eq!(Content::decode(&c.encode()), Some(c));
        // Préréglage vide : enregistré quand même.
        let vide = Content::default();
        assert_eq!(vide.encode(), "|");
        assert_eq!(Content::decode("|"), Some(Content::default()));
        assert_eq!(Content::decode(""), None);
        // Modules inconnus, doublons et excédent écartés.
        let d = Content::decode("notes,inconnu,notes,hue,clock,calendar,system").unwrap();
        assert_eq!(d.modules, ["notes", "hue", "clock", "calendar"]);
        assert_eq!(d.window, None);
    }

    #[test]
    fn prereglage_suivant_en_boucle() {
        let def = [0, 2, 4];
        assert_eq!(next_preset(&def, Some(0), 1), Some(2));
        assert_eq!(next_preset(&def, Some(4), 1), Some(0));
        assert_eq!(next_preset(&def, Some(0), -1), Some(4));
        // Depuis un préréglage vide (rang 1) : ses voisins enregistrés.
        assert_eq!(next_preset(&def, Some(1), 1), Some(2));
        assert_eq!(next_preset(&def, Some(1), -1), Some(0));
        assert_eq!(next_preset(&def, None, 1), Some(0));
        assert_eq!(next_preset(&def, None, -1), Some(4));
        assert_eq!(next_preset(&[], None, 1), None);
    }
}
