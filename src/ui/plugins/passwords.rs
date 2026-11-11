//! Paquet intégré « Mots de passe » (MLG-44) : le trousseau myMFA, client
//! Linux de myMFA et alternative à gnome-keyring. État du coffre
//! (déverrouiller, verrouiller), création ou installation de la sauvegarde
//! du web, ouverture à la connexion (pam_mymfa), reprise du Secret Service
//! tenu par gnome-keyring (import de ses secrets, puis arrêt), et retour.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use iced::widget::{button, column, container, row};
use iced::{Alignment, Color, Element, Length, Task};
use std::path::PathBuf;

use super::super::panel::PanelMsg;
use super::super::{App, Msg as UiMsg, color};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "passwords",
    label: "Mots de passe",
    icon: "system-lock-screen",
    summary: "Trousseau myMFA : codes 2FA, mots de passe, secrets des applications.",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Passwords(m)))
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    pub loaded: bool,
    /// gnome-keyring encore en service (socket systemd non masquée).
    pub gnome_keyring: bool,
    /// Lignes pam_mymfa posées dans /etc/pam.d/greetd.
    pub pam_mymfa: bool,
    pub mymfa_module: bool,
    /// Sauvegarde du web trouvée dans Téléchargements.
    pub backup: Option<PathBuf>,
    /// Reprise de gnome-keyring demandée, à confirmer.
    pub confirm: bool,
    pub busy: bool,
    pub outcome: Option<(bool, String)>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Loaded(Box<State>),
    Unlock,
    Lock,
    /// Ouvre un terminal sur `mymfa init` (avec la sauvegarde : `--from`).
    Init(Option<PathBuf>),
    Confirm(bool),
    TakeOver,
    GiveBack,
    Done(Result<String, String>),
    /// Lanceur ouvert en mode `:2fa` ou `:mdp`.
    Search(&'static str),
    LoginScreen,
}

fn read() -> State {
    let g = crate::greeter_setup::status();
    State {
        loaded: true,
        gnome_keyring: crate::mymfa::gnome_keyring_in_use(),
        pam_mymfa: g.pam_mymfa,
        mymfa_module: g.mymfa_module,
        backup: crate::mymfa::find_backup(),
        ..Default::default()
    }
}

pub fn load() -> Task<UiMsg> {
    Task::batch([
        Task::perform(
            async { tokio::task::spawn_blocking(read).await.unwrap_or_default() },
            |s| wrap(Msg::Loaded(Box::new(s))),
        ),
        Task::done(UiMsg::Mymfa(super::super::mymfa::MMsg::Refresh)),
    ])
}

impl App {
    fn pw_check<'a>(&self, ok: bool, label: &'a str, detail: String) -> Element<'a, UiMsg> {
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

    fn pw_button<'a>(&self, label: &'a str, msg: Option<Msg>, busy: bool) -> Element<'a, UiMsg> {
        button(text(label).font(self.font).size(13))
            .padding([6, 14])
            .on_press_maybe(msg.filter(|_| !busy).map(wrap))
            .into()
    }

    pub(in crate::ui) fn passwords_page(&self) -> Element<'_, UiMsg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let warn = Color::from_rgb8(0xff, 0xb8, 0x6c);
        let Some(st) = self
            .panel
            .as_ref()
            .map(|p| &p.passwords)
            .filter(|s| s.loaded)
        else {
            return text("Lecture…").font(self.font).size(13).color(dim).into();
        };
        let m = &self.mfa;
        let small = |t: String, c: Color| -> Element<'_, UiMsg> {
            text(t).font(self.font).size(12).color(c).into()
        };
        let mut col = column![self.section("Coffre myMFA")].spacing(10);
        col = col.push(self.pw_check(
            m.installed,
            "myMFA",
            if m.installed {
                crate::i18n::tr("client Linux installé (démon mymfa)").to_string()
            } else {
                crate::i18n::tr(
                    "client Linux absent : mindlog.mymfa.clients, linux/scripts/installer.sh",
                )
                .to_string()
            },
        ));
        if !m.installed {
            return col.into();
        }
        if !m.vault {
            col = col.push(self.pw_check(
                false,
                "Coffre",
                crate::i18n::tr("pas encore de coffre sur cet ordinateur").to_string(),
            ));
            let mut actions =
                row![self.pw_button("Créer un coffre…", Some(Msg::Init(None)), st.busy)].spacing(8);
            if let Some(b) = &st.backup {
                actions = actions.push(self.pw_button(
                    "Installer la sauvegarde du web…",
                    Some(Msg::Init(Some(b.clone()))),
                    st.busy,
                ));
            }
            col = col.push(actions);
            col = col.push(small(
                match &st.backup {
                    Some(b) => crate::tr!(
                        "Sauvegarde trouvée : {f}. Le mot de passe du coffre est celui du web.",
                        f = b.display()
                    ),
                    None => crate::i18n::tr(
                        "Pour reprendre le coffre du web : exportez mymfa-vault.json \
                         (app.mymfa.mindlog.today → Réglages → Sauvegarde) dans Téléchargements.",
                    )
                    .to_string(),
                },
                dim,
            ));
        } else {
            col = col.push(self.pw_check(
                !m.locked,
                "Coffre",
                if m.locked {
                    crate::i18n::tr("verrouillé").to_string()
                } else {
                    crate::i18n::tr("ouvert : codes, mots de passe et secrets disponibles")
                        .to_string()
                },
            ));
            col = col.push(
                row![
                    if m.locked {
                        self.pw_button("Déverrouiller…", Some(Msg::Unlock), st.busy)
                    } else {
                        self.pw_button("Verrouiller", Some(Msg::Lock), st.busy)
                    },
                    self.pw_button("Codes 2FA (:2fa)", Some(Msg::Search(":2fa ")), st.busy),
                    self.pw_button("Mots de passe (:mdp)", Some(Msg::Search(":mdp ")), st.busy),
                ]
                .spacing(8),
            );
        }

        // Ouverture à la connexion.
        col = col.push(self.section("Connexion"));
        col = col.push(self.pw_check(
            st.pam_mymfa,
            "pam_mymfa",
            if st.pam_mymfa {
                crate::i18n::tr("l'écran de connexion de hyprdesk ouvre le coffre").to_string()
            } else if st.mymfa_module {
                crate::i18n::tr(
                    "module installé, pas encore dans l'écran de connexion : réécrire sa configuration",
                )
                .to_string()
            } else {
                crate::i18n::tr("module pam_mymfa non installé (en root : voir le README du client)")
                    .to_string()
            },
        ));
        col = col.push(self.pw_check(
            m.sealed,
            "Sceau",
            if m.sealed {
                crate::i18n::tr("le coffre s'ouvre seul à la connexion").to_string()
            } else {
                crate::i18n::tr(
                    "posé au premier déverrouillage après une connexion par l'écran de hyprdesk",
                )
                .to_string()
            },
        ));
        if st.mymfa_module && !st.pam_mymfa {
            col = col.push(self.pw_button("Écran de connexion…", Some(Msg::LoginScreen), st.busy));
        }

        // Secret Service des applications.
        col = col.push(self.section("Trousseau des applications"));
        col = col.push(self.pw_check(
            m.secret_service,
            "Secret Service",
            if m.secret_service {
                crate::i18n::tr("tenu par myMFA : les applications y rangent leurs secrets")
                    .to_string()
            } else if st.gnome_keyring {
                crate::i18n::tr("tenu par gnome-keyring ; myMFA attend en file").to_string()
            } else {
                crate::i18n::tr("tenu par un autre programme ; myMFA attend en file").to_string()
            },
        ));
        if !m.secret_service && st.gnome_keyring {
            if !m.vault || m.locked {
                col = col.push(small(
                    crate::i18n::tr(
                        "Pour reprendre gnome-keyring, le coffre myMFA doit exister et être ouvert.",
                    )
                    .to_string(),
                    warn,
                ));
            } else if st.confirm {
                col = col.push(
                    container(
                        column![
                            text(crate::i18n::tr(
                                "Les secrets de gnome-keyring sont copiés dans le trousseau myMFA, \
                                 puis gnome-keyring est arrêté et masqué : myMFA prend sa place.",
                            ))
                            .font(self.font)
                            .size(13)
                            .color(fg),
                            small(
                                crate::i18n::tr(
                                    "gnome-keyring peut demander son mot de passe pendant l'import. \
                                     Ses fichiers (~/.local/share/keyrings) ne sont pas touchés : \
                                     le retour reste possible.",
                                )
                                .to_string(),
                                dim,
                            ),
                            row![
                                self.pw_button(
                                    "Reprendre gnome-keyring",
                                    Some(Msg::TakeOver),
                                    st.busy
                                ),
                                self.pw_button("Annuler", Some(Msg::Confirm(false)), st.busy),
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
                col = col.push(self.pw_button(
                    "Remplacer gnome-keyring par myMFA…",
                    Some(Msg::Confirm(true)),
                    st.busy,
                ));
            }
        } else if m.secret_service && !st.gnome_keyring {
            col = col.push(self.pw_button("Revenir à gnome-keyring", Some(Msg::GiveBack), st.busy));
        }

        if st.busy {
            col = col.push(small(crate::i18n::tr("En cours…").to_string(), dim));
        }
        if let Some((ok, msg)) = &st.outcome {
            col = col.push(small(
                msg.clone(),
                if *ok {
                    Color::from_rgb8(0x50, 0xfa, 0x7b)
                } else {
                    warn
                },
            ));
        }
        col.into()
    }

    pub(in crate::ui) fn passwords_update(&mut self, m: Msg) -> Task<UiMsg> {
        use super::super::mymfa::MMsg;
        match m {
            Msg::Loaded(s) => {
                if let Some(p) = self.panel.as_mut() {
                    let keep = (p.passwords.confirm, p.passwords.outcome.take());
                    p.passwords = *s;
                    (p.passwords.confirm, p.passwords.outcome) = keep;
                }
                Task::none()
            }
            Msg::Unlock => self.mfa_update(MMsg::Ask),
            Msg::Lock => self.mfa_update(MMsg::Lock),
            Msg::Init(from) => {
                let cmd = match from {
                    Some(f) => format!(
                        "mymfa init --from {}",
                        crate::launch::sh_quote(&f.to_string_lossy())
                    ),
                    None => "mymfa init".to_string(),
                };
                // Le terminal reste ouvert sur le résultat, le temps de le lire.
                let line = format!("{cmd}; echo; read -rp \"Entrée pour fermer \" _");
                let full = crate::launch::in_terminal(
                    &self.cfg.general.terminal,
                    &format!("sh -c {}", crate::launch::sh_quote(&line)),
                );
                Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || crate::hypr::exec(&full)).await;
                    },
                    |_| UiMsg::Noop,
                )
            }
            Msg::Confirm(v) => {
                if let Some(p) = self.panel.as_mut() {
                    p.passwords.confirm = v;
                }
                Task::none()
            }
            Msg::TakeOver | Msg::GiveBack => {
                let take = matches!(m, Msg::TakeOver);
                if let Some(p) = self.panel.as_mut() {
                    p.passwords.busy = true;
                    p.passwords.confirm = false;
                    p.passwords.outcome = None;
                }
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            if take {
                                crate::mymfa::take_over_gnome_keyring().map(|()| {
                                    crate::i18n::tr(
                                        "myMFA tient le trousseau ; réécrivez la configuration de \
                                         l'écran de connexion pour retirer gnome-keyring.",
                                    )
                                    .to_string()
                                })
                            } else {
                                crate::mymfa::give_back_to_gnome_keyring().map(|()| {
                                    crate::i18n::tr("gnome-keyring est rétabli.").to_string()
                                })
                            }
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| wrap(Msg::Done(r)),
                )
            }
            Msg::Done(r) => {
                if let Some(p) = self.panel.as_mut() {
                    p.passwords.busy = false;
                    p.passwords.outcome = Some(match r {
                        Ok(m) => (true, m),
                        Err(e) => (false, e),
                    });
                }
                load()
            }
            Msg::Search(prefix) => {
                let close = self.close_panel();
                Task::batch([close, self.open_search_with(prefix)])
            }
            Msg::LoginScreen => self.panel_update(PanelMsg::Tab("login")),
        }
    }
}
