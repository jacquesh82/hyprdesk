//! Centre de contrôle de Continuité, composé comme celui de macOS : une carte
//! étroite sous son module, faite de tuiles translucides — les connexions à
//! gauche, Silence et deux commandes carrées à droite —, puis les bandeaux
//! Affichage et Son, enfin la lecture en cours. Aucune source système
//! supplémentaire : chaque tuile reprend une commande que le shell a déjà.
use std::time::Duration;

use iced::widget::{
    Space, button, column, container, mouse_area, row, scrollable, slider, stack, tooltip,
};
use iced::{Alignment, Border, Color, Element, Length, Theme};

use super::{PMsg, Popover, Request};
use crate::config::{RenderState, WidgetKind};
use crate::control::Command;
use crate::i18n::{text, tr};
use crate::ui::plugins::bar::surface::{BarPalette, truncate};
use crate::ui::{App, Msg, color};
use crate::widgets::{Value, media::Status};

mod keyboard;
pub(in crate::ui) use keyboard::handle as handle_key;

/// Largeur de la carte, marges comprises, ombre non comprise.
pub(in crate::ui) const WIDTH: f32 = 340.0;
const GAP: f32 = 10.0;
const PADDING: f32 = 12.0;
/// Rayon des tuiles ; la carte garde celui des menus.
const RADIUS: f32 = 14.0;
/// Hauteurs des tuiles : Silence, bandeaux Affichage et Son, lecture.
const SILENCE: f32 = 56.0;
const BAND: f32 = 66.0;
const MUSIC: f32 = 64.0;
/// Épaisseur de la piste des curseurs, diamètre des pastilles rondes.
const TRACK: f32 = 22.0;
const ORB: f32 = 30.0;

pub(in crate::ui) const ICONS: &[&str] = &[
    "network-wireless-symbolic",
    "network-wireless-disabled-symbolic",
    "bluetooth-symbolic",
    "weather-clear-night-symbolic",
    "night-light-symbolic",
    "night-light-disabled-symbolic",
    "camera-photo-symbolic",
    "media-record-symbolic",
    "video-display-symbolic",
    "system-lock-screen-symbolic",
    "display-brightness-symbolic",
    "send-to-symbolic",
    "audio-volume-muted-symbolic",
    "audio-volume-low-symbolic",
    "audio-volume-medium-symbolic",
    "audio-volume-high-symbolic",
    "audio-x-generic-symbolic",
    "media-playback-pause-symbolic",
    "media-playback-start-symbolic",
    "media-skip-forward-symbolic",
];

/// Lecture en cours, relevée tant que le centre est ouvert.
pub(in crate::ui) struct Dashboard {
    pub id: iced::window::Id,
    pub media: Option<Status>,
}

impl Dashboard {
    pub fn new() -> Self {
        Self {
            id: iced::window::Id::unique(),
            media: None,
        }
    }

    pub fn update(&mut self, sample: Sample) {
        self.media = sample.media;
    }
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub media: Option<Status>,
}

/// Le relevé s'arrête à la fermeture du centre et ses résultats portent son
/// identifiant.
pub(in crate::ui) fn samples(
    id: &iced::window::Id,
) -> impl iced::futures::Stream<Item = Msg> + use<> {
    use iced::futures::SinkExt;

    let id = *id;
    iced::stream::channel(1, async move |mut out| {
        loop {
            let media =
                tokio::time::timeout(Duration::from_secs(2), crate::widgets::media::status())
                    .await
                    .ok()
                    .flatten();
            if out
                .send(Msg::DashboardSample(id, Box::new(Sample { media })))
                .await
                .is_err()
            {
                break;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    })
}

/// Largeur d'une moitié de rangée, et côté d'une tuile carrée, pour une
/// carte de largeur `width`.
fn halves(width: f32) -> (f32, f32) {
    let half = (width - 2.0 * PADDING - GAP) / 2.0;
    (half, (half - GAP) / 2.0)
}

#[derive(Clone, Copy)]
struct Palette {
    fg: Color,
    muted: Color,
    accent: Color,
    ink: Color,
    /// Fond des tuiles : un voile du texte, plus clair que la carte.
    tile: Color,
    hover: Color,
    /// Pastille éteinte, piste et remplissage des curseurs.
    orb: Color,
    track: Color,
    fill: Color,
    /// Symbole posé sur le remplissage d'un curseur.
    on_fill: Color,
    tip: Color,
}

impl Palette {
    fn new(bar: &BarPalette, alpha: f32) -> Self {
        let fg = color(bar.fg, alpha);
        let bg = color(bar.bg, alpha);
        let veil = |a: f32| Color { a: a * alpha, ..fg };
        Self {
            fg,
            muted: Color {
                a: 0.62 * alpha,
                ..fg
            },
            accent: color(bar.accent, alpha),
            ink: crate::ui::plugins::bar::style::continuite::selected_ink(bar, alpha),
            tile: veil(0.09),
            hover: veil(0.14),
            orb: veil(0.16),
            track: Color {
                a: 0.30 * alpha,
                ..bg
            },
            fill: veil(0.92),
            on_fill: Color {
                a: 0.70 * alpha,
                ..bg
            },
            tip: color(bar.bg, alpha),
        }
    }

    fn tile(self) -> container::Style {
        container::Style {
            background: Some(self.tile.into()),
            border: Border::default().rounded(RADIUS),
            text_color: Some(self.fg),
            ..Default::default()
        }
    }

    /// Une tuile qui est aussi un bouton.
    fn tile_button(self, status: button::Status) -> button::Style {
        button::Style {
            background: Some(
                match status {
                    button::Status::Hovered | button::Status::Pressed => self.hover,
                    _ => self.tile,
                }
                .into(),
            ),
            border: Border::default().rounded(RADIUS),
            text_color: self.fg,
            ..Default::default()
        }
    }

    /// Zone cliquable posée sur une tuile : seul le survol la marque.
    fn flat(self, status: button::Status) -> button::Style {
        button::Style {
            background: matches!(status, button::Status::Hovered | button::Status::Pressed)
                .then(|| Color { a: 0.06, ..self.fg }.into()),
            border: Border::default().rounded(RADIUS - 4.0),
            text_color: self.fg,
            ..Default::default()
        }
    }

    /// Pastille ronde, `on` de 0 (éteinte) à 1 (allumée), atténuée de `beat`
    /// pendant une action (HIG SYS-06, SYS-07).
    fn orb(self, on: f32, beat: f32, status: button::Status) -> button::Style {
        let rest = if matches!(status, button::Status::Hovered | button::Status::Pressed) {
            self.hover
        } else {
            self.orb
        };
        let lit = if status == button::Status::Pressed {
            self.accent.scale_alpha(0.86)
        } else {
            self.accent
        };
        button::Style {
            background: Some(
                crate::ui::design::mix(rest, lit, on)
                    .scale_alpha(beat)
                    .into(),
            ),
            border: Border::default().rounded(ORB / 2.0),
            text_color: crate::ui::design::mix(self.fg, self.ink, on),
            ..Default::default()
        }
    }

    /// Curseur épais : piste sombre, remplissage clair, poignée ronde
    /// blanche du diamètre de la piste.
    fn slider(self, style: slider::Style) -> slider::Style {
        let mut style = style.with_circular_handle(TRACK / 2.0);
        style.rail.backgrounds = (self.fill.into(), self.track.into());
        style.rail.border = Border::default().rounded(TRACK / 2.0);
        style.rail.width = TRACK;
        style.handle.background = Color {
            a: 1.0,
            ..Color::WHITE
        }
        .into();
        style.handle.border_width = 0.5;
        style.handle.border_color = Color {
            a: 0.18,
            ..Color::BLACK
        };
        style
    }

    fn scroll(self, theme: &Theme, status: scrollable::Status) -> scrollable::Style {
        let mut style = scrollable::default(theme, status);
        style.vertical_rail.background = None;
        style.vertical_rail.scroller.background = self.hover.into();
        style
    }
}

impl App {
    /// Hauteur de la carte pour une largeur `width` sur l'écran `monitor` :
    /// le bandeau Affichage n'y est que si la luminosité de l'écran se règle.
    pub(super) fn dashboard_height(&self, width: f32, monitor: &str) -> f32 {
        let (_, square) = halves(width);
        let bands = if self.light.target_for(monitor).is_some() {
            2.0
        } else {
            1.0
        };
        2.0 * PADDING + SILENCE + GAP + square + bands * (GAP + BAND) + GAP + MUSIC
    }

    pub(super) fn continuity_dashboard(
        &self,
        popover: &Popover,
        dashboard: &Dashboard,
        alpha: f32,
    ) -> Element<'_, Msg> {
        let pal = Palette::new(&self.bar_palette(), alpha);
        let width = popover.size.width - 2.0 * super::SHADOW;
        let (half, square) = halves(width);
        let top = SILENCE + GAP + square;
        let bar = popover.bar;
        let monitor = self
            .bar_surfaces
            .get(&bar)
            .map_or("", |b| b.monitor.as_str());

        let connections = container(
            column![
                self.dashboard_connection(pal, bar, monitor, "network", "Wi-Fi"),
                self.dashboard_connection(pal, bar, monitor, "bluetooth", "Bluetooth"),
            ]
            .spacing(4),
        )
        .padding([8, 6])
        .width(half)
        .height(top)
        .center_y(top)
        .style(move |_| pal.tile());

        // Deux tuiles carrées : les commandes de l'éclairage nocturne et de la
        // capture quand leurs widgets sont là, sinon le verrouillage et les
        // écrans — toujours deux.
        let command = |cmd| Msg::On(monitor.into(), Box::new(Msg::DashboardCommand(cmd)));
        let squares = [
            self.dashboard_control_square(
                pal,
                bar,
                (
                    "nightlight",
                    "night-light-symbolic",
                    tr("Éclairage nocturne"),
                ),
                square,
            ),
            self.dashboard_control_square(
                pal,
                bar,
                ("capture", "camera-photo-symbolic", tr("Capture d'écran")),
                square,
            ),
            Some(self.dashboard_square(
                pal,
                ("system-lock-screen-symbolic", tr("Verrouiller")),
                (0.0, 1.0),
                Some(Msg::Popover(PMsg::Spawn("hyprdesk lock".into()))),
                square,
            )),
            Some(self.dashboard_square(
                pal,
                ("video-display-symbolic", tr("Écrans")),
                (0.0, 1.0),
                Some(command(Command::Settings("screens".into()))),
                square,
            )),
        ];
        let squares = row(squares.into_iter().flatten().take(2)).spacing(GAP);
        let right = column![self.dashboard_silence(pal, bar, half), squares].spacing(GAP);

        let mut bands = column![].spacing(GAP);
        if let Some(display) = self.dashboard_display(pal, monitor) {
            bands = bands.push(display);
        }
        bands = bands.push(self.dashboard_sound(pal, bar));

        // Quatre groupes de contrôles, dans l'ordre de lecture : Ctrl+Tab et
        // Ctrl+Maj + Tab passent de l'un au suivant (HIG ENT-11).
        let body = column![
            row![keyboard::group(0, connections), keyboard::group(1, right)].spacing(GAP),
            keyboard::group(2, bands),
            keyboard::group(3, self.dashboard_music(pal, dashboard)),
        ]
        .spacing(GAP);
        container(
            scrollable(body)
                .id(keyboard::SCROLL)
                .style(move |theme, status| pal.scroll(theme, status))
                .height(Length::Fill)
                .width(Length::Fill),
        )
        .id(keyboard::PANE)
        .padding(PADDING)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn dash_label(&self, label: impl Into<String>, size: f32, fg: Color) -> iced::widget::Text<'_> {
        text(label.into())
            .font(self.font)
            .size(crate::ui::tokens::ts(size))
            .color(fg)
            .wrapping(iced::widget::text::Wrapping::None)
    }

    fn dash_strong(
        &self,
        label: impl Into<String>,
        size: f32,
        fg: Color,
    ) -> iced::widget::Text<'_> {
        self.dash_label(label, size, fg)
            .font(self.ui_font(iced::font::Weight::Semibold))
    }

    fn dash_icon(&self, name: &str, fallback: &str, size: f32, fg: Color) -> Element<'_, Msg> {
        self.icon_named(name, size, fg)
            .unwrap_or_else(|| self.dash_label(fallback, size, fg).into())
    }

    /// Info-bulle d'une commande du centre : son libellé, sous le curseur.
    fn dash_tooltip<'a>(
        &'a self,
        pal: Palette,
        content: impl Into<Element<'a, Msg>>,
        label: impl Into<String>,
    ) -> Element<'a, Msg> {
        tooltip(
            content,
            self.dash_label(label, 12.0, pal.fg),
            tooltip::Position::FollowCursor,
        )
        .style(move |_| container::Style {
            background: Some(pal.tip.into()),
            border: Border::default().rounded(6),
            text_color: Some(pal.fg),
            ..Default::default()
        })
        .into()
    }

    /// Pastille ronde d'une commande : son symbole, accent quand elle est
    /// allumée.
    fn dash_orb<'a>(
        &'a self,
        pal: Palette,
        icon: &str,
        (on, beat): (f32, f32),
        action: Option<Msg>,
    ) -> Element<'a, Msg> {
        let ink = crate::ui::design::mix(pal.fg, pal.ink, on).scale_alpha(beat);
        let b = button(
            container(
                self.icon_named_weighted(icon, 15.0, ink, iced::font::Weight::Medium)
                    .unwrap_or_else(|| Space::new().into()),
            )
            .center_x(ORB)
            .center_y(ORB),
        )
        .padding(0)
        .width(ORB)
        .height(ORB)
        .on_press_maybe(action.clone())
        .style(move |_, status| pal.orb(on, beat, status));
        crate::ui::focus::press(b, action, pal.fg)
    }

    /// Ligne d'une connexion (Wi-Fi, Bluetooth) : la pastille l'allume ou
    /// l'éteint, le titre et son état ouvrent ses détails — le menu de son
    /// widget, sinon sa page des Réglages.
    fn dashboard_connection(
        &self,
        pal: Palette,
        bar: iced::window::Id,
        monitor: &str,
        type_id: &str,
        label: &str,
    ) -> Element<'_, Msg> {
        let Some(widget) = self.control_source(type_id) else {
            return Space::new().into();
        };
        let value = self.values.get(&widget.id);
        let rich = match value {
            Some(Value::Rich(r)) if r.state == RenderState::Hidden => return Space::new().into(),
            Some(Value::Rich(r)) => Some(r),
            _ => None,
        };
        let pending = self
            .control_actions
            .get(&widget.id)
            .is_some_and(Option::is_none);
        let error = self
            .control_actions
            .get(&widget.id)
            .and_then(|e| e.as_deref())
            .map(str::to_owned)
            .or_else(|| match value {
                Some(Value::Error(e)) => Some(e.clone()),
                _ => None,
            });
        let active = rich.is_some_and(|r| r.toggled == Some(true));
        let details = Self::dashboard_widget_menu(bar, widget).unwrap_or_else(|| {
            Msg::On(
                monitor.into(),
                Box::new(Msg::DashboardCommand(Command::Settings(type_id.into()))),
            )
        });
        let can_toggle = matches!(
            widget.kind,
            WidgetKind::Lua {
                toggle: Some(_),
                ..
            }
        ) && rich.is_some_and(|r| r.toggled.is_some());
        let toggle = if can_toggle {
            Msg::ControlToggle(widget.id.clone())
        } else {
            details.clone()
        };
        // Pendant l'action, la pastille pulse ; l'état change en fondu (HIG
        // SYS-06, SYS-07).
        let pulse = if pending {
            self.pending_pulse(&widget.id)
        } else {
            None
        };
        let on = self.toggle_progress(&widget.id, active);
        // Symbole de la radio Wi-Fi quand son état est connu (HIG SYS-08).
        let icon = crate::ui::controls::radio_symbol(type_id, rich.and_then(|r| r.toggled))
            .or_else(|| rich.and_then(|r| r.icon.as_deref()))
            .unwrap_or(if type_id == "network" {
                "network-wireless-symbolic"
            } else {
                "bluetooth-symbolic"
            });
        // L'état : le réseau ou l'appareil connu, sinon allumé ou éteint.
        let state = if pending && pulse.is_none() {
            tr("En cours…").to_string()
        } else if error.is_some() {
            tr("Indisponible").to_string()
        } else {
            rich.and_then(|r| r.tooltip.as_deref())
                .and_then(|t| t.lines().map(str::trim).find(|l| !l.is_empty()))
                .map(str::to_owned)
                .unwrap_or_else(|| {
                    tr(match rich.and_then(|r| r.toggled) {
                        Some(true) => "Activé",
                        Some(false) => "Désactivé",
                        None => "Chargement…",
                    })
                    .to_string()
                })
        };
        let orb = self.dash_orb(
            pal,
            icon,
            (on, pulse.unwrap_or(1.0)),
            (!pending).then_some(toggle),
        );
        let words = button(
            column![
                self.dash_strong(tr(label), 13.0, pal.fg),
                self.dash_label(truncate(&state, 18), 11.0, pal.muted),
            ]
            .spacing(1),
        )
        .padding([4, 6])
        .width(Length::Fill)
        .on_press(details.clone())
        .style(move |_, status| pal.flat(status));
        let words = crate::ui::focus::press(words, Some(details), pal.fg);
        let words = match error {
            Some(e) => self.dash_tooltip(pal, words, e),
            None => words,
        };
        container(row![orb, words].spacing(4).align_y(Alignment::Center))
            .padding([0, 4])
            .height(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn dashboard_widget_menu(
        bar: iced::window::Id,
        widget: &crate::config::WidgetDef,
    ) -> Option<Msg> {
        let WidgetKind::Lua {
            menu: Some(func), ..
        } = widget.kind
        else {
            return None;
        };
        Some(Msg::OpenPopover(
            bar,
            widget.id.clone(),
            Request::LuaMenu {
                widget: widget.id.clone(),
                func,
            },
        ))
    }

    /// Tuile Silence (« Ne pas déranger » de macOS). Allumée, elle nomme sa
    /// fin et un clic l'éteint ; éteinte, elle ouvre le choix de la fin —
    /// choisir, c'est allumer (HIG PAT-04).
    fn dashboard_silence(
        &self,
        pal: Palette,
        bar: iced::window::Id,
        width: f32,
    ) -> Element<'_, Msg> {
        let silence = self.toasts.dnd;
        // La lune de macOS, allumée ou non : la pastille dit l'état.
        let icon = "weather-clear-night-symbolic";
        let (label, action) = if self.toasts.active || self.preview {
            let action = if silence {
                Msg::Control(Command::Notifications("dnd off".into()))
            } else {
                Msg::OpenPopover(bar, "silence".into(), Request::Silence)
            };
            (tr("Silence"), Some(action))
        } else {
            // Un autre démon tient les notifications : son menu, s'il en a un.
            (
                tr("Notifications"),
                self.control_source("notifications")
                    .and_then(|widget| Self::dashboard_widget_menu(bar, widget)),
            )
        };
        let state = match (silence, self.toasts.dnd_until) {
            (true, Some(until)) => Some(crate::tr!(
                "jusqu'à {hour}",
                hour = crate::notifications::silence_end_label(until)
            )),
            (true, None) => Some(tr("Activé").to_string()),
            (false, _) => None,
        };
        let on = if silence { 1.0 } else { 0.0 };
        let mut words = column![self.dash_strong(label, 13.0, pal.fg)].spacing(1);
        if let Some(state) = state {
            words = words.push(self.dash_label(truncate(&state, 16), 11.0, pal.muted));
        }
        let tile = button(
            container(
                row![self.dash_orb(pal, icon, (on, 1.0), None), words]
                    .spacing(10)
                    .align_y(Alignment::Center),
            )
            .center_y(Length::Fill),
        )
        .padding([0, 10])
        .width(width)
        .height(SILENCE)
        .on_press_maybe(action.clone())
        .style(move |_, status| pal.tile_button(status));
        crate::ui::focus::press(tile, action, pal.fg)
    }

    /// Tuile carrée : un symbole dans sa pastille — accent quand la commande
    /// est allumée, `on` de 0 à 1, atténuée de `beat` pendant l'action —, son
    /// nom dessous, sur deux lignes au plus.
    fn dashboard_square(
        &self,
        pal: Palette,
        (icon, label): (&str, &str),
        (on, beat): (f32, f32),
        action: Option<Msg>,
        side: f32,
    ) -> Element<'_, Msg> {
        let tile = button(
            container(
                column![
                    self.dash_orb(pal, icon, (on, beat), None),
                    self.dash_label(label, 11.0, pal.fg)
                        .align_x(Alignment::Center)
                        .wrapping(iced::widget::text::Wrapping::Word),
                ]
                .spacing(5)
                .align_x(Alignment::Center),
            )
            .center(Length::Fill),
        )
        .padding([6, 4])
        .width(side)
        .height(side)
        .on_press_maybe(action.clone())
        .style(move |_, status| pal.tile_button(status));
        crate::ui::focus::press(tile, action, pal.fg)
    }

    /// Tuile carrée d'une commande Lua (éclairage nocturne, capture) : un
    /// clic l'allume ou l'éteint quand elle se bascule, sinon ouvre son menu ;
    /// le clic droit ouvre toujours son menu. Rien sans son widget, ni quand
    /// il se dit caché.
    fn dashboard_control_square(
        &self,
        pal: Palette,
        bar: iced::window::Id,
        (type_id, fallback, label): (&str, &str, &str),
        side: f32,
    ) -> Option<Element<'_, Msg>> {
        let widget = self.control_source(type_id)?;
        let rich = match self.values.get(&widget.id) {
            Some(Value::Rich(r)) if r.state == RenderState::Hidden => return None,
            Some(Value::Rich(r)) => Some(r),
            _ => None,
        };
        let menu = Self::dashboard_widget_menu(bar, widget);
        let can_toggle = matches!(
            widget.kind,
            WidgetKind::Lua {
                toggle: Some(_),
                ..
            }
        ) && rich.is_some_and(|r| r.toggled.is_some());
        let pending = self
            .control_actions
            .get(&widget.id)
            .is_some_and(Option::is_none);
        let action = if can_toggle {
            Some(Msg::ControlToggle(widget.id.clone()))
        } else {
            menu.clone()
        };
        let active = rich.is_some_and(|r| r.toggled == Some(true));
        let on = self.toggle_progress(&widget.id, active);
        let beat = if pending {
            self.pending_pulse(&widget.id).unwrap_or(1.0)
        } else {
            1.0
        };
        let icon = rich.and_then(|r| r.icon.as_deref()).unwrap_or(fallback);
        let tile = self.dashboard_square(
            pal,
            (icon, label),
            (on, beat),
            (!pending).then_some(action).flatten(),
            side,
        );
        let tile = match menu.filter(|_| can_toggle) {
            Some(menu) => mouse_area(tile).on_right_press(menu).into(),
            None => tile,
        };
        Some(match rich.and_then(|r| r.tooltip.clone()) {
            Some(tip) => self.dash_tooltip(pal, tile, tip),
            None => tile,
        })
    }

    /// Curseur épais d'un bandeau, son symbole posé au début de la piste ;
    /// `symbol` peut le rendre cliquable (couper le son).
    fn dash_slider<'a>(
        &'a self,
        pal: Palette,
        control: Element<'a, Msg>,
        icon: &str,
        symbol: Option<(Msg, String)>,
    ) -> Element<'a, Msg> {
        let glyph = container(self.dash_icon(icon, "•", 13.0, pal.on_fill))
            .center_x(TRACK + 4.0)
            .center_y(TRACK);
        let glyph: Element<'a, Msg> = match symbol {
            Some((press, tip)) => self.dash_tooltip(
                pal,
                mouse_area(glyph)
                    .on_press(press)
                    .interaction(iced::mouse::Interaction::Pointer),
                tip,
            ),
            None => glyph.into(),
        };
        stack![
            control,
            container(glyph)
                .padding([0, 2])
                .height(TRACK)
                .center_y(TRACK)
        ]
        .width(Length::Fill)
        .height(TRACK)
        .into()
    }

    /// Bouton rond à droite d'un bandeau.
    fn dash_side_button<'a>(
        &'a self,
        pal: Palette,
        icon: &str,
        tip: &str,
        action: Msg,
    ) -> Element<'a, Msg> {
        let b = button(
            container(self.dash_icon(icon, "›", 15.0, pal.fg))
                .center_x(28)
                .center_y(28),
        )
        .padding(0)
        .width(28)
        .height(28)
        .on_press(action.clone())
        .style(move |_, status| button::Style {
            background: Some(
                match status {
                    button::Status::Hovered | button::Status::Pressed => pal.hover,
                    _ => pal.orb,
                }
                .into(),
            ),
            border: Border::default().rounded(14),
            text_color: pal.fg,
            ..Default::default()
        });
        self.dash_tooltip(pal, crate::ui::focus::press(b, Some(action), pal.fg), tip)
    }

    fn dash_band<'a>(
        &'a self,
        pal: Palette,
        title: &str,
        line: Element<'a, Msg>,
    ) -> Element<'a, Msg> {
        container(column![self.dash_strong(title.to_string(), 13.0, pal.fg), line].spacing(8))
            .padding([9, 12])
            .width(Length::Fill)
            .height(BAND)
            .clip(true)
            .style(move |_| pal.tile())
            .into()
    }

    /// Bandeau Affichage : la luminosité de l'écran ; le bouton ouvre ses
    /// Réglages. Rien si la luminosité de l'écran ne se règle pas.
    fn dashboard_display(&self, pal: Palette, monitor: &str) -> Option<Element<'_, Msg>> {
        let target = self.light.target_for(monitor)?;
        let level = self.light.level(target).unwrap_or_default();
        let set = move |v| Msg::LightSet(target, v);
        let commit = Msg::LightApply(target);
        let control = slider(0.0..=1.0, level, set)
            .on_release(commit.clone())
            .step(0.01_f32)
            .width(Length::Fill)
            .height(TRACK)
            .style(move |theme, status| pal.slider(slider::default(theme, status)));
        let control = crate::ui::focus::adjust(control, level, 1.0, set, Some(commit), pal.fg);
        let line = row![
            self.dash_slider(pal, control, "display-brightness-symbolic", None),
            self.dash_side_button(
                pal,
                "video-display-symbolic",
                tr("Écrans"),
                Msg::On(
                    monitor.into(),
                    Box::new(Msg::DashboardCommand(Command::Settings("screens".into()))),
                ),
            ),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        Some(self.dash_band(pal, tr("Affichage"), line.into()))
    }

    /// Bandeau Son : le volume de la sortie ; son symbole la coupe, le bouton
    /// ouvre la carte du son (choix de la sortie, micro, applications).
    fn dashboard_sound(&self, pal: Palette, bar: iced::window::Id) -> Element<'_, Msg> {
        let line: Element<'_, Msg> = match self.volume {
            Some((volume, muted)) => {
                let control = slider(0.0..=1.5, volume.min(1.5), Msg::VolumeSet)
                    .step(0.01_f32)
                    .width(Length::Fill)
                    .height(TRACK)
                    .style(move |theme, status| pal.slider(slider::default(theme, status)));
                let control =
                    crate::ui::focus::adjust(control, volume, 1.5, Msg::VolumeSet, None, pal.fg);
                let tip = if muted {
                    tr("Rétablir le son")
                } else {
                    tr("Couper le son")
                };
                row![
                    self.dash_slider(
                        pal,
                        control,
                        crate::ui::plugins::bar::surface::volume_icon(self.volume),
                        Some((Msg::VolumeMute, tip.to_string())),
                    ),
                    self.dash_side_button(
                        pal,
                        "send-to-symbolic",
                        tr("Sortie audio"),
                        Msg::OpenPopover(bar, "volume".into(), Request::Volume),
                    ),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .into()
            }
            None => self
                .dash_label(tr("Audio indisponible"), 12.0, pal.muted)
                .into(),
        };
        self.dash_band(pal, tr("Son"), line)
    }

    /// Tuile de la lecture en cours : pochette, titre et artiste, lecture et
    /// piste suivante.
    fn dashboard_music(&self, pal: Palette, dashboard: &Dashboard) -> Element<'_, Msg> {
        let media = dashboard.media.as_ref();
        let title = media
            .map(|m| {
                if m.title.is_empty() {
                    m.player.as_str()
                } else {
                    m.title.as_str()
                }
            })
            .unwrap_or(tr("Aucune lecture"));
        let artist = media.map_or("", |m| m.artist.as_str());
        let playing = media.is_some_and(|m| m.playing);
        let action = |method| media.map(|m| Msg::Media(m.bus.clone(), method));
        let cover: Element<'_, Msg> = match media
            .and_then(|m| m.art.strip_prefix("file://"))
            .filter(|p| std::path::Path::new(p).is_file())
        {
            Some(path) => iced::widget::image(iced::widget::image::Handle::from_path(path))
                .width(40)
                .height(40)
                .content_fit(iced::ContentFit::Cover)
                .border_radius(8.0)
                .into(),
            None => container(self.dash_icon("audio-x-generic-symbolic", "♪", 18.0, pal.ink))
                .center_x(40)
                .center_y(40)
                .style(move |_| container::Style {
                    background: Some(pal.accent.into()),
                    border: Border::default().rounded(8),
                    ..Default::default()
                })
                .into(),
        };
        let mut words = column![self.dash_strong(truncate(title, 24), 13.0, pal.fg)].spacing(1);
        if !artist.is_empty() {
            words = words.push(self.dash_label(truncate(artist, 28), 11.0, pal.muted));
        }
        let control = |(icon, fallback, label): (&str, &str, &str), action: Option<Msg>| {
            let fg = if action.is_some() { pal.fg } else { pal.muted };
            let b = button(
                container(self.dash_icon(icon, fallback, 17.0, fg))
                    .center_x(30)
                    .center_y(30),
            )
            .padding(0)
            .on_press_maybe(action.clone())
            .style(move |_, status| pal.flat(status));
            self.dash_tooltip(pal, crate::ui::focus::press(b, action, pal.fg), label)
        };
        let play = if playing {
            ("media-playback-pause-symbolic", "Ⅱ", tr("Pause"))
        } else {
            ("media-playback-start-symbolic", "▶", tr("Lecture"))
        };
        container(
            row![
                cover,
                words.width(Length::Fill),
                control(play, action("PlayPause")),
                control(
                    ("media-skip-forward-symbolic", "›", tr("Suivant")),
                    action("Next")
                ),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .padding([0, 12])
        .width(Length::Fill)
        .height(MUSIC)
        .center_y(MUSIC)
        .clip(true)
        .style(move |_| pal.tile())
        .into()
    }
}
