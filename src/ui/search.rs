//! Lanceur avec recherche (MLG-7, `hyprdesk search`) : une carte du thème au
//! centre de l'écran actif, un champ de saisie et les applications installées
//! qui correspondent (recherche floue, `crate::search`). Entrée ou un clic
//! lance, les flèches choisissent, Échap ferme.
//!
//! Une requête qui commence par `:clip` cherche dans l'historique du
//! presse-papiers (MLG-13, `crate::clipboard`) : Entrée remet l'élément choisi
//! dans le presse-papiers.

use iced::widget::{button, column, container, row, text, text_input};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Shadow, Task, Vector};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::{App, Msg, color};

pub const NAMESPACE: &str = "hyprdesk-search";
const INPUT_ID: &str = "hyprdesk-search-input";
/// Résultats affichés.
pub const SHOWN: usize = 8;
const WIDTH: f32 = 620.0;
const ROW: f32 = 46.0;
const SHADOW: f32 = 18.0;
/// Icône des éléments de l'historique du presse-papiers.
const CLIP_ICON: &str = "edit-paste-symbolic";

pub struct Search {
    pub win: Id,
    pub query: String,
    /// Indices dans `App::apps` (ou dans `clips` en mode presse-papiers), du
    /// meilleur au moins bon.
    pub results: Vec<usize>,
    pub selected: usize,
    /// Historique du presse-papiers, lu à l'entrée en mode `:clip`.
    pub clips: Option<Vec<crate::clipboard::Entry>>,
}

impl Search {
    /// Mode presse-papiers (`:clip …`) : la requête dans l'historique.
    pub fn clip_query(&self) -> Option<&str> {
        crate::clipboard::query(&self.query)
    }

    fn rank_clips(&mut self) {
        let Some(q) = crate::clipboard::query(&self.query) else {
            return;
        };
        let clips = self.clips.as_deref().unwrap_or_default();
        self.results = if q.is_empty() {
            // Sans mot : les plus récents d'abord.
            (0..clips.len().min(SHOWN)).collect()
        } else {
            let names: Vec<&str> = clips.iter().map(|c| c.preview.as_str()).collect();
            crate::search::rank(q, &names, SHOWN)
        };
        self.selected = 0;
    }
}

#[derive(Debug, Clone)]
pub enum SMsg {
    Query(String),
    Launch(usize),
    Resolved(String),
    Clips(Vec<crate::clipboard::Entry>),
}

fn height() -> f32 {
    // Champ, séparation, résultats, marges.
    56.0 + 12.0 + SHOWN as f32 * ROW + 2.0 * 16.0
}

impl App {
    /// Applications proposées : installées, visibles.
    fn search_candidates(&self) -> Vec<usize> {
        self.apps
            .iter()
            .enumerate()
            .filter(|(_, a)| a.available)
            .map(|(i, _)| i)
            .collect()
    }

    fn search_rank(&self, query: &str) -> Vec<usize> {
        let cand = self.search_candidates();
        let names: Vec<&str> = cand.iter().map(|&i| self.apps[i].name.as_str()).collect();
        crate::search::rank(query, &names, SHOWN)
            .into_iter()
            .map(|k| cand[k])
            .collect()
    }

    /// Ouvre la recherche, ou la referme si elle est ouverte.
    pub(super) fn toggle_search(&mut self) -> Task<Msg> {
        if self.search.is_some() {
            return self.close_search();
        }
        let win = Id::unique();
        let results = self.search_rank("");
        let icons: Vec<Task<Msg>> = results
            .iter()
            .filter_map(|&i| self.apps[i].icon.clone())
            .collect::<Vec<_>>()
            .iter()
            .map(|n| self.want_icon(n))
            .collect();
        self.search = Some(Search {
            win,
            query: String::new(),
            results,
            selected: 0,
            clips: None,
        });
        let mut tasks = vec![Task::done(Msg::NewLayerShell {
            settings: NewLayerShellSettings {
                size: Some((
                    (WIDTH + 2.0 * SHADOW) as u32,
                    (height() + 2.0 * SHADOW) as u32,
                )),
                layer: Layer::Overlay,
                anchor: LsAnchor::empty(),
                exclusive_zone: Some(-1),
                keyboard_interactivity: KeyboardInteractivity::Exclusive,
                output_option: OutputOption::Active,
                namespace: Some(NAMESPACE.into()),
                ..Default::default()
            },
            id: win,
        })];
        tasks.extend(icons);
        tasks.push(iced::widget::operation::focus(INPUT_ID));
        Task::batch(tasks)
    }

    pub(super) fn close_search(&mut self) -> Task<Msg> {
        match self.search.take() {
            Some(s) => iced::window::close(s.win),
            None => Task::none(),
        }
    }

    /// Flèches : sélection suivante ou précédente (en boucle).
    pub(super) fn search_move(&mut self, down: bool) -> Task<Msg> {
        if let Some(s) = self.search.as_mut()
            && !s.results.is_empty()
        {
            let n = s.results.len();
            s.selected = if down {
                (s.selected + 1) % n
            } else {
                (s.selected + n - 1) % n
            };
        }
        Task::none()
    }

    pub(super) fn search_update(&mut self, m: SMsg) -> Task<Msg> {
        match m {
            SMsg::Query(q) if crate::clipboard::query(&q).is_some() => {
                let Some(s) = self.search.as_mut() else {
                    return Task::none();
                };
                s.query = q;
                if s.clips.is_some() {
                    s.rank_clips();
                    return Task::none();
                }
                // Premier passage en mode presse-papiers : lire l'historique.
                s.clips = Some(Vec::new());
                s.results.clear();
                Task::perform(
                    async {
                        tokio::task::spawn_blocking(crate::clipboard::history)
                            .await
                            .unwrap_or_default()
                    },
                    |c| Msg::Search(SMsg::Clips(c)),
                )
            }
            SMsg::Clips(c) => {
                if let Some(s) = self.search.as_mut() {
                    s.clips = Some(c);
                    s.rank_clips();
                }
                self.want_icon(CLIP_ICON)
            }
            SMsg::Launch(k)
                if self
                    .search
                    .as_ref()
                    .is_some_and(|s| s.clip_query().is_some()) =>
            {
                let entry = self.search.as_ref().and_then(|s| {
                    let i = *s.results.get(k)?;
                    s.clips.as_ref()?.get(i).cloned()
                });
                let Some(entry) = entry else {
                    return Task::none();
                };
                let close = self.close_search();
                let copy = Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || crate::clipboard::copy(&entry))
                            .await
                            .ok()
                    },
                    |r| {
                        if let Some(Err(e)) = r {
                            tracing::warn!("{}", crate::tr!("presse-papiers : {e}", e = e));
                        }
                        Msg::Noop
                    },
                );
                Task::batch([close, copy])
            }
            SMsg::Query(q) => {
                let results = self.search_rank(&q);
                let icons: Vec<String> = results
                    .iter()
                    .filter_map(|&i| self.apps[i].icon.clone())
                    .collect();
                if let Some(s) = self.search.as_mut() {
                    s.query = q;
                    s.results = results;
                    s.selected = 0;
                }
                Task::batch(icons.iter().map(|n| self.want_icon(n)).collect::<Vec<_>>())
            }
            SMsg::Launch(k) => {
                let Some(&i) = self.search.as_ref().and_then(|s| s.results.get(k)) else {
                    return Task::none();
                };
                let def = crate::config::IconDef {
                    origin: Default::default(),
                    sel: Default::default(),
                    id: self.apps[i].id.clone(),
                    label: None,
                    kind: crate::config::IconKind::App(self.apps[i].id.clone()),
                    icon: None,
                    pos: None,
                };
                let terminal = self.cfg.general.terminal.clone();
                let close = self.close_search();
                let resolve = Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::launch::resolve(&def, None, 16, &terminal).command
                        })
                        .await
                        .unwrap_or_default()
                    },
                    |cmd| Msg::Search(SMsg::Resolved(cmd)),
                );
                Task::batch([close, resolve])
            }
            SMsg::Resolved(cmd) if !cmd.is_empty() => self.run_from_desktop(cmd, false),
            SMsg::Resolved(_) => Task::none(),
        }
    }

    pub(super) fn view_search(&self) -> Element<'_, Msg> {
        let Some(s) = &self.search else {
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
        let input = text_input(
            crate::i18n::tr("Rechercher une application… (:clip : presse-papiers)"),
            &s.query,
        )
        .id(INPUT_ID)
        .on_input(|q| Msg::Search(SMsg::Query(q)))
        .on_submit(Msg::Search(SMsg::Launch(s.selected)))
        .font(self.font)
        .size(20)
        .padding(14)
        .width(Length::Fill);
        let mut list = column![].spacing(2);
        if s.clip_query().is_some() {
            return container(self.search_card(
                s,
                input,
                self.view_clips(s, fg, dim, accent),
                bg,
                fg,
            ))
            .padding(SHADOW)
            .into();
        }
        if s.results.is_empty() {
            list = list.push(
                text(crate::i18n::tr("Aucune application ne correspond."))
                    .font(self.font)
                    .size(13)
                    .color(dim),
            );
        }
        for (k, &i) in s.results.iter().enumerate() {
            let a = &self.apps[i];
            let icon: Element<'_, Msg> = a
                .icon
                .as_deref()
                .and_then(|n| self.icon_named(n, 28.0, fg))
                .unwrap_or_else(|| iced::widget::Space::new().width(28).height(28).into());
            let selected = k == s.selected;
            list = list.push(
                button(
                    row![
                        icon,
                        text(a.name.clone()).font(self.font).size(15).color(fg)
                    ]
                    .spacing(14)
                    .align_y(Alignment::Center),
                )
                .padding([8, 12])
                .width(Length::Fill)
                .height(ROW)
                .style(move |_, status| button::Style {
                    background: (selected || matches!(status, button::Status::Hovered)).then(
                        || {
                            Color {
                                a: if selected { 0.22 } else { 0.1 },
                                ..accent
                            }
                            .into()
                        },
                    ),
                    border: Border::default().rounded(10),
                    text_color: fg,
                    ..Default::default()
                })
                .on_press(Msg::Search(SMsg::Launch(k))),
            );
        }
        container(self.search_card(s, input, list.into(), bg, fg))
            .padding(SHADOW)
            .into()
    }

    /// Résultats du mode presse-papiers : aperçus sur une ligne.
    fn view_clips<'a>(
        &'a self,
        s: &'a Search,
        fg: Color,
        dim: Color,
        accent: Color,
    ) -> Element<'a, Msg> {
        let mut list = column![].spacing(2);
        use crate::i18n::tr;
        let missing = || tr("cliphist n'est pas installé (Réglages → Bureau → Dépendances).");
        let empty = match s.clips.as_deref() {
            None | Some([]) if !crate::clipboard::available() => Some(missing()),
            None => Some(tr("Lecture de l'historique…")),
            Some([]) => Some(tr("L'historique du presse-papiers est vide.")),
            Some(_) if s.results.is_empty() => Some(tr("Aucun élément ne correspond.")),
            Some(_) => None,
        };
        if let Some(msg) = empty {
            list = list.push(text(msg).font(self.font).size(13).color(dim));
        }
        let clips = s.clips.as_deref().unwrap_or_default();
        for (k, &i) in s.results.iter().enumerate() {
            let Some(c) = clips.get(i) else { continue };
            let preview: String = c.preview.chars().take(90).collect();
            let icon: Element<'_, Msg> = self
                .icon_named(CLIP_ICON, 22.0, dim)
                .unwrap_or_else(|| iced::widget::Space::new().width(22).height(22).into());
            let selected = k == s.selected;
            list = list.push(
                button(
                    row![icon, text(preview).font(self.font).size(14).color(fg)]
                        .spacing(14)
                        .align_y(Alignment::Center),
                )
                .padding([8, 12])
                .width(Length::Fill)
                .height(ROW)
                .style(move |_, status| button::Style {
                    background: (selected || matches!(status, button::Status::Hovered)).then(
                        || {
                            Color {
                                a: if selected { 0.22 } else { 0.1 },
                                ..accent
                            }
                            .into()
                        },
                    ),
                    border: Border::default().rounded(10),
                    text_color: fg,
                    ..Default::default()
                })
                .on_press(Msg::Search(SMsg::Launch(k))),
            );
        }
        list.into()
    }

    fn search_card<'a>(
        &'a self,
        _s: &'a Search,
        input: iced::widget::TextInput<'a, Msg>,
        list: Element<'a, Msg>,
        bg: Color,
        fg: Color,
    ) -> Element<'a, Msg> {
        container(column![input, list].spacing(12))
            .padding(16)
            .width(WIDTH)
            .height(height())
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
            })
            .into()
    }
}
