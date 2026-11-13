//! Paquet intégré « Couleurs des écrans » : luminosité, contraste, gamma,
//! gain et gamma de chaque canal (courbe de couleurs), fichier de calibration
//! (.cal, .icc, .icm), par écran ; image de référence pour juger, et mire
//! plein écran étalée sur tous les écrans actifs, avec les réglages de chacun
//! à portée de main pour comparer leur rendu.
//!
//! Les tables gamma sont tenues par `crate::colors::Gamma` (fil du démon).
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use std::collections::BTreeMap;

use crate::colors::{Adjust, FIELDS};
use crate::i18n::text;
use iced::widget::{
    Space, button, column, container, image, pick_list, row, slider, stack, text_input,
};
use iced::{Alignment, Border, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "colors",
    label: "Couleurs des écrans",
    icon: "preferences-desktop-color",
    summary: "Luminosité, contraste, gamma et courbes de chaque écran, fichier de calibration, mire de comparaison.",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Colors(m)))
}

#[derive(Debug, Clone, Default)]
pub struct State {
    /// Écran réglé dans la page.
    pub selected: Option<String>,
    pub reference: Option<image::Handle>,
    pub curves: Option<image::Handle>,
    pub cal_input: String,
    /// Fichiers de calibration trouvés sur le poste.
    pub profiles: Vec<String>,
    /// Problèmes des tables gamma : écran → message (clé vide : le service).
    pub status: std::collections::HashMap<String, String>,
    /// Programmes qui empêchent ou faussent les réglages (killswitch).
    pub blockers: Vec<crate::colors::Blocker>,
    /// Arrêt en cours.
    pub stopping: bool,
    pub info: Option<String>,
    pub error: Option<String>,
}

/// Mire plein écran : une surface par écran actif.
#[derive(Debug, Clone, Default)]
pub struct ColorTest {
    /// Surface, écran, part de la mire.
    pub wins: Vec<(iced::window::Id, String, image::Handle)>,
    /// Écrans dont le bloc de réglages est replié.
    pub folded: std::collections::HashSet<String>,
    /// Dégradés étalés sur tous les écrans au lieu de la mire sur chacun.
    pub span: bool,
    /// La mire (une seule image, la même sur chaque écran).
    pub mire: Option<image::Handle>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Loaded(Vec<String>),
    Select(String),
    /// Curseur déplacé : appliqué aussitôt, gardé au relâcher.
    Drag(String, f32),
    Release,
    ResetField(String),
    ResetScreen,
    CalInput(String),
    CalImport,
    CalPick(String),
    CalRemove,
    Imported(Result<String, String>),
    Status,
    Blockers(Vec<crate::colors::Blocker>),
    /// Killswitch : arrêter les programmes gênants ; `true` : ne plus les lancer.
    Stop(bool),
    Stopped(Result<String, String>),
    TestOpen,
    TestClose,
    /// Bouton − / + de la mire : écran, réglage, pas.
    Step(String, String, f32),
    TestNeutral(String),
    TestFold(String),
    /// Mire sur chaque écran ⇄ dégradés étalés sur l'ensemble.
    TestSpan,
}

/// Fichiers de calibration du poste (hors du fil de l'interface).
pub fn load() -> Task<UiMsg> {
    Task::batch([
        Task::perform(
            async {
                tokio::task::spawn_blocking(crate::colors::installed_profiles)
                    .await
                    .unwrap_or_default()
            },
            |l| wrap(Msg::Loaded(l)),
        ),
        Task::done(wrap(Msg::Status)),
    ])
}

/// Pas des boutons − / + de la mire.
fn step_of(key: &str) -> f32 {
    if key.starts_with("gamma") { 0.05 } else { 0.02 }
}

fn handle((w, h, px): crate::colors::Rgba) -> image::Handle {
    image::Handle::from_rgba(w, h, px)
}

impl App {
    /// Réglages voulus de tous les écrans (Réglages, sinon configuration).
    fn colors_map(&self) -> BTreeMap<String, Adjust> {
        self.settings
            .screen_colors
            .clone()
            .unwrap_or_else(|| self.cfg.general.screen_colors.clone())
    }

    fn colors_of(&self, monitor: &str) -> Adjust {
        self.colors_map().get(monitor).cloned().unwrap_or_default()
    }

    /// Écran réglé : celui choisi, sinon celui qui a le focus.
    fn colors_selected(&self) -> Option<String> {
        self.panel
            .as_ref()
            .and_then(|p| p.colors.selected.clone())
            .filter(|s| self.monitors.iter().any(|m| &m.name == s))
            .or_else(|| {
                self.monitors
                    .iter()
                    .find(|m| m.focused)
                    .map(|m| m.name.clone())
            })
            .or_else(|| self.monitors.first().map(|m| m.name.clone()))
    }

    /// Change les réglages d'un écran ; `save` : les garder (settings.lua).
    fn colors_edit(
        &mut self,
        monitor: &str,
        save: bool,
        f: impl FnOnce(&mut Adjust),
    ) -> Task<UiMsg> {
        let mut map = self.colors_map();
        let a = map.entry(monitor.to_string()).or_default();
        f(a);
        if a.is_default() {
            map.remove(monitor);
        }
        self.settings.screen_colors = Some(map.clone());
        self.colors_curves();
        let status = Task::perform(
            async { tokio::time::sleep(std::time::Duration::from_millis(400)).await },
            |_| wrap(Msg::Status),
        );
        if save {
            Task::batch([self.commit(), status])
        } else {
            // Aperçu pendant le glissé : la table seule, sans recharger la configuration.
            if self.cfg.plugin_on(INFO.id) {
                self.gamma.set(map);
            }
            status
        }
    }

    /// Courbes de l'écran choisi, redessinées.
    fn colors_curves(&mut self) {
        let Some(m) = self.colors_selected() else {
            return;
        };
        let a = self.colors_of(&m);
        let cal = crate::colors::load_cal(&a).ok().flatten();
        let img = handle(crate::colors::curves_image(&a, cal.as_ref(), 260, 170));
        if let Some(p) = self.panel.as_mut() {
            p.colors.curves = Some(img);
        }
    }

    pub(in crate::ui) fn colors_page(&self) -> Element<'_, UiMsg> {
        let ink = self.ink();
        let (fg, dim, accent) = (ink.fg, ink.dim, ink.accent);
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let Some(p) = self.panel.as_ref() else {
            return Space::new().into();
        };
        let s = &p.colors;
        let Some(mon) = self.colors_selected() else {
            return text("Aucun écran détecté.")
                .font(self.font)
                .size(13)
                .color(dim)
                .into();
        };
        let a = self.colors_of(&mon);
        let mut col = column![
            text("Réglages appliqués aussitôt à l'écran choisi, et gardés. Un fichier .cal (ArgyllCMS) ajoute ses courbes à celles-ci ; un profil .icc ou .icm est confié à Hyprland.")
                .font(self.font)
                .size(12)
                .color(dim),
        ]
        .spacing(12);
        // Choix de l'écran.
        let mut screens = row![].spacing(8).align_y(Alignment::Center);
        for m in &self.monitors {
            let on = m.name == mon;
            let tuned = self
                .colors_map()
                .get(&m.name)
                .is_some_and(|x| !x.is_default());
            let label = if tuned {
                format!("{} ●", m.name)
            } else {
                m.name.clone()
            };
            screens = screens.push(
                button(iced::widget::text(label).font(self.font).size(13))
                    .padding([6, 12])
                    .style(if on {
                        button::primary
                    } else {
                        button::secondary
                    })
                    .on_press(wrap(Msg::Select(m.name.clone()))),
            );
        }
        col = col.push(screens);
        if let Some(e) = s.status.get("") {
            col = col.push(
                iced::widget::text(e.clone())
                    .font(self.font)
                    .size(12)
                    .color(warn),
            );
        }
        if let Some(e) = s.status.get(&mon) {
            col = col.push(
                row![
                    iced::widget::text(crate::tr!("{m} : {e}", m = mon, e = e))
                        .font(self.font)
                        .size(12)
                        .color(warn)
                        .width(Length::Fill),
                    self.lyra_button(
                        crate::i18n::tr("Couleurs des écrans"),
                        crate::tr!(
                            "hyprdesk ne peut pas régler les couleurs de l'écran {m} : {e}. Lance « hyprdesk colors diag », trouve pourquoi Hyprland refuse sa table gamma (programme qui la tient, profil ICC, pilote sans table) et propose une solution.",
                            m = mon,
                            e = e
                        ),
                    ),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        if !s.blockers.is_empty() {
            col = col.push(self.colors_blockers(s));
        }
        // Curseurs et courbes.
        let mut sliders = column![].spacing(6).width(Length::Fill);
        for (key, label, lo, hi, neutral) in FIELDS {
            let v = a.get(key).unwrap_or(*neutral);
            let k = key.to_string();
            let k2 = key.to_string();
            sliders = sliders.push(
                row![
                    container(self.marked(crate::i18n::tr(label), 13, fg, self.font)).width(110),
                    slider(*lo..=*hi, v, move |x| wrap(Msg::Drag(k.clone(), x)))
                        .step(0.01f32)
                        .on_release(wrap(Msg::Release))
                        .width(Length::Fill),
                    iced::widget::text(format!("{v:.2}"))
                        .font(self.font)
                        .size(12)
                        .color(dim)
                        .width(40),
                    button(iced::widget::text("↺").size(12))
                        .padding([2, 8])
                        .style(button::text)
                        .on_press_maybe(
                            ((v - neutral).abs() >= 1e-3).then(|| wrap(Msg::ResetField(k2)))
                        ),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        let curves: Element<'_, UiMsg> = match &s.curves {
            Some(h) => column![
                image(h.clone()).width(260).height(170),
                text("Courbes rouge, verte et bleue (entrée → sortie) ; la diagonale grise est neutre.")
                    .font(self.font)
                    .size(11)
                    .color(dim)
                    .width(260),
            ]
            .spacing(4)
            .into(),
            None => Space::new().into(),
        };
        col = col.push(sliders);
        col = col.push(curves);
        col = col.push(
            row![
                button(text("Rétablir cet écran").font(self.font).size(13))
                    .padding([6, 12])
                    .style(button::secondary)
                    .on_press_maybe((!a.is_default()).then(|| wrap(Msg::ResetScreen))),
                button(
                    iced::widget::text(if self.monitors.len() > 1 {
                        crate::tr!(
                            "Mire plein écran sur les {n} écrans",
                            n = self.monitors.len()
                        )
                    } else {
                        crate::i18n::tr("Mire plein écran").to_string()
                    })
                    .font(self.font)
                    .size(13)
                )
                .padding([6, 12])
                .on_press(wrap(Msg::TestOpen)),
            ]
            .spacing(10),
        );
        // Fichier de calibration.
        col = col.push(self.section(crate::i18n::tr("Fichier de calibration")));
        let current = if a.calibration.trim().is_empty() {
            crate::i18n::tr("aucun").to_string()
        } else {
            a.calibration.clone()
        };
        col = col.push(
            row![
                iced::widget::text(crate::tr!("En place : {f}", f = current))
                    .font(self.font)
                    .size(13)
                    .color(fg)
                    .width(Length::Fill),
                button(text("Retirer").font(self.font).size(12))
                    .padding([4, 10])
                    .style(button::secondary)
                    .on_press_maybe(
                        (!a.calibration.trim().is_empty()).then(|| wrap(Msg::CalRemove))
                    ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
        col = col.push(
            row![
                text_input(crate::i18n::tr("~/Téléchargements/ecran.icc"), &s.cal_input)
                    .on_input(|v| wrap(Msg::CalInput(v)))
                    .on_submit(wrap(Msg::CalImport))
                    .padding(6)
                    .size(13),
                button(text("Importer").font(self.font).size(13))
                    .padding([6, 12])
                    .on_press_maybe((!s.cal_input.trim().is_empty()).then(|| wrap(Msg::CalImport))),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
        if !s.profiles.is_empty() {
            col = col.push(
                row![
                    text("Profils du poste").font(self.font).size(13).color(fg),
                    pick_list(s.profiles.clone(), None::<String>, |v| wrap(Msg::CalPick(
                        v
                    )))
                    .placeholder(crate::i18n::tr("choisir…"))
                    .text_size(13),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }
        col = col.push(
            text("Importer copie le fichier dans ~/.config/hypr/hyprdesk/calibration. .cal : courbes d'ArgyllCMS (dispcal), appliquées par hyprdesk ; .icc / .icm : profil appliqué par Hyprland (monitors.lua).")
                .font(self.font)
                .size(11)
                .color(dim),
        );
        if let Some(m) = &s.info {
            col = col.push(
                iced::widget::text(m.clone())
                    .font(self.font)
                    .size(12)
                    .color(accent),
            );
        }
        if let Some(e) = &s.error {
            col = col.push(
                iced::widget::text(e.clone())
                    .font(self.font)
                    .size(12)
                    .color(warn),
            );
        }
        // Image de référence.
        col = col.push(self.section(crate::i18n::tr("Image de référence")));
        if let Some(h) = &s.reference {
            col = col.push(image(h.clone()).width(Length::Fill));
        }
        col = col.push(
            text("Les cases des ombres (1 à 32) et des hautes lumières (223 à 254) doivent toutes se distinguer de leur fond ; le dégradé de gris doit rester neutre, sans dominante ni marches ; les peaux doivent paraître naturelles. « Mire plein écran » l'affiche sur chaque écran, avec ses réglages.")
                .font(self.font)
                .size(11)
                .color(dim),
        );
        col.into()
    }

    /// Killswitch : programmes qui tiennent les tables gamma (wlsunset,
    /// gammastep…), et de quoi les arrêter pour rendre la main aux réglages.
    fn colors_blockers<'a>(&'a self, s: &'a State) -> Element<'a, UiMsg> {
        let ink = self.ink();
        let (fg, dim) = (ink.fg, ink.dim);
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let mut list = column![
            text("Programmes qui gênent les réglages de couleur")
                .font(self.font)
                .size(13)
                .color(warn),
        ]
        .spacing(4);
        for b in &s.blockers {
            let what =
                crate::i18n::tr("tient les tables gamma : les réglages d'ici ne passent pas");
            let mut from = Vec::new();
            if let Some(u) = &b.unit {
                from.push(crate::tr!("service {u}", u = u));
            }
            if let Some(a) = &b.autostart {
                from.push(crate::tr!("démarrage automatique « {a} »", a = a));
            }
            let mut line = format!("{} ({}) — {what}", b.name, b.pid);
            if !from.is_empty() {
                line += &format!(" ({})", from.join(", "));
            }
            list = list.push(iced::widget::text(line).font(self.font).size(12).color(fg));
        }
        let busy = s.stopping;
        let mut buttons = row![
            button(
                iced::widget::text(if busy {
                    crate::i18n::tr("Arrêt…").to_string()
                } else if s.blockers.len() > 1 {
                    crate::tr!("Arrêter les {n}", n = s.blockers.len())
                } else {
                    crate::tr!("Arrêter {p}", p = s.blockers[0].name)
                })
                .font(self.font)
                .size(13)
            )
            .padding([6, 12])
            .style(button::danger)
            .on_press_maybe((!busy).then(|| wrap(Msg::Stop(false)))),
        ]
        .spacing(10)
        .align_y(Alignment::Center);
        if s.blockers.iter().any(|b| b.relaunched()) {
            buttons = buttons.push(
                button(text("Arrêter et ne plus lancer").font(self.font).size(13))
                    .padding([6, 12])
                    .style(button::secondary)
                    .on_press_maybe((!busy).then(|| wrap(Msg::Stop(true)))),
            );
        }
        list = list.push(buttons);
        list = list.push(
            text("Arrêter : son service s'il en a un, sinon le programme lui-même ; les réglages d'ici reprennent la main aussitôt. Sans « ne plus lancer », il revient à la session suivante.")
                .font(self.font)
                .size(11)
                .color(dim),
        );
        container(list)
            .padding(10)
            .width(Length::Fill)
            .style(move |_| container::Style {
                border: Border {
                    color: Color { a: 0.6, ..warn },
                    width: 1.0,
                    radius: 8.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    pub(in crate::ui) fn colors_update(&mut self, m: Msg) -> Task<UiMsg> {
        match m {
            Msg::Loaded(l) => {
                if let Some(p) = self.panel.as_mut() {
                    p.colors.profiles = l;
                    if p.colors.reference.is_none() {
                        p.colors.reference = Some(image::Handle::from_bytes(crate::colors::MIRE));
                    }
                }
                self.colors_curves();
                Task::none()
            }
            Msg::Select(name) => {
                if let Some(p) = self.panel.as_mut() {
                    p.colors.selected = Some(name);
                    p.colors.info = None;
                    p.colors.error = None;
                }
                self.colors_curves();
                Task::none()
            }
            Msg::Drag(key, v) => match self.colors_selected() {
                Some(mon) => self.colors_edit(&mon, false, |a| {
                    a.set(&key, v);
                }),
                None => Task::none(),
            },
            Msg::Release => self.commit(),
            Msg::ResetField(key) => match self.colors_selected() {
                Some(mon) => self.colors_edit(&mon, true, |a| {
                    if let Some(&(_, _, _, _, n)) = FIELDS.iter().find(|f| f.0 == key) {
                        a.set(&key, n);
                    }
                }),
                None => Task::none(),
            },
            Msg::ResetScreen => match self.colors_selected() {
                Some(mon) => self.colors_edit(&mon, true, |a| *a = Adjust::default()),
                None => Task::none(),
            },
            Msg::CalInput(v) => {
                if let Some(p) = self.panel.as_mut() {
                    p.colors.cal_input = v;
                }
                Task::none()
            }
            Msg::CalImport => {
                let file = self
                    .panel
                    .as_ref()
                    .map(|p| p.colors.cal_input.clone())
                    .unwrap_or_default();
                let path = self.config_path.clone();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || crate::colors::import(&path, &file))
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| wrap(Msg::Imported(r)),
                )
            }
            Msg::CalPick(f) => self.colors_update(Msg::Imported(Ok(f))),
            Msg::Imported(r) => match (r, self.colors_selected()) {
                (Ok(f), Some(mon)) => {
                    if let Some(p) = self.panel.as_mut() {
                        p.colors.cal_input.clear();
                        p.colors.error = None;
                        p.colors.info = Some(crate::tr!("{m} : calibration {f}", m = mon, f = f));
                    }
                    self.colors_edit(&mon, true, |a| a.calibration = f)
                }
                (Err(e), _) => {
                    if let Some(p) = self.panel.as_mut() {
                        p.colors.info = None;
                        p.colors.error = Some(e);
                    }
                    Task::none()
                }
                _ => Task::none(),
            },
            Msg::CalRemove => match self.colors_selected() {
                Some(mon) => self.colors_edit(&mon, true, |a| a.calibration.clear()),
                None => Task::none(),
            },
            Msg::Status => {
                let st = self.gamma.status();
                if let Some(p) = self.panel.as_mut() {
                    p.colors.status = st;
                }
                Task::perform(
                    async {
                        tokio::task::spawn_blocking(crate::colors::blockers)
                            .await
                            .unwrap_or_default()
                    },
                    |b| wrap(Msg::Blockers(b)),
                )
            }
            Msg::Blockers(b) => {
                if let Some(p) = self.panel.as_mut() {
                    p.colors.blockers = b;
                }
                Task::none()
            }
            Msg::Stop(disable) => {
                let Some(p) = self.panel.as_mut() else {
                    return Task::none();
                };
                if p.colors.stopping || p.colors.blockers.is_empty() {
                    return Task::none();
                }
                p.colors.stopping = true;
                let list = p.colors.blockers.clone();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::colors::stop_blockers(&list, disable)
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| wrap(Msg::Stopped(r)),
                )
            }
            Msg::Stopped(r) => {
                if let Some(p) = self.panel.as_mut() {
                    p.colors.stopping = false;
                    match r {
                        Ok(m) => {
                            p.colors.info = Some(m);
                            p.colors.error = None;
                        }
                        Err(e) => {
                            p.colors.info = None;
                            p.colors.error = Some(e);
                        }
                    }
                }
                // Tables libérées : les réglages sont redemandés tout de suite
                // (sans attendre la revue des 5 s du fil des couleurs).
                if self.cfg.plugin_on(INFO.id) {
                    self.gamma.set(self.colors_map());
                }
                Task::perform(
                    async { tokio::time::sleep(std::time::Duration::from_millis(800)).await },
                    |_| wrap(Msg::Status),
                )
            }
            Msg::TestOpen => self.color_test_open(),
            Msg::TestClose => Task::batch(
                std::mem::take(&mut self.color_test.wins)
                    .into_iter()
                    .map(|(id, _, _)| iced::window::close(id)),
            ),
            Msg::TestSpan => {
                self.color_test.span = !self.color_test.span;
                Task::none()
            }
            Msg::Step(mon, key, d) => self.colors_edit(&mon, true, |a| {
                let v = a.get(&key).unwrap_or(1.0);
                a.set(&key, ((v + d) * 100.0).round() / 100.0);
            }),
            Msg::TestNeutral(mon) => self.colors_edit(&mon, true, |a| {
                let cal = std::mem::take(&mut a.calibration);
                *a = Adjust {
                    calibration: cal,
                    ..Adjust::default()
                };
            }),
            Msg::TestFold(mon) => {
                if !self.color_test.folded.remove(&mon) {
                    self.color_test.folded.insert(mon);
                }
                Task::none()
            }
        }
    }

    /// Ouvre la mire sur chaque écran (ou les dégradés étalés sur l'ensemble).
    fn color_test_open(&mut self) -> Task<UiMsg> {
        use iced_layershell::reexport::{
            Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
        };
        let mut tasks: Vec<Task<UiMsg>> = std::mem::take(&mut self.color_test.wins)
            .into_iter()
            .map(|(id, _, _)| iced::window::close(id))
            .collect();
        let ms = &self.monitors;
        if ms.is_empty() {
            return Task::batch(tasks);
        }
        let x0 = ms.iter().map(|m| m.x).fold(f32::MAX, f32::min);
        let y0 = ms.iter().map(|m| m.y).fold(f32::MAX, f32::min);
        let x1 = ms
            .iter()
            .map(|m| m.x + m.width as f32)
            .fold(f32::MIN, f32::max);
        let y1 = ms
            .iter()
            .map(|m| m.y + m.height as f32)
            .fold(f32::MIN, f32::max);
        let all = (x0, y0, x1 - x0, y1 - y0);
        if self.color_test.mire.is_none() {
            self.color_test.mire = Some(image::Handle::from_bytes(crate::colors::MIRE));
        }
        for m in ms {
            let id = iced::window::Id::unique();
            // Image réduite (étirée à l'écran) : la mire n'a pas besoin du pixel près.
            let w = 640u32;
            let h = ((w as f32 * m.height as f32 / m.width.max(1) as f32).round() as u32).max(1);
            let img = handle(crate::colors::span_image(
                all,
                (m.x, m.y, m.width as f32, m.height as f32),
                w,
                h,
            ));
            self.color_test.wins.push((id, m.name.clone(), img));
            tasks.push(Task::done(UiMsg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some((0, 0)),
                    layer: Layer::Overlay,
                    anchor: Anchor::Top | Anchor::Bottom | Anchor::Left | Anchor::Right,
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    output_option: OutputOption::OutputName(m.name.clone()),
                    namespace: Some("hyprdesk-mire".into()),
                    ..Default::default()
                },
                id,
            }));
        }
        Task::batch(tasks)
    }

    /// Surface de la mire, si `id` en est une.
    pub(in crate::ui) fn color_test_of(
        &self,
        id: iced::window::Id,
    ) -> Option<(&str, &image::Handle)> {
        self.color_test
            .wins
            .iter()
            .find(|(i, _, _)| *i == id)
            .map(|(_, n, h)| (n.as_str(), h))
    }

    /// Mire sur un écran, avec les réglages de cet écran en bas.
    pub(in crate::ui) fn view_color_test<'a>(
        &'a self,
        mon: &'a str,
        img: &'a image::Handle,
    ) -> Element<'a, UiMsg> {
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let bg = color(th.bg, 0.9);
        let a = self.colors_of(mon);
        let folded = self.color_test.folded.contains(mon);
        let small = |s: String| iced::widget::text(s).font(self.font).size(13).color(fg);
        let mut head = row![
            iced::widget::text(mon.to_string())
                .font(self.title_font())
                .size(18)
                .color(accent),
            Space::new().width(Length::Fill),
            button(small(
                crate::i18n::tr(if self.color_test.span {
                    "Mire"
                } else {
                    "Dégradés étalés"
                })
                .to_string()
            ))
            .padding([4, 10])
            .style(button::secondary)
            .on_press(wrap(Msg::TestSpan)),
            button(small(
                crate::i18n::tr(if folded { "Réglages" } else { "Replier" }).to_string()
            ))
            .padding([4, 10])
            .style(button::secondary)
            .on_press(wrap(Msg::TestFold(mon.to_string()))),
            button(small(crate::i18n::tr("Fermer la mire").to_string()))
                .padding([4, 10])
                .on_press(wrap(Msg::TestClose)),
        ]
        .spacing(8)
        .align_y(Alignment::Center);
        if !folded {
            head = head.push(
                button(small(crate::i18n::tr("Neutre").to_string()))
                    .padding([4, 10])
                    .style(button::secondary)
                    .on_press_maybe(
                        (!a.neutral_values()).then(|| wrap(Msg::TestNeutral(mon.to_string()))),
                    ),
            );
        }
        let mut panel = column![head].spacing(6);
        if !folded {
            // Trois colonnes : luminosité/contraste/gamma, gains, gammas des canaux.
            let mut cols = row![].spacing(18);
            for chunk in FIELDS.chunks(3) {
                let mut c = column![].spacing(4);
                for (key, label, _, _, n) in chunk {
                    let v = a.get(key).unwrap_or(*n);
                    let d = step_of(key);
                    let b = |sym: &'static str, delta: f32| {
                        button(iced::widget::text(sym).size(14).color(fg))
                            .padding([2, 10])
                            .style(button::secondary)
                            .on_press(wrap(Msg::Step(mon.to_string(), key.to_string(), delta)))
                    };
                    c = c.push(
                        row![
                            small(crate::i18n::tr(label).to_string()).width(100),
                            b("−", -d),
                            iced::widget::text(format!("{v:.2}"))
                                .font(self.font)
                                .size(13)
                                .color(if (v - n).abs() < 1e-3 { fg } else { accent })
                                .width(44)
                                .align_x(iced::alignment::Horizontal::Center),
                            b("+", d),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    );
                }
                cols = cols.push(c);
            }
            panel = panel.push(cols);
            if let Some(e) = self.gamma.status().get(mon) {
                panel = panel.push(
                    iced::widget::text(e.clone())
                        .font(self.font)
                        .size(12)
                        .color(Color::from_rgb8(0xff, 0xb8, 0x6c)),
                );
            }
        }
        let card = container(panel)
            .padding(14)
            .max_width(if folded { 420.0 } else { 900.0 })
            .style(move |_| container::Style {
                background: Some(bg.into()),
                border: Border::default()
                    .rounded(14)
                    .width(1)
                    .color(Color { a: 0.4, ..accent }),
                ..Default::default()
            });
        match (&self.color_test.mire, self.color_test.span) {
            // Mire entière sur l'écran, réglages dessous : rien ne cache les
            // paliers des ombres et des hautes lumières, en bas de l'image.
            (Some(mire), false) => {
                let [r, g, b] = crate::colors::MIRE_FOND;
                let fond = Color::from_rgb8(r, g, b);
                container(
                    column![
                        image(mire.clone())
                            .width(Length::Fill)
                            .height(Length::Fill)
                            .content_fit(iced::ContentFit::Contain),
                        container(card)
                            .width(Length::Fill)
                            .align_x(Alignment::Center),
                    ]
                    .spacing(10),
                )
                .padding(12)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(fond.into()),
                    ..Default::default()
                })
                .into()
            }
            _ => stack![
                image(img.clone())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .content_fit(iced::ContentFit::Fill),
                container(card)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::End)
                    .padding(28),
            ]
            .into(),
        }
    }
}

impl App {
    /// Configuration (re)chargée : tables gamma envoyées au fil, et écrans
    /// réappliqués si un profil ICC a changé (gardé dans monitors.lua quand
    /// hyprdesk tient les écrans).
    pub(in crate::ui) fn apply_screen_colors(&mut self) -> Task<UiMsg> {
        let wanted = if self.cfg.plugin_on(INFO.id) {
            self.cfg.general.screen_colors.clone()
        } else {
            Default::default()
        };
        if self.colors_applied.as_ref() == Some(&wanted) {
            return Task::none();
        }
        self.gamma.set(wanted.clone());
        let first = self.colors_applied.is_none();
        self.colors_applied = Some(wanted.clone());
        let icc_changed = crate::colors::set_icc_map(&wanted);
        // Au démarrage sans profil : rien à réappliquer.
        if !icc_changed || (first && !wanted.values().any(|a| a.icc().is_some())) {
            return Task::none();
        }
        let keep = self.cfg.plugin_on("screens");
        let path = crate::screens::monitors_path(&self.config_path);
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || -> Result<(), String> {
                    let list = crate::screens::detect().map_err(|e| e.to_string())?;
                    crate::screens::apply(&list)?;
                    if keep {
                        crate::screens::save(&path, &list).map_err(|e| e.to_string())?;
                    }
                    Ok(())
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
            },
            |r| {
                if let Err(e) = r {
                    tracing::warn!("{}", crate::tr!("profil ICC des écrans : {e}", e = e));
                }
                wrap(Msg::Status)
            },
        )
    }
}
