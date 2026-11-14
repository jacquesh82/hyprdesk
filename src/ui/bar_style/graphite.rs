//! Graphite : barre des tâches rectangulaire, matière uniforme et boutons plats.
//! Le rendu relief des espaces reste disponible pour le sélecteur optionnel.
use iced::widget::{Space, button, column, container, mouse_area, row};
use iced::{Alignment, Background, Border, Color, Element, Length, window::Id};

use super::super::bar::{BarPalette, BarSurface, scroll_dir};
use super::super::{App, Msg, bar_motion, design};
use super::{Tokens, WsCtx, color, urgent_mark};
use crate::config::Rgba;

pub(super) const TOKENS: Tokens = Tokens {
    hover: 140,
    press: 90,
    switch: 170,
};

/// Opacité de référence ; une valeur Lua explicite reste prioritaire.
pub(crate) const ACRYLIC: f32 = 0.94;
/// Rayon des touches et sièges gravés.
const KEY_RADIUS: f32 = 6.0;

/// Le gris du graphite : la luminance du thème, sans sa teinte. La lame est
/// neutre — le menu Démarrer de Windows 11 n'est jamais bleu parce que le
/// fond d'écran l'est.
pub(crate) fn neutral(c: Rgba) -> Rgba {
    let l = (0.2126 * c.0 as f32 + 0.7152 * c.1 as f32 + 0.0722 * c.2 as f32).round() as u8;
    Rgba(l, l, l, c.3)
}

/// Mélange monochrome de deux couleurs.
fn mix(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

/// Surface bord à bord ; le conducteur dessine son seul filet supérieur.
pub(super) fn surface(pal: &BarPalette, _top: bool) -> iced::widget::container::Style {
    let fg = color(pal.fg, 1.0);
    iced::widget::container::Style {
        background: Some(color(pal.bg, pal.opacity).into()),
        text_color: Some(fg),
        ..Default::default()
    }
}

/// Boutons plats, transparents au repos, sans siège ni relief permanent.
pub(super) fn module_button(
    alpha: f32,
    fg: Color,
    _bg: Color,
    active: bool,
    st: button::Status,
) -> button::Style {
    let fill = match st {
        button::Status::Pressed => 0.04,
        button::Status::Hovered => 0.09,
        _ if active => 0.07,
        _ => 0.0,
    };
    button::Style {
        background: Some(
            Color {
                a: fill * alpha,
                ..fg
            }
            .into(),
        ),
        text_color: fg,
        border: Border::default().rounded(4),
        ..Default::default()
    }
}

/// Fond d'une touche : gravée au niveau de la surface, ou surélevée avec son
/// arête claire (l'ombre et le reflet sont dessinés par `bar_motion::lift`).
fn key_style(raised: bool, bg: Color, fg: Color, hovered: bool) -> button::Style {
    if raised {
        let gradient = iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2))
            .add_stop(0.0, mix(bg, fg, 0.085))
            .add_stop(1.0, bg);
        button::Style {
            background: Some(Background::Gradient(gradient.into())),
            text_color: fg,
            border: Border::default()
                .rounded(KEY_RADIUS)
                .width(1)
                .color(Color { a: 0.28, ..fg }),
            ..Default::default()
        }
    } else {
        // Gravure : creux plus sombre en haut, la lumière tombe dedans.
        let gradient = iced::gradient::Linear::new(iced::Radians(std::f32::consts::FRAC_PI_2))
            .add_stop(
                0.0,
                mix(bg, Color::BLACK, if hovered { 0.14 } else { 0.20 }),
            )
            .add_stop(
                1.0,
                mix(bg, Color::BLACK, if hovered { 0.05 } else { 0.08 }),
            );
        button::Style {
            background: Some(Background::Gradient(gradient.into())),
            text_color: fg,
            border: Border::default().rounded(KEY_RADIUS).width(1).color(Color {
                a: 0.14,
                ..Color::BLACK
            }),
            ..Default::default()
        }
    }
}

/// Touches d'espaces de travail gravées : la touche active se soulève de
/// quelques pixels, l'ancienne redescend de sa hauteur affichée.
pub(super) fn workspaces<'a>(
    app: &'a App,
    bar_id: Id,
    bar: &'a BarSurface,
    ctx: &WsCtx,
    alpha: f32,
) -> Element<'a, Msg> {
    let pal = app.bar_palette();
    let fg = color(pal.fg, 1.0);
    let bg = color(pal.bg, 1.0);
    let size = (bar.height as f32 * 0.38).clamp(11.0, 18.0);
    let h = (bar.height as f32 * 0.62).round();
    let ink_num = |active: bool, windows: u32| -> Color {
        fg.scale_alpha(if active {
            1.0 * alpha
        } else if windows > 0 {
            0.78 * alpha
        } else {
            0.55 * alpha
        })
    };
    let keys: Vec<Element<'_, Msg>> = ctx
        .rows
        .iter()
        .map(|w| {
            let raised = w.name == ctx.current;
            let name = w.name.clone();
            let windows = w.windows;
            let ink = ink_num(raised, windows);
            // Le point plein d'un espace occupé, sous le numéro ; sa place est
            // réservée pour que les touches gardent leur hauteur.
            let dot =
                container(
                    Space::new()
                        .width(Length::Fixed(3.0))
                        .height(Length::Fixed(3.0)),
                )
                .style(move |_| container::Style {
                    background: (windows > 0).then_some(Background::Color(
                        fg.scale_alpha(if raised { 1.0 * alpha } else { 0.75 * alpha }),
                    )),
                    border: Border::default().rounded(1.5),
                    ..Default::default()
                });
            let mut head = row![app.label(name.clone(), size * 0.92, ink)]
                .spacing(4)
                .align_y(Alignment::Center);
            if w.urgent
                && let Some(mark) = urgent_mark(app, bar, size * 0.5, fg.scale_alpha(0.9 * alpha))
            {
                head = head.push(mark);
            }
            let content = column![head, dot].spacing(2).align_x(Alignment::Center);
            bar_motion::lift(
                button(container(content).center(Length::Shrink))
                    .height(h)
                    .padding([0, if name.chars().count() > 1 { 8 } else { 0 }])
                    .width(if name.chars().count() > 1 {
                        Length::Shrink
                    } else {
                        Length::Fixed(h)
                    })
                    .on_press(Msg::BarWorkspace(ctx.monitor.clone(), name))
                    .style(move |_, st| key_style(raised, bg, fg, st == button::Status::Hovered)),
                raised,
                fg,
                app.bar_duration(TOKENS.switch),
            )
        })
        .collect();
    let mut group = row(keys).spacing(4).align_y(Alignment::Center);
    if ctx.folded {
        group = group.push(
            button(app.label("…", size, fg.scale_alpha(0.78 * alpha)))
                .padding([0, 8])
                .height(h)
                .on_press(Msg::OpenPopover(
                    bar_id,
                    "workspaces".into(),
                    crate::ui::popover::Request::Workspaces,
                ))
                .style(move |_, st| button::Style {
                    background: matches!(st, button::Status::Hovered | button::Status::Pressed)
                        .then_some(
                            Color {
                                a: 0.08 * alpha,
                                ..fg
                            }
                            .into(),
                        ),
                    text_color: fg,
                    border: Border::default().rounded(design::SMALL_RADIUS),
                    ..Default::default()
                }),
        );
    }
    let mon = ctx.monitor.clone();
    mouse_area(container(group).padding([0, 4]))
        .on_scroll(move |d| Msg::BarScroll(mon.clone(), -scroll_dir(d) as i32))
        .into()
}
