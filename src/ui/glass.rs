//! Verre dessiné par hyprdesk au-dessus du fond d'une surface (MLG-268) :
//! filet, reflet, réfraction au bord, reflet qui suit le pointeur.
//!
//! Hyprland ne fait que flouter ce qu'il y a derrière une couche ; la
//! lumière du verre est donc dessinée ici, par la surface elle-même. La
//! réfraction est simulée : une bande au bord où la lumière se concentre
//! et se décompose (liseré froid dehors, chaud dedans), comme le verre
//! courbé d'une lentille — l'image derrière n'est pas déplacée, la surface
//! n'en dispose pas.

use iced::advanced::{
    Layout, Widget, layout, mouse, renderer,
    widget::{Tree, tree},
};
use iced::border::Radius;
use iced::{Background, Border, Color, Element, Length, Point, Rectangle, Size};

use crate::config::Place;
use crate::theme_engine::design::Glass;

/// Couche de verre à poser au-dessus d'une surface (même rayon).
pub struct Rim {
    glass: Glass,
    ink: Color,
    radius: Radius,
}

/// Pose le verre de `place` au-dessus de `content` : `radius` est celui du
/// fond de la surface, `ink` la couleur du texte du thème.
pub fn over<'a, M: 'a>(
    content: impl Into<Element<'a, M, iced::Theme, iced::Renderer>>,
    place: Place,
    radius: impl Into<Radius>,
    ink: Color,
) -> Element<'a, M, iced::Theme, iced::Renderer> {
    let radius = radius.into();
    let glass = super::tokens::glass(place);
    if !visible(&glass) {
        return content.into();
    }
    let rim: Element<'a, M, iced::Theme, iced::Renderer> = Element::new(Rim { glass, ink, radius });
    iced::widget::stack![content.into(), rim].into()
}

/// Surface de verre complète (MLG-275) : `content` dans un conteneur de
/// style `style`, le verre du thème par-dessus ; `bg` est la teinte de la
/// lentille. En rendu
/// matériel, avec de la réfraction au thème, le fond est dessiné par la
/// lentille ([`super::lens`]) : ce qui est derrière la surface (fond
/// d'écran, icônes ; ou `under`, la tranche de fond peinte sous une
/// surface qui ne le voit pas) est flouté si `blur`, réfracté au bord et
/// teinté de `bg` ; sinon (logiciel, réfraction nulle) fond plein et
/// liseré simulé.
#[allow(clippy::too_many_arguments)]
pub fn card<'a, M: 'a>(
    content: impl Into<Element<'a, M, iced::Theme, iced::Renderer>>,
    place: Place,
    radius: impl Into<Radius>,
    ink: Color,
    bg: Color,
    mut style: iced::widget::container::Style,
    blur: bool,
    under: Option<Element<'a, M, iced::Theme, iced::Renderer>>,
) -> Element<'a, M, iced::Theme, iced::Renderer> {
    let radius = radius.into();
    let glass = super::tokens::glass(place);
    // Matériau de secours (MLG-282) : la surface ne voit pas ce qu'il y a
    // derrière elle (rendu logiciel, ou fenêtres sans capture).
    let blind = !super::lens::available()
        || (under.is_none() && matches!(place, Place::Menus | Place::Settings | Place::Launcher));
    let fallback = blind
        .then(|| {
            super::tokens::design()
                .fallback
                .get(place.as_str())
                .copied()
        })
        .flatten();
    if !(super::lens::available() && glass.refraction > 0.0) {
        // Sans lentille : le fond du style tel quel (`bg` à défaut).
        if style.background.is_none() {
            style.background = Some(bg.into());
        }
        if let (Some(o), Some(iced::Background::Color(c))) = (fallback, style.background.as_mut()) {
            c.a = c.a.max(o);
        }
        return over(
            iced::widget::container(content).style(move |_| style),
            place,
            radius,
            ink,
        );
    }
    style.background = None;
    let tint = match fallback {
        Some(o) => Color {
            a: bg.a.max(o),
            ..bg
        },
        None => bg,
    };
    let lens = iced::widget::shader(super::lens::Lens {
        glass: glass.clone(),
        radius,
        tint,
        blur,
        erase: under.is_some(),
    })
    .width(Length::Fill)
    .height(Length::Fill);
    let body = iced::widget::container(content).style(move |_| style);
    // La lentille fait la réfraction et le liseré ; le reflet du haut et le
    // filet restent dessinés en rectangles.
    let light = Glass {
        refraction: 0.0,
        pointer_response: false,
        ..glass
    };
    let mut layers = iced::widget::stack![body].push_under(lens);
    if visible(&light) {
        let rim: Element<'a, M, iced::Theme, iced::Renderer> = Element::new(Rim {
            glass: light,
            ink,
            radius,
        });
        layers = layers.push(rim);
    }
    if let Some(under) = under {
        layers = layers.push_under(under);
    }
    layers.into()
}

/// La surface `place` est dessinée par la lentille (rendu matériel,
/// réfraction au thème).
pub fn refracts(place: Place) -> bool {
    super::lens::available() && super::tokens::glass(place).refraction > 0.0
}

/// Le verre a quelque chose à dessiner.
pub fn visible(g: &Glass) -> bool {
    g.highlight > 0.001 || g.refraction > 0.001 || g.border > 0.001
}

#[derive(Default)]
struct State {
    /// Dernière position du pointeur dessinée (reflet qui le suit).
    last: Option<Point>,
}

fn quad(
    bounds: Rectangle,
    radius: impl Into<iced::border::Radius>,
    border: (f32, Color),
) -> renderer::Quad {
    renderer::Quad {
        bounds,
        border: Border {
            radius: radius.into(),
            width: border.0,
            color: border.1,
        },
        ..Default::default()
    }
}

impl<M> Widget<M, iced::Theme, iced::Renderer> for Rim {
    fn tag(&self) -> tree::Tag {
        tree::Tag::of::<State>()
    }

    fn state(&self) -> tree::State {
        tree::State::new(State::default())
    }

    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.max())
    }

    fn update(
        &mut self,
        tree: &mut Tree,
        event: &iced::Event,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _renderer: &iced::Renderer,
        _clipboard: &mut dyn iced::advanced::Clipboard,
        shell: &mut iced::advanced::Shell<'_, M>,
        _viewport: &Rectangle,
    ) {
        // Reflet qui suit le pointeur : redessiner au mouvement, sans
        // message d'application.
        if !self.glass.pointer_response || self.glass.highlight <= 0.0 {
            return;
        }
        if let iced::Event::Mouse(mouse::Event::CursorMoved { .. } | mouse::Event::CursorLeft) =
            event
        {
            let state = tree.state.downcast_mut::<State>();
            let now = cursor.position_over(layout.bounds());
            if now != state.last {
                state.last = now;
                shell.request_redraw();
            }
        }
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        paint(
            renderer,
            layout.bounds(),
            self.radius,
            &self.glass,
            self.ink,
            cursor,
        );
    }
}

/// Rayons rétrécis de `d` (bande intérieure).
fn inset(r: Radius, d: f32) -> Radius {
    Radius {
        top_left: (r.top_left - d).max(0.0),
        top_right: (r.top_right - d).max(0.0),
        bottom_right: (r.bottom_right - d).max(0.0),
        bottom_left: (r.bottom_left - d).max(0.0),
    }
}

/// Rayons bornés à la moitié de la surface.
fn bounded(r: Radius, b: Rectangle) -> Radius {
    let m = (b.width / 2.0).min(b.height / 2.0);
    Radius {
        top_left: r.top_left.min(m),
        top_right: r.top_right.min(m),
        bottom_right: r.bottom_right.min(m),
        bottom_left: r.bottom_left.min(m),
    }
}

/// Peint le verre sur `b` : reflet, halo du pointeur, réfraction, filet.
/// Réutilisé par les surfaces qui dessinent leur propre forme (capsule
/// d'Island).
pub fn paint(
    renderer: &mut iced::Renderer,
    b: Rectangle,
    radius: impl Into<Radius>,
    g: &Glass,
    ink: Color,
    cursor: mouse::Cursor,
) {
    use iced::advanced::Renderer as _;
    if b.width < 2.0 || b.height < 2.0 || !visible(g) {
        return;
    }
    let r = bounded(radius.into(), b);
    let a = |k: f32| Color {
        a: ink.a * k.clamp(0.0, 1.0),
        ..ink
    };
    // La lumière est blanche, en thème clair comme sombre.
    let light = |k: f32| Color::from_rgba(1.0, 1.0, 1.0, k.clamp(0.0, 1.0));
    let clear = Color::TRANSPARENT;

    // Reflet : lumière venue d'en haut, sur la moitié supérieure.
    if g.highlight > 0.0 {
        let top = Rectangle {
            height: b.height * 0.5,
            ..b
        };
        let gradient = iced::gradient::Linear::new(iced::Radians(std::f32::consts::PI))
            .add_stop(0.0, light(g.highlight * 0.55))
            .add_stop(0.35, light(g.highlight * 0.18))
            .add_stop(1.0, clear);
        let top_r = Radius {
            bottom_left: 0.0,
            bottom_right: 0.0,
            ..bounded(r, top)
        };
        renderer.fill_quad(
            quad(top, top_r, (0.0, clear)),
            Background::Gradient(gradient.into()),
        );
        // Lumière qui suit le pointeur : halo doux, en disques concentriques.
        if g.pointer_response
            && let Some(p) = cursor.position_over(b)
        {
            let size = (b.width.min(b.height) * 0.9).clamp(60.0, 320.0);
            for i in 0..6 {
                let s = size * (1.0 - i as f32 / 7.0);
                let disc = Rectangle {
                    x: p.x - s / 2.0,
                    y: p.y - s / 2.0,
                    width: s,
                    height: s,
                };
                if let Some(clip) = disc.intersection(&b) {
                    renderer.with_layer(clip, |renderer| {
                        renderer.fill_quad(
                            quad(disc, s / 2.0, (0.0, clear)),
                            light(g.highlight * 0.05),
                        );
                    });
                }
            }
        }
    }

    // Réfraction : bande au bord où la lumière se concentre, puis liserés
    // décomposés (froid dehors, chaud dedans).
    if g.refraction > 0.0 && g.refraction_width > 0.5 {
        let w = g.refraction_width.min(b.width / 3.0).min(b.height / 3.0);
        let steps = 5;
        for i in 0..steps {
            let t = i as f32 / steps as f32;
            let d = w * t;
            let band = Rectangle {
                x: b.x + d,
                y: b.y + d,
                width: b.width - 2.0 * d,
                height: b.height - 2.0 * d,
            };
            renderer.fill_quad(
                quad(
                    band,
                    inset(r, d),
                    (
                        w / steps as f32 + 0.5,
                        light(g.refraction * 0.16 * (1.0 - t) * (1.0 - t)),
                    ),
                ),
                clear,
            );
        }
        let cold = Color::from_rgba(0.55, 0.85, 1.0, 0.22 * g.refraction);
        let warm = Color::from_rgba(1.0, 0.78, 0.55, 0.16 * g.refraction);
        renderer.fill_quad(quad(b, r, (1.0, cold)), clear);
        let inner = Rectangle {
            x: b.x + 1.5,
            y: b.y + 1.5,
            width: b.width - 3.0,
            height: b.height - 3.0,
        };
        renderer.fill_quad(quad(inner, inset(r, 1.5), (1.0, warm)), clear);
    }

    // Filet du verre.
    if g.border > 0.0 {
        renderer.fill_quad(quad(b, r, (1.0, a(g.border))), clear);
    }
}

/// Fond d'écran d'un écran, peint à la place qu'occupe ce widget sur
/// l'écran (MLG-275) : sous la barre ou le dock, la lentille de verre a
/// ainsi le vrai fond à réfracter. `origin` est le coin de la surface sur
/// l'écran ; la place du widget dans la surface n'est connue qu'au rendu.
pub struct Backdrop {
    pub wall: Backing,
    /// Taille logique de l'écran.
    pub screen: Size,
    /// Coin haut gauche de la surface sur l'écran.
    pub origin: Point,
}

/// Ce que montre le fond de l'écran.
pub enum Backing {
    Image(iced::widget::image::Handle, iced::ContentFit),
    Color(Color),
}

impl<M> Widget<M, iced::Theme, iced::Renderer> for Backdrop {
    fn size(&self) -> Size<Length> {
        Size::new(Length::Fill, Length::Fill)
    }

    fn layout(
        &mut self,
        _tree: &mut Tree,
        _renderer: &iced::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        layout::Node::new(limits.max())
    }

    fn draw(
        &self,
        _tree: &Tree,
        renderer: &mut iced::Renderer,
        _theme: &iced::Theme,
        _style: &renderer::Style,
        layout: Layout<'_>,
        _cursor: mouse::Cursor,
        _viewport: &Rectangle,
    ) {
        use iced::advanced::Renderer as _;
        use iced::advanced::image::Renderer as _;
        let b = layout.bounds();
        match &self.wall {
            Backing::Color(c) => renderer.fill_quad(quad(b, 0.0, (0.0, Color::TRANSPARENT)), *c),
            Backing::Image(handle, fit) => {
                let Some(size) = renderer.measure_image(handle) else {
                    return;
                };
                let img = Size::new(size.width as f32, size.height as f32);
                let fitted = fit.fit(img, self.screen);
                // Image placée sur l'écran, centrée comme le bureau la
                // montre, puis ramenée dans la surface.
                let on_screen = Rectangle {
                    x: (self.screen.width - fitted.width) / 2.0,
                    y: (self.screen.height - fitted.height) / 2.0,
                    width: fitted.width,
                    height: fitted.height,
                };
                let at = Rectangle {
                    x: on_screen.x - self.origin.x,
                    y: on_screen.y - self.origin.y,
                    ..on_screen
                };
                renderer.draw_image(
                    iced::advanced::image::Image {
                        handle: handle.clone(),
                        filter_method: iced::advanced::image::FilterMethod::Linear,
                        rotation: iced::Radians(0.0),
                        border_radius: Radius::default(),
                        opacity: 1.0,
                        snap: true,
                    },
                    at,
                    b,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verre_vide_sans_couche() {
        let g = Glass {
            highlight: 0.0,
            refraction: 0.0,
            border: 0.0,
            ..Glass::default()
        };
        assert!(!visible(&g));
        assert!(visible(&Glass::default()));
    }
}
