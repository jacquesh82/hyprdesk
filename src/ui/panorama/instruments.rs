//! Instruments du bandeau : horloges et mesures réelles, dessin mis en cache.
use super::{Msg, fx};
use chrono::Timelike;
use iced::widget::canvas::{self, Path, Stroke};
use iced::{Color, Element, Font, Length, Point, Rectangle, Size};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::f32::consts::{PI, TAU};

enum Kind {
    Clock {
        style: String,
        seconds: bool,
        date: bool,
        now: chrono::DateTime<chrono::Local>,
    },
    Metrics {
        style: String,
        lines: Vec<crate::widgets::SysLine>,
        history: HashMap<String, VecDeque<f32>>,
        limit: usize,
    },
    Record,
}
struct Instrument {
    kind: Kind,
    fg: Color,
    accent: Color,
    font: Font,
    scale: f32,
    key: String,
}
#[derive(Default)]
struct Cache {
    geometry: canvas::Cache,
    key: RefCell<String>,
}

pub(super) fn clock(
    style: &str,
    seconds: bool,
    date: bool,
    fg: Color,
    accent: Color,
    font: Font,
    scale: f32,
) -> Element<'static, Msg> {
    let now = chrono::Local::now();
    let key = format!(
        "{style}:{seconds}:{date}:{}:{fg:?}:{accent:?}:{scale}:{font:?}",
        if seconds {
            now.timestamp()
        } else {
            now.timestamp() / 60
        }
    );
    canvas::Canvas::new(Instrument {
        kind: Kind::Clock {
            style: style.into(),
            seconds,
            date,
            now,
        },
        fg,
        accent,
        font,
        scale,
        key,
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
pub(super) fn metrics(
    style: &str,
    lines: &[crate::widgets::SysLine],
    history: &HashMap<String, VecDeque<f32>>,
    limit: usize,
    palette: (Color, Color),
    font: Font,
    scale: f32,
) -> Element<'static, Msg> {
    let (fg, accent) = palette;
    let key = format!("{style}:{lines:?}:{history:?}:{fg:?}:{accent:?}:{scale}:{font:?}");
    canvas::Canvas::new(Instrument {
        kind: Kind::Metrics {
            style: style.into(),
            lines: lines.to_vec(),
            history: history.clone(),
            limit,
        },
        fg,
        accent,
        font,
        scale,
        key,
    })
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}
pub(super) fn record(side: f32, fg: Color, accent: Color) -> Element<'static, Msg> {
    canvas::Canvas::new(Instrument {
        kind: Kind::Record,
        fg,
        accent,
        font: Font::DEFAULT,
        scale: 1.0,
        key: format!("{fg:?}:{accent:?}"),
    })
    .width(side)
    .height(side)
    .into()
}
fn label(f: &mut canvas::Frame, s: impl Into<String>, p: Point, size: f32, c: Color, font: Font) {
    f.fill_text(canvas::Text {
        content: s.into(),
        position: p,
        size: size.into(),
        color: c,
        font,
        align_x: iced::alignment::Horizontal::Center.into(),
        align_y: iced::alignment::Vertical::Center,
        ..Default::default()
    });
}
fn line(f: &mut canvas::Frame, a: Point, b: Point, c: Color, width: f32) {
    f.stroke(
        &Path::line(a, b),
        Stroke::default().with_color(c).with_width(width),
    );
}
fn polar(c: Point, r: f32, a: f32) -> Point {
    Point::new(c.x + r * a.cos(), c.y + r * a.sin())
}

impl canvas::Program<Msg> for Instrument {
    type State = Cache;
    fn draw(
        &self,
        state: &Cache,
        renderer: &iced::Renderer,
        _: &iced::Theme,
        b: Rectangle,
        _: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        if *state.key.borrow() != self.key {
            state.geometry.clear();
            *state.key.borrow_mut() = self.key.clone();
        }
        vec![
            state
                .geometry
                .draw(renderer, b.size(), |f| match &self.kind {
                    Kind::Clock {
                        style,
                        seconds,
                        date,
                        now,
                    } => {
                        let (w, h) = (f.width(), f.height());
                        let center = Point::new(w / 2.0, h * 0.47);
                        if style == "analog" {
                            let r = (w.min(h) * 0.40).max(10.0);
                            f.fill(
                                &Path::circle(center, r + 8.0),
                                Color::from_rgba(0.02, 0.04, 0.07, 0.55),
                            );
                            f.stroke(
                                &Path::circle(center, r + 8.0),
                                Stroke::default()
                                    .with_color(fx::alpha(self.fg, 0.18))
                                    .with_width(2.0),
                            );
                            for i in 0..60 {
                                let a = i as f32 / 60.0 * TAU - PI / 2.0;
                                line(
                                    f,
                                    polar(center, r * if i % 5 == 0 { 0.88 } else { 0.96 }, a),
                                    polar(center, r, a),
                                    fx::alpha(self.fg, if i % 5 == 0 { 0.8 } else { 0.35 }),
                                    if i % 5 == 0 { 2.5 } else { 1.0 },
                                );
                            }
                            for hour in [3, 6, 9, 12] {
                                label(
                                    f,
                                    hour.to_string(),
                                    polar(center, r * 0.72, hour as f32 / 12.0 * TAU - PI / 2.0),
                                    r * 0.16,
                                    self.fg,
                                    self.font,
                                );
                            }
                            let minute = now.minute() as f32
                                + if *seconds {
                                    now.second() as f32 / 60.0
                                } else {
                                    0.0
                                };
                            for (value, period, length, width, c) in [
                                (now.hour() as f32 + minute / 60.0, 12.0, 0.49, 4.0, self.fg),
                                (minute, 60.0, 0.78, 3.0, self.accent),
                            ] {
                                line(
                                    f,
                                    center,
                                    polar(center, r * length, value / period * TAU - PI / 2.0),
                                    c,
                                    width,
                                );
                            }
                            if *seconds {
                                line(
                                    f,
                                    polar(
                                        center,
                                        r * 0.14,
                                        now.second() as f32 / 60.0 * TAU + PI / 2.0,
                                    ),
                                    polar(
                                        center,
                                        r * 0.88,
                                        now.second() as f32 / 60.0 * TAU - PI / 2.0,
                                    ),
                                    fx::alpha(self.accent, 0.65),
                                    1.0,
                                );
                            }
                            f.fill(&Path::circle(center, 4.0), self.accent);
                        } else {
                            let s = now
                                .format(if *seconds { "%H:%M:%S" } else { "%H:%M" })
                                .to_string();
                            let size =
                                (w / (s.len() as f32 * 0.64)).min(h * 0.55).min(180.0) * self.scale;
                            if style == "flip" {
                                let cw = (size * 0.66).min(w / s.len() as f32);
                                let left = (w - cw * s.len() as f32) / 2.0;
                                for (i, c) in s.chars().enumerate() {
                                    let x = left + i as f32 * cw;
                                    if c != ':' {
                                        let rect = Path::rounded_rectangle(
                                            Point::new(x + 2.0, center.y - size * 0.64),
                                            Size::new((cw - 4.0).max(1.0), size * 1.28),
                                            5.0.into(),
                                        );
                                        f.fill(&rect, Color::from_rgba(0.16, 0.18, 0.22, 0.95));
                                        line(
                                            f,
                                            Point::new(x + 3.0, center.y),
                                            Point::new(x + cw - 3.0, center.y),
                                            Color::from_rgba(0.0, 0.0, 0.0, 0.7),
                                            1.0,
                                        );
                                    }
                                    label(
                                        f,
                                        c.to_string(),
                                        Point::new(x + cw / 2.0, center.y),
                                        size,
                                        self.fg,
                                        Font::MONOSPACE,
                                    );
                                }
                            } else {
                                label(
                                    f,
                                    s,
                                    center,
                                    size,
                                    self.fg,
                                    Font {
                                        weight: iced::font::Weight::Light,
                                        ..self.font
                                    },
                                );
                            }
                        }
                        if *date {
                            label(
                                f,
                                now.format_localized("%A %e %B", crate::i18n::chrono_locale())
                                    .to_string(),
                                Point::new(w / 2.0, h * 0.91),
                                (w * 0.045).min(h * 0.065).clamp(10.0, 26.0) * self.scale,
                                fx::alpha(self.fg, 0.75),
                                self.font,
                            );
                        }
                    }
                    Kind::Metrics {
                        style,
                        lines,
                        history,
                        limit,
                    } => {
                        let n = lines.len().max(1);
                        let (w, h) = (f.width(), f.height());
                        let cols = if w / h > 2.4 {
                            n.min(4)
                        } else if n > 2 {
                            2
                        } else {
                            1
                        };
                        let rows = n.div_ceil(cols);
                        let cw = w / cols as f32;
                        let ch = h / rows as f32;
                        for (i, m) in lines.iter().enumerate() {
                            // Une couleur par mesure (CPU cyan, RAM violet…).
                            let tint = metric_color(&m.label, self.accent);
                            let (x, y) = ((i % cols) as f32 * cw, (i / cols) as f32 * ch);
                            let center = Point::new(x + cw / 2.0, y + ch * 0.45);
                            let small = (cw * 0.065).min(ch * 0.12).clamp(10.0, 20.0) * self.scale;
                            if style == "gauges" {
                                let r = (cw * 0.36).min(ch * 0.34).max(8.0);
                                let arc = |v: f32| {
                                    Path::new(|p| {
                                        p.arc(canvas::path::Arc {
                                            center,
                                            radius: r,
                                            start_angle: iced::Radians(PI * 0.75),
                                            end_angle: iced::Radians(PI * 0.75 + PI * 1.5 * v),
                                        })
                                    })
                                };
                                f.stroke(
                                    &arc(1.0),
                                    Stroke::default()
                                        .with_width(6.0)
                                        .with_color(fx::alpha(self.fg, 0.12)),
                                );
                                if let Some(ratio) = m.ratio {
                                    // Halo, puis l'arc : dégradé de la couleur.
                                    f.stroke(
                                        &arc(ratio.clamp(0.001, 1.0)),
                                        Stroke::default()
                                            .with_width(14.0)
                                            .with_color(fx::alpha(tint, 0.18))
                                            .with_line_cap(canvas::LineCap::Round),
                                    );
                                    f.stroke(
                                        &arc(ratio.clamp(0.001, 1.0)),
                                        Stroke::default()
                                            .with_width(6.0)
                                            .with_color(tint)
                                            .with_line_cap(canvas::LineCap::Round),
                                    );
                                }
                                label(
                                    f,
                                    &m.value,
                                    center,
                                    (cw * 0.12)
                                        .min(ch * 0.17)
                                        .min(
                                            r * 1.65
                                                / (m.value.chars().count().max(1) as f32 * 0.60),
                                        )
                                        .clamp(10.0, 34.0)
                                        * self.scale,
                                    self.fg,
                                    self.font,
                                );
                                label(
                                    f,
                                    &m.label,
                                    Point::new(center.x, y + ch * 0.91),
                                    small,
                                    fx::alpha(self.fg, 0.72),
                                    self.font,
                                );
                            } else {
                                label(
                                    f,
                                    &m.label,
                                    Point::new(center.x, y + small),
                                    small,
                                    tint,
                                    self.font,
                                );
                                label(
                                    f,
                                    &m.value,
                                    Point::new(center.x, y + ch * 0.32),
                                    (cw * 0.115).min(ch * 0.19).clamp(12.0, 38.0) * self.scale,
                                    self.fg,
                                    self.font,
                                );
                                let (left, right, top, bottom) =
                                    (x + 12.0, x + cw - 12.0, y + ch * 0.51, y + ch - 15.0);
                                line(
                                    f,
                                    Point::new(left, bottom),
                                    Point::new(right, bottom),
                                    fx::alpha(self.fg, 0.2),
                                    1.0,
                                );
                                if let Some(samples) =
                                    history.get(&m.label).filter(|s| !s.is_empty())
                                {
                                    let pts: Vec<_> = samples
                                        .iter()
                                        .enumerate()
                                        .map(|(j, v)| {
                                            Point::new(
                                                right
                                                    - (samples.len() - 1 - j) as f32
                                                        / (*limit).max(2) as f32
                                                        * (right - left),
                                                bottom - v.clamp(0.0, 1.0) * (bottom - top),
                                            )
                                        })
                                        .collect();
                                    // Courbe lissée (Catmull-Rom en Bézier),
                                    // bornée à la zone du graphique.
                                    let trace = Path::new(|p| {
                                        p.move_to(pts[0]);
                                        smooth(p, &pts, top, bottom);
                                    });
                                    let fill = Path::new(|p| {
                                        p.move_to(Point::new(pts[0].x, bottom));
                                        p.line_to(pts[0]);
                                        smooth(p, &pts, top, bottom);
                                        p.line_to(Point::new(right, bottom));
                                        p.close();
                                    });
                                    f.fill(
                                        &fill,
                                        canvas::gradient::Linear::new(
                                            Point::new(0.0, top),
                                            Point::new(0.0, bottom),
                                        )
                                        .add_stop(0.0, fx::alpha(tint, 0.45))
                                        .add_stop(1.0, fx::alpha(tint, 0.02)),
                                    );
                                    f.stroke(
                                        &trace,
                                        Stroke::default()
                                            .with_width(7.0)
                                            .with_color(fx::alpha(tint, 0.15))
                                            .with_line_join(canvas::LineJoin::Round),
                                    );
                                    f.stroke(
                                        &trace,
                                        Stroke::default()
                                            .with_width(2.4)
                                            .with_color(tint)
                                            .with_line_join(canvas::LineJoin::Round),
                                    );
                                    if let Some(last) = pts.last() {
                                        f.fill(&Path::circle(*last, 6.0), fx::alpha(tint, 0.25));
                                        f.fill(&Path::circle(*last, 3.2), tint);
                                    }
                                }
                            }
                        }
                    }
                    Kind::Record => {
                        let c = f.center();
                        let r = f.width().min(f.height()) * 0.49;
                        f.fill(&Path::circle(c, r), Color::from_rgb(0.035, 0.04, 0.055));
                        for i in 0..22 {
                            f.stroke(
                                &Path::circle(c, r * (0.36 + i as f32 * 0.028)),
                                Stroke::default()
                                    .with_width(0.6)
                                    .with_color(fx::alpha(self.fg, 0.10)),
                            );
                        }
                        f.fill(&Path::circle(c, r * 0.30), self.accent);
                        f.fill(&Path::circle(c, r * 0.04), Color::BLACK);
                        line(
                            f,
                            polar(c, r * 0.4, -0.8),
                            polar(c, r * 0.94, -0.8),
                            fx::alpha(self.fg, 0.1),
                            6.0,
                        );
                    }
                }),
        ]
    }
}

/// Couleur d'une mesure, d'après son libellé ; l'accent sinon.
fn metric_color(label: &str, accent: Color) -> Color {
    let rgb = |r: u8, g: u8, b: u8| Color::from_rgb8(r, g, b);
    match label {
        "CPU" => rgb(0x4c, 0xc9, 0xf0),
        "RAM" => rgb(0xb3, 0x88, 0xff),
        "GPU" => rgb(0x2e, 0xe5, 0x9d),
        "VRAM" => rgb(0x2e, 0xc4, 0xb6),
        "CPU °C" => rgb(0xff, 0x9f, 0x43),
        "GPU °C" => rgb(0xff, 0x5c, 0x8a),
        l if l.starts_with('/') => rgb(0xff, 0xd1, 0x66),
        l if l.to_lowercase().contains("load") || l.contains("Charge") => rgb(0xa0, 0xc4, 0xff),
        _ => accent,
    }
}

/// Prolonge le chemin le long de `pts` par une courbe lissée (Catmull-Rom
/// convertie en Bézier cubiques), sans dépasser `top`..`bottom`.
fn smooth(p: &mut canvas::path::Builder, pts: &[Point], top: f32, bottom: f32) {
    let n = pts.len();
    for i in 0..n.saturating_sub(1) {
        let p0 = pts[i.saturating_sub(1)];
        let (p1, p2) = (pts[i], pts[i + 1]);
        let p3 = pts[(i + 2).min(n - 1)];
        let clamp = |y: f32| y.clamp(top, bottom);
        let c1 = Point::new(p1.x + (p2.x - p0.x) / 6.0, clamp(p1.y + (p2.y - p0.y) / 6.0));
        let c2 = Point::new(p2.x - (p3.x - p1.x) / 6.0, clamp(p2.y - (p3.y - p1.y) / 6.0));
        p.bezier_curve_to(c1, c2, p2);
    }
}
