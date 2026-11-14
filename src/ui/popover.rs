//! Menus déroulants de la barre, à la manière de macOS : une carte arrondie,
//! translucide et floutée, ombre portée, qui descend sous le module cliqué.
//!
//! Deux surfaces : la carte (calque `overlay`, clavier exclusif pour Échap)
//! et un « capteur » transparent plein écran juste dessous, qui ferme le menu
//! au premier clic ailleurs.

use crate::i18n::text;
use iced::widget::{Space, button, column, container, mouse_area, row, rule, slider, toggler};
use iced::window::Id;
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Size, Task, Vector};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::{App, Fade, Msg, color, format_now};
use crate::config::{BarPosition, MenuEntry};
mod bar_menus;

pub const POPOVER_NAMESPACE: &str = "hyprdesk-popover";
/// Marge transparente autour de la carte, pour son ombre.
pub const SHADOW: f32 = 18.0;
const WIDTH: f32 = 328.0;
/// Côté d'une image de menu (pochette).
const IMAGE: f32 = 120.0;
/// Écart entre la barre et le haut de la carte.
pub(super) const GAP: f32 = 10.0;

/// Ce qu'on demande d'ouvrir (le contenu peut devoir être calculé avant).
#[derive(Debug, Clone)]
pub enum Request {
    Volume,
    Brightness,
    Calendar,
    Widget(String),
    Workspaces,
    Overflow,
    /// Menu d'un widget Lua : sa fonction `menu`, et l'id du widget à rafraîchir après une action.
    LuaMenu {
        widget: String,
        func: usize,
    },
    /// Menu dbusmenu d'une icône de la zone de notification.
    Tray {
        bus: String,
        menu_path: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::{Point, Rectangle};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    fn app() -> (App, Id) {
        let (mut app, _) = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-bar-tests/config.lua"),
            true,
        );
        let bar = Id::unique();
        app.monitors = vec![crate::hypr::Monitor {
            name: "A".into(),
            width: 900,
            height: 640,
            x: 0.0,
            y: 0.0,
            scale: 1.0,
            workspace: "1".into(),
            base: String::new(),
            focused: true,
            reserved: (0.0, 44.0, 0.0, 0.0),
        }];
        app.bar_surfaces.insert(
            bar,
            super::super::bar::BarSurface {
                monitor: "A".into(),
                position: BarPosition::Top,
                height: 44,
                margin: 0,
            },
        );
        Arc::make_mut(&mut app.cfg).general.transition = crate::config::Transition::Fade;
        (app, bar)
    }

    #[test]
    fn cards_are_bounded_and_anchored_without_a_pointer() {
        for width in [320.0, 640.0, 900.0, 1440.0] {
            for x in [0.0, width / 2.0, width - 1.0] {
                let (left, w, origin) = placement(
                    width,
                    Rectangle::new(Point::new(x, 0.0), Size::new(1.0, 44.0)),
                );
                assert!(left + SHADOW >= 12.0 - 0.01);
                assert!(left + w - SHADOW <= width - 12.0 + 0.01);
                assert!((0.05..=0.95).contains(&origin));
            }
        }
        let (mut app, bar) = app();
        app.bar_anchors.insert(
            (bar, "volume".into()),
            Rectangle::new(Point::new(820.0, 0.0), Size::new(30.0, 44.0)),
        );
        let _ = app.open_popover(bar, "volume".into(), Kind::Volume);
        assert!(app.popover.as_ref().unwrap().left > 500.0);
        assert!(app.cursor.is_empty());
        app.bar_anchors.insert(
            (bar, "controls".into()),
            Rectangle::new(Point::new(850.0, 0.0), Size::new(30.0, 44.0)),
        );
        app.bar_anchors.insert(
            (bar, "overflow".into()),
            Rectangle::new(Point::new(20.0, 0.0), Size::new(30.0, 44.0)),
        );
        let _ = app.open_popover(bar, "controls".into(), Kind::Widget("controls".into()));
        let _ = app.open_popover(
            bar,
            "bar:controls:reseau".into(),
            Kind::Widget("controls".into()),
        );
        assert!(
            app.popover.as_ref().unwrap().left > 500.0,
            "une ancienne ancre de débordement ne déplace pas les sous-menus du centre de contrôle"
        );
    }

    #[test]
    fn close_can_reverse_and_switches_reuse_both_surfaces() {
        let (mut app, bar) = app();
        let _ = app.open_popover(bar, "volume".into(), Kind::Volume);
        let (win, catcher) = {
            let p = app.popover.as_mut().unwrap();
            p.visibility.from = 1.0;
            p.visibility.to = 1.0;
            (p.win, p.catcher)
        };
        let _ = app.close_popover();
        assert!(app.popover.as_ref().unwrap().closing());
        let _ = app.open_popover(bar, "volume".into(), Kind::Volume);
        assert!(!app.popover.as_ref().unwrap().closing());
        assert_eq!(app.popover.as_ref().unwrap().win, win);
        let _ = app.open_popover(bar, "controls".into(), Kind::Widget("controls".into()));
        let p = app.popover.as_ref().unwrap();
        assert_eq!((p.win, p.catcher), (win, catcher));
        assert!(matches!(p.previous, Some(Kind::Volume)));
        app.popover.as_mut().unwrap().change.as_mut().unwrap().start =
            Instant::now() - Duration::from_secs(1);
        app.popover.as_mut().unwrap().visibility.start = Instant::now() - Duration::from_secs(1);
        let _ = app.popover_frame();
        assert!(app.popover.as_ref().unwrap().previous.is_none());
        assert!(!app.popover.as_ref().unwrap().animating(Instant::now()));
        let _ = app.close_popover();
        app.popover.as_mut().unwrap().visibility.start = Instant::now() - Duration::from_secs(1);
        let _ = app.popover_frame();
        assert!(app.popover.is_none());
    }

    #[test]
    fn final_frame_is_committed_even_after_a_slow_first_present() {
        let (mut app, bar) = app();
        let _ = app.open_popover(bar, "controls".into(), Kind::Widget("controls".into()));
        app.popover.as_mut().unwrap().visibility.start = Instant::now() - Duration::from_secs(1);
        assert!(app.popover.as_ref().unwrap().animating(Instant::now()));
        let _ = app.popover_frame();
        assert!(!app.popover.as_ref().unwrap().animating(Instant::now()));
        Arc::make_mut(&mut app.cfg).general.transition = crate::config::Transition::None;
        let _ = app.close_popover();
        assert!(app.popover.is_none());
    }

    #[test]
    fn opening_before_the_first_layout_follows_the_late_anchor() {
        let (mut app, bar) = app();
        let _ = app.open_popover(bar, "controls".into(), Kind::Widget("controls".into()));
        let win = app.popover.as_ref().unwrap().win;
        let _ = app.update_inner(Msg::BarAnchor(
            bar,
            "controls".into(),
            Rectangle::new(Point::new(850.0, 0.0), Size::new(30.0, 44.0)),
        ));
        assert_eq!(app.popover.as_ref().unwrap().win, win);
        assert!(app.popover.as_ref().unwrap().left > 500.0);
    }
}

#[derive(Debug, Clone)]
pub enum MenuSource {
    Lua { widget: String },
    Tray { bus: String, menu_path: String },
}

#[derive(Debug, Clone)]
pub enum Kind {
    Volume,
    Brightness,
    Calendar,
    Widget(String),
    Workspaces,
    Overflow(Vec<crate::config::BarItem>),
    Menu {
        entries: Vec<MenuEntry>,
        source: MenuSource,
    },
}

#[derive(Debug, Clone)]
pub enum PMsg {
    /// Clic sur l'entrée n du menu.
    Activate(usize),
    /// Confirmation d'une entrée « confirm ».
    Confirm(usize),
    CancelConfirm,
    Spawn(String),
    /// Saisie dans l'entrée n (champ `input`).
    Input(usize, String),
    /// Entrée validée : l'action reçoit le texte.
    Submit(usize),
}

pub struct Popover {
    pub win: Id,
    pub catcher: Id,
    pub bar: Id,
    pub position: BarPosition,
    pub left: f32,
    pub origin: f32,
    pub visibility: Fade,
    pub change: Option<Fade>,
    pub previous: Option<Kind>,
    pub slide_from: f32,
    /// Module d'origine : recliquer dessus referme.
    pub key: String,
    anchor_key: String,
    pub kind: Kind,
    pub size: Size,
    /// Dernière relecture du menu (menus du tray : suivis ouverts).
    pub refreshed: std::time::Instant,
    max_height: f32,
    /// Entrée en attente de confirmation.
    pub confirm: Option<usize>,
    /// Texte des champs de saisie, par entrée.
    pub inputs: std::collections::HashMap<usize, String>,
}

impl Popover {
    pub(super) fn closing(&self) -> bool {
        self.visibility.to == 0.0
    }
    pub(super) fn animating(&self, now: std::time::Instant) -> bool {
        !self.visibility.done(now)
            || self.visibility.from != self.visibility.to
            || self.change.is_some()
    }
}

impl Drop for Popover {
    fn drop(&mut self) {
        for input in self.inputs.values_mut() {
            crate::lock::pam::wipe(input);
        }
    }
}

/// Carte ancrée à la boîte rendue du module, même à l'ouverture au clavier.
pub(super) fn placement(screen_width: f32, anchor: iced::Rectangle) -> (f32, f32, f32) {
    let width = WIDTH.min((screen_width - 24.0).max(80.0));
    let card_left =
        (anchor.center_x() - width / 2.0).clamp(12.0, (screen_width - width - 12.0).max(12.0));
    let left = card_left - SHADOW;
    let origin = ((anchor.center_x() - left) / (width + 2.0 * SHADOW)).clamp(0.05, 0.95);
    (left, width + 2.0 * SHADOW, origin)
}

/// Détail sous le libellé (deux lignes) plutôt qu'aligné à droite, quand
/// l'ensemble ne tiendrait pas sur une ligne.
fn stacked(label: &str, detail: Option<&str>) -> bool {
    detail.is_some_and(|d| d.chars().count() > 22 || label.chars().count() + d.chars().count() > 32)
}

/// Hauteur de la carte pour un contenu donné (les surfaces layer-shell ont une taille fixe).
/// Hauteur d'une section du menu Son : titre, curseur, muet.
const AUDIO_SECTION: f32 = 26.0 + 32.0 + 30.0;

/// Périphériques listés : seulement s'il y a un choix à faire.
fn device_rows(list: &[super::bar::AudioDevice]) -> f32 {
    if list.len() > 1 {
        list.len() as f32 * 30.0
    } else {
        0.0
    }
}

fn card_height(
    kind: &Kind,
    confirm: Option<usize>,
    audio: &super::bar::Audio,
    light: &super::bar::Light,
) -> f32 {
    let inner = match kind {
        Kind::Widget(_) => 420.0,
        Kind::Workspaces => 300.0,
        Kind::Overflow(items) => 44.0 + items.len() as f32 * 48.0,
        // Sortie, séparateur, micro (ou « aucun »), séparateur, réglages.
        Kind::Volume => {
            let mic = if audio.mic.is_some() {
                AUDIO_SECTION + device_rows(&audio.inputs)
            } else {
                26.0 + 30.0
            };
            AUDIO_SECTION + device_rows(&audio.outputs) + 11.0 + mic + 11.0 + 30.0 + 16.0
        }
        // Un curseur (titre + ligne) par rétroéclairage ou écran.
        Kind::Brightness => {
            let n = light.screen.is_some() as usize
                + light.kbd.is_some() as usize
                + light.displays.len();
            n.max(1) as f32 * (26.0 + 32.0) + 16.0
        }
        // Six lignes au plus : on change de mois sans changer la taille de la carte.
        Kind::Calendar => 112.0 + 28.0 + 6.0 * 23.0,
        Kind::Menu { entries, .. } => {
            let rows: f32 = entries
                .iter()
                .enumerate()
                .map(|(i, e)| match e {
                    MenuEntry::Header(_) => 26.0,
                    MenuEntry::Image(_) => IMAGE + 12.0,
                    MenuEntry::Separator => 11.0,
                    MenuEntry::Input { label, .. } => {
                        44.0 + if label.is_some() { 20.0 } else { 0.0 }
                    }
                    MenuEntry::Item { label, detail, .. } => {
                        let base = if stacked(label, detail.as_deref()) {
                            44.0
                        } else {
                            30.0
                        };
                        base + if confirm == Some(i) { 40.0 } else { 0.0 }
                    }
                })
                .sum();
            rows.max(30.0)
        }
    };
    inner + 12.0
}

impl App {
    /// L'ouverture peut précéder le premier relevé des limites (démarrage,
    /// accès clavier, aperçu isolé). Recaler la carte dès que son ancre existe.
    pub(super) fn refresh_popover_anchor(&mut self) -> Task<Msg> {
        let Some(p) = self.popover.as_ref().filter(|p| !p.closing()) else {
            return Task::none();
        };
        let Some(bar) = self.bar_surfaces.get(&p.bar) else {
            return Task::none();
        };
        let Some(m) = self.monitors.iter().find(|m| m.name == bar.monitor) else {
            return Task::none();
        };
        let hidden = self.bar_overflow_items(p.bar);
        let overflowed = hidden.iter().any(|item| {
            super::bar::bar_item_key(item) == p.key
                || (matches!(item, crate::config::BarItem::Tray) && p.key.starts_with("tray:"))
        });
        let key = if overflowed {
            "overflow"
        } else {
            p.anchor_key.as_str()
        };
        let Some(anchor) = self.bar_anchors.get(&(p.bar, key.into())).copied() else {
            return Task::none();
        };
        let (left, width, origin) = placement(m.width as f32, anchor);
        let p = self.popover.as_mut().unwrap();
        p.origin = origin;
        let mut tasks = Vec::new();
        if (p.left - left).abs() > 0.5 {
            if p.change.is_some() {
                p.slide_from = (p.slide_from + p.left - left).clamp(-90.0, 90.0);
            }
            p.left = left;
            let v = (GAP - SHADOW) as i32;
            tasks.push(Task::done(Msg::MarginChange {
                id: p.win,
                margin: if p.position == BarPosition::Top {
                    (v, 0, 0, left as i32)
                } else {
                    (0, 0, v, left as i32)
                },
            }));
        }
        if (p.size.width - width).abs() > 0.5 {
            p.size.width = width;
            tasks.push(Task::done(Msg::SizeChange {
                id: p.win,
                size: (width as u32, p.size.height as u32),
            }));
        }
        Task::batch(tasks)
    }

    /// Ouvre un menu sous le module cliqué de la barre `bar` (ou le referme).
    pub(super) fn open_popover(&mut self, bar: Id, key: String, kind: Kind) -> Task<Msg> {
        let now = std::time::Instant::now();
        let Some(b) = self.bar_surfaces.get(&bar).cloned() else {
            return Task::none();
        };
        let Some(m) = self.monitors.iter().find(|m| m.name == b.monitor).cloned() else {
            return Task::none();
        };
        let hidden = self.bar_overflow_items(bar);
        let overflowed = hidden.iter().any(|item| {
            super::bar::bar_item_key(item) == key
                || (matches!(item, crate::config::BarItem::Tray) && key.starts_with("tray:"))
        });
        let anchor_key = if overflowed {
            "overflow".to_string()
        } else if self.bar_anchors.contains_key(&(bar, key.clone())) {
            key.clone()
        } else {
            self.popover
                .as_ref()
                .filter(|p| p.bar == bar)
                .map_or_else(|| key.clone(), |p| p.anchor_key.clone())
        };
        let anchor = self
            .bar_anchors
            .get(&(bar, anchor_key.clone()))
            .copied()
            .unwrap_or_else(|| {
                iced::Rectangle::new(
                    iced::Point::new(m.width as f32 / 2.0, 0.0),
                    Size::new(1.0, b.height as f32),
                )
            });
        let (left, width, origin) = placement(m.width as f32, anchor);
        let max_height = (m.height as f32 - b.height as f32 - 2.0 * GAP).max(80.0);
        let h = (card_height(&kind, None, &self.audio, &self.light) + 2.0 * SHADOW).min(max_height);
        let size = Size::new(width, h);
        let duration = self.bar_duration(180);
        let change_duration = self.bar_duration(160);
        let v = (GAP - SHADOW) as i32;
        let margin = if b.position == BarPosition::Top {
            (v, 0, 0, left as i32)
        } else {
            (0, 0, v, left as i32)
        };
        let mut tasks = Vec::new();
        if let Kind::Menu { entries, .. } = &kind {
            for e in entries {
                if let MenuEntry::Item { icon: Some(i), .. } = e {
                    tasks.push(self.want_icon(i));
                }
            }
        }
        if let Some(p) = self
            .popover
            .as_mut()
            .filter(|p| p.bar == bar && p.position == b.position)
        {
            let reversing = p.closing() && p.key == key;
            p.visibility = Fade {
                from: p.visibility.value(now),
                to: 1.0,
                start: now,
                dur: duration,
            };
            if !reversing {
                p.previous = Some(std::mem::replace(&mut p.kind, kind));
                p.change = Some(Fade {
                    from: 0.0,
                    to: 1.0,
                    start: now,
                    dur: change_duration,
                });
                p.slide_from = (p.left - left).clamp(-90.0, 90.0);
            }
            p.key = key;
            p.anchor_key = anchor_key;
            p.left = left;
            p.origin = origin;
            p.size = size;
            p.max_height = max_height;
            p.confirm = None;
            for input in p.inputs.values_mut() {
                crate::lock::pam::wipe(input);
            }
            p.inputs.clear();
            tasks.push(Task::done(Msg::MarginChange { id: p.win, margin }));
            tasks.push(Task::done(Msg::SizeChange {
                id: p.win,
                size: (size.width as u32, size.height as u32),
            }));
            return Task::batch(tasks);
        }
        tasks.push(self.destroy_popover());
        let (catcher, win) = (Id::unique(), Id::unique());
        // Le capteur d'abord : la carte, créée ensuite et sur le calque overlay, passe au-dessus.
        tasks.push(Task::done(Msg::NewLayerShell {
            settings: NewLayerShellSettings {
                size: Some((0, 0)),
                layer: Layer::Top,
                anchor: LsAnchor::Top | LsAnchor::Bottom | LsAnchor::Left | LsAnchor::Right,
                exclusive_zone: Some(-1),
                keyboard_interactivity: KeyboardInteractivity::None,
                // La barre garde ses événements pour passer directement d'un
                // menu à l'autre, sans traverser le capteur plein écran.
                margin: Some(if b.position == BarPosition::Top {
                    (b.height as i32, 0, 0, 0)
                } else {
                    (0, 0, b.height as i32, 0)
                }),
                output_option: OutputOption::OutputName(m.name.clone()),
                namespace: Some("hyprdesk-catcher".into()),
                ..Default::default()
            },
            id: catcher,
        }));
        let edge = if b.position == BarPosition::Top {
            LsAnchor::Top
        } else {
            LsAnchor::Bottom
        };
        tasks.push(Task::done(Msg::NewLayerShell {
            settings: NewLayerShellSettings {
                size: Some((size.width as u32, size.height as u32)),
                layer: Layer::Overlay,
                anchor: edge | LsAnchor::Left,
                exclusive_zone: Some(0),
                margin: Some(margin),
                keyboard_interactivity: KeyboardInteractivity::Exclusive,
                output_option: OutputOption::OutputName(m.name.clone()),
                namespace: Some(POPOVER_NAMESPACE.into()),
                ..Default::default()
            },
            id: win,
        }));
        if let Kind::Menu { entries, .. } = &kind {
            for e in entries {
                if let MenuEntry::Item { icon: Some(i), .. } = e {
                    tasks.push(self.want_icon(i));
                }
            }
        }
        tasks.push(self.want_icon("object-select-symbolic"));
        self.popover = Some(Popover {
            win,
            catcher,
            bar,
            position: b.position,
            left,
            origin,
            visibility: Fade {
                from: 0.0,
                to: 1.0,
                start: now,
                dur: duration,
            },
            change: None,
            previous: None,
            slide_from: 0.0,
            key,
            anchor_key,
            kind,
            size,
            refreshed: std::time::Instant::now(),
            max_height,
            confirm: None,
            inputs: Default::default(),
        });
        Task::batch(tasks)
    }

    /// Champ de saisie d'un menu (`{ input = "…", secret = true, action = … }`).
    fn pop_input<'a>(
        &'a self,
        alpha: f32,
        i: usize,
        label: Option<&'a str>,
        placeholder: &'a str,
        secret: bool,
    ) -> Element<'a, Msg> {
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, alpha);
        let value = self
            .popover
            .as_ref()
            .and_then(|p| p.inputs.get(&i))
            .map(String::as_str)
            .unwrap_or("");
        let mut col = column![].spacing(4).padding([4, 10]);
        if let Some(l) = label {
            col = col.push(text(l.to_string()).font(self.font).size(12).color(Color {
                a: 0.7 * alpha,
                ..fg
            }));
        }
        col.push(
            iced::widget::text_input(placeholder, value)
                .secure(secret)
                .on_input(move |s| Msg::Popover(PMsg::Input(i, s)))
                .on_submit(Msg::Popover(PMsg::Submit(i)))
                .font(self.font)
                .size(13)
                .padding(8)
                .style(move |theme, status| {
                    let mut style = iced::widget::text_input::default(theme, status);
                    style.background = style.background.scale_alpha(alpha);
                    style.border.color.a *= alpha;
                    style.icon.a *= alpha;
                    style.placeholder.a *= alpha;
                    style.value.a *= alpha;
                    style.selection.a *= alpha;
                    style
                }),
        )
        .into()
    }

    pub(super) fn destroy_popover(&mut self) -> Task<Msg> {
        self.cal_months = 0;
        match self.popover.take() {
            Some(p) => Task::batch([iced::window::close(p.win), iced::window::close(p.catcher)]),
            None => Task::none(),
        }
    }

    pub(super) fn close_popover(&mut self) -> Task<Msg> {
        let duration = self.bar_duration(120);
        if duration.is_zero() {
            return self.destroy_popover();
        }
        if let Some(p) = self.popover.as_mut()
            && !p.closing()
        {
            let now = std::time::Instant::now();
            p.visibility = Fade {
                from: p.visibility.value(now),
                to: 0.0,
                start: now,
                dur: duration,
            };
        }
        Task::none()
    }

    pub(super) fn popover_frame(&mut self) -> Task<Msg> {
        let now = std::time::Instant::now();
        let still = self.bar_duration(180).is_zero();
        if self
            .popover
            .as_ref()
            .is_some_and(|p| p.closing() && (still || p.visibility.done(now)))
        {
            return self.destroy_popover();
        }
        if let Some(p) = self.popover.as_mut() {
            if p.change.is_some_and(|f| still || f.done(now)) {
                p.previous = None;
                p.change = None;
            }
            if still || p.visibility.done(now) {
                p.visibility.from = p.visibility.to;
                p.visibility.dur = std::time::Duration::ZERO;
            }
        }
        Task::none()
    }

    pub(super) fn popover_update(&mut self, m: PMsg) -> Task<Msg> {
        let Some(p) = self.popover.as_mut() else {
            return Task::none();
        };
        match m {
            PMsg::Spawn(cmd) => {
                let t = self.run_from_desktop(cmd, false);
                Task::batch([t, self.close_popover()])
            }
            PMsg::Input(i, s) => {
                let old = p.inputs.insert(i, s);
                if let Some(mut old) = old {
                    crate::lock::pam::wipe(&mut old);
                }
                Task::none()
            }
            PMsg::Submit(i) => {
                let Kind::Menu { entries, source } = &p.kind else {
                    return Task::none();
                };
                let Some(MenuEntry::Input {
                    action: Some(action),
                    ..
                }) = entries.get(i)
                else {
                    return Task::none();
                };
                let (action, source) = (*action, source.clone());
                // Le texte part vers le Lua ; il n'en reste rien ici.
                let text = p.inputs.remove(&i).unwrap_or_default();
                let t = match source {
                    MenuSource::Lua { widget } => {
                        let lua = self.lua.clone();
                        Task::perform(
                            async move {
                                if let Err(e) = lua.menu_action(action as usize, Some(text)).await {
                                    tracing::warn!("{}", crate::tr!("menu : {e}", e = e));
                                }
                                widget
                            },
                            Msg::Repoll,
                        )
                    }
                    MenuSource::Tray { .. } => Task::none(),
                };
                Task::batch([t, self.close_popover()])
            }
            PMsg::CancelConfirm => {
                p.confirm = None;
                self.resize_popover()
            }
            PMsg::Activate(i) => {
                let Kind::Menu { entries, .. } = &p.kind else {
                    return Task::none();
                };
                let Some(MenuEntry::Item {
                    enabled: true,
                    confirm,
                    ..
                }) = entries.get(i)
                else {
                    return Task::none();
                };
                if confirm.is_some() && p.confirm != Some(i) {
                    p.confirm = Some(i);
                    return self.resize_popover();
                }
                self.popover_update(PMsg::Confirm(i))
            }
            PMsg::Confirm(i) => {
                let Kind::Menu { entries, source } = &p.kind else {
                    return Task::none();
                };
                let Some(MenuEntry::Item {
                    action: Some(action),
                    ..
                }) = entries.get(i)
                else {
                    return self.close_popover();
                };
                let action = *action;
                let t = match source.clone() {
                    MenuSource::Lua { widget } => {
                        let lua = self.lua.clone();
                        Task::perform(
                            async move {
                                if let Err(e) = lua.menu_action(action as usize, None).await {
                                    tracing::warn!("{}", crate::tr!("menu : {e}", e = e));
                                }
                                widget
                            },
                            Msg::Repoll,
                        )
                    }
                    MenuSource::Tray { bus, menu_path } => Task::perform(
                        async move { crate::tray::menu_event(&bus, &menu_path, action as i32).await },
                        |_| Msg::Noop,
                    ),
                };
                Task::batch([t, self.close_popover()])
            }
        }
    }

    fn resize_popover(&mut self) -> Task<Msg> {
        // Le prochain rendu mesure le contenu, y compris les confirmations et les retours à la ligne.
        Task::none()
    }

    pub(super) fn fit_popover(&mut self, win: Id, content_height: f32) -> Task<Msg> {
        if !content_height.is_finite() || content_height <= 0.0 {
            return Task::none();
        }
        let Some(p) = self.popover.as_mut().filter(|p| p.win == win) else {
            return Task::none();
        };
        let height = (content_height + 24.0 + 2.0 * SHADOW)
            .ceil()
            .min(p.max_height);
        if (height - p.size.height).abs() < 1.0 {
            return Task::none();
        }
        p.size.height = height;
        Task::done(Msg::SizeChange {
            id: win,
            size: (p.size.width as u32, height as u32),
        })
    }

    pub(super) fn view_catcher(&self) -> Element<'_, Msg> {
        mouse_area(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(Msg::ClosePopover)
        .on_right_press(Msg::ClosePopover)
        .into()
    }

    fn pop_content<'a>(
        &'a self,
        kind: &'a Kind,
        confirm: Option<usize>,
        alpha: f32,
    ) -> Element<'a, Msg> {
        let content = match kind {
            Kind::Volume => self.pop_volume(alpha),
            Kind::Brightness => self.pop_brightness(alpha),
            Kind::Calendar => self.pop_calendar(alpha),
            Kind::Widget(id) => match self
                .cfg
                .widgets
                .iter()
                .find(|w| &w.id == id)
                .map(|w| &w.kind)
            {
                Some(crate::config::WidgetKind::Controls) => {
                    let bar = self.popover.as_ref().map(|p| p.bar);
                    let monitor = bar
                        .and_then(|id| self.bar_surfaces.get(&id))
                        .map_or("", |b| b.monitor.as_str());
                    self.controls_view(alpha, monitor, bar)
                }
                _ => Space::new().into(),
            },
            Kind::Workspaces => self.pop_workspaces(alpha),
            Kind::Overflow(items) => self.pop_overflow(alpha, items),
            Kind::Menu { entries, .. } => self.pop_menu(alpha, entries, confirm),
        };
        let fg = color(self.cfg.general.theme.fg, alpha);
        container(content)
            .style(move |_| container::Style {
                text_color: Some(fg),
                ..Default::default()
            })
            .into()
    }

    pub(super) fn view_popover(&self) -> Element<'_, Msg> {
        let Some(p) = &self.popover else {
            return Space::new().into();
        };
        let now = std::time::Instant::now();
        let still = self.bar_duration(180).is_zero();
        let alpha = if still {
            p.visibility.to
        } else {
            p.visibility.value(now)
        };
        let mix = if still {
            1.0
        } else {
            p.change.map_or(1.0, |f| f.value(now))
        };
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, alpha);
        let bg = color(
            th.bg,
            th.effect(crate::config::Place::Menus).opacity * alpha,
        );
        let win = p.win;
        let current = iced::widget::sensor(self.pop_content(&p.kind, p.confirm, alpha * mix))
            .key(win)
            .on_resize(move |size| Msg::PopoverMeasured(win, size.height));
        let mut content = iced::widget::stack![];
        if let Some(previous) = &p.previous {
            content = content.push(super::bar_motion::inert(self.pop_content(
                previous,
                None,
                alpha * (1.0 - mix),
            )));
        }
        content = content.push(current);
        let card: Element<'_, Msg> =
            container(iced::widget::scrollable(content).height(Length::Fill))
                .padding(12)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| {
                    let mut style = super::design::bar_card(bg, fg);
                    style.border.color.a *= alpha;
                    style.shadow.color.a *= alpha;
                    style
                })
                .into();
        let card = if p.closing() {
            super::bar_motion::inert(card)
        } else {
            card
        };
        let direction = if p.position == BarPosition::Top {
            -1.0
        } else {
            1.0
        };
        let origin = if p.position == BarPosition::Top {
            iced::Point::new(p.origin, 0.0)
        } else {
            iced::Point::new(p.origin, 1.0)
        };
        container(super::bar_motion::surface(
            card,
            0.975 + 0.025 * alpha,
            Vector::new(p.slide_from * (1.0 - mix), direction * 6.0 * (1.0 - alpha)),
            origin,
        ))
        .padding(SHADOW)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    pub(super) fn pop_header(&self, alpha: f32, label: String) -> Element<'_, Msg> {
        let dim = color(self.cfg.general.theme.fg, 0.65 * alpha);
        container(
            text(label)
                .font(self.font)
                .size(11.5)
                .color(dim)
                .font(self.ui_font(iced::font::Weight::Semibold)),
        )
        .padding(Padding {
            top: 6.0,
            right: 10.0,
            bottom: 4.0,
            left: 10.0,
        })
        .height(26)
        .into()
    }

    pub(super) fn pop_separator(&self, alpha: f32) -> Element<'_, Msg> {
        let line = color(self.cfg.general.theme.fg, 0.10 * alpha);
        container(rule::horizontal(1).style(move |_| rule::Style {
            color: line,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: true,
        }))
        .padding([5, 10])
        .height(11)
        .into()
    }

    /// Ligne de menu à la macOS : surlignage plein en couleur d'accent au survol.
    /// `enabled` à faux : entrée grisée ; sans action (`clickable` faux) : ligne
    /// d'information, lisible mais sans survol.
    #[allow(clippy::too_many_arguments)]
    fn pop_row<'a>(
        &'a self,
        alpha: f32,
        lead: Element<'a, Msg>,
        label: String,
        detail: Option<String>,
        enabled: bool,
        clickable: bool,
        msg: Option<Msg>,
    ) -> Element<'a, Msg> {
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0 * alpha), color(th.accent, 1.0 * alpha));
        let dim = color(th.fg, (if enabled { 0.65 } else { 0.3 }) * alpha);
        let font = self.ui_font(iced::font::Weight::Normal);
        let long = stacked(&label, detail.as_deref());
        let active = enabled && clickable;
        let label_el = text(label)
            .font(self.font)
            .size(13.5)
            .font(font)
            .wrapping(iced::widget::text::Wrapping::None);
        let body: Element<'_, Msg> = match detail {
            Some(d) if long => column![
                label_el,
                text(d)
                    .font(self.font)
                    .size(11.5)
                    .color(dim)
                    .font(font)
                    .wrapping(iced::widget::text::Wrapping::None)
            ]
            .spacing(1)
            .into(),
            Some(d) => row![
                label_el.width(Length::Fill),
                text(d).font(self.font).size(12).color(dim).font(font)
            ]
            .align_y(Alignment::Center)
            .into(),
            None => label_el.into(),
        };
        button(
            row![container(lead).width(18).align_x(Alignment::Center), body]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .height(if long { 44 } else { 30 })
        .padding([4, 10])
        .on_press_maybe(if active { msg } else { None })
        .style(move |_, st| {
            let hot = active && matches!(st, button::Status::Hovered | button::Status::Pressed);
            button::Style {
                background: hot.then_some(Background::Color(Color {
                    a: 0.12 * alpha,
                    ..accent
                })),
                text_color: if !enabled {
                    Color {
                        a: 0.35 * alpha,
                        ..fg
                    }
                } else {
                    fg
                },
                border: Border::default().rounded(super::design::SMALL_RADIUS),
                ..Default::default()
            }
        })
        .into()
    }

    fn pop_menu<'a>(
        &'a self,
        alpha: f32,
        entries: &'a [MenuEntry],
        confirm: Option<usize>,
    ) -> Element<'a, Msg> {
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0 * alpha);
        let mut col = column![].spacing(0);
        if entries.is_empty() {
            col = col.push(self.pop_row(
                alpha,
                Space::new().into(),
                crate::i18n::tr("Rien à afficher").into(),
                None,
                false,
                false,
                None,
            ));
        }
        for (i, e) in entries.iter().enumerate() {
            col = col.push(match e {
                MenuEntry::Header(h) => self.pop_header(alpha, h.clone()),
                MenuEntry::Image(p) => container(
                    iced::widget::image(iced::widget::image::Handle::from_path(p))
                        .width(IMAGE)
                        .height(IMAGE)
                        .opacity(alpha)
                        .content_fit(iced::ContentFit::Cover)
                        .border_radius(10),
                )
                .center_x(Length::Fill)
                .padding([6, 0])
                .into(),
                MenuEntry::Separator => self.pop_separator(alpha),
                MenuEntry::Input {
                    label,
                    placeholder,
                    secret,
                    ..
                } => self.pop_input(alpha, i, label.as_deref(), placeholder, *secret),
                MenuEntry::Item {
                    label,
                    detail,
                    icon,
                    checked,
                    enabled,
                    action,
                    confirm: ask,
                } => {
                    let lead: Element<'_, Msg> = match (checked, icon) {
                        (Some(true), _) => self
                            .icon_named("object-select-symbolic", 14.0, fg)
                            .unwrap_or_else(|| text("•").font(self.font).size(13).into()),
                        (_, Some(name)) => self
                            .icon_named(name, 16.0, fg)
                            .unwrap_or_else(|| Space::new().into()),
                        _ => Space::new().into(),
                    };
                    let row_el = self.pop_row(
                        alpha,
                        lead,
                        label.clone(),
                        detail.clone(),
                        *enabled,
                        action.is_some(),
                        Some(Msg::Popover(PMsg::Activate(i))),
                    );
                    if confirm == Some(i) {
                        let question = ask.clone().unwrap_or_else(|| "Confirmer ?".into());
                        let danger = Color::from_rgb8(0xff, 0x55, 0x55);
                        let font = self.ui_font(iced::font::Weight::Medium);
                        column![
                            row_el,
                            row![
                                text(question)
                                    .font(self.font)
                                    .size(12.5)
                                    .font(font)
                                    .width(Length::Fill),
                                button(text("Annuler").font(self.font).size(12.5).font(font))
                                    .padding([4, 10])
                                    .on_press(Msg::Popover(PMsg::CancelConfirm))
                                    .style(move |_, st| button::Style {
                                        background: Some(Background::Color(Color {
                                            a: if matches!(st, button::Status::Hovered) {
                                                0.18
                                            } else {
                                                0.10
                                            },
                                            ..fg
                                        })),
                                        text_color: fg,
                                        border: Border::default()
                                            .rounded(super::design::SMALL_RADIUS),
                                        ..Default::default()
                                    }),
                                button(text("Confirmer").font(self.font).size(12.5).font(font))
                                    .padding([4, 10])
                                    .on_press(Msg::Popover(PMsg::Confirm(i)))
                                    .style(move |_, st| button::Style {
                                        background: Some(Background::Color(Color {
                                            a: if matches!(st, button::Status::Hovered) {
                                                1.0
                                            } else {
                                                0.85
                                            },
                                            ..danger
                                        })),
                                        text_color: Color::WHITE,
                                        border: Border::default()
                                            .rounded(super::design::SMALL_RADIUS),
                                        ..Default::default()
                                    }),
                            ]
                            .spacing(6)
                            .align_y(Alignment::Center)
                            .padding([4, 10])
                            .height(40),
                        ]
                        .into()
                    } else {
                        row_el
                    }
                }
            });
        }
        col.into()
    }

    fn pop_volume(&self, alpha: f32) -> Element<'_, Msg> {
        let a = &self.audio;
        let mut col = column![self.pop_header(alpha, crate::i18n::tr("Sortie").into())];
        col = col.push(self.audio_controls(
            alpha,
            self.volume,
            super::bar::volume_icon(self.volume),
            Msg::VolumeSet,
            Msg::VolumeMute,
        ));
        col = col.push(self.device_list(alpha, &a.outputs));
        col = col.push(self.pop_separator(alpha));
        col = col.push(self.pop_header(alpha, crate::i18n::tr("Micro").into()));
        if a.mic.is_some() {
            col = col.push(self.audio_controls(
                alpha,
                a.mic,
                super::bar::mic_icon(a.mic),
                Msg::MicSet,
                Msg::MicMute,
            ));
            col = col.push(self.device_list(alpha, &a.inputs));
        } else {
            col = col.push(self.pop_row(
                alpha,
                Space::new().into(),
                crate::i18n::tr("Aucun micro").into(),
                None,
                false,
                false,
                None,
            ));
        }
        col.push(self.pop_separator(alpha))
            .push(self.pop_row(
                alpha,
                Space::new().into(),
                crate::i18n::tr("Réglages audio…").into(),
                None,
                true,
                true,
                Some(Msg::Popover(PMsg::Spawn("pavucontrol".into()))),
            ))
            .into()
    }

    /// Menu Luminosité : un curseur par rétroéclairage et par écran DDC/CI.
    fn pop_brightness(&self, alpha: f32) -> Element<'_, Msg> {
        use super::bar::LightTarget;
        let l = &self.light;
        let mut rows: Vec<(String, LightTarget)> = Vec::new();
        if l.screen.is_some() {
            rows.push((crate::i18n::tr("Écran").to_string(), LightTarget::Screen));
        }
        for d in &l.displays {
            rows.push((d.connector.clone(), LightTarget::Ddc(d.bus)));
        }
        if l.kbd.is_some() {
            rows.push((crate::i18n::tr("Clavier").to_string(), LightTarget::Kbd));
        }
        let mut col = column![];
        if rows.is_empty() {
            col = col
                .push(self.pop_header(alpha, crate::i18n::tr("Aucune luminosité réglable").into()));
        }
        for (title, t) in rows {
            col = col.push(self.pop_header(alpha, title));
            col = col.push(self.light_slider(alpha, t));
        }
        col.into()
    }

    pub(super) fn light_slider(&self, alpha: f32, t: super::bar::LightTarget) -> Element<'_, Msg> {
        let v = self.light.level(t).unwrap_or(0.0);
        row![
            slider(0.0..=1.0f32, v, move |x| Msg::LightSet(t, x))
                .on_release(Msg::LightApply(t))
                .step(0.01f32)
                .style(move |theme, status| super::design::bar_slider_alpha(theme, status, alpha))
                .width(Length::Fill),
            text(format!("{:.0} %", v * 100.0))
                .size(12.5)
                .font(self.ui_font(iced::font::Weight::Medium))
                .width(40)
                .align_x(Alignment::End),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .padding([6, 10])
        .height(32)
        .into()
    }

    /// Curseur de volume et interrupteur « Muet » d'un périphérique.
    fn audio_controls(
        &self,
        alpha: f32,
        value: Option<(f32, bool)>,
        icon: &str,
        set: fn(f32) -> Msg,
        mute: Msg,
    ) -> Element<'_, Msg> {
        let fg = color(self.cfg.general.theme.fg, 1.0 * alpha);
        let (v, muted) = value.unwrap_or((0.0, false));
        let font = self.ui_font(iced::font::Weight::Medium);
        column![
            row![
                self.icon_named(icon, 16.0, fg)
                    .unwrap_or_else(|| Space::new().into()),
                slider(0.0..=1.5f32, v.min(1.5), set)
                    .step(0.01f32)
                    .style(move |theme, status| super::design::bar_slider_alpha(
                        theme, status, alpha
                    ))
                    .width(Length::Fill),
                text(format!("{:.0} %", v * 100.0))
                    .font(self.font)
                    .size(12.5)
                    .font(font)
                    .width(40)
                    .align_x(Alignment::End),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .padding([6, 10])
            .height(32),
            row![
                text("Muet")
                    .font(self.font)
                    .size(13.5)
                    .font(self.ui_font(iced::font::Weight::Normal))
                    .width(Length::Fill),
                toggler(muted)
                    .on_toggle(move |_| mute.clone())
                    .style(move |theme, status| {
                        let mut style = toggler::default(theme, status);
                        style.background = style.background.scale_alpha(alpha);
                        style.foreground = style.foreground.scale_alpha(alpha);
                        style.background_border_color.a *= alpha;
                        style.foreground_border_color.a *= alpha;
                        style.text_color = style.text_color.map(|c| c.scale_alpha(alpha));
                        style
                    })
            ]
            .align_y(Alignment::Center)
            .padding([4, 10])
            .height(30),
        ]
        .into()
    }

    /// Choix du périphérique par défaut (coché), s'il y en a plusieurs.
    fn device_list<'a>(
        &'a self,
        alpha: f32,
        list: &'a [super::bar::AudioDevice],
    ) -> Element<'a, Msg> {
        if list.len() < 2 {
            return Space::new().into();
        }
        let fg = color(self.cfg.general.theme.fg, 1.0 * alpha);
        column(list.iter().map(|d| {
            let lead = if d.default {
                self.icon_named("object-select-symbolic", 14.0, fg)
                    .unwrap_or_else(|| Space::new().into())
            } else {
                Space::new().into()
            };
            self.pop_row(
                alpha,
                lead,
                super::bar::truncate(&d.name, 28),
                None,
                true,
                !d.default,
                Some(Msg::AudioDefault(d.id)),
            )
        }))
        .into()
    }

    fn pop_calendar(&self, alpha: f32) -> Element<'_, Msg> {
        let th = &self.cfg.general.theme;
        let (fg, dim, accent) = (
            color(th.fg, 1.0 * alpha),
            color(th.fg, 0.55 * alpha),
            color(th.accent, 1.0 * alpha),
        );
        let title = format_now("%A %-d %B %Y");
        let mut t = title.chars();
        let title = t
            .next()
            .map(|c| c.to_uppercase().collect::<String>() + t.as_str())
            .unwrap_or_default();
        column![
            container(
                column![
                    text(title)
                        .font(self.font)
                        .size(15)
                        .font(self.ui_font(iced::font::Weight::Semibold))
                        .color(fg),
                    text(format_now("%H:%M"))
                        .font(self.font)
                        .size(12.5)
                        .color(dim)
                        .font(self.ui_font(iced::font::Weight::Normal)),
                ]
                .spacing(2)
            )
            .padding([6, 10]),
            container(self.calendar_nav(alpha, fg, accent)).padding([0, 10]),
            container(
                self.view_calendar(
                    12.5,
                    fg,
                    dim,
                    accent,
                    Some(
                        self.popover
                            .as_ref()
                            .map_or(WIDTH - 44.0, |p| p.size.width - 2.0 * SHADOW - 44.0)
                    ),
                    Some(self.cal_months)
                )
            )
            .padding([4, 10]),
        ]
        .into()
    }

    /// Mois précédent, aujourd'hui, mois suivant.
    fn calendar_nav(&self, alpha: f32, fg: Color, accent: Color) -> Element<'_, Msg> {
        let nav = |label: &str, delta: i32| {
            button(text(label.to_string()).font(self.font).size(13).color(fg))
                .padding([2, 10])
                .style(move |_, st| button::Style {
                    background: matches!(st, button::Status::Hovered).then(|| {
                        Color {
                            a: 0.10 * alpha,
                            ..accent
                        }
                        .into()
                    }),
                    border: Border::default().rounded(8),
                    text_color: fg,
                    ..Default::default()
                })
                .on_press(Msg::CalendarMonth(delta))
        };
        let first =
            super::widget_view::month_start(chrono::Local::now().date_naive(), self.cal_months);
        let month = first
            .format_localized("%B %Y", crate::i18n::chrono_locale())
            .to_string();
        let mut r = row![
            nav("‹", -1),
            text(month)
                .font(self.ui_font(iced::font::Weight::Semibold))
                .size(14)
                .color(fg)
                .width(Length::Fill)
                .align_x(Alignment::Center),
            nav("›", 1)
        ]
        .spacing(4);
        if self.cal_months != 0 {
            r = r.push(nav(crate::i18n::tr("Aujourd'hui"), 0));
        }
        r.align_y(Alignment::Center).into()
    }
}
