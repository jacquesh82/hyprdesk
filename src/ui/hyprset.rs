//! Rendu unique d'une option de Hyprland dans les Réglages (MLG-137) :
//! effets (`crate::fx`), layouts (`crate::tiling`) et autres options
//! (`crate::hyprset`) passent par `App::opt_field`. Chaque nature d'option a
//! son contrôle (interrupteur, curseur, liste, saisie avec aperçu de couleur
//! ou de dégradé), et l'origine de la valeur est dite : « réglé ici » (avec
//! Rétablir), la ligne de hyprland.lua qui la définit, ou la valeur par
//! défaut de Hyprland.

use std::collections::BTreeMap;
use std::sync::Arc;

use iced::widget::{Space, button, column, container, pick_list, row, slider, text_input, toggler};
use iced::{Alignment, Color, Element, Length, Task};

use super::{App, Msg as UiMsg, color};
use crate::config::Rgba;
use crate::i18n::text;
use crate::tiling::{Kind, Opt, OptValue};

/// Où une option est rangée dans hyprdesk.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Store {
    /// Paquets Animations et Performance (`fx`).
    Fx,
    /// Paquet Layouts (`tiling`).
    Tiling,
    /// Fenêtres, curseur, comportement… (`hypr`).
    Hypr,
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Valeur réglée ici (`None` : rendue à hyprland.lua).
    Set(Store, String, Option<OptValue>),
    /// Curseur en mouvement : appliqué à chaud, enregistré au relâchement.
    Drag(Store, String, f64),
    Commit,
    /// Saisie en cours (couleur, dégradé, côtés, texte).
    Draft(String, String),
    /// Saisie validée (Entrée).
    Submit(Store, String),
    /// Valeurs en vigueur des options `hypr` (`hl.get_config`).
    Current(BTreeMap<String, String>),
    /// Ce que définit hyprland.lua (`crate::hyprsrc`).
    Source(Arc<crate::hyprsrc::Source>),
    /// Variable d'environnement : (indice, Some((nom, valeur))) ou retrait.
    Env(usize, Option<(String, String)>),
    EnvAdd,
    /// Champ d'un élément de liste (règle, geste, périphérique) : (liste,
    /// indice, chemin pointé, valeur ; `None` : retiré), enregistré.
    Spec(
        super::hyprlists::List,
        usize,
        String,
        Option<serde_json::Value>,
    ),
    /// Saisie d'un champ texte d'un élément (enregistrée à Entrée).
    SpecText(
        super::hyprlists::List,
        usize,
        String,
        Option<serde_json::Value>,
    ),
    SpecAdd(super::hyprlists::List),
    SpecDel(super::hyprlists::List, usize),
    /// Déplace un élément (−1 : vers le haut, +1 : vers le bas).
    SpecMove(super::hyprlists::List, usize, i32),
    /// Permission modifiée ou retirée (`None`) ; enregistrée si `true`.
    Perm(usize, Option<crate::hyprset::Permission>, bool),
    PermAdd,
    /// Ajoute les permissions conseillées absentes.
    PermPresets,
    /// Bordures aux couleurs du thème.
    ThemeBorders(bool),
    /// Thème de curseur (XCursor et hyprcursor du même nom si présent).
    CursorTheme(String),
    /// Taille du curseur pendant le glissé, puis enregistrement.
    CursorSize(f64),
    CursorCommit,
}

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::HyprSet(m)
}

#[derive(Debug, Clone, PartialEq)]
struct Choice {
    value: OptValue,
    label: String,
}

impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

/// Option du registre de son magasin.
pub fn registry(store: Store, path: &str) -> Option<&'static Opt> {
    match store {
        Store::Fx => crate::fx::opt(path),
        Store::Tiling => crate::tiling::opt(path),
        Store::Hypr => crate::hyprset::opt(path),
    }
}

/// Couleurs d'une valeur (aperçu).
fn swatches(v: &OptValue) -> Vec<Rgba> {
    match v {
        OptValue::Color(c) => vec![*c],
        OptValue::Gradient(cs, _) => cs.clone(),
        _ => Vec::new(),
    }
}

impl App {
    /// Valeur réglée ici (config en vigueur).
    fn opt_ours(&self, store: Store, path: &str) -> Option<OptValue> {
        let g = &self.cfg.general;
        match store {
            Store::Fx => g.fx.options.get(path),
            Store::Tiling => g.tiling.options.get(path),
            Store::Hypr => g.hypr.options.get(path),
        }
        .cloned()
    }

    /// Valeur en vigueur dans Hyprland, forme lisible.
    fn opt_theirs(&self, store: Store, path: &str) -> Option<String> {
        let p = self.panel.as_ref()?;
        match store {
            Store::Fx => p.animations.current.get(path),
            Store::Tiling => p.layouts.current.get(path),
            Store::Hypr => self.hypr_current.get(path),
        }
        .cloned()
    }

    fn edit_store(&mut self, store: Store, f: impl FnOnce(&mut BTreeMap<String, OptValue>)) {
        match store {
            Store::Fx => self.edit_fx(|x| f(&mut x.options)),
            Store::Tiling => self.edit_tiling(|t| f(&mut t.options)),
            Store::Hypr => {
                let mut h = self
                    .settings
                    .hypr
                    .clone()
                    .unwrap_or_else(|| self.cfg.general.hypr.clone());
                f(&mut h.options);
                self.settings.hypr = Some(h);
            }
        }
    }

    /// Origine d'une valeur non réglée ici : la ligne de hyprland.lua qui la
    /// définit, ou le défaut de Hyprland.
    fn opt_origin_text(&self, path: &str) -> String {
        match self.hypr_source.as_ref().and_then(|s| s.options.get(path)) {
            Some(o) => {
                let file = o
                    .file
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_default();
                crate::tr!("{file}, ligne {n}", file = file, n = o.lines.0)
            }
            None => crate::i18n::tr("valeur par défaut").to_string(),
        }
    }

    /// Champ complet d'une option : libellé, contrôle, origine.
    pub(in crate::ui) fn opt_field(&self, store: Store, path: &str) -> Element<'_, UiMsg> {
        let Some(o) = registry(store, path) else {
            return Space::new().into();
        };
        let path = path.to_string();
        let ours = self.opt_ours(store, &path);
        let theirs = self.opt_theirs(store, &path);
        let dim = self.ink().dim;
        let control = self.opt_control(store, o, &path, ours.as_ref(), theirs.as_deref());
        let origin: Element<'_, UiMsg> = if ours.is_some() {
            let p = path.clone();
            row![
                text(crate::i18n::tr("réglé ici"))
                    .font(self.font)
                    .size(12)
                    .color(color(self.cfg.general.theme.accent, 1.0)),
                button(text(crate::i18n::tr("Rétablir")).font(self.font).size(12))
                    .padding([2, 8])
                    .style(button::text)
                    .on_press(wrap(Msg::Set(store, p, None))),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .into()
        } else {
            text(self.opt_origin_text(&path))
                .font(self.font)
                .size(12)
                .color(dim)
                .into()
        };
        let mut body = column![
            row![container(control).width(Length::Fill), origin]
                .spacing(10)
                .align_y(Alignment::Center)
        ]
        .spacing(4);
        if let Some(e) = self.opt_errors.get(&path) {
            body = body.push(
                text(e.clone())
                    .font(self.font)
                    .size(12)
                    .color(crate::ui::tone::danger()),
            );
        }
        self.field(o.label, "", body)
    }

    fn opt_control(
        &self,
        store: Store,
        o: &'static Opt,
        path: &str,
        ours: Option<&OptValue>,
        theirs: Option<&str>,
    ) -> Element<'_, UiMsg> {
        let p = path.to_string();
        match &o.kind {
            Kind::Bool => {
                let on = match ours {
                    Some(OptValue::Bool(b)) => *b,
                    _ => matches!(theirs, Some("true") | Some("1")),
                };
                toggler(on)
                    .on_toggle(move |v| wrap(Msg::Set(store, p.clone(), Some(OptValue::Bool(v)))))
                    .into()
            }
            Kind::Num { min, max, step } => {
                let v = match ours {
                    Some(OptValue::Num(n)) => *n,
                    _ => theirs.and_then(|s| s.parse::<f64>().ok()).unwrap_or(*min),
                };
                // Curseur extensible, plafonné : en colonne étroite il cède sa
                // place plutôt que de pousser la valeur sur l'origine.
                row![
                    container(
                        slider((*min as f32)..=(*max as f32), v as f32, move |x| wrap(
                            Msg::Drag(store, p.clone(), x as f64)
                        ))
                        .step(*step as f32)
                        .on_release(wrap(Msg::Commit))
                    )
                    .width(Length::Fill)
                    .max_width(240),
                    text(format!("{}", (v * 100.0).round() / 100.0))
                        .font(self.font)
                        .size(13),
                ]
                .spacing(12)
                .align_y(Alignment::Center)
                .into()
            }
            Kind::IntChoice(list) => {
                let choices: Vec<Choice> = list
                    .iter()
                    .map(|(v, l)| Choice {
                        value: OptValue::Num(*v as f64),
                        label: crate::i18n::tr(l).to_string(),
                    })
                    .collect();
                let now = ours.cloned().or_else(|| {
                    theirs
                        .and_then(|s| s.parse::<f64>().ok())
                        .map(OptValue::Num)
                });
                self.opt_pick(store, p, choices, now)
            }
            Kind::Choice(list) => {
                let choices: Vec<Choice> = list
                    .iter()
                    .map(|(v, l)| Choice {
                        value: OptValue::Str(v.to_string()),
                        label: crate::i18n::tr(l).to_string(),
                    })
                    .collect();
                let now = ours
                    .cloned()
                    .or_else(|| theirs.map(|s| OptValue::Str(s.to_string())));
                self.opt_pick(store, p, choices, now)
            }
            Kind::Color | Kind::Gradient | Kind::Text | Kind::Sides { .. } => {
                let shown = self
                    .opt_drafts
                    .get(path)
                    .cloned()
                    .unwrap_or_else(|| ours.map(OptValue::display).unwrap_or_default());
                let hint = match &o.kind {
                    Kind::Color => "#rrggbb ou #rrggbbaa",
                    Kind::Gradient => "#rrggbb #rrggbb 90deg",
                    Kind::Sides { .. } => "5, ou 5 10 5 10",
                    _ => "",
                };
                let placeholder = theirs
                    .map(str::to_string)
                    .unwrap_or_else(|| crate::i18n::tr(hint).to_string());
                let preview: Vec<Rgba> = o
                    .parse(&shown)
                    .ok()
                    .or_else(|| ours.cloned())
                    .map(|v| swatches(&v))
                    .unwrap_or_default();
                let p2 = p.clone();
                let mut r = row![
                    container(
                        text_input(&placeholder, &shown)
                            .on_input(move |s| wrap(Msg::Draft(p2.clone(), s)))
                            .on_submit(wrap(Msg::Submit(store, p)))
                            .font(self.font)
                            .size(13)
                            .width(Length::Fill)
                    )
                    .width(Length::Fill)
                    .max_width(280),
                ]
                .spacing(8)
                .align_y(Alignment::Center);
                for c in preview {
                    let c = color(c, 1.0);
                    r = r.push(
                        container(Space::new().width(16).height(16)).style(move |_| {
                            container::Style {
                                background: Some(c.into()),
                                border: iced::Border::default().rounded(4).width(1).color(Color {
                                    a: 0.3,
                                    ..Color::WHITE
                                }),
                                ..Default::default()
                            }
                        }),
                    );
                }
                r.into()
            }
        }
    }

    fn opt_pick(
        &self,
        store: Store,
        path: String,
        choices: Vec<Choice>,
        now: Option<OptValue>,
    ) -> Element<'_, UiMsg> {
        let sel = choices
            .iter()
            .find(|c| Some(&c.value) == now.as_ref())
            .cloned();
        pick_list(choices, sel, move |c: Choice| {
            wrap(Msg::Set(store, path.clone(), Some(c.value)))
        })
        .font(self.font)
        .width(280)
        .into()
    }

    /// Relit hyprland.lua (hors du fil de l'interface).
    pub(in crate::ui) fn load_hypr_source(&self) -> Task<UiMsg> {
        let path = self.config_path.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || crate::hyprsrc::read(&path))
                    .await
                    .unwrap_or_default()
            },
            |s| wrap(Msg::Source(Arc::new(s))),
        )
    }

    /// Relit les valeurs en vigueur des options `hypr`.
    pub(in crate::ui) fn load_hypr_current(&self) -> Task<UiMsg> {
        Task::perform(
            async {
                tokio::task::spawn_blocking(crate::hyprset::current)
                    .await
                    .unwrap_or_default()
            },
            |m| wrap(Msg::Current(m)),
        )
    }

    /// Écrit les fichiers de Hyprland générés par hyprdesk pour `cfg` (hors
    /// du fil de l'interface) ; vérifiés par Hyprland, rien si à jour.
    pub(in crate::ui) fn sync_genfiles(&self, cfg: &crate::config::Config) -> Task<UiMsg> {
        if self.preview {
            return Task::none();
        }
        let path = self.config_path.clone();
        let cfg = cfg.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let _reason = crate::history::reason("reglages");
                    crate::genfiles::sync(&path, &cfg)
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
            },
            |r| {
                if let Err(e) = r {
                    tracing::warn!("{}", crate::tr!("fichiers de Hyprland : {e}", e = e));
                    crate::hypr::notify(0, 10000, &crate::tr!("hyprdesk : {e}", e = e));
                }
                UiMsg::Noop
            },
        )
    }

    /// Pose les options `hypr` dans Hyprland (hors du fil de l'interface).
    pub(in crate::ui) fn sync_hypr(&self, removed: bool) -> Task<UiMsg> {
        let h = self.cfg.general.hypr.clone();
        Task::perform(
            async move {
                let r =
                    tokio::task::spawn_blocking(move || crate::hyprset::apply(&h, removed)).await;
                if let Ok(Err(e)) = r {
                    tracing::warn!("{}", crate::tr!("options de Hyprland : {e}", e = e));
                }
            },
            |_| UiMsg::Noop,
        )
    }

    pub(in crate::ui) fn hyprset_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::Set(store, path, v) => {
                self.opt_drafts.remove(&path);
                self.opt_errors.remove(&path);
                self.edit_store(store, |m| match v {
                    Some(v) => {
                        m.insert(path, v);
                    }
                    None => {
                        m.remove(&path);
                    }
                });
                self.commit()
            }
            Msg::Drag(store, path, x) => {
                let x = (x * 100.0).round() / 100.0;
                self.edit_store(store, |m| {
                    m.insert(path, OptValue::Num(x));
                });
                self.apply()
            }
            Msg::Commit => self.commit(),
            Msg::Draft(path, s) => {
                self.opt_errors.remove(&path);
                self.opt_drafts.insert(path, s);
                Task::none()
            }
            Msg::Submit(store, path) => {
                let Some(o) = registry(store, &path) else {
                    return Task::none();
                };
                let draft = self.opt_drafts.get(&path).cloned().unwrap_or_default();
                if draft.trim().is_empty() {
                    return self.hyprset_update(Msg::Set(store, path, None));
                }
                match o.parse(&draft) {
                    Ok(v) => self.hyprset_update(Msg::Set(store, path, Some(v))),
                    Err(e) => {
                        self.opt_errors.insert(path, e);
                        Task::none()
                    }
                }
            }
            Msg::Current(m) => {
                self.hypr_current = m;
                Task::none()
            }
            Msg::Source(s) => {
                self.hypr_source = Some(s);
                Task::none()
            }
            other => self.hyprset_edit(other),
        }
    }
}

/// Thèmes de curseur installés (dossiers `cursors/` ou `manifest.hl`).
pub fn cursor_themes() -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let dirs = [
        format!("{home}/.icons"),
        format!("{home}/.local/share/icons"),
        "/usr/share/icons".to_string(),
    ];
    let mut out: Vec<String> = Vec::new();
    for d in dirs {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            if (p.join("cursors").is_dir() || p.join("manifest.hl").is_file())
                && let Some(n) = p.file_name().map(|n| n.to_string_lossy().to_string())
                && !out.contains(&n)
            {
                out.push(n);
            }
        }
    }
    out.sort();
    out
}

impl App {
    fn small(&self, s: String) -> Element<'_, UiMsg> {
        text(s)
            .font(self.font)
            .size(12)
            .color(self.ink().dim)
            .into()
    }

    /// Sections du registre `hyprset` rangées sous `key` (`fenetres`,
    /// `groupes`, `curseur`, `gestes`, `comportement-fenetres`, `avance`,
    /// `permissions`). Options brutes de Hyprland : affichées seulement en
    /// configuration avancée, encadrées en cyan, sauf sur une page avancée
    /// ou pour un groupe de base (`hyprset::BASIC_GROUPS`).
    pub(in crate::ui) fn hyprset_tab(&self, key: &str) -> Element<'_, UiMsg> {
        let on_advanced_page = self
            .panel
            .as_ref()
            .is_some_and(|p| super::plugins::is_advanced(p.tab, Some(self.subtab(p.tab))));
        if crate::hyprset::BASIC_GROUPS.contains(&key) || on_advanced_page {
            self.hyprset_groups(key)
        } else {
            self.adv(self.hyprset_groups(key))
        }
    }

    /// Sections du registre rangées sous `key`, sans cadre.
    pub(in crate::ui) fn hyprset_groups(&self, key: &str) -> Element<'_, UiMsg> {
        let mut col = column![].spacing(14);
        for (k, title, paths) in crate::hyprset::GROUPS {
            if *k != key {
                continue;
            }
            col = col.push(self.section(title));
            for p in *paths {
                col = col.push(self.opt_field(Store::Hypr, p));
            }
        }
        col.into()
    }

    fn hypr_cfg(&self) -> &crate::hyprset::Hypr {
        &self.cfg.general.hypr
    }

    pub(in crate::ui) fn edit_hypr(&mut self, f: impl FnOnce(&mut crate::hyprset::Hypr)) {
        let mut h = self
            .settings
            .hypr
            .clone()
            .unwrap_or_else(|| self.cfg.general.hypr.clone());
        f(&mut h);
        self.settings.hypr = Some(h);
    }

    /// Interrupteur « Bordures aux couleurs du thème ».
    pub(in crate::ui) fn theme_borders_field(&self) -> Element<'_, UiMsg> {
        column![
            self.field(
                "Bordures aux couleurs du thème",
                "",
                toggler(self.hypr_cfg().theme_borders).on_toggle(|v| wrap(Msg::ThemeBorders(v))),
            ),
            self.small(
                crate::i18n::tr(
                    "Accent du thème pour la fenêtre active, fond pour les autres ; suit les \
                     changements de thème et l'emporte sur les couleurs réglées plus bas.",
                )
                .to_string()
            ),
        ]
        .spacing(4)
        .into()
    }

    /// Éditeur des variables d'environnement (`hl.env`).
    pub(in crate::ui) fn env_editor(&self) -> Element<'_, UiMsg> {
        let mut col = column![
            self.section("Variables d'environnement"),
            self.small(
                crate::i18n::tr(
                    "Posées par Hyprland au démarrage de la session (env.lua), visibles des \
                     applications qu'il lance. Les valeurs calculées de hyprland.lua (PATH…) y \
                     restent.",
                )
                .to_string()
            ),
        ]
        .spacing(10);
        for (i, (k, v)) in self.hypr_cfg().env.iter().enumerate() {
            let (kk, vv) = (k.clone(), v.clone());
            col = col.push(
                row![
                    text_input("NOM", k)
                        .on_input(move |s| wrap(Msg::Env(i, Some((s, vv.clone())))))
                        .on_submit(wrap(Msg::Commit))
                        .font(self.font)
                        .size(13)
                        .width(260),
                    text_input(crate::i18n::tr("valeur"), v)
                        .on_input(move |s| wrap(Msg::Env(i, Some((kk.clone(), s)))))
                        .on_submit(wrap(Msg::Commit))
                        .font(self.font)
                        .size(13)
                        .width(Length::Fill),
                    button(text("×").font(self.font).size(14))
                        .padding([2, 10])
                        .style(button::text)
                        .on_press(wrap(Msg::Env(i, None))),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        col = col.push(
            button(
                text(crate::i18n::tr("Ajouter une variable"))
                    .font(self.font)
                    .size(13),
            )
            .padding([4, 12])
            .on_press(wrap(Msg::EnvAdd)),
        );
        if let Some(s) = &self.hypr_source {
            let computed: Vec<String> = s
                .calls
                .iter()
                .filter(|c| c.func == "env" && c.computed)
                .filter_map(|c| c.args.first().and_then(|a| a.as_str()).map(str::to_string))
                .collect();
            if !computed.is_empty() {
                col = col.push(self.small(crate::tr!(
                    "Calculées dans hyprland.lua : {list}",
                    list = computed.join(", ")
                )));
            }
        }
        col.into()
    }

    /// Thème et taille du curseur (variables XCURSOR/HYPRCURSOR d'env.lua).
    pub(in crate::ui) fn cursor_theme_editor(&self) -> Element<'_, UiMsg> {
        let env = &self.hypr_cfg().env;
        let get = |k: &str| env.iter().find(|(n, _)| n == k).map(|(_, v)| v.clone());
        let theme = get("XCURSOR_THEME").or_else(|| std::env::var("XCURSOR_THEME").ok());
        let size: f64 = get("XCURSOR_SIZE")
            .or_else(|| std::env::var("XCURSOR_SIZE").ok())
            .and_then(|s| s.parse().ok())
            .unwrap_or(24.0);
        let themes = cursor_themes();
        column![
            self.section("Thème du curseur"),
            self.field(
                "Thème",
                "",
                pick_list(themes, theme, |t| wrap(Msg::CursorTheme(t)))
                    .font(self.font)
                    .width(320),
            ),
            self.field(
                "Taille",
                "",
                row![
                    slider(16.0f32..=96.0, size as f32, |x| wrap(Msg::CursorSize(
                        x as f64
                    )))
                    .step(2.0_f32)
                    .on_release(wrap(Msg::CursorCommit))
                    .width(240),
                    text(format!("{size:.0}")).font(self.font).size(13),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            ),
            self.small(
                crate::i18n::tr(
                    "Appliqué aussitôt (hyprctl setcursor) et gardé dans env.lua pour les \
                     applications ; un thème hyprcursor du même nom est pris s'il existe.",
                )
                .to_string()
            ),
        ]
        .spacing(10)
        .into()
    }

    /// Messages des éditeurs (variables, listes, bordures, curseur).
    fn hyprset_edit(&mut self, msg: Msg) -> Task<UiMsg> {
        let msg = match self.hyprlists_update(msg) {
            Ok(task) => return task,
            Err(msg) => msg,
        };
        match msg {
            Msg::Env(i, v) => {
                // Saisie : appliquée en mémoire, enregistrée à Entrée ;
                // retrait : enregistré tout de suite.
                let removed = v.is_none();
                self.edit_hypr(|h| match v {
                    Some(kv) if i < h.env.len() => h.env[i] = kv,
                    None if i < h.env.len() => {
                        h.env.remove(i);
                    }
                    _ => {}
                });
                if removed { self.commit() } else { self.apply() }
            }
            Msg::EnvAdd => {
                self.edit_hypr(|h| h.env.push(("NOM".into(), String::new())));
                self.commit()
            }
            Msg::ThemeBorders(v) => {
                self.edit_hypr(|h| h.theme_borders = v);
                self.commit()
            }
            Msg::CursorTheme(t) => {
                let hypr_theme = format!("{t}-Hyprcursor");
                let has_hc = cursor_themes().contains(&hypr_theme);
                self.edit_hypr(|h| {
                    set_env(h, "XCURSOR_THEME", &t);
                    if has_hc {
                        set_env(h, "HYPRCURSOR_THEME", &hypr_theme);
                    }
                });
                let size = self.cursor_size();
                self.set_cursor_now(t, size);
                self.commit()
            }
            Msg::CursorSize(x) => {
                let s = (x.round() as i64).to_string();
                self.edit_hypr(|h| {
                    set_env(h, "XCURSOR_SIZE", &s);
                    set_env(h, "HYPRCURSOR_SIZE", &s);
                });
                self.apply()
            }
            Msg::CursorCommit => {
                let theme = self
                    .settings
                    .hypr
                    .as_ref()
                    .and_then(|h| h.env.iter().find(|(k, _)| k == "XCURSOR_THEME"))
                    .map(|(_, v)| v.clone())
                    .unwrap_or_default();
                let size = self.cursor_size();
                if !theme.is_empty() {
                    self.set_cursor_now(theme, size);
                }
                self.commit()
            }
            _ => Task::none(),
        }
    }

    fn cursor_size(&self) -> u32 {
        self.settings
            .hypr
            .as_ref()
            .unwrap_or(&self.cfg.general.hypr)
            .env
            .iter()
            .find(|(k, _)| k == "XCURSOR_SIZE")
            .and_then(|(_, v)| v.parse().ok())
            .unwrap_or(24)
    }

    /// `hyprctl setcursor` hors du fil de l'interface.
    fn set_cursor_now(&self, theme: String, size: u32) {
        if self.preview {
            return;
        }
        std::thread::spawn(move || {
            let _ = std::process::Command::new("hyprctl")
                .args(["setcursor", &theme, &size.to_string()])
                .output();
        });
    }
}

fn set_env(h: &mut crate::hyprset::Hypr, k: &str, v: &str) {
    match h.env.iter_mut().find(|(n, _)| n == k) {
        Some(e) => e.1 = v.to_string(),
        None => h.env.push((k.to_string(), v.to_string())),
    }
}
