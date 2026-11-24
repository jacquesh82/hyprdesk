//! Paquet intégré « Écrans » : Détection, disposition, orientation, mode, écran principal (monitors.lua).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use std::time::Duration;

use crate::i18n::text;
use iced::widget::{Space, button, column, container, mouse_area, pick_list, radio, row, toggler};
use iced::{Alignment, Border, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::hypr;
use crate::screens::{self, Align, Screen, Side};

pub const INFO: PluginInfo = PluginInfo {
    id: "screens",
    label: "Écrans",
    icon: "video-display",
    summary: "Détection, disposition, orientation, mode, écran principal (monitors.lua).",
    builtin: true,
};

/// Message de ce paquet, enveloppé pour l'application.
/// Onglets du paquet.
pub const TABS: &[super::SubTab] = &[("disposition", "Disposition"), ("ecran", "Écran")];

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Screens(m)))
}

/// État propre à ce paquet, porté par le tiroir des réglages.
#[derive(Default)]
pub struct State {
    pub screen_sel: Option<String>,
    /// Dernière position du pointeur sur le plan (px du plan).
    pub cursor: iced::Point,
    /// Écran en cours de déplacement à la souris (MLG-147).
    pub drag: Option<Drag>,
}

/// Plan des écrans : échelle et décalages (px du plan ↔ px logiques).
#[derive(Debug, Clone, Copy, Default)]
pub struct Geom {
    pub k: f32,
    pub ox: f32,
    pub oy: f32,
    pub x0: f32,
    pub y0: f32,
}

impl Geom {
    fn to_logical(self, p: iced::Point) -> (f32, f32) {
        (
            (p.x - self.ox) / self.k + self.x0,
            (p.y - self.oy) / self.k + self.y0,
        )
    }
}

/// Glissé en cours : géométrie figée (le plan ne se recadre pas sous la
/// souris), prise dans l'écran, position aimantée.
#[derive(Debug, Clone)]
pub struct Drag {
    pub name: String,
    pub geom: Geom,
    /// Décalage pointeur → coin de l'écran, en px logiques.
    pub grab: (f32, f32),
    pub start: (i32, i32),
    pub pos: (i32, i32),
    pub snapped: (bool, bool),
}

/// Taille du plan des écrans dans le tiroir (≈ 520 px utiles).
const MAP_W: f32 = 420.0;
const MAP_H: f32 = 210.0;

/// Géométrie du plan pour ces écrans actifs.
fn geometry(on: &[&Screen]) -> Geom {
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for s in on {
        let (w, h) = s.logical();
        let (x, y) = (s.x as f32, s.y as f32);
        (x0, y0, x1, y1) = (
            x0.min(x),
            y0.min(y),
            x1.max(x + w as f32),
            y1.max(y + h as f32),
        );
    }
    if on.is_empty() {
        return Geom {
            k: 1.0,
            ..Default::default()
        };
    }
    let k = ((MAP_W - 8.0) / (x1 - x0).max(1.0)).min((MAP_H - 8.0) / (y1 - y0).max(1.0));
    Geom {
        k,
        ox: (MAP_W - (x1 - x0) * k) / 2.0,
        oy: (MAP_H - (y1 - y0) * k) / 2.0,
        x0,
        y0,
    }
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Écran sélectionné dans l'onglet Écrans.
    ScreenSelect(String),
    ScreenPrimary(String),
    ScreenEnable(bool),
    ScreenSide(crate::screens::Side),
    ScreenRef(String),
    ScreenAlign(crate::screens::Align),
    ScreenRotate(u16),
    ScreenFlip(bool),
    ScreenMode(String),
    ScreenScale(f32),
    /// Découpage de l'écran (connecteur) en zones (MLG-172).
    ScreenZones(String, crate::zones::Split),
    /// Zone Panorama : posée sur un écran (côté, taille), ou retirée.
    PanoramaZone(Option<crate::panorama::config::Zone>),
    ScreensAuto,
    ScreensDetect,
    /// Affiche le numéro et le nom de chaque écran, sur l'écran lui-même.
    Identify,
    /// Fin de l'identification (génération).
    IdentifyEnd(u64),
    /// Garder la disposition appliquée (écrite dans monitors.lua).
    ScreensKeep,
    /// Pointeur sur le plan (px du plan).
    MapMove(iced::Point),
    /// Bouton pressé sur un écran du plan : début de glissé.
    DragStart(String),
    /// Bouton relâché (ou pointeur sorti du plan) : position appliquée.
    DragEnd,
    /// Déplace l'écran sélectionné de (dx, dy) px logiques.
    Nudge(i32, i32),
    /// Revenir à la disposition d'avant.
    ScreensRevert,
}

/// Délai pour garder une nouvelle disposition d'écrans avant retour automatique.
pub(in crate::ui) const SCREENS_CONFIRM_SECS: u64 = 12;
/// Applique une disposition, puis relit l'état réel.
pub(in crate::ui) fn apply_screens(list: Vec<Screen>) -> Task<UiMsg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || screens::apply(&list))
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r)
        },
        |r| {
            if let Err(e) = r {
                tracing::error!("{}", crate::tr!("écrans : {e}", e = e));
                hypr::notify(3, 6000, &crate::tr!("hyprdesk : écrans — {e}", e = e));
            }
            UiMsg::Noop
        },
    )
    .chain(App::detect_screens())
}

impl App {
    pub(in crate::ui) fn tab_screens(&self) -> Element<'_, UiMsg> {
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let dim = self.ink().dim;
        let small = |s: String| {
            text(s)
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim)
        };
        let l = &self.screens;
        if l.is_empty() {
            return column![
                self.section("Écrans"),
                small("Aucun écran détecté pour l'instant.".into()),
                button(
                    text("Détecter")
                        .font(self.font)
                        .size(crate::ui::tokens::ts(13.0))
                )
                .padding([6, 12])
                .on_press(wrap(Msg::ScreensDetect)),
            ]
            .spacing(14)
            .into();
        }
        let sel = self.screen_sel().unwrap_or_default();
        let primary = self.cfg.general.primary.clone();

        // Bandeau de confirmation après un changement.
        let mut col = column![].spacing(14);
        if let Some((_, deadline)) = &self.screens_pending {
            let left = deadline
                .saturating_duration_since(std::time::Instant::now())
                .as_secs();
            col = col.push(
                container(
                    row![
                        text(crate::tr!(
                            "Garder cet affichage ? Retour automatique dans {left} s",
                            left = left
                        ))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(14.0))
                        .width(Length::Fill),
                        button(
                            text("Rétablir")
                                .font(self.font)
                                .size(crate::ui::tokens::ts(13.0))
                        )
                        .padding([5, 12])
                        .on_press(wrap(Msg::ScreensRevert))
                        .style(button::secondary),
                        button(
                            text("Garder")
                                .font(self.font)
                                .size(crate::ui::tokens::ts(13.0))
                        )
                        .padding([5, 12])
                        .on_press(wrap(Msg::ScreensKeep)),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .padding(12)
                .style(move |_| container::Style {
                    background: Some(Color { a: 0.14, ..accent }.into()),
                    border: Border::default().rounded(10).color(accent).width(1),
                    ..Default::default()
                }),
            );
        }

        // Plan des écrans actifs, à l'échelle ; un clic sélectionne, un
        // glissé déplace (aimanté aux voisins ; Maj : sans aimant, MLG-147).
        let on: Vec<&Screen> = l.iter().filter(|s| s.enabled).collect();
        let drag = self.panel.as_ref().and_then(|p| p.screens.drag.clone());
        let geom = drag.as_ref().map_or_else(|| geometry(&on), |d| d.geom);
        let rect = |s: &Screen| {
            let (w, h) = s.logical();
            let (x, y) = match &drag {
                Some(d) if d.name == s.name => d.pos,
                _ => (s.x, s.y),
            };
            (x as f32, y as f32, w as f32, h as f32)
        };
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for s in &on {
            let (x, y, w, h) = rect(s);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + w), y1.max(y + h));
        }
        let Geom {
            k,
            ox,
            oy,
            x0: gx0,
            y0: gy0,
        } = geom;
        let tiles: Vec<Element<'_, UiMsg>> = on
            .iter()
            .map(|s| {
                let (x, y, w, h) = rect(s);
                let selected = s.name == sel;
                let main = primary.as_deref() == Some(s.name.as_str());
                let number = l.iter().position(|o| o.name == s.name).map_or(0, |i| i + 1);
                let mut label = column![
                    text(format!("{number} · {}", s.name))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(13.0))
                ]
                .align_x(Alignment::Center)
                .spacing(2);
                if main {
                    label = label.push(
                        text("★ principal")
                            .font(self.font)
                            .size(crate::ui::tokens::ts(12.0))
                            .color(accent),
                    );
                }
                // Cotes de l'écran : pixels dans son sens d'affichage (la taille
                // d'image à préparer), échelle, position dans la disposition.
                let (pw, ph) = if s.transform % 2 == 1 {
                    (s.height, s.width)
                } else {
                    (s.width, s.height)
                };
                let scale = if (s.scale - 1.0).abs() > 0.01 {
                    format!(" · ×{}", (s.scale * 100.0).round() / 100.0)
                } else {
                    String::new()
                };
                label = label
                    .push(
                        text(format!("{pw} × {ph}{scale}"))
                            .font(self.font)
                            .size(crate::ui::tokens::ts(12.0))
                            .color(dim),
                    )
                    .push(
                        text(format!("{}, {}", s.x, s.y))
                            .font(self.font)
                            .size(crate::ui::tokens::ts(12.0))
                            .color(dim),
                    );
                let moving = drag.as_ref().is_some_and(|d| d.name == s.name);
                let tile = mouse_area(
                    container(label)
                        .center(Length::Fill)
                        .width((w * k - 4.0).max(24.0))
                        .height((h * k - 4.0).max(24.0))
                        .padding(4)
                        .style(move |_| container::Style {
                            background: Some(
                                Color {
                                    a: if selected || moving { 0.30 } else { 0.10 },
                                    ..if selected || moving { accent } else { fg }
                                }
                                .into(),
                            ),
                            text_color: Some(fg),
                            border: Border::default()
                                .rounded(6)
                                .width(if selected || moving { 2 } else { 1 })
                                .color(if selected || moving {
                                    accent
                                } else {
                                    Color { a: 0.3, ..fg }
                                }),
                            ..Default::default()
                        }),
                )
                .interaction(iced::mouse::Interaction::Grab)
                .on_press(wrap(Msg::DragStart(s.name.clone())));
                container(tile)
                    .padding(iced::Padding {
                        left: ox + (x - gx0) * k + 2.0,
                        top: oy + (y - gy0) * k + 2.0,
                        ..iced::Padding::ZERO
                    })
                    .width(MAP_W)
                    .height(MAP_H)
                    .into()
            })
            .collect();
        let map = mouse_area(
            container(iced::widget::Stack::with_children(tiles))
                .width(MAP_W)
                .height(MAP_H)
                .style(move |_| container::Style {
                    background: Some(Color { a: 0.04, ..fg }.into()),
                    border: Border::default().rounded(10),
                    ..Default::default()
                }),
        )
        .on_move(|p| wrap(Msg::MapMove(p)))
        .on_release(wrap(Msg::DragEnd))
        .on_exit(wrap(Msg::DragEnd));
        // Cotes de l'ensemble (pixels logiques), comme sur un plan : largeur
        // sous le plan, hauteur à droite, alignées sur les écrans.
        let (span_w, span_h) = ((x1 - x0).max(0.0), (y1 - y0).max(0.0));
        let ink = Color { a: 0.5, ..fg };
        let bar = move |w: Length, h: Length| {
            container(Space::new().width(w).height(h)).style(move |_| container::Style {
                background: Some(ink.into()),
                ..Default::default()
            })
        };
        let hdim = container(
            row![
                bar(Length::Fixed(1.0), Length::Fixed(9.0)),
                bar(Length::Fill, Length::Fixed(1.0)),
                text(format!("{} px", span_w.round() as i32))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(dim),
                bar(Length::Fill, Length::Fixed(1.0)),
                bar(Length::Fixed(1.0), Length::Fixed(9.0)),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .width((span_w * k).max(40.0)),
        )
        .padding(iced::Padding {
            left: ox,
            ..iced::Padding::ZERO
        });
        let vdim = container(
            column![
                bar(Length::Fixed(9.0), Length::Fixed(1.0)),
                bar(Length::Fixed(1.0), Length::Fill),
                text(format!("{} px", span_h.round() as i32))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(dim)
                    .wrapping(iced::widget::text::Wrapping::None),
                bar(Length::Fixed(1.0), Length::Fill),
                bar(Length::Fixed(9.0), Length::Fixed(1.0)),
            ]
            .spacing(6)
            .align_x(Alignment::Center)
            .width(64)
            .height((span_h * k).max(40.0)),
        )
        .padding(iced::Padding {
            top: oy,
            ..iced::Padding::ZERO
        });
        let map = column![
            row![map, vdim].spacing(8),
            hdim,
            // Deux lignes : les mesures, puis la légende du plan.
            text(crate::tr!(
                "Ensemble : {w} × {h} px logiques, de ({x0}, {y0}) à ({x1}, {y1}).",
                w = span_w.round() as i32,
                h = span_h.round() as i32,
                x0 = x0.round() as i32,
                y0 = y0.round() as i32,
                x1 = x1.round() as i32,
                y1 = y1.round() as i32
            ))
            .font(self.font)
            .size(crate::ui::tokens::ts(13.0))
            .color(self.ink().fg),
            text(match &drag {
                Some(d) => crate::tr!(
                    "{n} : {x}, {y}{s} — relâchez pour appliquer.",
                    n = d.name,
                    x = d.pos.0,
                    y = d.pos.1,
                    s = if d.snapped.0 || d.snapped.1 {
                        crate::i18n::tr(" (aimanté)")
                    } else {
                        ""
                    }
                ),
                None => crate::i18n::tr(
                    "Glissez un écran pour le déplacer : il s'aimante aux bords et aux centres \
                     de ses voisins (Maj : sans aimant). Sous chaque écran : sa taille \
                     d'image et sa position.",
                )
                .to_string(),
            })
            .font(self.font)
            .size(crate::ui::tokens::ts(13.0))
            .color(dim),
        ]
        .spacing(6);
        // Réglage fin de l'écran sélectionné : au pixel près (Maj : 10 px).
        let nudge = |label: &'static str, dx: i32, dy: i32| {
            button(
                text(label)
                    .font(self.font)
                    .size(crate::ui::tokens::ts(14.0)),
            )
            .padding([3, 10])
            .style(button::secondary)
            .on_press_maybe((!sel.is_empty()).then_some(wrap(Msg::Nudge(dx, dy))))
        };
        let fine = row![
            small(if sel.is_empty() {
                crate::i18n::tr("Choisissez un écran sur le plan pour l'ajuster au pixel près.")
                    .to_string()
            } else {
                crate::tr!("Ajuster {n} :", n = sel)
            }),
            nudge("←", -1, 0),
            nudge("↑", 0, -1),
            nudge("↓", 0, 1),
            nudge("→", 1, 0),
            small(crate::i18n::tr("1 px (Maj : 10 px)").to_string()),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        let off: Vec<String> = l
            .iter()
            .filter(|s| !s.enabled)
            .map(|s| s.name.clone())
            .collect();
        let mut chips = row![].spacing(8).align_y(Alignment::Center);
        if !off.is_empty() {
            chips = chips.push(small("Désactivés :".into()));
            for name in off {
                chips = chips.push(
                    button(
                        text(name.clone())
                            .font(self.font)
                            .size(crate::ui::tokens::ts(12.0)),
                    )
                    .padding([3, 10])
                    .style(button::secondary)
                    .on_press(wrap(Msg::ScreenSelect(name))),
                );
            }
        }
        if !self.on_subtab(INFO.id, "ecran") {
            col = col
                .push(self.section("Disposition"))
                .push(map)
                .push(fine)
                .push(chips)
                .push(
                    row![
                        button(
                            text("Disposition automatique")
                                .font(self.font)
                                .size(crate::ui::tokens::ts(13.0))
                        )
                        .padding([5, 12])
                        .on_press(wrap(Msg::ScreensAuto)),
                        button(
                            text("Détecter")
                                .font(self.font)
                                .size(crate::ui::tokens::ts(13.0))
                        )
                        .padding([5, 12])
                        .style(button::secondary)
                        .on_press(wrap(Msg::ScreensDetect)),
                        button(
                            text("Identifier")
                                .font(self.font)
                                .size(crate::ui::tokens::ts(13.0))
                        )
                        .padding([5, 12])
                        .style(button::secondary)
                        .on_press(wrap(Msg::Identify)),
                        button(
                            text("Enregistrer")
                                .font(self.font)
                                .size(crate::ui::tokens::ts(13.0))
                        )
                        .padding([5, 12])
                        .style(button::secondary)
                        .on_press_maybe(
                            self.screens_pending
                                .is_none()
                                .then_some(wrap(Msg::ScreensKeep)),
                        ),
                    ]
                    .spacing(10),
                );
            return col.into();
        }
        // Onglet « Écran » : choix de l'écran, puis ses réglages.
        col = col.push(
            row(l.iter().map(|s| {
                let mut label = s.name.clone();
                if primary.as_deref() == Some(s.name.as_str()) {
                    label.push_str("  ★");
                }
                if !s.enabled {
                    label.push_str("  (désactivé)");
                }
                button(
                    text(label)
                        .font(self.font)
                        .size(crate::ui::tokens::ts(13.0)),
                )
                .padding([5, 12])
                .style(if s.name == sel {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(wrap(Msg::ScreenSelect(s.name.clone())))
                .into()
            }))
            .spacing(8)
            .wrap(),
        );

        // Réglages de l'écran sélectionné.
        let Some(s) = l.iter().find(|s| s.name == sel) else {
            return col.into();
        };
        let title = if s.description.is_empty() {
            s.name.clone()
        } else {
            format!("{} — {}", s.name, s.description)
        };
        col = col.push(self.section("Écran sélectionné")).push(
            text(title)
                .font(self.font)
                .size(crate::ui::tokens::ts(14.0))
                .font(self.title_font()),
        );
        let is_main = primary.as_deref() == Some(s.name.as_str());
        col = col.push(
            row![
                toggler(s.enabled)
                    .label(crate::i18n::tr("Activé"))
                    .on_toggle(|v| wrap(Msg::ScreenEnable(v))),
                if is_main {
                    Element::from(
                        text("★ Écran principal")
                            .font(self.font)
                            .size(crate::ui::tokens::ts(13.0))
                            .color(accent),
                    )
                } else {
                    button(
                        text("Définir comme écran principal")
                            .font(self.font)
                            .size(crate::ui::tokens::ts(13.0)),
                    )
                    .padding([5, 12])
                    .on_press(wrap(Msg::ScreenPrimary(s.name.clone())))
                    .into()
                },
            ]
            .spacing(24)
            .align_y(Alignment::Center),
        );
        if !s.enabled {
            return col.into();
        }
        if let Some((reference, side, align)) = self.screen_placement(&s.name) {
            let refs: Vec<String> = l
                .iter()
                .filter(|o| o.name != s.name && o.enabled)
                .map(|o| o.name.clone())
                .collect();
            let aligns = [Align::Start, Align::Center, Align::End];
            col = col.push(
                self.field(
                    "Position",
                    "",
                    column![
                        row![
                            pick_list(Side::ALL, Some(side), |v| wrap(Msg::ScreenSide(v)))
                                .font(self.font)
                                .width(170),
                            pick_list(refs, Some(reference), |v| wrap(Msg::ScreenRef(v)))
                                .font(self.font)
                                .width(170),
                        ]
                        .spacing(10),
                        row(aligns.into_iter().map(|a| {
                            radio(a.label(side), a, Some(align), |v| wrap(Msg::ScreenAlign(v)))
                                .font(self.font)
                                .into()
                        }))
                        .spacing(18),
                    ]
                    .spacing(10),
                ),
            );
        }
        // Surcharges de cet écran (MLG-281) : hauteur de la barre, échelle
        // du texte des surfaces ; « 0 » / « 100 % » : comme les autres.
        {
            use crate::ui::plugins::theme::{Msg as TMsg, wrap as twrap};
            let g = &self.cfg.general;
            let bar_h = g
                .plugin_options
                .get(&format!("ui.screen.{}", s.name))
                .and_then(|m| m.get("bar_height"))
                .and_then(|v| match v {
                    crate::tiling::OptValue::Num(n) => Some(*n),
                    _ => None,
                });
            let text_scale = g
                .plugin_options
                .get(&format!("ui.screen.{}", s.name))
                .and_then(|m| m.get("text_scale"))
                .and_then(|v| match v {
                    crate::tiling::OptValue::Num(n) => Some(*n),
                    _ => None,
                })
                .unwrap_or(1.0);
            let base = self.cfg.bar.as_ref().map_or(36, |b| b.height);
            let (n1, n2, n3, n4) = (
                format!("screen.{}.bar_height", s.name),
                format!("screen.{}.bar_height", s.name),
                format!("screen.{}.text_scale", s.name),
                format!("screen.{}.text_scale", s.name),
            );
            let shown_h = bar_h.unwrap_or(f64::from(base));
            col = col.push(
                self.field(
                    "Barre sur cet écran",
                    "",
                    row![
                        iced::widget::slider(24.0..=64.0, shown_h, move |v| {
                            twrap(TMsg::DesignOpt(n1.clone(), format!("{}", v.round()), false))
                        })
                        .step(1.0)
                        .on_release(twrap(TMsg::DesignOpt(
                            n2,
                            format!("{}", shown_h.round()),
                            true
                        )))
                        .width(220),
                        text(format!("{} px", shown_h.round()))
                            .font(self.font)
                            .size(crate::ui::tokens::ts(13.0)),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                ),
            );
            col = col.push(
                self.field(
                    "Texte sur cet écran",
                    "",
                    row![
                        iced::widget::slider(0.5..=2.0, text_scale, move |v| {
                            twrap(TMsg::DesignOpt(n3.clone(), format!("{v:.2}"), false))
                        })
                        .step(0.05)
                        .on_release(twrap(TMsg::DesignOpt(n4, format!("{text_scale:.2}"), true)))
                        .width(220),
                        text(format!("{:.0} %", text_scale * 100.0))
                            .font(self.font)
                            .size(crate::ui::tokens::ts(13.0)),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                ),
            );
        }
        let rot = s.rotation();
        col = col.push(
            self.field(
                "Orientation",
                "",
                column![
                    column(
                        [
                            [
                                (0u16, crate::i18n::tr("Paysage")),
                                (180, crate::i18n::tr("Paysage retourné"))
                            ],
                            [
                                (90, crate::i18n::tr("Portrait")),
                                (270, crate::i18n::tr("Portrait retourné"))
                            ],
                        ]
                        .into_iter()
                        .map(|pair| {
                            row(pair.into_iter().map(|(d, label)| {
                                radio(crate::i18n::tr(label), d, Some(rot), |v| {
                                    wrap(Msg::ScreenRotate(v))
                                })
                                .font(self.font)
                                .width(190)
                                .into()
                            }))
                            .into()
                        })
                    )
                    .spacing(8),
                    self.adv(
                        "Mise en miroir de l'image",
                        toggler(s.flipped())
                            .label(crate::i18n::tr("Image en miroir"))
                            .on_toggle(|v| wrap(Msg::ScreenFlip(v)))
                    ),
                ]
                .spacing(10),
            ),
        );
        let mut modes = s.modes.clone();
        if !modes.contains(&s.mode()) {
            modes.insert(0, s.mode());
        }
        let scale_label = |v: f32| format!("{:.0} %", v * 100.0);
        let scales: Vec<String> = screens::SCALES.iter().map(|v| scale_label(*v)).collect();
        col = col
            .push(
                self.field(
                    "Résolution et fréquence",
                    "",
                    pick_list(modes, Some(s.mode()), |m| wrap(Msg::ScreenMode(m)))
                        .font(self.font)
                        .width(240),
                ),
            )
            .push(
                self.field(
                    "Échelle",
                    "",
                    pick_list(scales, Some(scale_label(s.scale)), |label| {
                        let v = screens::SCALES
                            .iter()
                            .copied()
                            .find(|v| format!("{:.0} %", v * 100.0) == label)
                            .unwrap_or(1.0);
                        wrap(Msg::ScreenScale(v))
                    })
                    .font(self.font)
                    .width(140),
                ),
            )
            .push(small(
                "Chaque changement s'applique tout de suite ; gardez-le, sinon l'affichage \
                 d'avant revient de lui-même. La disposition est écrite dans monitors.lua."
                    .into(),
            ))
            .push(self.screen_zones_view(s));
        // Zone Panorama : seulement quand le paquet Panorama est actif ;
        // découpe peu courante d'un écran ordinaire : avancé.
        if self.cfg.plugin_on("panorama") {
            col = col.push(self.adv("Zone Panorama", self.screen_panorama_zone_view(s)));
        }
        col.into()
    }

    /// Zone Panorama : une bande de l'écran réservée au Panorama, que les
    /// fenêtres laissent libre (même maximisées), comme un second écran.
    fn screen_panorama_zone_view(&self, s: &Screen) -> Element<'_, UiMsg> {
        use crate::panorama::config::Zone;
        let dim = self.ink().dim;
        let zone = self.panorama_cfg().zone;
        let mut col = column![
            self.section("Zone Panorama"),
            text(
                "Réserve une bande de cet écran au Panorama : les fenêtres, même maximisées, \
                 s'arrêtent avant, comme sur un second écran.",
            )
            .font(self.font)
            .size(crate::ui::tokens::ts(12.0))
            .color(dim),
        ]
        .spacing(10);
        match zone.clone().filter(|z| z.screen == s.name) {
            None => {
                if let Some(z) = &zone {
                    col = col.push(
                        text(crate::tr!(
                            "La zone Panorama est sur {s}.",
                            s = z.screen.clone()
                        ))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                        .color(dim),
                    );
                }
                col = col.push(
                    button(
                        text("Définir une zone Panorama")
                            .font(self.font)
                            .size(crate::ui::tokens::ts(13.0)),
                    )
                    .padding([6, 14])
                    .on_press(wrap(Msg::PanoramaZone(Some(Zone {
                        screen: s.name.clone(),
                        side: "bottom".into(),
                        size: 0.25,
                    })))),
                );
            }
            Some(z) => {
                let vertical = z.vertical();
                let orient = {
                    let z1 = z.clone();
                    let z2 = z.clone();
                    row![
                        radio(
                            crate::i18n::tr("Horizontale"),
                            false,
                            Some(vertical),
                            move |_| {
                                wrap(Msg::PanoramaZone(Some(Zone {
                                    side: "bottom".into(),
                                    ..z1.clone()
                                })))
                            }
                        )
                        .text_size(13),
                        radio(
                            crate::i18n::tr("Verticale"),
                            true,
                            Some(vertical),
                            move |_| {
                                wrap(Msg::PanoramaZone(Some(Zone {
                                    side: "right".into(),
                                    ..z2.clone()
                                })))
                            }
                        )
                        .text_size(13),
                    ]
                    .spacing(16)
                };
                let sides: &[(&'static str, &'static str)] = if vertical {
                    &[("left", "Gauche"), ("right", "Droite")]
                } else {
                    &[("top", "Haut"), ("bottom", "Bas")]
                };
                let cur = Zone::SIDES.iter().copied().find(|x| *x == z.side);
                let mut side_row = row![].spacing(16);
                for (id, label) in sides {
                    let zz = z.clone();
                    side_row = side_row.push(
                        radio(crate::i18n::tr(label), *id, cur, move |v: &'static str| {
                            wrap(Msg::PanoramaZone(Some(Zone {
                                side: v.into(),
                                ..zz.clone()
                            })))
                        })
                        .text_size(13),
                    );
                }
                let zs = z.clone();
                col = col
                    .push(orient)
                    .push(side_row)
                    .push(
                        row![
                            text(crate::i18n::tr("Taille"))
                                .font(self.font)
                                .size(crate::ui::tokens::ts(13.0))
                                .width(80),
                            iced::widget::slider(10.0..=50.0, z.size * 100.0, move |v| {
                                wrap(Msg::PanoramaZone(Some(Zone {
                                    size: (v / 100.0 * 20.0).round() / 20.0,
                                    ..zs.clone()
                                })))
                            })
                            .step(5.0_f32),
                            text(format!("{:.0} %", z.size * 100.0))
                                .font(self.font)
                                .size(crate::ui::tokens::ts(12.0))
                                .width(48),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    )
                    .push(
                        button(
                            text("Retirer la zone Panorama")
                                .font(self.font)
                                .size(crate::ui::tokens::ts(13.0)),
                        )
                        .padding([6, 14])
                        .on_press(wrap(Msg::PanoramaZone(None))),
                    );
            }
        }
        col.into()
    }

    /// Découpage de l'écran en zones : boutons 1 | 1×2 | 1×3 | 2×2 | 3×3, part
    /// de la zone 1, schéma de l'écran découpé.
    fn screen_zones_view(&self, s: &Screen) -> Element<'_, UiMsg> {
        use crate::zones::{First, Grid, Split};
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let dim = self.ink().dim;
        let small = |t: &'static str| {
            text(t)
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim)
        };
        let mut col = column![self.section("Zones")].spacing(10);
        let panorama = self.cfg.general.panorama.screens.iter().any(|e| {
            e == &s.name || (!s.description.is_empty() && s.description.contains(e.as_str()))
        });
        if panorama {
            return col
                .push(small(
                    "Écran Panorama : son découpage se règle dans Réglages → Panorama → Pages.",
                ))
                .into();
        }
        let cur = self
            .cfg
            .general
            .screen_zones
            .get(&s.name)
            .copied()
            .unwrap_or_default();
        let name = s.name.clone();
        let mut seg = row![].spacing(6);
        for g in Grid::ALL {
            let on = cur.grid == g;
            let n = name.clone();
            seg = seg.push(
                button(
                    text(g.short())
                        .font(self.font)
                        .size(crate::ui::tokens::ts(13.0)),
                )
                .padding([5, 14])
                .style(if on {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(wrap(Msg::ScreenZones(
                    n,
                    Split {
                        grid: g,
                        first: cur.first,
                    },
                ))),
            );
        }
        col = col.push(seg);
        if cur.grid.has_first() {
            let mut r = row![
                text("Part de la zone 1")
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
            ]
            .spacing(14)
            .align_y(Alignment::Center);
            for f in [First::Half, First::Third] {
                let n = name.clone();
                r = r.push(
                    radio(f.id(), f, Some(cur.first()), move |v| {
                        wrap(Msg::ScreenZones(
                            n.clone(),
                            Split {
                                grid: cur.grid,
                                first: Some(v),
                            },
                        ))
                    })
                    .font(self.font)
                    .size(crate::ui::tokens::ts(16.0))
                    .text_size(13),
                );
            }
            col = col.push(r);
        }
        // Schéma : l'écran à l'échelle, ses zones numérotées.
        let (lw, lh) = s.logical();
        let (bw, bh) = if lw >= lh {
            (240.0, 240.0 * lh.max(1) as f32 / lw.max(1) as f32)
        } else {
            (240.0 * lw.max(1) as f32 / lh.max(1) as f32, 240.0)
        };
        let mut plan = iced::widget::Stack::new().width(bw).height(bh).push(
            container(Space::new())
                .width(bw)
                .height(bh)
                .style(move |_| container::Style {
                    border: Border {
                        color: Color { a: 0.5, ..fg },
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    ..Default::default()
                }),
        );
        for (i, [x, y, w, h]) in cur.rects(bw, bh, 4.0).into_iter().enumerate() {
            plan = plan.push(
                container(
                    container(
                        text((i + 1).to_string())
                            .font(self.font)
                            .size(crate::ui::tokens::ts(12.0))
                            .color(fg),
                    )
                    .center_x(Length::Fill)
                    .center_y(Length::Fill)
                    .width(w)
                    .height(h)
                    .style(move |_| container::Style {
                        background: Some(Color { a: 0.18, ..accent }.into()),
                        border: Border {
                            color: Color { a: 0.7, ..accent },
                            width: 1.0,
                            radius: 3.0.into(),
                        },
                        ..Default::default()
                    }),
                )
                .padding(iced::Padding {
                    top: y,
                    left: x,
                    right: 0.0,
                    bottom: 0.0,
                }),
            );
        }
        col = col.push(plan);
        col = col.push(small(if cur.grid == Grid::One {
            "Découper un écran large en zones : chaque fenêtre garde sa zone, et \
             maximiser ou mettre en plein écran ne couvre que sa zone."
        } else {
            "Chaque fenêtre garde sa zone (celle sous la souris à l'ouverture) ; maximiser \
             ou mettre en plein écran ne couvre que sa zone. Changer de zone : SUPER + CTRL + \
             SHIFT + flèche (Réglages → Raccourcis). Les zones changent ensemble avec le \
             workspace."
        }));
        col.into()
    }
    /// Relit les écrans auprès de Hyprland.
    pub(in crate::ui) fn detect_screens() -> Task<UiMsg> {
        Task::perform(
            async {
                tokio::task::spawn_blocking(screens::detect)
                    .await
                    .ok()
                    .and_then(Result::ok)
                    .unwrap_or_default()
            },
            UiMsg::Screens,
        )
    }
    /// Écran sélectionné dans l'onglet, sinon le principal.
    pub(in crate::ui) fn screen_sel(&self) -> Option<String> {
        let sel = self
            .panel
            .as_ref()
            .and_then(|p| p.screens.screen_sel.clone());
        let known = |n: &String| self.screens.iter().any(|s| &s.name == n);
        sel.filter(known)
            .or_else(|| self.cfg.general.primary.clone().filter(known))
            .or_else(|| self.primary_monitor().filter(known))
            .or_else(|| self.screens.first().map(|s| s.name.clone()))
    }
    /// Où est placé un écran : (référence, côté, alignement).
    pub(in crate::ui) fn screen_placement(&self, name: &str) -> Option<(String, Side, Align)> {
        let l = &self.screens;
        let i = l.iter().position(|s| s.name == name)?;
        let a = screens::anchor(l, self.cfg.general.primary.as_deref())?;
        let linked = (i != a)
            .then(|| screens::links(l, a))
            .and_then(|k| k.iter().find(|k| k.screen == i).map(|k| k.reference));
        let r = linked.or_else(|| {
            l.iter()
                .enumerate()
                .filter(|(j, s)| *j != i && s.enabled)
                .min_by_key(|(_, s)| screens::gap(&l[i], s))
                .map(|(j, _)| j)
        })?;
        let (side, align) = screens::relation(&l[i], &l[r]).unwrap_or((Side::Right, Align::Start));
        Some((l[r].name.clone(), side, align))
    }
    /// Modifie la disposition et l'applique à chaud ; elle attend ensuite
    /// d'être gardée, sinon elle est rétablie à l'échéance.
    pub(in crate::ui) fn edit_screens(
        &mut self,
        f: impl FnOnce(&mut Vec<Screen>, &[screens::Link]),
    ) -> Task<UiMsg> {
        let before = self.screens.clone();
        if before.is_empty() {
            return Task::none();
        }
        let primary = self.cfg.general.primary.clone();
        let mut next = before.clone();
        let links = screens::anchor(&next, primary.as_deref())
            .map(|a| screens::links(&next, a))
            .unwrap_or_default();
        f(&mut next, &links);
        if screens::overlapping(&next) {
            screens::auto_arrange(&mut next, primary.as_deref());
        }
        screens::normalize(&mut next, primary.as_deref());
        if next == before {
            return Task::none();
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(SCREENS_CONFIRM_SECS);
        match &mut self.screens_pending {
            Some((_, d)) => *d = deadline,
            None => self.screens_pending = Some((before, deadline)),
        }
        self.screens = next.clone();
        apply_screens(next)
    }
    /// Place un écran à une position logique (glissé, réglage fin) ; refusé
    /// s'il chevaucherait un autre écran (la disposition automatique le
    /// déplacerait ailleurs).
    pub(in crate::ui) fn move_screen(&mut self, name: &str, pos: (i32, i32)) -> Task<UiMsg> {
        let mut next = self.screens.clone();
        if let Some(s) = next.iter_mut().find(|s| s.name == name) {
            (s.x, s.y) = pos;
        }
        if screens::overlapping(&next) {
            hypr::notify(
                3,
                3000,
                crate::i18n::tr("hyprdesk : les écrans se chevaucheraient — position refusée"),
            );
            return Task::none();
        }
        let name = name.to_string();
        self.edit_screens(move |l, _| {
            if let Some(s) = l.iter_mut().find(|s| s.name == name) {
                (s.x, s.y) = pos;
            }
        })
    }

    pub(in crate::ui) fn edit_selected(&mut self, f: impl FnOnce(&mut Screen)) -> Task<UiMsg> {
        let Some(sel) = self.screen_sel() else {
            return Task::none();
        };
        self.edit_screens(|l, links| {
            if let Some(s) = l.iter_mut().find(|s| s.name == sel) {
                f(s);
            }
            // Taille changée : chaque écran reste contre son voisin.
            screens::replay(l, links);
        })
    }
    pub(in crate::ui) fn place_selected(
        &mut self,
        f: impl FnOnce(&mut (String, Side, Align)),
    ) -> Task<UiMsg> {
        let Some(sel) = self.screen_sel() else {
            return Task::none();
        };
        let Some(mut p) = self.screen_placement(&sel) else {
            return Task::none();
        };
        f(&mut p);
        if p.0 == sel {
            return Task::none();
        }
        self.edit_screens(|l, _| {
            let Some(r) = l.iter().find(|s| s.name == p.0).cloned() else {
                return;
            };
            if let Some(s) = l.iter_mut().find(|s| s.name == sel) {
                screens::place(s, &r, p.1, p.2);
            }
        })
    }
    /// Garde la disposition : écrite dans monitors.lua.
    pub(in crate::ui) fn screens_keep(&mut self) -> Task<UiMsg> {
        self.screens_pending = None;
        let (list, path) = (
            self.screens.clone(),
            screens::monitors_path(&self.config_path),
        );
        if list.is_empty() {
            return Task::none();
        }
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || screens::save(&path, &list))
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|r| r.map_err(|e| e.to_string()))
            },
            |r| {
                match r {
                    Ok(()) => hypr::notify(
                        5,
                        3000,
                        crate::i18n::tr("hyprdesk : disposition des écrans enregistrée"),
                    ),
                    Err(e) => {
                        tracing::error!("{}", crate::tr!("monitors.lua : {e}", e = e));
                        hypr::notify(
                            3,
                            6000,
                            &crate::tr!("hyprdesk : monitors.lua non écrit — {e}", e = e),
                        );
                    }
                }
                UiMsg::Noop
            },
        )
    }
    pub(in crate::ui) fn screens_revert(&mut self) -> Task<UiMsg> {
        let Some((previous, _)) = self.screens_pending.take() else {
            return Task::none();
        };
        self.screens = previous.clone();
        apply_screens(previous)
    }
    /// Échéance passée sans confirmation : retour à la disposition d'avant.
    pub(in crate::ui) fn screens_tick(&mut self) -> Task<UiMsg> {
        match &self.screens_pending {
            Some((_, deadline)) if std::time::Instant::now() >= *deadline => {
                hypr::notify(
                    1,
                    4000,
                    crate::i18n::tr("hyprdesk : disposition des écrans rétablie"),
                );
                self.screens_revert()
            }
            _ => Task::none(),
        }
    }

    pub(in crate::ui) fn screens_update(&mut self, msg: Msg) -> Task<UiMsg> {
        let Some(panel) = self.panel.as_mut() else {
            return Task::none();
        };
        match msg {
            Msg::ScreenSelect(name) => {
                panel.screens.screen_sel = Some(name);
                // Choisi sur le plan : ses réglages sont dans l'onglet « Écran ».
                self.subtabs.insert(INFO.id, "ecran");
                Task::none()
            }
            Msg::ScreenPrimary(name) => {
                self.settings.primary = Some(name);
                let saved = self.commit();
                // L'écran principal est l'ancre de la disposition, en 0×0.
                let moved = self.edit_screens(|_, _| {});
                Task::batch([saved, moved])
            }
            Msg::ScreenEnable(on) => {
                let Some(sel) = self.screen_sel() else {
                    return Task::none();
                };
                let others_on = self.screens.iter().any(|s| s.name != sel && s.enabled);
                if !on && !others_on {
                    crate::hypr::notify(
                        3,
                        4000,
                        crate::i18n::tr("hyprdesk : au moins un écran doit rester actif"),
                    );
                    return Task::none();
                }
                self.edit_screens(|l, _| {
                    if let Some(s) = l.iter_mut().find(|s| s.name == sel) {
                        s.enabled = on;
                    }
                })
            }
            Msg::ScreenSide(side) => self.place_selected(|p| p.1 = side),
            Msg::ScreenRef(r) => self.place_selected(|p| p.0 = r),
            Msg::ScreenAlign(align) => self.place_selected(|p| p.2 = align),
            Msg::ScreenRotate(deg) => self.edit_selected(|s| {
                let flip = s.flipped();
                s.set_orientation(deg, flip);
            }),
            Msg::ScreenFlip(flip) => self.edit_selected(|s| {
                let rot = s.rotation();
                s.set_orientation(rot, flip);
            }),
            Msg::ScreenMode(m) => self.edit_selected(|s| {
                s.set_mode(&m);
            }),
            Msg::ScreenScale(v) => self.edit_selected(|s| s.scale = v),
            Msg::PanoramaZone(z) => {
                let mut p = self.panorama_cfg();
                p.zone = z;
                self.settings.panorama = Some(p);
                self.commit()
            }
            Msg::ScreenZones(name, split) => {
                let mut m = self
                    .settings
                    .screen_zones
                    .clone()
                    .unwrap_or_else(|| self.cfg.general.screen_zones.clone());
                if split.grid == crate::zones::Grid::One {
                    m.remove(&name);
                } else {
                    m.insert(name, split);
                }
                let lua = crate::zones::screens_lua(&m);
                self.settings.screen_zones = (!m.is_empty()).then_some(m);
                let saved = self.commit();
                // À chaud : layouts et règles par écran (le fichier suit).
                let live = Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || hyprpanel_core::ipc::eval(&lua))
                            .await
                            .ok()
                            .and_then(Result::ok)
                    },
                    |_| UiMsg::Noop,
                );
                Task::batch([saved, live])
            }
            Msg::ScreensAuto => {
                let primary = self.cfg.general.primary.clone();
                self.edit_screens(move |l, _| crate::screens::auto_arrange(l, primary.as_deref()))
            }
            Msg::ScreensDetect => Self::detect_screens(),
            Msg::Identify => self.identify_screens(),
            Msg::IdentifyEnd(generation) => {
                if generation != self.identify_gen {
                    return Task::none();
                }
                self.close_identify()
            }
            Msg::MapMove(p) => {
                panel.screens.cursor = p;
                let Some(d) = panel.screens.drag.as_mut() else {
                    return Task::none();
                };
                let (lx, ly) = d.geom.to_logical(p);
                let (x, y) = (
                    (lx - d.grab.0).round() as i32,
                    (ly - d.grab.1).round() as i32,
                );
                // Aimant : ~10 px du plan, au moins 8 px logiques.
                let threshold = ((10.0 / d.geom.k) as i32).max(8);
                let (x, y, sx, sy) = if self.mods.shift() {
                    (x, y, false, false)
                } else {
                    screens::snap(&self.screens, &d.name, x, y, threshold)
                };
                d.pos = (x, y);
                d.snapped = (sx, sy);
                Task::none()
            }
            Msg::DragStart(name) => {
                let on: Vec<&Screen> = self.screens.iter().filter(|s| s.enabled).collect();
                let geom = geometry(&on);
                let Some(s) = self.screens.iter().find(|s| s.name == name) else {
                    return Task::none();
                };
                let (lx, ly) = geom.to_logical(panel.screens.cursor);
                panel.screens.screen_sel = Some(name.clone());
                panel.screens.drag = Some(Drag {
                    name,
                    geom,
                    grab: (lx - s.x as f32, ly - s.y as f32),
                    start: (s.x, s.y),
                    pos: (s.x, s.y),
                    snapped: (false, false),
                });
                Task::none()
            }
            Msg::DragEnd => {
                let Some(d) = panel.screens.drag.take() else {
                    return Task::none();
                };
                if d.pos == d.start {
                    // Simple clic : l'écran choisi, ses réglages dans « Écran ».
                    self.subtabs.insert(INFO.id, "ecran");
                    return Task::none();
                }
                self.move_screen(&d.name, d.pos)
            }
            Msg::Nudge(dx, dy) => {
                let Some(sel) = self.screen_sel() else {
                    return Task::none();
                };
                let step = if self.mods.shift() { 10 } else { 1 };
                let Some(s) = self.screens.iter().find(|s| s.name == sel) else {
                    return Task::none();
                };
                let pos = (s.x + dx * step, s.y + dy * step);
                self.move_screen(&sel, pos)
            }
            Msg::ScreensKeep => self.screens_keep(),
            Msg::ScreensRevert => self.screens_revert(),
        }
    }
}

impl App {
    /// Repères d'alignement sur chaque écran actif (MLG-147) : numéro, nom
    /// et modèle au centre, bandes sur les bords partagés avec chaque voisin,
    /// lignes graduées aux mêmes coordonnées globales sur tous les écrans.
    /// Surfaces plein écran qui laissent passer la souris ; un second clic
    /// les retire (et au bout de 2 min). Elles suivent la disposition en
    /// direct pendant un glissé sur le plan.
    pub(in crate::ui) fn identify_screens(&mut self) -> Task<UiMsg> {
        use iced_layershell::reexport::{
            Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
        };
        self.identify_gen = self.identify_gen.wrapping_add(1);
        let open = !self.identify.is_empty();
        let mut tasks: Vec<Task<UiMsg>> = std::mem::take(&mut self.identify)
            .into_iter()
            .map(|(id, _)| iced::window::close(id))
            .collect();
        if open {
            return Task::batch(tasks);
        }
        for (i, s) in self.screens.iter().enumerate().filter(|(_, s)| s.enabled) {
            let id = iced::window::Id::unique();
            self.identify
                .push((id, format!("{}\n{}\n{}", i + 1, s.name, s.description)));
            tasks.push(Task::done(UiMsg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: None,
                    layer: Layer::Overlay,
                    anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    output_option: OutputOption::OutputName(s.name.clone()),
                    namespace: Some("hyprdesk-identify".into()),
                    ..Default::default()
                },
                id,
            }));
            // Aucune zone de saisie : clics et survol passent au travers.
            tasks.push(Task::done(UiMsg::SetInputRegion {
                id,
                callback: iced_layershell::actions::ActionCallback::new(|_region| {}),
            }));
        }
        let generation = self.identify_gen;
        tasks.push(Task::perform(
            async { tokio::time::sleep(Duration::from_secs(120)).await },
            move |_| wrap(Msg::IdentifyEnd(generation)),
        ));
        Task::batch(tasks)
    }

    /// Retire les repères d'« Identifier » (fin du délai, ou Réglages
    /// refermés : ils ne leur survivent pas).
    pub(in crate::ui) fn close_identify(&mut self) -> Task<UiMsg> {
        self.identify_gen = self.identify_gen.wrapping_add(1);
        Task::batch(
            std::mem::take(&mut self.identify)
                .into_iter()
                .map(|(id, _)| iced::window::close(id)),
        )
    }

    /// Repères d'un écran : lignes graduées et bords partagés (canvas), et
    /// au centre la carte numéro, connecteur, modèle.
    pub(in crate::ui) fn view_identify(&self, label: &str) -> Element<'_, UiMsg> {
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let mut parts = label.splitn(3, '\n');
        let (n, name, model) = (
            parts.next().unwrap_or_default().to_string(),
            parts.next().unwrap_or_default().to_string(),
            parts.next().unwrap_or_default().to_string(),
        );
        let bg = color(th.bg, 0.88);
        let card = container(
            column![
                text(n)
                    .font(self.title_font())
                    .size(crate::ui::tokens::ts(96.0))
                    .color(accent),
                text(name.clone())
                    .font(self.title_font())
                    .size(crate::ui::tokens::ts(28.0))
                    .color(fg),
                text(model)
                    .font(self.font)
                    .size(crate::ui::tokens::ts(14.0))
                    .color(Color { a: 0.7, ..fg }),
                text("Repères d'alignement : Identifier à nouveau pour les retirer.")
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(Color { a: 0.6, ..fg }),
            ]
            .spacing(4)
            .align_x(Alignment::Center),
        )
        .padding(24)
        .style(move |_| container::Style {
            background: Some(bg.into()),
            border: Border::default().rounded(18).width(2).color(accent),
            ..Default::default()
        });
        let Some(s) = self.screens.iter().find(|s| s.name == name) else {
            return container(card).center(Length::Fill).into();
        };
        let number = |n: &str| {
            self.screens
                .iter()
                .position(|o| o.name == n)
                .map_or(String::new(), |i| format!("{} · {n}", i + 1))
        };
        let guides = Guides {
            origin: (s.x, s.y),
            edges: screens::shared_edges(&self.screens, &name)
                .into_iter()
                .map(|e| (e.side, e.from, e.to, number(&e.neighbor)))
                .collect(),
            accent,
            fg,
        };
        iced::widget::Stack::with_children([
            iced::widget::canvas(guides)
                .width(Length::Fill)
                .height(Length::Fill)
                .into(),
            container(card).center(Length::Fill).into(),
        ])
        .into()
    }
}

/// Repères dessinés sur un écran pendant « Identifier » (MLG-147).
struct Guides {
    /// Position de l'écran dans la disposition (px logiques).
    origin: (i32, i32),
    /// Bords partagés : côté, segment local, voisin (« 2 · HDMI-A-2 »).
    edges: Vec<(Side, i32, i32, String)>,
    accent: Color,
    fg: Color,
}

impl iced::widget::canvas::Program<UiMsg> for Guides {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &iced::Renderer,
        _: &iced::Theme,
        bounds: iced::Rectangle,
        _: iced::mouse::Cursor,
    ) -> Vec<iced::widget::canvas::Geometry> {
        use iced::widget::canvas::{Frame, Path, Stroke, Text};
        use iced::{Point, Size};
        let mut f = Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        let (gx, gy) = self.origin;
        let label = |f: &mut Frame, s: String, p: Point, c: Color| {
            f.fill_text(Text {
                content: s,
                position: p,
                color: c,
                size: iced::Pixels(13.0),
                ..Default::default()
            });
        };
        // Lignes graduées en coordonnées globales : toutes les 100 px, plus
        // marquées toutes les 500 ; les mêmes valeurs sur chaque écran
        // permettent de vérifier qu'elles se prolongent d'un écran à l'autre.
        let first = |g: i32| ((g as f32 / 100.0).ceil() as i32) * 100;
        let mut v = first(gy);
        while (v - gy) as f32 <= h {
            let y = (v - gy) as f32;
            let strong = v % 500 == 0;
            let c = Color {
                a: if strong { 0.45 } else { 0.15 },
                ..self.fg
            };
            f.stroke(
                &Path::line(Point::new(0.0, y), Point::new(w, y)),
                Stroke::default().with_width(1.0).with_color(c),
            );
            if strong {
                label(&mut f, format!("y {v}"), Point::new(6.0, y + 2.0), c);
                label(&mut f, format!("y {v}"), Point::new(w - 64.0, y + 2.0), c);
            }
            v += 100;
        }
        let mut u = first(gx);
        while (u - gx) as f32 <= w {
            let x = (u - gx) as f32;
            let strong = u % 500 == 0;
            let c = Color {
                a: if strong { 0.45 } else { 0.15 },
                ..self.fg
            };
            f.stroke(
                &Path::line(Point::new(x, 0.0), Point::new(x, h)),
                Stroke::default().with_width(1.0).with_color(c),
            );
            if strong {
                label(&mut f, format!("x {u}"), Point::new(x + 4.0, 6.0), c);
                label(&mut f, format!("x {u}"), Point::new(x + 4.0, h - 22.0), c);
            }
            u += 100;
        }
        // Bords partagés : bande d'accent sur le segment commun, et le voisin.
        const BAND: f32 = 10.0;
        for (side, from, to, who) in &self.edges {
            let (a, b) = (*from as f32, *to as f32);
            let (rect, text_at, arrow) = match side {
                Side::Right => (
                    (Point::new(w - BAND, a), Size::new(BAND, b - a)),
                    Point::new(w - 220.0, (a + b) / 2.0),
                    "→",
                ),
                Side::Left => (
                    (Point::new(0.0, a), Size::new(BAND, b - a)),
                    Point::new(BAND + 8.0, (a + b) / 2.0),
                    "←",
                ),
                Side::Below => (
                    (Point::new(a, h - BAND), Size::new(b - a, BAND)),
                    Point::new((a + b) / 2.0 - 60.0, h - BAND - 24.0),
                    "↓",
                ),
                Side::Above => (
                    (Point::new(a, 0.0), Size::new(b - a, BAND)),
                    Point::new((a + b) / 2.0 - 60.0, BAND + 8.0),
                    "↑",
                ),
            };
            f.fill_rectangle(rect.0, rect.1, self.accent);
            label(&mut f, format!("{arrow} {who}"), text_at, self.accent);
            // Bornes du segment : où le voisin commence et finit.
            for (p, q) in match side {
                Side::Right | Side::Left => {
                    let x = if *side == Side::Right { w - 40.0 } else { 0.0 };
                    vec![
                        (Point::new(x, a), Point::new(x + 40.0, a)),
                        (Point::new(x, b), Point::new(x + 40.0, b)),
                    ]
                }
                Side::Above | Side::Below => {
                    let y = if *side == Side::Below { h - 40.0 } else { 0.0 };
                    vec![
                        (Point::new(a, y), Point::new(a, y + 40.0)),
                        (Point::new(b, y), Point::new(b, y + 40.0)),
                    ]
                }
            } {
                f.stroke(
                    &Path::line(p, q),
                    Stroke::default().with_width(3.0).with_color(self.accent),
                );
            }
        }
        vec![f.into_geometry()]
    }
}
