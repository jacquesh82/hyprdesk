//! Paquet intégré « Thème » : ce qui se voit. Couleurs, fonds d'écran, effet
//! de chaque endroit (opacité, flou), mode du bureau, voile du verrouillage ;
//! thèmes livrés et « Mes thèmes » (archives de thème, `crate::theme`).
//!
//! Le thème s'exporte seul ou avec tout le setup (paquet Partage).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::i18n::text;
use iced::widget::{Space, button, column, container, radio, row, slider, text_input, toggler};
use iced::{Alignment, Border, Color, Element, Length, Task};

use super::super::panel::{PanelMsg, SWATCHES};
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::config::{Effect, Mode, Place, Rgba};
use crate::theme::{PRESETS, Saved};

pub const INFO: PluginInfo = PluginInfo {
    id: "theme",
    label: "Thème",
    icon: "preferences-desktop-theme",
    summary: "Couleurs, fonds d'écran et effets de chaque endroit ; thèmes livrés et enregistrés.",
    builtin: true,
};

/// Onglets du paquet.
pub const TABS: &[super::SubTab] = &[
    ("themes", "Thèmes"),
    ("couleurs", "Couleurs"),
    ("texte", "Texte"),
    ("effets", "Effets"),
];

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Theme(m)))
}

/// Couleur réglable du thème.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Slot {
    Fg,
    Bg,
    Accent,
    Background,
}

impl Slot {
    const ALL: [Slot; 4] = [Slot::Fg, Slot::Bg, Slot::Accent, Slot::Background];

    fn label(self) -> &'static str {
        match self {
            Slot::Fg => "Texte",
            Slot::Bg => "Surfaces",
            Slot::Accent => "Accent",
            Slot::Background => crate::i18n::tr("Fond sans image"),
        }
    }

    fn key(self) -> &'static str {
        match self {
            Slot::Fg => "theme.fg",
            Slot::Bg => "theme.bg",
            Slot::Accent => "theme.accent",
            Slot::Background => "background",
        }
    }

    /// Couleurs proposées pour cet emplacement.
    fn swatches(self) -> Vec<Rgba> {
        match self {
            Slot::Fg => vec![
                Rgba(0xf2, 0xf4, 0xfc, 0xff),
                Rgba(0xff, 0xff, 0xff, 0xff),
                Rgba(0xfb, 0xf3, 0xe6, 0xff),
                Rgba(0x1f, 0x23, 0x30, 0xff),
                Rgba(0x00, 0x00, 0x00, 0xff),
            ],
            Slot::Bg => vec![
                Rgba(0x17, 0x1b, 0x27, 0xff),
                Rgba(0x22, 0x21, 0x2c, 0xff),
                Rgba(0x16, 0x22, 0x1b, 0xff),
                Rgba(0x24, 0x1a, 0x14, 0xff),
                Rgba(0x00, 0x00, 0x00, 0xff),
                Rgba(0xf3, 0xf4, 0xf8, 0xff),
            ],
            Slot::Accent => SWATCHES[..6].to_vec(),
            Slot::Background => vec![
                Rgba(0x10, 0x13, 0x20, 0xff),
                Rgba(0x1a, 0x30, 0x48, 0xff),
                Rgba(0x26, 0x25, 0x46, 0xff),
                Rgba(0x00, 0x00, 0x00, 0xff),
                Rgba(0xe6, 0xe8, 0xf0, 0xff),
            ],
        }
    }
}

#[derive(Default)]
pub struct State {
    /// Nom en cours de saisie (sinon celui du thème en place).
    pub name: Option<String>,
    /// Couleurs en cours de saisie (`#rrggbb`).
    pub hex: BTreeMap<Slot, String>,
    /// « Mes thèmes ».
    pub library: Vec<Saved>,
    /// Image ou pack d'où créer un thème.
    pub scene: String,
    pub report: Option<Result<String, String>>,
    /// Thème dont la suppression attend confirmation.
    pub pending_delete: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Preset(usize),
    Color(Slot, Rgba),
    Hex(Slot, String),
    HexSubmit(Slot),
    Name(String),
    NameSubmit,
    /// Enregistre le thème en place dans « Mes thèmes ».
    Save,
    /// Source d'un nouveau thème : image, pack `.zip` ou dossier.
    Scene(String),
    /// Crée un thème depuis cette scène, dans « Mes thèmes ».
    CreateFromScene,
    Saved(Result<String, String>),
    Library(Vec<Saved>),
    /// Applique un thème enregistré.
    Apply(PathBuf),
    /// Retire un thème de « Mes thèmes » (premier appui : arme ; second : confirme).
    Delete(PathBuf),
    Applied(Result<String, String>),
    Opacity(Place, f32),
    Blur(Place, bool),
    Mode(Mode),
    GlassOpacity(f32),
    LockBlur(bool),
    LockDim(f32),
    /// Fin d'un glissé de curseur : on enregistre.
    Commit,
    OpenWallpapers,
    OpenDir,
}

/// Relit « Mes thèmes ».
pub fn load_library(config_path: PathBuf) -> Task<UiMsg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || crate::theme::library(&config_path))
                .await
                .unwrap_or_default()
        },
        |l| wrap(Msg::Library(l)),
    )
}

impl App {
    pub(in crate::ui) fn theme_page(&self) -> Element<'_, UiMsg> {
        let Some(p) = &self.panel else {
            return text("").into();
        };
        let st = &p.theme;
        let g = &self.cfg.general;
        let th = &g.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let dim = self.ink().dim;
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let font = self.font;

        // Pastille de couleur ; `selected` l'entoure.
        let dot = move |c: Rgba, size: f32, selected: bool| {
            let col = color(c, 1.0);
            container(Space::new().width(size).height(size)).style(move |_| container::Style {
                background: Some(col.into()),
                border: Border::default()
                    .rounded(size / 2.0)
                    .width(if selected { 3 } else { 1 })
                    .color(if selected {
                        accent
                    } else {
                        Color { a: 0.3, ..fg }
                    }),
                ..Default::default()
            })
        };
        // Aperçu d'un thème : ses trois couleurs sur sa surface.
        let preview = move |name: String, f: Rgba, b: Rgba, a: Rgba, current: bool| {
            let (cf, cb, ca) = (color(f, 1.0), color(b, 1.0), color(a, 1.0));
            container(
                column![
                    text(name).font(self.title_font()).size(13).color(cf),
                    row![
                        container(Space::new().width(26).height(6)).style(move |_| {
                            container::Style {
                                background: Some(ca.into()),
                                border: Border::default().rounded(3),
                                ..Default::default()
                            }
                        }),
                        container(Space::new().width(14).height(6)).style(move |_| {
                            container::Style {
                                background: Some(Color { a: 0.5, ..cf }.into()),
                                border: Border::default().rounded(3),
                                ..Default::default()
                            }
                        }),
                    ]
                    .spacing(6),
                ]
                .spacing(8),
            )
            .padding([10, 12])
            .width(124)
            .style(move |_| container::Style {
                background: Some(cb.into()),
                border: Border::default()
                    .rounded(12)
                    .width(if current { 2 } else { 1 })
                    .color(if current {
                        accent
                    } else {
                        Color { a: 0.18, ..fg }
                    }),
                ..Default::default()
            })
        };
        let card_button = |content: Element<'static, UiMsg>, msg: UiMsg| {
            button(content)
                .padding(0)
                .style(|_, _| button::Style::default())
                .on_press(msg)
        };

        // Thème en place.
        let name = st.name.clone().unwrap_or_else(|| th.name.clone());
        let mut current = column![
            row![
                preview(th.name.clone(), th.fg, th.bg, th.accent, true),
                column![
                    text_input(crate::i18n::tr("Nom du thème"), &name)
                        .font(font)
                        .on_input(|v| wrap(Msg::Name(v)))
                        .on_submit(wrap(Msg::NameSubmit))
                        .padding(8)
                        .width(Length::Fill),
                    row![
                        button(text("Enregistrer dans Mes thèmes").font(font).size(13))
                            .padding([6, 14])
                            .on_press_maybe((!name.trim().is_empty()).then(|| wrap(Msg::Save))),
                        text("Exporter : Partage → Thème seul")
                            .font(font)
                            .size(12)
                            .color(dim),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                ]
                .spacing(8)
                .width(Length::Fill),
            ]
            .spacing(16)
            .align_y(Alignment::Center),
        ]
        .spacing(8);
        if let Some(r) = &st.report {
            current = current.push(match r {
                Ok(m) => text(m.clone()).font(font).size(12).color(accent),
                Err(e) => text(e.clone()).font(font).size(12).color(warn),
            });
        }

        // Thèmes livrés.
        let presets = row(PRESETS.iter().enumerate().map(|(i, p)| {
            let on = th.name == p.name && th.fg == p.fg && th.bg == p.bg && th.accent == p.accent;
            card_button(
                preview(p.name.into(), p.fg, p.bg, p.accent, on).into(),
                wrap(Msg::Preset(i)),
            )
            .into()
        }))
        .spacing(10)
        .wrap();

        // Mes thèmes.
        let library: Element<'_, UiMsg> = if st.library.is_empty() {
            text(
                "Aucun pour l'instant : enregistrez le thème en place, ou importez une archive \
                 de thème (Partage).",
            )
            .font(font)
            .size(12)
            .color(dim)
            .into()
        } else {
            row(st.library.iter().map(|s| {
                let (f, b, a) = s.summary.colors();
                let armed = st.pending_delete.as_ref() == Some(&s.path);
                column![
                    card_button(
                        preview(s.summary.name.clone(), f, b, a, false).into(),
                        wrap(Msg::Apply(s.path.clone())),
                    ),
                    self.delete_button(armed, wrap(Msg::Delete(s.path.clone()))),
                ]
                .spacing(4)
                .align_x(iced::Alignment::Center)
                .into()
            }))
            .spacing(10)
            .wrap()
            .into()
        };

        // Couleurs.
        let mut colors = column![].spacing(12);
        for slot in Slot::ALL {
            let value = match slot {
                Slot::Fg => th.fg,
                Slot::Bg => th.bg,
                Slot::Accent => th.accent,
                Slot::Background => g.background,
            };
            let draft = st
                .hex
                .get(&slot)
                .cloned()
                .unwrap_or_else(|| crate::settings::hex(value));
            let swatches = row(slot.swatches().into_iter().map(|c| {
                button(dot(c, 26.0, c == value))
                    .padding(0)
                    .style(|_, _| button::Style::default())
                    .on_press(wrap(Msg::Color(slot, c)))
                    .into()
            }))
            .spacing(8);
            colors = colors.push(
                self.field(
                    slot.label(),
                    slot.key(),
                    row![
                        swatches,
                        Space::new().width(Length::Fill),
                        dot(value, 22.0, false),
                        text_input(crate::i18n::tr("#rrggbb"), &draft)
                            .font(font)
                            .size(13)
                            .on_input(move |v| wrap(Msg::Hex(slot, v)))
                            .on_submit(wrap(Msg::HexSubmit(slot)))
                            .padding(6)
                            .width(96),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                ),
            );
        }

        // Effets de chaque endroit.
        let percent = |v: f32| {
            text(format!("{} %", (v * 100.0).round() as i32))
                .font(font)
                .size(13)
                .width(48)
                .wrapping(iced::widget::text::Wrapping::None)
        };
        let mut effects = column![].spacing(12);
        for place in Place::ALL {
            let e = th.effect(place);
            let mut line = row![
                slider(0..=100u32, (e.opacity * 100.0).round() as u32, move |v| {
                    wrap(Msg::Opacity(place, v as f32 / 100.0))
                })
                .on_release(wrap(Msg::Commit))
                .width(Length::Fill),
                percent(e.opacity),
            ]
            .spacing(12)
            .align_y(Alignment::Center);
            if place.blurrable() {
                line = line.push(
                    toggler(e.blur)
                        .label(crate::i18n::tr("Flou"))
                        .font(font)
                        .on_toggle(move |v| wrap(Msg::Blur(place, v))),
                );
            }
            effects = effects.push(self.field(
                place.label(),
                &format!("theme.effects.{}", place.as_str()),
                line,
            ));
        }
        effects = effects.push(
            self.field(
                "Bureau affiché",
                "mode",
                row![
                    radio(
                        crate::i18n::tr("Fond d'écran"),
                        Mode::Opaque,
                        Some(g.mode),
                        |v| wrap(Msg::Mode(v))
                    )
                    .font(font)
                    .size(16),
                    radio(crate::i18n::tr("Verre"), Mode::Glass, Some(g.mode), |v| {
                        wrap(Msg::Mode(v))
                    })
                    .font(font)
                    .size(16),
                ]
                .spacing(16),
            ),
        );
        if g.mode == Mode::Glass {
            effects = effects.push(
                self.field(
                    "Voile du verre",
                    "glass_opacity",
                    row![
                        slider(0..=100u32, (g.glass_opacity * 100.0).round() as u32, |v| {
                            wrap(Msg::GlassOpacity(v as f32 / 100.0))
                        })
                        .on_release(wrap(Msg::Commit))
                        .width(Length::Fill),
                        percent(g.glass_opacity),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center),
                ),
            );
        }
        effects = effects.push(
            self.field(
                "Verrouillage",
                "lock_dim",
                row![
                    slider(0..=100u32, (g.lock.dim * 100.0).round() as u32, |v| wrap(
                        Msg::LockDim(v as f32 / 100.0)
                    ))
                    .on_release(wrap(Msg::Commit))
                    .width(Length::Fill),
                    percent(g.lock.dim),
                    toggler(g.lock.blur)
                        .label(crate::i18n::tr("Flou"))
                        .font(font)
                        .on_toggle(|v| wrap(Msg::LockBlur(v))),
                ]
                .spacing(12)
                .align_y(Alignment::Center),
            ),
        );

        // Fonds d'écran : réglés dans leur paquet, ils font partie du thème.
        let walls = self.cfg.wallpapers.len();
        let wallpapers = row![
            text(match walls {
                0 => crate::i18n::tr("Aucun fond d'écran réglé : le fond sans image s'applique.")
                    .to_string(),
                1 => crate::i18n::tr("1 règle de fond d'écran.").to_string(),
                n => crate::tr!("{n} règles de fond d'écran.", n = n),
            })
            .font(font)
            .size(13)
            .width(Length::Fill),
            button(text("Choisir les fonds d'écran").font(font).size(12))
                .padding([4, 10])
                .style(button::secondary)
                .on_press(wrap(Msg::OpenWallpapers)),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        match self.subtab(INFO.id) {
            "couleurs" => {
                // Accessibilité : contraste du texte sur les surfaces (WCAG AA : 4,5:1).
                let ratio = crate::theme::contrast(th.fg, th.bg);
                let note = if ratio < 4.5 {
                    text(crate::tr!(
                        "Contraste faible : {r}:1 (au moins 4,5:1 conseillé ; thème « Contraste élevé » : 21:1).",
                        r = format!("{ratio:.1}")
                    ))
                    .font(font)
                    .size(12)
                    .color(Color::from_rgb8(0xff, 0xb8, 0x6c))
                } else {
                    text(crate::tr!(
                        "Contraste du texte : {r}:1.",
                        r = format!("{ratio:.1}")
                    ))
                    .font(font)
                    .size(12)
                    .color(dim)
                };
                return column![self.section("Couleurs"), colors, note]
                    .spacing(14)
                    .into();
            }
            "texte" => return self.typography(),
            "effets" => {
                return column![
                    self.section("Effets"),
                    text(
                        "Opacité du fond de chaque endroit et flou de ce qu'il y a derrière \
                         (fourni par Hyprland)."
                    )
                    .font(font)
                    .size(12)
                    .color(dim),
                    effects,
                ]
                .spacing(14)
                .into();
            }
            _ => {}
        }
        column![
            self.section("Thème en place"),
            current,
            self.section("Thèmes livrés"),
            text("Leurs couleurs et leurs effets ; les fonds d'écran restent en place.")
                .font(font)
                .size(12)
                .color(dim),
            presets,
            self.section("Mes thèmes"),
            row![
                text(
                    "Appliquer remplace le thème en place (fonds compris) : enregistrez-le \
                     d'abord pour le garder."
                )
                .font(font)
                .size(12)
                .color(dim)
                .width(Length::Fill),
                button(text("Ouvrir le dossier").font(font).size(12))
                    .padding([4, 10])
                    .style(button::secondary)
                    .on_press(wrap(Msg::OpenDir)),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
            library,
            row![
                text_input(
                    crate::i18n::tr("Créer depuis une image, un pack .zip ou un dossier…"),
                    &st.scene
                )
                .font(font)
                .on_input(|v| wrap(Msg::Scene(v)))
                .on_submit(wrap(Msg::CreateFromScene))
                .padding(8)
                .width(Length::Fill),
                button(text("Créer le thème").font(font).size(13))
                    .padding([6, 14])
                    .on_press_maybe(
                        (!st.scene.trim().is_empty()).then(|| wrap(Msg::CreateFromScene))
                    ),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            text(
                "Couleurs tirées de l'image ; la scène s'étale sur tous les écrans, découpée \
                 selon leur disposition, au bureau comme au verrouillage. Un pack multi-écrans \
                 (manifest.json) fournit sa scène maîtresse."
            )
            .font(font)
            .size(12)
            .color(dim),
            self.section("Fonds d'écran"),
            wallpapers,
        ]
        .spacing(14)
        .into()
    }

    /// Effet en place d'un endroit, réglé ici ou non.
    fn theme_effect(&self, place: Place) -> Effect {
        self.settings
            .effects
            .get(&place)
            .copied()
            .unwrap_or_else(|| self.cfg.general.theme.effect(place))
    }

    pub(in crate::ui) fn theme_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::Preset(i) => {
                let Some(p) = PRESETS.get(i) else {
                    return Task::none();
                };
                self.settings.apply_preset(p);
                if let Some(panel) = self.panel.as_mut() {
                    panel.theme.name = None;
                    panel.theme.hex.clear();
                }
                self.commit()
            }
            Msg::Color(slot, c) => {
                if let Some(panel) = self.panel.as_mut() {
                    panel.theme.hex.remove(&slot);
                }
                self.set_theme_color(slot, c)
            }
            Msg::Hex(slot, v) => {
                if let Some(panel) = self.panel.as_mut() {
                    panel.theme.hex.insert(slot, v);
                }
                Task::none()
            }
            Msg::HexSubmit(slot) => {
                let Some(v) = self
                    .panel
                    .as_mut()
                    .and_then(|panel| panel.theme.hex.remove(&slot))
                else {
                    return Task::none();
                };
                match Rgba::parse(v.trim()) {
                    Some(c) => self.set_theme_color(slot, c),
                    None => {
                        if let Some(panel) = self.panel.as_mut() {
                            panel.theme.report = Some(Err(crate::tr!(
                                "« {v} » : couleur illisible (#rrggbb)",
                                v = v
                            )));
                        }
                        Task::none()
                    }
                }
            }
            Msg::Name(v) => {
                if let Some(panel) = self.panel.as_mut() {
                    panel.theme.name = Some(v);
                }
                Task::none()
            }
            Msg::NameSubmit => {
                let Some(name) = self
                    .panel
                    .as_mut()
                    .and_then(|panel| panel.theme.name.take())
                    .map(|n| n.trim().to_string())
                    .filter(|n| !n.is_empty())
                else {
                    return Task::none();
                };
                self.settings.theme_name = Some(name);
                self.commit()
            }
            Msg::Save => {
                let name = self
                    .panel
                    .as_mut()
                    .and_then(|panel| panel.theme.name.take())
                    .unwrap_or_else(|| self.cfg.general.theme.name.clone())
                    .trim()
                    .to_string();
                if name.is_empty() {
                    return Task::none();
                }
                self.settings.theme_name = Some(name.clone());
                let cfg = self.config_path.clone();
                // Enregistrer d'abord les réglages : l'archive part du disque.
                self.commit().chain(Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::theme::save_current(&cfg, &name).map(|p| {
                                crate::tr!(
                                    "« {name} » enregistré : {path}",
                                    name = name,
                                    path = p.display()
                                )
                            })
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    |r| wrap(Msg::Saved(r)),
                ))
            }
            Msg::Scene(v) => {
                if let Some(panel) = self.panel.as_mut() {
                    panel.theme.scene = v;
                }
                Task::none()
            }
            Msg::CreateFromScene => {
                let Some(src) = self
                    .panel
                    .as_ref()
                    .map(|panel| panel.theme.scene.trim().to_string())
                    .filter(|s| !s.is_empty())
                else {
                    return Task::none();
                };
                let src = crate::config::lua::expand_path(&src, std::path::Path::new("/"));
                let cfg = self.config_path.clone();
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::theme::create(&cfg, &src, None).map(|(out, warning)| {
                                let mut m = crate::tr!(
                                    "Thème créé : {out} — à appliquer d'un clic dans Mes thèmes.",
                                    out = out.display()
                                );
                                if let Some(w) = warning {
                                    m += &crate::tr!("\nAttention : {w}", w = w);
                                }
                                m
                            })
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    |r| wrap(Msg::Saved(r)),
                )
            }
            Msg::Saved(r) => {
                let ok = r.is_ok();
                if let Some(panel) = self.panel.as_mut() {
                    panel.theme.report = Some(r);
                }
                if ok {
                    load_library(self.config_path.clone())
                } else {
                    Task::none()
                }
            }
            Msg::Library(l) => {
                if let Some(panel) = self.panel.as_mut() {
                    panel.theme.library = l;
                }
                Task::none()
            }
            Msg::Delete(path) => {
                let Some(panel) = self.panel.as_mut() else {
                    return Task::none();
                };
                if panel.theme.pending_delete.as_ref() != Some(&path) {
                    panel.theme.pending_delete = Some(path);
                    return Task::none();
                }
                panel.theme.pending_delete = None;
                panel.theme.report =
                    Some(crate::theme::retire(&path, &self.config_path).map(|to| {
                        crate::tr!(
                            "Thème retiré, gardé dans {dir}.",
                            dir = to.parent().unwrap_or(&to).display()
                        )
                    }));
                load_library(self.config_path.clone())
            }
            Msg::Apply(path) => {
                let cfg = self.config_path.clone();
                let opts = crate::share::ImportOpts {
                    scope: crate::share::Scope::Theme,
                    ..Default::default()
                };
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::share::import(&path, &cfg, opts).map(|r| {
                                crate::tr!(
                                    "Thème « {name} » appliqué.",
                                    name = r.manifest.theme.map(|t| t.name).unwrap_or_default()
                                )
                            })
                        })
                        .await
                        .map_err(|e| e.to_string())
                        .and_then(|r| r)
                    },
                    |r| wrap(Msg::Applied(r)),
                )
            }
            Msg::Applied(r) => {
                let ok = r.is_ok();
                if let Some(panel) = self.panel.as_mut() {
                    panel.theme.report = Some(r);
                    panel.theme.name = None;
                    panel.theme.hex.clear();
                }
                // Les réglages ont changé sur disque : on les relit.
                if ok {
                    Task::done(UiMsg::ConfigChanged)
                } else {
                    Task::none()
                }
            }
            Msg::Opacity(place, v) => {
                let mut e = self.theme_effect(place);
                e.opacity = v.clamp(0.0, 1.0);
                self.settings.effects.insert(place, e);
                self.apply()
            }
            Msg::Blur(place, v) => {
                let mut e = self.theme_effect(place);
                e.blur = v;
                self.settings.effects.insert(place, e);
                self.commit()
            }
            Msg::Mode(m) => {
                self.settings.mode = Some(m);
                self.commit()
            }
            Msg::GlassOpacity(v) => {
                self.settings.glass_opacity = Some(v.clamp(0.0, 1.0));
                self.apply()
            }
            Msg::LockBlur(v) => {
                self.settings.lock_blur = Some(v);
                self.commit()
            }
            Msg::LockDim(v) => {
                self.settings.lock_dim = Some(v.clamp(0.0, 1.0));
                self.apply()
            }
            Msg::Commit => self.commit(),
            Msg::OpenWallpapers => {
                Task::done(UiMsg::Panel(PanelMsg::Tab(super::wallpaper::INFO.id)))
            }
            Msg::OpenDir => {
                let dir = crate::theme::themes_dir(&self.config_path);
                let _ = std::fs::create_dir_all(&dir);
                // Le bureau reste affiché : la fenêtre s'ouvre dessus, flottante.
                self.run_from_desktop(
                    format!(
                        "xdg-open {}",
                        crate::launch::sh_quote(&dir.display().to_string())
                    ),
                    true,
                )
            }
        }
    }

    fn set_theme_color(&mut self, slot: Slot, c: Rgba) -> Task<UiMsg> {
        let s = &mut self.settings;
        match slot {
            Slot::Fg => s.fg = Some(c),
            Slot::Bg => s.bg = Some(c),
            Slot::Accent => s.accent = Some(c),
            Slot::Background => s.background = Some(c),
        }
        self.commit()
    }
}
