//! Problème signalé au démon (`control::Command::Problem`, par exemple une
//! mise à jour qui a échoué) : une fenêtre du thème montre le détail, avec un
//! bouton pour le copier et, si LYRA est là, un bouton pour le lui confier.
//! Rien ne part à l'assistant sans ce geste ; une demande explicite
//! (`ProblemShow`, `Ask`) va droit à LYRA.

use crate::ui::tokens::Shown as _;
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
    /// Titre de la conversation LYRA si l'utilisateur la lui confie.
    pub origin: String,
}

#[derive(Debug, Clone)]
pub enum PrMsg {
    Copy,
    Copied,
    /// Confier le problème à LYRA (geste de l'utilisateur).
    Ask,
    Close,
}

impl App {
    /// Un problème arrive : à l'assistant s'il est là, sinon à la fenêtre.
    pub(super) fn show_problem(&mut self, text: String, show: bool) -> Task<Msg> {
        let origin = crate::i18n::tr("Mises à jour").to_string();
        self.show_problem_from(&origin, text, show)
    }

    /// Même chose, au nom de `origin` (titre de la conversation LYRA).
    pub(super) fn show_problem_from(
        &mut self,
        origin: &str,
        text: String,
        show: bool,
    ) -> Task<Msg> {
        // Sans demande explicite, rien ne part à l'assistant (service en
        // ligne possible) : la fenêtre propose de le lui confier.
        if show && let Some(t) = self.ask_assistant(text.clone(), origin, true) {
            return t;
        }
        if let Some(p) = self.problem.as_mut() {
            p.text = text;
            p.copied = false;
            p.origin = origin.to_string();
            return Task::none();
        }
        let win = Id::unique();
        self.problem = Some(Problem {
            win,
            text,
            copied: false,
            origin: origin.to_string(),
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
            PrMsg::Ask => {
                let Some(p) = self.problem.take() else {
                    return Task::none();
                };
                let close = iced::window::close(p.win);
                match self.ask_assistant(p.text, &p.origin, true) {
                    Some(t) => Task::batch([close, t]),
                    None => close,
                }
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
            th.shown(crate::config::Place::Menus).opacity.max(0.97),
        );
        let lyra =
            self.cfg.plugin_on("assistant") && crate::assistant::serve_command(&self.cfg).is_some();
        let detail = container(
            scrollable(
                text(p.text.clone())
                    .font(iced::Font::MONOSPACE)
                    .size(crate::ui::tokens::ts(12.0))
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
                .size(crate::ui::tokens::ts(20.0))
                .color(fg),
            text(crate::i18n::tr(
                "Le détail ci-dessous peut être copié pour le chercher ou le confier à un assistant."
            ))
            .font(self.font)
            .size(crate::ui::tokens::ts(12.0))
            .color(dim),
            detail,
            row![
                text(if p.copied {
                    crate::i18n::tr("Copié dans le presse-papiers.")
                } else {
                    ""
                })
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim)
                .width(Length::Fill),
                button(text(crate::i18n::tr("Fermer")).font(self.font).size(crate::ui::tokens::ts(14.0)))
                    .padding([8, 16])
                    .style(button::text)
                    .on_press(Msg::Problem(PrMsg::Close)),
                button(text(crate::i18n::tr("Copier")).font(self.font).size(crate::ui::tokens::ts(14.0)))
                    .padding([8, 16])
                    .style(if lyra { button::secondary } else { button::primary })
                    .on_press(Msg::Problem(PrMsg::Copy)),
            ]
            .push(lyra.then(|| {
                button(text(crate::i18n::tr("Demander à LYRA")).font(self.font).size(crate::ui::tokens::ts(14.0)))
                    .padding([8, 16])
                    .on_press(Msg::Problem(PrMsg::Ask))
            }))
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Échap ferme l'alerte, comme son bouton (HIG PRES-01).
    #[test]
    fn echap_ferme_l_alerte() {
        let mut app = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-ui-test/config.lua"),
            true,
        )
        .0;
        let _ = app.show_problem_from("Mises à jour", "détail du problème".into(), false);
        assert!(app.problem.is_some());
        let _ = app.update_inner(Msg::Key(
            Id::unique(),
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
        ));
        assert!(app.problem.is_none());
    }
}
