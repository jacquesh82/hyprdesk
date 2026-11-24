//! Paquet intégré « Widgets » : Widgets du bureau (intégrés et plugins Lua) et leur placement.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{Space, button, column, row, toggler};
use iced::{Alignment, Background, Border, Color, Element, Length, Task};

use super::super::etats::Etat;
use super::super::panel::{Panel, PanelMsg, SliderLook, SliderSpec};
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};
use crate::config::resolve::resolve;
use crate::config::{Anchor, Origin, Placement, WidgetSource};
use crate::launch::{self};

pub const INFO: PluginInfo = PluginInfo {
    id: "widgets",
    label: "Widgets",
    icon: "applications-graphics",
    summary: "Widgets du bureau (intégrés et plugins Lua) et leur placement.",
    builtin: true,
};

/// Message de ce paquet, enveloppé pour l'application.
/// Onglets du paquet.
pub const TABS: &[super::SubTab] = &[("bureau", "Sur ce bureau"), ("disponibles", "Disponibles")];

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Widgets(m)))
}

/// Valeur d'un décalage de widget, en pixels signés.
fn en_px_signes(v: f64) -> String {
    format!("{v:+.0} px")
}

/// Valeur de la taille du texte d'un widget : « auto » tant qu'elle n'est pas fixée.
fn en_taille(v: f64) -> String {
    if v <= 0.0 {
        "auto".to_string()
    } else {
        format!("{v:.0} px")
    }
}

/// État propre à ce paquet, porté par le tiroir des réglages.
#[derive(Default)]
pub struct State {
    pub widget_sel: Option<String>,
    /// Widget dont la suppression attend confirmation.
    pub pending_delete: Option<String>,
    /// Astuce de la page (HIG DAT-07), choisie à son ouverture.
    pub tip: Option<&'static crate::tips::Tip>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Active ou retire un type de widget.
    WidgetToggle(String, bool),
    RescanWidgets,
    OpenWidgetDir,
    SelectWidget(String),
    /// Ferme l'astuce de la page (HIG DAT-07).
    TipClose,
    WidgetAnchor(Anchor),
    WidgetX(f32),
    WidgetY(f32),
    WidgetSize(f32),
    WidgetCard(bool),
    /// Écran du widget, et que faire s'il est débranché.
    WidgetScreen(crate::config::WidgetScreen),
    WidgetIfMissing(crate::config::IfMissing),
    /// Taille du widget : couches d'information (HIG SYS-15).
    WidgetFamily(crate::config::WidgetFamily),
    WidgetReset,
    /// Supprime un widget placé dans les réglages (premier appui : arme ; second : confirme).
    DeleteWidget(String),
}

/// Vignette d'aperçu d'une rangée du catalogue, en pixels (HIG SYS-11).
const PREVIEW: (f32, f32) = (112.0, 56.0);

/// Sorte d'aperçu d'un type du catalogue : données simulées pour les
/// widgets intégrés, forme provisoire pour les autres (HIG SYS-11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Preview {
    Clock,
    Calendar,
    System,
    Media,
    Controls,
    Launcher,
    Placeholder,
}

fn preview_of(type_id: &str) -> Preview {
    match type_id {
        "clock" => Preview::Clock,
        "calendar" => Preview::Calendar,
        "system" => Preview::System,
        "media" => Preview::Media,
        "controls" => Preview::Controls,
        "launcher" => Preview::Launcher,
        _ => Preview::Placeholder,
    }
}

impl App {
    /// Aperçu d'un widget du catalogue, rien d'actionnable, sans accent : ce
    /// qu'il montrera, en données simulées réalistes, ou une forme provisoire
    /// quand son rendu n'est pas connu sans l'exécuter (HIG SYS-11).
    fn catalog_preview(&self, type_id: &str) -> Element<'_, UiMsg> {
        use iced::widget::{container, progress_bar};
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let dim = self.ink().dim;
        let faint = Color { a: 0.18, ..fg };
        // La vignette grandit avec le texte (HIG TYP-08), jusqu'à une fois et
        // demie : la description garde sa place dans un tiroir étroit.
        let grow = crate::ui::tokens::text_growth().min(1.5);
        let line = |width: f32| -> Element<'_, UiMsg> {
            container(Space::new().width(width).height(6))
                .style(move |_| iced::widget::container::Style {
                    background: Some(faint.into()),
                    border: Border::default().rounded(3),
                    ..Default::default()
                })
                .into()
        };
        let small = |s: String, c: Color| {
            text(s)
                .font(self.font)
                .size(crate::ui::tokens::ts(11.0))
                .color(c)
        };
        // Pastille ronde (réglages rapides) ou icône d'application (lanceur).
        let tile = move |radius: f32| {
            container(Space::new().width(18).height(18)).style(move |_| {
                iced::widget::container::Style {
                    background: Some(faint.into()),
                    border: Border::default().rounded(radius),
                    ..Default::default()
                }
            })
        };
        let body: Element<'_, UiMsg> = match preview_of(type_id) {
            Preview::Clock => text(super::super::format_now("%H:%M"))
                .font(self.numeric_font(iced::font::Weight::Light))
                .size(crate::ui::tokens::ts_metric(24.0))
                .color(fg)
                .into(),
            Preview::Calendar => column![
                small(super::super::format_now("%A"), dim),
                text(super::super::format_now("%-d"))
                    .font(self.numeric_font(iced::font::Weight::Light))
                    .size(crate::ui::tokens::ts_metric(24.0))
                    .color(fg),
            ]
            .align_x(Alignment::Center)
            .into(),
            Preview::System => {
                let gauge = |label: &'static str, r: f32| {
                    row![
                        small(crate::i18n::tr(label).to_string(), dim).width(30.0 * grow),
                        progress_bar(0.0..=1.0, r)
                            .girth(4)
                            .length(Length::Fixed(56.0))
                            .style(move |_| progress_bar::Style {
                                background: faint.into(),
                                bar: fg.into(),
                                border: Border::default().rounded(2),
                            }),
                    ]
                    .spacing(4)
                    .align_y(Alignment::Center)
                };
                column![gauge("CPU", 0.12), gauge("RAM", 0.26)]
                    .spacing(6)
                    .into()
            }
            Preview::Media => column![
                small(crate::i18n::tr("Titre du morceau").to_string(), fg),
                small(crate::i18n::tr("Artiste").to_string(), dim),
            ]
            .spacing(2)
            .into(),
            Preview::Controls => row![tile(9.0), tile(9.0), tile(9.0)].spacing(8).into(),
            Preview::Launcher => row![tile(5.0), tile(5.0), tile(5.0), tile(5.0)]
                .spacing(4)
                .into(),
            Preview::Placeholder => column![line(64.0), line(40.0)].spacing(6).into(),
        };
        container(body)
            .center_x(PREVIEW.0 * grow)
            .center_y(PREVIEW.1 * grow)
            .style(move |_| iced::widget::container::Style {
                background: Some(Color { a: 0.06, ..fg }.into()),
                border: Border::default().rounded(super::super::design::SMALL_RADIUS),
                ..Default::default()
            })
            .into()
    }

    pub(in crate::ui) fn tab_widgets<'a>(&'a self, p: &'a Panel) -> Element<'a, UiMsg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = self.ink().dim;
        let warn = crate::ui::tone::warning();
        let (mon, ws) = self.panel_ctx().unwrap_or_default();
        let plugin_dir = crate::widgets::plugins::dirs(
            self.config_path
                .parent()
                .unwrap_or(std::path::Path::new(".")),
        )[0]
        .clone();

        // -- Catalogue ---------------------------------------------------------
        let mut catalog = column![].spacing(10);
        for t in &self.cfg.catalog {
            // Actif = présent sur le bureau, qu'il vienne des réglages ou de hyprdesk.lua.
            let active = self
                .cfg
                .widgets
                .iter()
                .any(|w| w.type_id == t.id && !w.in_bar);
            let in_user = self
                .cfg
                .widgets
                .iter()
                .filter(|w| w.type_id == t.id && w.origin == Origin::User && !w.in_bar)
                .count();
            let disabled = self.cfg.disabled.values().filter(|ty| *ty == &t.id).count();
            let source = match &t.source {
                WidgetSource::Builtin => Etat::Integre.libelle().to_string(),
                WidgetSource::Bundled => crate::i18n::tr("plugin livré").to_string(),
                WidgetSource::Plugin(path) => format!(
                    "plugin · {}",
                    path.file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default()
                ),
            };
            let mut info = column![
                row![
                    text(t.name.clone())
                        .font(self.font)
                        .size(crate::ui::tokens::ts(15.0))
                        .color(fg),
                    text(source)
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                        .color(dim)
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(2)
            .width(Length::Fill);
            if !t.description.is_empty() {
                info = info.push(
                    text(t.description.clone())
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                        .color(dim),
                );
            }
            if in_user > 0 {
                info = info.push(
                    text(crate::tr!(
                        "déclaré {in_user} fois dans hyprdesk.lua",
                        in_user = in_user
                    ))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(dim),
                );
            }
            if disabled > 0 {
                info = info.push(
                    text(crate::tr!(
                        "{disabled} instance(s) de hyprdesk.lua désactivée(s) ici",
                        disabled = disabled
                    ))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(dim),
                );
            }
            if let Some(e) = &t.error {
                info = info.push(
                    text(crate::tr!("⚠ {e}", e = e))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                        .color(warn),
                );
            }
            // Un interrupteur sur chaque ligne ; grisé quand le widget ne s'allume
            // pas d'un clic (plugin en erreur ; command / lua, qui demandent leur
            // option exec / render dans hyprdesk.lua, tant qu'aucun n'y est déclaré).
            let actionable = t.error.is_none() && (t.toggleable || active || disabled > 0);
            let control: Element<'_, UiMsg> = if actionable {
                let id = t.id.clone();
                self.focusable(
                    toggler(active).on_toggle(move |on| wrap(Msg::WidgetToggle(id.clone(), on))),
                )
            } else {
                toggler(active).into()
            };
            catalog = catalog.push(
                row![self.catalog_preview(&t.id), info, control]
                    .spacing(12)
                    .align_y(Alignment::Center),
            );
        }

        // -- Widgets de ce bureau ---------------------------------------------
        let mut desk = resolve(&self.cfg, &mon, &ws);
        self.keep_widgets_of(&mon, &mut desk);
        let mut active = column![].spacing(4);
        if desk.widgets.is_empty() {
            active = active.push(
                text("Aucun widget sur ce bureau.")
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
                    .color(dim),
            );
        }
        for w in &desk.widgets {
            let name = self
                .cfg
                .catalog
                .iter()
                .find(|t| t.id == w.type_id)
                .map(|t| t.name.clone())
                .unwrap_or_else(|| w.type_id.clone());
            let from = if w.origin == Origin::Settings {
                crate::i18n::tr("réglages")
            } else {
                "hyprdesk.lua"
            };
            let selected = p.widgets.widget_sel.as_deref() == Some(w.id.as_str());
            let accent = color(g.theme.accent, 1.0);
            // Tous se retirent d'ici : un widget des réglages est supprimé, un
            // widget de hyprdesk.lua désactivé (hd.disable, fichier intact ;
            // « Disponibles » le rallume).
            let armed = p.widgets.pending_delete.as_deref() == Some(w.id.as_str());
            let delete = self.delete_button(armed, wrap(Msg::DeleteWidget(w.id.clone())));
            // Libellé : le type du widget ; l'identifiant seulement s'il en
            // faut un pour distinguer deux widgets du même type.
            let twins = desk
                .widgets
                .iter()
                .filter(|o| o.type_id == w.type_id)
                .count();
            let label = if twins > 1 {
                w.id.as_str()
            } else {
                w.type_id.as_str()
            };
            active = active.push(
                row![
                    self.focusable(
                        button(
                            row![
                                text(name)
                                    .font(self.font)
                                    .size(crate::ui::tokens::ts(13.0))
                                    .width(Length::Fill),
                                text(format!("{label} · {from}"))
                                    .font(self.font)
                                    .size(crate::ui::tokens::ts(12.0))
                                    .color(dim)
                            ]
                            .spacing(8),
                        )
                        .width(Length::Fill)
                        .padding([6, 10])
                        .on_press(wrap(Msg::SelectWidget(w.id.clone())))
                        .style(move |_, s| button::Style {
                            background: if selected {
                                Some(Background::Color(Color { a: 0.3, ..accent }))
                            } else if matches!(s, button::Status::Hovered) {
                                Some(Background::Color(Color { a: 0.08, ..fg }))
                            } else {
                                None
                            },
                            text_color: fg,
                            border: Border::default().rounded(6),
                            ..Default::default()
                        })
                    ),
                    delete,
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            );
        }

        // -- Placement du widget sélectionné ----------------------------------
        let editor: Element<'_, UiMsg> = match self.selected_placement() {
            None => text("Sélectionnez un widget dans la liste ou cliquez-le sur le bureau.")
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim)
                .into(),
            Some((id, pl)) => {
                let accent = color(g.theme.accent, 1.0);
                let cell = |a: Anchor| {
                    let on = pl.anchor == a;
                    self.focusable(
                        button(Space::new().width(26).height(18))
                            .padding(0)
                            .on_press(wrap(Msg::WidgetAnchor(a)))
                            .style(move |_, s| button::Style {
                                background: Some(Background::Color(if on {
                                    accent
                                } else if matches!(s, button::Status::Hovered) {
                                    Color { a: 0.35, ..fg }
                                } else {
                                    Color { a: 0.12, ..fg }
                                })),
                                border: Border::default().rounded(4),
                                ..Default::default()
                            }),
                    )
                };
                let grid = column![
                    row![
                        cell(Anchor::TopLeft),
                        cell(Anchor::Top),
                        cell(Anchor::TopRight)
                    ]
                    .spacing(4),
                    row![
                        cell(Anchor::Left),
                        cell(Anchor::Center),
                        cell(Anchor::Right)
                    ]
                    .spacing(4),
                    row![
                        cell(Anchor::BottomLeft),
                        cell(Anchor::Bottom),
                        cell(Anchor::BottomRight)
                    ]
                    .spacing(4),
                ]
                .spacing(4);
                let user_widget = self
                    .cfg
                    .widgets
                    .iter()
                    .any(|w| w.id == id && w.origin == Origin::User);
                let mut c = column![
                    text(crate::tr!("Placement de « {id} »", id = id))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(13.0))
                        .color(fg),
                    self.field("Ancrage", "", grid),
                    self.field(
                        "Décalage horizontal",
                        "",
                        self.slider(
                            &SliderSpec::new(
                                "widget_x",
                                (-800.0, 800.0),
                                2.0,
                                |v| wrap(Msg::WidgetX(v as f32)),
                                UiMsg::Panel(PanelMsg::Commit),
                            ),
                            f64::from(pl.x.clamp(-800.0, 800.0)),
                            en_px_signes,
                            SliderLook::fixed(13.0, 300, Alignment::Start),
                        ),
                    ),
                    self.field(
                        "Décalage vertical",
                        "",
                        self.slider(
                            &SliderSpec::new(
                                "widget_y",
                                (-800.0, 800.0),
                                2.0,
                                |v| wrap(Msg::WidgetY(v as f32)),
                                UiMsg::Panel(PanelMsg::Commit),
                            ),
                            f64::from(pl.y.clamp(-800.0, 800.0)),
                            en_px_signes,
                            SliderLook::fixed(13.0, 300, Alignment::Start),
                        ),
                    ),
                    self.field(
                        "Taille du texte",
                        "",
                        self.slider(
                            &SliderSpec::new(
                                "widget_size",
                                (0.0, 160.0),
                                1.0,
                                |v| wrap(Msg::WidgetSize(v as f32)),
                                UiMsg::Panel(PanelMsg::Commit),
                            ),
                            f64::from(pl.size.clamp(0.0, 160.0)),
                            en_taille,
                            SliderLook::fixed(13.0, 300, Alignment::Start),
                        ),
                    ),
                    self.field(
                        "Fond en carte",
                        "",
                        self.focusable(toggler(pl.card).on_toggle(|v| wrap(Msg::WidgetCard(v))))
                    ),
                    self.widget_screen_field(&pl),
                    self.focusable(
                        button(
                            text("Réinitialiser la position")
                                .font(self.font)
                                .size(crate::ui::tokens::ts(13.0))
                        )
                        .padding([6, 12])
                        .on_press(wrap(Msg::WidgetReset))
                    ),
                ]
                .spacing(12);
                // Taille du widget, pour ceux qui en ont plusieurs (HIG SYS-15).
                let sized = self
                    .cfg
                    .widgets
                    .iter()
                    .find(|w| w.id == id)
                    .is_some_and(|w| w.kind.has_family());
                if sized {
                    use crate::config::WidgetFamily as F;
                    let choice = |label: &'static str, v: F| {
                        self.focusable(
                            iced::widget::radio(crate::i18n::tr(label), v, Some(pl.family), |v| {
                                wrap(Msg::WidgetFamily(v))
                            })
                            .font(self.font)
                            .size(crate::ui::tokens::ts(14.0)),
                        )
                    };
                    c = c.push(
                        self.field(
                            "Taille du widget",
                            "",
                            row![
                                choice("Petit", F::Small),
                                choice("Moyen", F::Medium),
                                choice("Grand", F::Large),
                                choice("Très grand", F::XLarge),
                            ]
                            .spacing(16)
                            .wrap(),
                        ),
                    );
                }
                if user_widget {
                    c = c.push(text("Widget de hyprdesk.lua : sa position est enregistrée dans les réglages (hd.place), votre fichier n'est pas modifié.").font(self.font).size(crate::ui::tokens::ts(12.0)).color(dim));
                }
                c.into()
            }
        };

        // Astuce de la page (HIG DAT-07) : le glisser d'un widget sur le
        // bureau, à la cadence des astuces, plus une ligne figée.
        let tip = p
            .widgets
            .tip
            .map(|tip| crate::ui::search::tip_row(self, tip, fg, dim, wrap(Msg::TipClose)));
        if self.on_subtab(INFO.id, "bureau") {
            return column![self.section("Sur ce bureau"), tip, active, editor,]
                .spacing(14)
                .into();
        }
        column![
            self.section("Widgets disponibles"),
            tip,
            catalog,
            row![
                self.focusable(
                    button(
                        text("Rechercher de nouveaux widgets")
                            .font(self.font)
                            .size(crate::ui::tokens::ts(13.0))
                    )
                    .padding([6, 12])
                    .on_press(wrap(Msg::RescanWidgets))
                ),
                self.focusable(
                    button(
                        text("Ouvrir le dossier des plugins")
                            .font(self.font)
                            .size(crate::ui::tokens::ts(13.0))
                    )
                    .padding([6, 12])
                    .on_press(wrap(Msg::OpenWidgetDir))
                ),
            ]
            .spacing(8),
            text(crate::tr!(
                "Nouveaux widgets : déposer un fichier .lua dans {dir} — détecté automatiquement.",
                dir = plugin_dir.display()
            ))
            .font(self.font)
            .size(crate::ui::tokens::ts(12.0))
            .color(dim),
        ]
        .spacing(14)
        .into()
    }
    /// Écran du widget sélectionné : principal, tous, ou un écran nommé ; et,
    /// pour un écran nommé, ce qui se passe quand il est débranché.
    fn widget_screen_field<'a>(&'a self, pl: &Placement) -> Element<'a, UiMsg> {
        use crate::config::{IfMissing, WidgetScreen};
        #[derive(Debug, Clone, PartialEq)]
        struct Choice(WidgetScreen, String);
        impl std::fmt::Display for Choice {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(&self.1)
            }
        }
        let primary = self.primary_monitor().unwrap_or_default();
        let mut choices = vec![
            Choice(
                WidgetScreen::Primary,
                crate::tr!("Écran principal ({name})", name = primary.clone()),
            ),
            Choice(WidgetScreen::All, crate::i18n::tr("Tous les écrans").into()),
        ];
        for m in &self.monitors {
            choices.push(Choice(WidgetScreen::Named(m.name.clone()), m.name.clone()));
        }
        if let WidgetScreen::Named(n) = &pl.screen
            && !self.monitors.iter().any(|m| &m.name == n)
        {
            choices.push(Choice(
                pl.screen.clone(),
                crate::tr!("{name} (débranché)", name = n.clone()),
            ));
        }
        let selected = choices.iter().find(|c| c.0 == pl.screen).cloned();
        let mut col = column![
            self.choosable(
                iced::widget::pick_list(choices, selected, |c: Choice| wrap(Msg::WidgetScreen(
                    c.0
                )))
                .font(self.font)
                .text_size(13)
                .width(260)
            )
        ]
        .spacing(8);
        if matches!(pl.screen, WidgetScreen::Named(_)) {
            let missing = |label: &'static str, v: IfMissing| {
                self.focusable(
                    iced::widget::radio(crate::i18n::tr(label), v, Some(pl.if_missing), |v| {
                        wrap(Msg::WidgetIfMissing(v))
                    })
                    .font(self.font)
                    .size(crate::ui::tokens::ts(14.0)),
                )
            };
            col = col.push(
                column![
                    text("Si cet écran est débranché :")
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0)),
                    row![
                        missing("sur l'écran principal", IfMissing::Primary),
                        missing("masqué", IfMissing::Hide),
                    ]
                    .spacing(16),
                ]
                .spacing(6),
            );
        }
        self.field("Écran", "", col)
    }

    pub(in crate::ui) fn selected_placement(&self) -> Option<(String, Placement)> {
        let id = self.panel.as_ref()?.widgets.widget_sel.clone()?;
        let w = self.cfg.widgets.iter().find(|w| w.id == id)?;
        Some((id, w.placement()))
    }
    /// Modifie le placement du widget sélectionné et l'applique tout de suite.
    pub(in crate::ui) fn edit_placement(&mut self, f: impl FnOnce(&mut Placement)) -> Task<UiMsg> {
        let Some((id, mut p)) = self.selected_placement() else {
            return Task::none();
        };
        f(&mut p);
        self.settings.place_widget(&id, p);
        self.apply()
    }

    pub(in crate::ui) fn widgets_update(&mut self, msg: Msg) -> Task<UiMsg> {
        let Some(panel) = self.panel.as_mut() else {
            return Task::none();
        };
        match msg {
            Msg::WidgetToggle(type_id, on) => {
                let anchor = self
                    .cfg
                    .catalog
                    .iter()
                    .find(|t| t.id == type_id)
                    .map(|t| t.anchor)
                    .unwrap_or_default();
                let taken: Vec<String> = self.cfg.widgets.iter().map(|w| w.id.clone()).collect();
                let user_ids: Vec<String> = self
                    .cfg
                    .widgets
                    .iter()
                    .filter(|w| w.type_id == type_id && w.origin == Origin::User && !w.in_bar)
                    .map(|w| w.id.clone())
                    .collect();
                if let Some(id) = self
                    .settings
                    .set_widget_type(&type_id, on, anchor, &user_ids, &taken)
                    && let Some(p) = self.panel.as_mut()
                {
                    p.widgets.widget_sel = Some(id);
                }
                // Éteindre s'applique tout de suite ; allumer demande de relire
                // la config (contenu du widget, fonctions Lua des plugins).
                if on {
                    self.commit_reload()
                } else {
                    self.commit()
                }
            }
            Msg::RescanWidgets => Task::done(UiMsg::ConfigChanged),
            Msg::OpenWidgetDir => {
                let dir = crate::widgets::plugins::dirs(
                    self.config_path
                        .parent()
                        .unwrap_or(std::path::Path::new(".")),
                )[0]
                .clone();
                let _ = std::fs::create_dir_all(&dir);
                // Le bureau reste affiché : la fenêtre s'ouvre dessus, flottante.
                self.run_from_desktop(
                    format!("xdg-open {}", launch::sh_quote(&dir.display().to_string())),
                    true,
                )
            }
            Msg::TipClose => {
                panel.widgets.tip = None;
                Task::none()
            }
            Msg::SelectWidget(id) => {
                panel.widgets.widget_sel = Some(id);
                Task::none()
            }
            Msg::WidgetAnchor(a) => {
                let t = self.edit_placement(|p| {
                    p.anchor = a;
                    p.x = 0.0;
                    p.y = 0.0;
                });
                Task::batch([t, self.save()])
            }
            Msg::WidgetX(v) => self.edit_placement(|p| p.x = v),
            Msg::WidgetY(v) => self.edit_placement(|p| p.y = v),
            Msg::WidgetSize(v) => self.edit_placement(|p| p.size = v),
            Msg::WidgetCard(v) => {
                let t = self.edit_placement(|p| p.card = v);
                Task::batch([t, self.save()])
            }
            Msg::WidgetScreen(v) => {
                let t = self.edit_placement(|p| p.screen = v);
                Task::batch([t, self.save()])
            }
            Msg::WidgetIfMissing(v) => {
                let t = self.edit_placement(|p| p.if_missing = v);
                Task::batch([t, self.save()])
            }
            Msg::WidgetFamily(v) => {
                let t = self.edit_placement(|p| p.family = v);
                Task::batch([t, self.save()])
            }
            Msg::DeleteWidget(id) => {
                let Some(p) = self.panel.as_mut() else {
                    return Task::none();
                };
                if p.widgets.pending_delete.as_deref() != Some(id.as_str()) {
                    p.widgets.pending_delete = Some(id);
                    return Task::none();
                }
                p.widgets.pending_delete = None;
                if p.widgets.widget_sel.as_deref() == Some(id.as_str()) {
                    p.widgets.widget_sel = None;
                }
                // Widget de hyprdesk.lua : désactivé ici, son fichier n'est pas touché.
                match self.cfg.widgets.iter().find(|w| w.id == id) {
                    Some(w) if w.origin == Origin::User => {
                        self.settings.disabled.insert(id, w.type_id.clone());
                    }
                    _ => self.settings.remove_widget(&id),
                }
                self.commit()
            }
            Msg::WidgetReset => {
                let Some(id) = panel.widgets.widget_sel.clone() else {
                    return Task::none();
                };
                if let Some(w) = self.settings.widgets.iter_mut().find(|w| w.id == id) {
                    let anchor = self
                        .cfg
                        .catalog
                        .iter()
                        .find(|t| t.id == w.type_id)
                        .map(|t| t.anchor)
                        .unwrap_or_default();
                    // Position remise à zéro ; l'écran choisi et la taille
                    // du widget (son contenu, HIG SYS-15) restent.
                    let (screen, if_missing) = (w.screen.clone(), w.if_missing);
                    w.set_placement(&Placement {
                        family: w.family,
                        anchor,
                        x: 0.0,
                        y: 0.0,
                        size: 0.0,
                        card: true,
                        screen,
                        if_missing,
                    });
                } else {
                    self.settings.placements.remove(&id);
                }
                // La position d'origine d'un widget de hyprdesk.lua n'existe que dans Lua.
                self.commit_reload()
            }
        }
    }
}

impl App {
    /// Astuce de la page Widgets (HIG DAT-07), à son ouverture : une fois,
    /// au plus une par jour toutes surfaces confondues.
    pub(in crate::ui) fn widgets_tip(&mut self) -> Task<UiMsg> {
        if self.panel.as_ref().is_none_or(|p| p.widgets.tip.is_some()) {
            return Task::none();
        }
        let (tip, saved) = self.take_tip(crate::tips::Surface::Settings);
        if let Some(p) = self.panel.as_mut() {
            p.widgets.tip = tip;
        }
        saved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Chaque widget intégré qu'on active d'un clic a son aperçu en données
    /// simulées ; commande, Lua et plugins prennent la forme provisoire
    /// (HIG SYS-11).
    #[test]
    fn chaque_widget_integre_a_son_apercu() {
        for b in crate::widgets::BUILTINS {
            let p = preview_of(b.id);
            if matches!(b.id, "command" | "lua") {
                assert_eq!(p, Preview::Placeholder, "{}", b.id);
            } else {
                assert_ne!(p, Preview::Placeholder, "{} sans aperçu", b.id);
            }
        }
        assert_eq!(preview_of("network"), Preview::Placeholder);
    }
}
