//! Menus déroulants de la barre, à la manière de macOS : une carte arrondie,
//! translucide et floutée, ombre portée, qui descend sous le module cliqué.
//!
//! Deux surfaces : la carte (calque `overlay`, clavier exclusif pour Échap)
//! et un « capteur » transparent plein écran juste dessous, qui ferme le menu
//! au premier clic ailleurs.

use crate::i18n::text;
use iced::widget::{Space, button, column, container, mouse_area, row, rule, slider, toggler};
use iced::window::Id;
use iced::{
    Alignment, Background, Border, Color, Element, Length, Padding, Shadow, Size, Task, Vector,
};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::{App, Msg, color, format_now};
use crate::config::{BarPosition, MenuEntry};

pub const POPOVER_NAMESPACE: &str = "hyprdesk-popover";
/// Marge transparente autour de la carte, pour son ombre.
pub const SHADOW: f32 = 18.0;
const WIDTH: f32 = 300.0;
/// Écart entre la barre et le haut de la carte.
const GAP: f32 = 6.0;

/// Ce qu'on demande d'ouvrir (le contenu peut devoir être calculé avant).
#[derive(Debug, Clone)]
pub enum Request {
    Volume,
    Calendar,
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

#[derive(Debug, Clone)]
pub enum MenuSource {
    Lua { widget: String },
    Tray { bus: String, menu_path: String },
}

#[derive(Debug, Clone)]
pub enum Kind {
    Volume,
    Calendar,
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
    /// Module d'origine : recliquer dessus referme.
    pub key: String,
    pub kind: Kind,
    pub size: Size,
    pub started: std::time::Instant,
    /// Dernière relecture du menu (menus du tray : suivis ouverts).
    pub refreshed: std::time::Instant,
    max_height: f32,
    /// Entrée en attente de confirmation.
    pub confirm: Option<usize>,
    /// Texte des champs de saisie, par entrée.
    pub inputs: std::collections::HashMap<usize, String>,
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

fn card_height(kind: &Kind, confirm: Option<usize>, audio: &super::bar::Audio) -> f32 {
    let inner = match kind {
        // Sortie, séparateur, micro (ou « aucun »), séparateur, réglages.
        Kind::Volume => {
            let mic = if audio.mic.is_some() {
                AUDIO_SECTION + device_rows(&audio.inputs)
            } else {
                26.0 + 30.0
            };
            AUDIO_SECTION + device_rows(&audio.outputs) + 11.0 + mic + 11.0 + 30.0 + 16.0
        }
        // Six lignes au plus : on change de mois sans changer la taille de la carte.
        Kind::Calendar => 112.0 + 28.0 + 6.0 * 23.0,
        Kind::Menu { entries, .. } => {
            let rows: f32 = entries
                .iter()
                .enumerate()
                .map(|(i, e)| match e {
                    MenuEntry::Header(_) => 26.0,
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
    /// Ouvre un menu sous le module cliqué de la barre `bar` (ou le referme).
    pub(super) fn open_popover(&mut self, bar: Id, key: String, kind: Kind) -> Task<Msg> {
        let toggling = self.popover.as_ref().is_some_and(|p| p.key == key);
        let mut tasks = vec![self.close_popover()];
        if toggling {
            return Task::batch(tasks);
        }
        let Some(b) = self.bar_surfaces.get(&bar).cloned() else {
            return Task::batch(tasks);
        };
        let Some(m) = self.monitors.iter().find(|m| m.name == b.monitor).cloned() else {
            return Task::batch(tasks);
        };
        let max_height = (m.height as f32 - b.height as f32 - 2.0 * GAP).max(100.0);
        let h = (card_height(&kind, None, &self.audio) + 2.0 * SHADOW).min(max_height);
        let size = Size::new(WIDTH + 2.0 * SHADOW, h);
        // Centré sous le curseur (position demandée à Hyprland), sans déborder de l'écran.
        let cx = self
            .cursor
            .get(&bar)
            .map(|p| p.x)
            .filter(|x| (0.0..=m.width as f32).contains(x))
            .unwrap_or(m.width as f32 - WIDTH / 2.0 - 16.0);
        let left = (cx - WIDTH / 2.0).clamp(8.0, (m.width as f32 - WIDTH - 8.0).max(8.0)) - SHADOW;
        let (catcher, win) = (Id::unique(), Id::unique());
        // Le capteur d'abord : la carte, créée ensuite et sur le calque overlay, passe au-dessus.
        tasks.push(Task::done(Msg::NewLayerShell {
            settings: NewLayerShellSettings {
                size: Some((0, 0)),
                layer: Layer::Top,
                anchor: LsAnchor::Top | LsAnchor::Bottom | LsAnchor::Left | LsAnchor::Right,
                exclusive_zone: Some(-1),
                keyboard_interactivity: KeyboardInteractivity::None,
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
        // Zone exclusive 0 : la carte se place après les barres ; la marge (négative)
        // ramène la carte à GAP pixels de la barre malgré la marge d'ombre.
        let v = (GAP - SHADOW) as i32;
        let margin = if b.position == BarPosition::Top {
            (v, 0, 0, left as i32)
        } else {
            (0, 0, v, left as i32)
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
            key,
            kind,
            size,
            started: std::time::Instant::now(),
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
        i: usize,
        label: Option<&'a str>,
        placeholder: &'a str,
        secret: bool,
    ) -> Element<'a, Msg> {
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let value = self
            .popover
            .as_ref()
            .and_then(|p| p.inputs.get(&i))
            .map(String::as_str)
            .unwrap_or("");
        let mut col = column![].spacing(4).padding([4, 10]);
        if let Some(l) = label {
            col = col.push(
                text(l.to_string())
                    .font(self.font)
                    .size(12)
                    .color(Color { a: 0.7, ..fg }),
            );
        }
        col.push(
            iced::widget::text_input(placeholder, value)
                .secure(secret)
                .on_input(move |s| Msg::Popover(PMsg::Input(i, s)))
                .on_submit(Msg::Popover(PMsg::Submit(i)))
                .font(self.font)
                .size(13)
                .padding(8),
        )
        .into()
    }

    pub(super) fn close_popover(&mut self) -> Task<Msg> {
        // Le calendrier rouvre toujours sur le mois en cours.
        self.cal_months = 0;
        match self.popover.take() {
            Some(p) => Task::batch([iced::window::close(p.win), iced::window::close(p.catcher)]),
            None => Task::none(),
        }
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
        let height = (content_height + 12.0 + 2.0 * SHADOW)
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

    pub(super) fn view_popover(&self) -> Element<'_, Msg> {
        let Some(p) = &self.popover else {
            return Space::new().into();
        };
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let content: Element<'_, Msg> = match &p.kind {
            Kind::Volume => self.pop_volume(),
            Kind::Calendar => self.pop_calendar(),
            Kind::Menu { entries, .. } => self.pop_menu(entries, p.confirm),
        };
        let bg = color(th.bg, th.effect(crate::config::Place::Menus).opacity);
        let edge = Color { a: 0.12, ..fg };
        let win = p.win;
        let measured = iced::widget::sensor(content)
            .key(win)
            .on_resize(move |size| Msg::PopoverMeasured(win, size.height));
        let content = iced::widget::scrollable(measured).height(Length::Fill);
        let card = container(content)
            .padding(6)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(bg.into()),
                border: Border::default()
                    .rounded(super::design::RADIUS)
                    .width(1)
                    .color(edge),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
                    offset: Vector::new(0.0, 8.0),
                    blur_radius: 22.0,
                },
                text_color: Some(fg),
                ..Default::default()
            });
        let duration = self.motion_duration();
        let t = if duration.is_zero() {
            1.0
        } else {
            (p.started.elapsed().as_secs_f32() / duration.as_secs_f32()).clamp(0.0, 1.0)
        };
        let settle = 1.0 - (1.0 - t).powi(3);
        container(
            iced::widget::float(card)
                .scale(0.98 + 0.02 * settle)
                .translate(move |_, _| Vector::new(0.0, -4.0 * (1.0 - settle))),
        )
        .padding(SHADOW)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn pop_header(&self, label: String) -> Element<'_, Msg> {
        let dim = color(self.cfg.general.theme.fg, 0.5);
        container(
            text(label)
                .font(self.font)
                .size(11.5)
                .color(dim)
                .font(self.title_font()),
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

    fn pop_separator(&self) -> Element<'_, Msg> {
        let line = color(self.cfg.general.theme.fg, 0.10);
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
    fn pop_row<'a>(
        &'a self,
        lead: Element<'a, Msg>,
        label: String,
        detail: Option<String>,
        enabled: bool,
        clickable: bool,
        msg: Option<Msg>,
    ) -> Element<'a, Msg> {
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let dim = color(th.fg, if enabled { 0.55 } else { 0.3 });
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
                background: hot.then_some(Background::Color(Color { a: 0.18, ..accent })),
                text_color: if !enabled {
                    Color { a: 0.35, ..fg }
                } else if hot {
                    Color::WHITE
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
        entries: &'a [MenuEntry],
        confirm: Option<usize>,
    ) -> Element<'a, Msg> {
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let mut col = column![].spacing(0);
        if entries.is_empty() {
            col = col.push(self.pop_row(
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
                MenuEntry::Header(h) => self.pop_header(h.clone()),
                MenuEntry::Separator => self.pop_separator(),
                MenuEntry::Input {
                    label,
                    placeholder,
                    secret,
                    ..
                } => self.pop_input(i, label.as_deref(), placeholder, *secret),
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

    fn pop_volume(&self) -> Element<'_, Msg> {
        let a = &self.audio;
        let mut col = column![self.pop_header(crate::i18n::tr("Sortie").into())];
        col = col.push(self.audio_controls(
            self.volume,
            super::bar::volume_icon(self.volume),
            Msg::VolumeSet,
            Msg::VolumeMute,
        ));
        col = col.push(self.device_list(&a.outputs));
        col = col.push(self.pop_separator());
        col = col.push(self.pop_header(crate::i18n::tr("Micro").into()));
        if a.mic.is_some() {
            col = col.push(self.audio_controls(
                a.mic,
                super::bar::mic_icon(a.mic),
                Msg::MicSet,
                Msg::MicMute,
            ));
            col = col.push(self.device_list(&a.inputs));
        } else {
            col = col.push(self.pop_row(
                Space::new().into(),
                crate::i18n::tr("Aucun micro").into(),
                None,
                false,
                false,
                None,
            ));
        }
        col.push(self.pop_separator())
            .push(self.pop_row(
                Space::new().into(),
                crate::i18n::tr("Réglages audio…").into(),
                None,
                true,
                true,
                Some(Msg::Popover(PMsg::Spawn("pavucontrol".into()))),
            ))
            .into()
    }

    /// Curseur de volume et interrupteur « Muet » d'un périphérique.
    fn audio_controls(
        &self,
        value: Option<(f32, bool)>,
        icon: &str,
        set: fn(f32) -> Msg,
        mute: Msg,
    ) -> Element<'_, Msg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let (v, muted) = value.unwrap_or((0.0, false));
        let font = self.ui_font(iced::font::Weight::Medium);
        column![
            row![
                self.icon_named(icon, 16.0, fg)
                    .unwrap_or_else(|| Space::new().into()),
                slider(0.0..=1.5f32, v.min(1.5), set)
                    .step(0.01f32)
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
                toggler(muted).on_toggle(move |_| mute.clone())
            ]
            .align_y(Alignment::Center)
            .padding([4, 10])
            .height(30),
        ]
        .into()
    }

    /// Choix du périphérique par défaut (coché), s'il y en a plusieurs.
    fn device_list<'a>(&'a self, list: &'a [super::bar::AudioDevice]) -> Element<'a, Msg> {
        if list.len() < 2 {
            return Space::new().into();
        }
        let fg = color(self.cfg.general.theme.fg, 1.0);
        column(list.iter().map(|d| {
            let lead = if d.default {
                self.icon_named("object-select-symbolic", 14.0, fg)
                    .unwrap_or_else(|| Space::new().into())
            } else {
                Space::new().into()
            };
            self.pop_row(
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

    fn pop_calendar(&self) -> Element<'_, Msg> {
        let th = &self.cfg.general.theme;
        let (fg, dim, accent) = (color(th.fg, 1.0), color(th.fg, 0.55), color(th.accent, 1.0));
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
                        .font(self.title_font())
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
            container(self.calendar_nav(fg, accent)).padding([0, 10]),
            container(self.view_calendar(
                12.5,
                fg,
                dim,
                accent,
                Some(WIDTH - 32.0),
                Some(self.cal_months)
            ))
            .padding([4, 10]),
        ]
        .into()
    }

    /// Mois précédent, aujourd'hui, mois suivant.
    fn calendar_nav(&self, fg: Color, accent: Color) -> Element<'_, Msg> {
        let nav = |label: &str, delta: i32| {
            button(text(label.to_string()).font(self.font).size(13).color(fg))
                .padding([2, 10])
                .style(move |_, st| button::Style {
                    background: matches!(st, button::Status::Hovered)
                        .then(|| Color { a: 0.14, ..accent }.into()),
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
                .font(self.title_font())
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
