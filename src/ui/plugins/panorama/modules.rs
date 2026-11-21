//! Réglages → Plugins → Modules : les modules de la bande du profil
//! Raccourcis (ajouter, retirer), puis les réglages de ceux qui en ont :
//! Hue (pont, appairage, pièces montrées) et Notes (commande, écran).
//!
//! Logique des modules : `crate::panorama::modules`.

use crate::i18n::text;
use crate::panorama::modules::{self, hue, hue::Bridge, hue::Room, notes};
use iced::widget::{button, column, row, text_input, toggler};
use iced::{Alignment, Element, Length, Task};

use super::super::super::{App, Msg as UiMsg, color};

pub fn wrap(m: Msg) -> UiMsg {
    super::wrap(super::Msg::Module(m))
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    pub hue_loaded: bool,
    pub bridge: Option<Bridge>,
    pub rooms: Vec<Room>,
    /// Ponts trouvés sur le réseau.
    pub found: Vec<String>,
    /// Adresse saisie à la main.
    pub ip: String,
    pub busy: bool,
    pub hue_status: Option<Result<String, String>>,
    /// Commande Notes en cours de saisie (aucune : celle de la config).
    pub notes_draft: Option<String>,
    pub notes_status: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Module ajouté à la bande ou retiré.
    Toggle(String),
    HueLoaded(Option<Bridge>, Result<Vec<Room>, String>),
    HueDiscover,
    HueFound(Result<Vec<String>, String>),
    HueIp(String),
    HuePair(String),
    HuePaired(Result<Bridge, String>),
    HueForget,
    /// Pièce montrée (ou non) dans la tuile Hue.
    HueShow(String, bool),
    NotesDraft(String),
    NotesSubmit,
    NotesMonitor(String),
    NotesOpen,
    NotesOpened(Result<(), String>),
    /// Cast vers la zone vidéo (page du téléphone, récepteur YouTube).
    Cast(bool),
}

/// Pont enregistré et ses pièces.
pub fn load() -> Task<UiMsg> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(|| {
                let b = hue::load();
                let rooms = match &b {
                    Some(b) => hue::rooms(b),
                    None => Ok(Vec::new()),
                };
                (b, rooms)
            })
            .await
            .unwrap_or((None, Ok(Vec::new())))
        },
        |(b, r)| wrap(Msg::HueLoaded(b, r)),
    )
}

impl App {
    fn modules_state(&self) -> Option<&State> {
        self.panel.as_ref().map(|p| &p.panorama_modules)
    }

    /// Enregistre une option de module (`plugin_options.panorama`).
    fn module_option(&mut self, module: &str, key: &str, value: &str) -> Task<UiMsg> {
        let all = self
            .settings
            .plugin_options
            .clone()
            .unwrap_or_else(|| self.cfg.general.plugin_options.clone());
        self.settings.plugin_options = Some(modules::with_option(all, module, key, value));
        self.commit()
    }

    pub(in crate::ui) fn panorama_modules_tab(&self) -> Element<'_, UiMsg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let dim = self.ink().dim;
        let warn = iced::Color::from_rgb8(0xff, 0xb8, 0x6c);
        let c = self.panorama_cfg();
        let current = c.design.modules();
        let mut col = column![
            text(
                "Le profil Raccourcis montre ces modules côte à côte, dans une bande qui défile \
                 de côté ; la tuile « + » de la bande les ajoute ou les retire aussi.",
            )
            .font(self.font)
            .size(12)
            .color(dim),
            self.section("Modules de la bande"),
        ]
        .spacing(10);
        for (id, label, _) in modules::CATALOGUE {
            let on = current.iter().any(|m| m == id);
            col = col.push(
                row![
                    text(crate::i18n::tr(label))
                        .font(self.font)
                        .size(13)
                        .color(fg)
                        .width(Length::Fill),
                    toggler(on).on_toggle(move |_| wrap(Msg::Toggle(id.to_string()))),
                ]
                .align_y(Alignment::Center),
            );
        }

        // Zone vidéo : cast depuis le téléphone (MLG-217, MLG-218).
        let cast = crate::panorama::cast::enabled(&self.cfg);
        col = col
            .push(self.section("Zone vidéo"))
            .push(
                row![
                    text("Recevoir depuis le téléphone (Partager, cast YouTube)")
                        .font(self.font)
                        .size(13)
                        .color(fg)
                        .width(Length::Fill),
                    toggler(cast).on_toggle(|v| wrap(Msg::Cast(v))),
                ]
                .align_y(Alignment::Center),
            )
            .push(
                text(crate::tr!(
                    "Un petit serveur du réseau local (port {port}) : la tuile « Téléphone » de la \
                     zone vidéo montre le code à scanner ; les liens partagés depuis Disney+, HBO \
                     Max… s'y ouvrent, et l'application YouTube y voit « {name} » sous son icône cast.",
                    port = crate::panorama::cast::PORT,
                    name = crate::panorama::cast::device_name()
                ))
                .font(self.font)
                .size(12)
                .color(dim),
            );
        if let Some(p) = cast.then(crate::panorama::cast::problem).flatten() {
            col = col.push(text(p).font(self.font).size(12).color(warn));
        }

        // Hue
        col = col.push(self.section("Hue"));
        let Some(s) = self.modules_state().filter(|s| s.hue_loaded) else {
            return col
                .push(text("Lecture…").font(self.font).size(13).color(dim))
                .into();
        };
        match &s.bridge {
            Some(b) => {
                col = col.push(
                    row![
                        text(crate::tr!("Pont appairé : {ip}", ip = b.ip.clone()))
                            .font(self.font)
                            .size(13)
                            .color(fg)
                            .width(Length::Fill),
                        button(text("Oublier ce pont").font(self.font).size(12))
                            .padding([4, 10])
                            .on_press(wrap(Msg::HueForget)),
                    ]
                    .align_y(Alignment::Center),
                );
            }
            None => {
                col = col.push(
                    text(
                        "Appuyez sur le bouton rond du pont Hue, puis touchez « Appairer » dans les \
                         30 secondes.",
                    )
                    .font(self.font)
                    .size(12)
                    .color(dim),
                );
                for ip in &s.found {
                    col = col.push(
                        row![
                            text(ip.clone())
                                .font(self.font)
                                .size(13)
                                .color(fg)
                                .width(Length::Fill),
                            button(text("Appairer").font(self.font).size(12))
                                .padding([4, 10])
                                .on_press_maybe((!s.busy).then(|| wrap(Msg::HuePair(ip.clone()))),),
                        ]
                        .align_y(Alignment::Center),
                    );
                }
                col = col.push(
                    row![
                        button(text("Chercher le pont").font(self.font).size(12))
                            .padding([4, 10])
                            .on_press_maybe((!s.busy).then(|| wrap(Msg::HueDiscover))),
                        text_input("192.168.1.20", &s.ip)
                            .font(self.font)
                            .size(13)
                            .padding(6)
                            .width(180)
                            .on_input(|v| wrap(Msg::HueIp(v)))
                            .on_submit(wrap(Msg::HuePair(s.ip.trim().to_string()))),
                        button(text("Appairer cette adresse").font(self.font).size(12))
                            .padding([4, 10])
                            .on_press_maybe(
                                (!s.busy && !s.ip.trim().is_empty())
                                    .then(|| wrap(Msg::HuePair(s.ip.trim().to_string()))),
                            ),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                );
            }
        }
        if s.busy {
            col = col.push(text("…").font(self.font).size(12).color(dim));
        }
        if let Some(st) = &s.hue_status {
            let (msg, c) = match st {
                Ok(m) => (m.clone(), dim),
                Err(e) => (e.clone(), warn),
            };
            col = col.push(text(msg).font(self.font).size(12).color(c));
        }
        if s.bridge.is_some() {
            col = col.push(
                text("Pièces montrées dans la tuile Hue du Panorama.")
                    .font(self.font)
                    .size(12)
                    .color(dim),
            );
            let hidden = hue::hidden(&self.cfg);
            for r in &s.rooms {
                let id = r.id.clone();
                col = col.push(
                    row![
                        text(r.name.clone())
                            .font(self.font)
                            .size(13)
                            .color(fg)
                            .width(Length::Fill),
                        text(crate::tr!("{n} lampe(s)", n = r.lights.len()))
                            .font(self.font)
                            .size(12)
                            .color(dim),
                        toggler(!hidden.contains(&r.id))
                            .on_toggle(move |v| wrap(Msg::HueShow(id.clone(), v))),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                );
            }
        }

        // Notes
        let (command, monitor) = notes::options(&self.cfg);
        let shown = s.notes_draft.clone().unwrap_or(command);
        col = col.push(self.section("Notes")).push(
            text(
                "La commande lancée par la tuile Notes : un éditeur de texte, une application de \
                 notes, un script… Elle s'ouvre sur le workspace actif de l'écran choisi.",
            )
            .font(self.font)
            .size(12)
            .color(dim),
        );
        // La commande lancée : avancé (par défaut, un éditeur détecté).
        col = col.push(
            self.adv(
                row![
                    text_input("gnome-text-editor", &shown)
                        .font(self.font)
                        .size(13)
                        .padding(6)
                        .width(Length::Fill)
                        .on_input(|v| wrap(Msg::NotesDraft(v)))
                        .on_submit(wrap(Msg::NotesSubmit)),
                    button(text("Enregistrer").font(self.font).size(12))
                        .padding([4, 10])
                        .on_press(wrap(Msg::NotesSubmit)),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ),
        );
        let mut screens = row![].spacing(8);
        let mut names: Vec<&str> = vec![""];
        names.extend(self.monitors.iter().map(|m| m.name.as_str()));
        for name in names {
            let on = monitor == name;
            let label = if name.is_empty() {
                crate::i18n::tr("Écran actif").to_string()
            } else {
                name.to_string()
            };
            screens = screens.push(
                button(text(label).font(self.font).size(12).color(fg))
                    .padding([4, 10])
                    .style(move |t, st| {
                        if on {
                            button::primary(t, st)
                        } else {
                            button::secondary(t, st)
                        }
                    })
                    .on_press(wrap(Msg::NotesMonitor(name.to_string()))),
            );
        }
        col = col.push(screens).push(
            button(text("Ouvrir maintenant").font(self.font).size(12))
                .padding([4, 10])
                .on_press(wrap(Msg::NotesOpen)),
        );
        if let Some(e) = &s.notes_status {
            col = col.push(text(e.clone()).font(self.font).size(12).color(warn));
        }
        col.into()
    }

    pub(in crate::ui) fn panorama_modules_settings_update(&mut self, m: Msg) -> Task<UiMsg> {
        match m {
            Msg::Toggle(id) => {
                let mut c = self.panorama_cfg();
                c.design.toggle_module(&id);
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::HueShow(id, shown) => {
                let mut list = hue::hidden(&self.cfg);
                list.retain(|h| *h != id);
                if !shown {
                    list.push(id);
                }
                self.module_option("hue", "hidden", &list.join(","))
            }
            Msg::NotesSubmit => {
                let Some(v) = self
                    .panel
                    .as_mut()
                    .and_then(|p| p.panorama_modules.notes_draft.take())
                else {
                    return Task::none();
                };
                self.module_option("notes", "command", &v)
            }
            Msg::NotesMonitor(name) => self.module_option("notes", "monitor", &name),
            Msg::Cast(on) => self.module_option("video", "cast", if on { "" } else { "off" }),
            Msg::NotesOpen => {
                let (command, monitor) = notes::options(&self.cfg);
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || notes::open(&command, &monitor))
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| wrap(Msg::NotesOpened(r)),
                )
            }
            m => {
                let Some(p) = self.panel.as_mut() else {
                    return Task::none();
                };
                let s = &mut p.panorama_modules;
                match m {
                    Msg::HueLoaded(b, rooms) => {
                        s.hue_loaded = true;
                        s.busy = false;
                        s.bridge = b;
                        match rooms {
                            Ok(r) => s.rooms = r,
                            Err(e) => s.hue_status = Some(Err(e)),
                        }
                        Task::none()
                    }
                    Msg::HueDiscover => {
                        s.busy = true;
                        s.hue_status = None;
                        Task::perform(
                            async {
                                tokio::task::spawn_blocking(hue::discover)
                                    .await
                                    .unwrap_or_else(|e| Err(e.to_string()))
                            },
                            |r| wrap(Msg::HueFound(r)),
                        )
                    }
                    Msg::HueFound(r) => {
                        s.busy = false;
                        match r {
                            Ok(list) if list.is_empty() => {
                                s.hue_status = Some(Err(crate::i18n::tr(
                                    "Aucun pont trouvé : saisissez son adresse (application Hue → Réglages → Ponts).",
                                )
                                .to_string()))
                            }
                            Ok(list) => s.found = list,
                            Err(e) => s.hue_status = Some(Err(e)),
                        }
                        Task::none()
                    }
                    Msg::HueIp(v) => {
                        s.ip = v;
                        Task::none()
                    }
                    Msg::HuePair(ip) if !ip.is_empty() => {
                        s.busy = true;
                        s.hue_status = None;
                        Task::perform(
                            async move {
                                tokio::task::spawn_blocking(move || {
                                    let user = hue::pair(&ip)?;
                                    let b = Bridge { ip, user };
                                    hue::save(&b)?;
                                    Ok(b)
                                })
                                .await
                                .unwrap_or_else(|e| Err(e.to_string()))
                            },
                            |r| wrap(Msg::HuePaired(r)),
                        )
                    }
                    Msg::HuePaired(Ok(_)) => {
                        s.hue_status = Some(Ok(crate::i18n::tr("Pont appairé.").to_string()));
                        Task::batch([load(), self.panorama_hue_refresh()])
                    }
                    Msg::HuePaired(Err(e)) => {
                        s.busy = false;
                        s.hue_status = Some(Err(e));
                        Task::none()
                    }
                    Msg::HueForget => {
                        hue::forget();
                        s.bridge = None;
                        s.rooms.clear();
                        s.found.clear();
                        s.hue_status = None;
                        self.panorama_hue_refresh()
                    }
                    Msg::NotesDraft(v) => {
                        s.notes_draft = Some(v);
                        Task::none()
                    }
                    Msg::NotesOpened(r) => {
                        s.notes_status = r.err();
                        Task::none()
                    }
                    _ => Task::none(),
                }
            }
        }
    }
}
