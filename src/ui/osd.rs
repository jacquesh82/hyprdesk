//! Affichage à l'écran (OSD) des touches de volume et de luminosité : une
//! carte du thème en bas de l'écran actif — icône, jauge, pourcentage — qui
//! s'efface d'elle-même. Commandé par `hyprdesk osd …` (`crate::osd`), qui
//! fait l'action puis prévient le démon (`control::Command::Osd`).

use std::time::{Duration, Instant};

use iced::widget::{container, row, text};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Shadow, Task, Vector};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::{App, Msg, color};
use crate::osd::Kind;

pub const NAMESPACE: &str = "hyprdesk-osd";
/// Durée d'affichage après la dernière touche.
pub const SHOW_FOR: Duration = Duration::from_millis(1500);
const WIDTH: f32 = 300.0;
const HEIGHT: f32 = 64.0;
/// Marge transparente pour l'ombre.
const SHADOW: f32 = 18.0;
/// Hauteur au-dessus du bas de l'écran (au-dessus du dock).
const LIFT: i32 = 140;

/// Carte affichée.
pub struct Osd {
    pub win: Id,
    pub kind: Kind,
    /// 0..1, jusqu'à 1,5 pour le volume.
    pub value: f32,
    pub muted: bool,
    pub shown_at: Instant,
}

/// `kind value muted` (ligne de commande du démon) → valeurs.
pub fn parse(line: &str) -> Option<(Kind, f32, bool)> {
    let mut it = line.split_whitespace();
    let kind = Kind::parse(it.next()?)?;
    let value: f32 = it.next()?.parse().ok().filter(|v: &f32| v.is_finite())?;
    let muted = it.next().is_some_and(|m| m == "1" || m == "true");
    Some((kind, value.clamp(0.0, 1.5), muted))
}

/// Icône d'interface pour cet état.
pub fn icon(kind: Kind, value: f32, muted: bool) -> &'static str {
    match kind {
        Kind::Volume if muted || value <= 0.0 => "audio-volume-muted-symbolic",
        Kind::Volume if value < 0.34 => "audio-volume-low-symbolic",
        Kind::Volume if value < 0.67 => "audio-volume-medium-symbolic",
        Kind::Volume => "audio-volume-high-symbolic",
        Kind::Mic if muted => "microphone-sensitivity-muted-symbolic",
        Kind::Mic => "audio-input-microphone-symbolic",
        Kind::Brightness => "display-brightness-symbolic",
        Kind::Kbd => "keyboard-brightness-symbolic",
    }
}

impl App {
    /// Affiche (ou rafraîchit) la carte ; elle s'efface `SHOW_FOR` après.
    pub(super) fn show_osd(&mut self, kind: Kind, value: f32, muted: bool) -> Task<Msg> {
        let name = icon(kind, value, muted);
        let want = self.want_icon(name);
        if let Some(o) = self.osd.as_mut() {
            o.kind = kind;
            o.value = value;
            o.muted = muted;
            o.shown_at = Instant::now();
            return want;
        }
        let win = Id::unique();
        self.osd = Some(Osd {
            win,
            kind,
            value,
            muted,
            shown_at: Instant::now(),
        });
        Task::batch([
            want,
            Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some((
                        (WIDTH + 2.0 * SHADOW) as u32,
                        (HEIGHT + 2.0 * SHADOW) as u32,
                    )),
                    layer: Layer::Overlay,
                    anchor: LsAnchor::Bottom,
                    exclusive_zone: Some(-1),
                    margin: Some((0, 0, LIFT, 0)),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    // L'écran actif, choisi par le compositeur.
                    output_option: OutputOption::Active,
                    namespace: Some(NAMESPACE.into()),
                    ..Default::default()
                },
                id: win,
            }),
        ])
    }

    /// Carte expirée : on ferme sa surface.
    pub(super) fn osd_tick(&mut self) -> Task<Msg> {
        match &self.osd {
            Some(o) if o.shown_at.elapsed() >= SHOW_FOR => {
                let win = o.win;
                self.osd = None;
                iced::window::close(win)
            }
            _ => Task::none(),
        }
    }

    pub(super) fn view_osd(&self) -> Element<'_, Msg> {
        let Some(o) = &self.osd else {
            return iced::widget::Space::new().into();
        };
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let accent = color(th.accent, 1.0);
        let bg = color(th.bg, th.effect(crate::config::Place::Menus).opacity);
        let dim = Color { a: 0.35, ..fg };
        let fill = if o.muted { dim } else { accent };
        let icon: Element<'_, Msg> = self
            .icon_named(icon(o.kind, o.value, o.muted), 26.0, fg)
            .unwrap_or_else(|| iced::widget::Space::new().width(26).into());
        // Jauge : 0..1 (au-delà de 100 %, le volume la remplit).
        let filled = (o.value.clamp(0.0, 1.0) * 1000.0).round() as u16;
        let bar = |portion: u16, c: Option<Color>| -> Element<'_, Msg> {
            container(iced::widget::Space::new())
                .width(Length::FillPortion(portion))
                .height(6)
                .style(move |_| container::Style {
                    background: c.map(Into::into),
                    border: Border::default().rounded(3),
                    ..Default::default()
                })
                .into()
        };
        let mut parts = row![];
        if filled > 0 {
            parts = parts.push(bar(filled, Some(fill)));
        }
        if filled < 1000 {
            parts = parts.push(bar(1000 - filled, None));
        }
        let gauge = container(parts)
            .width(Length::Fill)
            .height(6)
            .style(move |_| container::Style {
                background: Some(Color { a: 0.15, ..fg }.into()),
                border: Border::default().rounded(3),
                ..Default::default()
            });
        let label = if o.muted {
            crate::i18n::tr("muet").to_string()
        } else {
            format!("{:.0} %", o.value * 100.0)
        };
        let card = container(
            row![
                icon,
                gauge,
                text(label).font(self.font).size(14).color(fg).width(52),
            ]
            .spacing(14)
            .align_y(Alignment::Center),
        )
        .padding([0, 18])
        .width(WIDTH)
        .height(HEIGHT)
        .center_y(HEIGHT)
        .style(move |_| container::Style {
            background: Some(bg.into()),
            border: Border::default()
                .rounded(super::design::RADIUS)
                .width(1)
                .color(Color { a: 0.12, ..fg }),
            shadow: Shadow {
                color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
                offset: Vector::new(0.0, 8.0),
                blur_radius: 22.0,
            },
            text_color: Some(fg),
            ..Default::default()
        });
        container(card).padding(SHADOW).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ligne_du_demon() {
        assert_eq!(parse("volume 0.55 0"), Some((Kind::Volume, 0.55, false)));
        assert_eq!(parse("mic 0.8 1"), Some((Kind::Mic, 0.8, true)));
        assert_eq!(parse("volume 9 0"), Some((Kind::Volume, 1.5, false)));
        assert_eq!(parse("brightness NaN 0"), None);
        assert_eq!(parse("bass 0.5 0"), None);
    }

    #[test]
    fn icones() {
        assert_eq!(icon(Kind::Volume, 0.5, true), "audio-volume-muted-symbolic");
        assert_eq!(icon(Kind::Volume, 0.9, false), "audio-volume-high-symbolic");
        assert_eq!(
            icon(Kind::Mic, 0.5, true),
            "microphone-sensitivity-muted-symbolic"
        );
        for k in [Kind::Volume, Kind::Mic, Kind::Brightness, Kind::Kbd] {
            assert!(
                super::super::icons::get(icon(k, 0.5, false)).is_some(),
                "{k:?}"
            );
        }
    }
}
