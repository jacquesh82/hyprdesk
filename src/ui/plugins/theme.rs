//! Paquet intégré « Thème » : ce qui se voit. Couleurs, fonds d'écran, effet
//! de chaque endroit (opacité, flou), mode du bureau, voile du verrouillage ;
//! thèmes livrés et « Thèmes personnels » (archives de thème, `crate::theme`).
//!
//! Le thème s'exporte seul ou avec tout le setup (paquet Partage).
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::i18n::text;
use iced::widget::{Space, button, column, container, radio, row, text_input, toggler};
use iced::{Alignment, Border, Color, Element, Length, Task};

use super::super::panel::{PanelMsg, SWATCHES, SliderLook, SliderSpec, beside_slider};
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::config::{Effect, Mode, Place, Rgba};

/// Valeur affichée d'un curseur en pour cent (HIG SEL-08) : « 100 % ».
fn affiche_pourcent(v: f64) -> String {
    format!("{} %", v.round() as i32)
}
use crate::theme::{PRESETS, Saved};
use crate::ui::color_picker::{Picker, PickerMsg, well};

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
    ("transparence", "Transparence"),
    ("composants", "Composants"),
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
    /// Sélecteur de couleur ouvert, et son emplacement (HIG SEL-02).
    pub picker: Option<(Slot, Picker)>,
    /// « Thèmes personnels ».
    pub library: Vec<Saved>,
    /// Packs du moteur de thème (embarqués, installés, `perso`) : les thèmes
    /// livrés (MLG-233).
    pub catalog: Vec<crate::theme_engine::bridge::Entry>,
    /// Image ou pack d'où créer un thème.
    pub scene: String,
    pub report: Option<Result<String, String>>,
    /// Thème dont la suppression attend confirmation.
    pub pending_delete: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Preset(usize),
    /// Pose un pack du moteur (son thème, dans sa variante par défaut).
    Pack(Box<crate::config::Theme>),
    Color(Slot, Rgba),
    Hex(Slot, String),
    HexSubmit(Slot),
    /// Ouvre ou referme le sélecteur de couleur de l'emplacement (HIG SEL-02).
    OpenPicker(Slot),
    Picker(PickerMsg),
    PickerApply,
    PickerCancel,
    Name(String),
    NameSubmit,
    /// Enregistre le thème en place dans « Thèmes personnels ».
    Save,
    /// Source d'un nouveau thème : image, pack `.zip` ou dossier.
    Scene(String),
    /// Crée un thème depuis cette scène, dans « Thèmes personnels ».
    CreateFromScene,
    Saved(Result<String, String>),
    Library(Vec<Saved>, Vec<crate::theme_engine::bridge::Entry>),
    /// Applique un thème enregistré.
    Apply(PathBuf),
    /// Retire un thème de « Thèmes personnels » (premier appui : arme ; second : confirme).
    Delete(PathBuf),
    Applied(Result<String, String>),
    Opacity(Place, f32),
    Blur(Place, bool),
    /// Transparence adaptative d'une surface posée sur le fond (MLG-261).
    Adaptive(Place, bool),
    /// Option d'apparence d'un composant (MLG-279) : nom, valeur, à
    /// enregistrer.
    DesignOpt(String, String, bool),
    /// Transparence réduite (MLG-269).
    ReducedTransparency(bool),
    /// Mouvement réduit (MLG-271).
    ReducedMotion(bool),
    /// Contraste renforcé (MLG-282).
    IncreaseContrast(bool),
    /// Échelle du texte de l'interface (MLG-282).
    TextScale(f32),
    Mode(Mode),
    GlassOpacity(f32),
    LockBlur(bool),
    LockDim(f32),
    /// Fin d'un glissé de curseur : on enregistre.
    Commit,
    OpenWallpapers,
    OpenDir,
}

/// Relit « Thèmes personnels » et les packs du moteur.
pub fn load_library(config_path: PathBuf) -> Task<UiMsg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                (
                    crate::theme::library(&config_path),
                    crate::theme_engine::bridge::catalog(
                        &crate::theme_engine::paths::Roots::from_env(),
                    ),
                )
            })
            .await
            .unwrap_or_default()
        },
        |(l, c)| wrap(Msg::Library(l, c)),
    )
}

/// Thèmes livrés à montrer : les packs du moteur ; tant qu'ils ne sont pas
/// lus, les préréglages (identiques aux packs embarqués).
fn shown_catalog(st: &State) -> Vec<crate::theme_engine::bridge::Entry> {
    if !st.catalog.is_empty() {
        return st.catalog.clone();
    }
    PRESETS
        .iter()
        .map(|p| crate::theme_engine::bridge::Entry {
            id: crate::theme::slug(p.name),
            origin: "bundled",
            variant: "dark".to_string(),
            theme: crate::config::Theme {
                name: p.name.to_string(),
                fg: p.fg,
                bg: p.bg,
                accent: p.accent,
                effects: p.effects(),
            },
            other: None,
        })
        .collect()
}

impl App {
    /// Apparence des composants (MLG-279) : espaces de travail,
    /// réglages rapides, notifications, monitoring, horloge, média,
    /// verrouillage — les options du registre, regroupées.
    fn design_components(&self) -> Element<'_, UiMsg> {
        let font = self.font;
        // Le dock a ses options dans son plugin (Réglages → Dock → Apparence).
        let groups: [(&str, &str); 8] = [
            ("workspaces", "Espaces de travail"),
            ("controls", "Réglages rapides"),
            ("notifications", "Notifications"),
            ("monitoring", "Monitoring"),
            ("clock", "Horloge"),
            ("media", "Média"),
            ("lock", "Verrouillage"),
            ("wallpaper", "Fond d'écran"),
        ];
        let mut col = column![
            self.section("Composants"),
            text(crate::i18n::tr(
                "Apparence de chaque composant du bureau ; aussi réglable par un thème ou par hyprdesk config set <composant>.<option>."
            ))
            .font(font)
            .size(crate::ui::tokens::ts(12.0))
            .color(self.ink().dim),
        ]
        .spacing(14);
        for (component, title) in groups {
            col = col.push(self.design_fields(component, title));
        }
        col = col.push(
            text(crate::i18n::tr("Le dock : Réglages → Dock → Apparence."))
                .font(font)
                .size(crate::ui::tokens::ts(12.0))
                .color(self.ink().dim),
        );
        col.into()
    }

    /// Options du registre `design_options` d'un composant, sous un titre :
    /// Thème → Composants, et la page de son plugin (Dock → Apparence).
    pub(in crate::ui) fn design_fields(
        &self,
        component: &str,
        title: &'static str,
    ) -> iced::widget::Column<'_, UiMsg> {
        use crate::design_options::{Kind, OPTIONS};
        use crate::tiling::OptValue;
        let font = self.font;
        let g = &self.cfg.general;
        let mut col = column![self.section(title)].spacing(14);
        for o in OPTIONS.iter().filter(|o| o.component == component) {
            let name = o.name();
            let field: Element<'_, UiMsg> = match (o.kind, o.value(g)) {
                (Kind::Bool, OptValue::Bool(b)) => {
                    let n = name.clone();
                    toggler(b)
                        .on_toggle(move |v| {
                            wrap(Msg::DesignOpt(
                                n.clone(),
                                if v { "on" } else { "off" }.into(),
                                true,
                            ))
                        })
                        .into()
                }
                (Kind::Choice(list), OptValue::Str(cur)) => {
                    row(list.iter().enumerate().map(|(i, c)| {
                        let n = name.clone();
                        let current = list.iter().position(|x| *x == cur);
                        radio(*c, i, current, move |v| {
                            wrap(Msg::DesignOpt(n.clone(), list[v].to_string(), true))
                        })
                        .font(font)
                        .size(crate::ui::tokens::ts(14.0))
                        .into()
                    }))
                    .spacing(12)
                    .wrap()
                    .into()
                }
                (Kind::Int(lo, hi), OptValue::Num(v)) => {
                    let (n, key) = (name.clone(), name.clone());
                    self.slider(
                        &SliderSpec::new(
                            key,
                            (lo as f64, hi as f64),
                            1.0,
                            move |x| {
                                wrap(Msg::DesignOpt(n.clone(), format!("{}", x.round()), false))
                            },
                            // Après la pose, l'enregistrement du paquet : le
                            // message de relâchement portait la valeur d'avant.
                            wrap(Msg::Commit),
                        ),
                        v,
                        |x| format!("{}", x.round()),
                        SliderLook::fixed(13.0, 220, Alignment::Center),
                    )
                }
                (Kind::Float(lo, hi), OptValue::Num(v)) => {
                    let (n, key) = (name.clone(), name.clone());
                    self.slider(
                        &SliderSpec::new(
                            key,
                            (lo, hi),
                            (hi - lo) / 100.0,
                            move |x| wrap(Msg::DesignOpt(n.clone(), format!("{x:.2}"), false)),
                            wrap(Msg::Commit),
                        ),
                        v,
                        |x| format!("{x:.2}"),
                        SliderLook::fixed(13.0, 220, Alignment::Center),
                    )
                }
                (Kind::Text, OptValue::Str(cur)) => {
                    let (n, c) = (name.clone(), name.clone());
                    let v = cur.clone();
                    iced::widget::text_input(crate::i18n::tr("chemin d'une image"), &cur)
                        .font(font)
                        .size(crate::ui::tokens::ts(13.0))
                        .padding(6)
                        .width(360)
                        .on_input(move |x| wrap(Msg::DesignOpt(n.clone(), x, false)))
                        .on_submit(wrap(Msg::DesignOpt(c, v, true)))
                        .into()
                }
                _ => text("—").into(),
            };
            col = col.push(self.field(o.label, "", field));
        }
        col
    }

    pub(in crate::ui) fn theme_page(&self) -> Element<'_, UiMsg> {
        let Some(p) = &self.panel else {
            return text("").into();
        };
        let st = &p.theme;
        let g = &self.cfg.general;
        let th = &g.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let dim = self.ink().dim;
        let warn = crate::ui::tone::warning();
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
                    text(name)
                        .font(self.title_font())
                        .size(crate::ui::tokens::ts(13.0))
                        .color(cf),
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
                        button(
                            text("Enregistrer dans les thèmes personnels")
                                .font(font)
                                .size(crate::ui::tokens::ts(13.0))
                        )
                        .padding([6, 14])
                        .on_press_maybe((!name.trim().is_empty()).then(|| wrap(Msg::Save))),
                        text("Exporter : Partage → Thème seul")
                            .font(font)
                            .size(crate::ui::tokens::ts(12.0))
                            .color(dim),
                    ]
                    .spacing(12)
                    .align_y(Alignment::Center)
                    .wrap()
                    .vertical_spacing(6),
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
                Ok(m) => text(m.clone())
                    .font(font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(accent),
                Err(e) => text(e.clone())
                    .font(font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(warn),
            });
        }

        // Thèmes livrés : les packs du moteur de thème, qui fait foi.
        let presets = row(shown_catalog(st).into_iter().map(|e| {
            use crate::theme_engine::bridge::same_theme;
            // Variante montrée : celle en place si c'est l'autre, sinon celle
            // par défaut ; l'autre se choisit sous la tuile (MLG-254).
            let (shown, alt) = match e.other {
                Some((_, t)) if same_theme(th, &t) => (t, Some((e.variant, e.theme))),
                other => (e.theme, other),
            };
            let on = same_theme(th, &shown);
            let name = match e.origin {
                "bundled" => shown.name.clone(),
                "user" => crate::tr!("{n} · à vous", n = shown.name),
                _ => crate::tr!("{n} · installé", n = shown.name),
            };
            let card = card_button(
                preview(name, shown.fg, shown.bg, shown.accent, on).into(),
                wrap(Msg::Pack(Box::new(shown))),
            );
            match alt {
                Some((v, t)) => column![
                    card,
                    button(
                        text(crate::i18n::tr(if v == "light" {
                            "Version claire"
                        } else {
                            "Version sombre"
                        }))
                        .font(font)
                        .size(crate::ui::tokens::ts(11.0)),
                    )
                    .padding([2, 8])
                    .style(button::text)
                    .on_press(wrap(Msg::Pack(Box::new(t)))),
                ]
                .spacing(2)
                .align_x(Alignment::Center)
                .into(),
                None => card.into(),
            }
        }))
        .spacing(10)
        .wrap();

        // Thèmes personnels.
        let library: Element<'_, UiMsg> = if st.library.is_empty() {
            text(
                "Aucun pour l'instant : enregistrez le thème en place, ou importez une archive \
                 de thème (Partage).",
            )
            .font(font)
            .size(crate::ui::tokens::ts(12.0))
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
            // Puits de couleur (HIG SEL-02) : la pastille de la couleur en
            // cours ouvre le sélecteur, qui se dessine sous la rangée.
            let open = st.picker.as_ref();
            let is_open = open.is_some_and(|(s, _)| *s == slot);
            let mut line = column![].spacing(8);
            line = line.push(
                row![
                    swatches,
                    Space::new().width(Length::Fill),
                    well(
                        value,
                        22.0,
                        is_open,
                        accent,
                        Color { a: 0.3, ..fg },
                        wrap(Msg::OpenPicker(slot)),
                    ),
                    text_input(crate::i18n::tr("#rrggbb"), &draft)
                        .font(font)
                        .size(crate::ui::tokens::ts(13.0))
                        .on_input(move |v| wrap(Msg::Hex(slot, v)))
                        .on_submit(wrap(Msg::HexSubmit(slot)))
                        .padding(6)
                        .width(96),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
            if let Some((_, pk)) = open.filter(|_| is_open) {
                line = line.push(pk.view(
                    font,
                    fg,
                    dim,
                    |m| wrap(Msg::Picker(m)),
                    wrap(Msg::PickerApply),
                    wrap(Msg::PickerCancel),
                ));
            }
            colors = colors.push(self.field(slot.label(), slot.key(), line));
        }

        // Effets de chaque endroit.
        let percent = |v: f64| format!("{} %", v.round() as i32);
        let mut effects = column![].spacing(12);
        for place in Place::ALL {
            let e = th.effect(place);
            let spec = SliderSpec::new(
                place.as_str(),
                (0.0, 100.0),
                1.0,
                move |v| wrap(Msg::Opacity(place, v.round() as u32 as f32 / 100.0)),
                wrap(Msg::Commit),
            );
            // Les interrupteurs restent sur la ligne du curseur, au-dessus de
            // son échelle (HIG SEL-09).
            let mut line = row![self.slider(
                &spec,
                f64::from((e.opacity * 100.0).round() as u32),
                percent,
                SliderLook::fill(13.0, 48, Alignment::Center),
            ),]
            .spacing(12)
            .align_y(Alignment::Center);
            if place.blurrable() {
                line = line.push(beside_slider(
                    &spec,
                    toggler(e.blur)
                        .label(crate::i18n::tr("Flou"))
                        .font(font)
                        .on_toggle(move |v| wrap(Msg::Blur(place, v))),
                ));
            }
            // Transparence adaptative : les surfaces posées sur le fond
            // d'écran (le lanceur l'est toujours ; menus et Réglages
            // passent devant des fenêtres, invisibles pour hyprdesk).
            let adaptable = matches!(place, Place::Bar | Place::Widgets);
            if adaptable {
                line = line.push(beside_slider(
                    &spec,
                    toggler(e.adaptive.is_some())
                        .label(crate::i18n::tr("Adaptatif"))
                        .font(font)
                        .on_toggle(move |v| wrap(Msg::Adaptive(place, v))),
                ));
            }
            let mut cell = column![line].spacing(4);
            let note = match (place, e.adaptive) {
                (Place::Launcher, _) => Some(
                    crate::i18n::tr("Densifié selon le fond d'écran pour rester lisible.")
                        .to_string(),
                ),
                (Place::Bar, Some(max)) => {
                    let now = self.bar_palette().opacity;
                    Some(crate::tr!(
                        "Densifié jusqu'à {max} % si le fond gêne la lecture — en ce moment {now} %.",
                        max = (max * 100.0).round() as i32,
                        now = (now * 100.0).round() as i32
                    ))
                }
                (Place::Widgets, Some(max)) => Some(crate::tr!(
                    "Densifié jusqu'à {max} % si le fond gêne la lecture.",
                    max = (max * 100.0).round() as i32
                )),
                _ => None,
            };
            if let Some(n) = note {
                cell = cell.push(
                    text(n)
                        .font(font)
                        .size(crate::ui::tokens::ts(11.0))
                        .color(self.ink().dim),
                );
            }
            // Verre du pack (MLG-268) : ce que le thème dessine en plus.
            let glass = crate::ui::tokens::glass(place);
            if glass.highlight > 0.0 || glass.refraction > 0.0 || glass.tint_strength > 0.0 {
                let pct = |v: f32| (v * 100.0).round() as i32;
                cell = cell.push(
                    text(crate::tr!(
                        "Verre du thème : reflet {h} %, réfraction {r} %, teinte {t} %.",
                        h = pct(glass.highlight),
                        r = pct(glass.refraction),
                        t = pct(glass.tint_strength)
                    ))
                    .font(font)
                    .size(crate::ui::tokens::ts(11.0))
                    .color(self.ink().dim),
                );
            }
            if place == Place::Settings && self.hypr_blur_off() {
                cell = cell.push(
                    text("Opaque tant que le flou de Hyprland est coupé (Performance).")
                        .font(font)
                        .size(crate::ui::tokens::ts(11.0))
                        .color(self.ink().dim),
                );
            }
            effects = effects.push(self.field(
                place.label(),
                &format!("theme.effects.{}", place.as_str()),
                cell,
            ));
        }
        // Accessibilité (MLG-269, MLG-271) : quel que soit le thème.
        let pack = crate::ui::tokens::design();
        let text_scale = SliderSpec::new(
            "text_scale",
            (75.0, 200.0),
            1.0,
            |v| wrap(Msg::TextScale(v.round() as f32 / 100.0)),
            wrap(Msg::Commit),
        );
        let mut access = column![
            toggler(g.reduced_transparency)
                .label(crate::i18n::tr("Transparence réduite"))
                .font(font)
                .on_toggle(|v| wrap(Msg::ReducedTransparency(v))),
            toggler(g.reduced_motion)
                .label(crate::i18n::tr("Mouvement réduit"))
                .font(font)
                .on_toggle(|v| wrap(Msg::ReducedMotion(v))),
            toggler(g.increase_contrast)
                .label(crate::i18n::tr("Contraste renforcé"))
                .font(font)
                .on_toggle(|v| wrap(Msg::IncreaseContrast(v))),
            row![
                // Sur la ligne du curseur, au-dessus de son échelle (HIG SEL-09).
                beside_slider(
                    &text_scale,
                    text(crate::i18n::tr("Taille du texte"))
                        .font(font)
                        .size(crate::ui::tokens::ts(13.0))
                ),
                self.slider(
                    &text_scale,
                    f64::from((g.text_scale * 100.0).round()),
                    affiche_pourcent,
                    SliderLook::fill(13.0, 48, Alignment::Center),
                ),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        ]
        .spacing(8);
        if pack.contrast.reduced_transparency || pack.motion.reduced {
            access = access.push(
                text(crate::i18n::tr(
                    "Le thème en place réduit déjà l'un ou l'autre.",
                ))
                .font(font)
                .size(crate::ui::tokens::ts(11.0))
                .color(self.ink().dim),
            );
        }
        effects = effects.push(self.field("Accessibilité", "reduced_transparency", access));
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
                    .size(crate::ui::tokens::ts(16.0)),
                    radio(crate::i18n::tr("Verre"), Mode::Glass, Some(g.mode), |v| {
                        wrap(Msg::Mode(v))
                    })
                    .font(font)
                    .size(crate::ui::tokens::ts(16.0)),
                ]
                .spacing(16),
            ),
        );
        if g.mode == Mode::Glass {
            effects = effects.push(self.field(
                "Voile du verre",
                "glass_opacity",
                self.slider(
                    &SliderSpec::new(
                        "glass_opacity",
                        (0.0, 100.0),
                        1.0,
                        |v| wrap(Msg::GlassOpacity(v.round() as f32 / 100.0)),
                        wrap(Msg::Commit),
                    ),
                    f64::from((g.glass_opacity * 100.0).round()),
                    affiche_pourcent,
                    SliderLook::fill(13.0, 48, Alignment::Center),
                ),
            ));
        }
        let lock_dim = SliderSpec::new(
            "lock_dim",
            (0.0, 100.0),
            1.0,
            |v| wrap(Msg::LockDim(v.round() as f32 / 100.0)),
            wrap(Msg::Commit),
        );
        effects = effects.push(
            self.field(
                "Verrouillage",
                "lock_dim",
                row![
                    self.slider(
                        &lock_dim,
                        f64::from((g.lock.dim * 100.0).round()),
                        affiche_pourcent,
                        SliderLook::fill(13.0, 48, Alignment::Center),
                    ),
                    // Sur la ligne du curseur, au-dessus de son échelle
                    // (HIG SEL-09).
                    beside_slider(
                        &lock_dim,
                        toggler(g.lock.blur)
                            .label(crate::i18n::tr("Flou"))
                            .font(font)
                            .on_toggle(|v| wrap(Msg::LockBlur(v)))
                    ),
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
            .size(crate::ui::tokens::ts(13.0))
            .width(Length::Fill),
            button(
                text("Choisir les fonds d'écran")
                    .font(font)
                    .size(crate::ui::tokens::ts(12.0))
            )
            .padding([4, 10])
            .style(button::secondary)
            .on_press(wrap(Msg::OpenWallpapers)),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        match self.subtab(INFO.id) {
            "couleurs" => {
                // Accessibilité : contraste du texte et des repères sur les
                // surfaces, aux cibles du thème (MLG-269, MLG-274). Les
                // couleurs choisies ici sont gardées telles quelles : on
                // signale, on ne corrige pas.
                let target = crate::ui::tokens::design().contrast.clone();
                let ratio = crate::theme::contrast(th.fg, th.bg);
                let accent = crate::theme::contrast(th.accent, th.bg);
                let mut lines = Vec::new();
                if ratio + 1e-6 < f64::from(target.text) {
                    lines.push((
                        crate::tr!(
                            "Contraste faible : {r}:1 (au moins {t}:1 conseillé ; thème « Contraste élevé » : 21:1).",
                            r = format!("{ratio:.1}"),
                            t = target.text
                        ),
                        true,
                    ));
                } else {
                    lines.push((
                        crate::tr!("Contraste du texte : {r}:1.", r = format!("{ratio:.1}")),
                        false,
                    ));
                }
                if accent + 1e-6 < f64::from(target.ui) {
                    lines.push((
                        crate::tr!(
                            "Accent peu visible sur le fond : {r}:1 (au moins {t}:1 conseillé pour les repères).",
                            r = format!("{accent:.1}"),
                            t = target.ui
                        ),
                        true,
                    ));
                }
                if lines.iter().any(|(_, w)| *w) {
                    lines.push((
                        crate::i18n::tr(
                            "Vos couleurs sont gardées telles quelles ; un thème livré serait corrigé.",
                        )
                        .to_string(),
                        false,
                    ));
                }
                let note = column(lines.into_iter().map(|(l, warn)| {
                    text(l)
                        .font(font)
                        .size(crate::ui::tokens::ts(12.0))
                        .color(if warn {
                            crate::ui::tone::warning()
                        } else {
                            dim
                        })
                        .into()
                }))
                .spacing(4);
                return column![self.section("Couleurs"), colors, note]
                    .spacing(14)
                    .into();
            }
            "texte" => return self.typography(),
            "composants" => return self.design_components(),
            "transparence" => {
                return column![
                    self.section("Transparence"),
                    text(
                        "Opacité du fond de chaque endroit et flou de ce qu'il y a derrière \
                         (fourni par Hyprland)."
                    )
                    .font(font)
                    .size(crate::ui::tokens::ts(12.0))
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
                .size(crate::ui::tokens::ts(12.0))
                .color(dim),
            presets,
            self.section("Thèmes personnels"),
            row![
                text(
                    "Appliquer remplace le thème en place (fonds compris) : enregistrez-le \
                     d'abord pour le garder."
                )
                .font(font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim)
                .width(Length::Fill),
                button(
                    text("Ouvrir le dossier")
                        .font(font)
                        .size(crate::ui::tokens::ts(12.0))
                )
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
                button(
                    text("Créer le thème")
                        .font(font)
                        .size(crate::ui::tokens::ts(13.0))
                )
                .padding([6, 14])
                .on_press_maybe((!st.scene.trim().is_empty()).then(|| wrap(Msg::CreateFromScene))),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            text(
                "Couleurs tirées de l'image ; la scène s'étale sur tous les écrans, découpée \
                 selon leur disposition, au bureau comme au verrouillage. Un pack multi-écrans \
                 (manifest.json) fournit sa scène maîtresse."
            )
            .font(font)
            .size(crate::ui::tokens::ts(12.0))
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
            Msg::Pack(t) => {
                // Le thème du pack ; le moteur le reconnaît à l'installation
                // de la config et le pose comme pack actif.
                self.settings.apply_theme(&t);
                if let Some(panel) = self.panel.as_mut() {
                    panel.theme.name = None;
                    panel.theme.hex.clear();
                }
                self.commit()
            }
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
                    // La couleur change sans le sélecteur : il se referme.
                    panel.theme.picker = None;
                }
                self.set_theme_color(slot, c)
            }
            // Puits de couleur (HIG SEL-02).
            Msg::OpenPicker(slot) => {
                let g = &self.cfg.general;
                let value = match slot {
                    Slot::Fg => g.theme.fg,
                    Slot::Bg => g.theme.bg,
                    Slot::Accent => g.theme.accent,
                    Slot::Background => g.background,
                };
                if let Some(theme) = self.panel.as_mut().map(|p| &mut p.theme) {
                    let open = theme.picker.as_ref().is_some_and(|(s, _)| *s == slot);
                    // Le sélecteur part du brouillon s'il est lisible, sinon
                    // de la couleur en place.
                    let initial = theme
                        .hex
                        .get(&slot)
                        .and_then(|v| Rgba::parse(v.trim()))
                        .unwrap_or(value);
                    theme.picker = (!open).then_some((slot, Picker::new(initial)));
                }
                Task::none()
            }
            Msg::Picker(m) => {
                if let Some((_, pk)) = self.panel.as_mut().and_then(|p| p.theme.picker.as_mut()) {
                    pk.update(m);
                }
                Task::none()
            }
            Msg::PickerCancel => {
                if let Some(panel) = self.panel.as_mut() {
                    panel.theme.picker = None;
                }
                Task::none()
            }
            Msg::PickerApply => {
                match self
                    .panel
                    .as_mut()
                    .and_then(|p| p.theme.picker.take())
                    .map(|(slot, pk)| (slot, pk.color()))
                {
                    Some((slot, c)) => self.theme_update(Msg::Color(slot, c)),
                    None => Task::none(),
                }
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
                    Some(c) => {
                        if let Some(panel) = self.panel.as_mut() {
                            panel.theme.picker = None;
                        }
                        self.set_theme_color(slot, c)
                    }
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
                                    "Thème créé : {out} — à appliquer d'un clic dans les thèmes personnels.",
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
            Msg::Library(l, c) => {
                if let Some(panel) = self.panel.as_mut() {
                    panel.theme.library = l;
                    panel.theme.catalog = c;
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
            Msg::Adaptive(place, v) => {
                let mut e = self.theme_effect(place);
                // Plafond : celui du thème s'il en avait un, sinon presque
                // opaque (le verre reste du verre).
                e.adaptive = v.then(|| e.adaptive.unwrap_or(0.95).max(e.opacity));
                self.settings.effects.insert(place, e);
                self.commit()
            }
            Msg::DesignOpt(name, value, commit) => {
                if let Err(e) =
                    crate::design_options::set(&mut self.settings, &self.cfg, &name, &value)
                {
                    tracing::warn!("{name} : {e}");
                    return Task::none();
                }
                if commit { self.commit() } else { self.apply() }
            }
            Msg::ReducedTransparency(v) => {
                self.settings.reduced_transparency = Some(v);
                self.commit()
            }
            Msg::ReducedMotion(v) => {
                self.settings.reduced_motion = Some(v);
                self.commit()
            }
            Msg::IncreaseContrast(v) => {
                self.settings.increase_contrast = Some(v);
                self.commit()
            }
            Msg::TextScale(v) => {
                self.settings.text_scale = Some(v.clamp(0.75, 2.0));
                self.apply()
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
