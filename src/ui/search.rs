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
pub(super) const INPUT_ID: &str = "hyprdesk-search-input";
/// Résultats affichés.
pub const SHOWN: usize = 8;
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
    /// Grille plein écran des applications (`hyprdesk apps`, bouton du dock).
    pub grid: bool,
    /// Catégorie choisie dans la grille (indice dans `apps_grid::CATEGORIES`).
    pub category: Option<usize>,
    /// Applications mises en avant (les plus lancées), indices dans `App::apps`.
    pub suggested: Vec<usize>,
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
    /// Lance une application par son indice dans `App::apps` (suggestions).
    LaunchApp(usize),
    /// Grille : catégorie choisie (`None` : toutes).
    Category(Option<usize>),
    Launch(usize),
    Resolved(String),
    Clips(Vec<crate::clipboard::Entry>),
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
        self.search_rank_in(query, None, SHOWN)
    }

    /// Classement, limité à une catégorie de la grille, au plus `limit`.
    pub(super) fn search_rank_in(
        &self,
        query: &str,
        category: Option<usize>,
        limit: usize,
    ) -> Vec<usize> {
        let cand: Vec<usize> = self
            .search_candidates()
            .into_iter()
            .filter(|&i| {
                category.is_none_or(|c| super::apps_grid::in_category(&self.apps[i].categories, c))
            })
            .collect();
        let names: Vec<&str> = cand.iter().map(|&i| self.apps[i].name.as_str()).collect();
        // Les applications souvent lancées d'ici passent devant.
        let usage = crate::search::load_usage();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let bonus: Vec<i32> = cand
            .iter()
            .map(|&i| {
                usage
                    .get(&self.apps[i].id)
                    .map(|&(n, t)| crate::search::usage_bonus(n, t, now))
                    .unwrap_or(0)
            })
            .collect();
        crate::search::rank_with(query, &names, &bonus, limit)
            .into_iter()
            .map(|k| cand[k])
            .collect()
    }

    /// Le lanceur s'ouvre en grande feuille (sinon en carte centrée) : selon
    /// le réglage, ou selon le mode — feuille en plein bureau.
    pub(super) fn launcher_sheet(&self) -> bool {
        use crate::config::AppsStyle;
        match self.cfg.general.apps_launcher.style {
            AppsStyle::Sheet => true,
            AppsStyle::Spotlight => false,
            AppsStyle::Auto => self.mode() == super::DesktopMode::FullDesktop,
        }
    }

    /// Applications mises en avant : les plus lancées d'abord, complétées
    /// par celles épinglées au dock (au premier usage, rien n'est encore noté).
    pub(super) fn launcher_suggestions(&self) -> Vec<usize> {
        let n = self.cfg.general.apps_launcher.suggestions as usize;
        if n == 0 {
            return Vec::new();
        }
        let usage = crate::search::load_usage();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let mut scored: Vec<(i32, usize)> = self
            .search_candidates()
            .into_iter()
            .filter_map(|i| {
                let (c, t) = usage.get(&self.apps[i].id)?;
                Some((crate::search::usage_bonus(*c, *t, now), i))
            })
            .filter(|(b, _)| *b > 0)
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        let mut out: Vec<usize> = scored.into_iter().map(|(_, i)| i).take(n).collect();
        for id in &self.cfg.general.launcher.pinned {
            if out.len() >= n {
                break;
            }
            if let Some(i) = self.apps.iter().position(|a| {
                a.available && (&a.id == id || a.id.trim_end_matches(".desktop") == id)
            }) && !out.contains(&i)
            {
                out.push(i);
            }
        }
        out
    }

    /// `hyprdesk apps` : le lanceur (ouvre ou referme).
    pub(super) fn toggle_apps(&mut self) -> Task<Msg> {
        self.toggle_launcher()
    }

    /// `hyprdesk search` : le même lanceur (un seul, quelle que soit la commande).
    pub(super) fn toggle_search(&mut self) -> Task<Msg> {
        self.toggle_launcher()
    }

    fn toggle_launcher(&mut self) -> Task<Msg> {
        if self.search.is_some() {
            return self.close_search();
        }
        let win = Id::unique();
        let sheet = self.launcher_sheet();
        let results = if sheet {
            self.search_rank_in("", None, super::apps_grid::LIMIT)
        } else {
            self.search_rank("")
        };
        let suggested = self.launcher_suggestions();
        let icons: Vec<Task<Msg>> = results
            .iter()
            .chain(suggested.iter())
            .filter_map(|&i| self.apps[i].icon.clone())
            .collect::<Vec<_>>()
            .iter()
            .map(|n| self.want_icon(n))
            .collect();
        let size = if sheet {
            (0, 0)
        } else {
            (
                (super::apps_grid::SPOT_WIDTH + 2.0 * SHADOW) as u32,
                (super::apps_grid::spot_height(!suggested.is_empty()) + 2.0 * SHADOW) as u32,
            )
        };
        self.search = Some(Search {
            win,
            query: String::new(),
            results,
            selected: 0,
            clips: None,
            grid: sheet,
            category: None,
            suggested,
        });
        let mut tasks = vec![Task::done(Msg::NewLayerShell {
            settings: NewLayerShellSettings {
                size: Some(size),
                layer: Layer::Overlay,
                anchor: if sheet {
                    LsAnchor::Top | LsAnchor::Bottom | LsAnchor::Left | LsAnchor::Right
                } else {
                    LsAnchor::empty()
                },
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

    /// Flèches : sélection suivante ou précédente (en boucle) ; dans la
    /// grille, haut et bas sautent d'une rangée.
    pub(super) fn search_move(&mut self, down: bool) -> Task<Msg> {
        let step = self
            .search
            .as_ref()
            .filter(|s| s.grid)
            .map(|_| super::apps_grid::COLUMNS as i64)
            .unwrap_or(1);
        self.search_step(if down { step } else { -step })
    }

    /// Déplace la sélection de `d` (en boucle).
    pub(super) fn search_step(&mut self, d: i64) -> Task<Msg> {
        if let Some(s) = self.search.as_mut()
            && !s.results.is_empty()
        {
            let n = s.results.len() as i64;
            s.selected = (s.selected as i64 + d).rem_euclid(n) as usize;
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
            SMsg::Category(c) => {
                let q = self
                    .search
                    .as_ref()
                    .map(|s| s.query.clone())
                    .unwrap_or_default();
                let results = self.search_rank_in(&q, c, super::apps_grid::LIMIT);
                let icons: Vec<String> = results
                    .iter()
                    .filter_map(|&i| self.apps[i].icon.clone())
                    .collect();
                if let Some(s) = self.search.as_mut() {
                    s.category = c;
                    s.results = results;
                    s.selected = 0;
                }
                Task::batch(icons.iter().map(|n| self.want_icon(n)).collect::<Vec<_>>())
            }
            SMsg::Query(q) => {
                let (grid, cat) = self
                    .search
                    .as_ref()
                    .map(|s| (s.grid, s.category))
                    .unwrap_or_default();
                let results = if grid {
                    self.search_rank_in(&q, cat, super::apps_grid::LIMIT)
                } else {
                    self.search_rank(&q)
                };
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
                self.search_update(SMsg::LaunchApp(i))
            }
            SMsg::LaunchApp(i) => {
                if i >= self.apps.len() {
                    return Task::none();
                }
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
                let id = self.apps[i].id.clone();
                std::thread::spawn(move || crate::search::record_launch(&id));
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
        if s.clip_query().is_none() {
            return if s.grid {
                self.view_apps_grid(s)
            } else {
                self.view_spotlight(s)
            };
        }
        // Mode presse-papiers (`:clip`) : la même carte, couleurs lisibles.
        let th = &self.cfg.general.theme;
        let r = crate::theme::readable(th.fg, th.bg, th.accent);
        let (fg, dim, accent) = (color(r.fg, 1.0), color(r.dim, 1.0), color(r.focus, 1.0));
        let bg = color(th.bg, 1.0);
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
        container(self.search_card(s, input, self.view_clips(s, fg, dim, accent), bg, fg))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
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
            .width(super::apps_grid::SPOT_WIDTH)
            .height(super::apps_grid::spot_height(!_s.suggested.is_empty()))
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
