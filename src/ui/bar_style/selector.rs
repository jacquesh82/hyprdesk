//! Modes de contenu du sélecteur de workspaces (`workspace_selector`) :
//! « miniatures » — la minimap des fenêtres de chaque espace — et « icônes
//! d'activité » — l'icône de l'application au premier plan. Les modes
//! « relief » et « rail » reprennent les rendus natifs des styles de barre
//! (touches soulevées en Graphite, capsule glissante en Continuité).
use iced::widget::{Space, column, container, mouse_area, row, stack, text, tooltip};
use iced::{Alignment, Background, Border, Color, Element, Length, window::Id};

use super::super::bar::{BarSurface, scroll_dir};
use super::super::{App, IconImg, Msg, bar_motion};
use super::{WsCtx, WsRow, color, urgent_mark};

/// Minimap des fenêtres de chaque espace : un bloc à sa place réelle.
pub(super) fn miniatures<'a>(
    app: &'a App,
    bar_id: Id,
    bar: &'a BarSurface,
    ctx: &WsCtx,
    alpha: f32,
) -> Element<'a, Msg> {
    let pal = app.bar_palette();
    let fg = color(pal.fg, 0.92 * alpha);
    let dim = color(pal.fg, 0.65 * alpha);
    let accent = color(pal.accent, alpha);
    let warning = color(pal.warning, alpha);
    let size = (bar.height as f32 * 0.38).clamp(11.0, 18.0);
    let h = (bar.height as f32 * 0.64).round();
    let press = app.bar_duration(super::durations(app).press);
    let chips: Vec<Element<'_, Msg>> = ctx
        .rows
        .iter()
        .map(|w| {
            let active = w.name == ctx.current;
            // Une application de l'espace vient d'émettre une notification :
            // le bord passe à la couleur d'alerte, tant que la bulle vit.
            let notif = app.bar_notif(&ctx.monitor, &w.name);
            let inner_w = ((h - 4.0) * 1.6).round().max(24.0);
            let inner_h = h - 16.0;
            // La minimap : les fenêtres de l'espace à leur place (0..1).
            let mut map = stack![];
            for (r, _win) in app.overview_windows(&w.name) {
                let left = r.x * inner_w;
                let top = r.y * inner_h;
                let width = (r.width * inner_w).max(6.0);
                let height = (r.height * inner_h).max(4.0);
                let block = container(
                    Space::new()
                        .width(Length::Fixed(width))
                        .height(Length::Fixed(height)),
                )
                .style(move |_| container::Style {
                    background: Some(
                        Color {
                            a: 0.30 * alpha,
                            ..fg
                        }
                        .into(),
                    ),
                    border: Border::default().rounded(2).width(1).color(Color {
                        a: 0.18 * alpha,
                        ..fg
                    }),
                    ..Default::default()
                });
                map = map.push(
                    container(block)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .padding(iced::Padding {
                            top,
                            left,
                            right: (inner_w - left - width).max(0.0),
                            bottom: (inner_h - top - height).max(0.0),
                        }),
                );
            }
            // Le nom de l'espace au-dessus, la minimap en dessous.
            let head = text(w.name.clone())
                .font(app.font)
                .size(9)
                .color(if active { accent } else { dim });
            let chip = bar_motion::press(
                iced::widget::button(
                    container(
                        column![
                            container(head)
                                .width(Length::Fixed(inner_w))
                                .center_x(Length::Fill)
                                .height(11.0),
                            container(map)
                                .width(Length::Fixed(inner_w))
                                .height(Length::Fill),
                        ]
                        .spacing(1),
                    )
                    .padding(2)
                    .width(Length::Fixed(inner_w + 4.0))
                    .height(Length::Fixed(h)),
                )
                .on_press(Msg::BarWorkspace(ctx.monitor.clone(), w.name.clone()))
                .style(move |_, st| iced::widget::button::Style {
                    background: Some(
                        if matches!(st, iced::widget::button::Status::Hovered) {
                            Color {
                                a: 0.14 * alpha,
                                ..fg
                            }
                        } else {
                            Color {
                                a: 0.10 * alpha,
                                ..fg
                            }
                        }
                        .into(),
                    ),
                    border: Border::default()
                        .rounded(4)
                        .width(if notif || active { 1.5 } else { 1.0 })
                        .color(if notif {
                            warning
                        } else if active {
                            accent
                        } else {
                            Color {
                                a: 0.22 * alpha,
                                ..fg
                            }
                        }),
                    ..Default::default()
                }),
                press,
            );
            with_urgent(app, bar, w, alpha, chip)
        })
        .collect();
    ws_row(app, bar_id, bar, ctx, alpha, chips, fg, size)
}

/// Icônes d'activité : l'icône de la première fenêtre de chaque espace, un
/// carré vide sinon ; l'active soulignée d'accent, info-bulle « N · Nom ».
pub(super) fn icones<'a>(
    app: &'a App,
    bar_id: Id,
    bar: &'a BarSurface,
    ctx: &WsCtx,
    alpha: f32,
) -> Element<'a, Msg> {
    let pal = app.bar_palette();
    let fg = color(pal.fg, 0.92 * alpha);
    let accent = color(pal.accent, alpha);
    let warning = color(pal.warning, alpha);
    let size = (bar.height as f32 * 0.38).clamp(11.0, 18.0);
    let isz = (size * 1.25).round();
    let h = (bar.height as f32 * 0.66).round();
    let press = app.bar_duration(super::durations(app).press);
    let chips: Vec<Element<'_, Msg>> = ctx
        .rows
        .iter()
        .map(|w| {
            let active = w.name == ctx.current;
            let class = app
                .clients
                .iter()
                .find(|c| {
                    c.workspace == w.name && (c.monitor.is_empty() || c.monitor == ctx.monitor)
                })
                .map(|c| c.class.clone());
            let img = class
                .as_ref()
                .and_then(|c| app.class_imgs.get(c))
                .copied()
                .unwrap_or(IconImg::None);
            let glyph: Element<'_, Msg> = app
                .img_element(
                    img,
                    isz,
                    if active || w.windows > 0 {
                        alpha
                    } else {
                        alpha * 0.75
                    },
                )
                .unwrap_or_else(|| {
                    // Aucune fenêtre : un carré vide, comme un espace libre.
                    container(
                        Space::new()
                            .width(Length::Fixed(isz))
                            .height(Length::Fixed(isz)),
                    )
                    .style(move |_| container::Style {
                        border: Border::default().rounded(3).width(1.5).color(Color {
                            a: 0.38 * alpha,
                            ..fg
                        }),
                        ..Default::default()
                    })
                    .into()
                });
            // Soulignement : accent pour l'active, couleur d'alerte quand une
            // application de l'espace émet une notification (elle prime).
            // Sa place est réservée dans tous les cas.
            let notif = app.bar_notif(&ctx.monitor, &w.name);
            let underline = container(
                Space::new()
                    .width(Length::Fixed(14.0))
                    .height(Length::Fixed(2.0)),
            )
            .style(move |_| container::Style {
                background: if notif {
                    Some(Background::Color(warning))
                } else if active {
                    Some(Background::Color(accent))
                } else {
                    None
                },
                border: Border::default().rounded(1.0),
                ..Default::default()
            });
            let chip = bar_motion::press(
                iced::widget::button(
                    container(
                        column![glyph, underline]
                            .spacing(3)
                            .align_x(Alignment::Center),
                    )
                    .center_x(Length::Shrink)
                    .center_y(Length::Fill),
                )
                .height(h)
                .padding([0, 4])
                .on_press(Msg::BarWorkspace(ctx.monitor.clone(), w.name.clone()))
                .style(move |_, st| iced::widget::button::Style {
                    background: matches!(st, iced::widget::button::Status::Hovered).then_some(
                        Background::Color(Color {
                            a: 0.12 * alpha,
                            ..fg
                        }),
                    ),
                    text_color: fg,
                    border: Border::default().rounded(6),
                    ..Default::default()
                }),
                press,
            );
            let label = if w.name == w.id.to_string() {
                w.name.clone()
            } else {
                format!("{} · {}", w.id, w.name)
            };
            with_urgent(app, bar, w, alpha, bar_tooltip(app, bar, chip, label))
        })
        .collect();
    ws_row(app, bar_id, bar, ctx, alpha, chips, fg, size)
}

/// Info-bulle sous la barre (au-dessus si la barre est en bas).
fn bar_tooltip<'a>(
    app: &'a App,
    bar: &'a BarSurface,
    content: Element<'a, Msg>,
    label: String,
) -> Element<'a, Msg> {
    let pal = app.bar_palette();
    let bg = color(pal.bg, 0.96);
    let fg = color(pal.fg, 1.0);
    tooltip(
        content,
        container(text(label).font(app.font).size(12))
            .padding([4, 8])
            .style(move |_| container::Style {
                background: Some(bg.into()),
                border: Border::default()
                    .rounded(8)
                    .width(1)
                    .color(Color { a: 0.12, ..fg }),
                text_color: Some(fg),
                ..Default::default()
            }),
        if bar.position == crate::config::BarPosition::Top {
            tooltip::Position::Bottom
        } else {
            tooltip::Position::Top
        },
    )
    .gap(5)
    .delay(std::time::Duration::from_millis(200))
    .into()
}

/// Le losange « fenêtre demandant l'attention », posé à côté du contenu.
fn with_urgent<'a>(
    app: &'a App,
    bar: &'a BarSurface,
    w: &WsRow,
    alpha: f32,
    content: Element<'a, Msg>,
) -> Element<'a, Msg> {
    if !w.urgent {
        return content;
    }
    let pal = app.bar_palette();
    let Some(mark) = urgent_mark(app, bar, 10.0, color(pal.warning, alpha)) else {
        return content;
    };
    row![content, mark]
        .spacing(2)
        .align_y(Alignment::Center)
        .into()
}

/// Rangée finale : les chips, le « … » si replié, la molette pour défiler.
#[allow(clippy::too_many_arguments)]
fn ws_row<'a>(
    app: &'a App,
    bar_id: Id,
    bar: &'a BarSurface,
    ctx: &WsCtx,
    alpha: f32,
    mut chips: Vec<Element<'a, Msg>>,
    fg: Color,
    size: f32,
) -> Element<'a, Msg> {
    if ctx.folded {
        chips.push(
            iced::widget::button(app.label("…", size, fg))
                .padding([0, 8])
                .height((bar.height as f32 * 0.6).round())
                .on_press(Msg::OpenPopover(
                    bar_id,
                    "workspaces".into(),
                    crate::ui::popover::Request::Workspaces,
                ))
                .style(move |_, st| iced::widget::button::Style {
                    background: matches!(
                        st,
                        iced::widget::button::Status::Hovered
                            | iced::widget::button::Status::Pressed
                    )
                    .then_some(
                        Color {
                            a: 0.10 * alpha,
                            ..fg
                        }
                        .into(),
                    ),
                    text_color: fg,
                    border: Border::default().rounded(6),
                    ..Default::default()
                })
                .into(),
        );
    }
    let mon = ctx.monitor.clone();
    mouse_area(row(chips).spacing(4).align_y(Alignment::Center))
        .on_scroll(move |d| Msg::BarScroll(mon.clone(), -scroll_dir(d) as i32))
        .into()
}
