//! Conflits de raccourcis posés en question (MLG-209) : quand un raccourci
//! de hyprdesk (SUPER + D…) est déjà lié dans la config de Hyprland, une
//! fenêtre demande pour chacun : le prendre pour hyprdesk (`keys.take`),
//! garder l'existant (hyprdesk ne l'a alors que bureau affiché), ou capturer
//! une autre combinaison.
//!
//! Chaque réponse est retenue (`config::keys_answers_path` : clé,
//! combinaison, commande qui la tient) : un conflit répondu n'est plus
//! demandé, un nouveau l'est — après une mise à jour aussi, ou si un autre
//! outil prend la combinaison. Installation et remise à zéro effacent les
//! réponses. « Plus tard » : la question revient au prochain démarrage.

use iced::keyboard::key::{Named, Physical};
use iced::keyboard::{Key, Modifiers};
use iced::widget::{Space, button, column, container, row, scrollable, text};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Shadow, Task, Vector};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::{App, Msg, color};
use crate::keymap::Kind;

pub const NAMESPACE: &str = "hyprdesk-key-conflict";
const WIDTH: f32 = 640.0;
const HEIGHT: f32 = 460.0;
const SHADOW: f32 = 18.0;

/// Ce que l'utilisateur a choisi pour un conflit.
#[derive(Debug, Clone, PartialEq)]
pub enum Answer {
    Taken,
    Kept,
    Changed(String),
}

/// Un raccourci de hyprdesk en conflit.
#[derive(Debug, Clone)]
pub struct Item {
    /// Clé de configuration (`keys.toggle`).
    pub key: &'static str,
    /// Libellé français (traduit à l'affichage).
    pub label: &'static str,
    pub combo: String,
    /// Qui la tient (« wofi --show drun » (ui/binds.lua, ligne 62)).
    pub holder: String,
    /// Identifiant de la réponse : clé, combinaison, commande (sans la
    /// ligne, qui bouge à chaque retouche du fichier).
    pub id: String,
    /// Une autre combinaison peut être capturée (pas « Fenêtres, en
    /// arrière », déduit de `keys.switch`).
    pub changeable: bool,
    pub answer: Option<Answer>,
}

pub struct KeyConflict {
    pub win: Id,
    pub items: Vec<Item>,
    /// Conflit dont on capture la nouvelle combinaison.
    pub capture: Option<usize>,
    pub capture_seq: u64,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum KcMsg {
    Take(usize),
    Keep(usize),
    Change(usize),
    CaptureKey(Key, Physical, Modifiers),
    CaptureCancel,
    CaptureTimeout(u64),
    Close,
}

/// Les conflits d'un relevé qui se règlent ici : un raccourci de hyprdesk
/// lié ailleurs dans la config de Hyprland (pas un doublon interne).
pub fn items(report: &crate::keymap::Report) -> Vec<Item> {
    let mut out: Vec<Item> = Vec::new();
    for i in &report.issues {
        let (Kind::Hyprland(h) | Kind::Universal(h)) = &i.kind else {
            continue;
        };
        if out.iter().any(|o| o.combo == i.action.combo) {
            continue;
        }
        let combo = crate::keymap::norm(&i.action.combo)
            .map(|(m, k)| format!("{m}:{k}"))
            .unwrap_or_else(|| i.action.combo.clone());
        out.push(Item {
            id: format!(
                "{}\t{}\t{}",
                i.action.key,
                combo,
                h.what.replace(['\t', '\n'], " ")
            ),
            key: i.action.key,
            label: i.action.label,
            combo: i.action.combo.clone(),
            holder: h.describe(),
            changeable: i.action.id != "switch-back"
                && super::plugins::keys::key_index(i.action.key).is_some(),
            answer: None,
        });
    }
    out
}

/// Réponses déjà données (identifiants d'`Item`).
pub fn answered(config_path: &std::path::Path) -> Vec<String> {
    std::fs::read_to_string(crate::config::keys_answers_path(config_path))
        .map(|s| s.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

/// Retient la réponse à un conflit.
fn remember(config_path: &std::path::Path, id: &str) {
    let mut list = answered(config_path);
    if list.iter().any(|l| l == id) {
        return;
    }
    list.push(id.to_string());
    let path = crate::config::keys_answers_path(config_path);
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if let Err(e) = std::fs::write(&path, list.join("\n") + "\n") {
        tracing::warn!(
            "{}",
            crate::tr!("raccourcis : réponse non retenue ({e})", e = e)
        );
    }
}

/// Conflits sans réponse : ceux que la fenêtre pose en question.
pub fn unanswered(report: &crate::keymap::Report, config_path: &std::path::Path) -> Vec<Item> {
    let done = answered(config_path);
    items(report)
        .into_iter()
        .filter(|i| !done.contains(&i.id))
        .collect()
}

impl App {
    /// Conflits sans réponse : la fenêtre (ou, déjà ouverte, ils s'y
    /// ajoutent) ; rien après « Plus tard » jusqu'au prochain démarrage.
    pub(super) fn review_key_conflicts(&mut self) -> Task<Msg> {
        if self.key_conflict_later {
            return Task::none();
        }
        let found = unanswered(&self.key_report, &self.config_path);
        if let Some(kc) = self.key_conflict.as_mut() {
            // Déjà ouverte : les conflits répondus restent, les nouveaux s'ajoutent.
            for it in found {
                if !kc.items.iter().any(|o| o.combo == it.combo) {
                    kc.items.push(it);
                }
            }
            return Task::none();
        }
        if found.is_empty() {
            return Task::none();
        }
        let win = Id::unique();
        self.key_conflict = Some(KeyConflict {
            win,
            items: found,
            capture: None,
            capture_seq: 0,
            error: None,
        });
        Task::done(Msg::NewLayerShell {
            settings: NewLayerShellSettings {
                size: Some((
                    (WIDTH + 2.0 * SHADOW) as u32,
                    (HEIGHT + 2.0 * SHADOW) as u32,
                )),
                layer: Layer::Overlay,
                anchor: LsAnchor::empty(),
                exclusive_zone: Some(-1),
                keyboard_interactivity: KeyboardInteractivity::OnDemand,
                output_option: OutputOption::Active,
                namespace: Some(NAMESPACE.into()),
                ..Default::default()
            },
            id: win,
        })
    }

    /// Fin de capture : Hyprland retrouve sa submap.
    fn kc_capture_end(&mut self) -> Task<Msg> {
        if let Some(kc) = self.key_conflict.as_mut() {
            kc.capture = None;
        }
        let back = self.in_submap.then_some(crate::hypr::SUBMAP);
        super::plugins::keys::submap_task(back)
    }

    /// Combinaison capturée pour le conflit `n` : refusée si elle est déjà
    /// prise (par hyprdesk ou par Hyprland), enregistrée sinon.
    fn kc_apply(&mut self, n: usize, combo: String) -> Task<Msg> {
        let Some(item) = self
            .key_conflict
            .as_ref()
            .and_then(|k| k.items.get(n))
            .cloned()
        else {
            return Task::none();
        };
        let Some(i) = super::plugins::keys::key_index(item.key) else {
            return Task::none();
        };
        let error = |app: &mut App, e: String| {
            if let Some(kc) = app.key_conflict.as_mut() {
                kc.error = Some(e);
            }
            Task::none()
        };
        let same =
            |a: &str, b: &str| match (crate::hypr::parse_combo(a), crate::hypr::parse_combo(b)) {
                (Some(x), Some(y)) => crate::binds_file::same_combo(&x, &y),
                _ => false,
            };
        if let Some(l) = self
            .key_report
            .hyprland
            .iter()
            .find(|l| l.origin != crate::essentials::Origin::Hyprdesk && same(&l.combo, &combo))
        {
            let what = l.what.clone();
            return error(
                self,
                crate::tr!(
                    "« {c} » est déjà lié à « {w} » : choisissez une autre combinaison.",
                    c = combo.as_str(),
                    w = what
                ),
            );
        }
        let mut keys = self.cfg.general.keys.clone();
        match item.key {
            "keys.toggle" => keys.toggle = combo.clone(),
            "keys.settings" => keys.settings = combo.clone(),
            "keys.leave" => keys.leave = combo.clone(),
            "keys.launcher" => keys.launcher = combo.clone(),
            "keys.hide" => keys.hide = combo.clone(),
            "keys.lock" => keys.lock = combo.clone(),
            "keys.search" => keys.search = combo.clone(),
            "keys.switch" => keys.switch = combo.clone(),
            "keys.overview" => keys.overview = combo.clone(),
            "keys.search_desktop" => keys.search_desktop = combo.clone(),
            _ => return Task::none(),
        }
        if let Some(other) =
            crate::keymap::duplicate_of(&keys, self.cfg.general.apps_launcher.unified, item.key)
        {
            return error(
                self,
                crate::tr!(
                    "« {v} » sert déjà à « {other} » : choisissez une autre combinaison.",
                    v = combo.as_str(),
                    other = crate::i18n::tr(other)
                ),
            );
        }
        if let Some(kc) = self.key_conflict.as_mut() {
            kc.error = None;
            if let Some(it) = kc.items.get_mut(n) {
                it.answer = Some(Answer::Changed(combo.clone()));
                remember(&self.config_path, &it.id);
            }
        }
        self.set_key(i, Some(combo));
        self.commit()
    }

    pub(super) fn key_conflict_update(&mut self, m: KcMsg) -> Task<Msg> {
        let Some(kc) = self.key_conflict.as_mut() else {
            return Task::none();
        };
        match m {
            KcMsg::Take(n) => {
                let Some(it) = kc.items.get_mut(n) else {
                    return Task::none();
                };
                it.answer = Some(Answer::Taken);
                remember(&self.config_path, &it.id);
                let combo = it.combo.clone();
                kc.error = None;
                let mut list = self.cfg.general.keys.take.clone();
                if !list.iter().any(|t| {
                    crate::keymap::norm(t).is_some_and(|x| Some(x) == crate::keymap::norm(&combo))
                }) {
                    list.push(combo);
                }
                self.settings.key_take = Some(list);
                self.commit()
            }
            KcMsg::Keep(n) => {
                // Rien à écrire : la combinaison reste à son raccourci
                // d'origine ; rendue si hyprdesk l'avait prise ici.
                let Some(it) = kc.items.get_mut(n) else {
                    return Task::none();
                };
                let was_taken = it.answer == Some(Answer::Taken);
                it.answer = Some(Answer::Kept);
                remember(&self.config_path, &it.id);
                kc.error = None;
                if !was_taken {
                    return Task::none();
                }
                let combo = it.combo.clone();
                let mut list = self.cfg.general.keys.take.clone();
                list.retain(|t| {
                    !crate::keymap::norm(t).is_some_and(|x| Some(x) == crate::keymap::norm(&combo))
                });
                self.settings.key_take = Some(list);
                self.commit()
            }
            KcMsg::Change(n) => {
                kc.capture = Some(n);
                kc.capture_seq += 1;
                kc.error = None;
                let seq = kc.capture_seq;
                Task::batch([
                    super::plugins::keys::submap_task(Some(crate::hypr::CAPTURE_SUBMAP)),
                    Task::perform(
                        async move {
                            tokio::time::sleep(std::time::Duration::from_secs(15)).await;
                            seq
                        },
                        |s| Msg::KeyConflict(KcMsg::CaptureTimeout(s)),
                    ),
                ])
            }
            KcMsg::CaptureCancel => {
                kc.error = None;
                self.kc_capture_end()
            }
            KcMsg::CaptureTimeout(seq) => {
                if kc.capture.is_some() && kc.capture_seq == seq {
                    self.kc_capture_end()
                } else {
                    Task::none()
                }
            }
            KcMsg::CaptureKey(key, physical, mods) => {
                let Some(n) = kc.capture else {
                    return Task::none();
                };
                let plain = !(mods.logo() || mods.alt() || mods.control() || mods.shift());
                if key == Key::Named(Named::Escape) && plain {
                    kc.error = None;
                    return self.kc_capture_end();
                }
                match super::plugins::keys::combo_of(&key, &physical, mods) {
                    Ok(None) => Task::none(),
                    Err(e) => {
                        kc.error = Some(e);
                        Task::none()
                    }
                    Ok(Some(c)) => {
                        let end = self.kc_capture_end();
                        Task::batch([end, self.kc_apply(n, c)])
                    }
                }
            }
            KcMsg::Close => {
                let capturing = kc.capture.is_some();
                let win = kc.win;
                // « Plus tard » : la question revient au prochain démarrage
                // (les réponses données sont retenues, chacune à son clic).
                let all = kc.items.iter().all(|i| i.answer.is_some());
                self.key_conflict = None;
                if !all {
                    self.key_conflict_later = true;
                }
                let close = iced::window::close(win);
                if capturing {
                    Task::batch([close, self.kc_capture_end()])
                } else {
                    close
                }
            }
        }
    }

    pub(super) fn view_key_conflict(&self) -> Element<'_, Msg> {
        let Some(kc) = &self.key_conflict else {
            return Space::new().into();
        };
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let dim = Color { a: 0.65, ..fg };
        let accent = color(th.accent, 1.0);
        let bg = color(
            th.bg,
            th.effect(crate::config::Place::Menus).opacity.max(0.97),
        );
        let font = self.font;
        let btn = |label: String, on: KcMsg, primary: bool| {
            button(text(label).font(font).size(13))
                .padding([6, 12])
                .style(if primary {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(Msg::KeyConflict(on))
        };
        let mut list = column![].spacing(10);
        for (n, it) in kc.items.iter().enumerate() {
            let head = row![
                container(
                    text(crate::essentials::pretty(&it.combo))
                        .font(font)
                        .size(14)
                        .color(accent)
                )
                .padding([3, 8])
                .style(move |_| container::Style {
                    background: Some(Color { a: 0.12, ..accent }.into()),
                    border: Border::default().rounded(6),
                    ..Default::default()
                }),
                text(crate::i18n::tr(it.label))
                    .font(font)
                    .size(14)
                    .color(fg),
            ]
            .spacing(10)
            .align_y(Alignment::Center);
            let detail = text(crate::tr!(
                "Déjà lié à {h}. Gardé tel quel, le raccourci de hyprdesk n'agit que bureau affiché.",
                h = it.holder.as_str()
            ))
            .font(font)
            .size(12)
            .color(dim);
            let status: Element<'_, Msg> = match &it.answer {
                Some(Answer::Taken) => text(crate::i18n::tr("Pris pour hyprdesk."))
                    .font(font)
                    .size(12)
                    .color(accent)
                    .into(),
                Some(Answer::Kept) => text(crate::i18n::tr("Gardé tel quel."))
                    .font(font)
                    .size(12)
                    .color(accent)
                    .into(),
                Some(Answer::Changed(c)) => text(crate::tr!(
                    "Remplacé par {c}.",
                    c = crate::essentials::pretty(c)
                ))
                .font(font)
                .size(12)
                .color(accent)
                .into(),
                None => Space::new().into(),
            };
            let actions: Element<'_, Msg> = if kc.capture == Some(n) {
                row![
                    text(crate::i18n::tr(
                        "Appuyez sur la nouvelle combinaison… (Échap : annuler)"
                    ))
                    .font(font)
                    .size(13)
                    .color(fg)
                    .width(Length::Fill),
                    btn(
                        crate::i18n::tr("Annuler").to_string(),
                        KcMsg::CaptureCancel,
                        false
                    ),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .into()
            } else {
                let unanswered = it.answer.is_none();
                row![
                    status,
                    Space::new().width(Length::Fill),
                    btn(
                        crate::i18n::tr("Prendre pour hyprdesk").to_string(),
                        KcMsg::Take(n),
                        unanswered
                    ),
                    btn(
                        crate::i18n::tr("Garder l'existant").to_string(),
                        KcMsg::Keep(n),
                        false
                    ),
                ]
                .push(it.changeable.then(|| {
                    btn(
                        crate::i18n::tr("Changer…").to_string(),
                        KcMsg::Change(n),
                        false,
                    )
                }))
                .spacing(8)
                .align_y(Alignment::Center)
                .into()
            };
            list = list.push(
                container(column![head, detail, actions].spacing(6))
                    .padding(12)
                    .width(Length::Fill)
                    .style(move |_| container::Style {
                        background: Some(Color { a: 0.06, ..fg }.into()),
                        border: Border::default().rounded(10),
                        ..Default::default()
                    }),
            );
        }
        let all = kc.items.iter().all(|i| i.answer.is_some());
        let col = column![
            text(crate::i18n::tr("Raccourcis déjà pris"))
                .font(self.title_font())
                .size(20)
                .color(fg),
            text(crate::i18n::tr(
                "Ces raccourcis de hyprdesk sont déjà liés dans la configuration de Hyprland. Pour chacun : le prendre pour hyprdesk, garder l'existant, ou choisir une autre combinaison."
            ))
            .font(font)
            .size(12)
            .color(dim),
            scrollable(list).height(Length::Fill),
            text(kc.error.clone().unwrap_or_default())
                .font(font)
                .size(12)
                .color(Color::from_rgb(0.95, 0.45, 0.45)),
            row![
                text(crate::i18n::tr("Modifiable plus tard dans Réglages → Raccourcis."))
                    .font(font)
                    .size(12)
                    .color(dim)
                    .width(Length::Fill),
                button(
                    text(if all {
                        crate::i18n::tr("Terminé")
                    } else {
                        crate::i18n::tr("Plus tard")
                    })
                    .font(font)
                    .size(14)
                )
                .padding([8, 16])
                .style(if all { button::primary } else { button::text })
                .on_press(Msg::KeyConflict(KcMsg::Close)),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        ]
        .spacing(12);
        let card = container(col)
            .padding(22)
            .width(WIDTH)
            .height(HEIGHT)
            .style(move |_| container::Style {
                background: Some(bg.into()),
                border: Border::default()
                    .rounded(super::design::RADIUS)
                    .width(1)
                    .color(Color { a: 0.12, ..fg }),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
                    offset: Vector::new(0.0, 8.0),
                    blur_radius: 22.0,
                },
                text_color: Some(fg),
                ..Default::default()
            });
        container(card).padding(SHADOW).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::{Holder, Issue};

    fn report(what: &str, line: usize) -> crate::keymap::Report {
        let acts = crate::keymap::actions(&crate::config::Keys::default());
        let toggle = acts.into_iter().find(|a| a.id == "toggle").unwrap();
        crate::keymap::Report {
            issues: vec![Issue {
                action: toggle,
                kind: Kind::Hyprland(Holder {
                    what: what.into(),
                    line: Some(line),
                    file: "ui/binds.lua".into(),
                }),
            }],
            ..Default::default()
        }
    }

    /// Un conflit répondu n'est plus demandé, même si la ligne bouge ; un
    /// autre outil sur la même combinaison l'est ; la remise à zéro efface.
    #[test]
    fn reponses_retenues() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-kc-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("hyprdesk")).unwrap();
        let cfg = dir.join("hyprdesk.lua");
        let r = report("wofi --show drun", 62);
        let found = unanswered(&r, &cfg);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].combo, "SUPER + D");
        remember(&cfg, &found[0].id);
        assert!(unanswered(&report("wofi --show drun", 70), &cfg).is_empty());
        assert_eq!(unanswered(&report("rofi -show drun", 62), &cfg).len(), 1);
        let _ = std::fs::remove_file(crate::config::keys_answers_path(&cfg));
        assert_eq!(unanswered(&r, &cfg).len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
