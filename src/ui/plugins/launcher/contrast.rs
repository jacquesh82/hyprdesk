//! Analyse légère du fond décodé, hors thread UI. Aucun relevé d'écran par image.
use super::App;
use crate::config::Fit;
use crate::ui::{Wall, color};
use iced::{Color, Rectangle, Size};

const GRID: usize = 24;

#[derive(Clone, Debug)]
pub(in crate::ui) struct Backdrop {
    size: Size,
    cells: Vec<(f32, f32)>,
}

pub(super) fn luminance(c: Color) -> f32 {
    let linear = |v: f32| {
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(c.r) + 0.7152 * linear(c.g) + 0.0722 * linear(c.b)
}

impl Backdrop {
    pub(in crate::ui) fn from_rgba(image: &::image::RgbaImage) -> Self {
        let (w, h) = image.dimensions();
        let mut cells = Vec::with_capacity(GRID * GRID);
        for gy in 0..GRID {
            for gx in 0..GRID {
                let (mut low, mut high) = (1.0f32, 0.0f32);
                for sy in 0..4 {
                    for sx in 0..4 {
                        let x =
                            (((gx * 4 + sx) as f32 + 0.5) * w as f32 / (GRID * 4) as f32) as u32;
                        let y =
                            (((gy * 4 + sy) as f32 + 0.5) * h as f32 / (GRID * 4) as f32) as u32;
                        let p = image.get_pixel(x.min(w - 1), y.min(h - 1));
                        let l = luminance(Color::from_rgb8(p[0], p[1], p[2]));
                        let a = p[3] as f32 / 255.0;
                        low = low.min(l * a);
                        high = high.max(l * a + 1.0 - a);
                    }
                }
                cells.push((low, high));
            }
        }
        Self {
            size: Size::new(w as f32, h as f32),
            cells,
        }
    }

    fn sample(&self, roi: Rectangle, screen: Size, fit: Fit, fallback: f32) -> (f32, f32) {
        let (sx, sy) = (
            screen.width / self.size.width,
            screen.height / self.size.height,
        );
        let (sx, sy) = match fit {
            Fit::Fill => (sx, sy),
            Fit::Cover => (sx.max(sy), sx.max(sy)),
            Fit::Contain => (sx.min(sy), sx.min(sy)),
            Fit::Center => (1.0, 1.0),
        };
        let extent = Size::new(self.size.width * sx, self.size.height * sy);
        let offset = iced::Point::new(
            (screen.width - extent.width) / 2.0,
            (screen.height - extent.height) / 2.0,
        );
        let mut range = (1.0f32, 0.0f32);
        for y in 0..GRID {
            for x in 0..GRID {
                let cell = Rectangle::new(
                    iced::Point::new(
                        offset.x + extent.width * x as f32 / GRID as f32,
                        offset.y + extent.height * y as f32 / GRID as f32,
                    ),
                    Size::new(extent.width / GRID as f32, extent.height / GRID as f32),
                );
                if cell.intersection(&roi).is_some() {
                    let (lo, hi) = self.cells[y * GRID + x];
                    range = (range.0.min(lo), range.1.max(hi));
                }
            }
        }
        // Les bandes de contain/center révèlent la couleur de fond.
        if roi.x < offset.x
            || roi.y < offset.y
            || roi.x + roi.width > offset.x + extent.width
            || roi.y + roi.height > offset.y + extent.height
            || range.0 > range.1
        {
            range = (range.0.min(fallback), range.1.max(fallback));
        }
        range
    }
}

/// Plus petite opacité qui garde le contraste sur les deux extrêmes du fond.
/// Le thème garde sa teinte et son opacité minimale ; seule la couverture augmente.
pub(super) fn readable_opacity(fg: Color, bg: Color, behind: (f32, f32), base: f32) -> f32 {
    material_opacity(fg, bg, behind, base, 0.84 / 0.88)
}

fn material_opacity(fg: Color, bg: Color, behind: (f32, f32), base: f32, ratio: f32) -> f32 {
    let fg = luminance(fg);
    let bg = luminance(bg);
    // Inclut le reflet en haut et le creux d'opacité au milieu du verre.
    let passes = |a: f32| {
        [behind.0, behind.1].into_iter().all(|wall| {
            [bg, bg + (fg - bg) * 0.065].into_iter().all(|tint| {
                let l = tint * a + wall * (1.0 - a);
                (fg.max(l) + 0.05) / (fg.min(l) + 0.05) >= 4.5
            })
        })
    };
    let base = base.clamp(0.0, 1.0);
    if passes((base * ratio).min(1.0)) {
        return base;
    }
    let (mut lo, mut hi) = (base, 1.0);
    for _ in 0..12 {
        let mid = (lo + hi) / 2.0;
        if passes((mid * ratio).min(1.0)) {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    hi
}

impl App {
    /// Une bulle peut recouvrir une fenêtre dont les pixels sont inconnus.
    /// Son plancher protège donc aussi bien un contenu blanc qu'un contenu noir.
    pub(super) fn launcher_popup_opacity(&self) -> f32 {
        let g = &self.cfg.general;
        material_opacity(
            color(g.theme.fg, 1.0),
            color(g.theme.bg, 1.0),
            (0.0, 1.0),
            g.theme.effect(crate::config::Place::Launcher).opacity,
            0.95 / 0.88,
        )
    }

    pub(super) fn launcher_material_opacity(&self, monitor: &str, roi: Rectangle) -> f32 {
        let g = &self.cfg.general;
        let base = g.theme.effect(crate::config::Place::Launcher).opacity;
        let fallback = luminance(color(g.background, 1.0));
        let screen = self
            .monitors
            .iter()
            .find(|m| m.name == monitor)
            .map_or(Size::new(1920.0, 1080.0), |m| {
                Size::new(m.width as f32, m.height as f32)
            });
        let sample = |wall: &Wall| match wall {
            Wall::Color(c) => {
                let l = luminance(color(*c, 1.0));
                (l, l)
            }
            Wall::Image(key) => {
                self.launcher_runtime
                    .backdrops
                    .get(key)
                    .map_or((fallback, fallback), |b| {
                        b.sample(
                            roi,
                            screen,
                            if key.4 { Fit::Cover } else { key.3 },
                            fallback,
                        )
                    })
            }
        };
        let behind = self.shown.get(monitor).map_or((fallback, fallback), |s| {
            let current = sample(&s.current);
            if let Some(prev) = &s.prev {
                let previous = sample(prev);
                let dur = self.crossfade().as_secs_f32();
                let p = if dur > 0.0 {
                    (s.since.elapsed().as_secs_f32() / dur).min(1.0)
                } else {
                    1.0
                };
                (
                    previous.0 + (current.0 - previous.0) * p,
                    previous.1 + (current.1 - previous.1) * p,
                )
            } else {
                current
            }
        });
        readable_opacity(color(g.theme.fg, 1.0), color(g.theme.bg, 1.0), behind, base)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transparent_dark_glass_gets_more_cover_on_a_bright_wall() {
        let fg = Color::from_rgb8(242, 244, 252);
        let bg = Color::from_rgb8(23, 27, 39);
        let dark = readable_opacity(fg, bg, (0.0, 0.0), 0.25);
        let bright = readable_opacity(fg, bg, (1.0, 1.0), 0.25);
        assert_eq!(dark, 0.25);
        assert!(bright > 0.8 && bright <= 1.0);
        assert!(
            readable_opacity(
                Color::from_rgb8(31, 35, 48),
                Color::from_rgb8(243, 244, 248),
                (0.0, 0.0),
                0.25
            ) > 0.25
        );
        assert_eq!(readable_opacity(fg, bg, (1.0, 1.0), 1.0), 1.0);
    }

    #[test]
    fn samples_follow_image_fit_and_the_region_under_the_dock() {
        let image = ::image::RgbaImage::from_fn(96, 96, |x, _| {
            if x < 48 {
                ::image::Rgba([0, 0, 0, 255])
            } else {
                ::image::Rgba([255, 255, 255, 255])
            }
        });
        let b = Backdrop::from_rgba(&image);
        let screen = Size::new(960.0, 960.0);
        let left = b.sample(
            Rectangle::new(iced::Point::new(0.0, 800.0), Size::new(300.0, 100.0)),
            screen,
            Fit::Cover,
            0.2,
        );
        let right = b.sample(
            Rectangle::new(iced::Point::new(660.0, 800.0), Size::new(300.0, 100.0)),
            screen,
            Fit::Cover,
            0.2,
        );
        assert!(left.1 < 0.01 && right.0 > 0.99);
        let letterbox = b.sample(
            Rectangle::new(iced::Point::ORIGIN, Size::new(200.0, 80.0)),
            Size::new(960.0, 1440.0),
            Fit::Contain,
            0.2,
        );
        assert!((letterbox.0 - 0.2).abs() < 0.01 && (letterbox.1 - 0.2).abs() < 0.01);
    }

    #[test]
    fn popup_material_remains_readable_over_unknown_application_content() {
        let fg = Color::from_rgb8(242, 244, 252);
        let bg = Color::from_rgb8(23, 27, 39);
        let base = material_opacity(fg, bg, (0.0, 1.0), 0.55, 0.95 / 0.88);
        let alpha = (base * 0.95 / 0.88).min(1.0);
        let foreground = luminance(fg);
        let reflection = luminance(bg) + (foreground - luminance(bg)) * 0.065;
        let background = reflection * alpha + 1.0 - alpha;
        assert!((foreground + 0.05) / (background + 0.05) >= 4.49);
        assert!((0.55..1.0).contains(&base));
    }
}
