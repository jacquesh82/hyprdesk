//! Paquet intégré « Écrans » : Détection, disposition, orientation, mode, écran principal (monitors.lua).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use std::time::Duration;

use iced::widget::{button, column, container, pick_list, radio, row, text, toggler};
use iced::{Alignment, Border, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::hypr;
use crate::screens::{self, Align, Screen, Side};

pub const INFO: PluginInfo = PluginInfo {
    id: "screens",
    label: "Écrans",
    icon: "video-display",
    summary: "Détection, disposition, orientation, mode, écran principal (monitors.lua).",
    builtin: true,
};

/// Message de ce paquet, enveloppé pour l'application.
pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Screens(m)))
}

/// État propre à ce paquet, porté par le tiroir des réglages.
#[derive(Default)]
pub struct State {
    pub screen_sel: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Écran sélectionné dans l'onglet Écrans.
    ScreenSelect(String),
    ScreenPrimary(String),
    ScreenEnable(bool),
    ScreenSide(crate::screens::Side),
    ScreenRef(String),
    ScreenAlign(crate::screens::Align),
    ScreenRotate(u16),
    ScreenFlip(bool),
    ScreenMode(String),
    ScreenScale(f32),
    ScreensAuto,
    ScreensDetect,
    /// Affiche le numéro et le nom de chaque écran, sur l'écran lui-même.
    Identify,
    /// Fin de l'identification (génération).
    IdentifyEnd(u64),
    /// Garder la disposition appliquée (écrite dans monitors.lua).
    ScreensKeep,
    /// Revenir à la disposition d'avant.
    ScreensRevert,
}

/// Délai pour garder une nouvelle disposition d'écrans avant retour automatique.
pub(in crate::ui) const SCREENS_CONFIRM_SECS: u64 = 12;
/// Applique une disposition, puis relit l'état réel.
pub(in crate::ui) fn apply_screens(list: Vec<Screen>) -> Task<UiMsg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || screens::apply(&list))
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r)
        },
        |r| {
            if let Err(e) = r {
                tracing::error!("écrans : {e}");
                hypr::notify(3, 6000, &format!("hyprdesk : écrans — {e}"));
            }
            UiMsg::Noop
        },
    )
    .chain(App::detect_screens())
}

impl App {
    pub(in crate::ui) fn tab_screens(&self) -> Element<'_, UiMsg> {
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let dim = Color { a: 0.6, ..fg };
        let small = |s: String| text(s).font(self.font).size(12).color(dim);
        let l = &self.screens;
        if l.is_empty() {
            return column![
                self.section("Écrans"),
                small("Aucun écran détecté pour l'instant.".into()),
                button(text("Détecter").font(self.font).size(13))
                    .padding([6, 12])
                    .on_press(wrap(Msg::ScreensDetect)),
            ]
            .spacing(14)
            .into();
        }
        let sel = self.screen_sel().unwrap_or_default();
        let primary = self.cfg.general.primary.clone();

        // Bandeau de confirmation après un changement.
        let mut col = column![].spacing(14);
        if let Some((_, deadline)) = &self.screens_pending {
            let left = deadline
                .saturating_duration_since(std::time::Instant::now())
                .as_secs();
            col = col.push(
                container(
                    row![
                        text(format!(
                            "Garder cet affichage ? Retour automatique dans {left} s"
                        ))
                        .font(self.font)
                        .size(14)
                        .width(Length::Fill),
                        button(text("Rétablir").font(self.font).size(13))
                            .padding([5, 12])
                            .on_press(wrap(Msg::ScreensRevert))
                            .style(button::secondary),
                        button(text("Garder").font(self.font).size(13))
                            .padding([5, 12])
                            .on_press(wrap(Msg::ScreensKeep)),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .padding(12)
                .style(move |_| container::Style {
                    background: Some(Color { a: 0.14, ..accent }.into()),
                    border: Border::default().rounded(10).color(accent).width(1),
                    ..Default::default()
                }),
            );
        }

        // Plan des écrans actifs, à l'échelle ; un clic sélectionne.
        const MAP_W: f32 = 500.0;
        const MAP_H: f32 = 210.0;
        let on: Vec<&Screen> = l.iter().filter(|s| s.enabled).collect();
        let rect = |s: &Screen| {
            let (w, h) = s.logical();
            (s.x as f32, s.y as f32, w as f32, h as f32)
        };
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for s in &on {
            let (x, y, w, h) = rect(s);
            (x0, y0, x1, y1) = (x0.min(x), y0.min(y), x1.max(x + w), y1.max(y + h));
        }
        let k = ((MAP_W - 8.0) / (x1 - x0).max(1.0)).min((MAP_H - 8.0) / (y1 - y0).max(1.0));
        let (ox, oy) = ((MAP_W - (x1 - x0) * k) / 2.0, (MAP_H - (y1 - y0) * k) / 2.0);
        let tiles: Vec<Element<'_, UiMsg>> = on
            .iter()
            .map(|s| {
                let (x, y, w, h) = rect(s);
                let selected = s.name == sel;
                let main = primary.as_deref() == Some(s.name.as_str());
                let number = l.iter().position(|o| o.name == s.name).map_or(0, |i| i + 1);
                let mut label = column![
                    text(format!("{number} · {}", s.name))
                        .font(self.font)
                        .size(13)
                ]
                .align_x(Alignment::Center)
                .spacing(2);
                if main {
                    label = label.push(text("★ principal").font(self.font).size(11).color(accent));
                }
                let tile = button(container(label).center(Length::Fill))
                    .width((w * k - 4.0).max(24.0))
                    .height((h * k - 4.0).max(24.0))
                    .padding(4)
                    .on_press(wrap(Msg::ScreenSelect(s.name.clone())))
                    .style(move |_, st| button::Style {
                        background: Some(
                            Color {
                                a: if selected {
                                    0.30
                                } else if matches!(st, button::Status::Hovered) {
                                    0.16
                                } else {
                                    0.08
                                },
                                ..if selected { accent } else { fg }
                            }
                            .into(),
                        ),
                        text_color: fg,
                        border: Border::default()
                            .rounded(6)
                            .width(if selected { 2 } else { 1 })
                            .color(if selected {
                                accent
                            } else {
                                Color { a: 0.3, ..fg }
                            }),
                        ..Default::default()
                    });
                container(tile)
                    .padding(iced::Padding {
                        left: ox + (x - x0) * k + 2.0,
                        top: oy + (y - y0) * k + 2.0,
                        ..iced::Padding::ZERO
                    })
                    .width(MAP_W)
                    .height(MAP_H)
                    .into()
            })
            .collect();
        let map = container(iced::widget::Stack::with_children(tiles))
            .width(MAP_W)
            .height(MAP_H)
            .style(move |_| container::Style {
                background: Some(Color { a: 0.04, ..fg }.into()),
                border: Border::default().rounded(10),
                ..Default::default()
            });
        let off: Vec<String> = l
            .iter()
            .filter(|s| !s.enabled)
            .map(|s| s.name.clone())
            .collect();
        let mut chips = row![].spacing(8).align_y(Alignment::Center);
        if !off.is_empty() {
            chips = chips.push(small("Désactivés :".into()));
            for name in off {
                chips = chips.push(
                    button(text(name.clone()).font(self.font).size(12))
                        .padding([3, 10])
                        .style(button::secondary)
                        .on_press(wrap(Msg::ScreenSelect(name))),
                );
            }
        }
        col = col
            .push(self.section("Disposition"))
            .push(map)
            .push(chips)
            .push(
                row![
                    button(text("Disposition automatique").font(self.font).size(13))
                        .padding([5, 12])
                        .on_press(wrap(Msg::ScreensAuto)),
                    button(text("Détecter").font(self.font).size(13))
                        .padding([5, 12])
                        .style(button::secondary)
                        .on_press(wrap(Msg::ScreensDetect)),
                    button(text("Identifier").font(self.font).size(13))
                        .padding([5, 12])
                        .style(button::secondary)
                        .on_press(wrap(Msg::Identify)),
                    button(text("Enregistrer").font(self.font).size(13))
                        .padding([5, 12])
                        .style(button::secondary)
                        .on_press_maybe(
                            self.screens_pending
                                .is_none()
                                .then_some(wrap(Msg::ScreensKeep)),
                        ),
                ]
                .spacing(10),
            );

        // Réglages de l'écran sélectionné.
        let Some(s) = l.iter().find(|s| s.name == sel) else {
            return col.into();
        };
        let title = if s.description.is_empty() {
            s.name.clone()
        } else {
            format!("{} — {}", s.name, s.description)
        };
        col = col
            .push(self.section("Écran sélectionné"))
            .push(text(title).font(self.font).size(14).font(self.title_font()));
        let is_main = primary.as_deref() == Some(s.name.as_str());
        col = col.push(
            row![
                toggler(s.enabled)
                    .label("Activé")
                    .on_toggle(|v| wrap(Msg::ScreenEnable(v))),
                if is_main {
                    Element::from(
                        text("★ Écran principal")
                            .font(self.font)
                            .size(13)
                            .color(accent),
                    )
                } else {
                    button(
                        text("Définir comme écran principal")
                            .font(self.font)
                            .size(13),
                    )
                    .padding([5, 12])
                    .on_press(wrap(Msg::ScreenPrimary(s.name.clone())))
                    .into()
                },
            ]
            .spacing(24)
            .align_y(Alignment::Center),
        );
        if !s.enabled {
            return col.into();
        }
        if let Some((reference, side, align)) = self.screen_placement(&s.name) {
            let refs: Vec<String> = l
                .iter()
                .filter(|o| o.name != s.name && o.enabled)
                .map(|o| o.name.clone())
                .collect();
            let aligns = [Align::Start, Align::Center, Align::End];
            col = col.push(
                self.field(
                    "Position",
                    "",
                    column![
                        row![
                            pick_list(Side::ALL, Some(side), |v| wrap(Msg::ScreenSide(v)))
                                .font(self.font)
                                .width(170),
                            pick_list(refs, Some(reference), |v| wrap(Msg::ScreenRef(v)))
                                .font(self.font)
                                .width(170),
                        ]
                        .spacing(10),
                        row(aligns.into_iter().map(|a| {
                            radio(a.label(side), a, Some(align), |v| wrap(Msg::ScreenAlign(v)))
                                .font(self.font)
                                .into()
                        }))
                        .spacing(18),
                    ]
                    .spacing(10),
                ),
            );
        }
        let rot = s.rotation();
        col = col.push(
            self.field(
                "Orientation",
                "",
                column![
                    column(
                        [
                            [(0u16, "Paysage"), (180, "Paysage retourné")],
                            [(90, "Portrait"), (270, "Portrait retourné")],
                        ]
                        .into_iter()
                        .map(|pair| {
                            row(pair.into_iter().map(|(d, label)| {
                                radio(label, d, Some(rot), |v| wrap(Msg::ScreenRotate(v)))
                                    .font(self.font)
                                    .width(190)
                                    .into()
                            }))
                            .into()
                        })
                    )
                    .spacing(8),
                    toggler(s.flipped())
                        .label("Image en miroir")
                        .on_toggle(|v| wrap(Msg::ScreenFlip(v))),
                ]
                .spacing(10),
            ),
        );
        let mut modes = s.modes.clone();
        if !modes.contains(&s.mode()) {
            modes.insert(0, s.mode());
        }
        let scale_label = |v: f32| format!("{:.0} %", v * 100.0);
        let scales: Vec<String> = screens::SCALES.iter().map(|v| scale_label(*v)).collect();
        col = col
            .push(
                self.field(
                    "Résolution et fréquence",
                    "",
                    pick_list(modes, Some(s.mode()), |m| wrap(Msg::ScreenMode(m)))
                        .font(self.font)
                        .width(240),
                ),
            )
            .push(
                self.field(
                    "Échelle",
                    "",
                    pick_list(scales, Some(scale_label(s.scale)), |label| {
                        let v = screens::SCALES
                            .iter()
                            .copied()
                            .find(|v| format!("{:.0} %", v * 100.0) == label)
                            .unwrap_or(1.0);
                        wrap(Msg::ScreenScale(v))
                    })
                    .font(self.font)
                    .width(140),
                ),
            )
            .push(small(
                "Chaque changement s'applique tout de suite ; gardez-le, sinon l'affichage \
                 d'avant revient de lui-même. La disposition est écrite dans monitors.lua."
                    .into(),
            ));
        col.into()
    }
    /// Relit les écrans auprès de Hyprland.
    pub(in crate::ui) fn detect_screens() -> Task<UiMsg> {
        Task::perform(
            async {
                tokio::task::spawn_blocking(screens::detect)
                    .await
                    .ok()
                    .and_then(Result::ok)
                    .unwrap_or_default()
            },
            UiMsg::Screens,
        )
    }
    /// Écran sélectionné dans l'onglet, sinon le principal.
    pub(in crate::ui) fn screen_sel(&self) -> Option<String> {
        let sel = self
            .panel
            .as_ref()
            .and_then(|p| p.screens.screen_sel.clone());
        let known = |n: &String| self.screens.iter().any(|s| &s.name == n);
        sel.filter(known)
            .or_else(|| self.cfg.general.primary.clone().filter(known))
            .or_else(|| self.primary_monitor().filter(known))
            .or_else(|| self.screens.first().map(|s| s.name.clone()))
    }
    /// Où est placé un écran : (référence, côté, alignement).
    pub(in crate::ui) fn screen_placement(&self, name: &str) -> Option<(String, Side, Align)> {
        let l = &self.screens;
        let i = l.iter().position(|s| s.name == name)?;
        let a = screens::anchor(l, self.cfg.general.primary.as_deref())?;
        let linked = (i != a)
            .then(|| screens::links(l, a))
            .and_then(|k| k.iter().find(|k| k.screen == i).map(|k| k.reference));
        let r = linked.or_else(|| {
            l.iter()
                .enumerate()
                .filter(|(j, s)| *j != i && s.enabled)
                .min_by_key(|(_, s)| screens::gap(&l[i], s))
                .map(|(j, _)| j)
        })?;
        let (side, align) = screens::relation(&l[i], &l[r]).unwrap_or((Side::Right, Align::Start));
        Some((l[r].name.clone(), side, align))
    }
    /// Modifie la disposition et l'applique à chaud ; elle attend ensuite
    /// d'être gardée, sinon elle est rétablie à l'échéance.
    pub(in crate::ui) fn edit_screens(
        &mut self,
        f: impl FnOnce(&mut Vec<Screen>, &[screens::Link]),
    ) -> Task<UiMsg> {
        let before = self.screens.clone();
        if before.is_empty() {
            return Task::none();
        }
        let primary = self.cfg.general.primary.clone();
        let mut next = before.clone();
        let links = screens::anchor(&next, primary.as_deref())
            .map(|a| screens::links(&next, a))
            .unwrap_or_default();
        f(&mut next, &links);
        if screens::overlapping(&next) {
            screens::auto_arrange(&mut next, primary.as_deref());
        }
        screens::normalize(&mut next, primary.as_deref());
        if next == before {
            return Task::none();
        }
        let deadline = std::time::Instant::now() + Duration::from_secs(SCREENS_CONFIRM_SECS);
        match &mut self.screens_pending {
            Some((_, d)) => *d = deadline,
            None => self.screens_pending = Some((before, deadline)),
        }
        self.screens = next.clone();
        apply_screens(next)
    }
    pub(in crate::ui) fn edit_selected(&mut self, f: impl FnOnce(&mut Screen)) -> Task<UiMsg> {
        let Some(sel) = self.screen_sel() else {
            return Task::none();
        };
        self.edit_screens(|l, links| {
            if let Some(s) = l.iter_mut().find(|s| s.name == sel) {
                f(s);
            }
            // Taille changée : chaque écran reste contre son voisin.
            screens::replay(l, links);
        })
    }
    pub(in crate::ui) fn place_selected(
        &mut self,
        f: impl FnOnce(&mut (String, Side, Align)),
    ) -> Task<UiMsg> {
        let Some(sel) = self.screen_sel() else {
            return Task::none();
        };
        let Some(mut p) = self.screen_placement(&sel) else {
            return Task::none();
        };
        f(&mut p);
        if p.0 == sel {
            return Task::none();
        }
        self.edit_screens(|l, _| {
            let Some(r) = l.iter().find(|s| s.name == p.0).cloned() else {
                return;
            };
            if let Some(s) = l.iter_mut().find(|s| s.name == sel) {
                screens::place(s, &r, p.1, p.2);
            }
        })
    }
    /// Garde la disposition : écrite dans monitors.lua.
    pub(in crate::ui) fn screens_keep(&mut self) -> Task<UiMsg> {
        self.screens_pending = None;
        let (list, path) = (
            self.screens.clone(),
            screens::monitors_path(&self.config_path),
        );
        if list.is_empty() {
            return Task::none();
        }
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || screens::save(&path, &list))
                    .await
                    .map_err(|e| e.to_string())
                    .and_then(|r| r.map_err(|e| e.to_string()))
            },
            |r| {
                match r {
                    Ok(()) => {
                        hypr::notify(5, 3000, "hyprdesk : disposition des écrans enregistrée")
                    }
                    Err(e) => {
                        tracing::error!("monitors.lua : {e}");
                        hypr::notify(3, 6000, &format!("hyprdesk : monitors.lua non écrit — {e}"));
                    }
                }
                UiMsg::Noop
            },
        )
    }
    pub(in crate::ui) fn screens_revert(&mut self) -> Task<UiMsg> {
        let Some((previous, _)) = self.screens_pending.take() else {
            return Task::none();
        };
        self.screens = previous.clone();
        apply_screens(previous)
    }
    /// Échéance passée sans confirmation : retour à la disposition d'avant.
    pub(in crate::ui) fn screens_tick(&mut self) -> Task<UiMsg> {
        match &self.screens_pending {
            Some((_, deadline)) if std::time::Instant::now() >= *deadline => {
                hypr::notify(1, 4000, "hyprdesk : disposition des écrans rétablie");
                self.screens_revert()
            }
            _ => Task::none(),
        }
    }

    pub(in crate::ui) fn screens_update(&mut self, msg: Msg) -> Task<UiMsg> {
        let Some(panel) = self.panel.as_mut() else {
            return Task::none();
        };
        match msg {
            Msg::ScreenSelect(name) => {
                panel.screens.screen_sel = Some(name);
                Task::none()
            }
            Msg::ScreenPrimary(name) => {
                self.settings.primary = Some(name);
                let saved = self.commit();
                // L'écran principal est l'ancre de la disposition, en 0×0.
                let moved = self.edit_screens(|_, _| {});
                Task::batch([saved, moved])
            }
            Msg::ScreenEnable(on) => {
                let Some(sel) = self.screen_sel() else {
                    return Task::none();
                };
                let others_on = self.screens.iter().any(|s| s.name != sel && s.enabled);
                if !on && !others_on {
                    crate::hypr::notify(3, 4000, "hyprdesk : au moins un écran doit rester actif");
                    return Task::none();
                }
                self.edit_screens(|l, _| {
                    if let Some(s) = l.iter_mut().find(|s| s.name == sel) {
                        s.enabled = on;
                    }
                })
            }
            Msg::ScreenSide(side) => self.place_selected(|p| p.1 = side),
            Msg::ScreenRef(r) => self.place_selected(|p| p.0 = r),
            Msg::ScreenAlign(align) => self.place_selected(|p| p.2 = align),
            Msg::ScreenRotate(deg) => self.edit_selected(|s| {
                let flip = s.flipped();
                s.set_orientation(deg, flip);
            }),
            Msg::ScreenFlip(flip) => self.edit_selected(|s| {
                let rot = s.rotation();
                s.set_orientation(rot, flip);
            }),
            Msg::ScreenMode(m) => self.edit_selected(|s| {
                s.set_mode(&m);
            }),
            Msg::ScreenScale(v) => self.edit_selected(|s| s.scale = v),
            Msg::ScreensAuto => {
                let primary = self.cfg.general.primary.clone();
                self.edit_screens(move |l, _| crate::screens::auto_arrange(l, primary.as_deref()))
            }
            Msg::ScreensDetect => Self::detect_screens(),
            Msg::Identify => self.identify_screens(),
            Msg::IdentifyEnd(generation) => {
                if generation != self.identify_gen {
                    return Task::none();
                }
                Task::batch(
                    std::mem::take(&mut self.identify)
                        .into_iter()
                        .map(|(id, _)| iced::window::close(id)),
                )
            }
            Msg::ScreensKeep => self.screens_keep(),
            Msg::ScreensRevert => self.screens_revert(),
        }
    }
}

impl App {
    /// Numéro (dans l'ordre de l'onglet), nom et modèle sur chaque écran actif,
    /// pendant trois secondes, au-dessus de tout.
    pub(in crate::ui) fn identify_screens(&mut self) -> Task<UiMsg> {
        use iced_layershell::reexport::{
            Anchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
        };
        self.identify_gen = self.identify_gen.wrapping_add(1);
        let mut tasks: Vec<Task<UiMsg>> = std::mem::take(&mut self.identify)
            .into_iter()
            .map(|(id, _)| iced::window::close(id))
            .collect();
        for (i, s) in self.screens.iter().enumerate().filter(|(_, s)| s.enabled) {
            let id = iced::window::Id::unique();
            self.identify
                .push((id, format!("{}\n{}\n{}", i + 1, s.name, s.description)));
            tasks.push(Task::done(UiMsg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some((520, 260)),
                    layer: Layer::Overlay,
                    anchor: Anchor::empty(),
                    exclusive_zone: Some(0),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    output_option: OutputOption::OutputName(s.name.clone()),
                    namespace: Some("hyprdesk-identify".into()),
                    ..Default::default()
                },
                id,
            }));
        }
        let generation = self.identify_gen;
        tasks.push(Task::perform(
            async { tokio::time::sleep(Duration::from_secs(3)).await },
            move |_| wrap(Msg::IdentifyEnd(generation)),
        ));
        Task::batch(tasks)
    }

    /// Carte d'identification d'un écran : grand numéro, connecteur, modèle.
    pub(in crate::ui) fn view_identify(&self, label: &str) -> Element<'_, UiMsg> {
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let mut parts = label.splitn(3, '\n');
        let (n, name, model) = (
            parts.next().unwrap_or_default().to_string(),
            parts.next().unwrap_or_default().to_string(),
            parts.next().unwrap_or_default().to_string(),
        );
        let bg = color(th.bg, 0.94);
        container(
            column![
                text(n).font(self.title_font()).size(96).color(accent),
                text(name).font(self.title_font()).size(28).color(fg),
                text(model)
                    .font(self.font)
                    .size(14)
                    .color(Color { a: 0.7, ..fg }),
            ]
            .spacing(4)
            .align_x(Alignment::Center),
        )
        .center(Length::Fill)
        .style(move |_| container::Style {
            background: Some(bg.into()),
            border: Border::default().rounded(18).width(2).color(accent),
            ..Default::default()
        })
        .into()
    }
}
