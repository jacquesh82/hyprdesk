//! Décor en perspective et lumière locale. Le contenu conserve son layout ;
//! les ressorts et les redraw restent dans l'arbre du widget, sans Msg par image.
use std::cell::{Cell, RefCell};
use std::f32::consts::{PI, TAU};
use std::time::{Duration, Instant};

use iced::advanced::graphics::geometry::Renderer as _;
use iced::advanced::widget::{Tree, tree};
use iced::advanced::{
    Clipboard, Layout, Renderer as _, Shell, Widget, layout, mouse, overlay, renderer,
};
use iced::widget::canvas::{self, Path, Stroke};
use iced::{Color, Element, Event, Length, Point, Rectangle, Size, Vector, touch};

use super::super::kinetic::Spring;
use super::Msg;
use crate::panorama::layout::Rect;

const RESPONSE: Duration = Duration::from_millis(240);
const FRAME: Duration = Duration::from_millis(33);

fn next_frame(animate: bool, playing: bool, moving: bool, visible: bool) -> Option<Duration> {
    if !animate || !visible {
        None
    } else if moving {
        Some(Duration::ZERO)
    } else if playing {
        Some(FRAME)
    } else {
        None
    }
}

#[derive(Clone)]
pub(super) struct Scene {
    pub key: String,
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub animate: bool,
    pub playing: bool,
    pub holes: Vec<Rect>,
    pub design: crate::panorama::design::Design,
    pub wallpaper: Option<(iced::widget::image::Handle, Size)>,
}

pub(super) fn stage<'a>(content: impl Into<Element<'a, Msg>>, scene: Scene) -> Element<'a, Msg> {
    Element::new(Stage {
        content: content.into(),
        scene,
    })
}

struct Stage<'a> {
    content: Element<'a, Msg>,
    scene: Scene,
}

struct State {
    key: Option<String>,
    reveal: Spring,
    x: Spring,
    y: Spring,
    ripple: Option<(Point, Instant)>,
    cache: canvas::Cache,
    motion: canvas::Cache,
    phase: f32,
    last_frame: Option<Instant>,
    paint: RefCell<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            key: None,
            reveal: Spring::rest(0.0),
            x: Spring::rest(0.0),
            y: Spring::rest(0.0),
            ripple: None,
            cache: canvas::Cache::default(),
            motion: canvas::Cache::default(),
            phase: 0.0,
            last_frame: None,
            paint: RefCell::new(String::new()),
        }
    }
}

pub(super) fn alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

pub(super) fn mix(a: Color, b: Color, t: f32) -> Color {
    Color::from_rgba(
        a.r + (b.r - a.r) * t,
        a.g + (b.g - a.g) * t,
        a.b + (b.b - a.b) * t,
        1.0,
    )
}

/// Illustration vectorielle de secours, sans texture ni téléchargement.
pub(super) fn album(side: f32, bg: Color, accent: Color) -> Element<'static, Msg> {
    canvas::Canvas::new(Album { bg, accent })
        .width(side)
        .height(side)
        .into()
}

struct Album {
    bg: Color,
    accent: Color,
}

impl canvas::Program<Msg> for Album {
    type State = (canvas::Cache, Cell<Option<(Color, Color)>>);
    fn draw(
        &self,
        cache: &Self::State,
        renderer: &iced::Renderer,
        _: &iced::Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let (cache, palette) = cache;
        if palette.get() != Some((self.bg, self.accent)) {
            cache.clear();
            palette.set(Some((self.bg, self.accent)));
        }
        vec![cache.draw(renderer, bounds.size(), |f| {
            let s = f.width().min(f.height());
            f.fill_rectangle(
                Point::ORIGIN,
                f.size(),
                canvas::gradient::Linear::new(Point::ORIGIN, Point::new(s, s))
                    .add_stop(0.0, mix(self.bg, self.accent, 0.3))
                    .add_stop(1.0, mix(self.bg, Color::BLACK, 0.35)),
            );
            let center = Point::new(s * 0.52, s * 0.43);
            for i in (1..=12).rev() {
                f.fill(
                    &Path::circle(center, s * (0.18 + i as f32 * 0.012)),
                    alpha(self.accent, 0.012),
                );
            }
            f.fill(
                &Path::circle(center, s * 0.19),
                canvas::gradient::Linear::new(Point::new(0.0, s * 0.2), Point::new(0.0, s * 0.63))
                    .add_stop(0.0, mix(self.accent, Color::WHITE, 0.6))
                    .add_stop(1.0, alpha(self.accent, 0.02)),
            );
            for i in 0..5 {
                let x = s * (0.03 + i as f32 * 0.22);
                let peak = s * (0.46 + ((i * 7) % 4) as f32 * 0.06);
                let p = Path::new(|p| {
                    p.move_to(Point::new(x - s * 0.25, s));
                    p.line_to(Point::new(x + s * 0.12, peak));
                    p.line_to(Point::new(x + s * 0.55, s));
                    p.close();
                });
                f.fill(
                    &p,
                    canvas::gradient::Linear::new(Point::new(x, peak), Point::new(x, s))
                        .add_stop(0.0, mix(self.bg, self.accent, 0.28 + i as f32 * 0.07))
                        .add_stop(1.0, mix(self.bg, Color::BLACK, 0.4)),
                );
                f.stroke(
                    &Path::line(Point::new(x - s * 0.25, s), Point::new(x + s * 0.12, peak)),
                    Stroke::default()
                        .with_width(0.9)
                        .with_color(alpha(self.accent, 0.5)),
                );
            }
            for i in 0..22 {
                let p = Point::new(
                    s * ((i * 31 + 7) % 97) as f32 / 100.0,
                    s * ((i * 17 + 3) % 41) as f32 / 100.0,
                );
                f.fill(&Path::circle(p, s * 0.0015), alpha(Color::WHITE, 0.35));
            }
        })]
    }
}

/// Matière commune aux cartes : une face éclairée et un chant fin, sans flou.
pub(super) fn glass(
    bg: Color,
    fg: Color,
    accent: Color,
    radius: f32,
) -> iced::widget::container::Style {
    iced::widget::container::Style {
        background: Some(
            iced::gradient::Linear::new(iced::Radians(1.35))
                .add_stop(0.0, alpha(mix(bg, accent, 0.18), 0.85))
                .add_stop(0.45, alpha(mix(bg, fg, 0.035), 0.80))
                .add_stop(1.0, alpha(bg, 0.94))
                .into(),
        ),
        border: iced::Border {
            color: alpha(fg, 0.13),
            width: 1.0,
            radius: radius.into(),
        },
        shadow: iced::Shadow {
            color: Color::from_rgba(0.0, 0.0, 0.0, 0.28),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 20.0,
        },
        ..Default::default()
    }
}

/// Complément exact des trous : aucun pigment ni FX sur la fenêtre vidéo.
fn visible_regions(size: Size, holes: &[Rect]) -> Vec<Rectangle> {
    let mut regions = vec![Rectangle::with_size(size)];
    for hole in holes {
        let cut = Rectangle::new(Point::new(hole.x, hole.y), Size::new(hole.w, hole.h));
        regions = regions
            .into_iter()
            .flat_map(|r| {
                let Some(i) = r.intersection(&cut) else {
                    return vec![r];
                };
                [
                    Rectangle::new(r.position(), Size::new(r.width, i.y - r.y)),
                    Rectangle::new(
                        Point::new(r.x, i.y + i.height),
                        Size::new(r.width, r.y + r.height - i.y - i.height),
                    ),
                    Rectangle::new(Point::new(r.x, i.y), Size::new(i.x - r.x, i.height)),
                    Rectangle::new(
                        Point::new(i.x + i.width, i.y),
                        Size::new(r.x + r.width - i.x - i.width, i.height),
                    ),
                ]
                .into_iter()
                .filter(|p| p.width > 0.0 && p.height > 0.0)
                .collect()
            })
            .collect();
    }
    regions
}

fn line(frame: &mut canvas::Frame, a: Point, b: Point, ink: Color, width: f32) {
    frame.stroke(
        &Path::line(a, b),
        Stroke::default().with_color(ink).with_width(width),
    );
}

fn backdrop(frame: &mut canvas::Frame, scene: &Scene) {
    let Size {
        width: w,
        height: h,
    } = frame.size();
    let light = mix(scene.bg, scene.accent, 0.15);
    frame.fill_rectangle(
        Point::ORIGIN,
        frame.size(),
        canvas::gradient::Linear::new(Point::ORIGIN, Point::new(w, h))
            .add_stop(0.0, light)
            .add_stop(0.5, scene.bg)
            .add_stop(1.0, mix(scene.bg, scene.accent, 0.07)),
    );
    if scene.design.preset != "cinematic"
        && scene.design.preset != "forest"
        && scene.wallpaper.is_none()
    {
        frame.fill_rectangle(
            Point::ORIGIN,
            frame.size(),
            canvas::gradient::Linear::new(Point::ORIGIN, Point::new(w, h))
                .add_stop(0.0, Color::from_rgb(0.025, 0.14, 0.30))
                .add_stop(0.45, scene.bg)
                .add_stop(0.7, Color::from_rgb(0.13, 0.07, 0.40))
                .add_stop(1.0, Color::from_rgb(0.28, 0.045, 0.29)),
        );
    }
    if let Some((handle, size)) = &scene.wallpaper {
        let scale = (w / size.width.max(1.0)).max(h / size.height.max(1.0));
        let dims = Size::new(size.width * scale, size.height * scale);
        frame.draw_image(
            Rectangle::new(
                Point::new((w - dims.width) / 2.0, (h - dims.height) / 2.0),
                dims,
            ),
            handle,
        );
        frame.fill_rectangle(
            Point::ORIGIN,
            frame.size(),
            Color::from_rgba(0.0, 0.0, 0.0, scene.design.dimming),
        );
    }
    if !scene.design.effects {
        return;
    }
    // Rubans de lumière translucides ; géométrie calculée une seule fois.
    for i in 0..12 {
        let d = i as f32;
        let ribbon = Path::new(|p| {
            p.move_to(Point::new(w * 0.44 + d * 6.0, 0.0));
            p.line_to(Point::new(w * 0.63 + d * 5.0, 0.0));
            p.line_to(Point::new(w * 0.15 + d * 9.0, h));
            p.line_to(Point::new(w * 0.10 + d * 7.0, h));
            p.close();
        });
        frame.fill(&ribbon, alpha(scene.accent, 0.008));
    }
    // Sol à point de fuite : profondeur réelle par projection perspective.
    let horizon = h * 0.63;
    let vanish = Point::new(w * 0.63, horizon);
    for i in -10..=10 {
        line(
            frame,
            vanish,
            Point::new(w * 0.5 + i as f32 * w * 0.15, h),
            alpha(scene.accent, 0.055),
            0.8,
        );
    }
    for i in 1..9 {
        let t = i as f32 / 9.0;
        let y = horizon + (h - horizon) * t * t;
        line(
            frame,
            Point::new(0.0, y),
            Point::new(w, y),
            alpha(scene.accent, 0.035 + t * 0.055),
            0.7,
        );
    }
    // Filets de la dalle, repères gravés et lumière rasante.
    line(
        frame,
        Point::new(0.0, 0.5),
        Point::new(w, 0.5),
        alpha(scene.fg, 0.14),
        1.0,
    );
    line(
        frame,
        Point::new(0.0, h - 1.0),
        Point::new(w, h - 1.0),
        alpha(scene.accent, 0.25),
        1.0,
    );
    for (x, sign) in [(12.0, 1.0), (w - 12.0, -1.0)] {
        for y in [12.0, h - 12.0] {
            line(
                frame,
                Point::new(x, y),
                Point::new(x + 24.0 * sign, y),
                alpha(scene.accent, 0.5),
                1.0,
            );
        }
    }
    // Poussière stellaire déterministe, sans générateur aléatoire par image.
    for i in 0..44 {
        let x = ((i * 197 + 53) % 997) as f32 / 997.0 * w;
        let y = ((i * 127 + 31) % 499) as f32 / 499.0 * h;
        frame.fill(
            &Path::circle(Point::new(x, y), if i % 7 == 0 { 1.4 } else { 0.65 }),
            alpha(scene.fg, if i % 7 == 0 { 0.22 } else { 0.075 }),
        );
    }
}

/// Orbites 3D projetées, limitées à un petit îlot décoratif.
fn orbits(frame: &mut canvas::Frame, scene: &Scene, time: f32, pointer: Vector, reveal: f32) {
    let (w, h) = (frame.width(), frame.height());
    let r = (w.min(h) * 0.23).clamp(38.0, 150.0);
    let center = if w >= h {
        Point::new(w * 0.84, h * 0.43)
    } else {
        Point::new(w * 0.68, h * 0.20)
    };
    let center = center + Vector::new(pointer.x * 12.0, pointer.y * 8.0);
    crystal(
        frame,
        center,
        r * 0.5,
        time * 0.19 + pointer.x * 0.18,
        pointer.y,
        scene.accent,
    );
    for ring in 0..3 {
        let tilt = 0.65 + ring as f32 * 0.72;
        let angle = time * 0.13 + ring as f32 * 0.9 + pointer.x * 0.08;
        let project = |u: f32| {
            let x = u.cos();
            let y = u.sin() * tilt.cos();
            let z = u.sin() * tilt.sin();
            let rx = x * angle.cos() - z * angle.sin();
            let rz = x * angle.sin() + z * angle.cos();
            let depth = 1.0 / (1.0 + rz * 0.28);
            Point::new(center.x + rx * r * depth, center.y + y * r * depth)
        };
        let path = Path::new(|p| {
            p.move_to(project(0.0));
            for i in 1..=64 {
                p.line_to(project(i as f32 / 64.0 * TAU));
            }
        });
        frame.stroke(
            &path,
            Stroke::default()
                .with_width(1.0)
                .with_color(alpha(scene.accent, 0.15)),
        );
        let satellite = project(time * 0.34 + ring as f32 * 2.1);
        for (radius, a) in [(9.0, 0.025), (5.0, 0.09), (2.0, 0.8)] {
            frame.fill(&Path::circle(satellite, radius), alpha(scene.accent, a));
        }
    }
    if reveal < 0.999 {
        let x = w * reveal;
        frame.fill_rectangle(
            Point::new(x, 0.0),
            Size::new(2.0, h),
            alpha(scene.accent, (PI * reveal).sin() * 0.3),
        );
    }
}

/// Octaèdre : rotation dans l'espace, tri des faces et projection perspective.
fn crystal(
    frame: &mut canvas::Frame,
    center: Point,
    radius: f32,
    yaw: f32,
    pointer_y: f32,
    accent: Color,
) {
    let pitch = 0.35 + pointer_y * 0.12;
    let vertices = [
        [0.0, -1.2, 0.0],
        [1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0],
        [-1.0, 0.0, 0.0],
        [0.0, 0.0, -1.0],
        [0.0, 1.2, 0.0],
    ];
    let points = vertices.map(|[x, y, z]| {
        let rx = x * yaw.cos() - z * yaw.sin();
        let rz = x * yaw.sin() + z * yaw.cos();
        let ry = y * pitch.cos() - rz * pitch.sin();
        let depth = y * pitch.sin() + rz * pitch.cos();
        (
            Point::new(
                center.x + rx * radius / (1.0 + depth * 0.28),
                center.y + ry * radius / (1.0 + depth * 0.28),
            ),
            depth,
        )
    });
    let mut faces = [
        [0, 1, 2],
        [0, 2, 3],
        [0, 3, 4],
        [0, 4, 1],
        [5, 2, 1],
        [5, 3, 2],
        [5, 4, 3],
        [5, 1, 4],
    ];
    faces.sort_by(|a, b| {
        let depth = |face: &[usize; 3]| face.iter().map(|i| points[*i].1).sum::<f32>();
        depth(b).total_cmp(&depth(a))
    });
    for face in faces {
        let [a, b, c] = face.map(|i| points[i].0);
        let p = Path::new(|p| {
            p.move_to(a);
            p.line_to(b);
            p.line_to(c);
            p.close();
        });
        frame.fill(
            &p,
            canvas::gradient::Linear::new(a, c)
                .add_stop(0.0, alpha(mix(accent, Color::WHITE, 0.3), 0.16))
                .add_stop(1.0, alpha(accent, 0.025)),
        );
        frame.stroke(
            &p,
            Stroke::default()
                .with_width(0.8)
                .with_color(alpha(accent, 0.26)),
        );
    }
}

impl Widget<Msg, iced::Theme, iced::Renderer> for Stage<'_> {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }
    fn state(&self) -> tree::State {
        let duration = if self.scene.animate {
            RESPONSE
        } else {
            Duration::ZERO
        };
        tree::State::new(State {
            key: Some(self.scene.key.clone()),
            reveal: Spring::rest(0.0).retarget(1.0, duration, Instant::now()),
            ..State::default()
        })
    }
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }
    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }
    fn layout(
        &mut self,
        tree: &mut Tree,
        renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content
            .as_widget_mut()
            .layout(&mut tree.children[0], renderer, limits)
    }
    fn operate(
        &mut self,
        tree: &mut Tree,
        layout: Layout<'_>,
        renderer: &iced::Renderer,
        op: &mut dyn iced::advanced::widget::Operation,
    ) {
        self.content
            .as_widget_mut()
            .operate(&mut tree.children[0], layout, renderer, op);
    }
    fn update(
        &mut self,
        tree: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        renderer: &iced::Renderer,
        clipboard: &mut dyn Clipboard,
        shell: &mut Shell<'_, Msg>,
        viewport: &Rectangle,
    ) {
        let now = Instant::now();
        let b = layout.bounds();
        let s = tree.state.downcast_mut::<State>();
        let dur = if self.scene.animate {
            RESPONSE
        } else {
            Duration::ZERO
        };
        if s.key.as_ref() != Some(&self.scene.key) {
            // Repartir de la valeur visible lors d'une interruption.
            if s.key.is_some() && s.reveal.done(now) {
                s.reveal = Spring::rest(0.0);
            }
            s.reveal = s.reveal.retarget(1.0, dur, now);
            s.key = Some(self.scene.key.clone());
        }
        if matches!(
            event,
            Event::Window(iced::window::Event::RedrawRequested(_))
        ) {
            if self.scene.animate && self.scene.playing {
                s.phase += s
                    .last_frame
                    .map_or(0.0, |t| now.duration_since(t).as_secs_f32().min(0.1));
            }
            s.last_frame = Some(now);
        }
        let pos = match event {
            Event::Mouse(mouse::Event::CursorMoved { position }) => Some(*position),
            Event::Mouse(mouse::Event::CursorLeft) => Some(b.center()),
            _ => None,
        };
        if let Some(p) = pos {
            let x = ((p.x - b.center_x()) / b.width.max(1.0) * 2.0).clamp(-1.0, 1.0);
            let y = ((p.y - b.center_y()) / b.height.max(1.0) * 2.0).clamp(-1.0, 1.0);
            s.x = s.x.retarget(x, dur, now);
            s.y = s.y.retarget(y, dur, now);
        }
        let press = match event {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                cursor.position_over(b)
            }
            Event::Touch(touch::Event::FingerPressed { position, .. }) if b.contains(*position) => {
                Some(*position)
            }
            _ => None,
        };
        if self.scene.animate
            && let Some(p) = press
        {
            s.ripple = Some((Point::new(p.x - b.x, p.y - b.y), now));
        }
        if !self.scene.animate {
            s.reveal.snap();
            s.x.snap();
            s.y.snap();
            s.ripple = None;
        }
        if s.ripple
            .is_some_and(|(_, t)| now.duration_since(t) >= Duration::from_millis(550))
        {
            s.ripple = None;
        }
        let moving = !s.reveal.done(now) || !s.x.done(now) || !s.y.done(now) || s.ripple.is_some();
        if let Some(delay) = next_frame(
            self.scene.animate,
            self.scene.playing,
            moving,
            b.intersection(viewport).is_some(),
        ) {
            if delay.is_zero() {
                shell.request_redraw();
            } else {
                shell.request_redraw_at(now + delay);
            }
        }
        // Les trous vidéo et les cibles restent immobiles pendant le raccord.
        let shift = self.shift(s, now, b);
        let child_cursor = cursor.position().map_or(mouse::Cursor::Unavailable, |p| {
            mouse::Cursor::Available(p - shift)
        });
        let translated = match event {
            Event::Mouse(mouse::Event::CursorMoved { position }) => {
                Some(Event::Mouse(mouse::Event::CursorMoved {
                    position: *position - shift,
                }))
            }
            Event::Touch(t) => Some(Event::Touch(match *t {
                touch::Event::FingerPressed { id, position } => touch::Event::FingerPressed {
                    id,
                    position: position - shift,
                },
                touch::Event::FingerMoved { id, position } => touch::Event::FingerMoved {
                    id,
                    position: position - shift,
                },
                touch::Event::FingerLifted { id, position } => touch::Event::FingerLifted {
                    id,
                    position: position - shift,
                },
                touch::Event::FingerLost { id, position } => touch::Event::FingerLost {
                    id,
                    position: position - shift,
                },
            })),
            _ => None,
        };
        self.content.as_widget_mut().update(
            &mut tree.children[0],
            translated.as_ref().unwrap_or(event),
            layout,
            child_cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }
    fn mouse_interaction(
        &self,
        tree: &Tree,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
        renderer: &iced::Renderer,
    ) -> mouse::Interaction {
        let shift = self.shift(
            tree.state.downcast_ref::<State>(),
            Instant::now(),
            layout.bounds(),
        );
        let cursor = cursor.position().map_or(mouse::Cursor::Unavailable, |p| {
            mouse::Cursor::Available(p - shift)
        });
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }
    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        theme: &iced::Theme,
        style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let s = tree.state.downcast_ref::<State>();
        let now = Instant::now();
        let b = layout.bounds();
        let paint = format!(
            "{:?}:{:?}:{:?}:{:?}:{}",
            self.scene.bg,
            self.scene.fg,
            self.scene.accent,
            self.scene.design,
            self.scene.wallpaper.is_some()
        );
        if *s.paint.borrow() != paint {
            s.cache.clear();
            *s.paint.borrow_mut() = paint;
        }
        s.motion.clear();
        for r in visible_regions(b.size(), &self.scene.holes) {
            let base = s
                .cache
                .draw(renderer, b.size(), |f| backdrop(f, &self.scene));
            let fx = s.motion.draw(renderer, b.size(), |frame| {
                if self.scene.design.effects {
                    orbits(
                        frame,
                        &self.scene,
                        s.phase,
                        Vector::new(s.x.value(now), s.y.value(now)),
                        s.reveal.value(now),
                    );
                }
                if let Some((p, t)) = s.ripple {
                    let progress = (now.duration_since(t).as_secs_f32() / 0.55).clamp(0.0, 1.0);
                    frame.stroke(
                        &Path::circle(p, 8.0 + progress * 85.0),
                        Stroke::default()
                            .with_width(1.5)
                            .with_color(alpha(self.scene.accent, (1.0 - progress).powi(2) * 0.55)),
                    );
                }
            });
            let global = r + Vector::new(b.x, b.y);
            if let Some(clip) = global.intersection(viewport) {
                renderer.with_layer(clip, |r| {
                    r.with_translation(Vector::new(b.x, b.y), |r| {
                        r.draw_geometry(base);
                        r.draw_geometry(fx);
                    })
                });
            }
        }
        let shift = self.shift(s, now, b);
        let cursor = cursor.position().map_or(mouse::Cursor::Unavailable, |p| {
            mouse::Cursor::Available(p - shift)
        });
        renderer.with_layer(b.intersection(viewport).unwrap_or_default(), |r| {
            r.with_translation(shift, |r| {
                self.content.as_widget().draw(
                    &tree.children[0],
                    r,
                    theme,
                    style,
                    layout,
                    cursor,
                    &(*viewport - shift),
                );
            })
        });
    }
    fn overlay<'b>(
        &'b mut self,
        tree: &'b mut Tree,
        layout: Layout<'b>,
        renderer: &iced::Renderer,
        viewport: &Rectangle,
        translation: Vector,
    ) -> Option<overlay::Element<'b, Msg, iced::Theme, iced::Renderer>> {
        let shift = self.shift(
            tree.state.downcast_ref::<State>(),
            Instant::now(),
            layout.bounds(),
        );
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation + shift,
        )
    }
}

impl Stage<'_> {
    fn shift(&self, s: &State, now: Instant, b: Rectangle) -> Vector {
        let d = if self.scene.animate && self.scene.holes.is_empty() {
            (1.0 - s.reveal.value(now)) * 28.0
        } else {
            0.0
        };
        if b.width >= b.height {
            Vector::new(d, 0.0)
        } else {
            Vector::new(0.0, d)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn animation_stops_when_paused_hidden_or_disabled() {
        assert_eq!(next_frame(true, true, false, true), Some(FRAME));
        assert_eq!(next_frame(true, false, true, true), Some(Duration::ZERO));
        assert_eq!(next_frame(true, false, false, true), None);
        assert_eq!(next_frame(false, true, true, true), None);
        assert_eq!(next_frame(true, true, true, false), None);
    }
    #[test]
    fn video_holes_are_never_painted() {
        let holes = [
            Rect {
                x: 100.0,
                y: 40.0,
                w: 200.0,
                h: 160.0,
            },
            Rect {
                x: 250.0,
                y: 0.0,
                w: 200.0,
                h: 100.0,
            },
        ];
        let regions = visible_regions(Size::new(640.0, 240.0), &holes);
        for r in &regions {
            for h in &holes {
                let hole = Rectangle::new(Point::new(h.x, h.y), Size::new(h.w, h.h));
                assert!(
                    r.intersection(&hole)
                        .is_none_or(|i| i.width * i.height < 0.01)
                );
            }
        }
        let area: f32 = regions.iter().map(|r| r.width * r.height).sum();
        assert!((area - (640.0 * 240.0 - 32000.0 - 20000.0 + 3000.0)).abs() < 0.1);
    }
}
