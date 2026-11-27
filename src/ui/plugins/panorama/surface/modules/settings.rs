//! Réglages d'un module sur le Panorama : un panneau sur la bande, ouvert
//! par un appui long sur une tuile, un rang par réglage déclaré
//! (`crate::panorama::modules::settings`). Même panneau pour tous les
//! modules ; ceux qui n'ont rien à régler y renvoient aux Réglages.
//!
//! Un réglage à saisir prête à la bande le clavier le temps de la frappe :
//! Entrée l'enregistre et le rend aussitôt ; toucher le champ le reprend.
//! Le module ayant répondu, le panneau se referme de lui-même (il reste en
//! cas d'erreur, pour corriger).

use std::time::{Duration, Instant};

use iced::advanced::widget::operation::{self, focusable};
use iced::widget::{Space, button, column, container, mouse_area, row, svg, text, text_input};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Task};
use iced_layershell::reexport::KeyboardInteractivity;

use super::super::panes::Surf;
use super::super::{App, Msg, PMsg, PREVIEW, fx, tap, wrap};
use crate::panorama::modules::CATALOGUE;
use crate::panorama::modules::settings::{self, Kind};

/// Le panneau oublié se referme : personne ne l'a touché depuis.
const SHEET_IDLE: Duration = Duration::from_secs(180);
/// Le module ayant répondu, le temps de lire sa réponse avant que le
/// panneau ne se referme (à la seconde du battement près).
const SHEET_ANSWER: Duration = Duration::from_secs(3);

/// Le champ d'un réglage à saisir, dans le panneau de `module`.
fn field_id(module: &str, key: &str) -> iced::widget::Id {
    iced::widget::Id::from(format!("hyprdesk-panorama-setting-{module}-{key}"))
}

/// Nom et glyphe d'un module, au catalogue.
fn catalogue(module: &str) -> Option<(&'static str, &'static str)> {
    CATALOGUE
        .iter()
        .find(|(id, _, _)| *id == module)
        .map(|(_, label, icon)| (*label, *icon))
}

impl App {
    /// La surface du Panorama de l'écran `screen` (ou de la fenêtre d'aperçu).
    pub(in crate::ui::plugins::panorama::surface) fn panorama_id_of(
        &self,
        screen: &str,
    ) -> Option<Id> {
        self.panorama
            .surfaces
            .iter()
            .find(|(_, s)| s.as_str() == screen)
            .map(|(id, _)| *id)
            .or_else(|| {
                self.panorama
                    .preview
                    .as_ref()
                    .filter(|_| screen == PREVIEW)
                    .map(|p| p.win)
            })
    }

    /// Réglages d'un module ouverts sur cet écran ?
    pub(in crate::ui::plugins::panorama::surface) fn panorama_module_sheet_on(
        &self,
        screen: &str,
    ) -> bool {
        self.panorama
            .module_sheet
            .as_ref()
            .is_some_and(|(s, _)| s == screen)
    }

    /// Le panneau des réglages se referme ; il rend le clavier qu'il tenait.
    pub(in crate::ui::plugins::panorama::surface) fn panorama_module_sheet_close(
        &mut self,
    ) -> Task<Msg> {
        if self.panorama.module_sheet.take().is_none() {
            return Task::none();
        }
        self.panorama_module_sheet_release()
    }

    /// Même chose, s'il est ouvert sur cet écran.
    pub(in crate::ui::plugins::panorama::surface) fn panorama_module_sheet_close_on(
        &mut self,
        screen: &str,
    ) -> Task<Msg> {
        if self.panorama_module_sheet_on(screen) {
            self.panorama_module_sheet_close()
        } else {
            Task::none()
        }
    }

    /// La surface `id` disparaît : les réglages ouverts dessus partent avec
    /// elle, et le clavier que leur panneau tenait.
    pub(in crate::ui::plugins::panorama::surface) fn panorama_module_sheet_gone(&mut self, id: Id) {
        let on = self
            .panorama
            .module_sheet
            .as_ref()
            .and_then(|(screen, _)| self.panorama_id_of(screen))
            .is_some_and(|s| s == id);
        if on {
            self.panorama.module_sheet = None;
            self.panorama.module_sheet_keyboard = None;
        }
    }

    /// À chaque battement : le module ayant répondu, le panneau se referme
    /// (à moins qu'on l'ait touché depuis) ; oublié, de même.
    pub(in crate::ui::plugins::panorama::surface) fn panorama_module_sheet_tick(
        &mut self,
    ) -> Task<Msg> {
        if self.panorama.module_sheet.is_none() {
            return Task::none();
        }
        // Touché depuis la réponse : la fermeture prévue ne vaut plus.
        let serial = self.panorama.module_sheet_serial;
        if self
            .panorama
            .module_sheet_close_at
            .is_some_and(|(_, s)| s != serial)
        {
            self.panorama.module_sheet_close_at = None;
        }
        let answered = self
            .panorama
            .module_sheet_close_at
            .is_some_and(|(at, _)| Instant::now() >= at);
        let forgotten = self
            .panorama
            .module_sheet_at
            .is_some_and(|t| t.elapsed() >= SHEET_IDLE);
        if answered || forgotten {
            return self.panorama_update(PMsg::ModuleSettings(None));
        }
        Task::none()
    }

    /// La surface du panneau prend le clavier : il y a un champ à saisir. Le
    /// premier champ aura le focus au tour suivant, la surface redessinée.
    fn panorama_module_sheet_take_keyboard(&mut self) -> Task<Msg> {
        let Some((screen, module)) = self.panorama.module_sheet.clone() else {
            return Task::none();
        };
        let Some(id) = self.panorama_id_of(&screen) else {
            return Task::none();
        };
        if self.panorama.module_sheet_keyboard == Some(id)
            || settings::text_setting(&module).is_none()
        {
            return Task::none();
        }
        self.panorama.module_sheet_keyboard = Some(id);
        // Une seule surface à la fois : celles qui le tiennent le rendent.
        let bar = match self.bar_keyboard {
            Some(id) => self.bar_release(id),
            None => Task::none(),
        };
        Task::batch([
            self.toasts_release(),
            self.launcher_release(),
            bar,
            Task::done(Msg::KeyboardInteractivityChange {
                id,
                keyboard_interactivity: KeyboardInteractivity::Exclusive,
            }),
            Task::done(wrap(PMsg::ModuleSheetFocus)),
        ])
    }

    /// Le panneau rend le clavier ; il ne le tenait pas : rien.
    fn panorama_module_sheet_release(&mut self) -> Task<Msg> {
        let Some(id) = self.panorama.module_sheet_keyboard.take() else {
            return Task::none();
        };
        Task::batch([
            Task::done(Msg::KeyboardInteractivityChange {
                id,
                keyboard_interactivity: KeyboardInteractivity::None,
            }),
            iced::advanced::widget::operate(operation::scope(
                super::super::scope(id),
                focusable::unfocus(),
            )),
        ])
    }

    /// Un réglage de `module` a changé : le module relit ce qu'il montre.
    fn panorama_module_read(&mut self, module: &str) -> Task<Msg> {
        match module {
            "meteo" => self.panorama_meteo_refresh(),
            _ => Task::none(),
        }
    }

    pub(in crate::ui::plugins::panorama::surface) fn panorama_module_sheet_update(
        &mut self,
        m: PMsg,
    ) -> Task<Msg> {
        match m {
            PMsg::ModuleSettings(open) => match open {
                Some((screen, module)) => {
                    self.panorama.last_press = Some(Instant::now());
                    // Une chose à la fois sur l'écran : le volet des
                    // notifications et le choix de la disposition se
                    // referment, la zone d'accueil quitte son édition.
                    self.panorama.notifs = None;
                    self.panorama.picker = None;
                    if let Some(sp) = self.panorama.split.as_mut() {
                        sp.editing = None;
                    }
                    let s = &mut self.panorama;
                    s.module_sheet_drafts.clear();
                    s.module_sheet_status = None;
                    s.module_sheet_busy = false;
                    s.module_sheet_at = Some(Instant::now());
                    s.module_sheet_serial += 1;
                    s.module_sheet_close_at = None;
                    s.module_sheet = Some((screen, module));
                    self.panorama_module_sheet_take_keyboard()
                }
                None => match self.panorama.module_sheet.take() {
                    Some(_) => {
                        self.panorama.last_press = Some(Instant::now());
                        self.panorama_module_sheet_release()
                    }
                    None => Task::none(),
                },
            },
            PMsg::ModuleSheetTouched => {
                self.panorama.last_press = Some(Instant::now());
                self.panorama.module_sheet_at = Some(Instant::now());
                self.panorama.module_sheet_serial += 1;
                // Le champ a rendu le clavier (Entrée) : le toucher le reprend.
                if self.panorama.module_sheet.is_some()
                    && self.panorama.module_sheet_keyboard.is_none()
                {
                    self.panorama_module_sheet_take_keyboard()
                } else {
                    Task::none()
                }
            }
            PMsg::ModuleSheetFocus => {
                let Some((screen, module)) = self.panorama.module_sheet.clone() else {
                    return Task::none();
                };
                let Some(id) = self.panorama_id_of(&screen) else {
                    return Task::none();
                };
                if self.panorama.module_sheet_keyboard != Some(id) {
                    return Task::none();
                }
                let Some(first) = settings::text_setting(&module) else {
                    return Task::none();
                };
                // Dans la surface du panneau seule.
                iced::advanced::widget::operate(operation::scope(
                    super::super::scope(id),
                    focusable::focus(field_id(&module, first.key)),
                ))
            }
            PMsg::ModuleSheetDraft(key, value) => {
                self.panorama.module_sheet_at = Some(Instant::now());
                self.panorama.module_sheet_serial += 1;
                self.panorama.module_sheet_drafts.insert(key, value);
                Task::none()
            }
            PMsg::ModuleSetting(module, key, value) => {
                self.panorama.last_press = Some(Instant::now());
                self.panorama.module_sheet_at = Some(Instant::now());
                let draft = format!("{module}.{key}");
                self.panorama.module_sheet_drafts.remove(&draft);
                self.panorama.module_sheet_status = None;
                // Enregistré : le clavier est rendu aussitôt ; la réponse se
                // lit avant que le panneau ne se referme.
                let keyboard = self.panorama_module_sheet_release();
                let save = self.module_option(&module, &key, &value);
                let read = self.panorama_module_read(&module);
                let check = settings::of(&module)
                    .iter()
                    .find(|s| s.key == key)
                    .and_then(|s| s.verify)
                    .map(|verify| {
                        let value = value.clone();
                        Task::perform(
                            async move {
                                tokio::task::spawn_blocking(move || verify(&value))
                                    .await
                                    .unwrap_or_else(|e| Err(e.to_string()))
                            },
                            |r| wrap(PMsg::ModuleSettingChecked(r)),
                        )
                    });
                self.panorama.module_sheet_busy = check.is_some();
                Task::batch([keyboard, save, read, check.unwrap_or_else(Task::none)])
            }
            PMsg::ModuleSettingChecked(r) => {
                // Réponse heureuse : le panneau n'a plus rien à dire. Il se
                // refermera aux battements qui suivent, le temps de la lire
                // — à moins qu'on l'ait touché. En erreur, il reste : la
                // ville se corrige.
                let ok = r.is_ok();
                let s = &mut self.panorama;
                s.module_sheet_busy = false;
                s.module_sheet_status = Some(r);
                s.module_sheet_close_at =
                    ok.then(|| (Instant::now() + SHEET_ANSWER, s.module_sheet_serial));
                Task::none()
            }
            PMsg::ModuleSheetSettings => {
                let close = self.panorama_module_sheet_close();
                Task::batch([
                    close,
                    Task::done(Msg::Control(crate::control::Command::Settings(
                        "panorama/modules".into(),
                    ))),
                ])
            }
            _ => Task::none(),
        }
    }

    /// Couche des réglages d'un module, sur la surface `id` s'ils y sont
    /// ouverts ; `None` sinon.
    pub(in crate::ui::plugins::panorama::surface) fn panorama_module_sheet_layer<'a>(
        &'a self,
        id: Id,
    ) -> Option<Element<'a, Msg>> {
        let s = self.panorama_surf(id)?;
        let module = self
            .panorama
            .module_sheet
            .as_ref()
            .filter(|(screen, _)| *screen == s.key)?
            .1
            .clone();
        Some(self.panorama_module_sheet(&s, &module))
    }

    /// Le panneau : un voile (un appui à côté ferme) et, au milieu, la carte
    /// du module — son nom, un rang par réglage, la réponse du module.
    fn panorama_module_sheet<'a>(&'a self, s: &Surf, module: &str) -> Element<'a, Msg> {
        let (bg, fg, accent) = self.panorama_palette();
        let font = self.panorama_font();
        let t = self.cfg.general.panorama.design.text_scale;
        let dim = fx::alpha(fg, 0.65);
        let warn = crate::ui::tone::warning();
        let semibold = iced::Font {
            weight: iced::font::Weight::Semibold,
            ..font
        };
        let (label, icon) = catalogue(module).unwrap_or((module, ""));
        let pill = move |on: bool| {
            move |_: &iced::Theme, st: button::Status| button::Style {
                background: Some(
                    fx::alpha(
                        accent,
                        if matches!(st, button::Status::Pressed) {
                            0.6
                        } else if on {
                            0.35
                        } else {
                            0.18
                        },
                    )
                    .into(),
                ),
                border: Border::default().rounded(12),
                text_color: fg,
                ..Default::default()
            }
        };
        // Titre : le glyphe du module, son nom, la fermeture.
        let glyph: Element<'a, Msg> = match crate::ui::icons::get(icon) {
            Some(h) => svg(h)
                .width(26.0 * t)
                .height(26.0 * t)
                .style(move |_, _| svg::Style { color: Some(fg) })
                .into(),
            None => Space::new().into(),
        };
        let title = row![
            glyph,
            text(crate::i18n::tr(label).to_string())
                .size(17.0 * t)
                .font(semibold)
                .color(fg)
                .width(Length::Fill),
            self.panorama_feedback(
                // Fermeture : le glyphe du jeu (HIG ICO-07).
                button(
                    crate::ui::icons::glyph("window-close", 26.0 * t, fg)
                        .expect("glyphe de fermeture"),
                )
                .padding(6)
                .on_press(wrap(PMsg::ModuleSettings(None)))
                .style(pill(false)),
            ),
        ]
        .spacing(12)
        .align_y(Alignment::Center);

        // Un rang par réglage déclaré.
        let p = &self.panorama;
        let mut body = column![].spacing(10.0 * t).width(Length::Fill);
        for setting in settings::of(module) {
            body = body.push(
                text(crate::i18n::tr(setting.label))
                    .size(12.0 * t)
                    .color(dim),
            );
            body = body.push(match setting.kind {
                Kind::Text { placeholder } => {
                    let key = format!("{module}.{}", setting.key);
                    let shown = p
                        .module_sheet_drafts
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(|| setting.value(&self.cfg, module));
                    let save = |shown: String| {
                        wrap(PMsg::ModuleSetting(
                            module.to_string(),
                            setting.key.to_string(),
                            shown,
                        ))
                    };
                    let field = text_input(crate::i18n::tr(placeholder), &shown)
                        .id(field_id(module, setting.key))
                        .font(font)
                        .size(15.0 * t)
                        .padding([11, 14])
                        .width(Length::Fill)
                        .on_input(move |v| wrap(PMsg::ModuleSheetDraft(key.clone(), v)))
                        .on_submit(save(shown.clone()))
                        .style(move |_, st| {
                            let focused = matches!(st, text_input::Status::Focused { .. });
                            text_input::Style {
                                background: fx::alpha(fg, 0.08).into(),
                                border: Border {
                                    radius: 12.into(),
                                    width: if focused { 2.0 } else { 1.0 },
                                    color: if focused { accent } else { fx::alpha(fg, 0.3) },
                                },
                                icon: fg,
                                placeholder: dim,
                                value: fg,
                                selection: Color { a: 0.4, ..accent },
                            }
                        });
                    // Toucher le champ : le clavier revient s'il a été rendu.
                    let field: Element<'a, Msg> =
                        tap::tap(field, wrap(PMsg::ModuleSheetTouched)).into();
                    row![
                        field,
                        self.panorama_feedback(
                            // Haut comme le champ, à côté.
                            button(
                                text(crate::i18n::tr("Enregistrer"))
                                    .size(13.0 * t)
                                    .color(fg),
                            )
                            .padding([10, 16])
                            .height(46.0 * t)
                            .on_press(save(shown))
                            .style(pill(true)),
                        ),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center)
                    .width(Length::Fill)
                    .into()
                }
                Kind::Screens => {
                    // L'écran actif, puis chaque écran branché.
                    let current = setting.value(&self.cfg, module);
                    let mut chips = row![].spacing(8);
                    let screens = std::iter::once((
                        String::new(),
                        crate::i18n::tr(settings::ACTIVE_SCREEN).to_string(),
                    ))
                    .chain(
                        self.monitors
                            .iter()
                            .map(|m| (m.name.clone(), m.name.clone())),
                    );
                    for (value, name) in screens {
                        let on = value == current;
                        chips = chips.push(
                            self.panorama_feedback(
                                button(text(name).size(13.0 * t).color(fg))
                                    .padding([10, 16])
                                    .height(40.0 * t)
                                    .on_press(wrap(PMsg::ModuleSetting(
                                        module.to_string(),
                                        setting.key.to_string(),
                                        value,
                                    )))
                                    .style(pill(on)),
                            ),
                        );
                    }
                    let chips: Element<'a, Msg> = chips.width(Length::Fill).wrap().into();
                    chips
                }
            });
        }
        if settings::of(module).is_empty() {
            let hint = match module {
                // Le pont s'appaire dans les Réglages : bouton du pont, pièces montrées.
                "hue" => "Le pont Hue s'appaire dans les Réglages de l'écran principal.",
                _ => "Ce module n'a pas de réglage.",
            };
            body = body.push(text(crate::i18n::tr(hint)).size(13.0 * t).color(dim));
        }
        // La réponse du module à son dernier réglage enregistré.
        if p.module_sheet_busy {
            body = body.push(text(crate::i18n::tr("Lecture…")).size(12.0 * t).color(dim));
        } else if let Some(st) = &p.module_sheet_status {
            let (msg, c) = match st {
                Ok(m) => (m.clone(), dim),
                Err(e) => (e.clone(), warn),
            };
            body = body.push(text(msg).size(12.0 * t).color(c));
        }

        let footer = self.panorama_feedback(
            button(
                text(crate::i18n::tr("Réglages complets…"))
                    .size(12.0 * t)
                    .color(dim),
            )
            .padding([4, 0])
            .on_press(wrap(PMsg::ModuleSheetSettings))
            .style(button::text),
        );
        let width = (s.w * 0.62).clamp(360.0, 760.0).min(s.w - 16.0);
        let radius = self.cfg.general.panorama.design.radius.min(18.0);
        let card = container(
            column![title, body, footer]
                .spacing(14.0 * t)
                .width(Length::Fill),
        )
        .padding(18.0 * t)
        .width(width)
        .style(move |_| container::Style {
            background: Some(fx::alpha(bg, 0.97).into()),
            border: Border {
                width: 1.0,
                color: fx::alpha(accent, 0.4),
                radius: radius.into(),
            },
            ..Default::default()
        });
        // La carte absorbe ses appuis ; à côté, un appui ferme.
        let panel = mouse_area(card).on_press(wrap(PMsg::ModuleSheetTouched));
        mouse_area(
            container(panel)
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(fx::alpha(bg, 0.35).into()),
                    ..Default::default()
                }),
        )
        .on_press(wrap(PMsg::ModuleSettings(None)))
        .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panorama::modules::meteo;
    use iced::keyboard::{Key, key::Named};

    /// Un Panorama d'aperçu à l'écran : sa fenêtre, l'écran `PREVIEW`.
    fn app() -> (App, Id, String) {
        let (mut app, _) = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-panorama-sheet/config.lua"),
            true,
        );
        let win = Id::unique();
        app.panorama.preview = Some(crate::ui::plugins::panorama::surface::Preview {
            win,
            virtual_size: (1600, 450),
            scale: 1.0,
        });
        (app, win, PREVIEW.to_string())
    }

    fn open(app: &mut App, screen: &str, module: &str) {
        let _ = app.panorama_update(PMsg::ModuleSettings(Some((
            screen.to_string(),
            module.to_string(),
        ))));
    }

    /// Le panneau des réglages s'ouvre seul à l'écran, prend le clavier pour
    /// son champ, retarde la veille ; le geste du bord haut le referme, et
    /// rend le clavier.
    #[test]
    fn ouvrir_referme_le_reste_et_prend_le_clavier() {
        let (mut app, win, screen) = app();
        assert!(
            !app.listens_to_events(),
            "au repos, le Panorama n'écoute pas"
        );
        let _ = app.panorama_update(PMsg::Notifs(Some(screen.clone())));
        let _ = app.panorama_update(PMsg::Cross(screen.clone(), 1));
        assert!(app.panorama.picker.is_some());
        open(&mut app, &screen, "meteo");
        let sheet = app
            .panorama
            .module_sheet
            .as_ref()
            .map(|(s, m)| (s.as_str(), m.as_str()));
        assert_eq!(sheet, Some((PREVIEW, "meteo")));
        assert!(
            app.panorama.picker.is_none() && app.panorama.notifs.is_none(),
            "une chose à la fois à l'écran"
        );
        assert_eq!(app.panorama.module_sheet_keyboard, Some(win));
        assert!(app.listens_to_events(), "Échap doit arriver au panneau");
        // Depuis une heure : la veille attendrait, sans le panneau.
        app.panorama.last_press = Some(Instant::now() - Duration::from_secs(3600));
        assert!(!app.panorama_asleep());
        let _ = app.panorama_update(PMsg::Cross(screen.clone(), -1));
        assert!(app.panorama.module_sheet.is_none(), "le bord haut referme");
        assert_eq!(
            app.panorama.module_sheet_keyboard, None,
            "et rend le clavier"
        );
        assert!(!app.listens_to_events());
        // Le geste vient de toucher l'écran ; une heure plus tard, la veille
        // reprend son droit.
        app.panorama.last_press = Some(Instant::now() - Duration::from_secs(3600));
        assert!(app.panorama_asleep());
    }

    /// Sans champ à saisir (module sans réglage, ou réglage par pastilles) :
    /// le Panorama ne prend pas le clavier.
    #[test]
    fn sans_champ_pas_de_clavier() {
        let (mut app, win, screen) = app();
        for module in ["clock", "hue"] {
            open(&mut app, &screen, module);
            assert!(app.panorama.module_sheet.is_some(), "{module}");
            assert_eq!(app.panorama.module_sheet_keyboard, None, "{module}");
            assert!(!app.listens_to_events(), "{module}");
            let _ = app.panorama_update(PMsg::ModuleSettings(None));
        }
        // Notes : une commande à saisir.
        open(&mut app, &screen, "notes");
        assert_eq!(app.panorama.module_sheet_keyboard, Some(win));
    }

    /// Entrée enregistre le réglage, et rend le clavier aussitôt ; le panneau
    /// reste, pour la réponse du module. Vide : le réglage revient par défaut.
    #[test]
    fn entrer_enregistre_et_rend_le_clavier() {
        let (mut app, _win, screen) = app();
        open(&mut app, &screen, "meteo");
        let _ = app.panorama_update(PMsg::ModuleSheetDraft("meteo.city".into(), "Lyon".into()));
        let _ = app.panorama_update(PMsg::ModuleSetting(
            "meteo".into(),
            "city".into(),
            "Lyon".into(),
        ));
        assert_eq!(meteo::city(&app.cfg), "Lyon");
        assert_eq!(
            app.panorama.module_sheet_keyboard, None,
            "le clavier est rendu"
        );
        assert!(app.panorama.module_sheet.is_some(), "le panneau reste");
        assert!(app.panorama.module_sheet_busy, "la ville se vérifie");
        assert!(
            !app.panorama.module_sheet_drafts.contains_key("meteo.city"),
            "le brouillon est consommé"
        );
        let lieu = meteo::summary(&meteo::sample());
        let _ = app.panorama_update(PMsg::ModuleSettingChecked(Ok(lieu.clone())));
        assert!(!app.panorama.module_sheet_busy);
        assert_eq!(
            app.panorama
                .module_sheet_status
                .as_ref()
                .map(|r| r.as_ref().ok()),
            Some(Some(&lieu))
        );
        // Écran de Notes : une pastille, sans clavier en jeu.
        let _ = app.panorama_update(PMsg::ModuleSetting(
            "notes".into(),
            "monitor".into(),
            "DP-2".into(),
        ));
        assert_eq!(crate::panorama::modules::notes::options(&app.cfg).1, "DP-2");
        // La ville effacée : plus de réglage, la position par l'adresse IP.
        let _ = app.panorama_update(PMsg::ModuleSetting(
            "meteo".into(),
            "city".into(),
            String::new(),
        ));
        assert_eq!(meteo::city(&app.cfg), "");
        assert!(
            !app.cfg.general.plugin_options["panorama"].contains_key("meteo.city"),
            "la clé est retirée"
        );
    }

    /// Enregistrer depuis le panneau garde la composition du Panorama. Chez
    /// le démon, les réglages en cours sont amorcés depuis settings.lua
    /// (`DeskSettings::from_config`) : sans eux, `apply_to` remettrait le
    /// Panorama à ses valeurs par défaut.
    #[test]
    fn enregistrer_garde_la_composition() {
        fn tiles(app: &App) -> Vec<String> {
            app.cfg
                .general
                .panorama
                .design
                .tiles
                .iter()
                .map(|t| t.widget.clone())
                .collect()
        }
        let (mut app, _win, screen) = app();
        {
            let g = &mut std::sync::Arc::make_mut(&mut app.cfg).general;
            g.panorama.design.tiles = vec![crate::panorama::design::Tile::new(
                "meteo", 0.0, 0.0, 1.0, 1.0,
            )];
            // Comme au chargement, chez le démon.
            app.settings.panorama = Some(g.panorama.clone());
        }
        open(&mut app, &screen, "meteo");
        let _ = app.panorama_update(PMsg::ModuleSetting(
            "meteo".into(),
            "city".into(),
            "nantes".into(),
        ));
        assert_eq!(meteo::city(&app.cfg), "nantes");
        assert_eq!(tiles(&app), ["meteo"], "la composition reste");
    }

    /// Le module ayant répondu, le panneau se referme de lui-même — à moins
    /// qu'on l'ait touché depuis ; en erreur, il reste.
    #[test]
    fn la_reponse_du_module_referme_le_panneau() {
        let (mut app, _win, screen) = app();
        open(&mut app, &screen, "meteo");
        // En erreur : aucune fermeture prévue, la ville se corrige.
        let _ = app.panorama_update(PMsg::ModuleSettingChecked(Err("introuvable".into())));
        assert!(
            app.panorama.module_sheet_close_at.is_none(),
            "en erreur, il reste"
        );
        let _ = app.panorama_module_sheet_tick();
        assert!(app.panorama.module_sheet.is_some());

        // Réponse heureuse : fermeture prévue, pas avant son heure.
        let _ = app.panorama_update(PMsg::ModuleSettingChecked(Ok("Rezé, France".into())));
        let (at, serial) = app.panorama.module_sheet_close_at.unwrap();
        let _ = app.panorama_module_sheet_tick();
        assert!(app.panorama.module_sheet.is_some(), "le temps de lire");

        // Touché depuis la réponse : la fermeture prévue est annulée.
        let _ = app.panorama_update(PMsg::ModuleSheetTouched);
        app.panorama.module_sheet_close_at = Some((at, serial));
        let _ = app.panorama_module_sheet_tick();
        assert!(app.panorama.module_sheet.is_some(), "touché : il reste");
        assert!(
            app.panorama.module_sheet_close_at.is_none(),
            "la fermeture prévue est retirée"
        );

        // Rien touché depuis la réponse : à l'heure, le panneau se referme.
        let _ = app.panorama_update(PMsg::ModuleSettingChecked(Ok("Rezé, France".into())));
        app.panorama.module_sheet_close_at =
            Some((Instant::now(), app.panorama.module_sheet_serial));
        let _ = app.panorama_module_sheet_tick();
        assert!(app.panorama.module_sheet.is_none(), "refermé");
        assert_eq!(app.panorama.module_sheet_keyboard, None);
        // Sans panneau : le battement ne fait rien.
        let _ = app.panorama_module_sheet_tick();
    }

    /// Le clavier part à autre chose : le panneau lâche sa prise et reste ;
    /// touché, il le reprend.
    #[test]
    fn le_clavier_parti_revient_au_toucher() {
        let (mut app, win, screen) = app();
        open(&mut app, &screen, "meteo");
        let _ = app.update_inner(Msg::KeyboardLeft(win));
        assert_eq!(app.panorama.module_sheet_keyboard, None, "prise lâchée");
        assert!(app.panorama.module_sheet.is_some(), "le panneau reste");
        let _ = app.panorama_update(PMsg::ModuleSheetTouched);
        assert_eq!(app.panorama.module_sheet_keyboard, Some(win), "reprise");
    }

    /// Échap referme le panneau (et rend le clavier) ; une autre touche, qui
    /// vient du champ, ne le touche pas.
    #[test]
    fn echap_referme() {
        let (mut app, win, screen) = app();
        open(&mut app, &screen, "meteo");
        let _ = app.update_inner(Msg::Key(win, Key::Named(Named::Enter)));
        assert!(app.panorama.module_sheet.is_some(), "Entrée est au champ");
        let _ = app.update_inner(Msg::Key(win, Key::Named(Named::Escape)));
        assert!(app.panorama.module_sheet.is_none());
        assert_eq!(app.panorama.module_sheet_keyboard, None);
        assert!(!app.listens_to_events());
    }

    /// « Réglages complets… », l'écran escamoté et la surface qui disparaît
    /// ferment aussi le panneau.
    #[test]
    fn toutes_les_fermetures_rendent_le_clavier() {
        let (mut app, _win, _) = app();
        open(&mut app, PREVIEW, "meteo");
        let _ = app.panorama_update(PMsg::ModuleSheetSettings);
        assert!(app.panorama.module_sheet.is_none(), "Réglages complets");
        assert_eq!(app.panorama.module_sheet_keyboard, None);

        // Un vrai écran, qu'on escamote.
        let (id, name) = (Id::unique(), "HDMI-A-1".to_string());
        app.panorama.surfaces.insert(id, name.clone());
        open(&mut app, &name, "meteo");
        assert_eq!(app.panorama.module_sheet_keyboard, Some(id));
        let _ = app.panorama_update(PMsg::Hide(name.clone()));
        assert!(app.panorama.hidden.contains(&name));
        assert!(app.panorama.module_sheet.is_none(), "écran escamoté");
        assert_eq!(app.panorama.module_sheet_keyboard, None);

        // La surface qui disparaît (écran débranché). L'escamotage a
        // resynchronisé les surfaces : on repose la nôtre.
        app.panorama.surfaces.insert(id, name.clone());
        open(&mut app, &name, "meteo");
        assert_eq!(app.panorama.module_sheet_keyboard, Some(id));
        assert!(app.panorama_closed(id));
        assert!(app.panorama.module_sheet.is_none(), "surface fermée");
        assert_eq!(app.panorama.module_sheet_keyboard, None);
    }

    /// Le panneau oublié se referme ; touché, il reste.
    #[test]
    fn le_panneau_oublie_se_referme() {
        let (mut app, win, screen) = app();
        open(&mut app, &screen, "meteo");
        let _ = app.panorama_module_sheet_tick();
        assert!(app.panorama.module_sheet.is_some(), "touché : il reste");
        assert_eq!(app.panorama.module_sheet_keyboard, Some(win));
        app.panorama.module_sheet_at = Some(Instant::now() - SHEET_IDLE - Duration::from_secs(1));
        let _ = app.panorama_module_sheet_tick();
        assert!(app.panorama.module_sheet.is_none(), "oublié : fermé");
        assert_eq!(app.panorama.module_sheet_keyboard, None);
    }

    /// Les réponses d'un module sans réglage : traduites (elles passent par
    /// une variable, que le relevé automatique ne voit pas).
    #[test]
    fn sans_reglage_traduit() {
        for s in [
            "Ce module n'a pas de réglage.",
            "Le pont Hue s'appaire dans les Réglages de l'écran principal.",
        ] {
            assert!(crate::i18n::has_english(s), "{s}");
        }
    }
}
