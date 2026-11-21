//! Modules du plugin Panorama, côté écran : la bande du profil Raccourcis
//! (modules côte à côte, défilement de côté, tuile « + » et catalogue) et
//! une tuile par module. La logique est dans `crate::panorama::modules`.
mod hue;
mod notes;
mod voice;

use std::time::Instant;

use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Border, Element, Length, Size, Task};

use super::{App, Msg, PMsg, fx, wrap};

impl App {
    /// Messages des modules, répartis entre eux.
    pub(super) fn panorama_modules_update(&mut self, m: PMsg) -> Task<Msg> {
        match m {
            PMsg::HueRooms(..)
            | PMsg::HueSet(..)
            | PMsg::HueLight(..)
            | PMsg::HueDrag(..)
            | PMsg::HueLightDrag(..)
            | PMsg::HueFocus(_)
            | PMsg::HueRelease => self.panorama_hue_update(m),
            PMsg::NotesOpen | PMsg::NotesOpened(_) => self.panorama_notes_update(m),
            PMsg::ModulesEdit(on) => {
                self.panorama.last_press = Some(Instant::now());
                self.panorama.modules_edit = on;
                Task::none()
            }
            PMsg::ModuleToggle(id) => {
                self.panorama.last_press = Some(Instant::now());
                let save = self.panorama_save_cfg(|c| c.design.toggle_module(&id));
                // Tuile Hue ajoutée : relevé aussitôt.
                let hue = if id == "hue" {
                    self.panorama_hue_refresh()
                } else {
                    Task::none()
                };
                Task::batch([save, hue])
            }
            _ => Task::none(),
        }
    }

    /// Tuile d'un module du catalogue, à la taille donnée (bande des
    /// modules, zone d'accueil du côte à côte).
    pub(super) fn panorama_module_body<'a>(&'a self, id: &str, inner: Size) -> Element<'a, Msg> {
        let (_, fg, accent) = self.panorama_palette();
        let d = &self.cfg.general.panorama.design;
        match id {
            "notes" => self.panorama_notes(inner),
            "hue" => self.panorama_hue(inner),
            "voice" => self.panorama_voice_module(inner),
            "playlist" => self.panorama_playlist(inner),
            "calendar" => self.panorama_calendar(inner),
            "media" => self.panorama_compact_media(inner),
            "clock" => {
                let z = crate::panorama::layout::zones(inner.width, inner.height, None);
                self.panorama_clock(&z, fg)
            }
            "system" => super::instruments::metrics(
                &d.system_style,
                &self.panorama.sys,
                &self.panorama.history,
                d.history_seconds as usize,
                (fg, accent),
                self.panorama_font(),
                d.text_scale,
                self.panorama_graph_phase(),
            ),
            _ => Space::new().into(),
        }
    }

    /// Bande du profil Raccourcis : les modules côte à côte (largeur selon
    /// le module), qui défilent de côté, puis une tuile « + » pour en
    /// ajouter ou retirer (catalogue à cocher).
    pub(super) fn panorama_modules<'a>(&'a self, size: Size) -> Element<'a, Msg> {
        let (bg, fg, accent) = self.panorama_palette();
        let d = &self.cfg.general.panorama.design;
        let gap = d.gap.min(16.0);
        // Hauteur des tuiles : la zone, barre de défilement (sous) comprise.
        let h = (size.height - 14.0).max(60.0);
        let card = move |body: Element<'a, Msg>, w: f32| -> Element<'a, Msg> {
            container(body)
                .width(w)
                .height(h)
                .clip(true)
                .style(move |_| container::Style {
                    background: Some(fx::alpha(bg, d.card_opacity.max(0.06)).into()),
                    border: Border {
                        radius: d.radius.min(18.0).into(),
                        width: 1.0,
                        color: fx::alpha(fg, d.border_opacity.max(0.12)),
                    },
                    ..Default::default()
                })
                .into()
        };
        let square =
            |label: String, icon: &str, on: bool, side: f32, msg: Msg| -> Element<'a, Msg> {
                // Glyphe intégré, teinté (pas d'icône du thème : elle peut manquer).
                let g = side * 0.3;
                let glyph: Element<'a, Msg> = match crate::ui::icons::get(icon) {
                    Some(h) => iced::widget::svg(h)
                        .width(g)
                        .height(g)
                        .style(move |_, _| iced::widget::svg::Style { color: Some(fg) })
                        .into(),
                    None => text(if icon == "list-add" { "+" } else { "•" })
                        .size(g * 1.2)
                        .color(fg)
                        .into(),
                };
                self.panorama_feedback(
                    button(
                        container(
                            column![
                                glyph,
                                text(label)
                                    .size(13.0 * d.text_scale)
                                    .color(fg)
                                    .wrapping(text::Wrapping::None)
                            ]
                            .spacing(8)
                            .align_x(Alignment::Center),
                        )
                        .center_x(Length::Fill)
                        .center_y(Length::Fill),
                    )
                    .width(side)
                    .height(side)
                    .padding(6)
                    .on_press(msg)
                    .style(move |_, st| button::Style {
                        background: Some(
                            fx::alpha(
                                accent,
                                if matches!(st, button::Status::Pressed) {
                                    0.55
                                } else if on {
                                    0.35
                                } else {
                                    0.1
                                },
                            )
                            .into(),
                        ),
                        border: Border {
                            radius: 14.0.into(),
                            width: if on { 2.0 } else { 1.0 },
                            color: fx::alpha(
                                if on { accent } else { fg },
                                if on { 0.9 } else { 0.2 },
                            ),
                        },
                        text_color: fg,
                        ..Default::default()
                    }),
                )
            };
        let mut strip = row![].spacing(gap).align_y(Alignment::Center);
        if self.panorama.modules_edit {
            // Catalogue : grille de petites tuiles sur trois lignes ; un appui
            // ajoute ou retire, « Terminé » referme.
            let current = d.modules();
            let side = ((h - 2.0 * gap) / 3.0).clamp(60.0, 150.0);
            let mut items: Vec<Element<'a, Msg>> = crate::panorama::modules::CATALOGUE
                .iter()
                .map(|(id, label, icon)| {
                    square(
                        crate::i18n::tr(label).to_string(),
                        icon,
                        current.iter().any(|m| m == id),
                        side,
                        wrap(PMsg::ModuleToggle(id.to_string())),
                    )
                })
                .collect();
            items.push(square(
                crate::i18n::tr("Terminé").to_string(),
                "object-select",
                false,
                side,
                wrap(PMsg::ModulesEdit(false)),
            ));
            let cols = items.len().div_ceil(3);
            let mut grid = column![].spacing(gap);
            let mut it = items.into_iter();
            for _ in 0..3 {
                let mut line = row![].spacing(gap);
                for e in it.by_ref().take(cols) {
                    line = line.push(e);
                }
                grid = grid.push(line);
            }
            return container(grid)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_y(Length::Fill)
                .into();
        } else {
            for m in d.modules() {
                // Largeur selon le module : Hue et Système plus larges.
                let w = match m.as_str() {
                    "hue" => h * 1.8,
                    "system" | "playlist" => h * 1.4,
                    "media" | "calendar" => h * 1.15,
                    _ => h,
                };
                let body = self.panorama_module_body(&m, Size::new(w, h));
                strip = strip.push(card(body, w));
            }
            strip = strip.push(square(
                crate::i18n::tr("Modules").to_string(),
                "list-add",
                false,
                (h * 0.42).min(150.0),
                wrap(PMsg::ModulesEdit(true)),
            ));
        }
        // Défile au doigt, n'importe où sur la bande (pas seulement sa barre).
        super::drag::drag_scroll(
            iced::widget::scrollable(strip)
                .width(Length::Fill)
                .height(Length::Fill)
                .direction(iced::widget::scrollable::Direction::Horizontal(
                    iced::widget::scrollable::Scrollbar::new()
                        .width(4)
                        .scroller_width(4)
                        .spacing(8),
                )),
        )
        .into()
    }
}
