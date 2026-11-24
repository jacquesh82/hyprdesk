//! Lanceur avec recherche (MLG-7, `hyprdesk search`) : une carte du thème au
//! centre de l'écran actif, un champ de saisie et les applications installées
//! qui correspondent (recherche floue, `crate::search`). Entrée ou un clic
//! lance, les flèches choisissent, Échap ferme.
//!
//! Une requête qui commence par `:clip` cherche dans l'historique du
//! presse-papiers (MLG-13, `crate::clipboard`) : Entrée remet l'élément choisi
//! dans le presse-papiers. `:2fa` et `:mdp` cherchent dans le coffre myMFA
//! (MLG-44, `crate::mymfa`) : Entrée copie le code ou le mot de passe (par
//! le démon mymfa) ; coffre verrouillé, Entrée ouvre la carte de
//! déverrouillage.
//!
//! `:f` (alias `:fichiers`) cherche dans l'index de fichiers
//! (`crate::fileindex`) : chaque frappe interroge le démon d'indexation
//! (socket unix, démarré au besoin), Entrée ouvre le fichier choisi avec
//! l'application par défaut.

use std::path::Path;

use iced::widget::{button, column, container, row, text};
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
/// Icône des codes et mots de passe du coffre.
const MFA_ICON: &str = "system-lock-screen";

/// État du mode fichiers (`:f`) : la dernière réponse du démon d'index.
#[derive(Debug, Clone, Default)]
pub struct FilesState {
    /// Résultats du démon, dans son ordre (le classement fin se fait ici).
    pub hits: Vec<crate::fileindex::index::Hit>,
    /// Scan initial en cours : entrées déjà vues (aucun résultat servi).
    pub scanning: Option<u64>,
    /// Snapshot en cours de chargement par le démon (aucun résultat servi).
    pub loading: bool,
    /// Démon indisponible ou réponse d'erreur.
    pub erreur: Option<String>,
    /// Nature demandée ([`KIND_FILE`], [`KIND_DIR`], [`KIND_LINK`]) ;
    /// `None` : toutes.
    pub kind: Option<u8>,
    /// Catégorie demandée (code [`categorie_de`]) ; `None` : toutes.
    pub cat: Option<u8>,
}

/// Aperçu du résultat de fichier sélectionné (HIG DAT-03).
#[derive(Debug, Clone)]
pub enum Apercu {
    /// Demandé, pas encore lu : la date seule.
    Attente,
    /// Vignette d'une image.
    Image(iced::widget::image::Handle),
    /// Premières lignes non vides d'un texte.
    Texte(Vec<String>),
    /// Ni image ni texte lisible : la date seule.
    Rien,
}

/// Taille au-delà de laquelle un fichier n'est pas lu pour son aperçu.
const APERCU_MAX_OCTETS: u64 = 32 * 1024 * 1024;
/// Pixels au-delà desquels une image n'est pas décodée.
const APERCU_MAX_PIXELS: u64 = 40_000_000;
/// Rangée d'aperçu sous la ligne sélectionnée.
const APERCU_H: f32 = 40.0;

/// Les deux premières lignes non vides d'un début de fichier, s'il est du
/// texte : UTF-8 sans octet nul, une coupure en fin de tampon tolérée ;
/// 80 caractères au plus par ligne.
pub fn texte_apercu(buf: &[u8]) -> Option<Vec<String>> {
    if buf.contains(&0) {
        return None;
    }
    let s = match std::str::from_utf8(buf) {
        Ok(s) => s,
        Err(e) if e.error_len().is_none() => std::str::from_utf8(&buf[..e.valid_up_to()]).ok()?,
        Err(_) => return None,
    };
    let lignes: Vec<String> = s
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.trim().is_empty())
        .take(2)
        .map(|l| {
            let l = l.replace('\t', "    ");
            if l.chars().count() > 80 {
                let mut c: String = l.chars().take(79).collect();
                c.push('…');
                c
            } else {
                l
            }
        })
        .collect();
    (!lignes.is_empty()).then_some(lignes)
}

/// Lit l'aperçu d'un chemin (hors du fil de l'interface).
fn charger_apercu(path: &Path) -> Apercu {
    use crate::fileindex::index::{CAT_IMAGES, categorie_de};
    use std::io::Read;
    let Ok(meta) = std::fs::metadata(path) else {
        return Apercu::Rien;
    };
    // Un fichier ordinaire seulement : une FIFO bloquerait la lecture.
    if !meta.is_file() || meta.len() > APERCU_MAX_OCTETS {
        return Apercu::Rien;
    }
    let nom = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    if categorie_de(nom) == CAT_IMAGES {
        return match ::image::image_dimensions(path) {
            Ok((w, h)) if u64::from(w) * u64::from(h) <= APERCU_MAX_PIXELS => {
                super::wizard::scene_thumb(path).map_or(Apercu::Rien, Apercu::Image)
            }
            _ => Apercu::Rien,
        };
    }
    let mut buf = Vec::with_capacity(4096);
    match std::fs::File::open(path).and_then(|f| f.take(4096).read_to_end(&mut buf)) {
        Ok(_) => texte_apercu(&buf).map_or(Apercu::Rien, Apercu::Texte),
        Err(_) => Apercu::Rien,
    }
}

/// Contenu du coffre en mode `:2fa` / `:mdp`.
#[derive(Debug, Clone)]
pub enum MfaList {
    Loading,
    Absent,
    NoVault,
    Locked,
    Error(String),
    Codes(Vec<crate::mymfa::Code>),
    Logins(Vec<crate::mymfa::Login>),
}

pub struct Search {
    pub(super) visibility: Option<super::Fade>,
    /// Ouverture hors mode convexe (MLG-278) : effet réglé.
    pub(super) opening: Option<super::Fade>,
    pub(super) shown: bool,
    pub win: Id,
    pub query: String,
    /// Indices dans `App::apps` (ou dans `clips` en mode presse-papiers), du
    /// meilleur au moins bon.
    pub results: Vec<usize>,
    pub selected: usize,
    /// Historique du presse-papiers, lu à l'entrée en mode `:clip`.
    pub clips: Option<Vec<crate::clipboard::Entry>>,
    /// Mode fichiers (`:f`) : dernière réponse du démon d'index.
    pub files: Option<FilesState>,
    /// Grille plein écran des applications (`hyprdesk apps`, bouton du dock).
    pub grid: bool,
    /// Catégorie choisie dans la grille (indice dans `apps_grid::CATEGORIES`).
    pub category: Option<usize>,
    /// Applications mises en avant (les plus lancées), indices dans `App::apps`.
    pub suggested: Vec<usize>,
    /// Mode coffre (`:2fa`, `:mdp`) : le mode chargé et son contenu.
    pub mfa: Option<(crate::mymfa::Mode, MfaList)>,
    /// Mode réglages (`:r`) : les entrées trouvées dans l'index des réglages,
    /// les plus parlantes d'abord ; `results` y sont des indices.
    pub settings: Vec<&'static super::settings_search::Entry>,
    /// Mode fichiers : aperçu du résultat sélectionné, avec son chemin.
    pub apercu: Option<(String, Apercu)>,
}

impl Search {
    /// Chemin du résultat de fichier sélectionné (mode `:f`).
    pub fn fichier_choisi(&self) -> Option<&str> {
        crate::fileindex::proto::query(&self.query)?;
        let i = *self.results.get(self.selected)?;
        self.files.as_ref()?.hits.get(i).map(|h| h.path.as_str())
    }

    /// Garde un aperçu arrivé s'il est celui de la sélection ; sinon jeté.
    pub fn garde_apercu(&mut self, path: String, a: Apercu) -> bool {
        if self.fichier_choisi() != Some(path.as_str()) {
            return false;
        }
        self.apercu = Some((path, a));
        true
    }

    /// Mode presse-papiers (`:clip …`) : la requête dans l'historique.
    pub fn clip_query(&self) -> Option<&str> {
        crate::clipboard::query(&self.query)
    }

    /// Mode réglages (`:r …`) : la requête dans l'index des réglages.
    pub fn setting_query(&self) -> Option<&str> {
        super::settings_search::query(&self.query)
    }

    /// Vue de la carte (applications, presse-papiers, fichiers, mots de
    /// passe, réglages) : en changer remplace le champ de saisie.
    fn view_kind(&self) -> u8 {
        if crate::mymfa::query(&self.query).is_some() {
            2
        } else if self.clip_query().is_some() {
            1
        } else if crate::fileindex::proto::query(&self.query).is_some() {
            3
        } else if self.setting_query().is_some() {
            4
        } else {
            0
        }
    }

    /// Mode fichiers (`:f`) : classement fin des résultats du démon par la
    /// recherche floue commune (les hits ont déjà été filtrés par le moteur
    /// trigramme).
    fn rank_files(&mut self) {
        let Some(q) = crate::fileindex::proto::query(&self.query) else {
            return;
        };
        let hits = self
            .files
            .as_ref()
            .map(|f| f.hits.as_slice())
            .unwrap_or(&[]);
        self.results = if q.is_empty() {
            (0..hits.len().min(SHOWN)).collect()
        } else {
            let noms: Vec<&str> = hits.iter().map(|h| h.name.as_str()).collect();
            crate::search::rank(q, &noms, SHOWN)
        };
        self.selected = 0;
    }

    fn rank_mfa(&mut self) {
        let Some((_, q)) = crate::mymfa::query(&self.query) else {
            return;
        };
        let labels: Vec<String> = match &self.mfa {
            Some((_, MfaList::Codes(c))) => c.iter().map(|c| c.label()).collect(),
            Some((_, MfaList::Logins(l))) => l.iter().map(|l| l.label()).collect(),
            _ => Vec::new(),
        };
        self.results = if q.is_empty() {
            (0..labels.len().min(SHOWN)).collect()
        } else {
            let names: Vec<&str> = labels.iter().map(String::as_str).collect();
            crate::search::rank(q, &names, SHOWN)
        };
        self.selected = 0;
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
    Close,
    Query(String),
    /// Lance une application par son indice dans `App::apps` (suggestions).
    LaunchApp(usize),
    /// Grille : catégorie choisie (`None` : toutes).
    Category(Option<usize>),
    Launch(usize),
    Resolved(String),
    Clips(Vec<crate::clipboard::Entry>),
    Mfa(crate::mymfa::Mode, MfaList),
    /// Réponse du démon d'index pour la requête `:f` donnée (le mot, sans
    /// le préfixe — une réponse en retard pour un autre mot est jetée).
    Files(String, FilesState),
    /// Redemande au démon pendant un chargement ou un scan, si le mot n'a
    /// pas changé entre-temps.
    FilesRetry(String),
    /// Nature demandée dans l'UI fichiers (None : toutes) — relance la
    /// requête avec le filtre.
    FileFilter(Option<u8>),
    /// Catégorie demandée dans l'UI fichiers (None : toutes).
    CatFilter(Option<u8>),
    /// Carte affichée : le champ prend le focus (la surface existe enfin).
    Shown,
    /// Bouton Clear du champ : la requête est vidée et le champ reprend le focus.
    Clear,
    /// Aperçu voulu pour ce chemin, après un court délai (une flèche tenue
    /// ne lit pas un fichier par pas) : lu s'il est encore la sélection.
    ApercuVoulu(String),
    /// Aperçu lu pour ce chemin.
    Apercu(String, Apercu),
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
        if self.convex_options().is_some_and(|o| o.attached_panels) {
            return false;
        }
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
        // Avant la saisie (MLG-278) : champ seul, ou les épinglées.
        match self.cfg.general.apps_launcher.shape.empty_view {
            crate::config::EmptyView::Field => return Vec::new(),
            crate::config::EmptyView::Favorites => {
                return self
                    .cfg
                    .general
                    .launcher
                    .pinned
                    .iter()
                    .filter_map(|id| self.apps.iter().position(|a| &a.id == id))
                    .take(n.max(8))
                    .collect();
            }
            crate::config::EmptyView::Suggestions => {}
        }
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
    /// Ouvre le lanceur sur une requête (`:2fa `, `:mdp `).
    pub(super) fn open_search_with(&mut self, query: &str) -> Task<Msg> {
        let open = if self.search.is_some() {
            Task::none()
        } else {
            self.toggle_search()
        };
        Task::batch([
            open,
            Task::done(Msg::Search(SMsg::Query(query.to_string()))),
        ])
    }

    pub(super) fn toggle_search(&mut self) -> Task<Msg> {
        self.toggle_launcher()
    }

    fn toggle_launcher(&mut self) -> Task<Msg> {
        let duration = self.bar_duration(self.convex_options().map_or(0, |o| o.duration as u64));
        if let Some(s) = self.search.as_mut()
            && let Some(f) = s.visibility.as_mut()
            && f.to == 0.0
        {
            *f = super::Fade {
                from: f.value(std::time::Instant::now()),
                to: 1.0,
                start: std::time::Instant::now(),
                dur: duration,
            };
            return Task::none();
        }
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
        let convex = self.convex_options().filter(|o| o.attached_panels);
        // Toujours plein écran : le voile flouté couvre le bureau.
        let size = (0, 0);
        self.search = Some(Search {
            shown: cfg!(test),
            visibility: convex.map(|o| super::Fade {
                from: 0.0,
                to: 1.0,
                start: std::time::Instant::now(),
                dur: self.bar_duration(o.duration as u64),
            }),
            opening: {
                let shape = self.cfg.general.apps_launcher.shape;
                (convex.is_none() && shape.open_effect != crate::config::OpenEffect::None).then(
                    || super::Fade {
                        from: 0.0,
                        to: 1.0,
                        start: std::time::Instant::now(),
                        dur: if shape.open_duration > 0 {
                            super::tokens::duration(u64::from(shape.open_duration))
                        } else {
                            self.bar_duration(220)
                        },
                    },
                )
            },
            win,
            query: String::new(),
            results,
            selected: 0,
            clips: None,
            files: None,
            grid: sheet,
            category: None,
            suggested,
            mfa: None,
            settings: Vec::new(),
            apercu: None,
        });
        let mut tasks = vec![Task::done(Msg::NewLayerShell {
            settings: NewLayerShellSettings {
                size: Some(size),
                layer: Layer::Overlay,
                anchor: LsAnchor::Top | LsAnchor::Bottom | LsAnchor::Left | LsAnchor::Right,
                exclusive_zone: Some(-1),
                keyboard_interactivity: KeyboardInteractivity::Exclusive,
                // Sur l'écran du clic (bouton du dock), sinon l'écran actif.
                output_option: self
                    .click_monitor
                    .clone()
                    .map_or(OutputOption::Active, OutputOption::OutputName),
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
        let duration = self.bar_duration(self.convex_options().map_or(0, |o| o.duration as u64));
        if !duration.is_zero()
            && let Some(s) = self.search.as_mut()
            && let Some(f) = s.visibility.as_mut()
        {
            if f.to != 0.0 {
                *f = super::Fade {
                    from: f.value(std::time::Instant::now()),
                    to: 0.0,
                    start: std::time::Instant::now(),
                    dur: duration,
                };
            }
            return Task::none();
        }
        match self.search.take() {
            Some(s) => iced::window::close(s.win),
            None => Task::none(),
        }
    }

    pub(super) fn search_frame(&mut self) -> Task<Msg> {
        let now = std::time::Instant::now();
        if self
            .search
            .as_ref()
            .and_then(|s| s.visibility)
            .is_some_and(|f| f.done(now) && f.to == 0.0)
        {
            return self
                .search
                .take()
                .map_or_else(Task::none, |s| iced::window::close(s.win));
        }
        if let Some(s) = self.search.as_mut()
            && s.shown
            && let Some(f) = s.visibility.as_mut()
            && f.done(now)
        {
            f.from = f.to;
            f.dur = std::time::Duration::ZERO;
        }
        Task::none()
    }

    pub(super) fn convex_search_surface<'a>(
        &'a self,
        s: &'a Search,
        body: Element<'a, Msg>,
        width: f32,
        height: f32,
    ) -> Element<'a, Msg> {
        use super::{
            kinetic::{self, Track},
            plugins::bar::style::convexe::Edge,
        };
        let track = if !s.shown {
            super::Fade {
                from: 0.0,
                to: 0.0,
                start: std::time::Instant::now(),
                dur: std::time::Duration::ZERO,
            }
        } else {
            s.visibility.unwrap_or(super::Fade {
                from: 1.0,
                to: 1.0,
                start: std::time::Instant::now(),
                dur: std::time::Duration::ZERO,
            })
        };
        let card = container(body)
            .padding(iced::Padding {
                top: 24.0,
                bottom: 12.0,
                left: 40.0,
                right: 40.0,
            })
            .width(Length::Fill)
            .height(Length::Fill);
        let card: Element<'_, Msg> = if s.visibility.is_some_and(|f| f.to == 0.0) {
            crate::ui::plugins::bar::motion::inert(card)
        } else {
            card.into()
        };
        let pal = self.bar_palette();
        let drawer = container(kinetic::reveal(
            card,
            Track::Ease(track),
            Edge::Bottom,
            color(pal.bg, pal.opacity),
            s.shown.then_some(Msg::MotionSettled),
        ))
        .width(width)
        .max_width(width)
        .height(height)
        .clip(true);
        // Le voile s'éclaircit avec le tiroir : le flou part avec lui.
        let catch = backdrop(
            super::apps_grid::shape().backdrop_dim * track.value(std::time::Instant::now()),
        );
        iced::widget::stack![
            catch,
            container(drawer)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_x(Alignment::Center)
                .align_y(Alignment::End)
        ]
        .into()
    }

    /// Flèches : sélection suivante ou précédente (en boucle) ; dans la
    /// grille, haut et bas sautent d'une rangée.
    pub(super) fn search_move(&mut self, down: bool) -> Task<Msg> {
        let step = self
            .search
            .as_ref()
            .filter(|s| s.grid)
            .map(|_| super::apps_grid::columns() as i64)
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
        self.file_preview()
    }

    /// Mode fichiers : demande l'aperçu de la sélection s'il n'est ni là ni
    /// en route (HIG DAT-03).
    fn file_preview(&mut self) -> Task<Msg> {
        let Some(s) = self.search.as_mut() else {
            return Task::none();
        };
        let Some(path) = s.fichier_choisi().map(str::to_string) else {
            return Task::none();
        };
        if s.apercu.as_ref().is_some_and(|(p, _)| *p == path) {
            return Task::none();
        }
        s.apercu = Some((path.clone(), Apercu::Attente));
        if self.preview {
            return Task::none();
        }
        Task::perform(
            tokio::time::sleep(std::time::Duration::from_millis(120)),
            move |_| Msg::Search(SMsg::ApercuVoulu(path.clone())),
        )
    }

    /// Interroge le démon d'index pour la requête `:f` en cours (après
    /// l'avoir garanti : démarré s'il manque, avec les réglages
    /// `index.*` de la configuration) ; la réponse revient en
    /// `SMsg::Files`, hors du thread UI.
    fn fichiers_requete(&mut self) -> Task<Msg> {
        let mot = self
            .search
            .as_ref()
            .and_then(|s| crate::fileindex::proto::query(&s.query))
            .unwrap_or_default()
            .to_string();
        let (kind, cat) = self
            .search
            .as_ref()
            .and_then(|s| s.files.as_ref())
            .map(|f| (f.kind, f.cat))
            .unwrap_or((None, None));
        let reglage = crate::fileindex::IndexConfig::depuis_general(&self.cfg.general);
        let mot_reponse = mot.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    crate::fileindex::proto::ensure_daemon(&reglage).and_then(|_| {
                        crate::fileindex::proto::request(&crate::fileindex::proto::Request::Query {
                            q: mot,
                            limit: crate::fileindex::proto::LIMIT_DEFAULT,
                            kinds: kind.map(|k| vec![k]),
                            cat,
                        })
                    })
                })
                .await
                .unwrap_or_else(|e| Err(format!("{e}")))
            },
            move |rep| {
                let st = match rep {
                    Ok(crate::fileindex::proto::Reply::Hits {
                        hits,
                        scanning,
                        loading,
                        ..
                    }) => FilesState {
                        hits,
                        scanning,
                        loading,
                        erreur: None,
                        kind,
                        cat,
                    },
                    Ok(crate::fileindex::proto::Reply::Err(m)) => FilesState {
                        hits: Vec::new(),
                        scanning: None,
                        loading: false,
                        erreur: Some(m),
                        kind,
                        cat,
                    },
                    Err(e) => FilesState {
                        hits: Vec::new(),
                        scanning: None,
                        loading: false,
                        erreur: Some(e),
                        kind,
                        cat,
                    },
                    Ok(_) => FilesState {
                        hits: Vec::new(),
                        kind,
                        ..Default::default()
                    },
                };
                Msg::Search(SMsg::Files(mot_reponse, st))
            },
        )
    }

    pub(super) fn search_update(&mut self, m: SMsg) -> Task<Msg> {
        match m {
            SMsg::Close => self.close_search(),
            // Fenêtre myMFA complète (MLG-46) : le lanceur se referme.
            SMsg::Query(q) if q.trim() == ":mymfa" => {
                let fermer = self.toggle_search();
                Task::batch([fermer, self.mfa_fenetre_open()])
            }
            SMsg::Query(q) if crate::mymfa::query(&q).is_some() => {
                let Some((mode, _)) = crate::mymfa::query(&q) else {
                    return Task::none();
                };
                let st = self.mfa.clone();
                let Some(s) = self.search.as_mut() else {
                    return Task::none();
                };
                s.query = q;
                if s.mfa.as_ref().is_some_and(|(m, _)| *m == mode) {
                    s.rank_mfa();
                    return Task::none();
                }
                // Premier passage dans ce mode : contenu lu au démon.
                s.results.clear();
                let list = if !st.installed {
                    MfaList::Absent
                } else if !st.vault {
                    MfaList::NoVault
                } else if st.locked {
                    MfaList::Locked
                } else {
                    MfaList::Loading
                };
                let load = matches!(list, MfaList::Loading);
                s.mfa = Some((mode, list));
                let icon = self.want_icon(MFA_ICON);
                if !load {
                    return icon;
                }
                let fetch = Task::perform(
                    async move {
                        match mode {
                            crate::mymfa::Mode::Codes => match crate::mymfa::codes().await {
                                Ok(c) => MfaList::Codes(c),
                                Err(e) => MfaList::Error(e),
                            },
                            crate::mymfa::Mode::Logins => {
                                match crate::mymfa::logins(String::new()).await {
                                    Ok(l) => MfaList::Logins(l),
                                    Err(e) => MfaList::Error(e),
                                }
                            }
                        }
                    },
                    move |l| Msg::Search(SMsg::Mfa(mode, l)),
                );
                Task::batch([icon, fetch])
            }
            SMsg::Mfa(mode, list) => {
                if let Some(s) = self.search.as_mut()
                    && s.mfa.as_ref().is_some_and(|(m, _)| *m == mode)
                {
                    s.mfa = Some((mode, list));
                    s.rank_mfa();
                }
                Task::none()
            }
            SMsg::Launch(k)
                if self
                    .search
                    .as_ref()
                    .is_some_and(|s| crate::mymfa::query(&s.query).is_some()) =>
            {
                enum Pick {
                    Unlock,
                    Code(String),
                    Login(String),
                    Nothing,
                }
                let pick = self
                    .search
                    .as_ref()
                    .map(|s| match &s.mfa {
                        Some((_, MfaList::Locked | MfaList::NoVault)) => Pick::Unlock,
                        Some((_, MfaList::Codes(c))) => s
                            .results
                            .get(k)
                            .and_then(|&i| c.get(i))
                            .map(|c| Pick::Code(c.id.clone()))
                            .unwrap_or(Pick::Nothing),
                        Some((_, MfaList::Logins(l))) => s
                            .results
                            .get(k)
                            .and_then(|&i| l.get(i))
                            .map(|l| Pick::Login(l.id.clone()))
                            .unwrap_or(Pick::Nothing),
                        _ => Pick::Nothing,
                    })
                    .unwrap_or(Pick::Nothing);
                let report = |r: Result<(), String>| {
                    if let Err(e) = r {
                        tracing::warn!("{}", crate::tr!("myMFA : {e}", e = e));
                        // L'échec est montré, pas seulement journalisé (HIG
                        // PAT-11).
                        crate::hypr::notify(
                            0,
                            8000,
                            &crate::tr!("Copie depuis myMFA impossible : {e}", e = e),
                        );
                    }
                    Msg::Noop
                };
                match pick {
                    Pick::Nothing => Task::none(),
                    Pick::Unlock => {
                        let close = self.close_search();
                        Task::batch([close, self.mfa_update(super::mymfa::MMsg::Ask)])
                    }
                    Pick::Code(id) => Task::batch([
                        self.close_search(),
                        Task::perform(crate::mymfa::copy_code(id), report),
                    ]),
                    Pick::Login(id) => Task::batch([
                        self.close_search(),
                        Task::perform(crate::mymfa::copy_login(id, "password"), report),
                    ]),
                }
            }
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
            SMsg::Query(q) if crate::fileindex::proto::query(&q).is_some() => {
                let avant = self
                    .search
                    .as_ref()
                    .map(|s| s.view_kind())
                    .unwrap_or_default();
                let Some(s) = self.search.as_mut() else {
                    return Task::none();
                };
                s.query = q;
                if s.files.is_none() {
                    s.files = Some(FilesState::default());
                }
                s.results.clear();
                s.selected = 0;
                let refocus = s.view_kind() != avant;
                // Chaque frappe interroge le démon (le moteur trigramme
                // répond en quelques millisecondes) ; le démon est garanti
                // au passage (démarré s'il manque).
                let mut t = self.fichiers_requete();
                if refocus {
                    t = Task::batch([t, iced::widget::operation::focus(INPUT_ID)]);
                }
                t
            }
            SMsg::Files(mot, st) => {
                let mut a_jour = false;
                // chargement ou scan : la réponse viendra, sans attendre une
                // nouvelle frappe
                let patienter = st.loading || st.scanning.is_some();
                if let Some(s) = self.search.as_mut()
                    && crate::fileindex::proto::query(&s.query) == Some(mot.as_str())
                {
                    // réponse en retard pour un mot depuis changé : jetée
                    s.files = Some(st);
                    s.rank_files();
                    a_jour = true;
                }
                let mut taches: Vec<Task<Msg>> = vec![self.file_preview()];
                if a_jour && patienter {
                    let delai = if self
                        .search
                        .as_ref()
                        .and_then(|s| s.files.as_ref())
                        .is_some_and(|f| f.loading)
                    {
                        500
                    } else {
                        2000
                    };
                    taches.push(Task::perform(
                        tokio::time::sleep(std::time::Duration::from_millis(delai)),
                        move |_| Msg::Search(SMsg::FilesRetry(mot.clone())),
                    ));
                }
                if a_jour {
                    // icônes des résultats, résolues hors du thread UI
                    let noms: std::collections::HashSet<&'static str> = self
                        .search
                        .as_ref()
                        .and_then(|s| s.files.as_ref())
                        .map(|f| {
                            f.hits
                                .iter()
                                .map(|h| crate::launch::path_icon_name(Path::new(&h.path)))
                                .collect()
                        })
                        .unwrap_or_default();
                    taches.extend(noms.into_iter().map(|n| self.want_icon(n)));
                }
                Task::batch(taches)
            }
            SMsg::FilesRetry(mot) => {
                if self
                    .search
                    .as_ref()
                    .is_some_and(|s| crate::fileindex::proto::query(&s.query) == Some(mot.as_str()))
                {
                    self.fichiers_requete()
                } else {
                    Task::none()
                }
            }
            SMsg::FileFilter(k) => {
                if let Some(s) = self.search.as_mut()
                    && crate::fileindex::proto::query(&s.query).is_some()
                    && let Some(f) = s.files.as_mut()
                {
                    f.kind = k;
                }
                // la requête repart avec la nature demandée
                self.fichiers_requete()
            }
            SMsg::CatFilter(c) => {
                if let Some(s) = self.search.as_mut()
                    && crate::fileindex::proto::query(&s.query).is_some()
                    && let Some(f) = s.files.as_mut()
                {
                    f.cat = c;
                }
                self.fichiers_requete()
            }
            SMsg::Launch(k)
                if self
                    .search
                    .as_ref()
                    .is_some_and(|s| crate::fileindex::proto::query(&s.query).is_some()) =>
            {
                let hit = self.search.as_ref().and_then(|s| {
                    let i = *s.results.get(k)?;
                    s.files.as_ref()?.hits.get(i).cloned()
                });
                let Some(hit) = hit else {
                    return Task::none();
                };
                let close = self.close_search();
                let ouvre = Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::launch::open_path_checked(Path::new(&hit.path))
                        })
                        .await
                        .unwrap_or(Err("tâche interrompue".into()))
                    },
                    |r| {
                        if let Err(e) = r {
                            tracing::warn!("{}", crate::tr!("ouverture : {e}", e = e));
                            // L'échec est montré, pas seulement journalisé
                            // (HIG PAT-11) ; la cause nomme le chemin.
                            crate::hypr::notify(
                                0,
                                8000,
                                &crate::tr!("Ouverture impossible : {e}", e = e),
                            );
                        }
                        Msg::Noop
                    },
                );
                Task::batch([close, ouvre])
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
            // Mode réglages (`:r`) : l'index du tiroir, son filtre de
            // visibilité — ce que le lanceur trouve, le tiroir le montre
            // (HIG PAT-13).
            SMsg::Launch(k)
                if self
                    .search
                    .as_ref()
                    .is_some_and(|s| s.setting_query().is_some()) =>
            {
                let Some((e, q)) = self.search.as_ref().and_then(|s| {
                    let e = s.settings.get(*s.results.get(k)?)?;
                    Some((*e, s.setting_query()?.to_string()))
                }) else {
                    return Task::none();
                };
                // Le lanceur laisse la place au tiroir, ouvert sur le paquet
                // du réglage. `Search` pose la requête — les mots cherchés y
                // sont surlignés —, `SearchOpen` quitte la liste des
                // résultats et montre le paquet : l'ordre compte.
                let close = self.close_search();
                let open = self.open_panel();
                Task::batch([
                    close,
                    open,
                    Task::done(Msg::Panel(super::panel::PanelMsg::Search(q))),
                    Task::done(Msg::Panel(super::panel::PanelMsg::SearchOpen(
                        e.page, e.tab,
                    ))),
                ])
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
            SMsg::ApercuVoulu(path) => {
                let attendu = self.search.as_ref().is_some_and(|s| {
                    s.fichier_choisi() == Some(path.as_str())
                        && matches!(&s.apercu, Some((p, Apercu::Attente)) if *p == path)
                });
                if !attendu {
                    return Task::none();
                }
                Task::perform(
                    async move {
                        let p = path.clone();
                        let a = tokio::task::spawn_blocking(move || charger_apercu(Path::new(&p)))
                            .await
                            .unwrap_or(Apercu::Rien);
                        (path, a)
                    },
                    |(p, a)| Msg::Search(SMsg::Apercu(p, a)),
                )
            }
            SMsg::Apercu(path, a) => {
                if let Some(s) = self.search.as_mut() {
                    s.garde_apercu(path, a);
                }
                Task::none()
            }
            SMsg::Shown => {
                if let Some(s) = self.search.as_mut()
                    && !s.shown
                {
                    s.shown = true;
                    if let Some(f) = s.visibility.as_mut() {
                        f.from = 0.0;
                        f.start = std::time::Instant::now();
                    }
                }
                iced::widget::operation::focus(INPUT_ID)
            }
            // Mode réglages (`:r`) : même index et même filtre que le tiroir.
            SMsg::Query(q) if super::settings_search::query(&q).is_some() => {
                let avant = self
                    .search
                    .as_ref()
                    .map(|s| s.view_kind())
                    .unwrap_or_default();
                let mot = super::settings_search::query(&q)
                    .unwrap_or_default()
                    .to_string();
                // Même index et même filtre que le tiroir, classés pour une
                // liste courte.
                let found = super::settings_search::launcher(&mot, |id| self.setting_visible(id));
                let Some(s) = self.search.as_mut() else {
                    return Task::none();
                };
                s.query = q;
                s.settings = found;
                s.results = (0..s.settings.len().min(SHOWN)).collect();
                s.selected = 0;
                // Changement de vue : le champ reprend le focus.
                if s.view_kind() != avant {
                    iced::widget::operation::focus(INPUT_ID)
                } else {
                    Task::none()
                }
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
                // `:clip`, `:2fa`… changent de vue : le nouveau champ reprend le focus.
                let mut refocus = false;
                if let Some(s) = self.search.as_mut() {
                    let before = s.view_kind();
                    s.query = q;
                    s.results = results;
                    s.selected = 0;
                    refocus = s.view_kind() != before;
                }
                let mut tasks: Vec<Task<Msg>> = icons.iter().map(|n| self.want_icon(n)).collect();
                if refocus {
                    tasks.push(iced::widget::operation::focus(INPUT_ID));
                }
                Task::batch(tasks)
            }
            SMsg::Clear => {
                // Le bouton disparaît avec la requête : le champ reprend le focus.
                let cleared = self.search_update(SMsg::Query(String::new()));
                Task::batch([cleared, iced::widget::operation::focus(INPUT_ID)])
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

    /// La carte de recherche. Son apparition donne le focus au champ : le
    /// focus demandé à l'ouverture part avant que la surface n'existe.
    pub(super) fn view_search(&self) -> Element<'_, Msg> {
        let Some(s) = &self.search else {
            return iced::widget::Space::new().into();
        };
        let card = self.view_search_card(s);
        // Le mode convexe pose son propre voile, qui suit le tiroir.
        let content: Element<'_, Msg> = if self.convex_options().is_some_and(|o| o.attached_panels)
        {
            card
        } else {
            // Voile réglé (MLG-278), plus dense sous la feuille.
            let shape = super::apps_grid::shape();
            let dim = shape.backdrop_dim;
            let alpha = if s.grid { (dim * 1.5).min(0.9) } else { dim };
            // Ouverture (MLG-278) : effet et durée réglés, mouvement du thème.
            let p = s
                .opening
                .map_or(1.0, |f| f.value(std::time::Instant::now()));
            let mo = super::tokens::motion();
            let (scale, dy) = match shape.open_effect {
                crate::config::OpenEffect::Fade => (1.0, mo.offset * (1.0 - p)),
                crate::config::OpenEffect::Scale => {
                    (mo.scale.min(0.94) + (1.0 - mo.scale.min(0.94)) * p, 0.0)
                }
                crate::config::OpenEffect::Slide => (1.0, 48.0 * (1.0 - p)),
                crate::config::OpenEffect::None => (1.0, 0.0),
            };
            let card = crate::ui::plugins::bar::motion::surface(
                card,
                scale,
                iced::Vector::new(0.0, dy),
                iced::Point::new(0.5, 0.5),
            );
            // Place : centre de l'écran, ou en haut (décalage réglé).
            let placed: Element<'_, Msg> = if shape.top {
                container(card)
                    .center_x(Length::Fill)
                    .height(Length::Fill)
                    .align_y(Alignment::Start)
                    .padding(iced::Padding {
                        top: f32::from(shape.offset.max(0)) + 48.0,
                        ..iced::Padding::ZERO
                    })
                    .into()
            } else {
                container(card)
                    .center(Length::Fill)
                    .padding(iced::Padding {
                        top: f32::from(shape.offset.max(0)) * 2.0,
                        bottom: f32::from((-shape.offset).max(0)) * 2.0,
                        ..iced::Padding::ZERO
                    })
                    .into()
            };
            iced::widget::stack![backdrop(alpha * p), placed].into()
        };
        iced::widget::sensor(content)
            .key(format!("{:?}", s.win))
            .on_show(|_| Msg::Search(SMsg::Shown))
            .into()
    }

    fn view_search_card<'a>(&'a self, s: &'a Search) -> Element<'a, Msg> {
        if crate::mymfa::query(&s.query).is_some() {
            return self.view_mfa_search(s);
        }
        let reglages = s.setting_query();
        if crate::fileindex::proto::query(&s.query).is_some() || reglages.is_some() {
            // Mode fichiers (`:f`) ou réglages (`:r`) : la même carte.
            let th = &self.cfg.general.theme;
            let r = crate::theme::readable(th.fg, th.bg, th.accent);
            let (fg, dim, accent) = (color(r.fg, 1.0), color(r.dim, 1.0), color(r.focus, 1.0));
            let bg = color(th.bg, 1.0);
            let (placeholder, list) = match reglages {
                // Réglages : l'index du tiroir.
                Some(q) => (
                    crate::i18n::tr("Rechercher dans les réglages…"),
                    self.view_settings(s, q, fg, dim, accent),
                ),
                // Fichiers : résultats du démon d'index.
                None => (
                    crate::i18n::tr("Rechercher un fichier ou un dossier…"),
                    self.view_files(s, fg, dim, accent),
                ),
            };
            let input = self.launcher_field(s, Length::Fill, 22.0, placeholder);
            return container(self.search_card(s, input, list, bg, fg))
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .padding(if s.visibility.is_some() { 0.0 } else { SHADOW })
                .into();
        }
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
        let input = self.launcher_field(
            s,
            Length::Fill,
            22.0,
            crate::i18n::tr("Rechercher une application… (:f, :clip, :2fa, :mdp, :r)"),
        );
        container(self.search_card(s, input, self.view_clips(s, fg, dim, accent), bg, fg))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(if s.visibility.is_some() { 0.0 } else { SHADOW })
            .into()
    }

    /// Résultats du mode fichiers (`:f`) : icône par type de fichier, nom,
    /// taille et dossier parent.
    fn view_files<'a>(
        &'a self,
        s: &'a Search,
        fg: Color,
        dim: Color,
        accent: Color,
    ) -> Element<'a, Msg> {
        let mut list = column![].spacing(2);
        use crate::i18n::tr;
        let suffixe = crate::fileindex::proto::query(&s.query).unwrap_or_default();
        let note = match s.files.as_ref() {
            None => Some(tr("Démarrage du démon d'index…").to_string()),
            Some(f) => match (&f.scanning, &f.erreur) {
                _ if f.loading => Some(tr("Chargement de l'index…").to_string()),
                (Some(n), _) => Some(format!("{} ({n})", tr("Indexation en cours…"))),
                (_, Some(e)) => Some(format!("{} : {e}", tr("Index indisponible"))),
                _ if suffixe.is_empty() => {
                    Some(tr("Nom du fichier, ou chemin avec « / » (ex. docs/rapport).").to_string())
                }
                _ if f.hits.is_empty() => Some(tr("Aucun fichier ne correspond.").to_string()),
                _ => None,
            },
        };
        if let Some(msg) = note {
            list = list.push(
                text(msg)
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
                    .color(dim),
            );
        }
        // Puces de nature : tout, fichiers, dossiers, liens — la puce
        // active relance la requête avec la nature demandée côté démon.
        use crate::fileindex::index::{KIND_DIR, KIND_FILE, KIND_LINK};
        let kind_active = s.files.as_ref().and_then(|f| f.kind);
        let mut chips = row![].spacing(6);
        for (k, label) in [
            (None, tr("Tout").to_string()),
            (Some(KIND_FILE), tr("Fichiers").to_string()),
            (Some(KIND_DIR), tr("Dossiers").to_string()),
            (Some(KIND_LINK), tr("Liens").to_string()),
        ] {
            let actif = kind_active == k;
            chips = chips.push(
                button(
                    text(label)
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                        .color(fg),
                )
                .padding([4, 10])
                .style(move |_, status| button::Style {
                    background: (actif || matches!(status, button::Status::Hovered)).then(|| {
                        Color {
                            a: if actif { 0.35 } else { 0.12 },
                            ..accent
                        }
                        .into()
                    }),
                    border: Border::default().rounded(999),
                    text_color: fg,
                    ..Default::default()
                })
                .on_press(Msg::Search(SMsg::FileFilter(k))),
            );
        }
        list = list.push(chips);
        // Puces de catégorie : déduites de l'extension au vol, le filtre est
        // appliqué côté démon.
        let cat_active = s.files.as_ref().and_then(|f| f.cat);
        let mut chats = row![].spacing(6);
        for (c, label) in crate::fileindex::index::CATEGORIES {
            let actif = cat_active == Some(*c);
            chats = chats.push(
                button(
                    text(tr(label).to_string())
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                        .color(fg),
                )
                .padding([4, 10])
                .style(move |_, status| button::Style {
                    background: (actif || matches!(status, button::Status::Hovered)).then(|| {
                        Color {
                            a: if actif { 0.35 } else { 0.12 },
                            ..accent
                        }
                        .into()
                    }),
                    border: Border::default().rounded(999),
                    text_color: fg,
                    ..Default::default()
                })
                .on_press(Msg::Search(SMsg::CatFilter(Some(*c)))),
            );
        }
        chats = chats.push(
            button(
                text(tr("Toutes").to_string())
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(fg),
            )
            .padding([4, 10])
            .style(move |_, status| button::Style {
                background: (cat_active.is_none() || matches!(status, button::Status::Hovered))
                    .then(|| {
                        Color {
                            a: if cat_active.is_none() { 0.35 } else { 0.12 },
                            ..accent
                        }
                        .into()
                    }),
                border: Border::default().rounded(999),
                text_color: fg,
                ..Default::default()
            })
            .on_press(Msg::Search(SMsg::CatFilter(None))),
        );
        list = list.push(chats);
        let hits = s.files.as_ref().map(|f| f.hits.as_slice()).unwrap_or(&[]);
        for (k, &i) in s.results.iter().enumerate() {
            let Some(h) = hits.get(i) else { continue };
            let icone = crate::launch::path_icon_name(Path::new(&h.path));
            let detail = {
                let dir = Path::new(&h.path)
                    .parent()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                let dir = if dir.chars().count() > 34 {
                    let queue: String = dir
                        .chars()
                        .rev()
                        .take(33)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect();
                    format!("…{queue}")
                } else {
                    dir
                };
                format!("{} · {}", crate::fileindex::fmt_taille(h.size), dir)
            };
            let icon: Element<'_, Msg> = self
                .icon_named(icone, 22.0, dim)
                .unwrap_or_else(|| iced::widget::Space::new().width(22).height(22).into());
            let selected = k == s.selected;
            let ligne = row![
                icon,
                text(h.name.clone())
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
                    .color(fg),
                iced::widget::Space::new().width(Length::Fill),
                text(detail)
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
                    .color(dim),
            ]
            .spacing(14)
            .align_y(Alignment::Center);
            // Le résultat sélectionné porte son aperçu (HIG DAT-03).
            let apercu = s
                .apercu
                .as_ref()
                .filter(|(p, _)| selected && *p == h.path)
                .map(|(_, a)| self.file_preview_row(a, h.mtime, dim));
            let (contenu, haut): (Element<'_, Msg>, f32) = match apercu {
                Some(a) => (
                    column![
                        container(ligne).height(ROW - 16.0).center_y(Length::Fill),
                        a
                    ]
                    .spacing(8)
                    .into(),
                    ROW + APERCU_H + 8.0,
                ),
                None => (ligne.into(), ROW),
            };
            list = list.push(
                button(contenu)
                    .padding([8, 12])
                    .width(Length::Fill)
                    .height(haut)
                    .style(move |_, status| {
                        crate::ui::tokens::item(
                            accent,
                            fg,
                            selected,
                            status,
                            f32::from(super::apps_grid::shape().selection_radius).min(10.0),
                        )
                    })
                    .on_press(Msg::Search(SMsg::Launch(k))),
            );
        }
        list.into()
    }

    /// Rangée d'aperçu sous le résultat sélectionné, alignée sur le nom :
    /// vignette d'image ou premières lignes d'un texte, puis la date de
    /// modification. Hauteur fixe : la liste ne saute pas à l'arrivée.
    fn file_preview_row<'a>(&self, a: &Apercu, mtime: i64, dim: Color) -> Element<'a, Msg> {
        let petit = |t: String| {
            text(t)
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim)
                .wrapping(iced::widget::text::Wrapping::None)
        };
        let date = (mtime > 0)
            .then(|| chrono::DateTime::from_timestamp(mtime, 0))
            .flatten()
            .map(|d| {
                crate::tr!(
                    "Modifié le {d}",
                    d = d
                        .with_timezone(&chrono::Local)
                        .format("%d/%m/%Y %H:%M")
                        .to_string()
                )
            })
            .unwrap_or_default();
        let corps: Element<'a, Msg> = match a {
            Apercu::Image(h) => row![
                container(
                    iced::widget::image(h.clone())
                        .width(64)
                        .height(36)
                        .content_fit(iced::ContentFit::Cover)
                        .border_radius(10),
                ),
                petit(date),
            ]
            .spacing(12)
            .align_y(Alignment::Center)
            .into(),
            Apercu::Texte(l) => {
                let mut c = column![].spacing(2);
                for (i, t) in l.iter().enumerate() {
                    // Une ligne longue est coupée au bord, pas dessinée sur la date.
                    let ligne = container(petit(t.clone())).width(Length::Fill).clip(true);
                    c = c.push(if i == 0 {
                        row![ligne, petit(date.clone())].spacing(12)
                    } else {
                        row![ligne]
                    });
                }
                c.into()
            }
            Apercu::Attente | Apercu::Rien => petit(date).into(),
        };
        container(corps)
            .padding(iced::Padding {
                left: 36.0,
                ..iced::Padding::ZERO
            })
            .height(APERCU_H)
            .center_y(APERCU_H)
            .width(Length::Fill)
            .into()
    }

    /// Mode coffre (`:2fa`, `:mdp`) : la même carte que le presse-papiers.
    fn view_mfa_search<'a>(&'a self, s: &'a Search) -> Element<'a, Msg> {
        let th = &self.cfg.general.theme;
        let r = crate::theme::readable(th.fg, th.bg, th.accent);
        let (fg, dim, accent) = (color(r.fg, 1.0), color(r.dim, 1.0), color(r.focus, 1.0));
        let bg = color(th.bg, 1.0);
        let input = self.launcher_field(
            s,
            Length::Fill,
            22.0,
            crate::i18n::tr("Rechercher une application… (:f, :clip, :2fa, :mdp, :r)"),
        );
        use crate::i18n::tr;
        let mut list = column![].spacing(2);
        let note = match s.mfa.as_ref().map(|(_, l)| l) {
            None | Some(MfaList::Loading) => Some(tr("Lecture du coffre…").to_string()),
            Some(MfaList::Absent) => Some(
                tr("Le client myMFA n'est pas installé (Réglages → Mots de passe).").to_string(),
            ),
            Some(MfaList::NoVault) => {
                Some(tr("Pas encore de coffre myMFA : Entrée pour le créer.").to_string())
            }
            Some(MfaList::Locked) => {
                Some(tr("Coffre verrouillé : Entrée pour le déverrouiller.").to_string())
            }
            Some(MfaList::Error(e)) => Some(e.clone()),
            Some(_) if s.results.is_empty() => Some(tr("Aucun élément ne correspond.").to_string()),
            Some(_) => None,
        };
        if let Some(msg) = note {
            list = list.push(
                text(msg)
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
                    .color(dim),
            );
        }
        let rows: Vec<(String, String)> = match s.mfa.as_ref().map(|(_, l)| l) {
            Some(MfaList::Codes(c)) => s
                .results
                .iter()
                .filter_map(|&i| c.get(i))
                .map(|c| {
                    let code = if c.code.len() == 6 {
                        format!("{} {}", &c.code[..3], &c.code[3..])
                    } else {
                        c.code.clone()
                    };
                    let detail = if c.remaining > 0 {
                        format!("{code} · {} s", c.remaining)
                    } else {
                        code
                    };
                    (c.label(), detail)
                })
                .collect(),
            Some(MfaList::Logins(l)) => s
                .results
                .iter()
                .filter_map(|&i| l.get(i))
                .map(|l| (l.label(), tr("copier le mot de passe").to_string()))
                .collect(),
            _ => Vec::new(),
        };
        for (k, (label, detail)) in rows.into_iter().enumerate() {
            let icon: Element<'_, Msg> = self
                .icon_named(MFA_ICON, 22.0, dim)
                .unwrap_or_else(|| iced::widget::Space::new().width(22).height(22).into());
            let selected = k == s.selected;
            list = list.push(
                button(
                    row![
                        icon,
                        text(label)
                            .font(self.font)
                            .size(crate::ui::tokens::ts(13.0))
                            .color(fg),
                        iced::widget::Space::new().width(Length::Fill),
                        text(detail)
                            .font(self.font)
                            .size(crate::ui::tokens::ts(13.0))
                            .color(dim),
                    ]
                    .spacing(14)
                    .align_y(Alignment::Center),
                )
                .padding([8, 12])
                .width(Length::Fill)
                .height(ROW)
                .style(move |_, status| {
                    crate::ui::tokens::item(
                        accent,
                        fg,
                        selected,
                        status,
                        f32::from(super::apps_grid::shape().selection_radius).min(10.0),
                    )
                })
                .on_press(Msg::Search(SMsg::Launch(k))),
            );
        }
        container(self.search_card(s, input, list.into(), bg, fg))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding(if s.visibility.is_some() { 0.0 } else { SHADOW })
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
            list = list.push(
                text(msg)
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
                    .color(dim),
            );
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
                    row![
                        icon,
                        text(preview)
                            .font(self.font)
                            .size(crate::ui::tokens::ts(13.0))
                            .color(fg)
                    ]
                    .spacing(14)
                    .align_y(Alignment::Center),
                )
                .padding([8, 12])
                .width(Length::Fill)
                .height(ROW)
                .style(move |_, status| {
                    crate::ui::tokens::item(
                        accent,
                        fg,
                        selected,
                        status,
                        f32::from(super::apps_grid::shape().selection_radius).min(10.0),
                    )
                })
                .on_press(Msg::Search(SMsg::Launch(k))),
            );
        }
        list.into()
    }

    /// Résultats du mode réglages (`:r`) : l'icône du paquet, le libellé du
    /// réglage — mots cherchés surlignés —, le paquet en texte secondaire.
    /// Une ligne d'invite sans mot ; rien ne défile, comme les autres modes.
    fn view_settings<'a>(
        &'a self,
        s: &'a Search,
        q: &str,
        fg: Color,
        dim: Color,
        accent: Color,
    ) -> Element<'a, Msg> {
        use crate::i18n::tr;
        let note = if q.is_empty() {
            Some(tr("Tapez le nom d'un réglage, d'un onglet ou d'un paquet.").to_string())
        } else if s.results.is_empty() {
            Some(crate::tr!("Aucun réglage ne correspond à « {q} ».", q = q))
        } else {
            None
        };
        if let Some(msg) = note {
            return text(msg)
                .font(self.font)
                .size(crate::ui::tokens::ts(13.0))
                .color(dim)
                .into();
        }
        let mut list = column![].spacing(2);
        for (k, &i) in s.results.iter().enumerate() {
            let Some(e) = s.settings.get(i) else {
                continue;
            };
            let Some(info) = super::plugins::BUILTIN.iter().find(|p| p.id == e.page) else {
                continue;
            };
            let glyph: Element<'_, Msg> = super::icons::get(info.icon)
                .map(|h| {
                    iced::widget::svg(h)
                        .width(20)
                        .height(20)
                        .style(move |_, _| iced::widget::svg::Style { color: Some(fg) })
                        .into()
                })
                .unwrap_or_else(|| iced::widget::Space::new().width(20).height(20).into());
            // Une ligne par rangée : un texte long (une aide) est coupé, pour
            // qu'il tienne à côté du nom du paquet.
            let label = tr(&e.text);
            let label = if label.chars().count() > 44 {
                format!("{}…", label.chars().take(43).collect::<String>())
            } else {
                label.to_string()
            };
            let selected = k == s.selected;
            list = list.push(
                button(
                    row![
                        glyph,
                        self.marked_with(&label, q, 13, fg, self.font),
                        iced::widget::Space::new().width(Length::Fill),
                        text(tr(info.label))
                            .font(self.font)
                            .size(crate::ui::tokens::ts(13.0))
                            .color(dim),
                    ]
                    .spacing(14)
                    .align_y(Alignment::Center),
                )
                .padding([8, 12])
                .width(Length::Fill)
                .height(ROW)
                .style(move |_, status| {
                    crate::ui::tokens::item(
                        accent,
                        fg,
                        selected,
                        status,
                        f32::from(super::apps_grid::shape().selection_radius).min(10.0),
                    )
                })
                .on_press(Msg::Search(SMsg::Launch(k))),
            );
        }
        list.into()
    }

    fn search_card<'a>(
        &'a self,
        _s: &'a Search,
        input: Element<'a, Msg>,
        list: Element<'a, Msg>,
        bg: Color,
        fg: Color,
    ) -> Element<'a, Msg> {
        if self.convex_options().is_some_and(|o| o.attached_panels) {
            return self.convex_search_surface(
                _s,
                column![iced::widget::scrollable(list).height(Length::Fill), input]
                    .spacing(12)
                    .into(),
                super::apps_grid::spot_width(),
                super::apps_grid::spot_height(!_s.suggested.is_empty()),
            );
        }
        super::glass::card(
            container(
                column![
                    input,
                    crate::ui::tokens::content(list, &self.cfg.general.theme, 14.0)
                ]
                .spacing(12),
            )
            .padding(16)
            .width(super::apps_grid::spot_width())
            .height(super::apps_grid::spot_height(!_s.suggested.is_empty())),
            crate::config::Place::Launcher,
            super::design::RADIUS,
            fg,
            self.launcher_tint(bg),
            container::Style {
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
            },
            true,
            self.launcher_under(),
        )
    }
}

/// Voile plein écran derrière le lanceur ; un clic dessus le referme.
fn backdrop<'a>(alpha: f32) -> Element<'a, Msg> {
    let area = iced::widget::mouse_area(
        container(iced::widget::Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(Color::from_rgba(0.0, 0.0, 0.0, alpha).into()),
                ..Default::default()
            }),
    );
    // Un clic hors de la carte la ferme, sauf réglage contraire (MLG-278).
    if super::apps_grid::shape().close_on_outside_click {
        area.on_press(Msg::Search(SMsg::Close)).into()
    } else {
        area.on_press(Msg::Noop).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Début de fichier : du texte UTF-8 donne ses deux premières lignes
    /// non vides ; un octet nul ou de l'UTF-8 invalide, rien (HIG DAT-03).
    #[test]
    fn apercu_texte() {
        assert_eq!(
            texte_apercu(b"\n\n# Titre\n\nligne deux\nligne trois"),
            Some(vec!["# Titre".to_string(), "ligne deux".to_string()])
        );
        assert_eq!(texte_apercu(b"PNG\0\x01"), None);
        assert_eq!(texte_apercu(b"\xff\xfe abc"), None);
        assert_eq!(texte_apercu(b"   \n\t\n"), None);
        // Coupure au milieu d'un caractère en fin de tampon : tolérée.
        let mut b = "été".as_bytes().to_vec();
        b.push(0xc3);
        assert_eq!(texte_apercu(&b), Some(vec!["été".to_string()]));
        let long = "x".repeat(200);
        let l = texte_apercu(long.as_bytes()).unwrap();
        assert_eq!(l[0].chars().count(), 80);
        assert!(l[0].ends_with('…'));
    }

    /// Un aperçu arrivé pour un autre chemin que la sélection est jeté ; la
    /// sélection déplacée en demande un nouveau.
    #[test]
    fn apercu_de_la_selection() {
        let mut app = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-ui-test/config.lua"),
            true,
        )
        .0;
        let _ = app.toggle_search();
        let hit = |p: &str| crate::fileindex::index::Hit {
            path: p.into(),
            name: p.rsplit('/').next().unwrap_or(p).into(),
            kind: crate::fileindex::index::KIND_FILE,
            size: 10,
            mtime: 1_700_000_000,
            fs: "ext4".into(),
        };
        let s = app.search.as_mut().unwrap();
        s.query = ":f note".into();
        s.files = Some(FilesState {
            hits: vec![hit("/a/note.txt"), hit("/b/notes.md")],
            ..Default::default()
        });
        s.results = vec![0, 1];
        s.selected = 0;
        assert!(!s.garde_apercu("/b/notes.md".into(), Apercu::Rien));
        assert!(s.apercu.is_none());
        assert!(s.garde_apercu("/a/note.txt".into(), Apercu::Texte(vec!["x".into()])));
        let _ = app.search_step(1);
        let s = app.search.as_ref().unwrap();
        assert!(matches!(&s.apercu, Some((p, Apercu::Attente)) if p == "/b/notes.md"));
    }
}
