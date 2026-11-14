//! Écran « Espaces de travail » (`hyprdesk overview`, SUPER+Tab) : chaque
//! workspace en carte, ses fenêtres en minimap à leur place réelle. Une
//! fenêtre se glisse d'un espace à l'autre ; un clic l'affiche ; Entrée ouvre
//! l'espace choisi, Échap referme ; double-clic sur un titre : renomme
//! l'espace pour la session ; « + Nouvel espace » en crée un.

use std::time::Instant;

use iced::widget::{Space, button, column, container, mouse_area, row, stack, text, text_input};
use iced::window::Id;
use iced::{
    Alignment, Background, Border, Color, Element, Length, Point, Rectangle, Shadow, Size, Task,
    Vector, keyboard,
};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::{App, Fade, IconImg, Msg, color};
use crate::config::MAX_WORKSPACES;

pub const NAMESPACE: &str = "hyprdesk-overview";

/// Grille : cartes bornées, centrées sous le titre.
const CARD_MAX_W: f32 = 620.0;
const CARD_MAX_H: f32 = 380.0;
const GAP: f32 = 18.0;
const MARGIN: f32 = 40.0;
const TITLE_H: f32 = 72.0;
const HINTS_H: f32 = 64.0;

/// Champ de saisie du renommage (un seul à la fois).
const RENAME_ID: &str = "hyprdesk-overview-rename";

pub struct Overview {
    pub win: Id,
    /// Écran où l'écran s'affiche (celui qui a le focus à l'ouverture).
    pub monitor: String,
    /// Carte choisie au clavier (Entrée l'ouvre).
    pub selected: usize,
    /// Saisie en cours : carte et texte.
    pub renaming: Option<(usize, String)>,
    /// Le renommage est parti : le champ reste ouvert jusqu'au résultat
    /// (Hyprland peut tarder ; l'échec ne doit pas passer inaperçu).
    pub renaming_pending: bool,
    /// Pressé dans une carte : fenêtre saisie ou simple clic, origine.
    pub press: Option<Press>,
    /// Dernier clic sur un titre (double-clic : renommer).
    last_title: Option<(usize, Instant)>,
    pub visibility: Fade,
}

/// Pressé dans une carte : fenêtre saisie (glisser-déposer d'un espace à
/// l'autre) ou simple clic, avec l'origine du pressé.
#[derive(Debug, Clone)]
pub struct Press {
    pub card: usize,
    /// Fenêtre sous le curseur à la saisie (`None` : le fond de la carte).
    pub window: Option<Win>,
    pub start: Point,
    /// Le titre (double-clic de renommage), pas la minimap.
    pub title: bool,
}

#[derive(Debug, Clone)]
pub struct Win {
    pub address: String,
    pub class: String,
    pub title: String,
}

#[derive(Debug, Clone)]
pub enum OMsg {
    /// Géométries relues : minimaps à jour (écran d'organisation et
    /// miniatures de la barre).
    Geos(Vec<crate::hypr::WindowGeo>),
    /// Pressé dans une carte : (carte, sur le titre).
    Grab(usize, bool),
    /// Une carte est cliquée : afficher ce workspace.
    Pick(String),
    /// Titre pressé deux fois : renommer l'espace.
    RenameStart(usize),
    RenameDraft(String),
    RenameSubmit,
    /// Réponse du renommage : Ok, ou l'erreur à afficher.
    RenameDone(Result<(), String>),
    /// Le champ de renommage vient d'apparaître : il prend le focus.
    RenameFocus,
    /// Espace après le dernier existant.
    NewWorkspace,
}

/// Une carte, données tirées de l'état Hyprland courant.
#[derive(Clone, Debug)]
pub struct WsCard {
    pub name: String,
    pub monitor: String,
    pub active: bool,
    pub id: i64,
}

impl Overview {
    pub(super) fn animating(&self, now: Instant) -> bool {
        !self.visibility.done(now)
    }

    /// Écran vierge : aperçu et tests le posent avec leur fondu.
    pub(super) fn new(win: Id, monitor: String, visibility: Fade) -> Self {
        Overview {
            win,
            monitor,
            selected: 0,
            renaming: None,
            renaming_pending: false,
            press: None,
            last_title: None,
            visibility,
        }
    }
}

impl App {
    /// Ouvre l'écran sur l'écran qui a le focus.
    pub(super) fn open_overview(&mut self) -> Task<Msg> {
        let monitor = self
            .monitors
            .iter()
            .find(|m| m.focused)
            .or_else(|| self.monitors.first())
            .map(|m| m.name.clone())
            .unwrap_or_default();
        let win = Id::unique();
        let now = Instant::now();
        self.overview = Some(Overview::new(
            win,
            monitor.clone(),
            Fade {
                from: 0.0,
                to: 1.0,
                start: now,
                dur: self.bar_duration(180),
            },
        ));
        let output = if monitor.is_empty() {
            OutputOption::Active
        } else {
            OutputOption::OutputName(monitor)
        };
        Task::batch([
            Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    // Plein écran, par-dessus tout (calque overlay) : il
                    // ignore les zones réservées, la barre comprise.
                    size: Some((0, 0)),
                    layer: Layer::Overlay,
                    anchor: LsAnchor::Top | LsAnchor::Bottom | LsAnchor::Left | LsAnchor::Right,
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option: output,
                    namespace: Some(NAMESPACE.into()),
                    ..Default::default()
                },
                id: win,
            }),
            Task::perform(crate::hypr::window_geometries_async(), |l| {
                Msg::Overview(OMsg::Geos(l))
            }),
            Task::done(Msg::Frame),
        ])
    }

    pub(super) fn close_overview(&mut self) -> Task<Msg> {
        let duration = self.bar_duration(120);
        if duration.is_zero() {
            return self.destroy_overview();
        }
        if let Some(o) = self.overview.as_mut() {
            let now = Instant::now();
            o.visibility = Fade {
                from: o.visibility.value(now),
                to: 0.0,
                start: now,
                dur: duration,
            };
        }
        Task::done(Msg::Frame)
    }

    pub(super) fn destroy_overview(&mut self) -> Task<Msg> {
        match self.overview.take() {
            Some(o) => iced::window::close(o.win),
            None => Task::none(),
        }
    }

    /// Solde le fondu d'entrée/sortie : fermé à la fin de la sortie.
    pub(super) fn overview_frame(&mut self) -> Task<Msg> {
        let now = Instant::now();
        let still = self.bar_duration(180).is_zero();
        if self
            .overview
            .as_ref()
            .is_some_and(|o| o.visibility.to == 0.0 && (still || o.visibility.done(now)))
        {
            return self.destroy_overview();
        }
        Task::none()
    }

    /// Cartes de l'écran : les workspaces de tous les écrans, dans l'ordre.
    pub(super) fn overview_cards(&self) -> Vec<WsCard> {
        self.workspaces
            .iter()
            .map(|w| WsCard {
                name: w.name.clone(),
                monitor: w.monitor.clone(),
                active: self
                    .monitors
                    .iter()
                    .any(|m| m.name == w.monitor && m.workspace == w.name),
                id: w.id,
            })
            .collect()
    }

    /// Cartes avec leur rectangle à l'écran : une seule définition pour la
    /// vue et pour la détection du dépôt d'une fenêtre glissée.
    fn overview_layout(&self, surface: Size) -> Vec<(Rectangle, WsCard)> {
        let cards = self.overview_cards();
        let n = cards.len();
        if n == 0 {
            return Vec::new();
        }
        let cols = (n as f32).sqrt().ceil() as usize;
        let rows = n.div_ceil(cols);
        let avail_w = (surface.width - 2.0 * MARGIN).max(120.0);
        let avail_h = (surface.height - TITLE_H - HINTS_H - 2.0 * MARGIN).max(120.0);
        let cw = ((avail_w - GAP * (cols as f32 - 1.0)) / cols as f32).min(CARD_MAX_W);
        let ch = ((avail_h - GAP * (rows as f32 - 1.0)) / rows as f32).min(CARD_MAX_H);
        let grid_w = cw * cols as f32 + GAP * (cols as f32 - 1.0);
        let grid_h = ch * rows as f32 + GAP * (rows as f32 - 1.0);
        let x0 = ((surface.width - grid_w) / 2.0).max(MARGIN);
        let y0 = TITLE_H + ((surface.height - TITLE_H - HINTS_H - grid_h) / 2.0).max(MARGIN / 2.0);
        cards
            .into_iter()
            .enumerate()
            .map(|(i, c)| {
                let (col, row) = (i % cols, i / cols);
                (
                    Rectangle::new(
                        Point::new(x0 + (cw + GAP) * col as f32, y0 + (ch + GAP) * row as f32),
                        Size::new(cw, ch),
                    ),
                    c,
                )
            })
            .collect()
    }

    /// Taille de la surface (relue à chaque `Resized`, l'écran sinon).
    fn overview_surface(&self, o: &Overview) -> Size {
        self.sizes
            .get(&o.win)
            .copied()
            .or_else(|| {
                self.monitors
                    .iter()
                    .find(|m| m.name == o.monitor)
                    .map(|m| Size::new(m.width as f32, m.height as f32))
            })
            .unwrap_or(Size::new(1280.0, 800.0))
    }

    /// Fenêtres d'un espace, normalisées 0..1 dans sa minimap (bounding-box
    /// des fenêtres, une marge autour) : le tiling et les flottants sont
    /// montrés à leur place réelle.
    pub(super) fn overview_windows(&self, name: &str) -> Vec<(Rectangle, crate::hypr::WindowGeo)> {
        let wins: Vec<_> = self
            .ws_geos
            .iter()
            .filter(|w| w.workspace == name)
            .collect();
        if wins.is_empty() {
            return Vec::new();
        }
        let rects: Vec<_> = wins.iter().map(|w| w.rect).collect();
        let norm = normalize_rects(&rects);
        wins.into_iter()
            .zip(norm)
            .map(|(w, (x, y, wd, h))| {
                (
                    Rectangle::new(Point::new(x, y), Size::new(wd, h)),
                    w.clone(),
                )
            })
            .collect()
    }

    /// Intérieur d'une carte : la minimap sous le titre.
    fn overview_inner(rect: Rectangle) -> Rectangle {
        Rectangle::new(
            Point::new(rect.x + 12.0, rect.y + 40.0),
            Size::new(
                (rect.width - 24.0).max(10.0),
                (rect.height - 52.0).max(10.0),
            ),
        )
    }

    /// Rectangle à l'écran d'une fenêtre normalisée dans la minimap.
    fn overview_win_rect(inner: Rectangle, r: &Rectangle) -> Rectangle {
        Rectangle::new(
            Point::new(inner.x + r.x * inner.width, inner.y + r.y * inner.height),
            Size::new(r.width * inner.width, r.height * inner.height),
        )
    }

    /// Fenêtre sous le curseur dans la carte `card`, ou rien.
    fn overview_window_at(&self, card: usize, p: Point) -> Option<Win> {
        let rect = self
            .overview
            .as_ref()
            .map(|o| self.overview_surface(o))
            .unwrap_or_default();
        let (rect, c) = self.overview_layout(rect).get(card).cloned()?;
        let inner = Self::overview_inner(rect);
        if !inner.contains(p) {
            return None;
        }
        self.overview_windows(&c.name)
            .into_iter()
            .find_map(|(r, w)| {
                let wr = Self::overview_win_rect(inner, &r);
                wr.contains(p).then(|| Win {
                    address: w.address.clone(),
                    class: w.class.clone(),
                    title: w.title.clone(),
                })
            })
    }

    pub(super) fn overview_update(&mut self, m: OMsg) -> Task<Msg> {
        match m {
            OMsg::Geos(list) => {
                self.ws_geos = list;
                Task::none()
            }
            OMsg::Grab(card, title) => {
                if self.overview.as_ref().is_some_and(|o| o.renaming.is_some()) {
                    return Task::none();
                }
                let Some(win) = self.overview.as_ref().map(|o| o.win) else {
                    return Task::none();
                };
                let start = self.cursor.get(&win).copied().unwrap_or_default();
                let window = if title {
                    None
                } else {
                    self.overview_window_at(card, start)
                };
                if let Some(o) = self.overview.as_mut() {
                    o.press = Some(Press {
                        card,
                        window,
                        start,
                        title,
                    });
                }
                Task::none()
            }
            OMsg::Pick(name) => {
                let close = self.close_overview();
                Task::batch([focus_task(name), close])
            }
            OMsg::RenameStart(i) => {
                let Some(name) = self.overview_cards().get(i).map(|c| c.name.clone()) else {
                    return Task::none();
                };
                if let Some(o) = self.overview.as_mut() {
                    o.renaming = Some((i, name));
                }
                iced::widget::operation::focus(RENAME_ID)
            }
            OMsg::RenameFocus => iced::widget::operation::focus(RENAME_ID),
            OMsg::RenameDraft(s) => {
                if let Some((_, d)) = self.overview.as_mut().and_then(|o| o.renaming.as_mut()) {
                    *d = s;
                }
                Task::none()
            }
            OMsg::RenameSubmit => {
                // Un renommage déjà en vol : Entrée supplémentaire ignorée.
                if self.overview.as_ref().is_some_and(|o| o.renaming_pending) {
                    return Task::none();
                }
                let Some((i, name)) = self.overview.as_ref().and_then(|o| o.renaming.clone())
                else {
                    return Task::none();
                };
                let Some(card) = self.overview_cards().get(i).cloned() else {
                    return Task::none();
                };
                if name.trim().is_empty() || name == card.name {
                    return Task::none();
                }
                if let Some(o) = self.overview.as_mut() {
                    o.renaming_pending = true;
                }
                rename_task(card.id, name)
            }
            OMsg::RenameDone(r) => {
                let Some(o) = self.overview.as_mut() else {
                    return Task::none();
                };
                o.renaming_pending = false;
                match r {
                    // Confirmé : le champ se ferme, l'événement renameworkspace
                    // met le nom à jour partout.
                    Ok(()) => {
                        o.renaming = None;
                        Task::none()
                    }
                    // Échec (timeout du socket compris) : le champ reste ouvert,
                    // la bulle de notification porte l'erreur.
                    Err(e) => {
                        let msg = crate::tr!("Renommage de l'espace impossible : {e}", e = e);
                        crate::hypr::notify(0, 8000, &msg);
                        Task::none()
                    }
                }
            }
            OMsg::NewWorkspace => {
                let next = self.workspaces.iter().map(|w| w.id).max().unwrap_or(0) + 1;
                if next > MAX_WORKSPACES as i64 {
                    return Task::none();
                }
                focus_task(next.to_string())
            }
        }
    }

    /// Relâché dans l'écran : clic (ouvrir), dépôt (changer d'espace), ou
    /// double-clic sur un titre (renommer).
    pub(super) fn overview_release(&mut self) -> Task<Msg> {
        let Some(o) = self.overview.as_ref() else {
            return Task::none();
        };
        let win = o.win;
        let press = o.press.clone();
        let last_title = o.last_title;
        let Some(p) = press else {
            return Task::none();
        };
        let cur = self.cursor.get(&win).copied().unwrap_or(p.start);
        if p.title {
            // Double-clic sur le titre : renommer.
            let now = Instant::now();
            let double = last_title
                .is_some_and(|(i, at)| i == p.card && now.duration_since(at) < super::DOUBLE_CLICK);
            if let Some(o) = self.overview.as_mut() {
                o.press = None;
                o.last_title = if double { None } else { Some((p.card, now)) };
            }
            if double {
                return self.overview_update(OMsg::RenameStart(p.card));
            }
            return Task::none();
        }
        if let Some(o) = self.overview.as_mut() {
            o.press = None;
        }
        let moved = cur.distance(p.start) > super::DRAG_THRESHOLD;
        if !moved {
            return match &p.window {
                // Clic sur une fenêtre : au premier plan, et on referme.
                Some(w) => {
                    let close = self.close_overview();
                    Task::batch([focus_window_task(w.address.clone()), close])
                }
                // Clic sur le fond d'une carte : afficher ce workspace.
                None => {
                    let name = self
                        .overview_cards()
                        .get(p.card)
                        .map(|c| c.name.clone())
                        .unwrap_or_default();
                    if name.is_empty() {
                        Task::none()
                    } else {
                        self.overview_update(OMsg::Pick(name))
                    }
                }
            };
        }
        let Some(w) = &p.window else {
            return Task::none();
        };
        // Déposé sur une carte : la fenêtre change d'espace (sans y aller).
        let surface = self
            .overview
            .as_ref()
            .map(|o| self.overview_surface(o))
            .unwrap_or_default();
        let target = self
            .overview_layout(surface)
            .into_iter()
            .find(|(r, _)| r.contains(cur))
            .map(|(_, c)| c);
        let Some(card) = target else {
            return Task::none();
        };
        let same = self
            .ws_geos
            .iter()
            .any(|g| g.address == w.address && g.workspace == card.name);
        if same {
            return Task::none();
        }
        move_task(w.address.clone(), card.name.clone())
    }

    /// Clavier de l'écran : Échap referme (annule d'abord le renommage),
    /// Entrée ouvre l'espace choisi, les flèches déplacent le choix.
    pub(super) fn overview_key(&mut self, key: &keyboard::Key) -> Task<Msg> {
        use keyboard::key::Named;
        // Renommage : Échap annule, la saisie garde le reste (Entrée soumet).
        if self.overview.as_ref().is_some_and(|o| o.renaming.is_some()) {
            if matches!(key, keyboard::Key::Named(Named::Escape))
                && let Some(o) = self.overview.as_mut()
            {
                o.renaming = None;
            }
            return Task::none();
        }
        let selected = match self.overview.as_ref() {
            Some(o) => o.selected,
            None => return Task::none(),
        };
        let cards = self.overview_cards();
        let n = cards.len();
        if n == 0 {
            return Task::none();
        }
        match key {
            keyboard::Key::Named(Named::Escape) => self.close_overview(),
            keyboard::Key::Named(Named::Enter) => {
                let Some(card) = cards.get(selected) else {
                    return Task::none();
                };
                self.overview_update(OMsg::Pick(card.name.clone()))
            }
            keyboard::Key::Named(Named::ArrowDown | Named::Tab) => {
                let cols = (n as f32).sqrt().ceil() as usize;
                self.overview_select((selected + cols).min(n - 1));
                Task::none()
            }
            keyboard::Key::Named(Named::ArrowUp) => {
                let cols = (n as f32).sqrt().ceil() as usize;
                self.overview_select(selected.saturating_sub(cols.max(1)));
                Task::none()
            }
            keyboard::Key::Named(Named::ArrowLeft) => {
                self.overview_select(selected.saturating_sub(1));
                Task::none()
            }
            keyboard::Key::Named(Named::ArrowRight) => {
                self.overview_select((selected + 1).min(n - 1));
                Task::none()
            }
            _ => Task::none(),
        }
    }

    fn overview_select(&mut self, i: usize) {
        if let Some(o) = self.overview.as_mut() {
            o.selected = i;
        }
    }
}

/// Va au workspace (hors du fil de l'interface).
fn focus_task(name: String) -> Task<Msg> {
    Task::perform(
        async move {
            let _ = tokio::task::spawn_blocking(move || crate::hypr::focus_workspace(&name)).await;
        },
        |_| Msg::Noop,
    )
}

/// Met une fenêtre au premier plan.
fn focus_window_task(address: String) -> Task<Msg> {
    Task::perform(
        async move {
            let _ = tokio::task::spawn_blocking(move || crate::hypr::focus_window(&address)).await;
        },
        |_| Msg::Noop,
    )
}

/// Change une fenêtre d'espace sans la suivre.
fn move_task(address: String, workspace: String) -> Task<Msg> {
    Task::perform(
        async move {
            let _ =
                tokio::task::spawn_blocking(move || crate::hypr::move_window(&address, &workspace))
                    .await;
        },
        |_| Msg::Noop,
    )
}

/// Renomme un espace pour la session ; le résultat revient à l'écran
/// (échec silencieux interdit : timeout du socket compris).
fn rename_task(id: i64, name: String) -> Task<Msg> {
    Task::perform(
        async move {
            match tokio::task::spawn_blocking(move || crate::hypr::rename_workspace(id, &name))
                .await
            {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(e.to_string()),
                Err(e) => Err(e.to_string()),
            }
        },
        |r| Msg::Overview(OMsg::RenameDone(r)),
    )
}

impl App {
    pub(super) fn view_overview(&self) -> Element<'_, Msg> {
        let Some(o) = &self.overview else {
            return Space::new().into();
        };
        let th = &self.cfg.general.theme;
        let alpha = o.visibility.value(Instant::now());
        let fg = color(th.fg, alpha);
        let dim = Color {
            a: 0.62 * alpha,
            ..fg
        };
        let accent = color(th.accent, alpha);
        // Encre lisible sur l'accent, à l'alpha du fondu.
        let on_accent = {
            let c = super::design::on_accent(
                color(th.accent, 1.0),
                color(th.fg, 1.0),
                color(th.bg, 1.0),
            );
            Color {
                a: c.a * alpha,
                ..c
            }
        };
        let bg = color(th.bg, 0.66 * alpha);
        let card_bg = color(th.bg, 0.94 * alpha);
        let surface = self.overview_surface(o);
        let cards = self.overview_layout(surface);
        let dragging = o
            .press
            .as_ref()
            .filter(|p| !p.title && p.window.is_some())
            .filter(|p| {
                self.cursor
                    .get(&o.win)
                    .copied()
                    .unwrap_or(p.start)
                    .distance(p.start)
                    > super::DRAG_THRESHOLD
            })
            .cloned();

        // Titre.
        let title = container(
            text(crate::i18n::tr("Espaces de travail"))
                .font(self.font)
                .size(20)
                .color(fg),
        )
        .height(TITLE_H)
        .center_y(TITLE_H)
        .center_x(Length::Fill);

        // Aides et « + Nouvel espace ».
        let hint = |keys: &'static str, label: &'static str| {
            container(
                row![
                    container(text(keys).font(self.font).size(12).color(fg))
                        .padding([3, 8])
                        .style(move |_| container::Style {
                            border: Border::default().rounded(6).width(1).color(Color {
                                a: 0.35 * alpha,
                                ..fg
                            }),
                            ..Default::default()
                        }),
                    text(crate::i18n::tr(label))
                        .font(self.font)
                        .size(12)
                        .color(dim),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
        };
        let accent_soft = color(th.accent, alpha * 0.88);
        let hints = row![
            hint("↑↓←→", "Naviguer"),
            hint("Entrée", "Ouvrir"),
            hint("Échap", "Fermer"),
            Space::new().width(Length::Fill),
            button(
                row![
                    text("+").font(self.font).size(15).color(on_accent),
                    text(crate::i18n::tr("Nouvel espace"))
                        .font(self.font)
                        .size(13)
                        .color(on_accent),
                ]
                .spacing(6)
                .align_y(Alignment::Center)
            )
            .padding([6, 14])
            .on_press(Msg::Overview(OMsg::NewWorkspace))
            .style(move |_, st| button::Style {
                background: Some(
                    if matches!(st, button::Status::Hovered) {
                        accent
                    } else {
                        accent_soft
                    }
                    .into()
                ),
                border: Border::default().rounded(super::design::SMALL_RADIUS),
                text_color: on_accent,
                ..Default::default()
            }),
        ]
        .align_y(Alignment::Center)
        .padding([0.0, MARGIN])
        .height(HINTS_H);

        let content = column![title, Space::new().height(Length::Fill), hints]
            .width(Length::Fill)
            .height(Length::Fill);

        // Fond assombri, les cartes posées aux rectangles du layout partagé
        // (le dépôt d'une fenêtre relit exactement ces rectangles), la
        // fenêtre glissée tout au-dessus.
        let mut layers = stack![container(content).style(move |_| container::Style {
            background: Some(bg.into()),
            ..Default::default()
        })];
        for (i, (rect, card)) in cards.iter().enumerate() {
            let card = self.overview_card(alpha, rect, card, i, o, &dragging, on_accent);
            layers = layers.push(
                container(container(card).width(rect.width).height(rect.height))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(iced::Padding {
                        top: rect.y.max(0.0),
                        left: rect.x.max(0.0),
                        right: (surface.width - rect.x - rect.width).max(0.0),
                        bottom: (surface.height - rect.y - rect.height).max(0.0),
                    }),
            );
        }
        if let Some(p) = &dragging
            && let Some(pos) = self.cursor.get(&o.win)
        {
            let win = p.window.as_ref();
            let img = win
                .and_then(|w| self.class_imgs.get(&w.class))
                .copied()
                .unwrap_or(IconImg::None);
            let label = win.map(|w| truncate(&w.title, 26)).unwrap_or_default();
            let chip = container(
                row![
                    self.img_element(img, 18.0, alpha)
                        .unwrap_or_else(|| Space::new().width(18).height(18).into()),
                    text(label).font(self.font).size(12).color(fg),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([8, 12])
            .style(move |_| container::Style {
                background: Some(card_bg.into()),
                border: Border::default()
                    .rounded(super::design::SMALL_RADIUS)
                    .width(1)
                    .color(accent),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.4 * alpha),
                    offset: Vector::new(0.0, 6.0),
                    blur_radius: 16.0,
                },
                ..Default::default()
            });
            layers = layers.push(
                container(container(chip))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(iced::Padding {
                        top: (pos.y + 14.0).max(0.0),
                        left: (pos.x + 14.0).max(0.0),
                        right: 0.0,
                        bottom: 0.0,
                    }),
            );
        }
        layers.into()
    }

    /// Une carte : titre (double-clic : renommer), badge « Actif », minimap.
    #[allow(clippy::too_many_arguments)]
    fn overview_card(
        &self,
        alpha: f32,
        rect: &Rectangle,
        card: &WsCard,
        index: usize,
        o: &Overview,
        dragging: &Option<Press>,
        on_accent: Color,
    ) -> Element<'_, Msg> {
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, alpha);
        let dim = Color {
            a: 0.6 * alpha,
            ..fg
        };
        let accent = color(th.accent, alpha);
        let card_bg = color(th.bg, 0.94 * alpha);
        let i = index;
        let selected = o.selected == i;
        // Cible du dépôt : la carte sous le curseur pendant un glisser.
        let drop_target = dragging.is_some()
            && self
                .cursor
                .get(&o.win)
                .copied()
                .is_some_and(|p| rect.contains(p));
        let active = card.active;

        // Titre : renommage en cours, ou « Nom » (+ écran si plusieurs).
        let multi = self.monitors.len() > 1;
        let header: Element<'_, Msg> = if o.renaming.as_ref().is_some_and(|(ri, _)| *ri == i) {
            let draft = o
                .renaming
                .as_ref()
                .map(|(_, d)| d.clone())
                .unwrap_or_default();
            // En vol : le champ reste ouvert et s'atténue, la réponse
            // (confirmée ou en erreur) le referme ou l'éclaire.
            let t = if o.renaming_pending {
                alpha * 0.6
            } else {
                alpha
            };
            iced::widget::sensor(
                text_input(crate::i18n::tr("Nom de l'espace"), &draft)
                    .id(RENAME_ID)
                    .on_input(|s| Msg::Overview(OMsg::RenameDraft(s)))
                    .on_submit(Msg::Overview(OMsg::RenameSubmit))
                    .font(self.font)
                    .size(14)
                    .padding(4)
                    .width(Length::Fill)
                    .style(move |theme, status| {
                        let mut s = iced::widget::text_input::default(theme, status);
                        s.background = Background::Color(Color { a: 0.08 * t, ..fg });
                        s.border = Border::default().rounded(6).width(1).color(accent);
                        s
                    }),
            )
            .on_show(|_| Msg::Overview(OMsg::RenameFocus))
            .into()
        } else {
            mouse_area(
                row![
                    text(card.name.clone())
                        .font(self.font)
                        .size(16)
                        .color(if active { accent } else { fg }),
                    (multi && !card.monitor.is_empty()).then(|| {
                        text(card.monitor.clone())
                            .font(self.font)
                            .size(11)
                            .color(dim)
                    }),
                    Space::new().width(Length::Fill),
                    (active).then(|| {
                        container(
                            text(crate::i18n::tr("Actif"))
                                .font(self.font)
                                .size(11)
                                .color(on_accent),
                        )
                        .padding([3, 10])
                        .style(move |_| container::Style {
                            background: Some(accent.into()),
                            border: Border::default().rounded(10),
                            ..Default::default()
                        })
                    }),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .on_press(Msg::Overview(OMsg::Grab(i, true)))
            .into()
        };

        // Minimap : les fenêtres à leur place, saisissables.
        let inner = Self::overview_inner(*rect);
        let wins = self.overview_windows(&card.name);
        let mut minimap = stack![];
        if wins.is_empty() {
            minimap = minimap.push(
                container(
                    text(crate::i18n::tr("Aucune fenêtre"))
                        .font(self.font)
                        .size(12)
                        .color(dim),
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill),
            );
        }
        for (r, w) in &wins {
            let wr = Self::overview_win_rect(inner, r);
            let img = self
                .class_imgs
                .get(&w.class)
                .copied()
                .unwrap_or(IconImg::None);
            let left = (wr.x - inner.x).max(0.0);
            let top = (wr.y - inner.y).max(0.0);
            let right = (inner.width - left - wr.width).max(0.0);
            let bottom = (inner.height - top - wr.height).max(0.0);
            let win_card = container(
                row![
                    self.img_element(img, 13.0, alpha)
                        .unwrap_or_else(|| Space::new().width(13).height(13).into()),
                    text(truncate(&w.title, 22))
                        .font(self.font)
                        .size(10)
                        .color(fg),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            )
            .padding([4, 6])
            .width(wr.width.max(22.0))
            .height(wr.height.max(16.0))
            .style(move |_| container::Style {
                background: Some(
                    Color {
                        a: 0.13 * alpha,
                        ..fg
                    }
                    .into(),
                ),
                border: Border::default().rounded(5).width(1).color(Color {
                    a: 0.22 * alpha,
                    ..fg
                }),
                ..Default::default()
            });
            minimap = minimap.push(
                container(win_card)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(iced::Padding {
                        top,
                        right,
                        bottom,
                        left,
                    }),
            );
        }
        // Saisie dans la minimap : fenêtre (glisser) ou fond (ouvrir l'espace).
        let minimap = mouse_area(container(minimap).width(Length::Fill).height(Length::Fill))
            .on_press(Msg::Overview(OMsg::Grab(i, false)));

        let border = if drop_target {
            Border::default()
                .rounded(super::design::RADIUS)
                .width(2)
                .color(accent)
        } else if active || selected {
            Border::default()
                .rounded(super::design::RADIUS)
                .width(1.5)
                .color(accent)
        } else {
            Border::default()
                .rounded(super::design::RADIUS)
                .width(1)
                .color(Color {
                    a: 0.14 * alpha,
                    ..fg
                })
        };
        container(
            column![
                container(header)
                    .height(40.0)
                    .center_y(40.0)
                    .padding([0, 12]),
                container(minimap)
                    .padding(iced::Padding {
                        top: 0.0,
                        right: 12.0,
                        bottom: 12.0,
                        left: 12.0,
                    })
                    .width(Length::Fill)
                    .height(Length::Fill),
            ]
            .width(Length::Fill)
            .height(Length::Fill),
        )
        .width(rect.width)
        .height(rect.height)
        .style(move |_| container::Style {
            background: Some(card_bg.into()),
            border,
            shadow: if drop_target {
                Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.35 * alpha),
                    offset: Vector::new(0.0, 6.0),
                    blur_radius: 18.0,
                }
            } else {
                Shadow::default()
            },
            text_color: Some(fg),
            ..Default::default()
        })
        .into()
    }
}

/// Titre court : une fenêtre de minimap n'a pas la place d'un long titre.
fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{cut}…")
}

/// Normalise des rectangles globaux en 0..1 : bounding-box du contenu avec
/// une marge de 4 % autour (les fenêtres plein écran y tiennent), une place
/// minimale gardée pour les fenêtres minuscules.
fn normalize_rects(rects: &[(i32, i32, i32, i32)]) -> Vec<(f32, f32, f32, f32)> {
    if rects.is_empty() {
        return Vec::new();
    }
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for &(x, y, w, h) in rects {
        x0 = x0.min(x);
        y0 = y0.min(y);
        x1 = x1.max(x + w);
        y1 = y1.max(y + h);
    }
    let pad_w = (((x1 - x0) as f32 * 0.04).max(8.0)) as i32;
    let pad_h = (((y1 - y0) as f32 * 0.04).max(8.0)) as i32;
    let (bx, by, bw, bh) = (
        x0 - pad_w,
        y0 - pad_h,
        x1 - x0 + 2 * pad_w,
        y1 - y0 + 2 * pad_h,
    );
    rects
        .iter()
        .map(|&(x, y, w, h)| {
            let nx = |v: i32| ((v - bx) as f32 / bw.max(1) as f32).clamp(0.0, 1.0);
            let ny = |v: i32| ((v - by) as f32 / bh.max(1) as f32).clamp(0.0, 1.0);
            (
                nx(x),
                ny(y),
                (nx(x + w) - nx(x)).max(0.05),
                (ny(y + h) - ny(y)).max(0.05),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WsSelector;

    #[test]
    fn minimap_normalise_le_contenu() {
        // Deux fenêtres en mosaïque : côte à côte, toute la hauteur.
        let r = normalize_rects(&[(0, 44, 720, 856), (720, 44, 720, 856)]);
        assert_eq!(r.len(), 2);
        assert!((r[0].0 - r[1].0).abs() > 0.3, "côte à côte");
        assert!(r[0].3 > 0.9 && r[1].3 > 0.9, "toute la hauteur");
        // La bounding-box : contenu aux bornes 0..1, marge autour.
        let r = normalize_rects(&[(100, 100, 200, 100)]);
        assert_eq!(r.len(), 1);
        assert!(r[0].0 > 0.0 && r[0].1 > 0.0, "marge");
        assert!((r[0].0 + r[0].2) < 1.0 && (r[0].1 + r[0].3) < 1.0, "marge");
        assert!(normalize_rects(&[]).is_empty());
    }

    #[test]
    fn le_selecteur_va_et_vient() {
        for (s, v) in [
            ("relief", WsSelector::Relief),
            ("rail", WsSelector::Rail),
            ("miniatures", WsSelector::Miniatures),
            ("icones", WsSelector::Icones),
        ] {
            assert_eq!(WsSelector::parse(s), Some(v));
            assert_eq!(v.as_str(), s);
        }
        assert_eq!(WsSelector::parse("inconnu"), None);
    }
}
