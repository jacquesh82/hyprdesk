//! Affichage des widgets sur le bureau : un rendu par type (intégrés), le
//! texte rendu pour les commandes et plugins Lua, et le placement (ancrage
//! + décalage). En mode réglages, un widget se sélectionne et se déplace à
//! la souris.

use chrono::{Datelike, Local};
use iced::widget::{Space, button, column, container, mouse_area, progress_bar, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, mouse};

use super::{App, IconImg, Msg, chrono_locale, color, format_now};
use crate::config::{Anchor, WidgetDef, WidgetKind};
use crate::widgets;

impl App {
    pub(super) fn view_widget(&self, win: iced::window::Id, w: &WidgetDef, t: f32) -> Element<'_, Msg> {
        let g = &self.cfg.general;
        let th = &g.theme;
        let fg = color(th.fg, t);
        let dim = color(th.fg, 0.65 * t);
        let accent = color(th.accent, t);
        let font = self.font;
        let base = g.font_size;
        let content: Element<'_, Msg> = match &w.kind {
            WidgetKind::Clock { format } => {
                let size = if w.size > 0.0 { w.size } else { 56.0 };
                text(format_now(format)).size(size).color(fg).font(font).into()
            }
            WidgetKind::Calendar => self.view_calendar(if w.size > 0.0 { w.size } else { base }, fg, dim, accent),
            WidgetKind::System { .. } => {
                let size = if w.size > 0.0 { w.size } else { base };
                match self.values.get(&w.id) {
                    Some(widgets::Value::Sys(lines)) => column(lines.iter().map(|l| {
                        let mut c = column![row![
                            text(l.label.clone()).size(size).color(dim).font(font).width(Length::Fixed(size * 5.0)),
                            text(l.value.clone()).size(size).color(fg).font(font),
                        ]];
                        if let Some(r) = l.ratio {
                            let (track, bar) = (color(th.fg, 0.15 * t), accent);
                            c = c.push(progress_bar(0.0..=1.0, r).girth(4).length(Length::Fixed(size * 16.0)).style(move |_| {
                                progress_bar::Style { background: track.into(), bar: bar.into(), border: Border::default().rounded(2) }
                            }));
                        }
                        c.spacing(4).into()
                    }))
                    .spacing(10)
                    .into(),
                    other => self.view_value(other, size, fg, dim),
                }
            }
            WidgetKind::Media { .. } => {
                let size = if w.size > 0.0 { w.size } else { base };
                match self.values.get(&w.id) {
                    Some(widgets::Value::Media(Some(mi))) => {
                        // Icône du thème si elle existe, sinon un repli en ASCII
                        // (les glyphes ⏮ ⏯ ⏭ manquent à la plupart des polices).
                        let ctl = |icon: usize, fallback: &'static str, method: &'static str| {
                            let img = self.ui_imgs.get(icon).copied().unwrap_or(IconImg::None);
                            let label: Element<'_, Msg> = self
                                .img_element(img, size * 1.3, t)
                                .unwrap_or_else(|| text(fallback).size(size).color(fg).font(font).into());
                            button(label)
                                .on_press(Msg::Media(mi.bus.clone(), method))
                                .padding([4, 10])
                                .style(move |_, status| button::Style {
                                    background: matches!(status, button::Status::Hovered).then(|| Background::Color(Color { a: 0.15 * t, ..fg })),
                                    text_color: fg,
                                    border: Border::default().rounded(8),
                                    ..Default::default()
                                })
                        };
                        column![
                            text(if mi.title.is_empty() { "—".to_string() } else { mi.title.clone() }).size(size * 1.15).color(fg).font(font),
                            text(mi.artist.clone()).size(size).color(dim).font(font),
                            row![
                                ctl(0, "|<", "Previous"),
                                if mi.playing { ctl(1, "||", "PlayPause") } else { ctl(2, ">", "PlayPause") },
                                ctl(3, ">|", "Next")
                            ]
                            .spacing(6),
                        ]
                        .spacing(4)
                        .max_width(size * 26.0)
                        .into()
                    }
                    Some(widgets::Value::Media(None)) => text("Aucun lecteur").size(size).color(dim).font(font).into(),
                    other => self.view_value(other, size, fg, dim),
                }
            }
            WidgetKind::Command { .. } | WidgetKind::Lua { .. } => {
                let size = if w.size > 0.0 { w.size } else { base };
                self.view_value(self.values.get(&w.id), size, fg, dim)
            }
        };
        let card_bg = color(th.bg, 0.72 * t);
        let boxed: Element<'_, Msg> = if w.card {
            container(content)
                .padding(16)
                .style(move |_| container::Style { background: Some(card_bg.into()), border: Border::default().rounded(14), ..Default::default() })
                .into()
        } else {
            content
        };
        // Réglages ouverts : chaque widget se sélectionne et se glisse.
        let boxed: Element<'_, Msg> = match &self.panel {
            Some(p) => {
                let selected = p.widget_sel.as_deref() == Some(w.id.as_str());
                let accent = color(th.accent, t);
                let edge = Color { a: 0.35 * t, ..color(th.fg, 1.0) };
                mouse_area(container(boxed).padding(2).style(move |_| container::Style {
                    border: Border::default().rounded(16).width(if selected { 2.5 } else { 1.0 }).color(if selected { accent } else { edge }),
                    ..Default::default()
                }))
                .on_press(Msg::WidgetPress(win, w.id.clone()))
                .interaction(mouse::Interaction::Grab)
                .into()
            }
            None => boxed,
        };
        let margin = 32.0;
        let (ax, ay) = match w.anchor {
            Anchor::TopLeft => (Alignment::Start, Alignment::Start),
            Anchor::Top => (Alignment::Center, Alignment::Start),
            Anchor::TopRight => (Alignment::End, Alignment::Start),
            Anchor::Left => (Alignment::Start, Alignment::Center),
            Anchor::Center => (Alignment::Center, Alignment::Center),
            Anchor::Right => (Alignment::End, Alignment::Center),
            Anchor::BottomLeft => (Alignment::Start, Alignment::End),
            Anchor::Bottom => (Alignment::Center, Alignment::End),
            Anchor::BottomRight => (Alignment::End, Alignment::End),
        };
        // x > 0 décale vers la droite, y > 0 vers le bas, quel que soit l'ancrage.
        let axis = |a: Alignment, off: f32| -> (f32, f32) {
            match a {
                Alignment::Start => (margin + off, 0.0),
                Alignment::End => (0.0, (margin - off).max(0.0)),
                Alignment::Center => ((2.0 * off).max(0.0), (-2.0 * off).max(0.0)),
            }
        };
        let (left, right) = axis(ax, w.x);
        let (top, bottom) = axis(ay, w.y);
        container(boxed)
            .width(Length::Fill)
            .height(Length::Fill)
            .align_x(ax)
            .align_y(ay)
            .padding(Padding { top, right, bottom, left })
            .into()
    }

    fn view_value(&self, v: Option<&widgets::Value>, size: f32, fg: Color, dim: Color) -> Element<'_, Msg> {
        match v {
            Some(widgets::Value::Text(s)) => text(s.clone()).size(size).color(fg).font(self.font).into(),
            Some(widgets::Value::Error(e)) => text(format!("⚠ {e}")).size(size).color(Color { r: 1.0, g: 0.5, b: 0.5, a: dim.a }).font(self.font).into(),
            _ => text("…").size(size).color(dim).font(self.font).into(),
        }
    }

    fn view_calendar(&self, size: f32, fg: Color, dim: Color, accent: Color) -> Element<'_, Msg> {
        let today = Local::now().date_naive();
        let first = today.with_day(1).unwrap_or(today);
        let next_month = if first.month() == 12 {
            chrono::NaiveDate::from_ymd_opt(first.year() + 1, 1, 1)
        } else {
            chrono::NaiveDate::from_ymd_opt(first.year(), first.month() + 1, 1)
        };
        let days = next_month.map(|n| (n - first).num_days() as u32).unwrap_or(30);
        let offset = first.weekday().num_days_from_monday();
        let w = size * 2.2;
        let title = format!("{}", first.format_localized("%B %Y", chrono_locale()));
        let mut grid = column![text(title).size(size * 1.1).color(fg).font(self.font)].spacing(6);
        // Initiales des jours, lundi d'abord, dans la langue de l'utilisateur.
        let monday = first - chrono::Duration::days(offset as i64);
        let head = row((0..7).map(|i| {
            let d = monday + chrono::Duration::days(i);
            let s: String = format!("{}", d.format_localized("%a", chrono_locale())).chars().take(2).collect();
            text(s).size(size * 0.85).color(dim).font(self.font).width(w).align_x(Alignment::Center).into()
        }));
        grid = grid.push(head);
        let mut day = 1u32;
        let cells = (offset + days).div_ceil(7);
        for week in 0..cells {
            let r = row((0..7u32).map(|wd| {
                let slot = week * 7 + wd;
                if slot < offset || day > days {
                    return Space::new().width(w).into();
                }
                let d = day;
                day += 1;
                let is_today = d == today.day();
                let label = text(d.to_string()).size(size).font(self.font).color(if is_today { Color::WHITE } else { fg });
                container(label)
                    .width(w)
                    .align_x(Alignment::Center)
                    .style(move |_| container::Style {
                        background: is_today.then(|| Background::Color(accent)),
                        border: Border::default().rounded(6),
                        ..Default::default()
                    })
                    .into()
            }));
            grid = grid.push(r);
        }
        grid.into()
    }
}
