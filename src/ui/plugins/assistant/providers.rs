//! Fournisseurs IA (Réglages → IA → Autres services IA) : LYRA, intégrée, et
//! ceux qu'on ajoute (Anthropic, OpenAI, Z.ai, compatibles OpenAI), chacun
//! avec son nom, son endpoint, son modèle et sa clé dans le trousseau ; puis,
//! pour chaque fonction de hyprdesk, le fournisseur qu'elle utilise (LYRA
//! par défaut). Domaine : `crate::ai_services`.

use std::collections::BTreeMap;

use crate::ai_services::{self, CONSUMERS, KeySource, LYRA_ID, Provider, Service};
use crate::i18n::text;
use iced::widget::{Space, button, column, container, pick_list, row, text_input};
use iced::{Alignment, Color, Element, Length, Task};

use super::super::super::{App, Msg as UiMsg, color};
use super::{Msg, wrap};

/// Types qu'on peut ajouter (LYRA est intégrée).
const ADDABLE: [(Provider, &str); 4] = [
    (Provider::Anthropic, "Anthropic"),
    (Provider::OpenAi, "OpenAI"),
    (Provider::Zai, "Z.ai"),
    (Provider::Compatible, "Compatible OpenAI"),
];

/// Fiche d'un fournisseur en cours de modification.
#[derive(Debug, Clone)]
pub struct Draft {
    pub id: String,
    pub new: bool,
    pub kind: Provider,
    pub name: String,
    pub endpoint: String,
    pub model: String,
    pub key: String,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftField {
    Name,
    Endpoint,
    Model,
    Key,
}

#[derive(Default)]
pub struct State {
    pub draft: Option<Draft>,
    /// D'où vient la clé de chaque fournisseur (par identifiant).
    pub keys: BTreeMap<String, KeySource>,
    pub note: Option<Result<String, String>>,
    pub delete_armed: Option<String>,
}

/// Choix d'un fournisseur dans la liste déroulante.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Choice {
    id: String,
    label: String,
}

impl std::fmt::Display for Choice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

/// Relit l'origine des clés de tous les fournisseurs.
pub fn load_keys(app: &App) -> Task<UiMsg> {
    let services = app.cfg.general.ai.services.clone();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                services
                    .iter()
                    .map(|s| (s.id.clone(), ai_services::key_source(s)))
                    .collect::<Vec<_>>()
            })
            .await
            .unwrap_or_default()
        },
        |l| wrap(Msg::AiKeys(l)),
    )
}

/// Valeur enregistrée : vide quand c'est celle du type.
fn custom(v: &str, default: &str) -> String {
    let v = v.trim();
    if v == default {
        String::new()
    } else {
        v.to_string()
    }
}

impl App {
    fn providers_state(&mut self) -> Option<&mut State> {
        self.panel
            .as_mut()
            .map(|p| &mut p.assistant.services.providers)
    }

    pub(in crate::ui) fn providers_update(&mut self, msg: Msg) -> Task<UiMsg> {
        match msg {
            Msg::AiAdd(kind) => {
                let id = self.cfg.general.ai.new_id(kind);
                if let Some(s) = self.providers_state() {
                    s.note = None;
                    s.draft = Some(Draft {
                        id,
                        new: true,
                        kind,
                        name: crate::i18n::tr(kind.label()).to_string(),
                        endpoint: String::new(),
                        model: String::new(),
                        key: String::new(),
                        error: None,
                    });
                }
                Task::none()
            }
            Msg::AiEdit(id) => {
                let Some(svc) = self
                    .cfg
                    .general
                    .ai
                    .services
                    .iter()
                    .find(|s| s.id == id)
                    .cloned()
                else {
                    return Task::none();
                };
                if let Some(s) = self.providers_state() {
                    s.note = None;
                    s.draft = Some(Draft {
                        id: svc.id,
                        new: false,
                        kind: svc.kind,
                        name: svc.name,
                        endpoint: svc.endpoint,
                        model: svc.model,
                        key: String::new(),
                        error: None,
                    });
                }
                Task::none()
            }
            Msg::AiDraft(f, v) => {
                if let Some(d) = self.providers_state().and_then(|s| s.draft.as_mut()) {
                    match f {
                        DraftField::Name => d.name = v,
                        DraftField::Endpoint => d.endpoint = v,
                        DraftField::Model => d.model = v,
                        // Une clé tient sur une ligne.
                        DraftField::Key => d.key = v.trim().to_string(),
                    }
                }
                Task::none()
            }
            Msg::AiCancel => {
                if let Some(s) = self.providers_state() {
                    s.draft = None;
                }
                Task::none()
            }
            Msg::AiSave => {
                let Some(mut d) = self.providers_state().and_then(|s| s.draft.clone()) else {
                    return Task::none();
                };
                let endpoint = d.endpoint.trim();
                let error = if !endpoint.is_empty()
                    && !(endpoint.starts_with("https://") || endpoint.starts_with("http://"))
                {
                    Some(crate::i18n::tr(
                        "L'endpoint commence par https:// ou http://",
                    ))
                } else if d.model.trim().is_empty() && d.kind.default_model().is_empty() {
                    Some(crate::i18n::tr("Modèle à préciser pour ce fournisseur"))
                } else {
                    None
                };
                if let Some(e) = error {
                    d.error = Some(e.to_string());
                    if let Some(s) = self.providers_state() {
                        s.draft = Some(d);
                    }
                    return Task::none();
                }
                let svc = Service {
                    id: d.id.clone(),
                    name: d.name.trim().to_string(),
                    kind: d.kind,
                    endpoint: custom(&d.endpoint, d.kind.default_endpoint()),
                    model: custom(&d.model, d.kind.default_model()),
                };
                let mut list = self.cfg.general.ai.services.clone();
                match list.iter_mut().find(|s| s.id == svc.id) {
                    Some(s) => *s = svc,
                    None => list.push(svc),
                }
                self.settings.ai_services = Some(list);
                if let Some(s) = self.providers_state() {
                    s.draft = None;
                }
                let commit = self.commit();
                let key = std::mem::take(&mut d.key);
                let then = if key.is_empty() {
                    load_keys(self)
                } else {
                    let id = d.id.clone();
                    Task::perform(
                        async move {
                            let i = id.clone();
                            let r = tokio::task::spawn_blocking(move || {
                                ai_services::store_key(&i, &key)
                            })
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()));
                            (id, r)
                        },
                        |(_, r)| wrap(Msg::AiKeyStored(r.map(|_| true))),
                    )
                };
                Task::batch([commit, then])
            }
            Msg::AiDelete(id) => {
                let Some(s) = self.providers_state() else {
                    return Task::none();
                };
                if s.delete_armed.as_deref() != Some(id.as_str()) {
                    s.delete_armed = Some(id);
                    return Task::none();
                }
                s.delete_armed = None;
                if s.draft.as_ref().is_some_and(|d| d.id == id) {
                    s.draft = None;
                }
                let list: Vec<Service> = self
                    .cfg
                    .general
                    .ai
                    .services
                    .iter()
                    .filter(|s| s.id != id)
                    .cloned()
                    .collect();
                self.settings.ai_services = Some(list);
                // Les fonctions qui l'utilisaient reviennent à LYRA.
                for (c, used) in &self.cfg.general.ai.uses {
                    if *used == id {
                        self.settings.ai_use.insert(c.clone(), LYRA_ID.into());
                    }
                }
                let commit = self.commit();
                let clear = Task::perform(
                    async move {
                        let _ =
                            tokio::task::spawn_blocking(move || ai_services::clear_key(&id)).await;
                    },
                    |_| wrap(Msg::AiReloadKeys),
                );
                Task::batch([commit, clear])
            }
            Msg::AiKeyClear(id) => Task::perform(
                async move {
                    let i = id.clone();
                    let r = tokio::task::spawn_blocking(move || ai_services::clear_key(&i))
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()));
                    (id, r)
                },
                |(_, r)| wrap(Msg::AiKeyStored(r.map(|_| false))),
            ),
            Msg::AiKeyStored(r) => {
                if let Some(s) = self.providers_state() {
                    s.note = Some(r.map(|stored| {
                        crate::i18n::tr(if stored {
                            "Clé enregistrée dans le trousseau"
                        } else {
                            "Clé retirée du trousseau"
                        })
                        .to_string()
                    }));
                }
                load_keys(self)
            }
            Msg::AiReloadKeys => load_keys(self),
            Msg::AiKeys(l) => {
                if let Some(s) = self.providers_state() {
                    s.keys = l.into_iter().collect();
                }
                Task::none()
            }
            Msg::AiUse(consumer, id) => {
                self.settings.ai_use.insert(consumer.to_string(), id);
                self.commit()
            }
            _ => Task::none(),
        }
    }

    pub(in crate::ui) fn providers_section<'a>(&'a self, s: &'a State) -> Element<'a, UiMsg> {
        let g = &self.cfg.general;
        let th = &g.theme;
        let fg = color(th.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let small = |t: String, c: Color| -> Element<'a, UiMsg> {
            text(t).font(self.font).size(12).color(c).into()
        };
        let card = move |e: Element<'a, UiMsg>| -> Element<'a, UiMsg> {
            container(e)
                .padding(10)
                .width(Length::Fill)
                .style(move |_| container::Style {
                    border: iced::Border::default()
                        .rounded(10)
                        .width(1)
                        .color(Color { a: 0.18, ..fg }),
                    ..Default::default()
                })
                .into()
        };
        // LYRA, intégrée : l'abonnement choisi dans l'onglet Assistant.
        let sub = crate::assistant::provider(g.apps.get("ai").map(String::as_str))
            .unwrap_or("claude_code");
        let lyra_ok = crate::assistant::lyra_installed();
        let mut list = column![card(
            row![
                column![
                    text("LYRA").font(self.title_font()).size(14).color(fg),
                    small(
                        crate::tr!(
                            "Intégrée · abonnement de l'assistant : {sub}",
                            sub = sub.to_string()
                        ),
                        dim
                    ),
                    if lyra_ok {
                        small(format!("✓ {}", crate::i18n::tr("LYRA installée (hyprpanel-chat)")), fg)
                    } else {
                        small(
                            format!(
                                "• {}",
                                crate::i18n::tr(
                                    "LYRA absente : choisissez un autre fournisseur, ou installez LYRA"
                                )
                            ),
                            warn,
                        )
                    },
                ]
                .spacing(2)
                .width(Length::Fill),
            ]
            .into()
        )]
        .spacing(10);
        for svc in &g.ai.services {
            let key = match s.keys.get(&svc.id) {
                Some(KeySource::Keyring) => small(
                    format!("✓ {}", crate::i18n::tr("Clé dans le trousseau")),
                    fg,
                ),
                Some(KeySource::Env(v)) => small(
                    crate::tr!("✓ Clé lue dans la variable {v}", v = v.to_string()),
                    fg,
                ),
                Some(KeySource::Missing) if svc.kind.needs_key() => small(
                    format!(
                        "• {}",
                        crate::i18n::tr("Aucune clé : le service refusera les demandes")
                    ),
                    warn,
                ),
                Some(KeySource::Missing) => small(
                    format!(
                        "• {}",
                        crate::i18n::tr("Aucune clé (facultative pour un service local)")
                    ),
                    dim,
                ),
                None => small(crate::i18n::tr("Lecture du trousseau…").into(), dim),
            };
            let model = svc.model();
            let detail = format!(
                "{} · {} · {}",
                crate::i18n::tr(svc.kind.label()),
                if model.is_empty() {
                    "—".to_string()
                } else {
                    model
                },
                svc.endpoint()
            );
            let editing = s.draft.as_ref().filter(|d| !d.new && d.id == svc.id);
            let head = row![
                column![
                    text(svc.label()).font(self.title_font()).size(14).color(fg),
                    small(detail, dim),
                    key,
                ]
                .spacing(2)
                .width(Length::Fill),
                button(text("Modifier").font(self.font).size(12))
                    .padding([4, 10])
                    .style(button::secondary)
                    .on_press_maybe(editing.is_none().then(|| wrap(Msg::AiEdit(svc.id.clone())))),
                self.delete_button(
                    s.delete_armed.as_deref() == Some(svc.id.as_str()),
                    wrap(Msg::AiDelete(svc.id.clone())),
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center);
            let body: Element<'a, UiMsg> = match editing {
                Some(d) => column![head, self.draft_form(d, s.keys.get(&d.id))]
                    .spacing(12)
                    .into(),
                None => head.into(),
            };
            list = list.push(card(body));
        }
        if let Some(d) = s.draft.as_ref().filter(|d| d.new) {
            list = list.push(card(
                column![
                    text(crate::tr!(
                        "Nouveau fournisseur : {kind}",
                        kind = crate::i18n::tr(d.kind.label()).to_string()
                    ))
                    .font(self.title_font())
                    .size(14)
                    .color(fg),
                    self.draft_form(d, None),
                ]
                .spacing(12)
                .into(),
            ));
        }
        let add = ADDABLE.iter().fold(
            row![text("Ajouter :").font(self.font).size(13).color(dim)]
                .spacing(8)
                .align_y(Alignment::Center),
            |r, (kind, label)| {
                r.push(
                    button(text(format!("+ {label}")).font(self.font).size(12))
                        .padding([4, 10])
                        .style(button::secondary)
                        .on_press_maybe(s.draft.is_none().then(|| wrap(Msg::AiAdd(*kind)))),
                )
            },
        );
        let mut col = column![
            self.section("Fournisseurs"),
            text(
                "Chaque fonction IA de hyprdesk passe par LYRA tant que vous ne lui choisissez pas \
                 un autre fournisseur (Utilisation, ci-dessous). Les clés API restent dans le \
                 trousseau de la session, jamais dans settings.lua ni dans un export des réglages."
            )
            .font(self.font)
            .size(12)
            .color(dim),
            list,
            add,
        ]
        .spacing(12);
        match &s.note {
            Some(Ok(m)) => col = col.push(small(m.clone(), dim)),
            Some(Err(e)) => col = col.push(small(e.clone(), warn)),
            None => {}
        }
        col.into()
    }

    /// Fiche d'un fournisseur : nom, endpoint, modèle, clé.
    fn draft_form<'a>(&'a self, d: &'a Draft, key: Option<&'a KeySource>) -> Element<'a, UiMsg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let warn = Color::from_rgb8(0xff, 0x95, 0x80);
        let input = |placeholder: &str, value: &'a str, f: DraftField| {
            text_input(placeholder, value)
                .font(self.font)
                .on_input(move |v| wrap(Msg::AiDraft(f, v)))
                .on_submit(wrap(Msg::AiSave))
                .padding(8)
                .width(Length::Fill)
        };
        let has_key = key == Some(&KeySource::Keyring);
        let mut key_col = column![
            input(
                if has_key {
                    crate::i18n::tr("vide : clé inchangée")
                } else {
                    crate::i18n::tr("Coller la clé API…")
                },
                &d.key,
                DraftField::Key,
            )
            .secure(true)
        ]
        .spacing(6);
        if has_key {
            key_col = key_col.push(
                row![
                    text(format!("✓ {}", crate::i18n::tr("Clé dans le trousseau")))
                        .font(self.font)
                        .size(12)
                        .color(fg),
                    button(text("Effacer la clé").font(self.font).size(12))
                        .padding([3, 10])
                        .style(button::secondary)
                        .on_press(wrap(Msg::AiKeyClear(d.id.clone()))),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        } else if let Some(v) = d.kind.env_key() {
            key_col = key_col.push(
                text(crate::tr!(
                    "Sans clé ici, hyprdesk lit la variable {v}",
                    v = v.to_string()
                ))
                .font(self.font)
                .size(11)
                .color(dim),
            );
        }
        let mut col = column![
            self.field("Nom", "", input("", &d.name, DraftField::Name)),
            self.field(
                "Endpoint",
                "",
                input(d.kind.default_endpoint(), &d.endpoint, DraftField::Endpoint),
            ),
            self.field(
                "Modèle",
                "",
                input(
                    if d.kind.default_model().is_empty() {
                        crate::i18n::tr("nom du modèle (ex. llava, qwen2.5-vl)")
                    } else {
                        d.kind.default_model()
                    },
                    &d.model,
                    DraftField::Model,
                ),
            ),
            self.field("Clé API", "", key_col),
        ]
        .spacing(10);
        if let Some(e) = &d.error {
            col = col.push(text(e).font(self.font).size(12).color(warn));
        }
        col.push(
            row![
                button(text("Enregistrer").font(self.font).size(13))
                    .padding([6, 14])
                    .on_press(wrap(Msg::AiSave)),
                button(text("Annuler").font(self.font).size(13))
                    .padding([6, 14])
                    .style(button::secondary)
                    .on_press(wrap(Msg::AiCancel)),
                Space::new().width(Length::Fill),
                text(format!("id : {}", d.id))
                    .font(self.font)
                    .size(11)
                    .color(dim),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        )
        .into()
    }

    /// Pour chaque fonction de hyprdesk, le fournisseur qu'elle utilise.
    pub(in crate::ui) fn uses_section(&self) -> Element<'_, UiMsg> {
        let ai = &self.cfg.general.ai;
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let choices: Vec<Choice> = ai
            .all()
            .into_iter()
            .map(|s| Choice {
                label: s.label(),
                id: s.id,
            })
            .collect();
        let mut col = column![
            self.section("Utilisation"),
            text("Le fournisseur de chaque fonction ; LYRA par défaut.")
                .font(self.font)
                .size(12)
                .color(dim),
        ]
        .spacing(12);
        for c in CONSUMERS {
            let current = ai.for_consumer(c.id);
            let selected = choices.iter().find(|ch| ch.id == current.id).cloned();
            let cid = c.id;
            col = col.push(
                row![
                    column![
                        text(crate::i18n::tr(c.label))
                            .font(self.font)
                            .size(14)
                            .color(fg),
                        text(crate::i18n::tr(c.summary))
                            .font(self.font)
                            .size(11)
                            .color(dim),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    pick_list(choices.clone(), selected, move |ch: Choice| {
                        wrap(Msg::AiUse(cid, ch.id))
                    })
                    .font(self.font)
                    .text_size(13)
                    .width(220),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }
        col.into()
    }
}
