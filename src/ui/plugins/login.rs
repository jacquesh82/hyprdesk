//! Paquet intégré « Écran de connexion » (MLG-42) : remplacer le
//! gestionnaire de connexion en service (GDM…) par greetd avec
//! `hyprdesk greeter`, et revenir en arrière.
//!
//! Lecture et gestes : `crate::greeter_setup` (les gestes passent par
//! pkexec). La bascule demande une confirmation sur la page même, et ne
//! prend effet qu'au prochain démarrage.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::greeter_setup::{self, MonitorRule, Profile, Status, dm_label};
use crate::i18n::text;
use iced::widget::{Space, button, column, container, pick_list, row, toggler};
use iced::{Alignment, Color, Element, Length, Task};
use std::path::PathBuf;

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "login",
    label: "Écran de connexion",
    icon: "system-users",
    summary: "Écran de connexion de hyprdesk (greetd) à la place de GDM.",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Login(m)))
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    pub status: Status,
    /// Action en cours (pkexec ouvert).
    pub busy: Option<&'static str>,
    /// Bascule demandée, en attente de confirmation.
    pub confirm: bool,
    /// Issue de la dernière action : (réussie, message).
    pub outcome: Option<(bool, String)>,
    /// Clavier de la session (disposition, variante, options), lu dans
    /// Hyprland : repris par l'écran de connexion.
    pub keyboard: (String, String, String),
    /// Fond de l'écran (image de la bibliothèque) ; aucun : couleur du thème.
    pub wallpaper: Option<PathBuf>,
    /// Fond pour un écran de connexion vertical.
    pub wallpaper_vertical: Option<PathBuf>,
    /// Images de la bibliothèque (~/.wallpapers).
    pub images: Vec<PathBuf>,
    /// Disposition des écrans de la session, reprise par l'écran de connexion.
    pub screens: Vec<MonitorRule>,
    /// Écran qui porte la connexion (les autres restent noirs).
    pub login_monitor: String,
    /// Empreinte digitale à la connexion (appliquée à l'écriture).
    pub fingerprint: bool,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Loaded(
        Box<Status>,
        (String, String, String),
        Vec<PathBuf>,
        Vec<MonitorRule>,
    ),
    /// Fond (vertical : `true`).
    Wallpaper(bool, WallChoice),
    LoginMonitor(String),
    Fingerprint(bool),
    /// `packages`, `install`, `enable`, `restore`.
    Act(&'static str),
    Done(&'static str, Result<(), String>, Box<Status>),
    /// Bascule : demande (true) ou annulation (false) de la confirmation.
    Confirm(bool),
    Demo,
}

/// Entrée de la liste des fonds (aucun : couleur du thème).
#[derive(Debug, Clone, PartialEq)]
pub struct WallChoice(pub Option<PathBuf>);

impl std::fmt::Display for WallChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.0 {
            Some(p) => f.write_str(&p.file_name().unwrap_or_default().to_string_lossy()),
            None => f.write_str(crate::i18n::tr("Aucun (couleur du thème)")),
        }
    }
}

pub fn load() -> Task<UiMsg> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(|| {
                let i = crate::inputs::read();
                let images = super::super::list_images(&super::wallpaper::expand(
                    &super::wallpaper::default_folder(),
                ));
                let screens = crate::screens::detect()
                    .map(|l| l.iter().map(MonitorRule::from_screen).collect())
                    .unwrap_or_default();
                (
                    greeter_setup::status(),
                    (i.kb_layout, i.kb_variant, i.kb_options),
                    images,
                    screens,
                )
            })
            .await
            .unwrap_or_default()
        },
        |(s, k, l, m)| wrap(Msg::Loaded(Box::new(s), k, l, m)),
    )
}

/// Message de réussite d'une action.
fn done_label(action: &str) -> &'static str {
    match action {
        "packages" => crate::i18n::tr("greetd et cage sont installés."),
        "install" => crate::i18n::tr("Configuration de greetd écrite."),
        "enable" => crate::i18n::tr(
            "greetd est activé : l'écran de hyprdesk s'affichera au prochain démarrage.",
        ),
        _ => crate::i18n::tr("Gestionnaire d'origine rétabli pour le prochain démarrage."),
    }
}

impl App {
    /// Profil de l'écran d'après les réglages de la session.
    fn login_profile(&self, st: &State) -> Profile {
        let th = &self.cfg.general.theme;
        let c = |v: crate::config::Rgba| [v.0, v.1, v.2, v.3];
        Profile {
            layout: st.keyboard.0.clone(),
            variant: st.keyboard.1.clone(),
            options: st.keyboard.2.clone(),
            theme: th.name.clone(),
            fg: c(th.fg),
            bg: c(th.bg),
            accent: c(th.accent),
            fingerprint: st.fingerprint,
            wallpaper_source: st
                .wallpaper
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            wallpaper: String::new(),
            wallpaper_vertical_source: st
                .wallpaper_vertical
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
            wallpaper_vertical: String::new(),
            lang: std::env::var("LANG").unwrap_or_default(),
            monitors: st.screens.clone(),
            login_monitor: st.login_monitor.clone(),
        }
    }

    /// Ligne d'état : pastille verte ou orange, libellé, détail.
    fn login_check<'a>(&self, ok: bool, label: &'a str, detail: String) -> Element<'a, UiMsg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let mark = if ok {
            Color::from_rgb8(0x50, 0xfa, 0x7b)
        } else {
            Color::from_rgb8(0xff, 0xb8, 0x6c)
        };
        row![
            text(if ok { "●" } else { "○" }).size(13).color(mark),
            text(label).font(self.font).size(13).color(fg),
            text(detail).font(self.font).size(12).color(dim),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    }

    fn login_button<'a>(&self, label: &'a str, msg: Option<Msg>) -> Element<'a, UiMsg> {
        let busy = self.panel.as_ref().is_some_and(|p| p.login.busy.is_some());
        button(text(label).font(self.font).size(13))
            .padding([6, 14])
            .on_press_maybe(msg.filter(|_| !busy).map(wrap))
            .into()
    }

    pub(in crate::ui) fn login_page(&self) -> Element<'_, UiMsg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let Some(st) = self
            .panel
            .as_ref()
            .map(|p| &p.login)
            .filter(|s| s.status.loaded)
        else {
            return text("Lecture…").font(self.font).size(13).color(dim).into();
        };
        let s = &st.status;
        let small = |t: String, c: Color| -> Element<'_, UiMsg> {
            text(t).font(self.font).size(12).color(c).into()
        };

        // En service.
        let current = match s.dm.as_deref() {
            None => crate::i18n::tr("aucun : connexion en console").to_string(),
            Some("greetd.service") if s.ours => {
                crate::i18n::tr("greetd, avec l'écran de hyprdesk").to_string()
            }
            Some("greetd.service") => crate::tr!(
                "greetd, avec un autre écran ({c})",
                c = s.command.as_deref().unwrap_or("?")
            ),
            Some(u) => format!("{} ({u})", dm_label(u)),
        };
        let mut col = column![
            self.section("En service"),
            text(crate::tr!("Gestionnaire de connexion : {c}", c = current))
                .font(self.font)
                .size(13)
                .color(fg),
        ]
        .spacing(10);
        if s.dm.is_some() && !s.dm_active {
            col = col.push(small(
                crate::i18n::tr("Changement enregistré : il prendra effet au prochain démarrage.")
                    .to_string(),
                warn,
            ));
        }

        // Composants.
        col = col.push(self.section("Composants"));
        col = col.push(self.login_check(
            s.greetd,
            "greetd",
            crate::i18n::tr("démon de connexion (paquet greetd)").to_string(),
        ));
        col = col.push(self.login_check(
            s.hyprland || s.cage,
            if s.hyprland { "Hyprland" } else { "cage" },
            if s.hyprland {
                crate::i18n::tr(
                    "Hyprland réduit qui affiche l'écran de connexion (cage en secours)",
                )
                .to_string()
            } else {
                crate::i18n::tr("compositeur minimal qui affiche l'écran de connexion").to_string()
            },
        ));
        col = col.push(
            self.login_check(
                s.greeter_user,
                "greeter",
                crate::i18n::tr("utilisateur système de l'écran (créé par le paquet greetd)")
                    .to_string(),
            ),
        );
        col = col.push(
            self.login_check(
                s.binary.is_some() && !s.binary_stale,
                "hyprdesk",
                match (&s.binary, s.binary_stale) {
                    (Some(b), false) => crate::tr!("lisible par greetd : {b}", b = b),
                    (Some(b), true) => crate::tr!(
                        "{b} est plus ancien que le hyprdesk lancé : réécrire la configuration",
                        b = b
                    ),
                    (None, _) => crate::i18n::tr(
                        "absent de /usr : l'utilisateur greeter ne peut pas lire votre dossier",
                    )
                    .to_string(),
                },
            ),
        );
        if !(s.greetd && s.cage) {
            col =
                col.push(self.login_button("Installer greetd et cage", Some(Msg::Act("packages"))));
        }

        // Écran : ce que la configuration reprend de la session.
        let want = self.login_profile(st);
        col = col.push(self.section("Écran"));
        let kb = if want.layout.is_empty() {
            crate::i18n::tr("disposition par défaut (us)").to_string()
        } else if want.variant.is_empty() {
            want.layout.clone()
        } else {
            format!("{} ({})", want.layout, want.variant)
        };
        col = col.push(self.login_check(
            !want.layout.is_empty(),
            "Clavier",
            crate::tr!("{k}, repris de Réglages → Clavier et souris", k = kb),
        ));
        col = col.push(self.login_check(
            true,
            "Thème",
            crate::tr!("{t}, celui du bureau", t = want.theme),
        ));
        col = col.push(
            self.login_check(
                true,
                "Retenus",
                crate::i18n::tr(
                    "dernier identifiant et dernière session ; veille, redémarrage, extinction",
                )
                .to_string(),
            ),
        );
        // Écran qui porte la connexion : le choisi, sinon le principal
        // (Réglages → Écrans), sinon le premier ; les autres restent noirs.
        if !st.screens.is_empty() {
            let names: Vec<String> = st
                .screens
                .iter()
                .filter(|m| m.enabled)
                .map(|m| m.name.clone())
                .collect();
            let current = greeter_setup::login_monitor(&want).map(|m| m.name.clone());
            col = col.push(
                row![
                    text("Écran de la connexion").font(self.font).size(13),
                    pick_list(names, current, |n| wrap(Msg::LoginMonitor(n)))
                        .font(self.font)
                        .text_size(13)
                        .width(Length::Fill),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
            col = col.push(small(
                crate::i18n::tr("Les autres écrans restent noirs.").to_string(),
                dim,
            ));
        }
        for (vertical, label, chosen) in [
            (false, "Fond d'écran", &st.wallpaper),
            (true, "Fond (écran vertical)", &st.wallpaper_vertical),
        ] {
            let mut walls: Vec<WallChoice> = std::iter::once(WallChoice(None))
                .chain(st.images.iter().map(|p| WallChoice(Some(p.clone()))))
                .collect();
            if let Some(w) = chosen
                && !st.images.contains(w)
            {
                walls.push(WallChoice(Some(w.clone())));
            }
            col = col.push(
                row![
                    text(label).font(self.font).size(13),
                    pick_list(walls, Some(WallChoice(chosen.clone())), move |c| {
                        wrap(Msg::Wallpaper(vertical, c))
                    })
                    .font(self.font)
                    .text_size(13)
                    .width(Length::Fill),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            );
        }
        col = col.push(self.login_check(
            !want.lang.is_empty(),
            "Langue",
            if want.lang.is_empty() {
                crate::i18n::tr("aucune (LANG absent de la session)").to_string()
            } else {
                crate::tr!("{l}, celle de la session", l = want.lang)
            },
        ));
        col = col.push(
            toggler(st.fingerprint)
                .label(crate::i18n::tr(
                    "Empreinte digitale à la connexion (fprintd)",
                ))
                .on_toggle_maybe(s.fprintd.then_some(|v| wrap(Msg::Fingerprint(v))))
                .font(self.font)
                .text_size(13),
        );
        col = col.push(small(
            if s.fprintd {
                crate::i18n::tr(
                    "L'écran attend le doigt 10 s, puis demande le mot de passe. Connecté par \
                     l'empreinte, le trousseau myMFA reste verrouillé (il lui faut le mot de passe).",
                )
                .to_string()
            } else {
                crate::i18n::tr("fprintd n'est pas installé (paquet fprintd).").to_string()
            },
            dim,
        ));
        col = col.push(self.login_check(
            s.mymfa_module,
            "myMFA",
            if s.mymfa_module {
                crate::i18n::tr(
                    "trousseau myMFA déverrouillé à la connexion (pam_mymfa), à la place de gnome-keyring",
                )
                .to_string()
            } else {
                crate::i18n::tr(
                    "pam_mymfa absent : installez le client Linux de myMFA pour ouvrir le trousseau à la connexion",
                )
                .to_string()
            },
        ));
        let installed_differs = s.ours
            && !s.profile.as_ref().is_some_and(|p| p.same_settings(&want))
            || s.ours && s.mymfa_module != s.pam_mymfa;
        if installed_differs {
            col = col.push(small(
                crate::i18n::tr(
                    "L'écran installé ne suit plus ces réglages : réécrire la configuration.",
                )
                .to_string(),
                warn,
            ));
        }

        // Configuration de greetd.
        col = col.push(self.section("Configuration"));
        col = col.push(match &s.command {
            Some(c) => small(
                crate::tr!("{f} lance : {c}", f = greeter_setup::CONFIG, c = c),
                if s.ours { dim } else { warn },
            ),
            None => small(
                crate::tr!("{f} n'existe pas encore.", f = greeter_setup::CONFIG),
                dim,
            ),
        });
        col = col.push(small(
            crate::tr!(
                "« Écrire » copie hyprdesk dans {b} et fait lancer « cage -s -- hyprdesk greeter » \
                 par greetd. Un fichier d'un autre outil est gardé en config.toml.bak.",
                b = greeter_setup::SYSTEM_BIN
            ),
            dim,
        ));
        col = col.push(self.login_button(
            if s.ours && !s.binary_stale && !installed_differs {
                "Réécrire la configuration"
            } else {
                "Écrire la configuration"
            },
            s.greetd.then_some(Msg::Act("install")),
        ));

        // Bascule.
        col = col.push(self.section("Bascule"));
        if s.on_greetd() {
            let prev = s.previous.as_deref().unwrap_or("gdm.service");
            col = col.push(small(
                crate::tr!(
                    "Revenir à {d} : greetd est désactivé, {d} réactivé au prochain démarrage.",
                    d = dm_label(prev)
                ),
                dim,
            ));
            col = col.push(self.login_button(
                "Revenir au gestionnaire d'origine",
                Some(Msg::Act("restore")),
            ));
        } else if !s.ready() {
            col = col.push(small(
                crate::i18n::tr(
                    "Pour basculer, il faut greetd, cage, l'utilisateur greeter, hyprdesk dans /usr \
                     et la configuration écrite par hyprdesk.",
                )
                .to_string(),
                warn,
            ));
            col = col.push(self.login_button("Utiliser l'écran de hyprdesk…", None));
        } else if st.confirm {
            let old = s.dm.as_deref().map(dm_label).unwrap_or_default();
            col = col.push(
                container(
                    column![
                        text(crate::tr!(
                            "{d} sera désactivé et greetd activé, au prochain démarrage (la session \
                             ouverte n'est pas coupée).",
                            d = if old.is_empty() { "—".to_string() } else { old }
                        ))
                        .font(self.font)
                        .size(13)
                        .color(fg),
                        small(
                            crate::i18n::tr(
                                "Si l'écran ne s'affiche pas : Ctrl + Alt + F2, connexion en \
                                 console, puis « sudo hyprdesk greeter setup restore » et \
                                 redémarrage.",
                            )
                            .to_string(),
                            warn,
                        ),
                        row![
                            self.login_button("Confirmer la bascule", Some(Msg::Act("enable"))),
                            self.login_button("Annuler", Some(Msg::Confirm(false))),
                        ]
                        .spacing(8),
                    ]
                    .spacing(8),
                )
                .padding(12)
                .width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(Color { a: 0.08, ..warn }.into()),
                    border: iced::Border::default().rounded(10).color(warn).width(1),
                    ..Default::default()
                }),
            );
        } else {
            col = col
                .push(self.login_button("Utiliser l'écran de hyprdesk…", Some(Msg::Confirm(true))));
        }
        col = col.push(
            row![
                self.login_button("Essayer l'écran (démo)", Some(Msg::Demo)),
                small(
                    crate::i18n::tr("sans greetd : aucun mot de passe vérifié, Échap le referme.")
                        .to_string(),
                    dim,
                ),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
        );

        if st.busy.is_some() {
            col = col.push(small(
                crate::i18n::tr("En attente de l'authentification…").to_string(),
                dim,
            ));
        }
        if let Some((ok, m)) = &st.outcome {
            col = col.push(small(
                m.clone(),
                if *ok {
                    Color::from_rgb8(0x50, 0xfa, 0x7b)
                } else {
                    warn
                },
            ));
        }
        col.push(Space::new().height(8)).into()
    }

    pub(in crate::ui) fn login_update(&mut self, m: Msg) -> Task<UiMsg> {
        // Profil à installer, d'après la session (avant d'emprunter le panneau).
        let staged = match (&m, self.panel.as_ref()) {
            (Msg::Act("install"), Some(p)) => Some(self.login_profile(&p.login)),
            _ => None,
        };
        let Some(p) = self.panel.as_mut() else {
            return Task::none();
        };
        match m {
            Msg::Loaded(s, k, images, screens) => {
                // Empreinte et fond : ceux du profil installé tant qu'on n'y a
                // pas touché ; sans profil, le fond par défaut du bureau.
                if !p.login.status.loaded {
                    p.login.fingerprint = s.profile.as_ref().is_some_and(|p| p.fingerprint);
                    p.login.wallpaper_vertical = s
                        .profile
                        .as_ref()
                        .map(|pr| PathBuf::from(&pr.wallpaper_vertical_source))
                        .filter(|w| !w.as_os_str().is_empty());
                    p.login.login_monitor = s
                        .profile
                        .as_ref()
                        .map(|pr| pr.login_monitor.clone())
                        .filter(|n| !n.is_empty())
                        .or_else(|| self.cfg.general.primary.clone())
                        .unwrap_or_default();
                    p.login.wallpaper = match &s.profile {
                        Some(pr) => Some(PathBuf::from(&pr.wallpaper_source))
                            .filter(|w| !w.as_os_str().is_empty()),
                        None => self.cfg.wallpapers.iter().find_map(|w| match &w.source {
                            crate::config::WallSource::Path(path)
                                if w.sel == crate::config::Selector::default() =>
                            {
                                Some(path.clone())
                            }
                            _ => None,
                        }),
                    };
                }
                p.login.images = images;
                p.login.screens = screens;
                p.login.status = *s;
                p.login.keyboard = k;
                Task::none()
            }
            Msg::Wallpaper(vertical, WallChoice(w)) => {
                if vertical {
                    p.login.wallpaper_vertical = w;
                } else {
                    p.login.wallpaper = w;
                }
                Task::none()
            }
            Msg::LoginMonitor(n) => {
                p.login.login_monitor = n;
                Task::none()
            }
            Msg::Fingerprint(v) => {
                p.login.fingerprint = v;
                Task::none()
            }
            Msg::Confirm(v) => {
                p.login.confirm = v;
                Task::none()
            }
            Msg::Act(action) => {
                let profile = staged;
                p.login.busy = Some(action);
                p.login.outcome = None;
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            let extra = match &profile {
                                Some(pr) => match greeter_setup::stage_profile(pr) {
                                    Ok(path) => vec![path],
                                    Err(e) => return (Err(e), greeter_setup::status()),
                                },
                                None => Vec::new(),
                            };
                            let r = greeter_setup::run_privileged(action, &extra);
                            (r, greeter_setup::status())
                        })
                        .await
                        .unwrap_or_else(|e| (Err(e.to_string()), Status::default()))
                    },
                    move |(r, s)| wrap(Msg::Done(action, r, Box::new(s))),
                )
            }
            Msg::Done(action, r, s) => {
                p.login.busy = None;
                p.login.confirm = false;
                p.login.outcome = Some(match r {
                    Ok(()) => (true, done_label(action).to_string()),
                    Err(e) => (false, e),
                });
                if s.loaded {
                    p.login.status = *s;
                }
                Task::none()
            }
            Msg::Demo => {
                let exe = std::env::current_exe()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|_| "hyprdesk".into());
                let cmd = format!("{} greeter --demo", crate::launch::sh_quote(&exe));
                Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || crate::hypr::exec(&cmd)).await;
                    },
                    |_| UiMsg::Noop,
                )
            }
        }
    }
}
