//! Convex : niches et tiroirs raccordés au même cadre, inspirés de minflair.
use super::{Tokens, color};
use crate::config::ConvexDef;
use crate::ui::Msg;
use crate::ui::plugins::bar::surface::BarPalette;
use iced::mouse;
use iced::widget::{button, canvas, container};
use iced::{Color, Element, Length, Point, Rectangle, Size};
use std::cell::RefCell;

pub(super) const TOKENS: Tokens = Tokens {
    hover: 140,
    press: 90,
    switch: 180,
};
const INSET: f32 = 12.0;
const BRIDGE: f32 = 4.0;

pub(super) fn surface(pal: &BarPalette) -> container::Style {
    container::Style {
        text_color: Some(color(pal.fg, 1.0)),
        ..Default::default()
    }
}
pub(super) fn module_button(
    alpha: f32,
    fg: Color,
    bg: Color,
    active: bool,
    st: button::Status,
) -> button::Style {
    super::graphite::module_button(alpha, fg, bg, active, st)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(in crate::ui) enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq)]
enum Shape {
    Bar {
        height: f32,
        extents: [(f32, f32); 3],
        top: bool,
    },
    Frame {
        bezel: f32,
        radius: f32,
        bar_height: f32,
        top: bool,
    },
    Drawer {
        edge: Edge,
        progress: f32,
        radius: f32,
    },
}

pub(super) fn underlay<'a>(
    pal: &BarPalette,
    height: f32,
    extents: [(f32, f32); 3],
    top: bool,
) -> Element<'a, Msg> {
    paint(
        pal,
        Shape::Bar {
            height,
            extents,
            top,
        },
    )
}
pub(in crate::ui) fn frame<'a>(
    pal: &BarPalette,
    options: ConvexDef,
    bar_height: f32,
    top: bool,
) -> Element<'a, Msg> {
    paint(
        pal,
        Shape::Frame {
            bezel: options.frame_width as f32,
            radius: options.radius as f32,
            bar_height,
            top,
        },
    )
}
pub(in crate::ui) fn drawer<'a>(pal: &BarPalette, edge: Edge, progress: f32) -> Element<'a, Msg> {
    paint(
        pal,
        Shape::Drawer {
            edge,
            progress: progress.clamp(0.0, 1.0),
            radius: 24.0,
        },
    )
}
fn paint<'a>(pal: &BarPalette, shape: Shape) -> Element<'a, Msg> {
    let g = crate::ui::tokens::glass(crate::config::Place::Bar);
    canvas(Skin {
        fill: color(pal.bg, pal.opacity),
        shape,
        light: (g.highlight, g.refraction),
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
struct Skin {
    fill: Color,
    shape: Shape,
    /// Verre du thème (MLG-274) : reflet et réfraction.
    light: (f32, f32),
}

/// Verre du thème sur une niche : reflet venu d'en haut dans la forme,
/// liserés décomposés sur son contour.
fn glass(frame: &mut canvas::Frame, path: &canvas::Path, light: (f32, f32)) {
    let (highlight, refraction) = light;
    let h = frame.size().height;
    if highlight > 0.0 {
        let gradient = canvas::gradient::Linear::new(Point::ORIGIN, Point::new(0.0, h * 0.5))
            .add_stop(0.0, Color::from_rgba(1.0, 1.0, 1.0, highlight * 0.55))
            .add_stop(0.35, Color::from_rgba(1.0, 1.0, 1.0, highlight * 0.18))
            .add_stop(1.0, Color::TRANSPARENT);
        frame.fill(path, canvas::Fill::from(canvas::Gradient::Linear(gradient)));
    }
    if refraction > 0.0 {
        frame.stroke(
            path,
            canvas::Stroke::default()
                .with_width(1.0)
                .with_color(Color::from_rgba(0.55, 0.85, 1.0, 0.30 * refraction)),
        );
    }
}
/// Ce qui redessine la forme : couleur, géométrie, verre.
type SkinKey = (Color, Shape, (f32, f32));

#[derive(Default)]
pub(in crate::ui) struct State {
    cache: canvas::Cache,
    key: RefCell<Option<SkinKey>>,
}
impl canvas::Program<Msg> for Skin {
    type State = State;
    fn draw(
        &self,
        state: &State,
        renderer: &iced::Renderer,
        _: &iced::Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let key = (self.fill, self.shape, self.light);
        if *state.key.borrow() != Some(key) {
            state.cache.clear();
            *state.key.borrow_mut() = Some(key);
        }
        vec![state.cache.draw(renderer, bounds.size(), |frame| {
            let size = frame.size();
            match self.shape {
                Shape::Bar {
                    height,
                    extents,
                    top,
                } => {
                    let path = bar_path(size.width, height.min(size.height), extents, top);
                    frame.fill(&path, self.fill);
                    glass(frame, &path, self.light);
                }
                Shape::Frame {
                    bezel,
                    radius,
                    bar_height,
                    top,
                } => {
                    if bezel <= 0.0 {
                        return;
                    }
                    let b = bezel.min(size.width * 0.5).min(size.height * 0.5);
                    let mut p = canvas::path::Builder::new();
                    let bar_height = (bar_height - 4.0).max(1.0).min(size.height);
                    let start = if top { bar_height } else { 0.0 };
                    p.rectangle(
                        Point::new(0.0, start),
                        Size::new(size.width, (size.height - bar_height).max(0.0)),
                    );
                    p.rounded_rectangle(
                        Point::new(b, if top { bar_height } else { b }),
                        Size::new(
                            (size.width - 2.0 * b).max(0.0),
                            (size.height - bar_height - b).max(0.0),
                        ),
                        radius.into(),
                    );
                    frame.fill(
                        &p.build(),
                        canvas::Fill {
                            style: self.fill.into(),
                            rule: canvas::fill::Rule::EvenOdd,
                        },
                    );
                }
                Shape::Drawer {
                    edge,
                    progress,
                    radius,
                } => {
                    let path = drawer_path(size, edge, progress, radius);
                    frame.fill(&path, self.fill);
                    glass(frame, &path, self.light);
                }
            }
        })]
    }
}

/// Zones vides conservées : une horloge seule reste une niche centrale, et
/// ne se transforme jamais en une lame pleine largeur. Les ponts insuffisants
/// fusionnent sans path qui se croise, même avec des valeurs hors de l'écran.
fn slots(width: f32, extents: [(f32, f32); 3], flare: f32) -> Vec<(f32, f32)> {
    let mut zones: Vec<_> = extents
        .into_iter()
        .enumerate()
        .filter_map(|(zone, (a, b))| {
            if !a.is_finite() || !b.is_finite() || b <= a {
                return None;
            }
            let a = if zone == 0 {
                0.0
            } else {
                (a - INSET).clamp(0.0, width)
            };
            let b = if zone == 2 {
                width
            } else {
                (b + INSET).clamp(0.0, width)
            };
            (b > a).then_some((a, b))
        })
        .collect();
    zones.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut merged: Vec<(f32, f32)> = Vec::new();
    for (a, b) in zones {
        if let Some(last) = merged.last_mut()
            && a - last.1 < flare * 2.0 + 8.0
        {
            last.1 = last.1.max(b);
        } else {
            merged.push((a, b));
        }
    }
    merged
}

fn bar_path(width: f32, height: f32, extents: [(f32, f32); 3], top: bool) -> canvas::Path {
    // La hauteur réglée redimensionne toute la silhouette, au lieu de rogner
    // les courbes à 44 px ou de laisser les modules dépasser de leur niche.
    let h = (height - 4.0).max(1.0);
    let scale = h / 44.0;
    let bridge = BRIDGE.min(h);
    let fw = 26.0 * scale;
    let fh = (20.0 * scale).min((h - bridge) * 0.5);
    let y = |v: f32| if top { v } else { height - v };
    let mut p = canvas::path::Builder::new();
    p.move_to(Point::new(0.0, y(0.0)));
    p.line_to(Point::new(0.0, y(bridge)));
    for (a, b) in slots(width, extents, fw) {
        let r = (20.0 * scale).min((b - a) * 0.5).min((h - bridge) * 0.5);
        if a > 0.0 {
            p.line_to(Point::new((a - fw).max(0.0), y(bridge)));
            p.bezier_curve_to(
                Point::new(a - fw * 0.45, y(bridge)),
                Point::new(a, y(bridge + fh * 0.45)),
                Point::new(a, y(bridge + fh)),
            );
            p.line_to(Point::new(a, y(h - r)));
            p.bezier_curve_to(
                Point::new(a, y(h - r * 0.45)),
                Point::new(a + r * 0.45, y(h)),
                Point::new(a + r, y(h)),
            );
        } else {
            p.line_to(Point::new(a, y(h)));
        }
        if b < width {
            p.line_to(Point::new(b - r, y(h)));
            p.bezier_curve_to(
                Point::new(b - r * 0.45, y(h)),
                Point::new(b, y(h - r * 0.45)),
                Point::new(b, y(h - r)),
            );
            p.line_to(Point::new(b, y(bridge + fh)));
            p.bezier_curve_to(
                Point::new(b, y(bridge + fh * 0.45)),
                Point::new(b + fw * 0.45, y(bridge)),
                Point::new((b + fw).min(width), y(bridge)),
            );
        } else {
            p.line_to(Point::new(width, y(h)));
        }
    }
    p.line_to(Point::new(width, y(bridge)));
    p.line_to(Point::new(width, y(0.0)));
    p.close();
    p.build()
}

/// Une concavité au raccord, une convexité au bout : même profil pour les
/// quatre bords. Seule la silhouette s'étend, le contenu n'est pas étiré.
pub(in crate::ui) fn drawer_path(
    size: Size,
    edge: Edge,
    progress: f32,
    radius: f32,
) -> canvas::Path {
    let vertical = matches!(edge, Edge::Top | Edge::Bottom);
    let (w, h) = if vertical {
        (size.width, size.height * progress)
    } else {
        (size.height, size.width * progress)
    };
    let r = radius.min(w / 4.0).min(h / 2.1).max(0.0);
    let point = |x: f32, y: f32| match edge {
        Edge::Top => Point::new(x, y),
        Edge::Bottom => Point::new(x, size.height - y),
        Edge::Left => Point::new(y, x),
        Edge::Right => Point::new(size.width - y, x),
    };
    let mut p = canvas::path::Builder::new();
    p.move_to(point(0.0, 0.0));
    p.bezier_curve_to(point(r * 0.55, 0.0), point(r, r * 0.45), point(r, r));
    p.line_to(point(r, h - r));
    p.bezier_curve_to(
        point(r, h - r * 0.45),
        point(r * 1.45, h),
        point(r * 2.0, h),
    );
    p.line_to(point(w - r * 2.0, h));
    p.bezier_curve_to(
        point(w - r * 1.45, h),
        point(w - r, h - r * 0.45),
        point(w - r, h - r),
    );
    p.line_to(point(w - r, r));
    p.bezier_curve_to(
        point(w - r, r * 0.45),
        point(w - r * 0.55, 0.0),
        point(w, 0.0),
    );
    p.close();
    p.build()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_sides_do_not_expand_a_center_notch_to_the_screen_edges() {
        assert_eq!(
            slots(900.0, [(0.0, 0.0), (400.0, 500.0), (0.0, 0.0)], 26.0),
            vec![(388.0, 512.0)]
        );
        assert!(slots(640.0, [(0.0, 0.0); 3], 26.0).is_empty());
    }
    #[test]
    fn nearby_zones_merge_and_outside_or_invalid_extents_are_safe() {
        assert_eq!(
            slots(640.0, [(0.0, 220.0), (250.0, 360.0), (400.0, 640.0)], 26.0),
            vec![(0.0, 640.0)]
        );
        assert!(slots(640.0, [(f32::NAN, 20.0), (800.0, 900.0), (0.0, 0.0)], 26.0).is_empty());
        for height in [16.0, 24.0, 48.0, 64.0, 160.0] {
            for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
                for progress in [0.0, 0.5, 1.0] {
                    let _ = drawer_path(Size::new(320.0, height), edge, progress, 24.0);
                }
            }
        }
    }
}
