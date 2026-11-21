//! Côte à côte (profils Vidéo et Musique) : l'écran du Panorama partagé en
//! deux, le média en cours d'un côté, de l'autre une zone d'accueil où l'on
//! ajoute (« + ») un ou plusieurs modules, ou une fenêtre ouverte ramenée
//! par Hyprland (`crate::panorama::guest`). Le bandeau de la zone inverse
//! les deux côtés ou revient sans partage (la fenêtre est alors rendue).

use iced::widget::{Space, button, column, container, row, scrollable, svg, text};
use iced::{Alignment, Border, Color, Element, Length, Size, Task};

use super::panes::{Geo, Pane, Surf};
use super::{App, Msg, PMsg, fx, wrap};
use crate::panorama::guest::Guest;
use crate::panorama::layout::Rect;

/// Modules au plus dans la zone d'accueil.
const MAX_MODULES: usize = 4;

/// État du côte à côte (pour la session).
#[derive(Debug, Clone, Default)]
pub struct SideBySide {
    /// Le média passe à droite (en bas sur un écran portrait).
    pub swapped: bool,
    /// Modules de la zone d'accueil, dans l'ordre.
    pub modules: Vec<String>,
    /// Fenêtre ramenée dans la zone d'accueil.
    pub guest: Option<Guest>,
    /// Rectangle global où elle a été posée en dernier.
    pub placed: Option<(i32, i32, i32, i32)>,
    /// Choix « + » ouvert (modules, fenêtres).
    pub picking: bool,
    /// Fenêtre en cours de transfert.
    pub busy: bool,
    pub error: Option<String>,
}

impl App {
    /// Zones du côte à côte sur la surface `s` : le média (`video`, ou
    /// `media` pour une musique sans fenêtre) et la zone d'accueil (`aside`).
    pub(super) fn panorama_side_panes(&self, s: &Surf, sp: &SideBySide) -> Vec<Pane> {
        let d = &self.cfg.general.panorama.design;
        let (pad, gap) = (d.padding, d.gap);
        let (iw, ih) = ((s.w - 2.0 * pad).max(2.0), (s.h - 2.0 * pad).max(2.0));
        let (a, b) = if s.w >= s.h {
            let w = ((iw - gap) / 2.0).max(1.0);
            (
                Rect {
                    x: pad,
                    y: pad,
                    w,
                    h: ih,
                },
                Rect {
                    x: pad + w + gap,
                    y: pad,
                    w,
                    h: ih,
                },
            )
        } else {
            let h = ((ih - gap) / 2.0).max(1.0);
            (
                Rect {
                    x: pad,
                    y: pad,
                    w: iw,
                    h,
                },
                Rect {
                    x: pad,
                    y: pad + h + gap,
                    w: iw,
                    h,
                },
            )
        };
        let (media, aside) = if sp.swapped { (b, a) } else { (a, b) };
        // Musique d'un lecteur sans fenêtre (MPRIS) : ses commandes ; sinon
        // le lecteur du Panorama (fenêtre, radio, ou choix des sources).
        let player = self.panorama.video.is_some() || self.panorama.radio.is_some();
        let playing = self.panorama.media.as_ref().is_some_and(|m| m.playing);
        let widget = if !player && playing { "media" } else { "video" };
        vec![
            Pane {
                rect: media,
                widget: widget.into(),
                index: None,
                bare: false,
            },
            Pane {
                rect: aside,
                widget: "aside".into(),
                index: None,
                bare: false,
            },
        ]
    }

    /// Une fenêtre occupe la zone d'accueil : elle y est visible (trou).
    pub(super) fn panorama_side_hosting(&self) -> bool {
        self.panorama
            .split
            .as_ref()
            .is_some_and(|sp| sp.guest.is_some())
    }

    /// Rectangle global de la zone d'accueil (première surface qui en montre une).
    pub(super) fn panorama_aside_target(&self) -> Option<(i32, i32, i32, i32)> {
        self.panorama_surfs().into_iter().find_map(|s| {
            let geo = self.panorama_geo(&s);
            let p = geo.panes.iter().find(|p| p.widget == "aside")?;
            self.panorama_global(&s, geo.content(p))
        })
    }

    /// Pose (ou rend) la fenêtre invitée selon la zone d'accueil affichée.
    pub(super) fn panorama_side_sync(&mut self, no_surface: bool) -> Task<Msg> {
        if no_surface {
            // Plus aucun Panorama : la fenêtre retrouve sa place.
            let guest = self.panorama.split.as_mut().and_then(|sp| {
                sp.placed = None;
                sp.guest.take()
            });
            return restore(guest);
        }
        let target = self.panorama_aside_target();
        let Some(sp) = self.panorama.split.as_mut() else {
            return Task::none();
        };
        match (sp.guest.clone(), target) {
            (Some(g), Some(rect)) if sp.placed != Some(rect) => {
                sp.placed = Some(rect);
                blocking(move || g.place(rect))
            }
            _ => Task::none(),
        }
    }

    /// Renouvelle le bail du cadre de la fenêtre invitée.
    pub(super) fn panorama_side_anchor(&self) -> Task<Msg> {
        match (
            self.panorama.split.as_ref().and_then(|sp| sp.guest.clone()),
            self.panorama_aside_target(),
        ) {
            (Some(g), Some(rect)) => blocking(move || g.renew(rect)),
            _ => Task::none(),
        }
    }

    /// Fenêtre invitée fermée hors du Panorama : on l'oublie.
    pub(super) fn panorama_side_gone(&mut self) {
        let clients = &self.clients;
        if let Some(sp) = self.panorama.split.as_mut()
            && sp
                .guest
                .as_ref()
                .is_some_and(|g| !clients.iter().any(|c| c.address == g.address))
        {
            sp.guest = None;
            sp.placed = None;
        }
    }

    pub(super) fn panorama_side_update(&mut self, m: PMsg) -> Task<Msg> {
        self.panorama.last_press = Some(std::time::Instant::now());
        match m {
            PMsg::Side(op) => {
                let op = match op {
                    "toggle" if self.panorama.split.is_some() => "off",
                    "toggle" => "on",
                    o => o,
                };
                match op {
                    "on" => {
                        if self.panorama.split.is_none() {
                            self.panorama.split = Some(SideBySide::default());
                        }
                        // Les profils Vidéo et Musique portent le côte à côte.
                        let profile = &self.cfg.general.panorama.design.shortcut_profile;
                        if !matches!(profile.as_str(), "video" | "audio") {
                            return self.panorama_update(PMsg::Profile("video".into()));
                        }
                        self.sync_pip()
                    }
                    "off" => {
                        let guest = self.panorama.split.take().and_then(|sp| sp.guest);
                        Task::batch([restore(guest), self.sync_pip()])
                    }
                    "swap" => {
                        if let Some(sp) = self.panorama.split.as_mut() {
                            sp.swapped = !sp.swapped;
                        }
                        self.sync_pip()
                    }
                    "pick" | "done" => {
                        if let Some(sp) = self.panorama.split.as_mut() {
                            sp.picking = op == "pick";
                            sp.error = None;
                        }
                        Task::none()
                    }
                    "release" => {
                        let guest = self.panorama.split.as_mut().and_then(|sp| {
                            sp.placed = None;
                            sp.guest.take()
                        });
                        Task::batch([restore(guest), self.sync_pip()])
                    }
                    _ => Task::none(),
                }
            }
            PMsg::SideModule(id) => {
                if let Some(sp) = self.panorama.split.as_mut() {
                    if let Some(i) = sp.modules.iter().position(|m| *m == id) {
                        sp.modules.remove(i);
                    } else if sp.modules.len() < MAX_MODULES
                        && crate::panorama::modules::CATALOGUE
                            .iter()
                            .any(|(m, _, _)| *m == id)
                    {
                        sp.modules.push(id.clone());
                    }
                }
                // Tuile Hue : relevé aussitôt.
                if id == "hue" {
                    return self.panorama_hue_refresh();
                }
                Task::none()
            }
            PMsg::SideWindow(addr) => {
                let Some(rect) = self.panorama_aside_target() else {
                    return Task::none();
                };
                let Some(sp) = self.panorama.split.as_mut() else {
                    return Task::none();
                };
                // Une seule fenêtre : la précédente est d'abord rendue.
                let old = sp.guest.take();
                sp.placed = None;
                sp.picking = false;
                sp.busy = true;
                sp.error = None;
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            if let Some(o) = old {
                                let _ = o.restore();
                            }
                            Guest::take(&addr, rect)
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| wrap(PMsg::SideTaken(r)),
                )
            }
            PMsg::SideTaken(r) => match (r, self.panorama.split.as_mut()) {
                (Ok(g), Some(sp)) => {
                    sp.busy = false;
                    sp.guest = Some(g);
                    self.sync_pip()
                }
                // Côte à côte quitté pendant le transfert : la fenêtre repart.
                (Ok(g), None) => restore(Some(g)),
                (Err(e), Some(sp)) => {
                    sp.busy = false;
                    sp.error = Some(crate::tr!("Fenêtre non ramenée : {e}", e = e));
                    Task::none()
                }
                (Err(_), None) => Task::none(),
            },
            PMsg::SideDone(r) => {
                if let Err(e) = r {
                    tracing::warn!("Panorama : côte à côte : {e}");
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    // -- Vue -------------------------------------------------------------------

    /// Bouton du bandeau : icône intégrée et libellé.
    fn side_button(
        &self,
        icon: &str,
        label: &str,
        h: f32,
        fg: Color,
        msg: Msg,
    ) -> Element<'_, Msg> {
        let g = h * 0.42;
        let glyph: Element<'_, Msg> = match crate::ui::icons::get(icon) {
            Some(handle) => svg(handle)
                .width(g)
                .height(g)
                .style(move |_, _| svg::Style { color: Some(fg) })
                .into(),
            None => Space::new().width(0).into(),
        };
        self.panorama_feedback(
            button(
                container(
                    row![
                        glyph,
                        text(crate::i18n::tr(label).to_string())
                            .size(h * 0.36)
                            .font(self.panorama_font())
                            .color(fg)
                            .wrapping(text::Wrapping::None)
                    ]
                    .spacing(h * 0.15)
                    .align_y(Alignment::Center),
                )
                .center_y(Length::Fill)
                .padding([0.0, h * 0.3]),
            )
            .height(h)
            .on_press(msg)
            .style(move |_, st| button::Style {
                background: Some(
                    fx::alpha(
                        fg,
                        match st {
                            button::Status::Pressed => 0.28,
                            button::Status::Hovered => 0.16,
                            _ => 0.08,
                        },
                    )
                    .into(),
                ),
                border: Border::default().rounded(h / 2.0),
                ..Default::default()
            }),
        )
    }

    /// Bandeau de la zone d'accueil : ajouter, rendre la fenêtre, inverser
    /// les côtés, revenir sans partage.
    pub(super) fn panorama_aside_header(
        &self,
        geo: &Geo,
        p: &Pane,
        fg: Color,
        bg: Color,
    ) -> Element<'_, Msg> {
        let t = geo.target;
        let h = t * 0.8;
        let compact = p.rect.w < t * 9.0;
        let sp = self.panorama.split.clone().unwrap_or_default();
        let lbl = |l: &'static str| if compact { "" } else { l };
        let mut r = row![].spacing(8).align_y(Alignment::Center);
        let title = match &sp.guest {
            Some(g) if !g.title.is_empty() => g.title.clone(),
            Some(g) => g.class.clone(),
            None => crate::i18n::tr("Zone d'accueil").to_string(),
        };
        r = r.push(
            container(
                text(title)
                    .size(t * 0.34)
                    .color(fg)
                    .font(self.panorama_font())
                    .wrapping(text::Wrapping::None),
            )
            .width(Length::Fill)
            .clip(true),
        );
        if sp.guest.is_some() {
            r = r.push(self.side_button(
                "window-close",
                lbl("Rendre la fenêtre"),
                h,
                fg,
                wrap(PMsg::Side("release")),
            ));
        } else if !sp.picking {
            r = r.push(self.side_button(
                "list-add",
                lbl("Ajouter"),
                h,
                fg,
                wrap(PMsg::Side("pick")),
            ));
        }
        r = r
            .push(self.side_button(
                "hyprdesk-split-swap",
                lbl("Inverser"),
                h,
                fg,
                wrap(PMsg::Side("swap")),
            ))
            .push(self.side_button(
                "hyprdesk-split-off",
                lbl("Sans partage"),
                h,
                fg,
                wrap(PMsg::Side("off")),
            ));
        container(r)
            .padding([0.0, t * 0.15])
            .width(p.rect.w)
            .height(geo.header)
            .align_y(iced::alignment::Vertical::Center)
            .style(move |_| container::Style {
                background: Some(bg.into()),
                ..Default::default()
            })
            .clip(true)
            .into()
    }

    /// Contenu de la zone d'accueil : la fenêtre (trou), le choix « + »,
    /// ou les modules ajoutés.
    pub(super) fn panorama_aside(&self, size: Size) -> Element<'_, Msg> {
        let (bg, fg, accent) = self.panorama_palette();
        let d = &self.cfg.general.panorama.design;
        let sp = self.panorama.split.clone().unwrap_or_default();
        if sp.guest.is_some() {
            return Space::new().width(Length::Fill).height(Length::Fill).into();
        }
        if sp.picking {
            return self.panorama_aside_picker(&sp);
        }
        let dim = fx::alpha(fg, 0.65);
        if sp.busy {
            return container(
                text(crate::i18n::tr("Fenêtre en route…"))
                    .size(15)
                    .color(dim),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into();
        }
        if sp.modules.is_empty() {
            // Zone vide : un grand « + ».
            let side = (size.width.min(size.height) * 0.3).clamp(64.0, 180.0);
            let plus: Element<'_, Msg> = match crate::ui::icons::get("list-add") {
                Some(h) => svg(h)
                    .width(side * 0.4)
                    .height(side * 0.4)
                    .style(move |_, _| svg::Style { color: Some(fg) })
                    .into(),
                None => text("+").size(side * 0.5).color(fg).into(),
            };
            let mut col = column![
                self.panorama_feedback(
                    button(
                        container(plus)
                            .center_x(Length::Fill)
                            .center_y(Length::Fill)
                    )
                    .width(side)
                    .height(side)
                    .on_press(wrap(PMsg::Side("pick")))
                    .style(move |_, st| button::Style {
                        background: Some(
                            fx::alpha(
                                accent,
                                if matches!(st, button::Status::Pressed) {
                                    0.5
                                } else {
                                    0.15
                                },
                            )
                            .into(),
                        ),
                        border: Border {
                            radius: (side * 0.22).into(),
                            width: 1.0,
                            color: fx::alpha(accent, 0.5),
                        },
                        ..Default::default()
                    })
                ),
                text(crate::i18n::tr("Ajoutez un module ou ramenez une fenêtre"))
                    .size(15.0 * d.text_scale)
                    .color(dim),
            ]
            .spacing(14)
            .align_x(Alignment::Center);
            if let Some(e) = &sp.error {
                col = col.push(text(e.clone()).size(13).color(dim));
            }
            return container(col)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into();
        }
        // Modules : une ou deux colonnes selon la forme de la zone.
        let gap = d.gap.min(16.0);
        let n = sp.modules.len();
        let cols = if n > 1 && size.width > size.height * 1.3 {
            2
        } else {
            1
        };
        let rows = n.div_ceil(cols);
        let inner = 16.0;
        let w = ((size.width - inner - gap * (cols - 1) as f32) / cols as f32).max(40.0);
        let h = ((size.height - inner - gap * (rows - 1) as f32) / rows as f32).max(40.0);
        let mut grid = column![].spacing(gap);
        for chunk in sp.modules.chunks(cols) {
            let mut line = row![].spacing(gap);
            for m in chunk {
                line = line.push(
                    container(self.panorama_module_body(m, Size::new(w, h)))
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
                        }),
                );
            }
            grid = grid.push(line);
        }
        container(grid).padding(8).into()
    }

    /// Choix « + » : modules à cocher, puis les fenêtres ouvertes à ramener.
    fn panorama_aside_picker(&self, sp: &SideBySide) -> Element<'_, Msg> {
        let (bg, fg, accent) = self.panorama_palette();
        let dim = fx::alpha(fg, 0.65);
        let heading = |s: &str| {
            text(crate::i18n::tr(s).to_string())
                .size(15)
                .font(iced::Font {
                    weight: iced::font::Weight::Semibold,
                    ..self.panorama_font()
                })
                .color(fg)
        };
        let chip = move |on: bool| {
            move |_: &iced::Theme, st: button::Status| button::Style {
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
                    radius: 12.0.into(),
                    width: if on { 2.0 } else { 1.0 },
                    color: fx::alpha(if on { accent } else { fg }, if on { 0.9 } else { 0.2 }),
                },
                text_color: fg,
                ..Default::default()
            }
        };
        let mut mods = iced::widget::Row::new().spacing(8);
        for (id, label, icon) in crate::panorama::modules::CATALOGUE {
            let on = sp.modules.iter().any(|m| m == id);
            let glyph: Element<'_, Msg> = match crate::ui::icons::get(icon) {
                Some(h) => svg(h)
                    .width(22)
                    .height(22)
                    .style(move |_, _| svg::Style { color: Some(fg) })
                    .into(),
                None => Space::new().width(0).into(),
            };
            mods = mods.push(
                self.panorama_feedback(
                    button(
                        column![
                            glyph,
                            text(crate::i18n::tr(label).to_string())
                                .size(12)
                                .color(fg)
                                .wrapping(text::Wrapping::None)
                        ]
                        .spacing(6)
                        .align_x(Alignment::Center),
                    )
                    .width(104)
                    .padding([10, 6])
                    .on_press(wrap(PMsg::SideModule(id.to_string())))
                    .style(chip(on)),
                ),
            );
        }
        let video = self
            .panorama
            .video
            .as_ref()
            .and_then(|v| v.address.as_deref());
        let mut wins = column![].spacing(6);
        let mut any = false;
        for c in self
            .clients
            .iter()
            .filter(|c| !c.workspace.starts_with("special:") && Some(c.address.as_str()) != video)
        {
            any = true;
            let img = self
                .class_imgs
                .get(&c.class)
                .copied()
                .unwrap_or(crate::ui::IconImg::None);
            let icon: Element<'_, Msg> = self
                .img_element(img, 26.0, 1.0)
                .unwrap_or_else(|| Space::new().width(26).height(26).into());
            let title = if c.title.is_empty() {
                c.class.clone()
            } else {
                c.title.clone()
            };
            wins = wins.push(
                self.panorama_feedback(
                    button(
                        row![
                            icon,
                            container(
                                column![
                                    text(title)
                                        .size(13)
                                        .color(fg)
                                        .wrapping(text::Wrapping::None),
                                    text(format!("{} · {}", c.class, c.monitor))
                                        .size(11)
                                        .color(dim)
                                        .wrapping(text::Wrapping::None),
                                ]
                                .spacing(1)
                            )
                            .width(Length::Fill)
                            .clip(true),
                        ]
                        .spacing(8)
                        .align_y(Alignment::Center),
                    )
                    .width(Length::Fill)
                    .padding([6, 10])
                    .on_press(wrap(PMsg::SideWindow(c.address.clone())))
                    .style(move |_, st| button::Style {
                        background: Some(
                            fx::alpha(
                                if matches!(st, button::Status::Pressed | button::Status::Hovered) {
                                    accent
                                } else {
                                    bg
                                },
                                0.5,
                            )
                            .into(),
                        ),
                        border: Border {
                            radius: 10.0.into(),
                            width: 1.0,
                            color: fx::alpha(fg, 0.15),
                        },
                        text_color: fg,
                        ..Default::default()
                    }),
                ),
            );
        }
        if !any {
            wins = wins.push(
                text(crate::i18n::tr("Aucune fenêtre ouverte"))
                    .size(13)
                    .color(dim),
            );
        }
        let done = self.panorama_feedback(
            button(
                text(crate::i18n::tr("Terminé").to_string())
                    .size(14)
                    .color(fg),
            )
            .padding([8, 18])
            .on_press(wrap(PMsg::Side("done")))
            .style(chip(true)),
        );
        let mut col = column![
            heading("Modules"),
            scrollable(mods).direction(scrollable::Direction::Horizontal(
                scrollable::Scrollbar::new()
                    .width(4)
                    .scroller_width(4)
                    .spacing(6),
            )),
            heading("Ramener une fenêtre"),
            wins,
        ]
        .spacing(10);
        if let Some(e) = &sp.error {
            col = col.push(text(e.clone()).size(13).color(dim));
        }
        col = col.push(done);
        container(scrollable(col.padding(12)).height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }
}

/// Tâche bloquante (appels à Hyprland) hors du fil de l'interface.
fn blocking(f: impl FnOnce() -> Result<(), String> + Send + 'static) -> Task<Msg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(f)
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
        },
        |r| wrap(PMsg::SideDone(r)),
    )
}

/// Rend la fenêtre invitée, s'il y en a une.
fn restore(guest: Option<Guest>) -> Task<Msg> {
    match guest {
        Some(g) => blocking(move || g.restore()),
        None => Task::none(),
    }
}
