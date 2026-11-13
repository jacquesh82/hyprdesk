//! Paquet intégré « Démarrage automatique » : ce qui se lance à l'ouverture
//! de la session.
//!
//! - Onglet Hyprland : les programmes que hyprdesk lance (`autostart.lua`,
//!   ajoutés ici depuis la liste des applications ou par une commande),
//!   puis ceux du bloc de démarrage de hyprland.lua, coupés ou rétablis en
//!   commentant leur ligne (`crate::session_start`).
//! - Onglet Applications (XDG) : `~/.config/autostart` et
//!   `/etc/xdg/autostart` (`crate::autostart`), lancées par hyprdesk si le
//!   réglage est actif.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use crate::session_start::Item;
use iced::widget::{Space, button, column, container, pick_list, row, text_input, toggler};
use iced::{Alignment, Border, Color, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg};
use super::{PluginInfo, PluginMsg};

/// Interrupteur d'une ligne : état voulu → message.
type Toggle = Box<dyn Fn(bool) -> UiMsg>;

pub const INFO: PluginInfo = PluginInfo {
    id: "autostart",
    label: "Démarrage automatique",
    icon: "hyprdesk-autostart",
    summary: "Programmes lancés à l'ouverture de session : par hyprdesk, par hyprland.lua, applications XDG.",
    builtin: true,
};

pub const TABS: &[super::SubTab] = &[("hyprland", "Hyprland"), ("xdg", "Applications (XDG)")];

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Autostart(m)))
}

/// Application proposée à l'ajout : (commande, nom affiché).
#[derive(Debug, Clone, PartialEq)]
pub struct AppChoice(pub String, pub String);

impl std::fmt::Display for AppChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.1)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    pub loaded: bool,
    /// Lancés par hyprdesk (`autostart.lua`).
    pub ours: Vec<Item>,
    /// Bloc de démarrage de hyprland.lua.
    pub theirs: Vec<Item>,
    /// hyprland.lua trouvé.
    pub has_hyprland: bool,
    pub apps: Vec<AppChoice>,
    /// Commande en cours de saisie.
    pub draft: String,
    pub error: Option<String>,
    /// Dernière action réussie (« Lancé : … »).
    pub info: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Loaded(Box<State>),
    Draft(String),
    Submit,
    AddApp(AppChoice),
    /// Programme de hyprdesk : coupé ou rétabli.
    ToggleOurs(usize, bool),
    RemoveOurs(usize),
    /// Ligne de hyprland.lua : coupée ou rétablie.
    ToggleTheirs(usize, bool),
    /// Ligne de hyprland.lua : supprimée.
    RemoveTheirs(usize),
    RunNow(String),
    /// Action terminée : message d'information, ou erreur ; la liste est relue.
    Done(Result<Option<String>, String>),
}

pub fn load(config_path: std::path::PathBuf) -> Task<UiMsg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || {
                let (ours, theirs) = crate::session_start::read(&config_path);
                State {
                    loaded: true,
                    ours,
                    theirs,
                    has_hyprland: crate::session_start::hyprland_path(&config_path).is_file(),
                    apps: app_choices(),
                    ..Default::default()
                }
            })
            .await
            .unwrap_or_default()
        },
        |s| wrap(Msg::Loaded(Box::new(s))),
    )
}

/// Applications installées et leur commande (ligne Exec sans codes de champ).
fn app_choices() -> Vec<AppChoice> {
    let lang = std::env::var("LC_MESSAGES")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();
    let mut out: Vec<AppChoice> = crate::launch::installed_apps()
        .into_iter()
        .filter(|a| a.available)
        .filter_map(|a| {
            let src = std::fs::read_to_string(crate::launch::find_desktop_file(&a.id)?).ok()?;
            let e = crate::launch::parse_desktop_entry(&src, &lang)?;
            let cmd = crate::launch::strip_field_codes(&e.exec);
            (!cmd.trim().is_empty()).then(|| AppChoice(cmd.trim().to_string(), a.name))
        })
        .collect();
    out.sort_by_key(|c| c.1.to_lowercase());
    out.dedup_by(|a, b| a.1 == b.1);
    out
}

/// Action hors du fil de l'interface, puis relecture.
fn act(f: impl FnOnce() -> Result<Option<String>, String> + Send + 'static) -> Task<UiMsg> {
    Task::perform(
        async move {
            tokio::task::spawn_blocking(f)
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
        },
        |r| wrap(Msg::Done(r)),
    )
}

/// Enregistre la liste de hyprdesk modifiée par `f`.
fn edit_ours(
    config_path: std::path::PathBuf,
    ours: &[Item],
    f: impl FnOnce(&mut Vec<(bool, String)>),
) -> Task<UiMsg> {
    let mut list: Vec<(bool, String)> = ours
        .iter()
        .map(|i| (i.enabled, i.command.clone()))
        .collect();
    f(&mut list);
    act(move || crate::session_start::save_ours(&config_path, &list).map(|_| None))
}

impl App {
    pub(in crate::ui) fn autostart_page(&self) -> Element<'_, UiMsg> {
        if self.on_subtab(INFO.id, "xdg") {
            return self.xdg_autostart_section();
        }
        let ink = self.ink();
        let (fg, dim, accent) = (ink.fg, ink.dim, ink.accent);
        let Some(s) = self
            .panel
            .as_ref()
            .map(|p| &p.autostart)
            .filter(|s| s.loaded)
        else {
            return text("Lecture…").font(self.font).size(13).color(dim).into();
        };
        let font = self.font;
        let small = move |t: String, c: Color| iced::widget::text(t).font(font).size(12).color(c);
        // Pastille d'état : « actif » en couleur, « coupé » en retrait.
        let chip = move |on: bool, label: &'static str| -> Element<'_, UiMsg> {
            container(
                text(label)
                    .font(font)
                    .size(11)
                    .color(if on { fg } else { dim }),
            )
            .padding([2, 8])
            .style(move |_| container::Style {
                background: on.then(|| Color { a: 0.30, ..accent }.into()),
                border: Border::default().rounded(8).width(1).color(Color {
                    a: if on { 0.9 } else { 0.4 },
                    ..if on { accent } else { dim }
                }),
                ..Default::default()
            })
            .into()
        };
        let action = move |label: &'static str, m: Msg| {
            button(text(label).font(font).size(12))
                .padding([4, 10])
                .on_press(wrap(m))
        };
        // Une entrée : interrupteur (sauf verrouillée), état, commande et détail, actions.
        let entry = |it: &Item,
                     toggle: Option<Box<dyn Fn(bool) -> UiMsg>>,
                     detail: Vec<String>,
                     actions: Vec<Element<'static, UiMsg>>|
         -> Element<'_, UiMsg> {
            let mut line = row![].spacing(10).align_y(Alignment::Center);
            line = line.push(match toggle {
                Some(f) => Element::from(toggler(it.enabled).on_toggle(f)),
                None => Space::new().width(40).into(),
            });
            line = line.push(if it.locked {
                chip(true, "toujours actif")
            } else if it.enabled {
                chip(true, "actif")
            } else {
                chip(false, "coupé")
            });
            let mut label = column![
                iced::widget::text(it.command.clone())
                    .font(font)
                    .size(13)
                    .color(if it.enabled { fg } else { dim })
            ];
            if !detail.is_empty() {
                label = label.push(small(detail.join(" — "), dim));
            }
            line = line.push(label.width(Length::Fill));
            for a in actions {
                line = line.push(a);
            }
            line.into()
        };

        let mut col = column![
            text("Programmes lancés à chaque ouverture de session Hyprland. Une modification vaut pour la prochaine connexion ; « Lancer » l'essaie tout de suite.")
                .font(self.font)
                .size(12)
                .color(dim)
        ]
        .spacing(10);

        // -- Ajouter ------------------------------------------------------------------
        col = col.push(self.section("Ajouter au démarrage"));
        col = col.push(
            self.field(
                "Une application installée",
                "",
                pick_list(s.apps.clone(), None::<AppChoice>, |c| wrap(Msg::AddApp(c)))
                    .placeholder(crate::i18n::tr("Choisir dans la liste…"))
                    .font(self.font)
                    .width(Length::Fill),
            ),
        );
        let can_add = !s.draft.trim().is_empty();
        col = col.push(
            self.field(
                "Ou une commande",
                "",
                row![
                    text_input(crate::i18n::tr("ex. nm-applet --indicator"), &s.draft)
                        .font(self.font)
                        .on_input(|v| wrap(Msg::Draft(v)))
                        .on_submit(wrap(Msg::Submit))
                        .padding(8)
                        .width(Length::Fill),
                    button(text("Ajouter").font(self.font).size(13))
                        .padding([6, 14])
                        .on_press_maybe(can_add.then(|| wrap(Msg::Submit))),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            ),
        );
        if let Some(m) = &s.info {
            col = col.push(small(m.clone(), accent));
        }
        if let Some(e) = &s.error {
            col = col.push(small(e.clone(), Color::from_rgb8(0xff, 0xb8, 0x6c)));
        }

        // -- Lancés par hyprdesk --------------------------------------------------
        col = col.push(self.section("Lancés par hyprdesk"));
        if s.ours.is_empty() {
            col = col.push(
                text("Aucun pour l'instant : ajoutez une application ou une commande ci-dessus.")
                    .font(self.font)
                    .size(12)
                    .color(dim),
            );
        }
        for (i, it) in s.ours.iter().enumerate() {
            col = col.push(entry(
                it,
                Some(Box::new(move |v| wrap(Msg::ToggleOurs(i, v)))),
                Vec::new(),
                vec![
                    action("Lancer", Msg::RunNow(it.command.clone())).into(),
                    action("Supprimer", Msg::RemoveOurs(i)).into(),
                ],
            ));
        }

        // -- hyprland.lua --------------------------------------------------------
        col = col.push(self.section("Dans hyprland.lua"));
        if !s.has_hyprland {
            col = col.push(
                text("hyprland.lua introuvable à côté de hyprdesk.lua.")
                    .font(self.font)
                    .size(12)
                    .color(dim),
            );
        } else if s.theirs.is_empty() {
            col = col.push(
                text("Aucune commande dans le bloc de démarrage (hl.on(\"hyprland.start\", …)).")
                    .font(self.font)
                    .size(12)
                    .color(dim),
            );
        } else {
            col = col.push(text("Couper une ligne la met en commentaire ; Supprimer la retire. hyprland.lua est sauvegardé avant la première modification.").font(self.font).size(12).color(dim));
        }
        for (i, it) in s.theirs.iter().enumerate() {
            let mut detail = Vec::new();
            if it.locked {
                detail.push(crate::i18n::tr("prépare la session : laissé en place").to_string());
            }
            if let Some(n) = &it.note {
                detail.push(n.clone());
            }
            let (toggle, actions): (Option<Toggle>, Vec<Element<'static, UiMsg>>) = if it.locked {
                (None, Vec::new())
            } else {
                (
                    Some(Box::new(move |v| wrap(Msg::ToggleTheirs(i, v)))),
                    vec![
                        action("Lancer", Msg::RunNow(it.command.clone())).into(),
                        action("Supprimer", Msg::RemoveTheirs(i)).into(),
                    ],
                )
            };
            col = col.push(entry(it, toggle, detail, actions));
        }
        col.into()
    }

    pub(in crate::ui) fn autostart_update(&mut self, m: Msg) -> Task<UiMsg> {
        let path = self.config_path.clone();
        let Some(p) = self.panel.as_mut() else {
            return Task::none();
        };
        let s = &mut p.autostart;
        match m {
            Msg::Loaded(n) => {
                let (error, info) = (s.error.take(), s.info.take());
                *s = *n;
                s.error = error;
                s.info = info;
                Task::none()
            }
            Msg::Draft(v) => {
                s.draft = v;
                Task::none()
            }
            Msg::Submit => {
                let cmd = std::mem::take(&mut s.draft).trim().to_string();
                if cmd.is_empty() {
                    return Task::none();
                }
                edit_ours(path, &s.ours, move |l| l.push((true, cmd)))
            }
            Msg::AddApp(c) => {
                if s.ours.iter().any(|i| i.command == c.0) {
                    s.info = Some(crate::tr!("déjà dans la liste : {n}", n = c.1));
                    return Task::none();
                }
                edit_ours(path, &s.ours, move |l| l.push((true, c.0)))
            }
            Msg::ToggleOurs(i, v) => edit_ours(path, &s.ours, move |l| {
                if let Some(e) = l.get_mut(i) {
                    e.0 = v;
                }
            }),
            Msg::RemoveOurs(i) => edit_ours(path, &s.ours, move |l| {
                if i < l.len() {
                    l.remove(i);
                }
            }),
            Msg::ToggleTheirs(i, v) => {
                let Some(it) = s.theirs.get(i).cloned() else {
                    return Task::none();
                };
                act(move || crate::session_start::set_hyprland(&path, &it, v).map(|_| None))
            }
            Msg::RemoveTheirs(i) => {
                let Some(it) = s.theirs.get(i).cloned() else {
                    return Task::none();
                };
                act(move || {
                    crate::session_start::remove_hyprland(&path, &it)
                        .map(|_| Some(crate::tr!("retiré de hyprland.lua : {c}", c = it.command)))
                })
            }
            Msg::RunNow(cmd) => act(move || {
                crate::session_start::run_now(&cmd)
                    .map(|_| Some(crate::tr!("lancé : {c}", c = cmd)))
            }),
            Msg::Done(r) => {
                match r {
                    Ok(info) => {
                        s.error = None;
                        s.info = info;
                    }
                    Err(e) => {
                        s.info = None;
                        s.error = Some(e);
                    }
                }
                load(path)
            }
        }
    }
}
