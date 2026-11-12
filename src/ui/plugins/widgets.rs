//! Paquet intégré « Widgets » : Widgets du bureau (intégrés et plugins Lua) et leur placement.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{Space, button, column, row, slider, toggler};
use iced::{Alignment, Background, Border, Color, Element, Length, Task};

use super::super::panel::{Panel, PanelMsg};
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

/// État propre à ce paquet, porté par le tiroir des réglages.
#[derive(Default)]
pub struct State {
    pub widget_sel: Option<String>,
    /// Widget dont la suppression attend confirmation.
    pub pending_delete: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    /// Active ou retire un type de widget.
    WidgetToggle(String, bool),
    RescanWidgets,
    OpenWidgetDir,
    SelectWidget(String),
    WidgetAnchor(Anchor),
    WidgetX(f32),
    WidgetY(f32),
    WidgetSize(f32),
    WidgetCard(bool),
    /// Écran du widget, et que faire s'il est débranché.
    WidgetScreen(crate::config::WidgetScreen),
    WidgetIfMissing(crate::config::IfMissing),
    WidgetReset,
    /// Supprime un widget placé dans les réglages (premier appui : arme ; second : confirme).
    DeleteWidget(String),
}

impl App {
    pub(in crate::ui) fn tab_widgets<'a>(&'a self, p: &'a Panel) -> Element<'a, UiMsg> {
        let g = &self.cfg.general;
        let fg = color(g.theme.fg, 1.0);
        let dim = self.ink().dim;
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
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
                WidgetSource::Builtin => crate::i18n::tr("intégré").to_string(),
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
                    text(t.name.clone()).font(self.font).size(15).color(fg),
                    text(source).font(self.font).size(12).color(dim)
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
                        .size(12)
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
                    .size(12)
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
                    .size(12)
                    .color(dim),
                );
            }
            if let Some(e) = &t.error {
                info = info.push(
                    text(crate::tr!("⚠ {e}", e = e))
                        .font(self.font)
                        .size(12)
                        .color(warn),
                );
            }
            let control: Element<'_, UiMsg> = if t.error.is_some() {
                Space::new().into()
            } else if t.toggleable {
                let id = t.id.clone();
                toggler(active)
                    .on_toggle(move |on| wrap(Msg::WidgetToggle(id.clone(), on)))
                    .into()
            } else if active || disabled > 0 {
                // command / lua : pas de création d'un clic, mais on peut éteindre ou rallumer.
                let id = t.id.clone();
                toggler(active)
                    .on_toggle(move |on| wrap(Msg::WidgetToggle(id.clone(), on)))
                    .into()
            } else {
                text("dans hyprdesk.lua")
                    .font(self.font)
                    .size(12)
                    .color(dim)
                    .into()
            };
            catalog = catalog.push(row![info, control].spacing(12).align_y(Alignment::Center));
        }

        // -- Widgets de ce bureau ---------------------------------------------
        let mut desk = resolve(&self.cfg, &mon, &ws);
        self.keep_widgets_of(&mon, &mut desk);
        let mut active = column![].spacing(4);
        if desk.widgets.is_empty() {
            active = active.push(
                text("Aucun widget sur ce bureau.")
                    .font(self.font)
                    .size(13)
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
            // Seuls les widgets placés dans les réglages se suppriment ici
            // (ceux de hyprdesk.lua restent maîtres dans leur fichier).
            let delete: Element<'_, UiMsg> = if w.origin == Origin::Settings {
                let armed = p.widgets.pending_delete.as_deref() == Some(w.id.as_str());
                self.delete_button(armed, wrap(Msg::DeleteWidget(w.id.clone())))
            } else {
                Space::new().width(0).height(0).into()
            };
            active = active.push(
                row![
                    button(
                        row![
                            text(name).font(self.font).size(14).width(Length::Fill),
                            text(format!("{} · {from}", w.id))
                                .font(self.font)
                                .size(12)
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
                    }),
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
                .size(12)
                .color(dim)
                .into(),
            Some((id, pl)) => {
                let accent = color(g.theme.accent, 1.0);
                let cell = |a: Anchor| {
                    let on = pl.anchor == a;
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
                        })
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
                        .size(14)
                        .color(fg),
                    self.field("Ancrage", "", grid),
                    self.field(
                        "Décalage horizontal",
                        "",
                        row![
                            slider(-800.0..=800.0f32, pl.x.clamp(-800.0, 800.0), |v| {
                                wrap(Msg::WidgetX(v))
                            })
                            .step(2.0f32)
                            .on_release(UiMsg::Panel(PanelMsg::Commit))
                            .width(300),
                            text(format!("{:+.0} px", pl.x)).font(self.font).size(13),
                        ]
                        .spacing(12),
                    ),
                    self.field(
                        "Décalage vertical",
                        "",
                        row![
                            slider(-800.0..=800.0f32, pl.y.clamp(-800.0, 800.0), |v| {
                                wrap(Msg::WidgetY(v))
                            })
                            .step(2.0f32)
                            .on_release(UiMsg::Panel(PanelMsg::Commit))
                            .width(300),
                            text(format!("{:+.0} px", pl.y)).font(self.font).size(13),
                        ]
                        .spacing(12),
                    ),
                    self.field(
                        "Taille du texte",
                        "",
                        row![
                            slider(0.0..=160.0f32, pl.size.clamp(0.0, 160.0), |v| wrap(
                                Msg::WidgetSize(v)
                            ))
                            .step(1.0f32)
                            .on_release(UiMsg::Panel(PanelMsg::Commit))
                            .width(300),
                            text(if pl.size <= 0.0 {
                                "auto".to_string()
                            } else {
                                format!("{:.0} px", pl.size)
                            })
                            .font(self.font)
                            .size(13),
                        ]
                        .spacing(12),
                    ),
                    self.field(
                        "Fond en carte",
                        "",
                        toggler(pl.card).on_toggle(|v| wrap(Msg::WidgetCard(v)))
                    ),
                    self.widget_screen_field(&pl),
                    button(text("Réinitialiser la position").font(self.font).size(13))
                        .padding([6, 12])
                        .on_press(wrap(Msg::WidgetReset)),
                ]
                .spacing(12);
                if user_widget {
                    c = c.push(text("Widget de hyprdesk.lua : sa position est enregistrée dans les réglages (hd.place), votre fichier n'est pas modifié.").font(self.font).size(12).color(dim));
                }
                c.into()
            }
        };

        if self.on_subtab(INFO.id, "bureau") {
            return column![
                self.section("Sur ce bureau"),
                text("Astuce : réglages ouverts, glissez un widget directement sur le bureau.")
                    .font(self.font)
                    .size(12)
                    .color(dim),
                active,
                editor,
            ]
            .spacing(14)
            .into();
        }
        column![
            self.section("Widgets disponibles"),
            catalog,
            row![
                button(
                    text("Rechercher de nouveaux widgets")
                        .font(self.font)
                        .size(13)
                )
                .padding([6, 12])
                .on_press(wrap(Msg::RescanWidgets)),
                button(
                    text("Ouvrir le dossier des plugins")
                        .font(self.font)
                        .size(13)
                )
                .padding([6, 12])
                .on_press(wrap(Msg::OpenWidgetDir)),
            ]
            .spacing(8),
            text(crate::tr!(
                "Nouveaux widgets : déposer un fichier .lua dans {dir} — détecté automatiquement.",
                dir = plugin_dir.display()
            ))
            .font(self.font)
            .size(12)
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
            iced::widget::pick_list(choices, selected, |c: Choice| wrap(Msg::WidgetScreen(c.0)))
                .font(self.font)
                .text_size(13)
                .width(260)
        ]
        .spacing(8);
        if matches!(pl.screen, WidgetScreen::Named(_)) {
            let missing = |label: &'static str, v: IfMissing| {
                iced::widget::radio(crate::i18n::tr(label), v, Some(pl.if_missing), |v| {
                    wrap(Msg::WidgetIfMissing(v))
                })
                .font(self.font)
                .size(14)
            };
            col = col.push(
                column![
                    text("Si cet écran est débranché :")
                        .font(self.font)
                        .size(12),
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
                self.settings.remove_widget(&id);
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
                    // Position remise à zéro ; l'écran choisi reste.
                    let (screen, if_missing) = (w.screen.clone(), w.if_missing);
                    w.set_placement(&Placement {
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
