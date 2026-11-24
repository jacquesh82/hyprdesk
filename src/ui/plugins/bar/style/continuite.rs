//! Déclinaisons du thème pour la barre Continuité et son centre de contrôle.
use iced::widget::{Space, button, container, mouse_area, pin, row};
use iced::{Alignment, Background, Border, Color, Element, Length, window::Id};

use super::{Tokens, WsCtx, color, urgent_mark};
use crate::config::BarStyle;
use crate::ui::plugins::bar::motion;
use crate::ui::plugins::bar::surface::{BarPalette, BarSurface, scroll_dir};
use crate::ui::{App, Msg, design};

pub(super) const TOKENS: Tokens = Tokens {
    hover: 140,
    press: 90,
    switch: 180,
};

pub(super) fn surface(pal: &BarPalette, top: bool) -> container::Style {
    let radius = if top {
        iced::border::bottom(11)
    } else {
        iced::border::top(11)
    };
    container::Style {
        background: Some(frame(pal, pal.opacity).into()),
        border: Border::default().rounded(radius),
        text_color: Some(color(pal.fg, 1.0)),
        ..Default::default()
    }
}

pub(super) fn groups<'a>(
    pal: &BarPalette,
    height: f32,
    extents: [(f32, f32); 3],
) -> Element<'a, Msg> {
    use crate::ui::plugins::bar::layout::GROUP_INSET;

    let fill = color(pal.bg, pal.opacity);
    let height = (height - 2.0 * vertical_padding(height)).max(0.0);
    let mut groups = iced::widget::stack![Space::new().width(Length::Fill).height(Length::Fill)];
    for (left, right) in extents {
        if right > left {
            let group = container(Space::new())
                .width(right - left + 2.0 * GROUP_INSET)
                .height(height)
                .style(move |_| container::Style {
                    background: Some(fill.into()),
                    border: Border::default().rounded((height / 2.0).min(13.0)),
                    ..Default::default()
                });
            groups = groups.push(pin(group).x(left - GROUP_INSET));
        }
    }
    groups.into()
}

pub(in crate::ui) fn vertical_padding(height: f32) -> f32 {
    if height < 32.0 { 2.0 } else { 3.0 }
}

pub(in crate::ui) fn frame(pal: &BarPalette, alpha: f32) -> Color {
    tone(pal, 0.16, alpha)
}

pub(in crate::ui) fn tone(pal: &BarPalette, amount: f64, alpha: f32) -> Color {
    let mut tint = crate::theme::mix(pal.bg, pal.fg, amount);
    tint.3 = pal.bg.3;
    color(tint, alpha)
}

pub(in crate::ui) fn selected_ink(pal: &BarPalette, alpha: f32) -> Color {
    let accent = color(pal.accent, 1.0);
    let ink = design::on_accent(accent, color(pal.fg, 1.0), color(pal.bg, 1.0));
    let ink = if accent.relative_contrast(ink) >= 4.5 {
        ink
    } else {
        design::on_accent(accent, Color::WHITE, Color::BLACK)
    };
    Color { a: alpha, ..ink }
}

pub(super) fn module_button(
    pal: &BarPalette,
    alpha: f32,
    active: bool,
    st: button::Status,
) -> button::Style {
    let accent = color(pal.accent, alpha);
    let background = match (active, st) {
        (true, button::Status::Pressed) => Some(accent.scale_alpha(0.82).into()),
        (true, _) => Some(accent.into()),
        (false, button::Status::Pressed) => Some(tone(pal, 0.20, alpha).into()),
        _ => None,
    };
    button::Style {
        background,
        text_color: if active {
            selected_ink(pal, alpha)
        } else {
            color(pal.fg, alpha)
        },
        border: Border::default().rounded(design::SMALL_RADIUS),
        ..Default::default()
    }
}

/// Rail configuré avec des touches séparées et sa capsule glissante.
pub(super) fn workspaces<'a>(
    app: &'a App,
    bar_id: Id,
    bar: &'a BarSurface,
    ctx: &WsCtx,
    alpha: f32,
) -> Element<'a, Msg> {
    let pal = app.bar_palette();
    let continuity = app
        .cfg
        .bar
        .as_ref()
        .is_some_and(|b| b.style == BarStyle::Continuite);
    let accent = color(pal.accent, alpha);
    let selected = ctx.rows.iter().position(|w| w.name == ctx.current);
    let base = app
        .bar_item_color(&crate::config::BarItem::Workspaces)
        .unwrap_or(pal.fg);
    let fg = color(base, 0.92 * alpha);
    let dim = color(base, 0.65 * alpha);
    let fgc = color(base, alpha);
    let size = (bar.height as f32 * 0.38).clamp(11.0, 18.0);
    let mut pills: Vec<Element<'_, Msg>> = ctx
        .rows
        .iter()
        .map(|w| {
            let active = w.name == ctx.current;
            let txt = if active && continuity {
                selected_ink(&pal, alpha)
            } else if active || w.windows > 0 {
                fg
            } else {
                dim
            };
            let h = (bar.height as f32 * 0.6).round();
            let mut content = row![app.label(w.name.clone(), size * 0.92, txt)]
                .spacing(4)
                .align_y(Alignment::Center);
            if w.urgent
                && let Some(mark) = urgent_mark(app, bar, size * 0.5, color(pal.warning, alpha))
            {
                content = content.push(mark);
            }
            motion::press(
                button(
                    container(content)
                        .center_x(Length::Shrink)
                        .center_y(Length::Fill),
                )
                .height(h)
                .padding([0, if w.name.chars().count() > 1 { 8 } else { 0 }])
                .width(if w.name.chars().count() > 1 {
                    Length::Shrink
                } else {
                    Length::Fixed(h + 4.0)
                })
                .on_press(Msg::BarWorkspace(ctx.monitor.clone(), w.name.clone()))
                .style(move |_, st| button::Style {
                    background: matches!(st, button::Status::Hovered).then_some(Background::Color(
                        Color {
                            a: 0.12 * alpha,
                            ..fgc
                        },
                    )),
                    text_color: txt,
                    border: Border::default().rounded(motion::key_radius(h)),
                    ..Default::default()
                }),
                app.bar_duration(TOKENS.press),
            )
        })
        .collect();
    if ctx.folded {
        pills.push(
            button(app.label("…", size, fg))
                .padding([0, 8])
                .height((bar.height as f32 * 0.6).round())
                .on_press(Msg::OpenPopover(
                    bar_id,
                    "workspaces".into(),
                    crate::ui::plugins::bar::popover::Request::Workspaces,
                ))
                .style(move |_, st| button::Style {
                    background: matches!(st, button::Status::Hovered | button::Status::Pressed)
                        .then_some(
                            Color {
                                a: 0.10 * alpha,
                                ..fg
                            }
                            .into(),
                        ),
                    text_color: fg,
                    border: Border::default().rounded(design::SMALL_RADIUS),
                    ..Default::default()
                })
                .into(),
        );
    }
    let mon = ctx.monitor.clone();
    let h = (bar.height as f32 * 0.6).round();
    // Rail segmenté : un filet entre les touches (aucun si replié). Le filet
    // compte comme un enfant du rail : l'indice de la capsule suit.
    let mut segments: Vec<Element<'_, Msg>> = Vec::new();
    for (i, pill) in pills.into_iter().enumerate() {
        if i > 0 && !ctx.folded {
            segments.push(
                container(Space::new().width(1.0).height(h * 0.5))
                    .style(move |_| container::Style {
                        background: Some(
                            Color {
                                a: 0.16 * alpha,
                                ..fgc
                            }
                            .into(),
                        ),
                        ..Default::default()
                    })
                    .into(),
            );
        }
        segments.push(pill);
    }
    let selected = selected.map(|i| if ctx.folded { i } else { i * 2 });
    mouse_area(
        container(motion::rail(
            row(segments).spacing(3).align_y(Alignment::Center),
            selected,
            Vec::new(),
            motion::RailLook::Fill(Color {
                a: if continuity { alpha } else { 0.27 * alpha },
                ..accent
            }),
            app.bar_duration(TOKENS.switch),
        ))
        .padding([0, 4]),
    )
    .on_scroll(move |d| Msg::BarScroll(mon.clone(), -scroll_dir(d) as i32))
    .into()
}
