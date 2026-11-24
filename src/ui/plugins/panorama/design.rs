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
            .size(crate::ui::tokens::ts(12.0))
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
        let mut line = row![
            text_input("", &value)
                .on_input(move |s| wrap(Msg::DesignText(key.into(), s)))
                .on_submit(wrap(Msg::DesignApply(key.into())))
                .size(crate::ui::tokens::ts(13.0)),
            button(text("Appliquer").size(crate::ui::tokens::ts(12.0)))
                .on_press(wrap(Msg::DesignApply(key.into())))
        ]
        .spacing(8);
        // Image de fond : choisie aussi dans un explorateur (portail).
        if key == "wallpaper" {
            line = line.push(
                button(text("Parcourir…").size(crate::ui::tokens::ts(12.0)))
                    .on_press(wrap(Msg::DesignBrowse(key.into()))),
            );
        }
        column![
            text(label)
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0)),
            line
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
            text(label)
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0)),
            row![
                slider(min..=max, v, move |v| wrap(Msg::DesignNumber(
                    key.into(),
                    v
                )))
                .step(step)
                .on_release(wrap(Msg::DesignCommit)),
                text(format!("{v:.2}"))
                    .size(crate::ui::tokens::ts(12.0))
                    .width(50)
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
            text(label)
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0)),
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
    /// Profil du bloc de raccourcis : vidéo, audio, télécommande, système.
    fn panorama_profile_choice(&self) -> Element<'_, UiMsg> {
        let c = self.panorama_cfg();
        let cur = crate::panorama::design::PROFILES
            .iter()
            .copied()
            .find(|p| *p == c.design.shortcut_profile);
        let mut r = row![].spacing(16);
        for id in crate::panorama::design::PROFILES.iter().copied() {
            let (label, _) = crate::panorama::design::profile_label(id);
            r = r.push(
                iced::widget::radio(crate::i18n::tr(label), id, cur, |v: &'static str| {
                    wrap(Msg::DesignChoice("shortcut_profile".into(), v.into()))
                })
                .text_size(13),
            );
        }
        r.into()
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
                    button(text(label).size(crate::ui::tokens::ts(13.0)))
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
        let mut col=presets.push(text("Un préréglage remplace la composition et le style ; votre image et vos raccourcis sont conservés.").size(crate::ui::tokens::ts(12.0)))
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
            // Identifiants tapés à la main et durée en secondes : avancé.
            .push(self.adv("Capteurs et historique des courbes", column![
                self.panorama_design_input("Mesures (cpu, ram, gpu, vram, disk, load, uptime, cpu_temp, gpu_temp)","metrics"),
                self.panorama_design_slider("Historique des courbes (secondes)","history_seconds",15.0,600.0,1.0),
            ].spacing(10)))
            .push(text("Les capteurs absents affichent —. Les courbes enregistrent les relevés réels pendant l'affichage.").size(crate::ui::tokens::ts(12.0)))
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

    /// Zones de la page réglée (vue par défaut ou page du carrousel).
    pub(super) fn panorama_board_editor(&self) -> Element<'_, UiMsg> {
        let c = self.panorama_cfg();
        let board = self.panorama.edit_board.filter(|i| *i < c.pages.len());
        let tiles = board_tiles(&c, board).clone();
        let selected = self
            .panorama
            .selected_tile
            .min(tiles.len().saturating_sub(1));
        let mut col = column![
            self.section("Composition"),
            text("Choisissez une zone puis réglez son widget, sa position et sa taille. En portrait, la composition pivote et les widgets restent droits.").size(crate::ui::tokens::ts(12.0)),
        ]
        .spacing(12);
        col = match board {
            None if c.layout != crate::panorama::config::Layout::Dashboard => col.push(
                button(text("Utiliser cette composition").size(crate::ui::tokens::ts(13.0))).on_press(wrap(Msg::Layout(crate::panorama::config::Layout::Dashboard))),
            ),
            None => col,
            Some(_) => col.push(text("Mesures : une tuile par mesure, en graphique (metric:cpu) ou en jauge (metric:cpu:gauges).").size(crate::ui::tokens::ts(12.0))),
        };
        col = col.push(self.panorama_design_error());
        for (i, t) in tiles.iter().enumerate() {
            let on = i == selected;
            col = col.push(
                button(
                    text(format!(
                        "{} · {}",
                        i + 1,
                        crate::i18n::tr(crate::panorama::page_label(&t.widget))
                    ))
                    .size(crate::ui::tokens::ts(13.0)),
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
                button(text("Ajouter une zone").size(crate::ui::tokens::ts(12.0)))
                    .on_press_maybe((tiles.len() < 16).then(|| wrap(Msg::TileAdd))),
                button(text("Retirer la zone").size(crate::ui::tokens::ts(12.0)))
                    .on_press_maybe((tiles.len() > 1).then(|| wrap(Msg::TileRemove)))
            ]
            .spacing(8),
        );
        if let Some(tile) = tiles.get(selected) {
            let mut options: Vec<String> = crate::panorama::PAGES
                .iter()
                .map(|(id, _)| id.to_string())
                .collect();
            options.push("playlist".into());
            for m in crate::panorama::design::METRICS {
                options.push(format!("metric:{m}"));
                options.push(format!("metric:{m}:gauges"));
                options.push(format!("metric:{m}:cards"));
            }
            options.extend(self.cfg.widgets.iter().map(|w| format!("widget:{}", w.id)));
            col = col
                .push(text("Widget de la zone").size(crate::ui::tokens::ts(12.0)))
                .push(
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
                        text(label).size(crate::ui::tokens::ts(12.0)),
                        row![
                            slider(minimum..=(max * 100.0).max(minimum), v * 100.0, move |n| {
                                wrap(Msg::TileNumber(key.into(), n / 100.0))
                            })
                            .step(1.0_f32)
                            .on_release(wrap(Msg::DesignCommit)),
                            text(format!("{:.0}", v * 100.0))
                                .width(36)
                                .size(crate::ui::tokens::ts(12.0))
                        ]
                        .spacing(10)
                    ]
                    .spacing(5),
                );
            }
        }
        col.push(text("Les zones peuvent se superposer volontairement. La zone placée en dernier apparaît au-dessus.").size(crate::ui::tokens::ts(12.0))).into()
    }

    pub(super) fn panorama_shortcuts_tab(&self) -> Element<'_, UiMsg> {
        let c = self.panorama_cfg();
        let draft = &self.panorama.shortcut_draft;
        let mut col = column![
            self.section("Profil du bloc"),
            self.panorama_profile_choice(),
            text("Le bloc de raccourcis du Panorama s'ouvre sur ce profil ; son sélecteur en change au doigt. Vidéo, Audio et Télécommande ont leurs touches intégrées (la télécommande envoie flèches, OK et Retour à la fenêtre active) ; Système affiche la liste ci-dessous.")
                .size(crate::ui::tokens::ts(12.0)),
            self.section("Raccourcis tactiles (Système)"),
            text("Chaque touche lance la commande que vous choisissez, uniquement à l'appui.")
                .size(crate::ui::tokens::ts(12.0)),
            self.panorama_design_error(),
            self.panorama_design_slider("Nombre de colonnes", "shortcut_columns", 2.0, 8.0, 1.0)
        ]
        .spacing(12);
        let n = c.design.shortcuts.len();
        for (i, s) in c.design.shortcuts.iter().enumerate() {
            col = col.push(
                row![
                    self.panorama_icon_thumb(&s.icon, 24.0),
                    self.panorama_move_buttons(i, n, |i, d| wrap(Msg::ShortcutMove(i, d))),
                    text(s.name.clone())
                        .size(crate::ui::tokens::ts(13.0))
                        .width(Length::Fill),
                    button(text("Retirer").size(crate::ui::tokens::ts(12.0)))
                        .on_press(wrap(Msg::ShortcutRemove(i)))
                ]
                .spacing(8)
                .align_y(iced::Alignment::Center),
            );
        }
        col = col.push(self.panorama_icons_buttons("shortcuts"));
        col = col.push(self.panorama_icons_status());
        for (i, label, v) in [
            (0, "Nom du raccourci", &draft.0),
            (1, "Nom de l'icône", &draft.1),
            (2, "Commande à lancer", &draft.2),
        ] {
            col = col.push(
                column![
                    text(label).size(crate::ui::tokens::ts(12.0)),
                    text_input("", v)
                        .on_input(move |v| wrap(Msg::ShortcutText(i, v)))
                        .on_submit(wrap(Msg::ShortcutAdd))
                        .size(crate::ui::tokens::ts(13.0))
                ]
                .spacing(5),
            );
        }
        col.push(
            button(text("Ajouter le raccourci").size(crate::ui::tokens::ts(13.0))).on_press_maybe(
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
        let board = self.panorama.edit_board;
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
            Msg::DesignBrowse(key) => {
                // Départ : le dossier de l'image actuelle, sinon Images.
                let cur = c.design.text(&key).unwrap_or("").to_string();
                let home = std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
                let start = crate::ui::plugins::wallpaper::expand(&cur)
                    .parent()
                    .filter(|d| !cur.is_empty() && d.is_dir())
                    .map(std::path::Path::to_path_buf)
                    .or_else(|| Some(home.join("Images")).filter(|d| d.is_dir()))
                    .or(Some(home));
                let title = crate::i18n::tr("Image de fond du Panorama").to_string();
                return Task::perform(crate::portal::pick_image(title, start), move |p| {
                    wrap(Msg::DesignBrowsed(key.clone(), p))
                });
            }
            Msg::DesignBrowsed(_, None) => return Task::none(),
            Msg::DesignBrowsed(key, Some(path)) => {
                let v = path.to_string_lossy().to_string();
                if let Err(e) = c.design.set(&key, &v) {
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
                if let Some(t) = board_tiles_mut(&mut c, board).get_mut(self.panorama.selected_tile)
                {
                    t.widget = w;
                }
            }
            Msg::TileNumber(key, v) => {
                if let Some(t) = board_tiles_mut(&mut c, board).get_mut(self.panorama.selected_tile)
                {
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
                let tiles = board_tiles_mut(&mut c, board);
                if tiles.len() < 16 {
                    tiles.push(Tile::new("clock", 0.0, 0.0, 0.3, 0.5));
                    self.panorama.selected_tile = tiles.len() - 1;
                }
            }
            Msg::TileRemove => {
                let tiles = board_tiles_mut(&mut c, board);
                if tiles.len() > 1 {
                    let i = self.panorama.selected_tile.min(tiles.len() - 1);
                    tiles.remove(i);
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
            Msg::ShortcutMove(i, d) => {
                let j = i as i64 + d as i64;
                let l = &mut c.design.shortcuts;
                if i < l.len() && j >= 0 && (j as usize) < l.len() {
                    l.swap(i, j as usize);
                }
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

/// Zones de la composition réglée : la vue par défaut (`None`) ou une page.
fn board_tiles(c: &crate::panorama::Config, board: Option<usize>) -> &Vec<Tile> {
    match board.and_then(|i| c.pages.get(i)) {
        Some(p) => &p.tiles,
        None => &c.design.tiles,
    }
}

fn board_tiles_mut(c: &mut crate::panorama::Config, board: Option<usize>) -> &mut Vec<Tile> {
    match board {
        Some(i) if i < c.pages.len() => &mut c.pages[i].tiles,
        _ => &mut c.design.tiles,
    }
}
