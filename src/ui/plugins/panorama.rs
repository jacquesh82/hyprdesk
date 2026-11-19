//! Paquet intégré « Panorama » (MLG-164) : rôle des écrans bandeau, pages et
//! veille du widget plein écran tactile (`crate::panorama`, `ui::panorama`).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

mod design;

use crate::i18n::text;
use iced::widget::{Space, button, column, row, slider, toggler};
use iced::{Alignment, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg, SubTab};

pub const INFO: PluginInfo = PluginInfo {
    id: "panorama",
    label: "Panorama",
    icon: "video-display",
    summary: "Écran bandeau tactile en plein écran : lecteur multimédia, système, horloge.",
    builtin: true,
};

pub const TABS: &[SubTab] = &[
    ("ecrans", "Écrans"),
    ("pages", "Pages"),
    ("composition", "Composition"),
    ("style", "Style"),
    ("raccourcis", "Raccourcis"),
];

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Panorama(m)))
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Écran (connecteur), rôle Panorama.
    Role(String, bool),
    Page(String, bool),
    /// Page, ±1 dans l'ordre.
    Move(String, i32),
    Sleep(u32),
    SleepCommit,
    /// Relit les écrans et refait les propositions.
    Detect,
    /// Afficher ou escamoter le Panorama de cet écran.
    Toggle(String),
    /// Fenêtre d'aperçu au format donné.
    Preview((u32, u32)),
    Layout(crate::panorama::config::Layout),
    /// Zone, widget.
    PaneWidget(usize, String),
    SourceName(String),
    SourceUrl(String),
    SourceAdd,
    SourceRemove(usize),
    SourcesReset,
    Browser(String),
    BrowserCommit,
    Preset(String),
    DesignText(String, String),
    DesignApply(String),
    DesignChoice(String, String),
    DesignNumber(String, f32),
    DesignFlag(String, bool),
    DesignCommit,
    TileSelect(usize),
    TileWidget(String),
    TileNumber(String, f32),
    TileAdd,
    TileRemove,
    ShortcutText(usize, String),
    ShortcutAdd,
    ShortcutRemove(usize),
}

/// Libellé d'une disposition.
fn layout_label(l: crate::panorama::config::Layout) -> &'static str {
    use crate::panorama::config::Layout;
    match l {
        Layout::Single => "Une page à la fois",
        Layout::Split2 => "Écran partagé en 2",
        Layout::Split3 => "Écran partagé en 3",
        Layout::Dashboard => "Composition libre",
    }
}

/// Formats proposés pour l'aperçu : (taille simulée, libellé).
const PREVIEW_FORMATS: &[((u32, u32), &str)] = &[
    ((2560, 720), "2560×720"),
    ((1920, 480), "1920×480"),
    ((1280, 400), "1280×400"),
    ((720, 2560), "720×2560 (portrait)"),
];

pub fn load() -> Task<UiMsg> {
    super::super::panorama::detect_task()
}

impl App {
    pub(in crate::ui) fn panorama_page(&self) -> Element<'_, UiMsg> {
        if self.on_subtab(INFO.id, "style") {
            return self.panorama_style_tab();
        }
        if self.on_subtab(INFO.id, "composition") {
            return self.panorama_composition_tab();
        }
        if self.on_subtab(INFO.id, "raccourcis") {
            return self.panorama_shortcuts_tab();
        }
        if self.on_subtab(INFO.id, "pages") {
            return self.panorama_pages_tab();
        }
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let dim = self.ink().dim;
        let c = self.panorama_cfg();
        let mut col = column![
            text(
                "Un écran très allongé (2560×720, 1920×480, 1280×400…, posé à l'horizontale \
                 ou à la verticale) devient un Panorama : un plein écran à pages qu'on fait \
                 glisser au doigt ou à la souris. Il ne prend jamais le clavier aux fenêtres."
            )
            .font(self.font)
            .size(12)
            .color(dim),
            self.section("Écrans"),
        ]
        .spacing(10);
        let shown = &self.panorama.screens;
        if shown.is_empty() {
            col = col.push(
                text("Lecture des écrans…")
                    .font(self.font)
                    .size(12)
                    .color(dim),
            );
        }
        for s in shown {
            let on = c.has(s);
            let label = format!(
                "{} — {} ({}×{}{})",
                s.name,
                if s.edid_id().is_empty() {
                    "?".to_string()
                } else {
                    s.edid_id()
                },
                s.width,
                s.height,
                if s.is_candidate() {
                    crate::i18n::tr(", écran bandeau")
                } else {
                    ""
                }
            );
            let name = s.name.clone();
            let mut line = row![
                toggler(on)
                    .label(label)
                    .text_size(13)
                    .on_toggle(move |v| wrap(Msg::Role(name.clone(), v))),
                Space::new().width(Length::Fill),
            ]
            .spacing(10)
            .align_y(Alignment::Center);
            if on && self.monitors.iter().any(|m| m.name == s.name) {
                let hidden = self.panorama.hidden.contains(&s.name);
                line = line.push(
                    button(
                        text(if hidden { "Afficher" } else { "Escamoter" })
                            .font(self.font)
                            .size(12),
                    )
                    .padding([4, 10])
                    .on_press(wrap(Msg::Toggle(s.name.clone()))),
                );
            }
            col = col.push(line);
        }
        col = col.push(self.section("Tactile"));
        col = col.push(
            text(if self.panorama.touch.is_empty() {
                crate::i18n::tr("Aucun périphérique tactile : le Panorama se pilote à la souris.")
                    .to_string()
            } else {
                crate::tr!(
                    "Périphériques tactiles : {p0}. Ils sont rattachés à l'écran Panorama (inputs.lua).",
                    p0 = self.panorama.touch.join(", ")
                )
            })
            .font(self.font)
            .size(12)
            .color(fg),
        );
        if !c.declined.is_empty() {
            col = col.push(
                text(crate::tr!(
                    "Propositions refusées : {p0} (activer l'écran ci-dessus les annule).",
                    p0 = c.declined.join(", ")
                ))
                .font(self.font)
                .size(12)
                .color(dim),
            );
        }
        col = col.push(
            button(text("Rechercher les écrans").font(self.font).size(12))
                .padding([4, 10])
                .on_press(wrap(Msg::Detect)),
        );
        col = col.push(self.section("Aperçu"));
        col = col.push(
            text(
                "Le Panorama dans une fenêtre flottante, avec le lecteur et le système en \
                 direct : pour l'essayer sans écran bandeau. Pages au glisser de la souris ; \
                 la fenêtre se redimensionne (plus haute que large : portrait).",
            )
            .font(self.font)
            .size(12)
            .color(dim),
        );
        let open = self.panorama.preview.map(|p| p.virtual_size);
        let mut formats = row![].spacing(8);
        for (size, label) in PREVIEW_FORMATS {
            let on = open == Some(*size);
            formats = formats.push(
                button(text(crate::i18n::tr(label)).font(self.font).size(12))
                    .padding([4, 10])
                    .style(move |t, st| {
                        if on {
                            button::primary(t, st)
                        } else {
                            button::secondary(t, st)
                        }
                    })
                    .on_press(wrap(Msg::Preview(*size))),
            );
        }
        col = col.push(formats);
        col.into()
    }

    fn panorama_pages_tab(&self) -> Element<'_, UiMsg> {
        use crate::panorama::config::Layout;
        let dim = self.ink().dim;
        let c = self.panorama_cfg();
        let mut col = column![self.section("Disposition")].spacing(10);
        for l in Layout::ALL {
            col = col.push(
                iced::widget::radio(crate::i18n::tr(layout_label(l)), l, Some(c.layout), |v| {
                    wrap(Msg::Layout(v))
                })
                .font(self.font)
                .size(16)
                .text_size(13),
            );
        }
        if matches!(c.layout, Layout::Split2 | Layout::Split3) {
            let options: Vec<String> = crate::panorama::PAGES
                .iter()
                .map(|(id, _)| id.to_string())
                .collect();
            for i in 0..c.layout.panes() {
                let cur = c.pane(i).to_string();
                col = col.push(
                    row![
                        text(crate::tr!("Zone {n}", n = i + 1))
                            .font(self.font)
                            .size(13)
                            .width(80),
                        iced::widget::pick_list(options.clone(), Some(cur), move |v| {
                            wrap(Msg::PaneWidget(i, v))
                        })
                        .text_size(13),
                        text(crate::i18n::tr(crate::panorama::page_label(c.pane(i))))
                            .font(self.font)
                            .size(12)
                            .color(dim),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                );
            }
        }
        col = col.push(
            text(
                "Sur l'écran, les boutons 1 | 2 | 3 changent la disposition, et ◀ ▶ le widget \
                 de chaque zone.",
            )
            .font(self.font)
            .size(12)
            .color(dim),
        );
        col = col.push(self.panorama_sources_section());
        col = col.extend([
            self.section("Pages"),
            text("Glisser d'une page à l'autre ; les points en bas de l'écran y mènent aussi.")
                .font(self.font)
                .size(12)
                .color(dim)
                .into(),
        ]);
        // Pages actives dans leur ordre, puis les autres.
        let mut ids: Vec<String> = c.pages.clone();
        for (id, _) in crate::panorama::PAGES {
            if !ids.iter().any(|p| p == id) {
                ids.push(id.to_string());
            }
        }
        for id in ids {
            let on = c.pages.contains(&id);
            let (a, b) = (id.clone(), id.clone());
            let mut line = row![
                toggler(on)
                    .label(crate::i18n::tr(crate::panorama::page_label(&id)))
                    .text_size(13)
                    .on_toggle(move |v| wrap(Msg::Page(a.clone(), v))),
                Space::new().width(Length::Fill),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            if on && c.pages.len() > 1 {
                let up = b.clone();
                line = line
                    .push(
                        button(text("↑").size(12))
                            .padding([2, 8])
                            .on_press(wrap(Msg::Move(up, -1))),
                    )
                    .push(
                        button(text("↓").size(12))
                            .padding([2, 8])
                            .on_press(wrap(Msg::Move(b, 1))),
                    );
            }
            col = col.push(line);
        }
        col = col.push(self.section("Veille"));
        col = col.push(
            self.field(
                "Horloge après inactivité",
                "panorama",
                row![
                    slider(0..=30u32, c.sleep_minutes, |v| wrap(Msg::Sleep(v)))
                        .on_release(wrap(Msg::SleepCommit))
                        .width(260),
                    text(if c.sleep_minutes == 0 {
                        crate::i18n::tr("jamais").to_string()
                    } else {
                        format!("{} min", c.sleep_minutes)
                    })
                    .font(self.font)
                    .size(14),
                ]
                .spacing(12),
            ),
        );
        col = col.push(
            text("Rien ne joue et personne n'a touché l'écran : l'horloge s'affiche, jusqu'au prochain toucher.")
                .font(self.font)
                .size(12)
                .color(Color { a: 0.8, ..dim }),
        );
        col.into()
    }

    /// Sources du widget Vidéo et navigateur.
    fn panorama_sources_section(&self) -> Element<'_, UiMsg> {
        let dim = self.ink().dim;
        let c = self.panorama_cfg();
        let mut col = column![
            self.section("Sources vidéo"),
            text(
                "Le widget Vidéo ouvre ces services dans votre navigateur (fenêtre d'application, \
                 votre profil habituel : déjà connecté), posée exactement dans sa zone. Les \
                 services protégés (Netflix, Disney+…) demandent un navigateur Chromium avec \
                 Widevine ; sous Linux, ils sont limités à 720p.",
            )
            .font(self.font)
            .size(12)
            .color(dim),
        ]
        .spacing(8);
        for (i, s) in c.sources.iter().enumerate() {
            col = col.push(
                row![
                    text(s.name.clone()).font(self.font).size(13).width(160),
                    text(s.url.clone())
                        .font(self.font)
                        .size(12)
                        .color(dim)
                        .width(Length::Fill),
                    self.delete_button(false, wrap(Msg::SourceRemove(i))),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }
        let (name, url) = &self.panorama.draft;
        col = col.push(
            row![
                iced::widget::text_input(crate::i18n::tr("Nom"), name)
                    .on_input(|v| wrap(Msg::SourceName(v)))
                    .size(13)
                    .width(160),
                iced::widget::text_input("https://…", url)
                    .on_input(|v| wrap(Msg::SourceUrl(v)))
                    .on_submit(wrap(Msg::SourceAdd))
                    .size(13),
                button(text("Ajouter").font(self.font).size(12))
                    .padding([4, 10])
                    .on_press_maybe(
                        (url.starts_with("https://") || url.starts_with("http://"))
                            .then(|| wrap(Msg::SourceAdd))
                    ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
        col = col.push(
            button(text("Sources par défaut").font(self.font).size(12))
                .padding([4, 10])
                .on_press(wrap(Msg::SourcesReset)),
        );
        let found = crate::panorama::video::browser(&c.browser).map(|(cmd, _)| cmd);
        col = col.push(
            self.field(
                "Navigateur",
                "panorama",
                iced::widget::text_input(
                    found
                        .as_deref()
                        .unwrap_or("brave, chromium, google-chrome-stable…"),
                    &self
                        .panorama
                        .browser_draft
                        .clone()
                        .unwrap_or_else(|| c.browser.clone()),
                )
                .on_input(|v| wrap(Msg::Browser(v)))
                .on_submit(wrap(Msg::BrowserCommit))
                .size(13)
                .width(320),
            ),
        );
        col = col.push(
            text(match found {
                Some(cmd) => crate::tr!(
                    "Navigateur utilisé : {cmd} (vide : détection automatique).",
                    cmd = cmd
                ),
                None => crate::i18n::tr(
                    "Aucun navigateur Chromium trouvé : installez Brave, Chromium ou Chrome.",
                )
                .to_string(),
            })
            .font(self.font)
            .size(12)
            .color(dim),
        );
        col.into()
    }

    pub(in crate::ui) fn panorama_settings_update(&mut self, m: Msg) -> Task<UiMsg> {
        match m {
            m @ (Msg::Preset(_)
            | Msg::DesignText(..)
            | Msg::DesignApply(_)
            | Msg::DesignChoice(..)
            | Msg::DesignNumber(..)
            | Msg::DesignFlag(..)
            | Msg::DesignCommit
            | Msg::TileSelect(_)
            | Msg::TileWidget(_)
            | Msg::TileNumber(..)
            | Msg::TileAdd
            | Msg::TileRemove
            | Msg::ShortcutText(..)
            | Msg::ShortcutAdd
            | Msg::ShortcutRemove(_)) => self.panorama_design_update(m),
            Msg::Layout(l) => {
                let mut c = self.panorama_cfg();
                c.layout = l;
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::PaneWidget(i, w) => {
                let mut c = self.panorama_cfg();
                while c.panes.len() <= i {
                    c.panes.push("media".into());
                }
                c.panes[i] = w;
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::SourceName(v) => {
                self.panorama.draft.0 = v;
                Task::none()
            }
            Msg::SourceUrl(v) => {
                self.panorama.draft.1 = v.trim().to_string();
                Task::none()
            }
            Msg::SourceAdd => {
                let (name, url) = std::mem::take(&mut self.panorama.draft);
                if !(url.starts_with("https://") || url.starts_with("http://")) {
                    self.panorama.draft = (name, url);
                    return Task::none();
                }
                let mut c = self.panorama_cfg();
                c.sources.push(crate::panorama::config::Source {
                    name: if name.trim().is_empty() {
                        crate::panorama::video::host(&url)
                    } else {
                        name.trim().to_string()
                    },
                    url,
                });
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::SourceRemove(i) => {
                let mut c = self.panorama_cfg();
                if i < c.sources.len() {
                    c.sources.remove(i);
                }
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::SourcesReset => {
                let mut c = self.panorama_cfg();
                c.sources = crate::panorama::config::default_sources();
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::Browser(v) => {
                self.panorama.browser_draft = Some(v);
                Task::none()
            }
            Msg::BrowserCommit => {
                let Some(v) = self.panorama.browser_draft.take() else {
                    return Task::none();
                };
                let mut c = self.panorama_cfg();
                c.browser = v.trim().to_string();
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::Role(name, on) => {
                let mut c = self.panorama_cfg();
                let screen = self
                    .panorama
                    .screens
                    .iter()
                    .find(|s| s.name == name)
                    .cloned();
                let key = screen
                    .as_ref()
                    .map(|s| s.edid_id())
                    .filter(|k| !k.is_empty())
                    .unwrap_or_else(|| name.clone());
                if on {
                    c.declined.retain(|d| d != &key);
                    if !c.screens.contains(&key) {
                        c.screens.push(key);
                    }
                } else {
                    c.screens
                        .retain(|e| e != &name && screen.as_ref().is_none_or(|s| !s.matches(e)));
                    self.panorama.hidden.remove(&name);
                }
                // Le tactile suit le premier écran Panorama (aucun : Hyprland choisit).
                let target = if on {
                    Some(name)
                } else {
                    self.panorama
                        .screens
                        .iter()
                        .find(|s| c.has(s))
                        .map(|s| s.name.clone())
                };
                let touch = if self.panorama.touch.is_empty() {
                    Task::none()
                } else {
                    let path = self.config_path.clone();
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                crate::panorama::map_touch(&path, target.as_deref())
                            })
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                        },
                        |r| UiMsg::Panorama(super::super::panorama::PMsg::Touch(r)),
                    )
                };
                self.settings.panorama = Some(c);
                Task::batch([self.commit(), touch])
            }
            Msg::Page(id, on) => {
                let mut c = self.panorama_cfg();
                if on {
                    if !c.pages.contains(&id) {
                        c.pages.push(id);
                    }
                } else if c.pages.len() > 1 {
                    c.pages.retain(|p| p != &id);
                }
                self.panorama.page.clear();
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::Move(id, d) => {
                let mut c = self.panorama_cfg();
                if let Some(i) = c.pages.iter().position(|p| p == &id) {
                    let j = (i as i64 + d as i64).clamp(0, c.pages.len() as i64 - 1) as usize;
                    c.pages.swap(i, j);
                }
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::Sleep(v) => {
                let mut c = self.panorama_cfg();
                c.sleep_minutes = v;
                self.settings.panorama = Some(c);
                Task::none()
            }
            Msg::SleepCommit => self.commit(),
            Msg::Detect => self.panorama_command("detect"),
            Msg::Preview(size) => self.panorama_update(super::super::panorama::PMsg::Preview(size)),
            Msg::Toggle(name) => {
                self.panorama_update(super::super::panorama::PMsg::Toggle(Some(name)))
            }
        }
    }
}
