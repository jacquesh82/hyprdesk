//! Présence de LYRA : mouvement local au dessin, sans timer de l'application
//! ni recalcul de la conversation. La géométrie et les zones de clic restent fixes.
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use iced::advanced::widget::{Tree, tree};
use iced::advanced::{Clipboard, Layout, Renderer, Shell, Widget, layout, mouse, renderer};
use iced::widget::{button, container, image, pick_list, text_input};
use iced::{Border, Color, Element, Event, Length, Point, Rectangle, Size, Theme};

use super::{Conversation, Entry, Msg};

pub(super) fn portrait() -> image::Handle {
    static PORTRAIT: OnceLock<image::Handle> = OnceLock::new();
    PORTRAIT
        .get_or_init(|| {
            // Le renderer logiciel d'Iced ignore le rayon des images. Une
            // texture circulaire mise en cache garantit le même avatar sur
            // GPU et en secours logiciel, sans modifier l'image source.
            let mut pixels =
                ::image::load_from_memory(include_bytes!("../../../assets/lyra/portrait.png"))
                    .expect("portrait LYRA embarqué valide")
                    .resize_to_fill(256, 256, ::image::imageops::FilterType::Lanczos3)
                    .to_rgba8();
            for (x, y, pixel) in pixels.enumerate_pixels_mut() {
                let distance = ((x as f32 - 127.5).powi(2) + (y as f32 - 127.5).powi(2)).sqrt();
                let coverage = (128.0 - distance).clamp(0.0, 1.0);
                pixel.0[3] = (pixel.0[3] as f32 * coverage) as u8;
            }
            image::Handle::from_rgba(256, 256, pixels.into_raw())
        })
        .clone()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Presence {
    Connecting,
    Ready,
    Thinking,
    Writing,
    Waiting,
    Offline,
}

impl Presence {
    pub(super) fn of(c: &Conversation) -> Self {
        if c.ended.is_some() {
            Self::Offline
        } else if c.thinking {
            Self::Thinking
        } else if c.busy {
            if matches!(c.entries.last(), Some(Entry::Assistant { .. })) {
                Self::Writing
            } else {
                Self::Thinking
            }
        } else if c.entries.iter().any(|e| {
            matches!(
                e,
                Entry::Confirm { answer: None, .. } | Entry::Choice { answer: None, .. }
            )
        }) {
            Self::Waiting
        } else if !c.ready {
            Self::Connecting
        } else {
            Self::Ready
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Connecting => "Connexion…",
            Self::Ready => "À votre écoute",
            Self::Thinking => "Je réfléchis…",
            Self::Writing => "Je vous réponds…",
            Self::Waiting => "À vous de choisir",
            Self::Offline => "Déconnectée",
        }
    }

    fn working(self) -> bool {
        matches!(self, Self::Thinking | Self::Writing | Self::Connecting)
    }
}

/// L'image d'origine est embarquée une fois. Son léger zoom reste découpé
/// dans un cercle fixe ; seul l'anneau s'anime davantage pendant une réponse.
pub(super) fn avatar(
    size: f32,
    presence: Presence,
    animate: bool,
    accent: Color,
) -> Element<'static, Msg> {
    Element::new(PresenceWidget {
        size,
        presence,
        animate,
        accent,
        waveform: false,
    })
}

pub(super) fn activity(animate: bool, accent: Color) -> Element<'static, Msg> {
    Element::new(PresenceWidget {
        size: 22.0,
        presence: Presence::Thinking,
        animate,
        accent,
        waveform: true,
    })
}

struct PresenceWidget {
    size: f32,
    presence: Presence,
    animate: bool,
    accent: Color,
    waveform: bool,
}

impl PresenceWidget {
    fn redraw(
        &self,
        now: Instant,
        bounds: Rectangle,
        viewport: &Rectangle,
        shell: &mut Shell<'_, Msg>,
    ) {
        if self.animate
            && self.presence != Presence::Offline
            && bounds.intersection(viewport).is_some()
        {
            shell.request_redraw_at(
                now + Duration::from_millis(if self.presence.working() { 33 } else { 66 }),
            );
        }
    }
}

impl Widget<Msg, Theme, iced::Renderer> for PresenceWidget {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<Instant>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(Instant::now())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fixed(self.size), Length::Fixed(self.size))
    }

    fn layout(
        &mut self,
        _: &mut Tree,
        _: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.resolve(self.size, self.size, Size::new(self.size, self.size)))
    }

    fn update(
        &mut self,
        _: &mut Tree,
        event: &Event,
        layout: Layout<'_>,
        _: mouse::Cursor,
        _: &iced::Renderer,
        _: &mut dyn Clipboard,
        shell: &mut Shell<'_, Msg>,
        viewport: &Rectangle,
    ) {
        // Le widget disparaît avec le tiroir ; un élément hors de la zone
        // défilante ne programme plus d'image. Aucun message UI n'est émis.
        if let Event::Window(iced::window::Event::RedrawRequested(now)) = event {
            self.redraw(*now, layout.bounds(), viewport, shell);
        }
    }

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut iced::Renderer,
        _: &Theme,
        _: &renderer::Style,
        layout: Layout<'_>,
        _: mouse::Cursor,
        viewport: &Rectangle,
    ) {
        let b = layout.bounds();
        let Some(_) = b.intersection(viewport) else {
            return;
        };
        let t = if self.animate && self.presence != Presence::Offline {
            tree.state.downcast_ref::<Instant>().elapsed().as_secs_f32()
        } else {
            0.0
        };
        let breath = (t * std::f32::consts::TAU / 5.5).sin();
        renderer.with_layer(*viewport, |renderer| {
            if self.waveform {
                for i in 0..3 {
                    let wave = if self.animate {
                        (t * 5.0 - i as f32 * 0.9).sin() * 0.5 + 0.5
                    } else {
                        0.35
                    };
                    let h = 5.0 + wave * 11.0;
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle::new(
                                Point::new(b.x + 2.0 + i as f32 * 7.0, b.center_y() - h / 2.0),
                                Size::new(4.0, h),
                            ),
                            border: Border::default().rounded(2),
                            ..Default::default()
                        },
                        self.accent,
                    );
                }
                return;
            }
            let ring = Rectangle::new(
                Point::new(b.x + 3.0, b.y + 3.0),
                Size::new(b.width - 6.0, b.height - 6.0),
            );
            renderer.fill_quad(
                renderer::Quad {
                    bounds: ring,
                    border: Border::default()
                        .rounded(self.size)
                        .width(1.5)
                        .color(Color {
                            a: 0.30 + 0.1 * breath,
                            ..self.accent
                        }),
                    ..Default::default()
                },
                Color {
                    a: 0.08,
                    ..self.accent
                },
            );
            let inset = self.size * 0.115;
            let face = layout::Node::new(Size::new(b.width - inset * 2.0, b.height - inset * 2.0))
                .move_to(Point::new(b.x + inset, b.y + inset));
            image::draw(
                renderer,
                Layout::new(&face),
                &portrait(),
                None,
                (self.size / 2.0).into(),
                iced::ContentFit::Cover,
                image::FilterMethod::Linear,
                iced::Rotation::default(),
                if self.presence == Presence::Offline {
                    0.55
                } else {
                    1.0
                },
                1.04 + 0.018 * breath,
            );
            if self.presence.working() {
                let angle = t * 1.7 - std::f32::consts::FRAC_PI_2;
                let radius = ring.width / 2.0 - 0.75;
                for i in 0..3 {
                    let a = angle - i as f32 * 0.16;
                    renderer.fill_quad(
                        renderer::Quad {
                            bounds: Rectangle::new(
                                Point::new(
                                    b.center_x() + a.cos() * radius - 2.5,
                                    b.center_y() + a.sin() * radius - 2.5,
                                ),
                                Size::new(5.0, 5.0),
                            ),
                            border: Border::default().rounded(3),
                            ..Default::default()
                        },
                        Color {
                            a: 1.0 - i as f32 * 0.3,
                            ..self.accent
                        },
                    );
                }
            }
        });
    }
}

pub(super) fn pill(theme: &Theme, status: button::Status, primary: bool) -> button::Style {
    let mut style = if primary {
        button::primary(theme, status)
    } else {
        button::secondary(theme, status)
    };
    style.border = style.border.rounded(22);
    if !primary && !matches!(status, button::Status::Disabled) {
        let p = theme.palette();
        style.background = Some(
            Color {
                a: if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                    0.16
                } else {
                    0.07
                },
                ..p.text
            }
            .into(),
        );
        style.text_color = p.text;
        style.border = style.border.width(1).color(Color { a: 0.12, ..p.text });
    }
    style
}

pub(super) fn selector(theme: &Theme, status: pick_list::Status) -> pick_list::Style {
    let mut style = pick_list::default(theme, status);
    let p = theme.palette();
    style.background = Color { a: 0.05, ..p.text }.into();
    style.border = Border::default().rounded(18).width(1).color(Color {
        a: if matches!(status, pick_list::Status::Active) {
            0.12
        } else {
            0.6
        },
        ..p.primary
    });
    style
}

pub(super) fn input(theme: &Theme, status: text_input::Status) -> text_input::Style {
    let mut style = text_input::default(theme, status);
    style.border = Border::default()
        .rounded(16)
        .width(if matches!(status, text_input::Status::Focused { .. }) {
            1.0
        } else {
            0.0
        })
        .color(theme.palette().primary);
    style.background = Color::TRANSPARENT.into();
    style
}

pub(super) fn composer(theme: &Theme) -> container::Style {
    let p = theme.palette();
    container::Style {
        background: Some(Color { a: 0.045, ..p.text }.into()),
        border: Border::default().rounded(24).width(1).color(Color {
            a: 0.25,
            ..p.primary
        }),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouvement_local_visible_et_desactivable() {
        let now = Instant::now();
        let bounds = Rectangle::with_size(Size::new(82.0, 82.0));
        let hidden = Rectangle::new(Point::new(200.0, 200.0), Size::new(82.0, 82.0));
        for (animate, presence, viewport, expected) in [
            (
                true,
                Presence::Thinking,
                bounds,
                iced::window::RedrawRequest::At(now + Duration::from_millis(33)),
            ),
            (
                true,
                Presence::Ready,
                bounds,
                iced::window::RedrawRequest::At(now + Duration::from_millis(66)),
            ),
            (
                false,
                Presence::Thinking,
                bounds,
                iced::window::RedrawRequest::Wait,
            ),
            (
                true,
                Presence::Offline,
                bounds,
                iced::window::RedrawRequest::Wait,
            ),
            (
                true,
                Presence::Thinking,
                hidden,
                iced::window::RedrawRequest::Wait,
            ),
        ] {
            let widget = PresenceWidget {
                size: 82.0,
                presence,
                animate,
                accent: Color::WHITE,
                waveform: false,
            };
            let mut messages = Vec::new();
            let mut shell = Shell::new(&mut messages);
            widget.redraw(now, bounds, &viewport, &mut shell);
            assert_eq!(shell.redraw_request(), expected);
            assert!(shell.is_empty(), "aucun recalcul de l'application");
            assert!(!shell.is_layout_invalid(), "géométrie stable");
        }
    }

    #[test]
    fn presence_suit_le_protocole() {
        let mut c = Conversation::new(0, vec![], String::new());
        assert_eq!(Presence::of(&c), Presence::Connecting);
        c.ready = true;
        assert_eq!(Presence::of(&c), Presence::Ready);
        c.busy = true;
        assert_eq!(Presence::of(&c), Presence::Thinking);
        c.entries.push(Entry::Assistant {
            raw: "Bonjour".into(),
            md: iced::widget::markdown::Content::new(),
        });
        assert_eq!(Presence::of(&c), Presence::Writing);
        c.thinking = true;
        assert_eq!(Presence::of(&c), Presence::Thinking);
        c.thinking = false;
        c.busy = false;
        c.entries.push(Entry::Confirm {
            id: "c".into(),
            command: "true".into(),
            reason: String::new(),
            risk: "low".into(),
            answer: None,
        });
        assert_eq!(Presence::of(&c), Presence::Waiting);
        c.ended = Some("fin".into());
        assert_eq!(Presence::of(&c), Presence::Offline);
    }
}
