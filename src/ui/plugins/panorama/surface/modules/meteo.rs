//! Tuile Météo : le temps qu'il fait (glyphe, température, lieu), la ligne
//! ressentie · vent · humidité, les heures à venir et les jours suivants
//! (`crate::panorama::modules::meteo`). Un appui sur la tuile relit le
//! bulletin.

use std::time::{Duration, Instant};

use chrono::Timelike;
use iced::widget::{Space, button, column, container, row, svg, text};
use iced::{Alignment, Border, Color, Element, Length, Size, Task};

use super::super::{App, Msg, PMsg, fx, wrap};
use crate::panorama::modules::meteo::{self, Kind, Weather};

/// Relevé du bulletin tant que la tuile est à l'écran.
const METEO_EVERY: Duration = Duration::from_secs(15 * 60);
/// Après un relevé en erreur (réseau absent…) : nouvel essai plus tôt.
const METEO_RETRY: Duration = Duration::from_secs(60);
/// Deux appuis sur la tuile : au moins si loin l'un de l'autre, le service
/// limitant les requêtes d'une même adresse.
const METEO_MIN: Duration = Duration::from_secs(30);
/// Probabilité de pluie à partir de laquelle elle s'affiche ; à partir de
/// `RAIN_ACCENT`, elle prend l'accent.
const RAIN_SHOWN: u8 = 10;
const RAIN_ACCENT: u8 = 50;

/// Encres de la tuile.
#[derive(Debug, Clone, Copy)]
struct Ink {
    fg: Color,
    dim: Color,
    accent: Color,
}

impl Ink {
    /// La pluie : estompée, à l'accent quand elle est probable.
    fn rain(self, pct: u8) -> Color {
        if pct >= RAIN_ACCENT {
            self.accent
        } else {
            self.dim
        }
    }
}

impl App {
    /// Une tuile Météo est-elle affichée ? Zone d'une composition, module de
    /// la bande du profil Raccourcis, ou de la zone d'accueil du côte à côte.
    pub(in crate::ui::plugins::panorama::surface) fn panorama_meteo_shown(&self) -> bool {
        let d = &self.cfg.general.panorama.design;
        self.panorama_surfs().iter().any(|s| {
            self.panorama_geo(s).panes.iter().any(|p| {
                p.widget == "meteo"
                    || (p.widget == "modules"
                        && d.shortcut_profile == "system"
                        && d.modules().iter().any(|m| m == "meteo"))
            })
        }) || self
            .panorama
            .split
            .as_ref()
            // Sans fenêtre invitée (elle couvre les modules) ni choix ouvert.
            .is_some_and(|sp| {
                sp.guest.is_none() && !sp.picking && sp.modules.iter().any(|m| m == "meteo")
            })
    }

    /// Relit le bulletin de la ville réglée.
    pub(in crate::ui) fn panorama_meteo_refresh(&mut self) -> Task<Msg> {
        self.panorama.meteo_at = Some(Instant::now());
        let city = meteo::city(&self.cfg);
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || meteo::fetch(&city))
                    .await
                    .unwrap_or_else(|e| Err(e.to_string()))
            },
            |r| wrap(PMsg::MeteoRead(r)),
        )
    }

    /// À chaque battement : relevé quand la tuile est là et qu'il est temps.
    pub(in crate::ui::plugins::panorama::surface) fn panorama_meteo_tick(&mut self) -> Task<Msg> {
        // Après une erreur, on n'attend pas un quart d'heure.
        let every = if self.panorama.meteo_error.is_some() {
            METEO_RETRY
        } else {
            METEO_EVERY
        };
        if self.panorama.meteo_at.is_some_and(|t| t.elapsed() < every)
            || !self.panorama_meteo_shown()
        {
            return Task::none();
        }
        self.panorama_meteo_refresh()
    }

    pub(in crate::ui::plugins::panorama::surface) fn panorama_meteo_update(
        &mut self,
        m: PMsg,
    ) -> Task<Msg> {
        match m {
            PMsg::MeteoRead(r) => {
                self.panorama.meteo_read = true;
                match r {
                    Ok(w) => {
                        self.panorama.meteo = Some(w);
                        self.panorama.meteo_error = None;
                    }
                    // Le bulletin précédent reste affiché, avec la raison.
                    Err(e) => self.panorama.meteo_error = Some(e),
                }
                Task::none()
            }
            PMsg::MeteoRefresh => {
                self.panorama.last_press = Some(Instant::now());
                if self
                    .panorama
                    .meteo_at
                    .is_some_and(|t| t.elapsed() < METEO_MIN)
                {
                    return Task::none();
                }
                self.panorama_meteo_refresh()
            }
            _ => Task::none(),
        }
    }

    /// Glyphe d'un temps, teinté ; à défaut du jeu, la place qu'il prendrait.
    fn meteo_glyph<'a>(
        &'a self,
        kind: Kind,
        night: bool,
        side: f32,
        ink: Color,
    ) -> Element<'a, Msg> {
        match crate::ui::icons::get(kind.icon(night)) {
            Some(h) => svg(h)
                .width(side)
                .height(side)
                .style(move |_, _| svg::Style { color: Some(ink) })
                .into(),
            None => Space::new().width(side).height(side).into(),
        }
    }

    /// Une colonne des rangées (relevé d'heure, jour) : libellé, glyphe du
    /// temps, température, pluie. La ligne de pluie reste réservée : les
    /// colonnes s'alignent.
    fn meteo_cell<'a>(
        &'a self,
        label: String,
        glyph: Element<'a, Msg>,
        value: String,
        rain: u8,
        scale: f32,
        ink: Ink,
    ) -> Element<'a, Msg> {
        let rain_line: Element<'a, Msg> = if rain >= RAIN_SHOWN {
            text(format!("{rain} %"))
                .size(11.0 * scale)
                .color(ink.rain(rain))
                .into()
        } else {
            Space::new().height(14.0 * scale).into()
        };
        column![
            text(label).size(12.0 * scale).color(ink.dim),
            glyph,
            text(value)
                .size(15.0 * scale)
                .font(iced::Font {
                    weight: iced::font::Weight::Semibold,
                    ..self.panorama_font()
                })
                .color(ink.fg),
            rain_line,
        ]
        .spacing(4.0)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .into()
    }

    /// Le bulletin : en-tête, puis ce que la place permet (`meteo::plan`).
    fn meteo_tile<'a>(
        &'a self,
        w: &Weather,
        plan: meteo::Plan,
        ink: Ink,
        error: Option<&str>,
    ) -> Element<'a, Msg> {
        let scale = self.cfg.general.panorama.design.text_scale;
        let semibold = iced::Font {
            weight: iced::font::Weight::Semibold,
            ..self.panorama_font()
        };
        let lang = crate::i18n::current();
        let now = &w.now;
        // En-tête : le temps qu'il fait ; en face, s'ils tiennent, le lieu et
        // le jour. Le libellé du temps passe sous les deux : long, il ne
        // repousse pas le lieu. Tout y tient sur une ligne, rogné au bord.
        let today = w.days.first();
        let small_line = |s: String| {
            text(s)
                .size(13.0 * scale)
                .color(ink.dim)
                .wrapping(text::Wrapping::None)
        };
        let mut top = row![].align_y(Alignment::Center);
        top = top.push(
            text(format!("{}°", now.temp))
                .size(plan.temp)
                .font(semibold)
                .color(ink.fg)
                .wrapping(text::Wrapping::None),
        );
        if plan.beside {
            // Au moins un écart avec la température, fût-ce serré.
            top = top.push(Space::new().width(meteo::EDGE));
            top = top.push(Space::new().width(Length::Fill));
            let mut side = column![small_line(w.place.clone())].spacing(4.0);
            if let Some(d) = today {
                side = side.push(small_line(format!("↓ {}°  ↑ {}°", d.min, d.max)));
            }
            top = top.push(side);
        }
        let header = row![
            self.meteo_glyph(now.kind, now.night, plan.icon, ink.fg),
            column![
                top,
                text(crate::i18n::tr(now.kind.label(now.night)))
                    .size(15.0 * scale)
                    .color(ink.fg)
                    .wrapping(text::Wrapping::None),
            ]
            .spacing(2.0),
        ]
        .spacing(meteo::EDGE)
        .align_y(Alignment::Center)
        .width(Length::Fill);
        let mut col = column![container(header).width(Length::Fill).clip(true)].spacing(meteo::GAP);
        // Sous l'en-tête, une ligne : les détails, ou le lieu quand il n'est
        // pas à droite.
        if plan.line {
            let line = if plan.beside {
                let dir = meteo::wind_dir(&now.wind_dir, lang);
                let wind = if dir.is_empty() {
                    crate::tr!("Vent {n} km/h", n = now.wind)
                } else {
                    crate::tr!("Vent {n} km/h {dir}", n = now.wind, dir = dir)
                };
                [
                    crate::tr!("Ressentie {n}°", n = now.feels),
                    wind,
                    crate::tr!("Humidité {n} %", n = now.humidity),
                ]
                .join(" · ")
            } else {
                match today {
                    Some(d) => format!("{} · ↓ {}°  ↑ {}°", w.place, d.min, d.max),
                    None => w.place.clone(),
                }
            };
            col = col.push(text(line).size(12.0 * scale).color(ink.dim));
        }
        // Les heures à venir, depuis le relevé en cours. Il se place d'après
        // l'heure de la machine : celle du lieu par défaut, déduit de
        // l'adresse IP ; pour un lieu lointain, la rangée part d'un autre
        // moment de sa journée.
        if plan.hours > 0 {
            let minute = (chrono::Local::now().num_seconds_from_midnight() / 60) as u16;
            let cells = w.hours_from(minute).into_iter().take(plan.hours).map(|h| {
                let glyph = self.meteo_glyph(h.kind, h.night, plan.small, ink.fg);
                self.meteo_cell(
                    meteo::hour_label(h.hour, lang),
                    glyph,
                    format!("{}°", h.temp),
                    h.rain,
                    scale,
                    ink,
                )
            });
            col = col.push(row(cells).spacing(4.0).width(Length::Fill));
        }
        // Les jours suivants : leur nom, le temps de midi, leurs températures.
        if plan.days > 0 {
            let cells = w.next_days().iter().take(plan.days).map(|d| {
                let label = chrono::NaiveDate::parse_from_str(&d.date, "%Y-%m-%d")
                    .map(|date| {
                        date.format_localized("%a", crate::i18n::chrono_locale())
                            .to_string()
                    })
                    .unwrap_or_else(|_| d.date.clone());
                let glyph = self.meteo_glyph(d.kind, false, plan.small, ink.fg);
                self.meteo_cell(
                    label,
                    glyph,
                    format!("{}° / {}°", d.min, d.max),
                    d.rain,
                    scale,
                    ink,
                )
            });
            col = col.push(row(cells).spacing(4.0).width(Length::Fill));
        }
        if let Some(e) = error {
            col = col.push(
                text(e.to_string())
                    .size(12.0 * scale)
                    .color(crate::ui::tone::warning()),
            );
        }
        col.width(Length::Fill).into()
    }

    /// Tuile Météo : le temps qu'il fait, les heures à venir, les jours
    /// suivants ; un appui relit le bulletin.
    pub(in crate::ui::plugins::panorama::surface) fn panorama_meteo(
        &self,
        size: Size,
    ) -> Element<'_, Msg> {
        let (_, fg, accent) = self.panorama_palette();
        let scale = self.cfg.general.panorama.design.text_scale;
        let ink = Ink {
            fg,
            dim: fx::alpha(fg, 0.65),
            accent,
        };
        let plan = meteo::plan(size.width, size.height, scale);
        let body: Element<'_, Msg> = match (&self.panorama.meteo, &self.panorama.meteo_error) {
            (Some(w), error) => self.meteo_tile(w, plan, ink, error.as_deref()),
            // Rien pu être lu : pourquoi, et comment s'en sortir.
            (None, Some(e)) => column![
                text(crate::i18n::tr("Météo indisponible"))
                    .size(15.0 * scale)
                    .font(iced::Font {
                        weight: iced::font::Weight::Semibold,
                        ..self.panorama_font()
                    })
                    .color(fg),
                text(e.clone()).size(12.0 * scale).color(crate::ui::tone::warning()),
                text(crate::i18n::tr(
                    "Réglez la ville dans Réglages → Plugins → Modules ; toucher la tuile pour réessayer.",
                ))
                .size(12.0 * scale)
                .color(ink.dim),
            ]
            .spacing(6.0)
            .width(Length::Fill)
            .into(),
            (None, None) => text(crate::i18n::tr("Lecture…"))
                .size(13.0 * scale)
                .color(ink.dim)
                .into(),
        };
        // Le bulletin au milieu en hauteur : sur une grande tuile, le reste
        // se partage entre le dessus et le dessous.
        self.panorama_feedback(
            button(
                container(body)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_y(Length::Fill),
            )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(meteo::PAD)
            .on_press(wrap(PMsg::MeteoRefresh))
            .style(move |_, st| button::Style {
                background: matches!(st, button::Status::Pressed)
                    .then(|| fx::alpha(accent, 0.3).into()),
                border: Border::default().rounded(14),
                text_color: fg,
                ..Default::default()
            }),
        )
    }
}
