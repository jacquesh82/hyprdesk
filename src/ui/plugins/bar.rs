//! Plugin « Barre » : la barre native et sa page de réglages (position,
//! hauteur, écrans, modules par zone).
//!
//! - `surface` : la barre posée sur chaque écran (zones, modules, actions) ;
//! - `layout` : mesure des modules, centrage et débordement ;
//! - `menus` : menus du shell, toujours en tête de la barre ;
//! - `motion` : mouvement local pendant le geste ;
//! - `style` : rendus complets choisis par `hd.bar({ style = … })` ;
//! - `popover` : menus déroulants qui descendent sous un module ;
//! - `keyboard` : la barre au clavier, quand un raccourci lui donne (HIG ENT-02) ;
//! - ce fichier : manifeste et page de Réglages.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

pub(in crate::ui) mod keyboard;
pub(in crate::ui) mod layout;
pub(in crate::ui) mod menus;
pub(in crate::ui) mod motion;
pub(in crate::ui) mod popover;
pub(in crate::ui) mod style;
pub(crate) mod surface;
pub(in crate::ui) mod window;

use crate::i18n::text;
use iced::widget::{Space, button, column, container, mouse_area, pick_list, radio, row, tooltip};
use iced::{Alignment, Border, Color, Element, Length, Task, mouse};

use crate::config::{BAR_COLOR_SLOTS, Rgba};
use crate::ui::color_picker::{Picker, PickerMsg, well};

use super::super::panel::{PanelMsg, SliderLook, SliderSpec, beside_slider};
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "bar",
    label: "Barre",
    icon: "computer",
    summary: "Barre native : position, hauteur, écrans, modules par zone.",
    builtin: true,
};

/// Message de ce paquet, enveloppé pour l'application.
/// Onglets du paquet.
pub const TABS: &[super::SubTab] = &[
    ("general", "Général"),
    ("modules", "Modules"),
    ("colors", "Couleurs"),
];

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Bar(m)))
}

/// Valeur d'un curseur de la barre en pixels, en millisecondes, en pour cent.
fn en_px(v: f64) -> String {
    format!("{} px", v.round() as u16)
}

fn en_ms(v: f64) -> String {
    format!("{} ms", v.round() as u16)
}

fn en_pour_cent(v: f64) -> String {
    format!("{} %", v.round() as i32)
}

/// Les curseurs de la forme de la barre (HIG SEL-08) : leur pose emporte la
/// forme du rendu, modifiée d'un champ, comme son glissé.
pub(in crate::ui) fn side_margin_spec(shape: crate::config::BarShape) -> SliderSpec {
    SliderSpec::new(
        "bar_side_margin",
        (0.0, 200.0),
        1.0,
        move |v| {
            let mut s = shape;
            s.side_margin = v.round() as u16;
            wrap(Msg::Shape(s, false))
        },
        wrap(Msg::Commit),
    )
}

pub(in crate::ui) fn radius_spec(shape: crate::config::BarShape) -> SliderSpec {
    SliderSpec::new(
        "bar_radius",
        (0.0, 32.0),
        1.0,
        move |v| {
            let mut s = shape;
            s.radius = Some(v.round() as u16);
            wrap(Msg::Shape(s, false))
        },
        wrap(Msg::Commit),
    )
}

#[derive(Debug, Clone)]
pub enum Msg {
    Enable(bool),
    Pos(crate::config::BarPosition),
    Height(u16),
    /// Marge au bord de l'écran (glissé).
    Margin(u16),
    /// Forme de la barre (MLG-277) : longueur, alignement, marges,
    /// groupes, rayon, visibilité, place, couche.
    Shape(crate::config::BarShape, bool),
    /// Style visuel de la barre (`ui::plugins::bar::style`).
    Style(crate::config::BarStyle),
    ConvexFrame(u16),
    ConvexRadius(u16),
    ConvexDuration(u16),
    ConvexAttached(bool),
    ConvexAutoHide(bool),
    AllMonitors(bool),
    /// Barre seulement sur l'écran principal.
    Primary,
    /// Ajoute un module à une zone (0 gauche, 1 centre, 2 droite).
    Add(usize, BarChoice),
    /// Déplace un module dans sa zone (-1 avant, +1 après).
    Move(usize, usize, i32),
    /// Change un module de zone (-1 vers la gauche, +1 vers la droite).
    Zone(usize, usize, i32),
    Remove(usize, usize),
    /// Fin d'un glissé de curseur de la barre : enregistrer puis recharger.
    Commit,
    /// Barre compacte : auto (écrans étroits ou verticaux), toujours, jamais.
    Compact(crate::config::BarCompact),
    /// Éditer les modules des écrans verticaux (zones 3..=5) plutôt que l'ordinaire.
    EditPortrait(bool),
    /// Écrans verticaux : revenir aux mêmes modules que les autres.
    PortraitReset,
    /// Couleur d'un emplacement (`text`, `background`…) ou d'un module
    /// (`mod:clock`) ; `None` : celle du thème.
    SetColor(String, Option<crate::config::ColorRef>),
    /// Couleur différente en clair et en sombre (paire), ou de nouveau une
    /// seule (celle du mode en cours).
    ToggleByMode(String),
    /// Ouvre le sélecteur de couleur sous la ligne `clé` (le referme si déjà ouvert).
    OpenPicker(String),
    Picker(PickerMsg),
    PickerApply,
    PickerCancel,
    /// Transparence du fond (glissé), `None` : celle du thème.
    Opacity(Option<f32>),
}

/// État d'écran : quelles zones sont éditées.
#[derive(Default)]
pub struct State {
    pub portrait: bool,
    /// Sélecteur de couleur ouvert (onglet Couleurs) : la clé qu'il règle.
    pub picker: Option<(String, Picker)>,
}

/// Choix « ajouter un module » de la barre.
#[derive(Debug, Clone, PartialEq)]
pub struct BarChoice {
    pub type_id: String,
    pub label: String,
}

impl std::fmt::Display for BarChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

impl App {
    /// Forme de la barre (MLG-277) dans Réglages → Barre.
    fn bar_shape_fields(
        &self,
        shape: crate::config::BarShape,
        style: crate::config::BarStyle,
    ) -> Element<'_, UiMsg> {
        use crate::config::{BarAlign, BarLayer, BarLength, BarVisibility};
        let font = self.font;
        let set = move |f: &dyn Fn(&mut crate::config::BarShape), commit: bool| {
            let mut s = shape;
            f(&mut s);
            wrap(Msg::Shape(s, commit))
        };
        let lengths: [(BarLength, &str); 5] = [
            (BarLength::Full, "Bord entier"),
            (BarLength::Content, "Selon le contenu"),
            (BarLength::Percent(0.5), "50 %"),
            (BarLength::Percent(0.7), "70 %"),
            (BarLength::Percent(0.9), "90 %"),
        ];
        let current = lengths.iter().position(|(l, _)| *l == shape.length);
        let length = row(lengths.into_iter().enumerate().map(|(i, (l, label))| {
            self.focusable(
                radio(crate::i18n::tr(label), i, current, move |_| {
                    set(&|s: &mut crate::config::BarShape| s.length = l, true)
                })
                .font(font)
                .size(crate::ui::tokens::ts(15.0)),
            )
        }))
        .spacing(14)
        .wrap();
        let mut col = column![self.field("Longueur", "", length)].spacing(18);
        if shape.length != BarLength::Full {
            let aligns: [(BarAlign, &str); 3] = [
                (BarAlign::Start, "Début"),
                (BarAlign::Center, "Centre"),
                (BarAlign::End, "Fin"),
            ];
            col = col.push(
                self.field(
                    "Alignement",
                    "",
                    row(aligns.into_iter().map(|(a, label)| {
                        self.focusable(
                            radio(crate::i18n::tr(label), a, Some(shape.alignment), move |v| {
                                set(&|s: &mut crate::config::BarShape| s.alignment = v, true)
                            })
                            .font(font)
                            .size(crate::ui::tokens::ts(15.0)),
                        )
                    }))
                    .spacing(14),
                ),
            );
        }
        col = col.push(self.field(
            "Marges sur les côtés",
            crate::i18n::tr("0 = d'un bord à l'autre ; au-delà, la barre flotte"),
            self.slider(
                &side_margin_spec(shape),
                f64::from(shape.side_margin.min(200)),
                en_px,
                SliderLook::fixed(13.0, 260, Alignment::Start),
            ),
        ));
        col = col.push(self.field(
            "Rayon des coins",
            "",
            self.slider(
                &radius_spec(shape),
                f64::from(shape.radius.unwrap_or(11).min(32)),
                en_px,
                SliderLook::fixed(13.0, 260, Alignment::Start),
            ),
        ));
        let mut toggles = column![
            self.focusable(
                self.toggle(shape.visibility == BarVisibility::AutoHide)
                    .label(crate::i18n::tr(
                        "Masquer hors survol (un contact du bord la fait revenir)"
                    ))
                    .font(font)
                    .on_toggle(move |v| set(
                        &|s: &mut crate::config::BarShape| {
                            s.visibility = if v {
                                BarVisibility::AutoHide
                            } else {
                                BarVisibility::Always
                            }
                        },
                        true
                    ))
            ),
            self.focusable(
                self.toggle(shape.reserve_space)
                    .label(crate::i18n::tr(
                        "Les fenêtres laissent la place de la barre"
                    ))
                    .font(font)
                    .on_toggle(move |v| set(
                        &|s: &mut crate::config::BarShape| s.reserve_space = v,
                        true
                    ))
            ),
        ]
        .spacing(10);
        if style == crate::config::BarStyle::Continuite {
            toggles = toggles.push(
                self.focusable(
                    self.toggle(shape.groups)
                        .label(crate::i18n::tr("Groupes détachés (sans lame continue)"))
                        .font(font)
                        .on_toggle(move |v| {
                            set(&|s: &mut crate::config::BarShape| s.groups = v, true)
                        }),
                ),
            );
        }
        col = col.push(self.field("Comportement", "", toggles));
        let layers: [(BarLayer, &str); 3] = [
            (BarLayer::Top, "Sur les fenêtres"),
            (BarLayer::Overlay, "Sur tout, plein écran compris"),
            (BarLayer::Bottom, "Sous les fenêtres"),
        ];
        col = col.push(
            self.field(
                "Couche",
                "",
                row(layers.into_iter().map(|(l, label)| {
                    self.focusable(
                        radio(crate::i18n::tr(label), l, Some(shape.layer), move |v| {
                            set(&|s: &mut crate::config::BarShape| s.layer = v, true)
                        })
                        .font(font)
                        .size(crate::ui::tokens::ts(15.0)),
                    )
                }))
                .spacing(14)
                .wrap(),
            ),
        );
        col.into()
    }

    /// Même transaction pour les poignées de barre et la liste des réglages.
    pub(in crate::ui) fn finish_bar_drag(&mut self) -> bool {
        let Some((from, Some(to))) = self.bar_drag.take() else {
            return false;
        };
        self.settings
            .bar
            .as_mut()
            .is_some_and(|b| b.move_entry(from, to))
    }

    pub(in crate::ui) fn tab_bar(&self) -> Element<'_, UiMsg> {
        use crate::config::BarPosition;
        let g = &self.cfg.general;
        let dim = self.ink().dim;
        let warn = crate::ui::tone::warning();
        let (mon, _) = self.panel_ctx().unwrap_or_default();
        let b = self
            .settings
            .bar
            .clone()
            .unwrap_or_else(|| crate::settings::BarSettings::from_config(&self.cfg));
        let label_of = |type_id: &str| -> String {
            crate::config::BAR_ITEMS
                .iter()
                .find(|(id, _)| *id == type_id)
                .map(|(_, l)| crate::i18n::tr(l).to_string())
                .or_else(|| {
                    self.cfg
                        .catalog
                        .iter()
                        .find(|t| t.id == type_id)
                        .map(|t| t.name.clone())
                })
                .unwrap_or_else(|| type_id.to_string())
        };
        let choices: Vec<BarChoice> = crate::config::BAR_ITEMS
            .iter()
            .map(|(id, l)| BarChoice {
                type_id: id.to_string(),
                label: crate::i18n::tr(l).to_string(),
            })
            .chain(
                self.cfg
                    .catalog
                    .iter()
                    .filter(|t| t.toggleable && t.error.is_none())
                    .map(|t| BarChoice {
                        type_id: t.id.clone(),
                        label: format!("{} (widget)", t.name),
                    }),
            )
            .collect();
        if self.on_subtab(INFO.id, "colors") {
            // Modules présents dans la barre (toutes zones), sans doublon.
            let mut modules: Vec<(String, String)> = Vec::new();
            for e in b.zones.iter().chain(b.portrait.iter().flatten()).flatten() {
                if !modules.iter().any(|(k, _)| *k == e.type_id) {
                    modules.push((e.type_id.clone(), label_of(&e.type_id)));
                }
            }
            return self.bar_colors_page(&b, modules);
        }
        let general = self.on_subtab(INFO.id, "general");
        let mut col = if general {
            column![
                self.section("Barre de hyprdesk"),
                text("Au-dessus des fenêtres, avec sa zone réservée : les fenêtres et la grille du bureau se rangent à côté. Ses modules sont des widgets.").font(self.font).size(crate::ui::tokens::ts(12.0)).color(dim),
                self.field("Afficher la barre", "bar", self.focusable(self.toggle(b.enabled).on_toggle(|v| wrap(Msg::Enable(v))))),
            ]
        } else {
            column![self.section("Modules de la barre")]
        }
        .spacing(12);
        if self.cfg.keys_user.contains("bar") {
            col = col.push(text("hd.bar est aussi déclaré dans hyprdesk.lua, qui l'emporte : ces réglages sont sans effet.").font(self.font).size(crate::ui::tokens::ts(12.0)).color(warn));
        }
        if !b.enabled {
            col = col.push(
                text(if general {
                    crate::i18n::tr("Pensez ensuite à masquer la barre de wayle (sa configuration) pour éviter deux barres.")
                } else {
                    crate::i18n::tr("La barre est masquée : affichez-la dans l'onglet Général.")
                })
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim),
            );
            return col.into();
        }
        let small = |label: &'static str, msg: UiMsg| {
            self.focusable(
                button(
                    text(label)
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0)),
                )
                .padding([2, 8])
                .on_press(msg),
            )
        };
        if general {
            col = col
                .push(
                    self.field(
                        "Position",
                        "",
                        row![
                            self.focusable(radio(
                                crate::i18n::tr("En haut"),
                                BarPosition::Top,
                                Some(b.position),
                                |v| { wrap(Msg::Pos(v)) }
                            )
                            .font(self.font)),
                            self.focusable(radio(
                                crate::i18n::tr("En bas"),
                                BarPosition::Bottom,
                                Some(b.position),
                                |v| { wrap(Msg::Pos(v)) }
                            )
                            .font(self.font)),
                            // Barre verticale (MLG-277).
                            self.focusable(radio(
                                crate::i18n::tr("À gauche"),
                                BarPosition::Left,
                                Some(b.position),
                                |v| { wrap(Msg::Pos(v)) }
                            )
                            .font(self.font)),
                            self.focusable(radio(
                                crate::i18n::tr("À droite"),
                                BarPosition::Right,
                                Some(b.position),
                                |v| { wrap(Msg::Pos(v)) }
                            )
                            .font(self.font)),
                        ]
                        .spacing(16)
                        .wrap(),
                    ),
                )
                .push(
                    self.field(
                        "Hauteur",
                        "",
                        self.slider(
                            &SliderSpec::new("bar_height", (24.0, 64.0), 1.0, |v| wrap(Msg::Height(v.round() as u16)), wrap(Msg::Commit)),
                            f64::from(b.height.clamp(24, 64)),
                            |v| format!("{} px", v.round() as u16),
                            SliderLook::fixed(13.0, 260, Alignment::Start),
                        ),
                    ),
                )
                .push(
                    self.field(
                        "Marge au bord de l'écran",
                        crate::i18n::tr("0 = collée au bord"),
                        self.slider(
                            &SliderSpec::new("bar_margin", (0.0, 24.0), 1.0, |v| wrap(Msg::Margin(v.round() as u16)), wrap(Msg::Commit)),
                            f64::from(b.margin.min(24)),
                            |v| format!("{} px", v.round() as u16),
                            SliderLook::fixed(13.0, 260, Alignment::Start),
                        ),
                    ),
                )
                .push(self.bar_shape_fields(b.shape, b.style))
                .push({
                    // Le style : un rendu complet, chaque style son geste.
                    use crate::config::{BarStyle, BAR_STYLES};
                    self.field(
                        "Style",
                        match b.style {
                            BarStyle::Continuite => {
                                crate::i18n::tr("Groupes arrondis et sélection glissante.")
                            }
                            BarStyle::Graphite => crate::i18n::tr(
                                "Barre des tâches : lanceur centré, widgets à droite, surface rectangulaire au bord inférieur.",
                            ),
                            BarStyle::Island => crate::i18n::tr(
                                "Capsule noire centrée, discrète au repos ; commandes au survol et notifications au centre.",
                            ),
                            BarStyle::Convexe => crate::i18n::tr(
                                "Cadre continu aux angles arrondis ; les panneaux se déploient depuis ses bords.",
                            ),
                        },
                        row(
                            BAR_STYLES
                                .iter()
                                // Styles en développement : en configuration
                                // avancée seulement — sauf celui déjà choisi,
                                // pour que la sélection reste visible.
                                .filter(|&&(style, _, _)| {
                                    !style.in_development() || self.advanced() || style == b.style
                                })
                                .map(|&(style, _, label)| {
                                    let choice = self.focusable(radio(
                                        crate::i18n::tr(label),
                                        style,
                                        Some(b.style),
                                        |v| { wrap(Msg::Style(v)) }
                                    )
                                    .font(self.font));
                                    if !style.in_development() {
                                        return choice;
                                    }
                                    // Style pas encore terminé : une étiquette
                                    // le dit, à côté de son nom.
                                    let dim = self.ink().dim;
                                    let tag = container(
                                        text(crate::ui::etats::Etat::EnDeveloppement.libelle())
                                            .font(self.font)
                                            .size(crate::ui::tokens::ts(11.0))
                                            .color(dim),
                                    )
                                    .padding([1, 6])
                                    .style(move |_| container::Style {
                                        border: Border::default().rounded(8).width(1).color(Color { a: 0.5, ..dim }),
                                        ..Default::default()
                                    });
                                    row![choice, tag].spacing(6).align_y(Alignment::Center).into()
                                })
                                .collect::<Vec<_>>(),
                        )
                        .spacing(16)
                        .wrap()
                        .vertical_spacing(8),
                    )
                })
                .push(self.field("Écrans", "", {
                    // 0 : tous ; 1 : l'écran principal ; 2 : cet écran.
                    let which = match &b.monitors {
                        None => 0u8,
                        Some(l) if l.len() == 1 && l[0] == crate::ui::plugins::bar::surface::PRIMARY => 1,
                        Some(_) => 2,
                    };
                    let pick = |v: u8| {
                        wrap(match v {
                            0 => Msg::AllMonitors(true),
                            1 => Msg::Primary,
                            _ => Msg::AllMonitors(false),
                        })
                    };
                    row![
                        self.focusable(radio(crate::i18n::tr("Tous"), 0u8, Some(which), pick).font(self.font)),
                        self.focusable(radio(crate::i18n::tr("Écran principal"), 1u8, Some(which), pick)
                            .font(self.font)),
                        self.focusable(radio(
                            crate::tr!("Seulement {mon}", mon = mon),
                            2u8,
                            Some(which),
                            pick
                        )
                        .font(self.font)),
                    ]
                    .spacing(16)
                    .wrap()
                    .vertical_spacing(8)
                }));
            if b.style == crate::config::BarStyle::Convexe {
                for (key, label, value, max, message, affiche) in [
                    (
                        "bar_convex_frame",
                        "Épaisseur du cadre",
                        b.convex.frame_width,
                        20,
                        Msg::ConvexFrame as fn(u16) -> Msg,
                        en_px as fn(f64) -> String,
                    ),
                    (
                        "bar_convex_radius",
                        "Arrondi intérieur",
                        b.convex.radius,
                        96,
                        Msg::ConvexRadius as fn(u16) -> Msg,
                        en_px as fn(f64) -> String,
                    ),
                    (
                        "bar_convex_duration",
                        "Durée d'ouverture des panneaux",
                        b.convex.duration,
                        800,
                        Msg::ConvexDuration as fn(u16) -> Msg,
                        en_ms as fn(f64) -> String,
                    ),
                ] {
                    col = col.push(self.field(
                        label,
                        "",
                        self.slider(
                            &SliderSpec::new(
                                key,
                                (0.0, f64::from(max)),
                                1.0,
                                move |v| wrap(message(v.round() as u16)),
                                wrap(Msg::Commit),
                            ),
                            f64::from(value.min(max)),
                            affiche,
                            SliderLook::fixed(13.0, 260, Alignment::Start),
                        ),
                    ));
                }
                col = col.push(
                    self.field(
                        "Masquer automatiquement le dock",
                        "",
                        self.focusable(
                            self.toggle(b.convex.auto_hide)
                                .on_toggle(|v| wrap(Msg::ConvexAutoHide(v))),
                        ),
                    ),
                );
                col = col.push(
                    self.field(
                        "Panneaux rattachés au cadre",
                        "",
                        self.focusable(
                            self.toggle(b.convex.attached_panels)
                                .on_toggle(|v| wrap(Msg::ConvexAttached(v))),
                        ),
                    ),
                );
            }
            use crate::config::BarCompact;
            let compact = |label: &'static str, v: BarCompact| {
                self.focusable(
                    radio(crate::i18n::tr(label), v, Some(b.compact), |v| {
                        wrap(Msg::Compact(v))
                    })
                    .font(self.font),
                )
            };
            col = col.push(
                self.field(
                    "Barre compacte (titres courts, heure seule, icônes sans libellé)",
                    "",
                    row![
                        compact(
                            crate::i18n::tr("Auto : selon l’écran et le contenu"),
                            BarCompact::Auto
                        ),
                        compact("Toujours", BarCompact::Always),
                        compact("Jamais", BarCompact::Never),
                    ]
                    .spacing(16)
                    .wrap()
                    .vertical_spacing(8),
                ),
            );
            return col.into();
        }
        let portrait = self.panel.as_ref().is_some_and(|p| p.bar.portrait);
        let mut which = row![
            self.focusable(
                radio(
                    crate::i18n::tr("Écrans horizontaux"),
                    false,
                    Some(portrait),
                    |v| wrap(Msg::EditPortrait(v))
                )
                .font(self.font)
            ),
            self.focusable(
                radio(
                    crate::i18n::tr("Écrans verticaux"),
                    true,
                    Some(portrait),
                    |v| wrap(Msg::EditPortrait(v))
                )
                .font(self.font)
            ),
        ]
        .spacing(16)
        .align_y(Alignment::Center);
        if portrait && b.portrait.is_some() {
            which = which.push(
                self.focusable(
                    button(
                        text("Mêmes modules que les autres écrans")
                            .font(self.font)
                            .size(crate::ui::tokens::ts(12.0)),
                    )
                    .padding([3, 10])
                    .style(button::secondary)
                    .on_press(wrap(Msg::PortraitReset)),
                ),
            );
        }
        col = col.push(self.adv("Modules par type d'écran", || self.field("", "", which)));
        if portrait && b.portrait.is_none() {
            col = col.push(
                text(
                    "Les écrans verticaux reprennent les modules des autres écrans ; \
                     un changement ici leur donne leurs propres modules.",
                )
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim),
            );
        }
        let (zones, off) = match (&b.portrait, portrait) {
            (Some(p), true) => (p, 3),
            (None, true) => (&b.zones, 3),
            _ => (&b.zones, 0),
        };
        let graphite = b.style == crate::config::BarStyle::Graphite;
        let accent = color(g.theme.accent, 1.0);
        col = col.push(
            text("Faites glisser un module pour le déplacer.")
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim),
        );
        if graphite {
            col = col.push(
                text("Le lanceur reste au centre ; placez vos modules à gauche, au centre ou à droite.")
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(dim),
            );
        }
        for (zi, zname) in ["Gauche", "Centre", "Droite"].iter().enumerate() {
            let z = zi + off;
            col = col.push(
                text(*zname)
                    .font(self.font)
                    .size(crate::ui::tokens::ts(15.0))
                    .color(color(g.theme.accent, 1.0)),
            );
            let mut group = column![].spacing(0);
            let marker = |i| {
                container(Space::new().height(2).width(Length::Fill)).style(move |_| {
                    container::Style {
                        background: self
                            .bar_drag
                            .is_some_and(|(_, to)| to == Some((z, i)))
                            .then(|| accent.into()),
                        ..Default::default()
                    }
                })
            };
            if zones[zi].is_empty() {
                let empty = container(
                    text("vide")
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                        .color(dim),
                )
                .width(Length::Fill)
                .padding([6, 0]);
                group = group.push({
                    Element::from(
                        mouse_area(empty)
                            .on_enter(UiMsg::BarDragOver(z, 0))
                            .on_release(UiMsg::BarDragEnd),
                    )
                });
            }
            for (i, e) in zones[zi].iter().enumerate() {
                let draggable = b.can_drag_entry((z, i));
                let dragged = self.bar_drag.is_some_and(|(from, _)| from == (z, i));
                let label = text(label_of(&e.type_id))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0));
                let label: Element<'_, UiMsg> = if draggable {
                    let grip = iced::widget::svg(
                        super::super::icons::get("grip-vertical").expect("drag grip"),
                    )
                    .width(14)
                    .height(16)
                    .style(move |_, _| iced::widget::svg::Style {
                        color: Some(if dragged { accent } else { dim }),
                    });
                    mouse_area(
                        container(row![grip, label].spacing(8).align_y(Alignment::Center))
                            .width(Length::Fill)
                            .height(32)
                            .align_y(Alignment::Center),
                    )
                    .interaction(if dragged {
                        mouse::Interaction::Grabbing
                    } else {
                        mouse::Interaction::Grab
                    })
                    .on_press(UiMsg::BarDragStart(z, i))
                    .into()
                } else {
                    container(label)
                        .width(Length::Fill)
                        .height(32)
                        .align_y(Alignment::Center)
                        .into()
                };
                let mut r = row![label].spacing(4).align_y(Alignment::Center);
                if draggable && i > 0 {
                    r = r.push(small("↑", wrap(Msg::Move(z, i, -1))));
                }
                if draggable && i + 1 < zones[zi].len() {
                    r = r.push(small("↓", wrap(Msg::Move(z, i, 1))));
                }
                if draggable && zi > 0 {
                    r = r.push(small("←", wrap(Msg::Zone(z, i, -1))));
                }
                if draggable && zi < 2 {
                    r = r.push(small("→", wrap(Msg::Zone(z, i, 1))));
                }
                if draggable {
                    // Retrait : le glyphe du jeu, comme les menus (HIG ICO-07).
                    r = r.push(
                        self.focusable(
                            button(
                                crate::ui::icons::glyph(
                                    "window-close",
                                    crate::ui::tokens::ts(12.0),
                                    self.ink().fg,
                                )
                                .expect("glyphe de fermeture"),
                            )
                            .padding([4, 8])
                            .on_press(wrap(Msg::Remove(z, i))),
                        ),
                    );
                }
                group = group.push(marker(i));
                let item = container(r)
                    .width(Length::Fill)
                    .style(move |_| container::Style {
                        background: dragged.then(|| Color { a: 0.10, ..accent }.into()),
                        border: Border::default().rounded(4),
                        ..Default::default()
                    });
                group = group.push(
                    mouse_area(item)
                        .on_move(move |p| UiMsg::BarDragOver(z, i + usize::from(p.y >= 16.0)))
                        .on_release(UiMsg::BarDragEnd),
                );
            }
            {
                group = group.push(marker(zones[zi].len())).push(
                    mouse_area(Space::new().width(Length::Fill).height(12))
                        .on_enter(UiMsg::BarDragOver(z, zones[zi].len()))
                        .on_release(UiMsg::BarDragEnd),
                );
            }
            col = col.push(mouse_area(group).on_exit(UiMsg::BarDragLeave(z)));
            col = col.push(
                pick_list(
                    choices
                        .iter()
                        .filter(|c| !graphite || c.type_id != "launcher")
                        .cloned()
                        .collect::<Vec<_>>(),
                    None::<BarChoice>,
                    move |c| wrap(Msg::Add(z, c)),
                )
                .font(self.font)
                .placeholder(crate::i18n::tr("Ajouter un module…"))
                .width(260),
            );
        }
        col.into()
    }

    pub(in crate::ui) fn bar_update(&mut self, msg: Msg) -> Task<UiMsg> {
        if self.settings.bar.is_none() && !matches!(msg, Msg::Commit) {
            self.settings.bar = Some(crate::settings::BarSettings::from_config(&self.cfg));
        }
        match msg {
            Msg::Enable(on) => {
                let b = self
                    .settings
                    .bar
                    .get_or_insert_with(crate::settings::BarSettings::default);
                b.enabled = on;
                self.commit_reload()
            }
            Msg::Pos(p) => {
                self.settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .position = p;
                self.commit_reload()
            }
            Msg::Height(h) => {
                self.settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .height = h;
                Task::none()
            }
            Msg::AllMonitors(all) => {
                let mon = self.panel_ctx().map(|c| c.0).unwrap_or_default();
                self.settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .monitors = if all { None } else { Some(vec![mon]) };
                self.commit_reload()
            }
            Msg::Primary => {
                self.settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .monitors = Some(vec![crate::ui::plugins::bar::surface::PRIMARY.to_string()]);
                self.commit_reload()
            }
            Msg::Add(z, choice) => {
                self.settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .zone_mut(z)
                    .push(crate::settings::BarEntry::new(&choice.type_id));
                self.commit_reload()
            }
            Msg::Move(z, i, d) => {
                let zone = self
                    .settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .zone_mut(z);
                let j = i as i32 + d;
                if i < zone.len() && j >= 0 && (j as usize) < zone.len() {
                    zone.swap(i, j as usize);
                }
                self.commit_reload()
            }
            Msg::Zone(z, i, d) => {
                let b = self.settings.bar.get_or_insert_with(Default::default);
                // Même jeu de zones : ordinaire (0..=2) ou écrans verticaux (3..=5).
                let (base, local) = (z / 3 * 3, (z % 3) as i32);
                let to = local + d;
                if (0..3).contains(&to) && i < b.zone_mut(z).len() {
                    let e = b.zone_mut(z).remove(i);
                    b.zone_mut(base + to as usize).push(e);
                }
                self.commit_reload()
            }
            Msg::Remove(z, i) => {
                let zone = self
                    .settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .zone_mut(z);
                if i < zone.len() {
                    zone.remove(i);
                }
                self.commit_reload()
            }
            Msg::ConvexFrame(v) => {
                self.settings.bar.as_mut().unwrap().convex.frame_width = v;
                Task::none()
            }
            Msg::ConvexRadius(v) => {
                self.settings.bar.as_mut().unwrap().convex.radius = v;
                Task::none()
            }
            Msg::ConvexDuration(v) => {
                self.settings.bar.as_mut().unwrap().convex.duration = v;
                Task::none()
            }
            Msg::ConvexAutoHide(v) => {
                self.settings.bar.as_mut().unwrap().convex.auto_hide = v;
                self.commit_reload()
            }
            Msg::ConvexAttached(v) => {
                self.settings.bar.as_mut().unwrap().convex.attached_panels = v;
                self.commit_reload()
            }
            Msg::Commit => self.commit_reload(),
            Msg::Compact(c) => {
                self.settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .compact = c;
                self.commit_reload()
            }
            Msg::Margin(m) => {
                self.settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .margin = m;
                Task::none()
            }
            Msg::Shape(shape, commit) => {
                self.settings.bar.get_or_insert_with(Default::default).shape = shape;
                if commit {
                    self.commit_reload()
                } else {
                    Task::none()
                }
            }
            Msg::Style(s) => {
                let b = self.settings.bar.get_or_insert_with(Default::default);
                // Choisir un style pose toute la barre du style : rendu,
                // composition des zones, pose, hauteur et marge. Les réglages
                // fins (Modules, Position, Hauteur, Marge) restent ajustables
                // ensuite et redeviennent prioritaires.
                b.style = s;
                b.position = s.default_position();
                b.height = s.default_height();
                b.margin = s.default_margin();
                b.portrait = None;
                b.zones = s.preset_settings();
                self.commit_reload()
            }
            Msg::EditPortrait(v) => {
                if let Some(p) = self.panel.as_mut() {
                    p.bar.portrait = v;
                }
                Task::none()
            }
            Msg::PortraitReset => {
                self.settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .portrait = None;
                self.commit_reload()
            }
            Msg::SetColor(key, c) => {
                if let Some(p) = self.panel.as_mut() {
                    p.bar.picker = None;
                }
                self.set_bar_color(&key, c);
                self.commit_reload()
            }
            Msg::ToggleByMode(key) => {
                use crate::config::ColorRef;
                let shown = self.bar_color_shown(&key);
                let next = match self.bar_color_spec(&key) {
                    Some(ColorRef::Pair { .. }) => ColorRef::Fixed(shown),
                    _ => ColorRef::Pair {
                        dark: shown,
                        light: shown,
                    },
                };
                if let Some(p) = self.panel.as_mut() {
                    p.bar.picker = None;
                }
                self.set_bar_color(&key, Some(next));
                self.commit_reload()
            }
            Msg::OpenPicker(key) => {
                let open = self
                    .panel
                    .as_ref()
                    .and_then(|p| p.bar.picker.as_ref())
                    .is_some_and(|(k, _)| *k == key);
                let initial = self.bar_color_shown(&key);
                if let Some(p) = self.panel.as_mut() {
                    p.bar.picker = (!open).then(|| (key, Picker::new(initial)));
                }
                Task::none()
            }
            Msg::Picker(m) => {
                if let Some((_, pk)) = self.panel.as_mut().and_then(|p| p.bar.picker.as_mut()) {
                    pk.update(m);
                }
                Task::none()
            }
            Msg::PickerCancel => {
                if let Some(p) = self.panel.as_mut() {
                    p.bar.picker = None;
                }
                Task::none()
            }
            Msg::PickerApply => {
                let Some((key, pk)) = self.panel.as_mut().and_then(|p| p.bar.picker.take()) else {
                    return Task::none();
                };
                // Paire sombre/clair : le sélecteur règle la moitié du mode
                // en cours, l'autre reste.
                let picked = pk.color();
                let next = match self.bar_color_spec(&key) {
                    Some(crate::config::ColorRef::Pair { dark, light }) => {
                        if surface::light_theme(&self.cfg.general.theme) {
                            crate::config::ColorRef::Pair {
                                dark,
                                light: picked,
                            }
                        } else {
                            crate::config::ColorRef::Pair {
                                dark: picked,
                                light,
                            }
                        }
                    }
                    _ => crate::config::ColorRef::Fixed(picked),
                };
                self.set_bar_color(&key, Some(next));
                self.commit_reload()
            }
            Msg::Opacity(o) => {
                self.settings
                    .bar
                    .get_or_insert_with(Default::default)
                    .colors
                    .opacity = o;
                // Glissé : enregistré au relâché (Commit) ; « Thème » : tout de suite.
                if o.is_none() {
                    self.commit_reload()
                } else {
                    Task::none()
                }
            }
        }
    }

    /// Couleur réglée d'une ligne (`text`, `mod:clock`…), telle qu'écrite.
    fn bar_color_spec(&self, key: &str) -> Option<crate::config::ColorRef> {
        let b = self
            .settings
            .bar
            .clone()
            .unwrap_or_else(|| crate::settings::BarSettings::from_config(&self.cfg));
        match key.strip_prefix("mod:") {
            Some(m) => b.colors.modules.get(m).cloned(),
            None => b.colors.slot(key).cloned(),
        }
    }

    /// Couleur affichée dans les réglages et valeur de départ du sélecteur.
    fn bar_color_shown(&self, key: &str) -> Rgba {
        let b = self
            .settings
            .bar
            .clone()
            .unwrap_or_else(|| crate::settings::BarSettings::from_config(&self.cfg));
        let pal = surface::BarPalette::resolve(&self.cfg.general.theme, Some(&b.colors));
        let light = surface::light_theme(&self.cfg.general.theme);
        match key.strip_prefix("mod:") {
            Some(m) => b
                .colors
                .modules
                .get(m)
                .and_then(|c| surface::resolve_color(c, light))
                .unwrap_or(pal.fg),
            None => match key {
                "background" => pal.bg,
                "accent" => pal.accent,
                "warning" => pal.warning,
                "critical" => pal.critical,
                _ => pal.fg,
            },
        }
    }

    fn set_bar_color(&mut self, key: &str, c: Option<crate::config::ColorRef>) {
        let colors = &mut self
            .settings
            .bar
            .get_or_insert_with(Default::default)
            .colors;
        match key.strip_prefix("mod:") {
            Some(m) => match c {
                Some(c) => {
                    colors.modules.insert(m.to_string(), c);
                }
                None => {
                    colors.modules.remove(m);
                }
            },
            None => colors.set_slot(key, c),
        }
    }

    /// Onglet Couleurs : couleurs de la barre et de chaque module, transparence.
    fn bar_colors_page(
        &self,
        b: &crate::settings::BarSettings,
        modules: Vec<(String, String)>,
    ) -> Element<'_, UiMsg> {
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let dim = self.ink().dim;
        let font = self.font;
        let st = self.panel.as_ref().map(|p| &p.bar);
        let edge = Color { a: 0.3, ..fg };
        let dot = move |c: Rgba, size: f32, selected: bool| {
            let c = color(c, 1.0);
            container(Space::new().width(size).height(size)).style(move |_| container::Style {
                background: Some(c.into()),
                border: Border::default()
                    .rounded(size / 2.0)
                    .width(if selected { 2.5 } else { 1.0 })
                    .color(if selected { fg } else { edge }),
                ..Default::default()
            })
        };
        // Couleurs proposées : celles du thème en place (pas de palette figée).
        let mut proposals: Vec<(Rgba, &'static str)> = Vec::new();
        for (c, l) in [
            (th.fg, "Texte du thème"),
            (th.bg, "Fond du thème"),
            (th.accent, "Accent du thème"),
            (self.cfg.general.background, "Couleur de fond du bureau"),
            (crate::ui::tone::rgba("warning"), "Avertissement"),
            (crate::ui::tone::rgba("danger"), "Critique"),
        ] {
            if !proposals.iter().any(|(p, _)| *p == c) {
                proposals.push((c, l));
            }
        }
        // Slots du pack qui suivent le mode (MLG-263) : l'accent et les
        // couleurs libres du pack actif (`active`, `focus`…), absentes de la
        // palette dérivée d'un thème simple.
        let dérivés: Vec<String> =
            crate::theme_engine::resolve::from_theme(&crate::config::Theme::default())
                .palette
                .into_keys()
                .collect();
        let slot_chips: Vec<(String, Rgba)> = crate::ui::tone::slots()
            .into_iter()
            .filter(|(k, _)| k == "accent" || !dérivés.contains(k))
            .collect();
        let open = st.and_then(|s| s.picker.as_ref());
        let light = surface::light_theme(th);
        // Une ligne : couleurs du thème, slots du pack, couleur en place,
        // sélecteur, couleur par mode, retour au thème.
        let line = |key: String,
                    value: Option<crate::config::ColorRef>,
                    default: Rgba|
         -> Element<'_, UiMsg> {
            use crate::config::ColorRef;
            let shown = value
                .as_ref()
                .and_then(|c| surface::resolve_color(c, light))
                .unwrap_or(default);
            let mut chips = row(proposals.iter().map(|&(c, l)| {
                let k = key.clone();
                tooltip(
                    self.focusable(
                        button(dot(c, 20.0, value == Some(ColorRef::Fixed(c))))
                            .padding(0)
                            .style(|_, _| button::Style::default())
                            .on_press(wrap(Msg::SetColor(k, Some(ColorRef::Fixed(c))))),
                    ),
                    container(
                        text(format!(
                            "{} — {}",
                            crate::i18n::tr(l),
                            crate::settings::hex(c)
                        ))
                        .font(font)
                        .size(crate::ui::tokens::ts(12.0)),
                    )
                    .padding([4, 8])
                    .style(container::rounded_box),
                    tooltip::Position::Top,
                )
                .delay(std::time::Duration::from_millis(400))
                .into()
            }))
            .spacing(6);
            if !slot_chips.is_empty() {
                chips = chips.push(
                    text("·")
                        .font(font)
                        .size(crate::ui::tokens::ts(12.0))
                        .color(dim),
                );
            }
            for (name, c) in &slot_chips {
                let selected = matches!(&value, Some(ColorRef::Slot(s)) if s == name);
                chips = chips.push(
                    tooltip(
                        self.focusable(
                            button(
                                row![
                                    dot(*c, 16.0, selected),
                                    text(name.clone())
                                        .font(font)
                                        .size(crate::ui::tokens::ts(11.0))
                                        .color(if selected { fg } else { dim }),
                                ]
                                .spacing(4)
                                .align_y(Alignment::Center),
                            )
                            .padding([1, 4])
                            .style(|_, _| button::Style::default())
                            .on_press(wrap(Msg::SetColor(
                                key.clone(),
                                Some(ColorRef::Slot(name.clone())),
                            ))),
                        ),
                        container(
                            text(crate::tr!(
                                "Slot « {name} » du thème — suit le mode clair/sombre",
                                name = name
                            ))
                            .font(font)
                            .size(crate::ui::tokens::ts(12.0)),
                        )
                        .padding([4, 8])
                        .style(container::rounded_box),
                        tooltip::Position::Top,
                    )
                    .delay(std::time::Duration::from_millis(400)),
                );
            }
            let is_open = open.is_some_and(|(k, _)| *k == key);
            let by_mode = matches!(value, Some(ColorRef::Pair { .. }));
            let mode_btn = tooltip(
                self.focusable(
                    button(
                        text(crate::i18n::tr("Selon le mode"))
                            .font(font)
                            .size(crate::ui::tokens::ts(12.0)),
                    )
                    .padding([3, 8])
                    .style(if by_mode {
                        button::primary
                    } else {
                        button::secondary
                    })
                    .on_press(wrap(Msg::ToggleByMode(key.clone()))),
                ),
                container(
                    // Une phrase de 75 caractères au plus, qui commence par
                    // un verbe (HIG DAT-20).
                    text(crate::i18n::tr(
                        "Donne une couleur au mode sombre, une autre au mode clair.",
                    ))
                    .font(font)
                    .size(crate::ui::tokens::ts(12.0)),
                )
                .padding([4, 8])
                .style(container::rounded_box),
                tooltip::Position::Top,
            )
            .delay(std::time::Duration::from_millis(400));
            let label = match &value {
                Some(ColorRef::Fixed(c)) => crate::settings::hex(*c),
                Some(other) => other.to_text(),
                None => crate::settings::hex(shown),
            };
            let reset: Element<'_, UiMsg> = if value.is_some() {
                self.focusable(
                    button(
                        text(crate::i18n::tr("Thème"))
                            .font(font)
                            .size(crate::ui::tokens::ts(12.0)),
                    )
                    .padding([3, 8])
                    .on_press(wrap(Msg::SetColor(key.clone(), None))),
                )
            } else {
                text(crate::i18n::tr("thème"))
                    .font(font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(dim)
                    .width(52)
                    .into()
            };
            let mut c = column![
                row![
                    chips,
                    Space::new().width(Length::Fill),
                    // Puits de couleur (HIG SEL-02) : ouvre le sélecteur.
                    well(
                        shown,
                        22.0,
                        is_open,
                        fg,
                        edge,
                        wrap(Msg::OpenPicker(key.clone())),
                    ),
                    text(label)
                        .font(font)
                        .size(crate::ui::tokens::ts(12.0))
                        .color(if value.is_some() { fg } else { dim })
                        .width(if by_mode { 120 } else { 64 }),
                    self.focusable(
                        button(
                            text(crate::i18n::tr(if is_open {
                                "Fermer"
                            } else {
                                "Choisir…"
                            }))
                            .font(font)
                            .size(crate::ui::tokens::ts(12.0))
                        )
                        .padding([3, 8])
                        .style(if is_open {
                            button::primary
                        } else {
                            button::secondary
                        })
                        .on_press(wrap(Msg::OpenPicker(key.clone())))
                    ),
                    mode_btn,
                    reset,
                ]
                .spacing(10)
                .align_y(Alignment::Center)
            ]
            .spacing(8);
            if let Some((_, pk)) = open.filter(|_| is_open) {
                c = c.push(pk.view(
                    font,
                    fg,
                    dim,
                    |m| wrap(Msg::Picker(m)),
                    wrap(Msg::PickerApply),
                    wrap(Msg::PickerCancel),
                ));
            }
            c.into()
        };
        let pal = self.bar_palette();
        let mut col = column![
            self.section("Couleurs de la barre"),
            text(crate::i18n::tr(
                "Par défaut, les couleurs de la barre suivent le thème actif."
            ))
            .font(font)
            .size(crate::ui::tokens::ts(12.0))
            .color(dim),
        ]
        .spacing(12);
        for (k, label) in BAR_COLOR_SLOTS {
            let shown = self.bar_color_shown(k);
            col = col.push(self.field(
                label,
                "",
                line(k.to_string(), b.colors.slot(k).cloned(), shown),
            ));
        }
        // Transparence du fond.
        let theme_opacity = th.effect(crate::config::Place::Bar).opacity;
        let o = b.colors.opacity.unwrap_or(theme_opacity);
        let spec = SliderSpec::new(
            "bar_colors_opacity",
            (0.0, 100.0),
            1.0,
            |v| wrap(Msg::Opacity(Some(v.round() as u32 as f32 / 100.0))),
            wrap(Msg::Commit),
        );
        col = col.push(
            self.field(
                "Opacité du fond",
                "",
                row![
                    self.slider(
                        &spec,
                        f64::from((o * 100.0).round()),
                        en_pour_cent,
                        SliderLook::fill(13.0, 48, Alignment::Center).spacing(10),
                    ),
                    // Sur la ligne du curseur, au-dessus de son échelle
                    // (HIG SEL-09).
                    beside_slider(
                        &spec,
                        if b.colors.opacity.is_some() {
                            Element::from(
                                self.focusable(
                                    button(
                                        text(crate::i18n::tr("Thème"))
                                            .font(font)
                                            .size(crate::ui::tokens::ts(12.0)),
                                    )
                                    .padding([3, 8])
                                    .on_press(wrap(Msg::Opacity(None))),
                                ),
                            )
                        } else {
                            text(crate::i18n::tr("thème"))
                                .font(font)
                                .size(crate::ui::tokens::ts(12.0))
                                .color(dim)
                                .width(52)
                                .into()
                        }
                    ),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            ),
        );
        col = col.push(
            text(crate::i18n::tr(
                "0 % : barre transparente (seuls les modules restent) ; 100 % : opaque. Le flou d'arrière-plan se règle dans Thème → Transparence.",
            ))
            .font(font)
            .size(crate::ui::tokens::ts(12.0))
            .color(dim),
        );
        // Couleur de chaque module : configuration avancée.
        let mut fine = column![].spacing(10);
        fine = fine.push(
            text(crate::i18n::tr(
                "Texte et icônes du module, y compris quand il est actif ; avertissement et critique gardent leurs couleurs.",
            ))
            .font(font)
            .size(crate::ui::tokens::ts(12.0))
            .color(dim),
        );
        if modules.is_empty() {
            fine = fine.push(
                text(crate::i18n::tr("Aucun module dans la barre."))
                    .font(font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(dim),
            );
        }
        for (k, label) in modules {
            let v = b.colors.modules.get(&k).cloned();
            fine = fine.push(
                column![
                    text(label)
                        .font(self.title_font())
                        .size(crate::ui::tokens::ts(13.0)),
                    line(format!("mod:{k}"), v, pal.fg),
                ]
                .spacing(6),
            );
        }
        col = col.push(self.adv("Couleur de chaque module", || fine));
        col.into()
    }
}
