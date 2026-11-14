//! Paquet intégré « Index de fichiers » : la recherche `:f` du lanceur
//! (`crate::fileindex`) — racines (éditeur graphique), exclusions par nom et
//! par expression régulière, type de fichiers (extensions, taille max),
//! rescan de sûreté, état du démon et emplacement/taille du snapshot.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::futures::{SinkExt, Stream};
use iced::widget::{button, column, row, text_input};
use iced::{Alignment, Element, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "index",
    label: "Index de fichiers",
    icon: "system-search",
    summary: "Recherche :f du lanceur : racines, exclusions, type de fichiers, état du démon.",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Index(m)))
}

/// L'état du démon, relu du JSON de `hyprdesk index status`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StatusView {
    /// "scan" (premier parcours), "pret", "attente".
    pub phase: String,
    /// Fichiers indexés (index chargé).
    pub files: u64,
    /// Entrées vues pendant le scan en cours.
    pub progression: u64,
    /// Racines et leur type de FS.
    pub volumes: Vec<(String, String)>,
    /// Racines effectivement sous veille inotify.
    pub watched: usize,
    /// Emplacement du snapshot.
    pub snapshot: String,
    /// Taille du snapshot sur disque (octets).
    pub snapshot_size: u64,
}

pub fn parse_status(json: &str) -> Option<StatusView> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    Some(StatusView {
        phase: v["phase"].as_str().unwrap_or("?").to_string(),
        files: v["fichiers"].as_u64().unwrap_or(0),
        progression: v["progression"].as_u64().unwrap_or(0),
        volumes: v["volumes"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| {
                        Some((
                            x["root"].as_str()?.to_string(),
                            x["fs"].as_str()?.to_string(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default(),
        watched: v["veilles"].as_array().map(|a| a.len()).unwrap_or(0),
        snapshot: v["snapshot"].as_str().unwrap_or_default().to_string(),
        snapshot_size: v["snapshot_size"].as_u64().unwrap_or(0),
    })
}

/// L'indicateur porté par la loupe du dock : prêt (pastille d'accent) ou
/// scan en cours (pastille ambrée) ; démon arrêté = pas de pastille.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::ui) enum IndexInk {
    Ready,
    Scan,
}

/// Flux périodique de l'état du démon (toutes les 5 s) : la loupe du dock
/// porte l'indicateur, la page Réglages se met à jour au passage.
pub fn status_stream() -> impl Stream<Item = UiMsg> {
    iced::stream::channel(
        4,
        async move |mut out: iced::futures::channel::mpsc::Sender<UiMsg>| {
            loop {
                let rep = tokio::task::spawn_blocking(|| {
                    crate::fileindex::proto::request(&crate::fileindex::proto::Request::Status)
                })
                .await
                .unwrap_or_else(|e| Err(format!("{e}")));
                let st = match rep {
                    Ok(crate::fileindex::proto::Reply::Status(j)) => parse_status(&j),
                    _ => None,
                };
                let _ = out.send(UiMsg::IndexStatus(st)).await;
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        },
    )
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    /// Réponse du démon ; None : injoignable (pas encore démarré).
    pub status: Option<StatusView>,
    /// Racines indexées en cours d'édition (éditeur graphique).
    pub roots: Vec<String>,
    pub root_draft: String,
    /// Exclusions par nom (texte, valeurs séparées par des virgules).
    pub exclude_text: String,
    /// Exclusions par expression régulière (éditeur, validées à l'ajout).
    pub regexes: Vec<String>,
    pub regex_draft: String,
    /// Type de fichiers : extensions exclues (texte), catégories exclues
    /// (noms) et taille max (Mio).
    pub ext_text: String,
    pub cats_excl: Vec<String>,
    pub max_text: String,
    /// Rescan de sûreté (secondes).
    pub interval_text: String,
    /// Retour inline : info (accent) et erreur (ambre).
    pub info: Option<String>,
    pub erreur: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// État du démon lu (None : pas de démon).
    Loaded(Option<StatusView>),
    RootDraft(String),
    RootAdd,
    RootRemove(usize),
    Exclude(String),
    RegexDraft(String),
    RegexAdd,
    RegexRemove(usize),
    ExtText(String),
    /// Bascule une catégorie exclue de l'indexation.
    CatToggle(String),
    MaxText(String),
    Interval(String),
    /// Applique tous les champs aux réglages ; le démon reprendra les
    /// réglages à son prochain démarrage (il est arrêté au passage).
    Appliquer,
    /// Rescan complet demandé au démon en place.
    Reindexer,
    /// Démarre le démon maintenant (réglages résolus, comme au premier :f).
    Demarrer,
    /// Relit l'état après une action.
    Refresh,
}

/// Valeurs séparées par des virgules, sans vides.
fn liste(v: &str) -> Vec<String> {
    v.split(',')
        .map(str::trim)
        .filter(|x| !x.is_empty())
        .map(String::from)
        .collect()
}

/// Lit l'état du démon, hors du thread UI.
pub fn load() -> Task<UiMsg> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(|| {
                crate::fileindex::proto::request(&crate::fileindex::proto::Request::Status)
            })
            .await
            .unwrap_or_else(|e| Err(format!("{e}")))
        },
        |rep| {
            let st = match rep {
                Ok(crate::fileindex::proto::Reply::Status(j)) => parse_status(&j),
                Ok(crate::fileindex::proto::Reply::Err(m)) => {
                    tracing::warn!("{}", crate::tr!("index : {m}", m = m));
                    None
                }
                Err(_) => None,
                Ok(_) => None,
            };
            wrap(Msg::Loaded(st))
        },
    )
}

/// Champs remplis depuis les réglages à l'entrée sur la page.
pub(in crate::ui) fn init_fields(app: &mut App) {
    let Some(p) = app.panel.as_mut() else {
        return;
    };
    let s = &mut p.index;
    s.roots = app.settings.index_paths.clone().unwrap_or_default();
    s.root_draft = String::new();
    s.exclude_text = app
        .settings
        .index_exclude
        .as_ref()
        .map(|l| l.join(", "))
        .unwrap_or_default();
    s.regexes = app.settings.index_exclude_regex.clone().unwrap_or_default();
    s.regex_draft = String::new();
    s.ext_text = app
        .settings
        .index_exclude_ext
        .as_ref()
        .map(|l| l.join(", "))
        .unwrap_or_default();
    s.cats_excl = app.settings.index_exclude_cat.clone().unwrap_or_default();
    s.max_text = app
        .settings
        .index_max_size
        .filter(|v| *v > 0)
        .map(|v| v.to_string())
        .unwrap_or_default();
    s.interval_text = app
        .settings
        .index_interval
        .map(|v| v.to_string())
        .unwrap_or_default();
    s.info = None;
    s.erreur = None;
}

impl App {
    pub(in crate::ui) fn index_page(&self) -> Element<'_, UiMsg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let dim = self.ink().dim;
        let accent = color(self.cfg.general.theme.accent, 1.0);
        let ambre = iced::Color::from_rgb8(0xff, 0xb8, 0x6c);
        let Some(s) = self.panel.as_ref().map(|p| &p.index) else {
            return text("Lecture…").font(self.font).size(13).color(dim).into();
        };
        let mut col = column![self.section("Recherche de fichiers (:f)")].spacing(10);
        col = col.push(
            text("Le lanceur indexe vos fichiers (nom, type, chemin, taille) pour répondre dès la première lettre : « :f » puis un mot. Un démon tourne en arrière-plan, tenu à jour par inotify et un rescan périodique.")
                .font(self.font)
                .size(12)
                .color(dim),
        );

        // — État —
        col = col.push(self.section("État"));
        match &s.status {
            None => {
                col = col.push(
                    text("Pas encore de démon d'index : il démarre au premier « :f » dans le lanceur, ou maintenant :")
                        .font(self.font)
                        .size(12)
                        .color(dim),
                );
                col = col.push(
                    button(text("Démarrer l'indexation").font(self.font).size(13))
                        .padding([6, 14])
                        .on_press(wrap(Msg::Demarrer)),
                );
            }
            Some(st) if st.phase == "scan" => {
                col = col.push(
                    text(crate::tr!(
                        "Indexation en cours… ({n} fichiers vus)",
                        n = st.progression
                    ))
                    .font(self.font)
                    .size(13)
                    .color(fg),
                );
            }
            Some(st) => {
                if st.phase == "pret" {
                    col = col.push(
                        text(crate::tr!("Index prêt : {n} fichiers", n = st.files))
                            .font(self.font)
                            .size(13)
                            .color(fg),
                    );
                }
                for (racine, fs) in &st.volumes {
                    col = col.push(
                        text(format!("{racine} ({fs})"))
                            .font(self.font)
                            .size(12)
                            .color(dim),
                    );
                }
                if st.watched < st.volumes.len() {
                    col = col.push(
                        text(crate::tr!(
                            "Sans veille inotify sur {n} racine(s) : le rescan périodique les couvre.",
                            n = st.volumes.len() - st.watched
                        ))
                        .font(self.font)
                        .size(12)
                        .color(dim),
                    );
                }
                // Localisation et taille de l'index.
                col = col.push(
                    text(crate::tr!(
                        "Snapshot : {p0} ({taille})",
                        p0 = st.snapshot,
                        taille = crate::fileindex::fmt_taille(st.snapshot_size)
                    ))
                    .font(self.font)
                    .size(12)
                    .color(dim),
                );
                col = col.push(
                    button(text("Réindexer maintenant").font(self.font).size(13))
                        .padding([6, 14])
                        .on_press(wrap(Msg::Reindexer)),
                );
            }
        }

        // — Racines indexées : éditeur graphique —
        col = col.push(self.section("Racines indexées"));
        for (i, racine) in s.roots.iter().enumerate() {
            col = col.push(
                row![
                    text(racine.clone()).font(self.font).size(13).color(fg),
                    iced::widget::Space::new().width(iced::Length::Fill),
                    button(text("Retirer").font(self.font).size(12))
                        .padding([4, 10])
                        .on_press(wrap(Msg::RootRemove(i))),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }
        if s.roots.is_empty() {
            col = col.push(
                text("Aucune racine : défaut = home et volumes montés (/media, /run/media/$USER).")
                    .font(self.font)
                    .size(12)
                    .color(dim),
            );
        }
        col = col.push(
            row![
                text_input(
                    "ajouter une racine (ex. ~/Documents, /media/cle)",
                    &s.root_draft
                )
                .font(self.font)
                .size(13)
                .padding(8)
                .width(iced::Length::Fill)
                .on_input(|v| wrap(Msg::RootDraft(v)))
                .on_submit(wrap(Msg::RootAdd)),
                button(text("Ajouter").font(self.font).size(13))
                    .padding([6, 14])
                    .on_press_maybe((!s.root_draft.trim().is_empty()).then(|| wrap(Msg::RootAdd)),),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );
        col = col.push(
            text("Une racine = un volume : les montages ne sont pas traversés. Vide : défaut.")
                .font(self.font)
                .size(12)
                .color(dim),
        );

        // — Exclusions —
        col = col.push(self.section("Exclusions"));
        col = col.push(
            self.field(
                "Noms de dossiers",
                "index_exclude",
                text_input("ex. musiques, dist", &s.exclude_text)
                    .font(self.font)
                    .size(13)
                    .padding(8)
                    .width(iced::Length::Fill)
                    .on_input(|v| wrap(Msg::Exclude(v))),
            ),
        );
        col = col.push(
            text("En plus des défauts : .git, node_modules, target, .cache, __pycache__…")
                .font(self.font)
                .size(12)
                .color(dim),
        );
        for (i, motif) in s.regexes.iter().enumerate() {
            col = col.push(
                row![
                    text(motif.clone()).font(self.font).size(13).color(fg),
                    iced::widget::Space::new().width(iced::Length::Fill),
                    button(text("Retirer").font(self.font).size(12))
                        .padding([4, 10])
                        .on_press(wrap(Msg::RegexRemove(i))),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }
        col = col.push(
            row![
                text_input(
                    "exclusion par expression régulière (ex. \\.iso$)",
                    &s.regex_draft
                )
                .font(self.font)
                .size(13)
                .padding(8)
                .width(iced::Length::Fill)
                .on_input(|v| wrap(Msg::RegexDraft(v)))
                .on_submit(wrap(Msg::RegexAdd)),
                button(text("Ajouter").font(self.font).size(13))
                    .padding([6, 14])
                    .on_press_maybe(
                        (!s.regex_draft.trim().is_empty()).then(|| wrap(Msg::RegexAdd)),
                    ),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );
        col = col.push(
            text("Appliquée au chemin complet ; une regex invalide est refusée à l'ajout.")
                .font(self.font)
                .size(12)
                .color(dim),
        );

        // — Type de fichiers —
        col = col.push(self.section("Type de fichiers"));
        col = col.push(
            text("Catégories exclues de l'indexation :")
                .font(self.font)
                .size(13)
                .color(fg),
        );
        let mut cats = row![].spacing(6);
        for (_code, label) in crate::fileindex::index::CATEGORIES {
            let nom = (*label).to_lowercase();
            let actif = s.cats_excl.iter().any(|c| c == &nom);
            cats = cats.push(
                button(text(*label).font(self.font).size(12).color(fg))
                    .padding([4, 10])
                    .style(move |_, status| button::Style {
                        background: (actif || matches!(status, button::Status::Hovered)).then(
                            || {
                                iced::Color {
                                    a: if actif { 0.4 } else { 0.12 },
                                    ..ambre
                                }
                                .into()
                            },
                        ),
                        border: iced::Border::default().rounded(999),
                        text_color: fg,
                        ..Default::default()
                    })
                    .on_press(wrap(Msg::CatToggle(nom))),
            );
        }
        col = col.push(cats.wrap());
        col = col.push(
            text("Une catégorie exclue n'est pas indexée (ex. toutes les vidéos), quelle que soit son extension.")
                .font(self.font)
                .size(12)
                .color(dim),
        );
        col = col.push(
            self.field(
                "Extensions exclues",
                "index_exclude_ext",
                text_input("ex. iso, vdi, img", &s.ext_text)
                    .font(self.font)
                    .size(13)
                    .padding(8)
                    .width(iced::Length::Fill)
                    .on_input(|v| wrap(Msg::ExtText(v))),
            ),
        );
        col = col.push(
            self.field(
                "Taille maximale indexée",
                "index_max_size",
                text_input("Mio, vide : illimitée", &s.max_text)
                    .font(self.font)
                    .size(13)
                    .padding(8)
                    .width(iced::Length::Fill)
                    .on_input(|v| wrap(Msg::MaxText(v))),
            ),
        );

        // — Rescan —
        col = col.push(
            self.field(
                "Rescan de sûreté",
                "index_interval",
                text_input("secondes (défaut 21600)", &s.interval_text)
                    .font(self.font)
                    .size(13)
                    .padding(8)
                    .width(iced::Length::Fill)
                    .on_input(|v| wrap(Msg::Interval(v))),
            ),
        );
        col = col.push(
            text("Réindexation complète à cet intervalle ; inotify couvre le reste.")
                .font(self.font)
                .size(12)
                .color(dim),
        );

        // — retour inline + Appliquer —
        if let Some(m) = &s.info {
            col = col.push(text(m.clone()).font(self.font).size(12).color(accent));
        }
        if let Some(e) = &s.erreur {
            col = col.push(text(e.clone()).font(self.font).size(12).color(ambre));
        }
        col = col.push(
            button(text("Appliquer").font(self.font).size(13))
                .padding([6, 14])
                .on_press(wrap(Msg::Appliquer)),
        );
        col = col.push(
            text("Appliqué au prochain démarrage du démon (premier « :f » ou bouton « Démarrer ») : il est arrêté au passage pour reprendre les réglages.")
                .font(self.font)
                .size(12)
                .color(dim),
        );
        col.into()
    }

    pub(in crate::ui) fn index_update(&mut self, m: Msg) -> Task<UiMsg> {
        match m {
            Msg::Loaded(st) => {
                if let Some(p) = self.panel.as_mut() {
                    p.index.status = st;
                }
                Task::none()
            }
            Msg::RootDraft(v) => {
                if let Some(p) = self.panel.as_mut() {
                    p.index.root_draft = v;
                }
                Task::none()
            }
            Msg::RootAdd => {
                let (brouillon, deja) = {
                    let Some(p) = self.panel.as_ref() else {
                        return Task::none();
                    };
                    (p.index.root_draft.trim().to_string(), p.index.roots.clone())
                };
                if brouillon.is_empty() {
                    return Task::none();
                }
                let mut info = None;
                let erreur = None;
                let mut racines = deja;
                let ecrase = crate::fileindex::expand_root(&brouillon);
                if racines
                    .iter()
                    .any(|r| crate::fileindex::expand_root(r) == ecrase)
                {
                    info =
                        Some(crate::tr!("racine déjà indexée : {p0}", p0 = brouillon).to_string());
                } else {
                    if !ecrase.exists() {
                        info = Some(
                            crate::tr!(
                                "ajoutée — absente du disque pour l'instant : {p0}",
                                p0 = ecrase.display()
                            )
                            .to_string(),
                        );
                    }
                    racines.push(brouillon);
                }
                if let Some(p) = self.panel.as_mut() {
                    p.index.roots = racines;
                    p.index.root_draft = String::new();
                    p.index.info = info;
                    p.index.erreur = erreur;
                }
                Task::none()
            }
            Msg::RootRemove(i) => {
                if let Some(p) = self.panel.as_mut()
                    && i < p.index.roots.len()
                {
                    p.index.roots.remove(i);
                    p.index.info = None;
                }
                Task::none()
            }
            Msg::Exclude(v) => {
                if let Some(p) = self.panel.as_mut() {
                    p.index.exclude_text = v;
                }
                Task::none()
            }
            Msg::RegexDraft(v) => {
                if let Some(p) = self.panel.as_mut() {
                    p.index.regex_draft = v;
                }
                Task::none()
            }
            Msg::RegexAdd => {
                let (brouillon, deja) = {
                    let Some(p) = self.panel.as_ref() else {
                        return Task::none();
                    };
                    (
                        p.index.regex_draft.trim().to_string(),
                        p.index.regexes.clone(),
                    )
                };
                if brouillon.is_empty() {
                    return Task::none();
                }
                let mut info = None;
                let mut erreur = None;
                let mut regexes = deja;
                if regexes.iter().any(|r| r == &brouillon) {
                    info = Some(
                        crate::tr!("exclusion déjà présente : {m}", m = brouillon).to_string(),
                    );
                } else if let Err(e) = regex::Regex::new(&brouillon) {
                    erreur = Some(
                        crate::tr!(
                            "expression régulière invalide : {m} ({e})",
                            m = brouillon,
                            e = e
                        )
                        .to_string(),
                    );
                } else {
                    regexes.push(brouillon);
                }
                if let Some(p) = self.panel.as_mut() {
                    p.index.regexes = regexes;
                    p.index.regex_draft = String::new();
                    p.index.info = info;
                    p.index.erreur = erreur;
                }
                Task::none()
            }
            Msg::RegexRemove(i) => {
                if let Some(p) = self.panel.as_mut()
                    && i < p.index.regexes.len()
                {
                    p.index.regexes.remove(i);
                    p.index.erreur = None;
                }
                Task::none()
            }
            Msg::ExtText(v) => {
                if let Some(p) = self.panel.as_mut() {
                    p.index.ext_text = v;
                }
                Task::none()
            }
            Msg::CatToggle(nom) => {
                if let Some(p) = self.panel.as_mut() {
                    if let Some(i) = p.index.cats_excl.iter().position(|c| c == &nom) {
                        p.index.cats_excl.remove(i);
                    } else {
                        p.index.cats_excl.push(nom);
                    }
                }
                Task::none()
            }
            Msg::MaxText(v) => {
                if let Some(p) = self.panel.as_mut() {
                    p.index.max_text = v;
                }
                Task::none()
            }
            Msg::Interval(v) => {
                if let Some(p) = self.panel.as_mut() {
                    p.index.interval_text = v;
                }
                Task::none()
            }
            Msg::Appliquer => {
                let champs = {
                    let Some(p) = self.panel.as_ref() else {
                        return Task::none();
                    };
                    (
                        p.index.roots.clone(),
                        p.index.exclude_text.clone(),
                        p.index.regexes.clone(),
                        p.index.ext_text.clone(),
                        p.index.max_text.clone(),
                        p.index.interval_text.clone(),
                    )
                };
                let (racines, exclus, regexes, exts, max, interval) = champs;
                let cats = {
                    let Some(p) = self.panel.as_ref() else {
                        return Task::none();
                    };
                    p.index.cats_excl.clone()
                };
                self.settings.index_paths = Some(racines);
                self.settings.index_exclude = Some(liste(&exclus));
                self.settings.index_exclude_regex = Some(regexes);
                self.settings.index_exclude_cat = Some(cats);
                self.settings.index_exclude_ext = Some(
                    liste(&exts)
                        .into_iter()
                        .map(|x| x.trim_start_matches('.').to_lowercase())
                        .collect(),
                );
                if let Ok(v) = max.trim().parse::<u64>() {
                    self.settings.index_max_size = Some(v);
                }
                if let Ok(v) = interval.trim().parse::<u64>() {
                    self.settings.index_interval = Some(v);
                }
                // le démon en place garde ses anciens réglages : on l'arrête,
                // le lanceur le redémarrera avec les réglages frais
                let arret = Task::perform(
                    async {
                        tokio::task::spawn_blocking(crate::fileindex::proto::stop)
                            .await
                            .unwrap_or(Ok(()))
                    },
                    |r| {
                        if let Err(e) = r {
                            tracing::debug!("{}", crate::tr!("index : {m}", m = e));
                        }
                        wrap(Msg::Refresh)
                    },
                );
                self.commit().chain(arret)
            }
            Msg::Reindexer => {
                let demande = Task::perform(
                    async {
                        tokio::task::spawn_blocking(|| {
                            crate::fileindex::proto::request(
                                &crate::fileindex::proto::Request::Rebuild,
                            )
                        })
                        .await
                        .unwrap_or_else(|e| Err(format!("{e}")))
                    },
                    |r| {
                        if let Err(e) = r {
                            tracing::warn!("{}", crate::tr!("index : {m}", m = e));
                        }
                        wrap(Msg::Refresh)
                    },
                );
                demande.chain(load())
            }
            Msg::Demarrer => {
                // réglages résolus, identiques à ceux du premier « :f »
                let mut reglage = crate::fileindex::IndexConfig::default();
                if let Some(l) = &self.settings.index_paths
                    && !l.is_empty()
                {
                    reglage.roots = l.iter().map(|p| crate::fileindex::expand_root(p)).collect();
                }
                reglage.extra_excludes = self.settings.index_exclude.clone().unwrap_or_default();
                reglage.exclude_regex = self
                    .settings
                    .index_exclude_regex
                    .clone()
                    .unwrap_or_default();
                reglage.exclude_ext = self.settings.index_exclude_ext.clone().unwrap_or_default();
                reglage.exclude_cat = self.settings.index_exclude_cat.clone().unwrap_or_default();
                reglage.max_size = self.settings.index_max_size.unwrap_or(0);
                reglage.interval = self
                    .settings
                    .index_interval
                    .unwrap_or(crate::fileindex::DEFAULT_INTERVAL_SECS);
                let demarre = Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::fileindex::proto::ensure_daemon(&reglage)
                        })
                        .await
                        .unwrap_or_else(|e| Err(format!("{e}")))
                    },
                    |r| {
                        if let Err(e) = r {
                            tracing::warn!("{}", crate::tr!("index : {m}", m = e));
                        }
                        wrap(Msg::Refresh)
                    },
                );
                demarre.chain(load())
            }
            Msg::Refresh => load(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statut_json_lu() {
        let j = r#"{"phase":"pret","progression":0,"fichiers":1234,"volumes":[{"root":"/home/x","fs":"ext4"},{"root":"/media/cle","fs":"vfat"}],"veilles":["/home/x"],"dernier_scan":1,"interval_s":1,"snapshot":"/cache/hyprdesk/index/snapshot.bin","snapshot_size":4321,"pid":1}"#;
        let st = parse_status(j).expect("statut");
        assert_eq!(st.phase, "pret");
        assert_eq!(st.files, 1234);
        assert_eq!(st.volumes.len(), 2);
        assert_eq!(st.volumes[0], ("/home/x".to_string(), "ext4".to_string()));
        assert_eq!(st.watched, 1);
        assert_eq!(st.snapshot, "/cache/hyprdesk/index/snapshot.bin");
        assert_eq!(st.snapshot_size, 4321);
    }

    #[test]
    fn statut_illisible() {
        assert!(parse_status("pas du json").is_none());
    }

    #[test]
    fn liste_de_valeurs() {
        assert_eq!(
            liste(" ~/Docs, /media/cle ,, "),
            vec!["~/Docs", "/media/cle"]
        );
        assert!(liste("").is_empty());
    }

    #[test]
    fn regex_ajout_valide() {
        // la validation passe par regex::Regex::new, le motif est gardé tel quel
        assert!(regex::Regex::new("\\.iso$").is_ok());
        // construit à l'exécution : un motif cassé doit être refusé
        let casse = concat!("([(", ")]").to_string();
        assert!(regex::Regex::new(&casse).is_err());
    }
}
