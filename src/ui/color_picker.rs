//! Sélecteur de couleur : champ teinte × saturation (on clique ou on glisse),
//! curseur de luminosité, aperçu avant / après et champ hexadécimal.
//!
//! Réutilisable par tout paquet : l'état (`Picker`) vit dans l'état du
//! paquet, ses messages (`PickerMsg`) sont enveloppés par l'appelant, qui
//! décide quoi faire de « Appliquer » et « Annuler ».

use std::sync::OnceLock;

use iced::widget::{Space, button, column, container, image, mouse_area, row, slider, stack};
use iced::widget::{text_input, tooltip};
use iced::{Alignment, Border, Color, Element, Font, Length, Padding, Point};

use crate::config::Rgba;
use crate::i18n::text;

/// Taille du champ teinte × saturation.
pub const W: f32 = 264.0;
pub const H: f32 = 132.0;
const RING: f32 = 14.0;

#[derive(Debug, Clone)]
pub struct Picker {
    /// Teinte (0..360), saturation et luminosité (0..1), modèle HSV.
    pub h: f32,
    pub s: f32,
    pub v: f32,
    /// Couleur au moment de l'ouverture (aperçu « avant »).
    pub initial: Rgba,
    hover: Option<Point>,
    dragging: bool,
    hex: String,
}

#[derive(Debug, Clone)]
pub enum PickerMsg {
    Hover(Point),
    Press,
    Release,
    Exit,
    Value(f32),
    Hex(String),
    HexSubmit,
}

impl Picker {
    pub fn new(c: Rgba) -> Self {
        let (h, s, v) = rgb_to_hsv(c);
        Picker {
            h,
            s,
            v,
            initial: c,
            hover: None,
            dragging: false,
            hex: crate::settings::hex(c),
        }
    }

    pub fn color(&self) -> Rgba {
        hsv_to_rgb(self.h, self.s, self.v)
    }

    fn set(&mut self, c: Rgba) {
        let (h, s, v) = rgb_to_hsv(c);
        // Gris : la teinte n'a pas de sens, on garde celle d'avant.
        if s > 0.0 {
            self.h = h;
        }
        self.s = s;
        self.v = v;
    }

    fn pick_at(&mut self, p: Point) {
        self.h = (p.x / W).clamp(0.0, 1.0) * 360.0;
        self.s = 1.0 - (p.y / H).clamp(0.0, 1.0);
        // Choisir une teinte sur un noir ne montrerait rien : on éclaircit.
        if self.v < 0.15 {
            self.v = 0.85;
        }
        self.hex = crate::settings::hex(self.color());
    }

    pub fn update(&mut self, m: PickerMsg) {
        match m {
            PickerMsg::Hover(p) => {
                self.hover = Some(p);
                if self.dragging {
                    self.pick_at(p);
                }
            }
            PickerMsg::Press => {
                self.dragging = true;
                if let Some(p) = self.hover {
                    self.pick_at(p);
                }
            }
            PickerMsg::Release | PickerMsg::Exit => self.dragging = false,
            PickerMsg::Value(v) => {
                self.v = v.clamp(0.0, 1.0);
                self.hex = crate::settings::hex(self.color());
            }
            PickerMsg::Hex(h) => {
                // Saisie complète et lisible : appliquée tout de suite.
                if let Some(c) = Rgba::parse(h.trim())
                    && h.trim().trim_start_matches('#').len() == 6
                {
                    self.set(c);
                }
                self.hex = h;
            }
            PickerMsg::HexSubmit => {
                if let Some(c) = Rgba::parse(self.hex.trim()) {
                    self.set(c);
                }
                self.hex = crate::settings::hex(self.color());
            }
        }
    }

    /// Le sélecteur. `wrap` enveloppe ses messages ; `apply` et `cancel`
    /// sont ceux des deux boutons.
    pub fn view<'a, M: Clone + 'a>(
        &'a self,
        font: Font,
        fg: Color,
        wrap: impl Fn(PickerMsg) -> M + Clone + 'a,
        apply: M,
        cancel: M,
    ) -> Element<'a, M> {
        let edge = Color { a: 0.3, ..fg };
        let current = self.color();
        // Champ teinte × saturation et son repère.
        let x = (self.h / 360.0 * W - RING / 2.0).clamp(0.0, W - RING);
        let y = ((1.0 - self.s) * H - RING / 2.0).clamp(0.0, H - RING);
        let cur = color(current);
        let ring =
            container(Space::new().width(RING).height(RING)).style(move |_| container::Style {
                background: Some(cur.into()),
                border: Border::default()
                    .rounded(RING / 2.0)
                    .width(2.0)
                    .color(Color::WHITE),
                shadow: iced::Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.6),
                    offset: iced::Vector::new(0.0, 0.0),
                    blur_radius: 3.0,
                },
                ..Default::default()
            });
        let (w1, w2, w3, w4, w5) = (
            wrap.clone(),
            wrap.clone(),
            wrap.clone(),
            wrap.clone(),
            wrap.clone(),
        );
        let field = mouse_area(
            container(stack![
                image(field_image()).width(W).height(H),
                container(ring).padding(Padding {
                    left: x,
                    top: y,
                    right: 0.0,
                    bottom: 0.0,
                }),
            ])
            .width(W)
            .height(H)
            .style(move |_| container::Style {
                border: Border::default().rounded(6).width(1).color(edge),
                ..Default::default()
            }),
        )
        .on_move(move |p| w1(PickerMsg::Hover(p)))
        .on_press(w2(PickerMsg::Press))
        .on_release(w3(PickerMsg::Release))
        .on_exit(w4(PickerMsg::Exit))
        .interaction(iced::mouse::Interaction::Crosshair);
        // Luminosité, sur un dégradé du noir à la teinte choisie.
        let bright = hsv_to_rgb(self.h, self.s, 1.0);
        let lum = row![
            text(crate::i18n::tr("Luminosité")).font(font).size(12),
            slider(0..=100u32, (self.v * 100.0).round() as u32, move |v| {
                w5(PickerMsg::Value(v as f32 / 100.0))
            })
            .width(Length::Fill),
            text(format!("{} %", (self.v * 100.0).round() as i32))
                .font(font)
                .size(12)
                .width(40),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        let gradient = {
            let (a, b) = (Color::BLACK, color(bright));
            container(Space::new().width(Length::Fill).height(6)).style(move |_| container::Style {
                background: Some(
                    iced::gradient::Linear::new(std::f32::consts::FRAC_PI_2)
                        .add_stop(0.0, a)
                        .add_stop(1.0, b)
                        .into(),
                ),
                border: Border::default().rounded(3),
                ..Default::default()
            })
        };
        // Aperçu avant / après.
        let swatch = |c: Rgba, label: &'static str| {
            let c = color(c);
            column![
                container(Space::new().width(56).height(32)).style(move |_| container::Style {
                    background: Some(c.into()),
                    border: Border::default().rounded(6).width(1).color(edge),
                    ..Default::default()
                }),
                text(crate::i18n::tr(label))
                    .font(font)
                    .size(11)
                    .color(Color { a: 0.6, ..fg }),
            ]
            .spacing(3)
            .align_x(Alignment::Center)
        };
        let (wh, ws) = (wrap.clone(), wrap);
        let side = column![
            row![swatch(self.initial, "avant"), swatch(current, "après")].spacing(8),
            text_input("#rrggbb", &self.hex)
                .font(font)
                .size(13)
                .on_input(move |v| wh(PickerMsg::Hex(v)))
                .on_submit(ws(PickerMsg::HexSubmit))
                .padding(6)
                .width(120),
            row![
                button(text(crate::i18n::tr("Appliquer")).font(font).size(12))
                    .padding([4, 10])
                    .on_press(apply),
                button(text(crate::i18n::tr("Annuler")).font(font).size(12))
                    .padding([4, 10])
                    .style(button::secondary)
                    .on_press(cancel),
            ]
            .spacing(6),
        ]
        .spacing(10);
        let field: Element<'a, M> = tooltip(
            field,
            container(
                text(crate::i18n::tr(
                    "Cliquer ou glisser : teinte (horizontal), saturation (vertical)",
                ))
                .font(font)
                .size(11),
            )
            .padding([4, 8])
            .style(container::rounded_box),
            tooltip::Position::Top,
        )
        .delay(std::time::Duration::from_millis(600))
        .into();
        container(row![column![field, gradient, lum].spacing(8).width(W), side,].spacing(16))
            .padding(12)
            .style(move |_| container::Style {
                border: Border::default().rounded(10).width(1).color(edge),
                ..Default::default()
            })
            .into()
    }
}

fn color(c: Rgba) -> Color {
    Color::from_rgb8(c.0, c.1, c.2)
}

/// Champ teinte × saturation, à pleine luminosité (calculé une fois).
fn field_image() -> image::Handle {
    static FIELD: OnceLock<image::Handle> = OnceLock::new();
    FIELD
        .get_or_init(|| {
            // Rendu au double de la taille affichée, pour les écrans à forte densité.
            let (w, h) = ((W * 2.0) as u32, (H * 2.0) as u32);
            let mut px = Vec::with_capacity((w * h * 4) as usize);
            for y in 0..h {
                let s = 1.0 - y as f32 / (h - 1) as f32;
                for x in 0..w {
                    let c = hsv_to_rgb(x as f32 / (w - 1) as f32 * 360.0, s, 1.0);
                    px.extend_from_slice(&[c.0, c.1, c.2, 0xff]);
                }
            }
            image::Handle::from_rgba(w, h, px)
        })
        .clone()
}

pub fn hsv_to_rgb(h: f32, s: f32, v: f32) -> Rgba {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    let q = |f: f32| ((f + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    Rgba(q(r), q(g), q(b), 0xff)
}

pub fn rgb_to_hsv(c: Rgba) -> (f32, f32, f32) {
    let (r, g, b) = (c.0 as f32 / 255.0, c.1 as f32 / 255.0, c.2 as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let s = if max == 0.0 { 0.0 } else { d / max };
    (h, s, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aller_retour_hsv() {
        for c in [
            Rgba(0xff, 0x80, 0xbf, 0xff),
            Rgba(0x1b, 0x1f, 0x33, 0xff),
            Rgba(0, 0, 0, 0xff),
            Rgba(0xff, 0xff, 0xff, 0xff),
            Rgba(0x8a, 0xff, 0x80, 0xff),
        ] {
            let (h, s, v) = rgb_to_hsv(c);
            assert_eq!(hsv_to_rgb(h, s, v), c, "{c:?}");
        }
        assert_eq!(hsv_to_rgb(0.0, 1.0, 1.0), Rgba(0xff, 0, 0, 0xff));
        assert_eq!(hsv_to_rgb(120.0, 1.0, 1.0), Rgba(0, 0xff, 0, 0xff));
    }

    #[test]
    fn glisser_et_saisir() {
        let mut p = Picker::new(Rgba(0x1b, 0x1f, 0x33, 0xff));
        // Clic en haut à gauche : rouge pur (luminosité relevée depuis le sombre ?).
        p.update(PickerMsg::Hover(Point::new(0.0, 0.0)));
        p.update(PickerMsg::Press);
        assert_eq!(p.s, 1.0);
        assert!(p.h < 1.0);
        // Glisser au milieu : teinte 180°.
        p.update(PickerMsg::Hover(Point::new(W / 2.0, H)));
        assert!((p.h - 180.0).abs() < 0.5 && p.s == 0.0);
        p.update(PickerMsg::Release);
        p.update(PickerMsg::Hover(Point::new(0.0, 0.0)));
        assert!(
            (p.h - 180.0).abs() < 0.5,
            "sans bouton, survoler ne change rien"
        );
        // Saisie hexadécimale complète : appliquée.
        p.update(PickerMsg::Hex("#8aff80".into()));
        assert_eq!(p.color(), Rgba(0x8a, 0xff, 0x80, 0xff));
        assert_eq!(p.initial, Rgba(0x1b, 0x1f, 0x33, 0xff));
        // Saisie illisible validée : on revient à la couleur en cours.
        p.update(PickerMsg::Hex("bleu".into()));
        p.update(PickerMsg::HexSubmit);
        assert_eq!(p.color(), Rgba(0x8a, 0xff, 0x80, 0xff));
    }
}
