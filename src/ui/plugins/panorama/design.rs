//! Réglages visuels et composition du Panorama, dans le plugin existant.
use super::{App, Msg, UiMsg, wrap};
use crate::i18n::text;
use crate::panorama::design::{Shortcut, Tile};
use iced::widget::{button, column, pick_list, row, slider, text_input, toggler};
use iced::{Alignment, Element, Length, Task};

#[derive(Debug, Clone, PartialEq, Eq)]
struct Choice(String);
impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let label = match self.0.as_str() {
            "digital" => "Numérique",
            "flip" => "À volets",
            "analog" => "À aiguilles",
            "cards" => "Cartes",
            "gauges" => "Cadrans",
            "graphs" => "Courbes",
            "standard" => "Classique",
            "vinyl" => "Vinyle",
            "compact" => "Compact",
            s => s,
        };
        write!(f, "{}", crate::i18n::tr(label))
    }
}

impl App {
    fn panorama_design_error(&self) -> Element<'_, UiMsg> {
        text(self.panorama.design_error.clone().unwrap_or_default())
            .size(12)
            .color(iced::Color::from_rgb8(255, 160, 120))
            .into()
    }
    fn panorama_design_input(&self, label: &'static str, key: &'static str) -> Element<'_, UiMsg> {
        let c = self.panorama_cfg();
        let value = self
            .panorama
            .design_drafts
            .get(key)
            .cloned()
            .unwrap_or_else(|| c.design.text(key).unwrap_or("").into());
        column![
            text(label).font(self.font).size(12),
            row![
                text_input("", &value)
                    .on_input(move |s| wrap(Msg::DesignText(key.into(), s)))
                    .on_submit(wrap(Msg::DesignApply(key.into())))
                    .size(13),
                button(text("Appliquer").size(12)).on_press(wrap(Msg::DesignApply(key.into())))
            ]
            .spacing(8)
        ]
        .spacing(5)
        .into()
    }
    fn panorama_design_slider(
        &self,
        label: &'static str,
        key: &'static str,
        min: f32,
        max: f32,
        step: f32,
    ) -> Element<'_, UiMsg> {
        let c = self.panorama_cfg();
        let v = c.design.number(key).unwrap_or(min);
        column![
            text(label).font(self.font).size(12),
            row![
                slider(min..=max, v, move |v| wrap(Msg::DesignNumber(
                    key.into(),
                    v
                )))
                .step(step)
                .on_release(wrap(Msg::DesignCommit)),
                text(format!("{v:.2}")).size(12).width(50)
            ]
            .spacing(12)
            .align_y(Alignment::Center)
        ]
        .spacing(5)
        .into()
    }
    fn panorama_design_choice(
        &self,
        label: &'static str,
        key: &'static str,
        values: &[&str],
    ) -> Element<'_, UiMsg> {
        let c = self.panorama_cfg();
        column![
            text(label).font(self.font).size(12),
            pick_list(
                values
                    .iter()
                    .map(|s| Choice(s.to_string()))
                    .collect::<Vec<_>>(),
                Some(Choice(c.design.text(key).unwrap_or("").to_string())),
                move |s: Choice| wrap(Msg::DesignChoice(key.into(), s.0))
            )
            .text_size(13)
        ]
        .spacing(5)
        .into()
    }
    pub(super) fn panorama_style_tab(&self) -> Element<'_, UiMsg> {
        let c = self.panorama_cfg();
        let mut presets = column![self.section("Compositions de départ")].spacing(10);
        for group in [
            ("forest", "Forêt", "telemetry", "Télémétrie"),
            ("studio", "Studio", "applications", "Applications"),
        ] {
            let mut r = row![].spacing(8);
            for (id, label) in [(group.0, group.1), (group.2, group.3)] {
                let selected = c.design.preset == id;
                r = r.push(
                    button(text(label).size(13))
                        .padding([10, 14])
                        .width(Length::Fill)
                        .style(move |t, s| {
                            if selected {
                                button::primary(t, s)
                            } else {
                                button::secondary(t, s)
                            }
                        })
                        .on_press(wrap(Msg::Preset(id.into()))),
                );
            }
            presets = presets.push(r);
        }
        let mut col=presets.push(text("Un préréglage remplace la composition et le style ; votre image et vos raccourcis sont conservés.").size(12))
            .push(self.panorama_design_error())
            .push(self.section("Fond et couleurs"))
            .push(self.panorama_design_input("Image de fond (chemin, vide pour le fond du style)","wallpaper"))
            .push(self.panorama_design_input("Fond (#rrggbb, vide pour le thème)","background"))
            .push(self.panorama_design_input("Texte (#rrggbb, vide pour le thème)","foreground"))
            .push(self.panorama_design_input("Accent (#rrggbb, vide pour le thème)","accent"))
            .push(self.panorama_design_slider("Assombrissement de l'image","dimming",0.0,1.0,0.01))
            .push(self.panorama_design_slider("Opacité des cartes","card_opacity",0.0,1.0,0.01))
            .push(self.panorama_design_slider("Opacité des contours","border_opacity",0.0,1.0,0.01))
            .push(self.panorama_design_input("Police de caractères (vide pour le thème)","font_family"))
            .push(self.section("Dimensions"));
        for (label, key, min, max, step) in [
            ("Arrondi des cartes", "radius", 0.0, 64.0, 1.0),
            ("Espacement des zones", "gap", 0.0, 64.0, 1.0),
            ("Marge du bandeau", "padding", 0.0, 64.0, 1.0),
            ("Taille du texte", "text_scale", 0.5, 2.0, 0.05),
        ] {
            col = col.push(self.panorama_design_slider(label, key, min, max, step));
        }
        col = col
            .push(self.section("Horloge"))
            .push(self.panorama_design_choice(
                "Style de l'horloge",
                "clock_style",
                crate::panorama::design::CLOCKS,
            ));
        for (key, label) in [
            ("seconds", "Afficher les secondes"),
            ("date", "Afficher la date"),
        ] {
            col = col.push(
                toggler(c.design.flag(key).unwrap_or(false))
                    .label(label)
                    .text_size(13)
                    .on_toggle(move |v| wrap(Msg::DesignFlag(key.into(), v))),
            );
        }
        col=col.push(self.section("Mesures système"))
            .push(self.panorama_design_choice("Présentation des mesures","system_style",crate::panorama::design::SYSTEMS))
            .push(self.panorama_design_input("Mesures (cpu, ram, disk, load, uptime, cpu_temp, gpu_temp)","metrics"))
            .push(self.panorama_design_slider("Historique des courbes (secondes)","history_seconds",15.0,600.0,1.0))
            .push(text("Les capteurs absents affichent —. Les courbes enregistrent les relevés réels pendant l'affichage.").size(12))
            .push(self.section("Lecteur et effets"))
            .push(self.panorama_design_choice("Présentation du lecteur","media_style",crate::panorama::design::MEDIA));
        for (key, label) in [
            ("effects", "Décor 3D et lumières"),
            ("controls", "Afficher les commandes du bandeau"),
        ] {
            col = col.push(
                toggler(c.design.flag(key).unwrap_or(false))
                    .label(label)
                    .text_size(13)
                    .on_toggle(move |v| wrap(Msg::DesignFlag(key.into(), v))),
            );
        }
        col.into()
    }

    pub(super) fn panorama_composition_tab(&self) -> Element<'_, UiMsg> {
        let c = self.panorama_cfg();
        let selected = self
            .panorama
            .selected_tile
            .min(c.design.tiles.len().saturating_sub(1));
        let mut col=column![self.section("Composition libre"),
            text("Choisissez une zone puis réglez son widget, sa position et sa taille. En portrait, la composition pivote et les widgets restent droits.").size(12),
            button(text("Utiliser cette composition").size(13)).on_press(wrap(Msg::Layout(crate::panorama::config::Layout::Dashboard))),
            self.panorama_design_error(),
        ].spacing(12);
        for (i, t) in c.design.tiles.iter().enumerate() {
            let on = i == selected;
            col = col.push(
                button(
                    text(format!(
                        "{} · {}",
                        i + 1,
                        crate::i18n::tr(crate::panorama::page_label(&t.widget))
                    ))
                    .size(13),
                )
                .width(Length::Fill)
                .style(move |th, s| {
                    if on {
                        button::primary(th, s)
                    } else {
                        button::secondary(th, s)
                    }
                })
                .on_press(wrap(Msg::TileSelect(i))),
            );
        }
        col = col.push(
            row![
                button(text("Ajouter une zone").size(12))
                    .on_press_maybe((c.design.tiles.len() < 16).then(|| wrap(Msg::TileAdd))),
                button(text("Retirer la zone").size(12))
                    .on_press_maybe((c.design.tiles.len() > 1).then(|| wrap(Msg::TileRemove)))
            ]
            .spacing(8),
        );
        if let Some(tile) = c.design.tiles.get(selected) {
            let mut options: Vec<String> = crate::panorama::PAGES
                .iter()
                .map(|(id, _)| id.to_string())
                .collect();
            options.extend(self.cfg.widgets.iter().map(|w| format!("widget:{}", w.id)));
            col = col.push(text("Widget de la zone").size(12)).push(
                pick_list(options, Some(tile.widget.clone()), |v| {
                    wrap(Msg::TileWidget(v))
                })
                .text_size(13),
            );
            for (key, label, v, max) in [
                ("x", "Position horizontale (%)", tile.x, 1.0 - tile.w),
                ("y", "Position verticale (%)", tile.y, 1.0 - tile.h),
                ("w", "Largeur (%)", tile.w, 1.0 - tile.x),
                ("h", "Hauteur (%)", tile.h, 1.0 - tile.y),
            ] {
                let minimum = if key == "w" || key == "h" { 5.0 } else { 0.0 };
                col = col.push(
                    column![
                        text(label).size(12),
                        row![
                            slider(minimum..=(max * 100.0).max(minimum), v * 100.0, move |n| {
                                wrap(Msg::TileNumber(key.into(), n / 100.0))
                            })
                            .step(1.0_f32)
                            .on_release(wrap(Msg::DesignCommit)),
                            text(format!("{:.0}", v * 100.0)).width(36).size(12)
                        ]
                        .spacing(10)
                    ]
                    .spacing(5),
                );
            }
        }
        col.push(text("Les zones peuvent se superposer volontairement. La zone placée en dernier apparaît au-dessus.").size(12)).into()
    }

    pub(super) fn panorama_shortcuts_tab(&self) -> Element<'_, UiMsg> {
        let c = self.panorama_cfg();
        let draft = &self.panorama.shortcut_draft;
        let mut col = column![
            self.section("Raccourcis tactiles"),
            text("Chaque touche lance la commande que vous choisissez, uniquement à l'appui.")
                .size(12),
            self.panorama_design_error(),
            self.panorama_design_slider("Nombre de colonnes", "shortcut_columns", 2.0, 8.0, 1.0)
        ]
        .spacing(12);
        for (i, s) in c.design.shortcuts.iter().enumerate() {
            col = col.push(
                row![
                    text(s.name.clone()).size(13).width(Length::Fill),
                    button(text("Retirer").size(12)).on_press(wrap(Msg::ShortcutRemove(i)))
                ]
                .spacing(8),
            );
        }
        for (i, label, v) in [
            (0, "Nom du raccourci", &draft.0),
            (1, "Nom de l'icône", &draft.1),
            (2, "Commande à lancer", &draft.2),
        ] {
            col = col.push(
                column![
                    text(label).size(12),
                    text_input("", v)
                        .on_input(move |v| wrap(Msg::ShortcutText(i, v)))
                        .on_submit(wrap(Msg::ShortcutAdd))
                        .size(13)
                ]
                .spacing(5),
            );
        }
        col.push(
            button(text("Ajouter le raccourci").size(13)).on_press_maybe(
                (!draft.0.trim().is_empty()
                    && !draft.2.trim().is_empty()
                    && c.design.shortcuts.len() < 64)
                    .then(|| wrap(Msg::ShortcutAdd)),
            ),
        )
        .into()
    }

    pub(super) fn panorama_design_update(&mut self, m: Msg) -> Task<UiMsg> {
        let mut c = self.panorama_cfg();
        match m {
            Msg::Preset(id) => {
                if let Err(e) = c.design.apply_preset(&id) {
                    self.panorama.design_error = Some(e);
                    return Task::none();
                }
                c.layout = crate::panorama::config::Layout::Dashboard;
                self.panorama.design_drafts.clear();
                self.panorama.selected_tile = 0;
            }
            Msg::DesignText(key, v) => {
                self.panorama.design_drafts.insert(key, v);
                return Task::none();
            }
            Msg::DesignApply(key) => {
                let value = self
                    .panorama
                    .design_drafts
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| c.design.text(&key).unwrap_or("").into());
                if let Err(e) = c.design.set(&key, &value) {
                    self.panorama.design_error = Some(e);
                    return Task::none();
                }
                self.panorama.design_drafts.remove(&key);
            }
            Msg::DesignChoice(key, v) => {
                if let Err(e) = c.design.set(&key, &v) {
                    self.panorama.design_error = Some(e);
                    return Task::none();
                }
            }
            Msg::DesignNumber(key, v) => {
                if let Err(e) = c.design.set(&key, &v.to_string()) {
                    self.panorama.design_error = Some(e);
                    return Task::none();
                }
                self.settings.panorama = Some(c);
                return Task::none();
            }
            Msg::DesignFlag(key, v) => {
                let _ = c.design.set(&key, &v.to_string());
            }
            Msg::DesignCommit => {}
            Msg::TileSelect(i) => {
                self.panorama.selected_tile = i;
                return Task::none();
            }
            Msg::TileWidget(w) => {
                if let Some(t) = c.design.tiles.get_mut(self.panorama.selected_tile) {
                    t.widget = w;
                }
            }
            Msg::TileNumber(key, v) => {
                if let Some(t) = c.design.tiles.get_mut(self.panorama.selected_tile) {
                    match key.as_str() {
                        "x" => t.x = v.clamp(0.0, 1.0 - t.w),
                        "y" => t.y = v.clamp(0.0, 1.0 - t.h),
                        "w" => t.w = v.clamp(0.05, 1.0 - t.x),
                        "h" => t.h = v.clamp(0.05, 1.0 - t.y),
                        _ => {}
                    }
                }
                self.settings.panorama = Some(c);
                return Task::none();
            }
            Msg::TileAdd => {
                if c.design.tiles.len() < 16 {
                    c.design.tiles.push(Tile::new("clock", 0.0, 0.0, 0.3, 0.5));
                    self.panorama.selected_tile = c.design.tiles.len() - 1;
                }
            }
            Msg::TileRemove => {
                if c.design.tiles.len() > 1 {
                    c.design
                        .tiles
                        .remove(self.panorama.selected_tile.min(c.design.tiles.len() - 1));
                    self.panorama.selected_tile = 0;
                }
            }
            Msg::ShortcutText(i, v) => {
                match i {
                    0 => self.panorama.shortcut_draft.0 = v,
                    1 => self.panorama.shortcut_draft.1 = v,
                    _ => self.panorama.shortcut_draft.2 = v,
                };
                return Task::none();
            }
            Msg::ShortcutAdd => {
                let (name, icon, command) = self.panorama.shortcut_draft.clone();
                if name.trim().is_empty()
                    || command.trim().is_empty()
                    || c.design.shortcuts.len() >= 64
                {
                    return Task::none();
                }
                c.design.shortcuts.push(Shortcut {
                    name: name.trim().into(),
                    icon: if icon.is_empty() {
                        "application-x-executable".into()
                    } else {
                        icon
                    },
                    command,
                });
                self.panorama.shortcut_draft = Default::default();
            }
            Msg::ShortcutRemove(i) => {
                if i < c.design.shortcuts.len() {
                    c.design.shortcuts.remove(i);
                }
            }
            _ => return Task::none(),
        }
        self.panorama.design_error = None;
        self.settings.panorama = Some(c);
        self.commit()
    }
}
