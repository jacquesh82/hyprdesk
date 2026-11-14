//! Langage commun : surfaces, profondeur, rayons et couleur d'accent.
use iced::widget::container;
use iced::{Border, Color, Shadow, Theme, Vector};

pub const RADIUS: f32 = 20.0;
pub const SMALL_RADIUS: f32 = 10.0;

/// Encre issue du thème qui reste lisible sur son accent, clair ou sombre.
pub fn on_accent(accent: Color, fg: Color, bg: Color) -> Color {
    if accent.relative_contrast(fg) >= accent.relative_contrast(bg) {
        fg
    } else {
        bg
    }
}

/// Barre et cartes associées : un reflet issu du texte du thème, jamais une
/// teinte fixe ; pas d'animation de flou ou de géométrie de la surface.
pub fn bar_surface(bg: Color, fg: Color, top: bool) -> container::Style {
    let lifted = Color {
        r: bg.r + (fg.r - bg.r) * 0.025,
        g: bg.g + (fg.g - bg.g) * 0.025,
        b: bg.b + (fg.b - bg.b) * 0.025,
        ..bg
    };
    let gradient = iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2))
        .add_stop(0.0, lifted)
        .add_stop(1.0, bg);
    container::Style {
        background: Some(iced::Background::Gradient(gradient.into())),
        text_color: Some(fg),
        shadow: Shadow {
            color: Color {
                a: 0.08 * bg.a,
                ..fg
            },
            offset: Vector::new(0.0, if top { 1.0 } else { -1.0 }),
            blur_radius: 0.0,
        },
        ..Default::default()
    }
}

pub fn bar_card(bg: Color, fg: Color) -> container::Style {
    let mut style = bar_surface(bg, fg, true);
    style.border = Border::default()
        .rounded(RADIUS)
        .width(1)
        .color(Color { a: 0.10, ..fg });
    style.shadow = Shadow {
        color: Color::from_rgba(0.0, 0.0, 0.0, 0.27),
        offset: Vector::new(0.0, 8.0),
        blur_radius: 22.0,
    };
    style
}

pub fn bar_slider(
    theme: &Theme,
    status: iced::widget::slider::Status,
) -> iced::widget::slider::Style {
    let mut style = iced::widget::slider::default(theme, status);
    style.rail.width = 10.0;
    style.rail.backgrounds = (
        theme.palette().primary.into(),
        Color {
            a: 0.10,
            ..theme.palette().text
        }
        .into(),
    );
    style.rail.border = Border::default().rounded(5);
    style.handle.shape = iced::widget::slider::HandleShape::Circle { radius: 8.0 };
    style.handle.border_width = 0.0;
    style.handle.background = theme.palette().text.into();
    style
}

pub fn bar_slider_alpha(
    theme: &Theme,
    status: iced::widget::slider::Status,
    alpha: f32,
) -> iced::widget::slider::Style {
    let mut style = bar_slider(theme, status);
    style.rail.backgrounds.0 = style.rail.backgrounds.0.scale_alpha(alpha);
    style.rail.backgrounds.1 = style.rail.backgrounds.1.scale_alpha(alpha);
    style.handle.background = style.handle.background.scale_alpha(alpha);
    style.handle.border_color.a *= alpha;
    style
}

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
