//! Affichage des widgets sur le bureau : un rendu par type (intégrés), le
//! texte rendu pour les commandes et plugins Lua, et le placement (ancrage
//! + décalage). En mode réglages, un widget se sélectionne et se déplace à
//!   la souris.

use crate::i18n::text;
use crate::ui::tokens::Shown as _;
use chrono::{Datelike, Local};
use iced::widget::{Space, button, column, container, mouse_area, progress_bar, row};
use iced::{Alignment, Background, Border, Color, Element, Length, mouse};

use super::{App, Msg, chrono_locale, color, format_now};
use crate::config::{Anchor, WidgetDef, WidgetFamily, WidgetKind};
use crate::widgets;

/// Une jauge est en alerte dès le seuil réglé (`monitoring.threshold`).
fn gauge_alert(ratio: f32, threshold: f64) -> bool {
    f64::from(ratio) >= threshold
}

impl App {
    /// Un widget du bureau : sa boîte, et où la poser (`desk_layout` la place
    /// sans recouvrir les autres).
    /// Le widget a un contenu lié qu'un clic sur son fond ouvre : horloge,
    /// calendrier, système, média en cours (HIG SYS-10).
    pub(super) fn widget_link(&self, w: &WidgetDef) -> bool {
        match &w.kind {
            WidgetKind::Clock { .. } | WidgetKind::Calendar | WidgetKind::System { .. } => true,
            WidgetKind::Media { .. } => {
                matches!(self.values.get(&w.id), Some(widgets::Value::Media(Some(_))))
            }
            _ => false,
        }
    }

    pub(super) fn view_widget(
        &self,
        win: iced::window::Id,
        w: &WidgetDef,
        t: f32,
    ) -> (super::desk_layout::Spot, Element<'_, Msg>) {
        let g = &self.cfg.general;
        let th = &g.theme;
        let fg = color(th.fg, t);
        let dim = color(th.fg, 0.65 * t);
        let accent = color(th.accent, t);
        let font = self.font;
        let base = g.font_size;
        let content: Element<'_, Msg> = match &w.kind {
            WidgetKind::Launcher => self
                .monitor(win)
                .map_or_else(|| Space::new().into(), |m| self.view_launcher_bar(m, t)),
            WidgetKind::Controls => container(self.controls_view(
                t,
                self.monitor(win).map_or("", |m| m.name.as_str()),
                None,
            ))
            // Largeur réglée des réglages rapides (MLG-279).
            .width(crate::design_options::num(g, "controls.width") as f32)
            .into(),
            WidgetKind::Clock { format } => {
                let size = if w.size > 0.0 { w.size } else { 56.0 };
                let layers = clock_layers(
                    w.family,
                    format,
                    crate::design_options::flag(g, "clock.date"),
                );
                let time = text(format_now(format))
                    .font(self.numeric_font(iced::font::Weight::Light))
                    .size(crate::ui::tokens::ts_metric(size))
                    .color(fg);
                let small = (size * 0.22).clamp(13.0, 20.0);
                let mut c = column![time].spacing(2).align_x(Alignment::Center);
                // Date sous l'heure, au choix (MLG-279).
                if layers.date {
                    c = c.push(
                        text(format_now("%A %-d %B"))
                            .font(font)
                            .size(small)
                            .color(dim),
                    );
                }
                // Grand : le jour de l'année et la semaine (HIG SYS-15).
                if layers.day_week {
                    c = c.push(
                        text(crate::tr!(
                            "Jour {jour} · semaine {semaine}",
                            jour = format_now("%-j"),
                            semaine = format_now("%-V")
                        ))
                        .font(font)
                        .size(small)
                        .color(dim),
                    );
                }
                // Très grand : la grille du mois, celle du calendrier.
                if layers.month {
                    c = c
                        .push(Space::new().height(8))
                        .push(self.view_calendar(base, fg, dim, accent, None, None));
                }
                c.into()
            }
            WidgetKind::Calendar => {
                let size = if w.size > 0.0 { w.size } else { base };
                match calendar_months(w.family) {
                    // Petit : le jour et sa date, sans la grille (HIG SYS-15).
                    0 => column![
                        text(format_now("%A")).font(font).size(size).color(dim),
                        text(format_now("%-d"))
                            .font(self.numeric_font(iced::font::Weight::Light))
                            .size(crate::ui::tokens::ts_metric(size * 3.0))
                            .color(fg),
                        text(format_now("%B")).font(font).size(size).color(dim),
                    ]
                    .spacing(2)
                    .align_x(Alignment::Center)
                    .into(),
                    1 => self.view_calendar(size, fg, dim, accent, None, None),
                    // Grand, très grand : les mois suivants à côté, sans
                    // agrandir le premier.
                    n => row((0..n as i32).map(|m| {
                        if m == 0 {
                            self.view_calendar(size, fg, dim, accent, None, None)
                        } else {
                            let first = month_start(Local::now().date_naive(), m);
                            column![
                                self.month_title(first, size, fg),
                                self.view_calendar(size, fg, dim, accent, None, Some(m)),
                            ]
                            .spacing(6)
                            .into()
                        }
                    }))
                    .spacing(24)
                    .into(),
                }
            }
            WidgetKind::System { .. } => {
                let size = if w.size > 0.0 { w.size } else { base };
                match self.values.get(&w.id) {
                    Some(widgets::Value::Sys(lines)) => column(lines.iter().map(|l| {
                        let values = row![
                            text(l.label.clone())
                                .font(self.font)
                                .size(crate::ui::tokens::ts(size))
                                .color(dim)
                                .font(font)
                                .width(Length::Fixed(size * 5.0)),
                            text(l.value.clone())
                                .size(crate::ui::tokens::ts_metric(size))
                                .color(fg)
                                .font(self.numeric_font(font.weight)),
                        ]
                        .spacing(size * 0.3);
                        let mut c = column![values];
                        // Monitoring (MLG-279) : jauges au choix, en alerte
                        // au-delà du seuil réglé.
                        let mut alert = false;
                        if let Some(r) = l
                            .ratio
                            .filter(|_| crate::design_options::flag(g, "monitoring.bars"))
                        {
                            alert = gauge_alert(
                                r,
                                crate::design_options::num(g, "monitoring.threshold"),
                            );
                            let bar = if alert {
                                Color {
                                    a: t,
                                    ..crate::ui::tone::warning()
                                }
                            } else {
                                accent
                            };
                            let track = color(th.fg, 0.15 * t);
                            c = c.push(
                                progress_bar(0.0..=1.0, r)
                                    .girth(4)
                                    .length(Length::Fixed(size * 16.0))
                                    .style(move |_| progress_bar::Style {
                                        background: track.into(),
                                        bar: bar.into(),
                                        border: Border::default().rounded(2),
                                    }),
                            );
                        }
                        // Ligne en alerte : un signe qui ne tient pas à la seule
                        // couleur de la jauge, dans l'encre du texte (HIG SYS-16).
                        // Sous la jauge : la largeur du widget ne bouge pas.
                        if alert
                            && let Some(sign) = crate::ui::icons::glyph(
                                "dialog-warning",
                                crate::ui::tokens::ts(size) * 0.9,
                                fg,
                            )
                        {
                            c = c.push(sign);
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
                        // Glyphe de hyprdesk teinté comme les autres commandes ;
                        // à défaut, un repli en ASCII (les glyphes ⏮ ⏯ ⏭ manquent
                        // à la plupart des polices).
                        let ctl = |icon: usize, fallback: &'static str, method: &'static str| {
                            let label: Element<'_, Msg> =
                                self.ui_icon(icon, size * 1.3, t, fg).unwrap_or_else(|| {
                                    text(fallback)
                                        .font(self.font)
                                        .size(crate::ui::tokens::ts(size))
                                        .color(fg)
                                        .font(font)
                                        .into()
                                });
                            button(label)
                                .on_press(Msg::Media(mi.bus.clone(), method))
                                .padding([4, 10])
                                .style(move |_, status| button::Style {
                                    background: matches!(status, button::Status::Hovered)
                                        .then(|| Background::Color(Color { a: 0.15 * t, ..fg })),
                                    text_color: fg,
                                    border: Border::default().rounded(8),
                                    ..Default::default()
                                })
                        };
                        // Pochette (MLG-279) : taille réglée ; la carte peut
                        // en prendre la teinte (voir plus bas).
                        let cover: Element<'_, Msg> = match &mi.art {
                            Some(p) => {
                                let s = crate::design_options::num(g, "media.cover_size") as f32;
                                iced::widget::image(iced::widget::image::Handle::from_path(p))
                                    .width(s)
                                    .height(s)
                                    .content_fit(iced::ContentFit::Cover)
                                    .border_radius(8.0)
                                    .into()
                            }
                            None => Space::new().width(0).into(),
                        };
                        let layers = media_layers(w.family);
                        let play = if mi.playing {
                            ctl(1, "||", "PlayPause")
                        } else {
                            ctl(2, ">", "PlayPause")
                        };
                        // Petit : la pochette et la lecture seules (HIG SYS-15).
                        let controls = if layers.info {
                            row![ctl(0, "|<", "Previous"), play, ctl(3, ">|", "Next")]
                        } else {
                            row![play]
                        };
                        let mut info = column![].spacing(4).max_width(size * 26.0);
                        if layers.info {
                            info = info
                                .push(
                                    text(if mi.title.is_empty() {
                                        "—".to_string()
                                    } else {
                                        mi.title.clone()
                                    })
                                    .font(self.font)
                                    .size(size * 1.15)
                                    .color(fg)
                                    .font(self.title_font()),
                                )
                                .push(
                                    text(mi.artist.clone())
                                        .font(self.font)
                                        .size(crate::ui::tokens::ts(size))
                                        .color(dim)
                                        .font(font),
                                );
                        }
                        // Très grand : l'album et le lecteur.
                        if layers.details {
                            let line = [mi.album.clone(), widgets::media::player_name(&mi.bus)]
                                .into_iter()
                                .filter(|s| !s.is_empty())
                                .collect::<Vec<_>>()
                                .join(" · ");
                            info = info.push(
                                text(line)
                                    .size(crate::ui::tokens::ts(size))
                                    .color(dim)
                                    .font(font),
                            );
                        }
                        info = info.push(controls.spacing(6));
                        // Grand : la progression, quand le lecteur la donne.
                        if layers.progress
                            && let Some((r, p, l)) = media_progress(mi.position, mi.length)
                        {
                            let track = color(th.fg, 0.15 * t);
                            let small = crate::ui::tokens::ts(size) * 0.85;
                            info = info.push(
                                column![
                                    progress_bar(0.0..=1.0, r)
                                        .girth(4)
                                        .length(Length::Fixed(size * 16.0))
                                        .style(move |_| progress_bar::Style {
                                            background: track.into(),
                                            bar: accent.into(),
                                            border: Border::default().rounded(2),
                                        }),
                                    row![
                                        text(super::plugins::panorama::surface::fmt_time(p))
                                            .size(small)
                                            .color(dim)
                                            .font(font),
                                        Space::new().width(Length::Fill),
                                        text(super::plugins::panorama::surface::fmt_time(l))
                                            .size(small)
                                            .color(dim)
                                            .font(font),
                                    ]
                                    .width(Length::Fixed(size * 16.0)),
                                ]
                                .spacing(4),
                            );
                        }
                        row![cover, info]
                            .spacing(12)
                            .align_y(iced::Alignment::Center)
                            .into()
                    }
                    Some(widgets::Value::Media(None)) => text("Aucun lecteur")
                        .font(self.font)
                        .size(crate::ui::tokens::ts(size))
                        .color(dim)
                        .font(font)
                        .into(),
                    other => self.view_value(other, size, fg, dim),
                }
            }
            WidgetKind::Command { .. } | WidgetKind::Lua { .. } => {
                let size = if w.size > 0.0 { w.size } else { base };
                self.view_value(self.values.get(&w.id), size, fg, dim)
            }
        };
        // Transparence adaptative (MLG-261) : la carte se densifie sur un
        // fond chargé, d'après la zone d'ancrage du widget.
        let card_opacity = {
            let base = th.shown(crate::config::Place::Widgets).opacity;
            match self.monitor(win) {
                Some(m) if th.shown(crate::config::Place::Widgets).adaptive.is_some() => {
                    let roi = widget_roi(self.monitor_size(&m.name), w.anchor, (w.x, w.y));
                    self.adaptive_opacity(crate::config::Place::Widgets, base, &m.name, roi)
                }
                _ => base,
            }
        };
        // Média (MLG-279) : la carte peut prendre la teinte de la pochette.
        let cover_tint = match (&w.kind, self.values.get(&w.id)) {
            (WidgetKind::Media { .. }, Some(widgets::Value::Media(Some(mi))))
                if crate::design_options::flag(g, "media.cover_tint") =>
            {
                mi.tint
            }
            _ => None,
        };
        let base_bg = super::tokens::tinted(th.bg, crate::config::Place::Widgets);
        let base_bg = match cover_tint {
            Some(c) => {
                let m = crate::theme_engine::color::mix(base_bg, c, 0.35);
                crate::config::Rgba(m.0, m.1, m.2, base_bg.3)
            }
            None => base_bg,
        };
        let card_bg = color(base_bg, card_opacity * t);
        let boxed: Element<'_, Msg> = if w.card {
            // Verre du thème (MLG-268) au rayon de la carte ; vraie
            // réfraction du fond d'écran et des icônes derrière elle
            // (MLG-275), la carte étant dessinée sur le bureau.
            super::glass::card(
                container(content).padding(16),
                crate::config::Place::Widgets,
                super::design::RADIUS,
                color(th.fg, t),
                card_bg,
                container::Style {
                    background: Some(card_bg.into()),
                    ..super::design::surface(&self.theme, super::design::Surface::Floating)
                },
                true,
                None,
            )
        } else {
            content
        };
        // Réglages ouverts : chaque widget se sélectionne et se glisse.
        let boxed: Element<'_, Msg> = match &self.panel {
            Some(p) => {
                let selected = p.widgets.widget_sel.as_deref() == Some(w.id.as_str());
                let accent = color(th.accent, t);
                let edge = Color {
                    a: 0.35 * t,
                    ..color(th.fg, 1.0)
                };
                mouse_area(container(boxed).padding(2).style(move |_| {
                    container::Style {
                        border: Border::default()
                            .rounded(16)
                            .width(if selected { 2.5 } else { 1.0 })
                            .color(if selected { accent } else { edge }),
                        ..Default::default()
                    }
                }))
                .on_press(Msg::WidgetPress(win, w.id.clone()))
                .on_right_press(Msg::WidgetContext(win, w.id.clone()))
                .interaction(mouse::Interaction::Grab)
                .into()
            }
            // Réglages fermés : le fond ouvre le contenu lié ; les boutons du
            // widget gardent leur clic (HIG SYS-10).
            None if self.widget_link(w) => mouse_area(boxed)
                .on_press(Msg::WidgetOpen(w.id.clone()))
                .interaction(mouse::Interaction::Pointer)
                .into(),
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
        let axis = |a: Alignment, off: f32, edge_margin: f32| -> (f32, f32) {
            match a {
                Alignment::Start => (edge_margin + off, 0.0),
                Alignment::End => (0.0, (edge_margin - off).max(0.0)),
                Alignment::Center => ((2.0 * off).max(0.0), (-2.0 * off).max(0.0)),
            }
        };
        let (left, right) = axis(ax, w.x, margin);
        let (top, bottom) = axis(ay, w.y, if ay == Alignment::End { 120.0 } else { margin });
        (
            super::desk_layout::Spot {
                x: ax,
                y: ay,
                pad: [top, right, bottom, left],
            },
            boxed,
        )
    }

    fn view_value(
        &self,
        v: Option<&widgets::Value>,
        size: f32,
        fg: Color,
        dim: Color,
    ) -> Element<'_, Msg> {
        match v {
            Some(widgets::Value::Text(s)) => text(s.clone())
                .font(self.font)
                .size(crate::ui::tokens::ts(size))
                .color(fg)
                .font(self.font)
                .into(),
            Some(widgets::Value::Rich(r)) if r.state != crate::config::RenderState::Hidden => {
                let mut c = row![].spacing(8).align_y(Alignment::Center);
                if let Some(icon) = r
                    .icon
                    .as_deref()
                    .and_then(|i| self.icon_named(i, size * 1.3, fg))
                {
                    c = c.push(icon);
                }
                c.push(
                    text(r.text.clone())
                        .font(self.font)
                        .size(crate::ui::tokens::ts(size))
                        .color(fg)
                        .font(self.font),
                )
                .into()
            }
            Some(widgets::Value::Rich(_)) => Space::new().into(),
            Some(widgets::Value::Error(e)) => text(crate::tr!("⚠ {e}", e = e))
                .font(self.font)
                .size(crate::ui::tokens::ts(size))
                .color(Color {
                    r: 1.0,
                    g: 0.5,
                    b: 0.5,
                    a: dim.a,
                })
                .font(self.font)
                .into(),
            _ => text("…")
                .font(self.font)
                .size(crate::ui::tokens::ts(size))
                .color(dim)
                .font(self.font)
                .into(),
        }
    }

    pub(super) fn view_calendar(
        &self,
        size: f32,
        fg: Color,
        dim: Color,
        accent: Color,
        width: Option<f32>,
        // Mois décalé de n (le titre est alors affiché par l'appelant), ou
        // le mois en cours avec son titre (widget du bureau).
        nav: Option<i32>,
    ) -> Element<'_, Msg> {
        let (months, with_title) = (nav.unwrap_or(0), nav.is_none());
        let today = Local::now().date_naive();
        // Mois affiché : celui d'aujourd'hui, décalé de `months`.
        let first = month_start(today, months);
        let days = month_days(first);
        let this_month = months == 0;
        let offset = first.weekday().num_days_from_monday();
        let w = width.map(|width| width / 7.0).unwrap_or(size * 2.2);
        let mut grid = column![].spacing(6);
        if with_title {
            grid = grid.push(self.month_title(first, size, fg));
        }
        // Initiales des jours, lundi d'abord, dans la langue de l'utilisateur.
        let monday = first - chrono::Duration::days(offset as i64);
        let head = row((0..7).map(|i| {
            let d = monday + chrono::Duration::days(i);
            let s: String = format!("{}", d.format_localized("%a", chrono_locale()))
                .chars()
                .take(2)
                .collect();
            text(s)
                .font(self.font)
                .size(size * 0.85)
                .color(dim)
                .font(self.font)
                .width(w)
                .align_x(Alignment::Center)
                .into()
        }));
        grid = grid.push(head);
        let mut day = 1u32;
        let cells = month_weeks(first);
        for week in 0..cells {
            let r = row((0..7u32).map(|wd| {
                let slot = week * 7 + wd;
                if slot < offset || day > days {
                    return Space::new().width(w).into();
                }
                let d = day;
                day += 1;
                let is_today = this_month && d == today.day();
                let label = text(d.to_string())
                    .font(self.font)
                    .size(crate::ui::tokens::ts(size))
                    .font(self.font)
                    .color(if is_today {
                        super::design::on_accent(
                            accent,
                            fg,
                            color(self.cfg.general.theme.bg, accent.a),
                        )
                    } else {
                        fg
                    });
                container(label)
                    .width(w)
                    .align_x(Alignment::Center)
                    .style(move |_| container::Style {
                        background: is_today.then_some(Background::Color(accent)),
                        border: Border::default().rounded(6),
                        ..Default::default()
                    })
                    .into()
            }));
            grid = grid.push(r);
        }
        grid.into()
    }

    /// Titre d'un mois du calendrier (« novembre 2026 »).
    fn month_title(&self, first: chrono::NaiveDate, size: f32, fg: Color) -> Element<'_, Msg> {
        text(format!(
            "{}",
            first.format_localized("%B %Y", chrono_locale())
        ))
        .size(size * 1.1)
        .color(fg)
        .font(self.title_font())
        .into()
    }
}

/// Ce que montre l'horloge selon sa taille (HIG SYS-15) : `Medium` est
/// l'affichage historique, chaque taille au-dessus ajoute une couche.
#[derive(Debug, PartialEq, Eq)]
struct ClockLayers {
    /// La date sous l'heure (format `%H:%M` et option `clock.date`).
    date: bool,
    /// Le jour de l'année et la semaine.
    day_week: bool,
    /// La grille du mois.
    month: bool,
}

fn clock_layers(family: WidgetFamily, format: &str, date_option: bool) -> ClockLayers {
    use WidgetFamily as F;
    ClockLayers {
        date: family != F::Small && format == "%H:%M" && date_option,
        day_week: matches!(family, F::Large | F::XLarge),
        month: family == F::XLarge,
    }
}

/// Ce que montre le widget média selon sa taille (HIG SYS-15).
#[derive(Debug, PartialEq, Eq)]
struct MediaLayers {
    /// Le titre, l'artiste et les commandes précédent et suivant (dès moyen).
    info: bool,
    /// La progression, position et durée (dès grand).
    progress: bool,
    /// L'album et le lecteur (très grand).
    details: bool,
}

fn media_layers(family: WidgetFamily) -> MediaLayers {
    use WidgetFamily as F;
    MediaLayers {
        info: family != F::Small,
        progress: matches!(family, F::Large | F::XLarge),
        details: family == F::XLarge,
    }
}

/// Progression d'une lecture : la part jouée, la position et la durée, si
/// elles sont plausibles (un direct annonce parfois une durée absurde).
fn media_progress(position: Option<i64>, length: Option<i64>) -> Option<(f32, i64, i64)> {
    let (p, l) = (position?, length?);
    (l > 0 && l < 48 * 3600 && (0..=l).contains(&p)).then(|| (p as f32 / l as f32, p, l))
}

/// Mois que montre le calendrier selon sa taille (HIG SYS-15) : aucun en
/// petit (le jour seul), puis un, deux ou trois à partir du mois en cours.
fn calendar_months(family: WidgetFamily) -> u32 {
    match family {
        WidgetFamily::Small => 0,
        WidgetFamily::Medium => 1,
        WidgetFamily::Large => 2,
        WidgetFamily::XLarge => 3,
    }
}

/// Premier jour du mois d'aujourd'hui décalé de `months` (négatif : avant).
pub(super) fn month_start(today: chrono::NaiveDate, months: i32) -> chrono::NaiveDate {
    let m0 = today.year() * 12 + today.month0() as i32 + months;
    chrono::NaiveDate::from_ymd_opt(m0.div_euclid(12), m0.rem_euclid(12) as u32 + 1, 1)
        .unwrap_or(today)
}

/// Jours du mois commençant à `first`.
pub(super) fn month_days(first: chrono::NaiveDate) -> u32 {
    let next = month_start(first, 1);
    ((next - first).num_days() as u32).clamp(28, 31)
}

/// Semaines (lignes) du calendrier de ce mois, lundi d'abord.
pub(super) fn month_weeks(first: chrono::NaiveDate) -> u32 {
    (first.weekday().num_days_from_monday() + month_days(first)).div_ceil(7)
}

/// Zone approximative d'un widget sur son écran, d'après son ancrage et son
/// décalage (sa taille n'est connue qu'après la mise en page) : une carte
/// de 360×220 au point d'ancrage, de quoi lire le fond qu'il couvre.
fn widget_roi(screen: iced::Size, anchor: Anchor, (dx, dy): (f32, f32)) -> iced::Rectangle {
    let (w, h, margin) = (360.0_f32, 220.0_f32, 32.0_f32);
    let x = match anchor {
        Anchor::TopLeft | Anchor::Left | Anchor::BottomLeft => margin + dx,
        Anchor::Top | Anchor::Center | Anchor::Bottom => (screen.width - w) / 2.0 + dx,
        Anchor::TopRight | Anchor::Right | Anchor::BottomRight => screen.width - w - margin + dx,
    };
    let y = match anchor {
        Anchor::TopLeft | Anchor::Top | Anchor::TopRight => margin + dy,
        Anchor::Left | Anchor::Center | Anchor::Right => (screen.height - h) / 2.0 + dy,
        Anchor::BottomLeft | Anchor::Bottom | Anchor::BottomRight => screen.height - h - 120.0 + dy,
    };
    iced::Rectangle::new(
        iced::Point::new(
            x.clamp(0.0, (screen.width - w).max(0.0)),
            y.clamp(0.0, (screen.height - h).max(0.0)),
        ),
        iced::Size::new(w.min(screen.width), h.min(screen.height)),
    )
}

#[cfg(test)]
mod calendar_tests {
    use super::*;

    #[test]
    fn mois_decales() {
        let d = |y, m, j| chrono::NaiveDate::from_ymd_opt(y, m, j).unwrap();
        assert_eq!(month_start(d(2026, 11, 11), 0), d(2026, 11, 1));
        assert_eq!(month_start(d(2026, 11, 11), 2), d(2027, 1, 1));
        assert_eq!(month_start(d(2026, 1, 31), -1), d(2025, 12, 1));
        assert_eq!(month_start(d(2026, 3, 15), -26), d(2024, 1, 1));
        assert_eq!(month_days(d(2028, 2, 1)), 29);
        assert_eq!(month_days(d(2026, 2, 1)), 28);
        // Novembre 2026 commence un dimanche : 6 lignes.
        assert_eq!(month_weeks(d(2026, 11, 1)), 6);
        assert_eq!(month_weeks(d(2027, 2, 1)), 4);
    }

    /// Taille moyenne : l'affichage d'avant les tailles (HIG SYS-15) ; les
    /// autres ajoutent ou retirent une couche sans toucher à l'heure.
    #[test]
    fn couches_par_taille() {
        use WidgetFamily as F;
        let l = |date, day_week, month| ClockLayers {
            date,
            day_week,
            month,
        };
        assert_eq!(
            clock_layers(F::Medium, "%H:%M", true),
            l(true, false, false)
        );
        assert_eq!(
            clock_layers(F::Medium, "%H:%M", false),
            l(false, false, false)
        );
        assert_eq!(
            clock_layers(F::Medium, "%H:%M:%S", true),
            l(false, false, false)
        );
        assert_eq!(
            clock_layers(F::Small, "%H:%M", true),
            l(false, false, false)
        );
        assert_eq!(clock_layers(F::Large, "%H:%M", true), l(true, true, false));
        assert_eq!(
            clock_layers(F::XLarge, "%H:%M", false),
            l(false, true, true)
        );
        assert_eq!(calendar_months(F::Medium), 1);
        assert_eq!(calendar_months(F::Small), 0);
        assert_eq!(calendar_months(F::Large), 2);
        let m = |info, progress, details| MediaLayers {
            info,
            progress,
            details,
        };
        assert_eq!(media_layers(F::Medium), m(true, false, false));
        assert_eq!(media_layers(F::Small), m(false, false, false));
        assert_eq!(media_layers(F::XLarge), m(true, true, true));
        assert_eq!(media_progress(Some(30), Some(120)), Some((0.25, 30, 120)));
        assert_eq!(media_progress(Some(30), None), None);
        assert_eq!(media_progress(Some(200), Some(120)), None);
        assert_eq!(media_progress(Some(0), Some(0)), None);
        assert_eq!(calendar_months(F::XLarge), 3);
    }
}
