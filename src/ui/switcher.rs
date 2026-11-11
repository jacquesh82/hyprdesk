//! Sélecteur de fenêtres (MLG-22, `hyprdesk switch`) : une carte du thème au
//! centre de l'écran actif, les fenêtres dans l'ordre du dernier focus.
//! Alt+Tab avance, Alt+Maj+Tab recule (le raccourci relance la commande, la
//! carte déjà ouverte change de sélection) ; relâcher la touche de
//! modification donne le focus à la fenêtre choisie ; Échap annule ; un clic
//! choisit directement.

use iced::widget::{button, column, container, row, text};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Shadow, Task, Vector};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::{App, IconImg, Msg, color};
use crate::switcher::Win;

pub const NAMESPACE: &str = "hyprdesk-switcher";
/// Fenêtres affichées au plus.
const SHOWN: usize = 10;
const WIDTH: f32 = 560.0;
const ROW: f32 = 46.0;
const SHADOW: f32 = 18.0;

pub struct Switcher {
    pub win: Id,
    pub wins: Vec<Win>,
    pub selected: usize,
    /// Fenêtres pas encore lues (la carte s'ouvre aussitôt).
    pub loading: bool,
    /// Sens demandé avant la lecture.
    pub back: bool,
    /// Vue d'ensemble (`hyprdesk overview`) : reste ouverte au relâchement
    /// des touches ; un clic, Entrée ou Échap la referme.
    pub sticky: bool,
}

#[derive(Debug, Clone)]
pub enum SwMsg {
    Loaded(Vec<Win>),
    Pick(usize),
}

fn height(n: usize) -> f32 {
    let rows = n.clamp(1, SHOWN) as f32;
    rows * ROW + 2.0 * 14.0 + 28.0
}

impl App {
    /// `hyprdesk switch [prev]` : ouvre la carte, ou avance la sélection.
    pub(super) fn switch_windows(&mut self, back: bool) -> Task<Msg> {
        if let Some(s) = self.switcher.as_mut() {
            let n = s.wins.len().max(1);
            s.selected = if back {
                (s.selected + n - 1) % n
            } else {
                (s.selected + 1) % n
            };
            return Task::none();
        }
        let win = Id::unique();
        self.switcher = Some(Switcher {
            win,
            wins: Vec::new(),
            selected: 0,
            loading: true,
            back,
            sticky: false,
        });
        let h = height(SHOWN) + 2.0 * SHADOW;
        // Plein bureau : aussi les fenêtres que le bureau a mises de côté.
        let full = self.mode() == super::DesktopMode::FullDesktop;
        Task::batch([
            Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some(((WIDTH + 2.0 * SHADOW) as u32, h as u32)),
                    layer: Layer::Overlay,
                    anchor: LsAnchor::empty(),
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option: OutputOption::Active,
                    namespace: Some(NAMESPACE.into()),
                    ..Default::default()
                },
                id: win,
            }),
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || crate::switcher::read_with(full))
                        .await
                        .unwrap_or_default()
                },
                |w| Msg::Switcher(SwMsg::Loaded(w)),
            ),
        ])
    }

    pub(super) fn close_switcher(&mut self) -> Task<Msg> {
        match self.switcher.take() {
            Some(s) => iced::window::close(s.win),
            None => Task::none(),
        }
    }

    /// Donne le focus à la fenêtre choisie et ferme la carte.
    /// Fenêtre choisie : elle prend le focus. En plein bureau, une fenêtre
    /// cachée par le bureau (ou d'un autre workspace) fait d'abord quitter le
    /// bureau (no-desktop : les fenêtres reviennent) ; une fenêtre ouverte sur
    /// le bureau prend le focus sans le quitter.
    pub(super) fn switcher_commit(&mut self) -> Task<Msg> {
        let chosen = self
            .switcher
            .as_ref()
            .and_then(|s| s.wins.get(s.selected))
            .map(|w| (w.address.clone(), w.stashed));
        let close = self.close_switcher();
        let Some((a, stashed)) = chosen else {
            return close;
        };
        let leave = self.mode() == super::DesktopMode::FullDesktop
            && (stashed || !self.session_windows.contains(&a));
        let focus = Task::perform(
            async move {
                let _ = tokio::task::spawn_blocking(move || crate::hypr::focus_window(&a)).await;
            },
            |_| Msg::Noop,
        );
        if leave {
            // Le bureau se masque (fenêtres rendues), puis le focus.
            return Task::batch([close, self.hide().chain(focus)]);
        }
        Task::batch([close, focus])
    }

    pub(super) fn switcher_update(&mut self, m: SwMsg) -> Task<Msg> {
        match m {
            SwMsg::Loaded(w) => {
                let Some(s) = self.switcher.as_mut() else {
                    return Task::none();
                };
                // Avancées faites pendant la lecture : gardées.
                let moved = s.selected;
                s.selected = (crate::switcher::initial(w.len(), s.back) + moved) % w.len().max(1);
                s.wins = w;
                s.loading = false;
                Task::none()
            }
            SwMsg::Pick(k) => {
                if let Some(s) = self.switcher.as_mut() {
                    s.selected = k;
                }
                self.switcher_commit()
            }
        }
    }

    pub(in crate::ui) fn view_switcher(&self) -> Element<'_, Msg> {
        let Some(s) = &self.switcher else {
            return iced::widget::Space::new().into();
        };
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let accent = color(th.accent, 1.0);
        let bg = color(
            th.bg,
            th.effect(crate::config::Place::Menus).opacity.max(0.97),
        );
        // En plein bureau, le choix d'une fenêtre cachée quitte le bureau.
        let title = if self.mode() == super::DesktopMode::FullDesktop {
            crate::i18n::tr("Fenêtres — le bureau se masque pour une fenêtre cachée")
        } else {
            crate::i18n::tr("Fenêtres")
        };
        let mut list =
            column![text(title).font(self.font).size(12).color(dim).height(28)].spacing(2);
        if s.wins.is_empty() {
            list = list.push(
                text(if s.loading {
                    crate::i18n::tr("Lecture des fenêtres…")
                } else {
                    crate::i18n::tr("Aucune fenêtre ouverte.")
                })
                .font(self.font)
                .size(13)
                .color(dim),
            );
        }
        // Fenêtre choisie toujours visible : on fait défiler par pages.
        let first = (s.selected / SHOWN) * SHOWN;
        for (k, w) in s.wins.iter().enumerate().skip(first).take(SHOWN) {
            let img = self
                .class_imgs
                .get(&w.class)
                .copied()
                .unwrap_or(IconImg::None);
            let icon: Element<'_, Msg> = self
                .img_element(img, 26.0, 1.0)
                .unwrap_or_else(|| iced::widget::Space::new().width(26).height(26).into());
            let selected = k == s.selected;
            list = list.push(
                button(
                    row![
                        icon,
                        column![
                            text(w.title.clone()).font(self.font).size(14).color(fg),
                            text(if w.stashed {
                                crate::tr!("{c} · cachée par le bureau", c = w.class)
                            } else {
                                format!("{} · {}", w.class, w.workspace)
                            })
                            .font(self.font)
                            .size(11)
                            .color(dim)
                        ]
                        .width(Length::Fill)
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                )
                .padding([4, 12])
                .width(Length::Fill)
                .height(ROW)
                .style(move |_, st| button::Style {
                    background: (selected || matches!(st, button::Status::Hovered)).then(|| {
                        Color {
                            a: if selected { 0.24 } else { 0.1 },
                            ..accent
                        }
                        .into()
                    }),
                    border: Border::default().rounded(10),
                    text_color: fg,
                    ..Default::default()
                })
                .on_press(Msg::Switcher(SwMsg::Pick(k))),
            );
        }
        let card = container(list)
            .padding(14)
            .width(WIDTH)
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
        container(card)
            .padding(SHADOW)
            .center_x(Length::Fill)
            .into()
    }
}
