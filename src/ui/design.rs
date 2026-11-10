//! Langage commun : surfaces, profondeur, rayons et couleur d'accent.
use iced::widget::container;
use iced::{Border, Color, Shadow, Theme, Vector};

pub const RADIUS: f32 = 20.0;
pub const SMALL_RADIUS: f32 = 10.0;

/// Verre du lanceur : la teinte reste issue du thème, le flou est fourni
/// par Hyprland sur les surfaces dédiées. Toutes les couches suivent le fondu
/// (`opacity`) ; `base` est l'opacité du fond réglée par le thème (0.88 livrée),
/// la bulle (`popup`) restant un peu plus couvrante.
pub fn launcher_surface(theme: &Theme, opacity: f32, popup: bool, base: f32) -> container::Style {
    let palette = theme.palette();
    let tint = palette.background;
    let light = |amount: f32, alpha: f32| Color {
        r: tint.r + (palette.text.r - tint.r) * amount,
        g: tint.g + (palette.text.g - tint.g) * amount,
        b: tint.b + (palette.text.b - tint.b) * amount,
        a: alpha * opacity,
    };
    let scale = |a: f32| (a * base / 0.88).clamp(0.0, 1.0);
    let gradient = iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2))
        .add_stop(0.0, light(0.065, scale(if popup { 0.96 } else { 0.88 })))
        .add_stop(0.45, light(0.025, scale(if popup { 0.95 } else { 0.84 })))
        .add_stop(1.0, light(0.0, scale(if popup { 0.96 } else { 0.88 })));
    container::Style {
        background: Some(iced::Background::Gradient(gradient.into())),
        border: Border::default()
            .rounded(if popup { 14.0 } else { RADIUS })
            .width(1)
            .color(Color {
                a: 0.13 * opacity,
                ..palette.text
            }),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.22 * opacity),
            offset: Vector::new(0.0, 3.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    }
}

pub fn wallpaper() -> container::Style {
    let gradient = iced::gradient::Linear::new(iced::Radians(0.7))
        .add_stop(0.0, Color::from_rgb8(0x1a, 0x30, 0x48))
        .add_stop(0.48, Color::from_rgb8(0x26, 0x25, 0x46))
        .add_stop(1.0, Color::from_rgb8(0x42, 0x28, 0x50));
    container::Style {
        background: Some(iced::Background::Gradient(gradient.into())),
        ..Default::default()
    }
}

#[derive(Clone, Copy)]
pub enum Surface {
    Floating,
    Panel,
    Inset,
}

/// Tiroir des réglages : `Surface::Panel` à l'opacité réglée par le thème.
pub fn panel_surface(theme: &Theme, opacity: f32) -> container::Style {
    let mut s = surface(theme, Surface::Panel);
    s.background = Some(
        Color {
            a: opacity,
            ..theme.palette().background
        }
        .into(),
    );
    s
}

pub fn surface(theme: &Theme, level: Surface) -> container::Style {
    let palette = theme.palette();
    let (opacity, edge, blur, y, radius) = match level {
        Surface::Floating => (0.86, 0.16, 28.0, 10.0, RADIUS),
        Surface::Panel => (0.97, 0.13, 40.0, 14.0, 24.0),
        Surface::Inset => (0.30, 0.07, 0.0, 0.0, 12.0),
    };
    container::Style {
        background: Some(
            Color {
                a: opacity,
                ..palette.background
            }
            .into(),
        ),
        text_color: Some(palette.text),
        border: Border::default().rounded(radius).width(1).color(Color {
            a: edge,
            ..palette.text
        }),
        shadow: Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, if blur > 0.0 { 0.28 } else { 0.0 }),
            offset: Vector::new(0.0, y),
            blur_radius: blur,
        },
        ..Default::default()
    }
}
