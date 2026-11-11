//! Problème signalé au démon (`control::Command::Problem`, par exemple une
//! mise à jour qui a échoué) : soumis à l'assistant LYRA s'il sait se faire
//! dessiner ici ; sinon une fenêtre du thème montre le détail, avec un bouton
//! pour le copier.

use iced::widget::{button, column, container, row, scrollable, text};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Shadow, Task, Vector};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::{App, Msg, color};

pub const NAMESPACE: &str = "hyprdesk-problem";
const WIDTH: f32 = 620.0;
const HEIGHT: f32 = 440.0;
const SHADOW: f32 = 18.0;

pub struct Problem {
    pub win: Id,
    pub text: String,
    pub copied: bool,
}

#[derive(Debug, Clone)]
pub enum PrMsg {
    Copy,
    Copied,
    Close,
}

impl App {
    /// Un problème arrive : à l'assistant s'il est là, sinon à la fenêtre.
    pub(super) fn show_problem(&mut self, text: String, show: bool) -> Task<Msg> {
        let origin = crate::i18n::tr("Mises à jour").to_string();
        if let Some(t) = self.ask_assistant(text.clone(), &origin, show) {
            return t;
        }
        if let Some(p) = self.problem.as_mut() {
            p.text = text;
            p.copied = false;
            return Task::none();
        }
        let win = Id::unique();
        self.problem = Some(Problem {
            win,
            text,
            copied: false,
        });
        Task::done(Msg::NewLayerShell {
            settings: NewLayerShellSettings {
                size: Some((
                    (WIDTH + 2.0 * SHADOW) as u32,
                    (HEIGHT + 2.0 * SHADOW) as u32,
                )),
                layer: Layer::Overlay,
                anchor: LsAnchor::empty(),
                exclusive_zone: Some(-1),
                keyboard_interactivity: KeyboardInteractivity::OnDemand,
                output_option: OutputOption::Active,
                namespace: Some(NAMESPACE.into()),
                ..Default::default()
            },
            id: win,
        })
    }

    pub(super) fn problem_update(&mut self, m: PrMsg) -> Task<Msg> {
        match m {
            PrMsg::Copy => {
                let Some(p) = &self.problem else {
                    return Task::none();
                };
                let t = p.text.clone();
                Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || {
                            use std::io::Write;
                            if let Ok(mut c) = std::process::Command::new("wl-copy")
                                .stdin(std::process::Stdio::piped())
                                .spawn()
                            {
                                if let Some(i) = c.stdin.as_mut() {
                                    let _ = i.write_all(t.as_bytes());
                                }
                                drop(c.stdin.take());
                                let _ = c.wait();
                            }
                        })
                        .await;
                    },
                    |_| Msg::Problem(PrMsg::Copied),
                )
            }
            PrMsg::Copied => {
                if let Some(p) = self.problem.as_mut() {
                    p.copied = true;
                }
                Task::none()
            }
            PrMsg::Close => match self.problem.take() {
                Some(p) => iced::window::close(p.win),
                None => Task::none(),
            },
        }
    }

    pub(super) fn view_problem(&self) -> Element<'_, Msg> {
        let Some(p) = &self.problem else {
            return iced::widget::Space::new().into();
        };
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let dim = Color { a: 0.65, ..fg };
        let bg = color(
            th.bg,
            th.effect(crate::config::Place::Menus).opacity.max(0.97),
        );
        let detail = container(
            scrollable(
                text(p.text.clone())
                    .font(iced::Font::MONOSPACE)
                    .size(12)
                    .color(fg),
            )
            .height(Length::Fill),
        )
        .padding(10)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Color { a: 0.06, ..fg }.into()),
            border: Border::default().rounded(10),
            ..Default::default()
        });
        let col = column![
            text(crate::i18n::tr("Un problème est survenu"))
                .font(self.title_font())
                .size(20)
                .color(fg),
            text(crate::i18n::tr(
                "Le détail ci-dessous peut être copié pour le chercher ou le confier à un assistant."
            ))
            .font(self.font)
            .size(12)
            .color(dim),
            detail,
            row![
                text(if p.copied {
                    crate::i18n::tr("Copié dans le presse-papiers.")
                } else {
                    ""
                })
                .font(self.font)
                .size(12)
                .color(dim)
                .width(Length::Fill),
                button(text(crate::i18n::tr("Fermer")).font(self.font).size(14))
                    .padding([8, 16])
                    .style(button::text)
                    .on_press(Msg::Problem(PrMsg::Close)),
                button(text(crate::i18n::tr("Copier")).font(self.font).size(14))
                    .padding([8, 16])
                    .on_press(Msg::Problem(PrMsg::Copy)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(12);
        let card = container(col)
            .padding(22)
            .width(WIDTH)
            .height(HEIGHT)
            .style(move |_| container::Style {
                background: Some(bg.into()),
                border: Border::default()
                    .rounded(super::design::RADIUS)
                    .width(1)
                    .color(Color { a: 0.12, ..fg }),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
                    offset: Vector::new(0.0, 8.0),
                    blur_radius: 22.0,
                },
                text_color: Some(fg),
                ..Default::default()
            });
        container(card).padding(SHADOW).into()
    }
}
