//! Onglet Courbes du paquet Animations (MLG-143) : les courbes du catalogue
//! de hyprdesk, et celles de l'utilisateur (reprises de hyprland.lua ou
//! créées ici), dessinées et éditables — Bézier (deux points de contrôle)
//! ou ressort (masse, raideur, amortissement). Elles sont écrites dans
//! `animations.lua` (`crate::genfiles`) et proposées pour chaque élément animé.

use iced::widget::{Space, button, canvas, column, container, row, text_input};
use iced::{Alignment, Color, Element, Length, Point, Rectangle, Task, mouse};

use super::panel::{SliderLook, SliderSpec, beside_slider};
use super::{App, Msg as UiMsg, color};
use crate::fx::CurveDef;
use crate::i18n::text;

#[derive(Debug, Clone)]
pub enum Msg {
    /// Paramètre `i` d'une courbe (Bézier : x1 y1 x2 y2 ; ressort : masse,
    /// raideur, amortissement) pendant le glissé.
    Param(String, usize, f64),
    /// Glissé terminé : enregistré.
    Commit,
    /// Nouvelle courbe (ressort ou Bézier).
    Add(bool),
    /// Nom de la nouvelle courbe en cours de saisie.
    NewName(String),
    Remove(String),
}

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Curves(m)
}

/// Position (0..1) au temps `t` (0..1) d'une courbe.
pub fn sample(def: &CurveDef, t: f32) -> f32 {
    match def {
        CurveDef::Builtin => {
            // Courbe intégrée de Hyprland : proche d'une sortie douce.
            sample(&CurveDef::Bezier([0.0, 0.75, 0.15, 1.0]), t)
        }
        CurveDef::Bezier([x1, y1, x2, y2]) => {
            // Paramètre u tel que x(u) = t (dichotomie), puis y(u).
            let b = |p1: f32, p2: f32, u: f32| {
                3.0 * (1.0 - u).powi(2) * u * p1 + 3.0 * (1.0 - u) * u * u * p2 + u.powi(3)
            };
            let (x1, y1, x2, y2) = (*x1 as f32, *y1 as f32, *x2 as f32, *y2 as f32);
            let (mut lo, mut hi) = (0.0f32, 1.0f32);
            for _ in 0..24 {
                let mid = (lo + hi) / 2.0;
                if b(x1, x2, mid) < t {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            b(y1, y2, (lo + hi) / 2.0)
        }
        CurveDef::Spring {
            mass,
            stiffness,
            damping,
        } => {
            // Oscillateur amorti de 0 vers 1, sur ~1 s.
            let (m, k, c) = (*mass as f32, *stiffness as f32, *damping as f32);
            let w0 = (k / m.max(0.01)).sqrt();
            let zeta = c / (2.0 * (k * m).sqrt().max(0.01));
            let time = t * 1.0;
            if zeta < 1.0 {
                let wd = w0 * (1.0 - zeta * zeta).sqrt();
                1.0 - (-zeta * w0 * time).exp()
                    * ((wd * time).cos() + zeta * w0 / wd * (wd * time).sin())
            } else {
                1.0 - (1.0 + w0 * time) * (-w0 * time).exp()
            }
        }
    }
}

/// Aperçu dessiné d'une courbe.
struct Plot {
    def: CurveDef,
    line: Color,
    grid: Color,
}

impl canvas::Program<UiMsg> for Plot {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &iced::Renderer,
        _: &iced::Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        // Le haut et le bas laissent de la place aux dépassements (rebond).
        let y = |v: f32| h * 0.8 - v * h * 0.6;
        let base = canvas::Path::line(Point::new(0.0, y(0.0)), Point::new(w, y(0.0)));
        let top = canvas::Path::line(Point::new(0.0, y(1.0)), Point::new(w, y(1.0)));
        let stroke = canvas::Stroke::default()
            .with_width(1.0)
            .with_color(self.grid);
        frame.stroke(&base, stroke);
        frame.stroke(&top, stroke);
        let path = canvas::Path::new(|p| {
            p.move_to(Point::new(0.0, y(0.0)));
            for i in 1..=60 {
                let t = i as f32 / 60.0;
                p.line_to(Point::new(t * w, y(sample(&self.def, t))));
            }
        });
        frame.stroke(
            &path,
            canvas::Stroke::default()
                .with_width(2.0)
                .with_color(self.line),
        );
        vec![frame.into_geometry()]
    }
}

/// Bornes et libellés des paramètres.
fn params(def: &CurveDef) -> Vec<(&'static str, f64, f64, f64, f64)> {
    match def {
        CurveDef::Bezier([a, b, c, d]) => vec![
            ("x1", *a, 0.0, 1.0, 0.01),
            ("y1", *b, -1.0, 2.0, 0.01),
            ("x2", *c, 0.0, 1.0, 0.01),
            ("y2", *d, -1.0, 2.0, 0.01),
        ],
        CurveDef::Spring {
            mass,
            stiffness,
            damping,
        } => vec![
            ("Masse", *mass, 0.1, 10.0, 0.1),
            ("Raideur", *stiffness, 1.0, 500.0, 1.0),
            ("Amortissement", *damping, 1.0, 100.0, 0.5),
        ],
        CurveDef::Builtin => Vec::new(),
    }
}

fn with_param(def: &CurveDef, i: usize, v: f64) -> CurveDef {
    let mut d = def.clone();
    match &mut d {
        CurveDef::Bezier(p) => {
            if let Some(x) = p.get_mut(i) {
                *x = v;
            }
        }
        CurveDef::Spring {
            mass,
            stiffness,
            damping,
        } => match i {
            0 => *mass = v,
            1 => *stiffness = v,
            _ => *damping = v,
        },
        CurveDef::Builtin => {}
    }
    d
}

impl App {
    fn plot(&self, def: &CurveDef) -> Element<'_, UiMsg> {
        let accent = color(self.cfg.general.theme.accent, 1.0);
        let fg = color(self.cfg.general.theme.fg, 1.0);
        container(
            canvas(Plot {
                def: def.clone(),
                line: accent,
                grid: Color { a: 0.2, ..fg },
            })
            .width(160)
            .height(72),
        )
        .padding(4)
        .into()
    }

    /// Éléments animés qui emploient une courbe.
    fn curve_users(&self, name: &str) -> Vec<&'static str> {
        crate::fx::LEAVES
            .iter()
            .filter(|l| {
                self.cfg
                    .general
                    .fx
                    .anims
                    .get(l.name)
                    .is_some_and(|a| a.curve == name)
            })
            .map(|l| l.label)
            .collect()
    }

    pub(in crate::ui) fn curves_page(&self) -> Element<'_, UiMsg> {
        let dim = self.ink().dim;
        let mut col = column![
            text(
                "Une courbe dit comment une animation avance dans le temps : lente puis \
                 rapide, avec un rebond… Les vôtres sont écrites dans animations.lua et \
                 proposées pour chaque élément (onglet Animations)."
            )
            .font(self.font)
            .size(crate::ui::tokens::ts(12.0))
            .color(dim),
            self.section("Courbes personnelles"),
        ]
        .spacing(14);
        let user = &self.cfg.general.fx.curves;
        if user.is_empty() {
            col = col.push(
                text("Aucune courbe à vous pour l'instant.")
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(dim),
            );
        }
        for (name, def) in user {
            let mut sliders = column![].spacing(6);
            for (i, (label, v, min, max, step)) in params(def).into_iter().enumerate() {
                let n = name.clone();
                let spec = SliderSpec::new(
                    format!("{name}#{i}"),
                    (min, max),
                    step,
                    move |x| wrap(Msg::Param(n.clone(), i, (x / step).round() * step)),
                    wrap(Msg::Commit),
                );
                sliders = sliders.push(
                    row![
                        // Sur la ligne du curseur, au-dessus de son échelle
                        // (HIG SEL-09).
                        beside_slider(
                            &spec,
                            text(crate::i18n::tr(label))
                                .font(self.font)
                                .size(crate::ui::tokens::ts(12.0))
                                .width(110)
                        ),
                        self.slider(
                            &spec,
                            v,
                            |x| format!("{}", (x * 100.0).round() / 100.0),
                            SliderLook::fixed(12.0, 200, Alignment::Center).spacing(10),
                        ),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                );
            }
            let users = self.curve_users(name);
            let note = if users.is_empty() {
                crate::i18n::tr("Employée par aucun élément").to_string()
            } else {
                crate::tr!(
                    "Employée par : {list}",
                    list = users
                        .iter()
                        .map(|l| crate::i18n::tr(l))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            col = col.push(
                row![
                    self.plot(def),
                    column![
                        row![
                            text(name.clone())
                                .font(self.title_font())
                                .size(crate::ui::tokens::ts(13.0))
                                .width(Length::Fill),
                            // Retrait : le glyphe du jeu (HIG ICO-07), estompé tant que la
                            // courbe est employée.
                            button(
                                // Trait du titre voisin (HIG ICO-12).
                                crate::ui::icons::glyph_weighted(
                                    "window-close",
                                    crate::ui::tokens::ts(14.0),
                                    if users.is_empty() { self.ink().fg } else { dim },
                                    self.title_font().weight,
                                )
                                .expect("glyphe de fermeture"),
                            )
                            .padding([2, 10])
                            .style(button::text)
                            .on_press_maybe(
                                users.is_empty().then(|| wrap(Msg::Remove(name.clone())))
                            ),
                        ]
                        .align_y(Alignment::Center),
                        sliders,
                        text(note)
                            .font(self.font)
                            .size(crate::ui::tokens::ts(11.0))
                            .color(dim),
                    ]
                    .spacing(6)
                    .width(Length::Fill),
                ]
                .spacing(14)
                .align_y(Alignment::Center),
            );
        }
        let draft = self.curve_name.clone();
        col = col.push(
            row![
                text_input(crate::i18n::tr("nom de la courbe"), &draft)
                    .on_input(|s| wrap(Msg::NewName(s)))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
                    .width(200),
                button(
                    text(crate::i18n::tr("Nouvelle courbe de Bézier"))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                )
                .padding([4, 10])
                .on_press(wrap(Msg::Add(false))),
                button(
                    text(crate::i18n::tr("Nouveau ressort"))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                )
                .padding([4, 10])
                .on_press(wrap(Msg::Add(true))),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );
        col = col.push(self.section("Courbes de hyprdesk"));
        for c in crate::fx::CURVES {
            col = col.push(
                row![
                    self.plot(&c.def),
                    text(crate::i18n::tr(c.label))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(13.0)),
                    Space::new().width(Length::Fill),
                ]
                .spacing(14)
                .align_y(Alignment::Center),
            );
        }
        col.into()
    }

    pub(in crate::ui) fn curves_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::Param(name, i, v) => {
                self.edit_fx(|f| {
                    if let Some(d) = f.curves.get(&name).cloned() {
                        f.curves.insert(name, with_param(&d, i, v));
                    }
                });
                self.apply()
            }
            Msg::Commit => self.commit(),
            Msg::NewName(s) => {
                self.curve_name = s;
                Task::none()
            }
            Msg::Add(spring) => {
                let base: String = self
                    .curve_name
                    .trim()
                    .chars()
                    .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                    .collect();
                let base = if base.is_empty() {
                    if spring { "ressort" } else { "courbe" }.to_string()
                } else {
                    base
                };
                let taken = |n: &str| {
                    self.cfg.general.fx.curves.contains_key(n) || crate::fx::curve(n).is_some()
                };
                let mut name = base.clone();
                let mut k = 2;
                while taken(&name) {
                    name = format!("{base}-{k}");
                    k += 1;
                }
                let def = if spring {
                    CurveDef::Spring {
                        mass: 1.0,
                        stiffness: 120.0,
                        damping: 14.0,
                    }
                } else {
                    CurveDef::Bezier([0.25, 0.1, 0.25, 1.0])
                };
                self.curve_name.clear();
                self.edit_fx(|f| {
                    f.curves.insert(name, def);
                });
                self.commit()
            }
            Msg::Remove(name) => {
                self.edit_fx(|f| {
                    f.curves.remove(&name);
                });
                self.commit()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn courbes_de_0_a_1() {
        for def in [
            CurveDef::Bezier([0.25, 0.1, 0.25, 1.0]),
            CurveDef::Spring {
                mass: 1.0,
                stiffness: 120.0,
                damping: 14.0,
            },
            CurveDef::Builtin,
        ] {
            assert!(sample(&def, 0.0).abs() < 0.05, "{def:?} part de 0");
            assert!((sample(&def, 1.0) - 1.0).abs() < 0.1, "{def:?} arrive à 1");
        }
        // Un rebond dépasse.
        let over = CurveDef::Bezier([0.05, 0.9, 0.1, 1.05]);
        assert!((0..=20).any(|i| sample(&over, i as f32 / 20.0) > 1.0));
    }
}
