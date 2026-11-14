//! Style « Continuité » : le rendu d'origine de la barre — verre fumé dérivé
//! du thème, capsule de survol, capsule glissante sur l'espace actif
//! (`design-plans/barre-continuite.md`). Extrait tel quel de `ui::bar` le jour
//! où les styles sont devenus un choix ; comportement inchangé.
use iced::widget::{Space, button, container, mouse_area, row};
use iced::{Alignment, Background, Border, Color, Element, Length, window::Id};

use super::super::bar::{BarPalette, BarSurface, scroll_dir};
use super::super::{App, Msg, bar_motion, design};
use super::{Tokens, WsCtx, color, urgent_mark};

pub(super) const TOKENS: Tokens = Tokens {
    hover: 140,
    press: 90,
    switch: 180,
};

/// Verre fumé : reflet issu du texte du thème, filet d'un pixel côté fenêtres.
pub(super) fn surface(pal: &BarPalette, top: bool) -> iced::widget::container::Style {
    design::bar_surface(color(pal.bg, pal.opacity), color(pal.fg, 1.0), top)
}

/// Capsules : teinte du texte posée par-dessus le fond, plus forte à l'appui.
pub(super) fn module_button(
    alpha: f32,
    fg: Color,
    active: bool,
    st: button::Status,
) -> button::Style {
    let a = match st {
        button::Status::Pressed => 0.18,
        _ if active => 0.12,
        _ => 0.0,
    };
    button::Style {
        background: (a > 0.0).then_some(Background::Color(Color { a: a * alpha, ..fg })),
        text_color: fg,
        border: Border::default().rounded(design::SMALL_RADIUS),
        ..Default::default()
    }
}

/// Pills d'espaces de travail sous une capsule glissante d'accent.
pub(super) fn workspaces<'a>(
    app: &'a App,
    bar_id: Id,
    bar: &'a BarSurface,
    ctx: &WsCtx,
    alpha: f32,
) -> Element<'a, Msg> {
    let pal = app.bar_palette();
    let accent = color(pal.accent, alpha);
    let selected = ctx.rows.iter().position(|w| w.name == ctx.current);
    let fg = color(pal.fg, 0.92 * alpha);
    let dim = color(pal.fg, 0.65 * alpha);
    let base = app
        .bar_item_color(&crate::config::BarItem::Workspaces)
        .unwrap_or(pal.fg);
    let fgc = color(base, alpha);
    let size = (bar.height as f32 * 0.38).clamp(11.0, 18.0);
    let mut pills: Vec<Element<'_, Msg>> = ctx
        .rows
        .iter()
        .map(|w| {
            let active = w.name == ctx.current;
            let txt = if active || w.windows > 0 { fg } else { dim };
            let h = (bar.height as f32 * 0.6).round();
            let mut content = row![app.label(w.name.clone(), size * 0.92, txt)]
                .spacing(4)
                .align_y(Alignment::Center);
            if w.urgent
                && let Some(mark) = urgent_mark(app, bar, size * 0.5, color(pal.warning, alpha))
            {
                content = content.push(mark);
            }
            bar_motion::press(
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
                    Length::Fixed(h)
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
                    border: Border::default().rounded(h / 2.0),
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
                    crate::ui::popover::Request::Workspaces,
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
    let selected = selected.map(|i| i * 2);
    mouse_area(
        container(bar_motion::rail(
            row(segments).spacing(3).align_y(Alignment::Center),
            selected,
            Vec::new(),
            bar_motion::RailLook::Fill(Color {
                a: 0.27 * alpha,
                ..accent
            }),
            app.bar_duration(TOKENS.switch),
        ))
        .padding([0, 4]),
    )
    .on_scroll(move |d| Msg::BarScroll(mon.clone(), -scroll_dir(d) as i32))
    .into()
}
