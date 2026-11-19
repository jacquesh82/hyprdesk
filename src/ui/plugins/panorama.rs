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
    /// Page (indice), ±1 dans l'ordre.
    PageMove(usize, i32),
    /// Nouvelle page : vide (une horloge), ou Système (graphiques).
    PageAdd(bool),
    PageRemove(usize),
    PageName(usize, String),
    PageNameCommit(usize),
    /// Composer la vue par défaut (`None`) ou une page.
    ComposePage(Option<usize>),
    Sleep(u32),
    SleepCommit,
    /// Relit les écrans et refait les propositions.
    Detect,
    /// Afficher ou escamoter le Panorama de cet écran.
    Toggle(String),
    /// Fenêtre d'aperçu au format donné.
    Preview((u32, u32)),
    Layout(crate::panorama::config::Layout),
    /// Part de la zone 1 d'une disposition (1×2, 1×3).
    First(crate::panorama::config::Layout, crate::zones::First),
    /// Disposition dont on règle les zones (active ou non).
    EditLayout(crate::panorama::config::Layout),
    /// Ouvre l'onglet Composition.
    OpenComposition,
    /// Zone, widget.
    PaneWidget(crate::panorama::config::Layout, usize, String),
    SourceName(String),
    SourceUrl(String),
    SourceAdd,
    SourceRemove(usize),
    /// Source, ±1 dans l'ordre.
    SourceMove(usize, i32),
    SourcesReset,
    /// Liste réglée : vidéo, musique, radio.
    SourcesKind(&'static str),
    /// Affichage d'une source dans le Panorama : auto, on, off.
    SourceShow(usize, &'static str),
    /// Ouvre la source dans le navigateur, pour s'y connecter.
    SourceLogin(usize),
    /// Micro de LYRA vocale (nœud PipeWire ; vide : par défaut).
    VoiceMic(String),
    /// Halo de conversation : écrans, couleur.
    VoiceHalo(&'static str),
    VoiceHaloColor(&'static str),
    /// Ambiance de la barre « flux vocal » (webradio).
    FluxAmbiance(&'static str),
    /// Service de l'onglet Webradio (profil Musique).
    Webradio(String),
    /// Raccourci, ±1 dans l'ordre.
    ShortcutMove(usize, i32),
    /// Icônes dessinées par SmartUI : (sources | shortcuts, toutes).
    DrawIcons(&'static str, bool),
    /// Fin de `hyprdesk panorama icons` : compte rendu.
    IconsDone(Result<String, String>),
    Browser(String),
    BrowserCommit,
    Preset(String),
    DesignText(String, String),
    DesignApply(String),
    /// Choisir le fichier du réglage (image de fond) dans un explorateur.
    DesignBrowse(String),
    DesignBrowsed(String, Option<std::path::PathBuf>),
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
        Layout::Split2 => "1×2 : deux zones côte à côte",
        Layout::Split3 => "1×3 : trois zones côte à côte",
        Layout::Grid4 => "2×2 : quatre zones",
        Layout::Grid9 => "3×3 : neuf zones",
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
        // Chaque disposition garde ses réglages : on règle celle qu'on veut,
        // active ou non.
        let edit = self.panorama.edit_layout.unwrap_or(c.layout);
        let mut seg = row![
            text(crate::i18n::tr("Régler"))
                .font(self.font)
                .size(13)
                .width(80)
        ]
        .spacing(6)
        .align_y(Alignment::Center);
        for l in Layout::ALL {
            let label = if l == c.layout {
                format!("{} ●", l.short())
            } else {
                l.short().to_string()
            };
            seg = seg.push(
                button(text(label).font(self.font).size(12))
                    .padding([4, 10])
                    .style(if l == edit {
                        button::primary
                    } else {
                        button::secondary
                    })
                    .on_press(wrap(Msg::EditLayout(l))),
            );
        }
        col = col.push(seg);
        col = col.push(
            text(crate::tr!(
                "Réglages de « {l} »{active}",
                l = crate::i18n::tr(layout_label(edit)).to_string(),
                active = if edit == c.layout {
                    crate::i18n::tr(" (disposition active)").to_string()
                } else {
                    String::new()
                }
            ))
            .font(self.font)
            .size(13),
        );
        match edit {
            Layout::Single => {
                col = col.push(
                    text("Une page à la fois : les pages et leur ordre sont réglés plus bas (Pages).")
                        .font(self.font)
                        .size(12)
                        .color(dim),
                );
            }
            Layout::Dashboard => {
                col = col.push(
                    row![
                        text(crate::tr!(
                            "Composition libre : {n} zone(s) placée(s) librement.",
                            n = c.design.tiles.len()
                        ))
                        .font(self.font)
                        .size(12)
                        .color(dim),
                        button(text("Ouvrir Composition").font(self.font).size(12))
                            .padding([4, 10])
                            .on_press(wrap(Msg::OpenComposition)),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                );
            }
            l => {
                if l.grid().is_some_and(crate::zones::Grid::has_first) {
                    let cur = c.split_of(l).map(|s| s.first());
                    let mut r = row![
                        text(crate::i18n::tr("Part de la zone 1"))
                            .font(self.font)
                            .size(13)
                            .width(140)
                    ]
                    .spacing(14)
                    .align_y(Alignment::Center);
                    for f in [crate::zones::First::Half, crate::zones::First::Third] {
                        r = r.push(
                            iced::widget::radio(f.id(), f, cur, move |v| wrap(Msg::First(l, v)))
                                .font(self.font)
                                .size(16)
                                .text_size(13),
                        );
                    }
                    col = col.push(r);
                }
                let options: Vec<String> = crate::panorama::PAGES
                    .iter()
                    .map(|(id, _)| id.to_string())
                    .collect();
                for i in 0..l.panes() {
                    let cur = c.pane_of(l, i).to_string();
                    col = col.push(
                        row![
                            text(crate::tr!("Zone {n}", n = i + 1))
                                .font(self.font)
                                .size(13)
                                .width(80),
                            iced::widget::pick_list(options.clone(), Some(cur), move |v| {
                                wrap(Msg::PaneWidget(l, i, v))
                            })
                            .text_size(13),
                            text(crate::i18n::tr(crate::panorama::page_label(
                                c.pane_of(l, i)
                            )))
                            .font(self.font)
                            .size(12)
                            .color(dim),
                        ]
                        .spacing(10)
                        .align_y(Alignment::Center),
                    );
                }
            }
        }
        col = col.push(
            text(
                "Sur l'écran : glisser de bas en haut ouvre le choix de la disposition, de haut en \
                 bas masque le Panorama ; glisser de côté passe de la vue composée aux pages. ◀ ▶ \
                 change le widget d'une zone. En plein écran, la vidéo reste dans sa zone.",
            )
            .font(self.font)
            .size(12)
            .color(dim),
        );
        col = col.push(self.panorama_sources_section());
        col = col.push(self.panorama_voice_section());
        col = col.extend([
            self.section("Pages"),
            text("Glisser d'une page à l'autre ; les points en bas de l'écran y mènent aussi.")
                .font(self.font)
                .size(12)
                .color(dim)
                .into(),
        ]);
        // Vue par défaut, puis chaque page : nom, ordre, composition.
        col = col.push(
            row![
                text(crate::i18n::tr("Vue par défaut")).size(13).width(Length::Fill),
                button(text("Composer").size(12))
                    .padding([2, 10])
                    .on_press(wrap(Msg::ComposePage(None))),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
        let n = c.pages.len();
        for (i, p) in c.pages.iter().enumerate() {
            let name = self
                .panorama
                .page_names
                .get(&i)
                .cloned()
                .unwrap_or_else(|| p.name.clone());
            col = col.push(
                row![
                    iced::widget::text_input("", &name)
                        .on_input(move |v| wrap(Msg::PageName(i, v)))
                        .on_submit(wrap(Msg::PageNameCommit(i)))
                        .size(13)
                        .width(Length::Fill),
                    text(crate::tr!("{n} zone(s)", n = p.tiles.len().to_string())).size(12),
                    self.panorama_move_buttons(i, n, |i, d| wrap(Msg::PageMove(i, d))),
                    button(text("Composer").size(12))
                        .padding([2, 10])
                        .on_press(wrap(Msg::ComposePage(Some(i)))),
                    button(text("Retirer").size(12))
                        .padding([2, 10])
                        .on_press(wrap(Msg::PageRemove(i))),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        col = col.push(
            row![
                button(text("Ajouter une page").size(12))
                    .padding([4, 10])
                    .on_press_maybe((n < 12).then(|| wrap(Msg::PageAdd(false)))),
                button(text("Ajouter une page Système").size(12))
                    .padding([4, 10])
                    .on_press_maybe((n < 12).then(|| wrap(Msg::PageAdd(true)))),
            ]
            .spacing(8),
        );
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
    /// Vignette d'une icône : chemin (SmartUI) ou nom d'icône déjà chargé.
    pub(super) fn panorama_icon_thumb(&self, icon: &str, size: f32) -> Element<'_, UiMsg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let p = std::path::Path::new(icon);
        let e: Option<Element<'_, UiMsg>> = if p.is_absolute() && icon.ends_with(".svg") {
            Some(
                iced::widget::svg(iced::widget::svg::Handle::from_path(p))
                    .width(size)
                    .height(size)
                    .into(),
            )
        } else if p.is_absolute() {
            Some(
                iced::widget::image(iced::widget::image::Handle::from_path(p))
                    .width(size)
                    .height(size)
                    .into(),
            )
        } else if icon.is_empty() {
            None
        } else {
            self.icon_named(icon, size, fg)
        };
        e.unwrap_or_else(|| iced::widget::Space::new().width(size).height(size).into())
    }

    /// Boutons ↑ ↓ d'un élément d'une liste ordonnée.
    pub(super) fn panorama_move_buttons(
        &self,
        i: usize,
        n: usize,
        msg: impl Fn(usize, i32) -> UiMsg,
    ) -> Element<'_, UiMsg> {
        row![
            button(text("↑").size(12))
                .padding([2, 7])
                .on_press_maybe((i > 0).then(|| msg(i, -1))),
            button(text("↓").size(12))
                .padding([2, 7])
                .on_press_maybe((i + 1 < n).then(|| msg(i, 1))),
        ]
        .spacing(4)
        .into()
    }

    /// « Dessiner les icônes » et « Tout redessiner » (SmartUI, par LYRA).
    pub(super) fn panorama_icons_buttons(&self, what: &'static str) -> Element<'_, UiMsg> {
        let idle = self.panorama.icons_job.is_none();
        row![
            button(
                text("Dessiner les icônes (SmartUI)")
                    .font(self.font)
                    .size(12)
            )
            .padding([4, 10])
            .on_press_maybe(idle.then(|| wrap(Msg::DrawIcons(what, false)))),
            button(text("Tout redessiner").font(self.font).size(12))
                .padding([4, 10])
                .style(button::secondary)
                .on_press_maybe(idle.then(|| wrap(Msg::DrawIcons(what, true)))),
        ]
        .spacing(8)
        .into()
    }

    /// Génération en cours, ou son compte rendu.
    pub(super) fn panorama_icons_status(&self) -> Element<'_, UiMsg> {
        let dim = self.ink().dim;
        let t: String = if self.panorama.icons_job.is_some() {
            crate::i18n::tr(
                "SmartUI dessine les icônes (fournisseur IA de SmartUI, LYRA par défaut) ; elles \
                 apparaissent une à une.",
            )
            .into()
        } else {
            match &self.panorama.icons_report {
                Some(Ok(r)) => r.clone(),
                Some(Err(e)) => e.clone(),
                None => crate::i18n::tr(
                    "Sans icône, la tuile montre l'initiale. SmartUI dessine les icônes \
                     manquantes dans un même style ; LYRA peut aussi le faire sur demande.",
                )
                .into(),
            }
        };
        text(t).font(self.font).size(12).color(dim).into()
    }

    /// LYRA vocale : marche, micro, essai de la voix.
    fn panorama_voice_section(&self) -> Element<'_, UiMsg> {
        use crate::ui::panorama::PMsg;
        let dim = self.ink().dim;
        let c = self.panorama_cfg();
        let mics = &self.panorama.voice_mics;
        // « Par défaut », puis chaque micro de PipeWire.
        let mut options = vec![Mic(String::new(), crate::i18n::tr("Micro par défaut").to_string())];
        options.extend(mics.iter().map(|(n, l, d)| {
            Mic(
                n.clone(),
                if *d {
                    crate::tr!("{l} (par défaut)", l = l.clone())
                } else {
                    l.clone()
                },
            )
        }));
        let current = options.iter().find(|m| m.0 == c.voice_mic).cloned().or_else(|| {
            (!c.voice_mic.is_empty()).then(|| Mic(c.voice_mic.clone(), c.voice_mic.clone()))
        });
        column![
            self.section("LYRA vocale"),
            text(
                "Dites « Lyra » puis votre demande : LYRA répond à voix haute (Piper), en français \
                 ou en anglais. Écoute et voix restent sur cette machine (Vosk, hors ligne). Le \
                 bouton micro du Panorama l'active aussi.",
            )
            .font(self.font)
            .size(12)
            .color(dim),
            toggler(c.voice)
                .label(crate::i18n::tr("Écouter le mot « Lyra »"))
                .text_size(13)
                .on_toggle(|_| UiMsg::Panorama(PMsg::VoiceToggle)),
            row![
                text(crate::i18n::tr("Micro")).font(self.font).size(13).width(80),
                iced::widget::pick_list(options, current, |m: Mic| wrap(Msg::VoiceMic(m.0)))
                    .text_size(13)
                    .width(Length::Fill),
                button(text("Actualiser").font(self.font).size(12))
                    .padding([4, 10])
                    .on_press(UiMsg::Panorama(PMsg::MicsRefresh)),
                button(text("Tester la voix").font(self.font).size(12))
                    .padding([4, 10])
                    .on_press_maybe(
                        crate::voice::ready().then_some(UiMsg::Panorama(PMsg::VoiceTest))
                    ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            text(crate::i18n::tr("Halo de conversation (« Lyra » entendu, réflexion, réponse)"))
                .font(self.font)
                .size(13),
            {
                let cur = Some(match c.voice_halo.as_str() {
                    "panorama" => "panorama",
                    "off" => "off",
                    _ => "screens",
                });
                let mut r = row![].spacing(14);
                for (id, label) in [
                    ("screens", "Tous les écrans"),
                    ("panorama", "Panorama"),
                    ("off", "Aucun"),
                ] {
                    r = r.push(
                        iced::widget::radio(crate::i18n::tr(label), id, cur, |v: &'static str| {
                            wrap(Msg::VoiceHalo(v))
                        })
                        .text_size(13),
                    );
                }
                r
            },
            {
                let cur = Some(if c.voice_halo_color == "accent" { "accent" } else { "lyra" });
                let mut r = row![].spacing(14);
                for (id, label) in [("lyra", "Violet-bleu de LYRA"), ("accent", "Accent du thème")] {
                    r = r.push(
                        iced::widget::radio(crate::i18n::tr(label), id, cur, |v: &'static str| {
                            wrap(Msg::VoiceHaloColor(v))
                        })
                        .text_size(13),
                    );
                }
                r
            },
        ]
        .spacing(8)
        .into()
    }

    fn panorama_sources_section(&self) -> Element<'_, UiMsg> {
        let dim = self.ink().dim;
        let c = self.panorama_cfg();
        let kind = match self.panorama.sources_kind {
            "music" | "radio" => self.panorama.sources_kind,
            _ => "video",
        };
        let mut kinds = row![].spacing(6);
        for (id, label) in [("video", "Vidéo"), ("music", "Musique"), ("radio", "Webradio")] {
            let on = kind == id;
            kinds = kinds.push(
                button(text(crate::i18n::tr(label)).font(self.font).size(12))
                    .padding([4, 12])
                    .style(move |th, st| {
                        if on {
                            button::primary(th, st)
                        } else {
                            button::secondary(th, st)
                        }
                    })
                    .on_press(wrap(Msg::SourcesKind(id))),
            );
        }
        let mut col = column![
            self.section("Sources du lecteur"),
            kinds,
            text(
                "Vidéo et Musique s'ouvrent dans votre navigateur (fenêtre d'application, votre \
                 profil habituel), posées dans la zone du lecteur ; Webradio : les flux des radios \
                 favorites. Le Panorama ne montre que les plateformes sans compte et celles où vous \
                 êtes connecté (lu dans le navigateur) ; « Se connecter » ouvre le service pour le \
                 faire. Les services protégés (Netflix, Disney+…) demandent un navigateur Chromium \
                 avec Widevine ; sous Linux, ils sont limités à 720p.",
            )
            .font(self.font)
            .size(12)
            .color(dim),
        ]
        .spacing(8);
        let list = match kind {
            "music" => &c.music_sources,
            "radio" => &c.radios,
            _ => &c.sources,
        };
        let n = list.len();
        for (i, s) in list.iter().enumerate() {
            let signed = self.panorama.signed.get(&s.url).copied().flatten();
            let state = if !crate::panorama::auth::needs_account(&s.url) {
                crate::i18n::tr("sans compte")
            } else {
                match signed {
                    Some(true) => crate::i18n::tr("connecté"),
                    Some(false) => crate::i18n::tr("non connecté"),
                    None => crate::i18n::tr("compte"),
                }
            };
            let current = Some(Show(match s.show.as_str() {
                "on" => "on",
                "off" => "off",
                _ => "auto",
            }));
            let shows = vec![Show("auto"), Show("on"), Show("off")];
            let mut line = row![
                self.panorama_icon_thumb(&s.icon, 24.0),
                self.panorama_move_buttons(i, n, |i, d| wrap(Msg::SourceMove(i, d))),
                column![
                    text(s.name.clone()).font(self.font).size(13),
                    text(format!("{} · {state}", s.url)).font(self.font).size(11).color(dim),
                ]
                .width(Length::Fill),
                iced::widget::pick_list(shows, current, move |v: Show| wrap(Msg::SourceShow(i, v.0)))
                    .text_size(12),
            ]
            .spacing(10)
            .align_y(Alignment::Center);
            if crate::panorama::auth::needs_account(&s.url) && signed != Some(true) {
                line = line.push(
                    button(text("Se connecter").font(self.font).size(12))
                        .padding([4, 10])
                        .on_press(wrap(Msg::SourceLogin(i))),
                );
            }
            col = col.push(line.push(self.delete_button(false, wrap(Msg::SourceRemove(i)))));
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
            row![
                button(text("Liste par défaut").font(self.font).size(12))
                    .padding([4, 10])
                    .on_press(wrap(Msg::SourcesReset)),
                self.panorama_icons_buttons("sources"),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );
        col = col.push(self.panorama_icons_status());
        // Webradio du profil Musique : annuaire Radio Browser dans l'onglet,
        // ou un service web ouvert dans le lecteur.
        col = col.push(self.section("Webradio (profil Musique)"));
        let mut services = row![].spacing(14);
        let current = crate::panorama::radio::service(&c.webradio).map(|s| s.0);
        for (id, label, _) in crate::panorama::radio::SERVICES.iter().copied() {
            services = services.push(
                iced::widget::radio(
                    label,
                    id,
                    current,
                    |v: &'static str| wrap(Msg::Webradio(v.into())),
                )
                .text_size(13),
            );
        }
        // Ambiance de la barre « flux vocal » (radio en cours).
        let amb = Some(match c.flux_ambiance.as_str() {
            "sunset" => "sunset",
            "ocean" => "ocean",
            _ => "aurora",
        });
        let mut ambiances = row![
            text(crate::i18n::tr("Ambiance du flux")).font(self.font).size(13).width(140)
        ]
        .spacing(14)
        .align_y(Alignment::Center);
        for (id, label) in [("aurora", "Aurore"), ("sunset", "Coucher de soleil"), ("ocean", "Océan")] {
            ambiances = ambiances.push(
                iced::widget::radio(crate::i18n::tr(label), id, amb, |v: &'static str| {
                    wrap(Msg::FluxAmbiance(v))
                })
                .text_size(13),
            );
        }
        col = col.push(ambiances);
        col = col.push(services).push(
            text(crate::tr!(
                "{n} radios favorites. Radio Browser est un annuaire libre, sans compte : l'onglet Webradio liste vos favoris et la dernière recherche (★ pour garder une station). Demandez à LYRA « mets FIP » ou « une radio jazz » ; elle peut aussi chercher de la musique sur la plateforme active.",
                n = c.radios.len().to_string()
            ))
            .size(12),
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
            | Msg::DesignBrowse(_)
            | Msg::DesignBrowsed(..)
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
            | Msg::ShortcutRemove(_)
            | Msg::ShortcutMove(..)) => self.panorama_design_update(m),
            Msg::Layout(l) => {
                self.panorama.page.clear();
                let mut c = self.panorama_cfg();
                c.layout = l;
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::First(l, f) => {
                let mut c = self.panorama_cfg();
                c.set_first(l, Some(f));
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::EditLayout(l) => {
                self.panorama.edit_layout = Some(l);
                Task::none()
            }
            Msg::OpenComposition => {
                self.subtabs.insert(INFO.id, "composition");
                Task::none()
            }
            Msg::PaneWidget(l, i, w) => {
                let mut c = self.panorama_cfg();
                c.set_pane(l, i, &w);
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
                let kind = self.panorama.sources_kind;
                list_mut(&mut c, kind).push(crate::panorama::config::Source {
                    name: if name.trim().is_empty() {
                        crate::panorama::video::host(&url)
                    } else {
                        name.trim().to_string()
                    },
                    url,
                    icon: String::new(),
                    ..Default::default()
                });
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::SourceMove(i, d) => {
                let mut c = self.panorama_cfg();
                let kind = self.panorama.sources_kind;
                let j = i as i64 + d as i64;
                if i < list_mut(&mut c, kind).len() && j >= 0 && (j as usize) < list_mut(&mut c, kind).len() {
                    list_mut(&mut c, kind).swap(i, j as usize);
                }
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::DrawIcons(what, force) => {
                if self.panorama.icons_job.is_some() {
                    return Task::none();
                }
                self.panorama.icons_job = Some(String::new());
                let config = self.config_path.clone();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let exe = std::env::current_exe().map_err(|e| e.to_string())?;
                            let mut cmd = std::process::Command::new(exe);
                            cmd.arg("--config")
                                .arg(&config)
                                .args(["panorama", "icons", what]);
                            if force {
                                cmd.arg("--force");
                            }
                            let out = cmd.output().map_err(|e| e.to_string())?;
                            let text = |b: &[u8]| String::from_utf8_lossy(b).trim().to_string();
                            let last = text(&out.stdout).lines().last().unwrap_or("").to_string();
                            if out.status.success() {
                                Ok(last)
                            } else {
                                Err(format!("{last}\n{}", text(&out.stderr)).trim().to_string())
                            }
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| wrap(Msg::IconsDone(r)),
                )
            }
            Msg::IconsDone(r) => {
                self.panorama.icons_job = None;
                self.panorama.icons_report = Some(r);
                Task::none()
            }
            Msg::SourceRemove(i) => {
                let mut c = self.panorama_cfg();
                let kind = self.panorama.sources_kind;
                if i < list_mut(&mut c, kind).len() {
                    list_mut(&mut c, kind).remove(i);
                }
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::VoiceHalo(v) => {
                let mut c = self.panorama_cfg();
                c.voice_halo = v.into();
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::FluxAmbiance(v) => {
                let mut c = self.panorama_cfg();
                c.flux_ambiance = v.into();
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::VoiceHaloColor(v) => {
                let mut c = self.panorama_cfg();
                c.voice_halo_color = v.into();
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::VoiceMic(m) => {
                let mut c = self.panorama_cfg();
                c.voice_mic = m;
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::SourcesKind(k) => {
                self.panorama.sources_kind = k;
                Task::none()
            }
            Msg::SourceShow(i, v) => {
                let mut c = self.panorama_cfg();
                let kind = self.panorama.sources_kind;
                if let Some(s) = list_mut(&mut c, kind).get_mut(i) {
                    s.show = v.into();
                }
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::SourceLogin(i) => {
                let mut c = self.panorama_cfg();
                let kind = self.panorama.sources_kind;
                let Some(url) = list_mut(&mut c, kind).get(i).map(|s| s.url.clone()) else {
                    return Task::none();
                };
                // Fenêtre ordinaire du navigateur des sources : même profil,
                // la connexion vaut ensuite pour le Panorama.
                let browser = crate::panorama::video::browser(&c.browser).map(|(cmd, _)| cmd);
                self.panorama.signed_at = None;
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || match browser {
                            Some(b) => {
                                let _ = std::process::Command::new(b)
                                    .arg(&url)
                                    .stdin(std::process::Stdio::null())
                                    .stdout(std::process::Stdio::null())
                                    .stderr(std::process::Stdio::null())
                                    .spawn();
                            }
                            None => {
                                let _ = crate::hypr::exec(&format!("xdg-open '{}'", url.replace('\'', "")));
                            }
                        })
                        .await
                    },
                    |_| UiMsg::Noop,
                )
            }
            Msg::Webradio(id) => {
                let mut c = self.panorama_cfg();
                c.webradio = id;
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::SourcesReset => {
                let mut c = self.panorama_cfg();
                let kind = self.panorama.sources_kind;
                *list_mut(&mut c, kind) = match kind {
                    "music" => crate::panorama::config::default_music_sources(),
                    "radio" => crate::panorama::radio::default_radios(),
                    _ => crate::panorama::config::default_sources(),
                };
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
            Msg::PageMove(i, d) => {
                let mut c = self.panorama_cfg();
                let j = i as i64 + d as i64;
                if i < c.pages.len() && j >= 0 && (j as usize) < c.pages.len() {
                    c.pages.swap(i, j as usize);
                }
                self.panorama.page.clear();
                self.panorama.page_names.clear();
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::PageAdd(system) => {
                let mut c = self.panorama_cfg();
                let mut p = if system {
                    crate::panorama::design::Page::system()
                } else {
                    crate::panorama::design::Page::single("clock")
                };
                p.name = if system {
                    crate::i18n::tr("Système").to_string()
                } else {
                    crate::tr!("Page {n}", n = (c.pages.len() + 2).to_string())
                };
                c.pages.push(p);
                self.panorama.page_names.clear();
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::PageRemove(i) => {
                let mut c = self.panorama_cfg();
                if i < c.pages.len() {
                    c.pages.remove(i);
                }
                if self.panorama.edit_board == Some(i) {
                    self.panorama.edit_board = None;
                }
                self.panorama.page.clear();
                self.panorama.page_names.clear();
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::PageName(i, v) => {
                self.panorama.page_names.insert(i, v);
                Task::none()
            }
            Msg::PageNameCommit(i) => {
                let Some(v) = self.panorama.page_names.remove(&i) else {
                    return Task::none();
                };
                let mut c = self.panorama_cfg();
                match c.pages.get_mut(i) {
                    Some(p) if !v.trim().is_empty() => p.name = v.trim().to_string(),
                    _ => return Task::none(),
                }
                self.settings.panorama = Some(c);
                self.commit()
            }
            Msg::ComposePage(board) => {
                self.panorama.edit_board = board;
                self.panorama.selected_tile = 0;
                self.subtabs.insert(INFO.id, "composition");
                Task::none()
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

/// Liste de sources réglée : vidéo, musique ou radio.
fn list_mut<'a>(
    c: &'a mut crate::panorama::Config,
    kind: &str,
) -> &'a mut Vec<crate::panorama::config::Source> {
    match kind {
        "music" => &mut c.music_sources,
        "radio" => &mut c.radios,
        _ => &mut c.sources,
    }
}

/// Affichage d'une source dans le Panorama (liste des Réglages).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Show(&'static str);

impl std::fmt::Display for Show {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&crate::i18n::tr(match self.0 {
            "on" => "Toujours",
            "off" => "Masquée",
            _ => "Auto",
        }))
    }
}

/// Micro proposé : (nœud PipeWire, libellé).
#[derive(Debug, Clone, PartialEq, Eq)]
struct Mic(String, String);

impl std::fmt::Display for Mic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.1)
    }
}
