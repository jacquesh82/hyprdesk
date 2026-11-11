//! Fenêtre myMFA (MLG-46) : codes 2FA et réglages (dont la synchronisation
//! Drive, MLG-47), au style de hyprdesk, dans une fenêtre layer-shell centrée.
//!
//! Tout passe par le démon `today.mindlog.MyMfa1` : les codes en cours de
//! `Codes` (clic = copie par le démon lui-même), le contenu de `Content` (sans
//! aucun secret), les modifications d'`Apply` (mêmes règles que le web), la
//! synchronisation de `SyncNow`/`SyncStatus`/`SyncJournal`/`DriveConnect`/
//! `DriveDisconnect`. Aucun secret ne transite ni ne reste dans hyprdesk.
//!
//! La fenêtre se rafraîchit toute seule : codes chaque seconde, contenu et
//! statut de synchro une fois sur cinq. Verrouillé ou fermé, la boucle s'arrête
//! d'elle-même. MLG-46 : les onglets Mots de passe (liste, détail, générateur,
//! édition), Activité et le Scanner d'QR (image, capture écran) viennent dans
//! les tranches suivantes.

use iced::widget::{button, column, container, row, scrollable, text, text_input};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Task};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};
use serde_json::Value;

use super::{App, Msg, color};
use crate::mymfa;

pub const NAMESPACE: &str = "hyprdesk-mymfa-fenetre";
const WIDTH: f32 = 860.0;
const HEIGHT: f32 = 600.0;
const MARGE: f32 = 18.0;

/// Onglet affiché.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Onglet {
    Codes,
    Reglages,
}

impl Onglet {
    fn titre(self) -> String {
        crate::i18n::tr(match self {
            Onglet::Codes => "Codes 2FA",
            Onglet::Reglages => "Réglages",
        })
        .to_string()
    }
    const TOUS: [Onglet; 2] = [Onglet::Codes, Onglet::Reglages];
}

/// Filtre de la liste des codes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Filtre {
    Tous,
    Favoris,
    Etiquette(String),
}

/// Formulaire d'un nouveau compte 2FA (URI otpauth ou clé base32).
#[derive(Debug, Clone, Default)]
pub struct NouveauCompte {
    pub ouvert: bool,
    pub texte: String,
    pub tag: String,
    pub description: String,
    /// Réponse de `Preview` : champs lus et code actuel.
    pub apercu: Option<Value>,
    /// Refus du démon (clé invalide, URI incompréhensible…).
    pub erreur: Option<String>,
}

/// Messages de la fenêtre.
#[derive(Debug, Clone)]
pub enum FMsg {
    Fermer,
    Onglet(Onglet),
    Tic,
    /// Codes en cours (`Codes` du démon) : 8-tuples.
    Codes(Result<Vec<crate::mymfa::CodeComplet>, String>),
    /// Contenu complet (`Content`).
    Contenu(Result<Value, String>),
    Recherche(String),
    Filtre(Filtre),
    Masquer(bool),
    CopierCode(String),
    HotpSuivant(String),
    Favori(String, bool),
    SupprimerCompte(String, String),
    NouveauCompte(bool),
    NouveauChamp(&'static str, String),
    VerifierCompte,
    ApercuCompte(Result<Value, String>),
    AjouterCompte,
    /// Réglages et synchro Drive (MLG-47).
    Verrouiller,
    StatutSynchro(Result<Value, String>),
    Journal(Result<Value, String>),
    SyncNow,
    ConnecterDrive,
    DeconnecterDrive,
    /// Message affiché en bas de fenêtre (erreur ?).
    Message(bool, String),
}

/// Fenêtre ouverte.
pub struct Fenetre {
    pub win: Id,
    pub onglet: Onglet,
    /// Contenu complet (`Content`), sans aucun secret.
    pub contenu: Option<Value>,
    /// Codes en cours (`Codes` du démon).
    pub codes: Vec<CodeVue>,
    pub recherche: String,
    pub filtre: Filtre,
    /// Codes masqués (l'état, cohérent dans la fenêtre).
    pub masque: bool,
    /// Nouveau compte 2FA (formulaire du code, pas encore du scanner d'image).
    pub nouveau: NouveauCompte,
    pub statut_synchro: Option<Value>,
    pub journal: Option<Value>,
    pub message: Option<(bool, String)>,
    pub occupe: bool,
    pub tiques: u32,
}

/// Code affiché (8-tuple de `Codes`, tous les champs cette fois).
#[derive(Debug, Clone)]
pub struct CodeVue {
    pub id: String,
    pub issuer: String,
    pub account: String,
    pub tag: String,
    pub code: String,
    pub remaining: u32,
    pub period: u32,
    pub favori: bool,
}

impl CodeVue {
    fn label(&self) -> String {
        match (self.issuer.is_empty(), self.account.is_empty()) {
            (false, false) => format!("{} — {}", self.issuer, self.account),
            (false, true) => self.issuer.clone(),
            _ => self.account.clone(),
        }
    }
}

impl Fenetre {
    /// Fenêtre d'aperçu : codes et contenu factices (scène de preview).
    pub fn apercu(win: Id) -> Self {
        let mut f = Self::neuf(win);
        f.contenu = Some(contenu_apercu());
        f.codes = vec![
            CodeVue {
                id: "1".into(),
                issuer: "GitHub".into(),
                account: "jacques".into(),
                tag: "travail".into(),
                code: "482 913".into(),
                remaining: 18,
                period: 30,
                favori: true,
            },
            CodeVue {
                id: "2".into(),
                issuer: "OVH".into(),
                account: "moi@example.net".into(),
                tag: String::new(),
                code: "035 771".into(),
                remaining: 7,
                period: 30,
                favori: false,
            },
            CodeVue {
                id: "3".into(),
                issuer: "Banque".into(),
                account: "principal".into(),
                tag: String::new(),
                code: "640 228".into(),
                remaining: 25,
                period: 60,
                favori: false,
            },
        ];
        f.statut_synchro = Some(serde_json::json!({
            "connecte": true, "coffreOuvert": true, "enCours": false,
            "horlogeVerifiee": true, "revision": "0000000000000000000001",
            "modifieeLe": "2026-11-11T14:05:00.000Z",
        }));
        f
    }

    fn neuf(win: Id) -> Self {
        Self {
            win,
            onglet: Onglet::Codes,
            contenu: None,
            codes: Vec::new(),
            recherche: String::new(),
            filtre: Filtre::Tous,
            masque: false,
            nouveau: NouveauCompte::default(),
            statut_synchro: None,
            journal: None,
            message: None,
            occupe: false,
            tiques: 0,
        }
    }

    fn codes_visibles(&self) -> Vec<CodeVue> {
        let q = self.recherche.to_lowercase();
        self.codes
            .iter()
            .filter(|c| match &self.filtre {
                Filtre::Tous => true,
                Filtre::Favoris => c.favori,
                Filtre::Etiquette(t) => &c.tag == t,
            })
            .filter(|c| {
                q.is_empty()
                    || c.issuer.to_lowercase().contains(&q)
                    || c.account.to_lowercase().contains(&q)
                    || c.tag.to_lowercase().contains(&q)
            })
            .cloned()
            .collect()
    }

    fn etiquettes(&self) -> Vec<String> {
        let mut t: Vec<String> = self
            .codes
            .iter()
            .map(|c| c.tag.clone())
            .filter(|t| !t.is_empty())
            .collect();
        t.sort();
        t.dedup();
        t
    }
}

/// Contenu factice de la scène d'aperçu (sans secret, comme le vrai).
fn contenu_apercu() -> Value {
    serde_json::json!({
        "accounts": [
            { "id": "1", "type": "totp", "issuer": "GitHub", "account": "jacques", "tag": "travail", "favorite": true },
            { "id": "2", "type": "totp", "issuer": "OVH", "account": "moi@example.net", "tag": "", "favorite": false },
        ],
        "tags": ["travail"],
    })
}

impl App {
    /// Ouvre la fenêtre myMFA (`hyprdesk mymfa open`, lanceur `:mymfa`,
    /// Réglages → Mots de passe). Coffre verrouillé : la carte d'abord (MLG-45).
    pub fn mfa_fenetre_open(&mut self) -> Task<Msg> {
        if !self.mfa.installed {
            return Task::none();
        }
        if self.mfa.vault && self.mfa.locked {
            return self.mfa_update(crate::ui::mymfa::MMsg::Ask);
        }
        if self.mfa_fenetre.is_some() {
            return Task::none();
        }
        let win = Id::unique();
        self.mfa_fenetre = Some(Fenetre::neuf(win));
        Task::batch([
            Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some(((WIDTH + 2.0 * MARGE) as u32, (HEIGHT + 2.0 * MARGE) as u32)),
                    layer: Layer::Overlay,
                    anchor: LsAnchor::empty(),
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::Exclusive,
                    output_option: OutputOption::Active,
                    namespace: Some(NAMESPACE.into()),
                    ..Default::default()
                },
                id: win,
            }),
            Task::done(Msg::MymfaF(FMsg::Tic)),
        ])
    }

    pub fn mfa_fenetre_close(&mut self) -> Task<Msg> {
        self.mfa_fenetre = None;
        Task::none()
    }

    pub fn view_mfa_fenetre(&self) -> Element<'_, Msg> {
        match &self.mfa_fenetre {
            Some(f) => f.view(self),
            None => iced::widget::Space::new().into(),
        }
    }

    pub fn mfa_fenetre_update(&mut self, m: FMsg) -> Task<Msg> {
        let Some(f) = self.mfa_fenetre.as_mut() else {
            return Task::none();
        };
        match m {
            FMsg::Fermer => self.mfa_fenetre_close(),
            FMsg::Onglet(o) => {
                f.onglet = o;
                f.nouveau.ouvert = false;
                Task::none()
            }
            FMsg::Tic => {
                f.tiques += 1;
                let tic = Task::perform(
                    tokio::time::sleep(std::time::Duration::from_secs(1)),
                    |()| Msg::MymfaF(FMsg::Tic),
                );
                let codes = Task::perform(mymfa::codes_pleines(), |r| Msg::MymfaF(FMsg::Codes(r)));
                let contenu = if f.tiques % 5 == 1 {
                    Task::perform(mymfa::content(), |r| Msg::MymfaF(FMsg::Contenu(r))).chain(
                        Task::perform(mymfa::sync_status(), |r| {
                            Msg::MymfaF(FMsg::StatutSynchro(r))
                        }),
                    )
                } else {
                    Task::none()
                };
                Task::batch([tic, codes, contenu])
            }
            FMsg::Codes(r) => {
                if let Ok(liste) = r {
                    f.codes = liste
                        .into_iter()
                        .map(
                            |(id, issuer, account, tag, code, remaining, period, favori)| CodeVue {
                                id,
                                issuer,
                                account,
                                tag,
                                code,
                                remaining,
                                period,
                                favori,
                            },
                        )
                        .collect();
                }
                Task::none()
            }
            FMsg::Contenu(r) => match r {
                Ok(c) => {
                    f.contenu = Some(c);
                    Task::none()
                }
                Err(_e) => {
                    // Coffre verrouillé entre-temps : la carte prend le relais.
                    Task::batch([
                        self.mfa_fenetre_close(),
                        self.mfa_update(crate::ui::mymfa::MMsg::Ask),
                    ])
                }
            },
            FMsg::Recherche(s) => {
                f.recherche = s;
                Task::none()
            }
            FMsg::Filtre(fil) => {
                f.filtre = if f.filtre == fil { Filtre::Tous } else { fil };
                Task::none()
            }
            FMsg::Masquer(v) => {
                f.masque = v;
                Task::none()
            }
            FMsg::CopierCode(id) => Task::perform(mymfa::copy_code(id), |r| {
                Msg::MymfaF(message(r.map(|()| tr_("Code copié."))))
            }),
            FMsg::HotpSuivant(id) => Task::perform(
                {
                    let id = id.clone();
                    async move { mymfa::apply(serde_json::json!({ "op": "next_hotp", "id": id })).await }
                },
                |r| {
                    Msg::MymfaF(message(r.map(|v| {
                        crate::tr!(
                            "Code HOTP suivant copié : {code}",
                            code = v.get("code").and_then(Value::as_str).unwrap_or("?")
                        )
                        .to_string()
                    })))
                },
            ),
            FMsg::Favori(id, v) => Task::perform(
                {
                    let id = id.clone();
                    async move {
                        mymfa::apply(serde_json::json!({
                            "op": "update_account", "id": id, "patch": { "favorite": v }
                        }))
                        .await
                    }
                },
                |r| Msg::MymfaF(message(r.map(|_| tr_("Modifié.").to_string()))),
            )
            .chain(Task::perform(mymfa::content(), |r| {
                Msg::MymfaF(FMsg::Contenu(r))
            })),
            FMsg::SupprimerCompte(id, label) => Task::perform(
                {
                    let id = id.clone();
                    async move {
                        mymfa::apply(serde_json::json!({ "op": "remove_account", "id": id })).await
                    }
                },
                move |r| {
                    Msg::MymfaF(message(r.map(|_| {
                        crate::tr!("« {label} » supprimé.", label = label).to_string()
                    })))
                },
            )
            .chain(
                Task::perform(mymfa::content(), |r| Msg::MymfaF(FMsg::Contenu(r))).chain(
                    Task::perform(mymfa::codes_pleines(), |r| Msg::MymfaF(FMsg::Codes(r))),
                ),
            ),
            FMsg::NouveauCompte(ouvert) => {
                f.nouveau = NouveauCompte {
                    ouvert,
                    ..Default::default()
                };
                Task::none()
            }
            FMsg::NouveauChamp(champ, v) => {
                match champ {
                    "texte" => f.nouveau.texte = v,
                    "tag" => f.nouveau.tag = v,
                    "description" => f.nouveau.description = v,
                    _ => {}
                }
                f.nouveau.apercu = None;
                Task::none()
            }
            FMsg::VerifierCompte => {
                if f.nouveau.texte.trim().is_empty() {
                    return Task::none();
                }
                let texte = f.nouveau.texte.trim().to_string();
                let (uri, secret) = if texte.starts_with("otpauth://") {
                    (texte, String::new())
                } else {
                    (String::new(), texte)
                };
                Task::perform(
                    async move {
                        mymfa::preview(serde_json::json!({ "uri": uri, "secret": secret })).await
                    },
                    |r| Msg::MymfaF(FMsg::ApercuCompte(r)),
                )
            }
            FMsg::ApercuCompte(r) => {
                match r {
                    Ok(apercu) => {
                        f.nouveau.apercu = Some(apercu);
                        f.nouveau.erreur = None;
                    }
                    Err(e) => {
                        f.nouveau.apercu = None;
                        f.nouveau.erreur = Some(e);
                    }
                }
                Task::none()
            }
            FMsg::AjouterCompte => {
                if f.occupe {
                    return Task::none();
                }
                f.occupe = true;
                let texte = f.nouveau.texte.trim().to_string();
                let (uri, secret) = if texte.starts_with("otpauth://") {
                    (texte, String::new())
                } else {
                    (String::new(), texte)
                };
                let (tag, description) = (f.nouveau.tag.clone(), f.nouveau.description.clone());
                Task::perform(
                    async move {
                        mymfa::apply(serde_json::json!({
                            "op": "add_account",
                            "uri": uri, "secret": secret,
                            "tag": tag, "description": description,
                        }))
                        .await
                    },
                    |r| {
                        Msg::MymfaF(message(
                            r.map(|_| crate::i18n::tr("Compte ajouté.").to_string()),
                        ))
                    },
                )
                .chain(
                    Task::perform(mymfa::content(), |r| Msg::MymfaF(FMsg::Contenu(r))).chain(
                        Task::perform(mymfa::codes_pleines(), |r| Msg::MymfaF(FMsg::Codes(r))),
                    ),
                )
            }
            FMsg::Verrouiller => Task::perform(mymfa::lock(), |r| match r {
                Ok(()) => Msg::MymfaF(FMsg::Fermer),
                Err(e) => Msg::MymfaF(FMsg::Message(true, e)),
            }),
            FMsg::StatutSynchro(r) => {
                if let Ok(v) = r {
                    f.statut_synchro = Some(v);
                }
                Task::none()
            }
            FMsg::Journal(r) => {
                if let Ok(j) = r {
                    f.journal = Some(j);
                }
                Task::none()
            }
            FMsg::SyncNow => {
                if f.occupe {
                    return Task::none();
                }
                f.occupe = true;
                Task::perform(mymfa::sync_now(), |r| match r {
                    Ok(v) => Msg::MymfaF(FMsg::Message(
                        false,
                        if v.get("envoye").and_then(Value::as_bool).unwrap_or(false) {
                            crate::i18n::tr("Synchronisation Drive effectuée.").to_string()
                        } else {
                            crate::i18n::tr("Rien à envoyer : Drive est déjà à jour.").to_string()
                        },
                    )),
                    Err(e) => Msg::MymfaF(FMsg::Message(true, e)),
                })
                .chain(
                    Task::perform(mymfa::sync_status(), |r| {
                        Msg::MymfaF(FMsg::StatutSynchro(r))
                    })
                    .chain(Task::perform(mymfa::sync_journal(), |r| {
                        Msg::MymfaF(FMsg::Journal(r))
                    })),
                )
            }
            FMsg::ConnecterDrive => {
                if f.occupe {
                    return Task::none();
                }
                f.occupe = true;
                Task::perform(mymfa::drive_connect(), |r| match r {
                    Ok(_) => Msg::MymfaF(FMsg::Message(
                        false,
                        crate::i18n::tr("Google Drive connecté.").to_string(),
                    )),
                    Err(e) => Msg::MymfaF(FMsg::Message(true, e)),
                })
                .chain(
                    Task::perform(mymfa::sync_status(), |r| {
                        Msg::MymfaF(FMsg::StatutSynchro(r))
                    })
                    .chain(Task::perform(mymfa::sync_journal(), |r| {
                        Msg::MymfaF(FMsg::Journal(r))
                    })),
                )
            }
            FMsg::DeconnecterDrive => Task::perform(mymfa::drive_disconnect(), |r| match r {
                Ok(()) => Msg::MymfaF(FMsg::StatutSynchro(Ok(serde_json::json!({
                    "connecte": false,
                })))),
                Err(e) => Msg::MymfaF(FMsg::Message(true, e)),
            }),
            FMsg::Message(erreur, texte) => {
                f.message = Some((erreur, texte));
                f.occupe = false;
                Task::none()
            }
        }
    }
}

/// Le message d'une action réussie (`Ok(texte)`) ou d'une erreur (`Err(texte)`).
fn message(r: Result<String, String>) -> FMsg {
    match r {
        Ok(texte) => FMsg::Message(false, texte),
        Err(e) => FMsg::Message(true, e),
    }
}

fn tr_(texte: &str) -> String {
    crate::i18n::tr(texte).to_string()
}

/* ───── Vue ──────────────────────────────────────────────────────────────── */

impl Fenetre {
    fn view(&self, app: &App) -> Element<'_, Msg> {
        let th = &app.cfg.general.theme;
        let r = crate::theme::readable(th.fg, th.bg, th.accent);
        let fg = color(r.fg, 1.0);
        let dim = color(r.dim, 1.0);
        let edge = color(r.edge, 1.0);
        let btn = color(r.button, 1.0);
        let on_btn = color(r.on_button, 1.0);
        let error = color(r.error, 1.0);
        let bg = color(th.bg, 1.0);
        let wrap = Msg::MymfaF;

        let titre = text(crate::i18n::tr("myMFA"))
            .font(app.title_font())
            .size(20)
            .color(fg);
        let onglets = row(Onglet::TOUS.iter().map(|o| {
            let actif = self.onglet == *o;
            Element::from(
                button(text(o.titre()).font(app.font).size(14).color(if actif {
                    on_btn
                } else {
                    fg
                }))
                .padding([8, 14])
                .style(move |_, st| button::Style {
                    background: Some(
                        if actif {
                            btn
                        } else {
                            match st {
                                button::Status::Hovered | button::Status::Pressed => {
                                    Color { a: 0.12, ..fg }
                                }
                                _ => Color { a: 0.05, ..fg },
                            }
                        }
                        .into(),
                    ),
                    text_color: if actif { on_btn } else { fg },
                    border: Border::default().rounded(8).width(1).color(if actif {
                        btn
                    } else {
                        edge
                    }),
                    ..Default::default()
                })
                .on_press(wrap(FMsg::Onglet(*o))),
            )
        }))
        .spacing(6);
        let fermer = Self::bouton_doux(app, crate::i18n::tr("Fermer"), FMsg::Fermer);

        let corps = match self.onglet {
            Onglet::Codes => self.vue_codes(app),
            Onglet::Reglages => self.vue_reglages(app),
        };

        let message: Option<Element<'_, Msg>> = self.message.as_ref().map(|(erreur, t)| {
            text(t.clone())
                .font(app.font)
                .size(13)
                .color(if *erreur { error } else { dim })
                .into()
        });
        let verrouiller = Self::bouton_doux(app, crate::i18n::tr("Verrouiller"), FMsg::Verrouiller);

        let cadre = column![
            container(
                row![
                    titre,
                    iced::widget::Space::new().width(Length::Fill),
                    onglets,
                    fermer
                ]
                .spacing(10)
                .align_y(Alignment::Center)
            )
            .width(Length::Fill),
            container(
                scrollable(container(corps).width(Length::Fill).padding([12, 4]))
                    .height(Length::Fill)
            )
            .height(Length::Fill)
            .width(Length::Fill),
            container(
                row![
                    message.unwrap_or_else(|| iced::widget::Space::new().into()),
                    iced::widget::Space::new().width(Length::Fill),
                    verrouiller,
                ]
                .spacing(10)
                .align_y(Alignment::Center)
            )
            .width(Length::Fill),
        ]
        .spacing(8)
        .width(Length::Fill)
        .height(Length::Fill);

        container(cadre)
            .padding(MARGE)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(bg.into()),
                text_color: Some(fg),
                border: Border::default().rounded(14).width(1).color(edge),
                ..Default::default()
            })
            .into()
    }

    /// Petit bouton discret (fermer, verrouiller…).
    fn bouton_doux(app: &App, t: &str, act: FMsg) -> Element<'static, Msg> {
        let th = &app.cfg.general.theme;
        let r = crate::theme::readable(th.fg, th.bg, th.accent);
        let fg = color(r.fg, 1.0);
        let edge = color(r.edge, 1.0);
        let wrap = Msg::MymfaF;
        button(text(t.to_string()).font(app.font).size(13).color(fg))
            .padding([7, 12])
            .style(move |_, st| button::Style {
                background: matches!(st, button::Status::Hovered | button::Status::Pressed)
                    .then(|| Color { a: 0.12, ..fg }.into()),
                text_color: fg,
                border: Border::default().rounded(8).width(1).color(edge),
                ..Default::default()
            })
            .on_press(wrap(act))
            .into()
    }

    /* ── Onglet Codes ── */

    fn vue_codes(&self, app: &App) -> Element<'_, Msg> {
        let th = &app.cfg.general.theme;
        let r = crate::theme::readable(th.fg, th.bg, th.accent);
        let fg = color(r.fg, 1.0);
        let dim = color(r.dim, 1.0);
        let accent = color(th.accent, 1.0);
        let edge = color(r.edge, 1.0);
        let wrap = Msg::MymfaF;
        let mut col = column![].spacing(10);

        // Recherche, masquage, ajout.
        col = col.push(
            row![
                text_input(
                    crate::i18n::tr("Rechercher (service, compte, étiquette)…"),
                    &self.recherche,
                )
                .font(app.font)
                .size(14)
                .padding([8, 10])
                .width(Length::Fill)
                .style(move |_, status| {
                    let bord = match status {
                        text_input::Status::Focused { .. } => accent,
                        _ => edge,
                    };
                    text_input::Style {
                        background: Color { a: 0.06, ..fg }.into(),
                        border: Border::default().rounded(8).width(1).color(bord),
                        icon: fg,
                        placeholder: dim,
                        value: fg,
                        selection: Color { a: 0.35, ..accent },
                    }
                })
                .on_input(move |s| wrap(FMsg::Recherche(s))),
                bouton_simple(
                    app,
                    if self.masque {
                        crate::i18n::tr("Afficher les codes")
                    } else {
                        crate::i18n::tr("Masquer les codes")
                    },
                    FMsg::Masquer(!self.masque)
                ),
                bouton_simple(
                    app,
                    crate::i18n::tr("Ajouter un compte"),
                    FMsg::NouveauCompte(true)
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );

        // Filtres : Tous / Favoris / étiquettes.
        let mut filtres = row![
            filtre_bouton(
                app,
                self.filtre == Filtre::Tous,
                crate::i18n::tr("Tous"),
                FMsg::Filtre(Filtre::Tous)
            ),
            filtre_bouton(
                app,
                self.filtre == Filtre::Favoris,
                &format!("★ {}", crate::i18n::tr("Favoris")),
                FMsg::Filtre(Filtre::Favoris)
            ),
        ]
        .spacing(6);
        for t in self.etiquettes() {
            let actif = f_meme_etiquette(&self.filtre, &t);
            filtres = filtres.push(filtre_bouton(
                app,
                actif,
                &format!("#{t}"),
                FMsg::Filtre(Filtre::Etiquette(t)),
            ));
        }
        col = col.push(filtres);

        // Formulaire du nouveau compte (aperçu puis ajout).
        if self.nouveau.ouvert {
            col = col.push(self.vue_nouveau_compte(app));
        }

        // Les cartes de codes.
        let visibles = self.codes_visibles();
        if visibles.is_empty() {
            col = col.push(
                text(
                    crate::i18n::tr(
                        "Aucun code : ajoutez un compte par son URI otpauth ou sa clé.",
                    )
                    .to_string(),
                )
                .font(app.font)
                .size(13)
                .color(dim),
            );
        }
        for c in visibles {
            let affiche = if self.masque {
                "••• •••".to_string()
            } else {
                c.code.clone()
            };
            let id = c.id.clone();
            let label = c.label();
            let carte = container(
                row![
                    column![
                        text(c.label()).font(app.font).size(14).color(fg),
                        text(if c.tag.is_empty() {
                            String::new()
                        } else {
                            format!("#{}", c.tag)
                        })
                        .font(app.font)
                        .size(11)
                        .color(accent),
                    ]
                    .spacing(2)
                    .width(Length::FillPortion(5)),
                    button(text(affiche).font(app.font).size(22).color(if self.masque {
                        dim
                    } else {
                        fg
                    }))
                    .padding([6, 14])
                    .style(move |_, st| button::Style {
                        background: Some(
                            match st {
                                button::Status::Hovered | button::Status::Pressed => {
                                    Color { a: 0.12, ..accent }
                                }
                                _ => Color { a: 0.07, ..fg },
                            }
                            .into(),
                        ),
                        text_color: fg,
                        border: Border::default().rounded(10).width(1).color(edge),
                        ..Default::default()
                    })
                    .on_press_maybe((!self.masque).then_some(wrap(FMsg::CopierCode(id.clone())))),
                    iced::widget::Space::new().width(Length::FillPortion(1)),
                    // HOTP : bouton « code suivant » ; TOTP : décompte.
                    if c.period == 0 {
                        container(bouton_simple(
                            app,
                            crate::i18n::tr("Suivant"),
                            FMsg::HotpSuivant(id.clone()),
                        ))
                        .width(Length::Shrink)
                    } else {
                        container(
                            text(format!("{} s", c.remaining))
                                .font(app.font)
                                .size(13)
                                .color(dim),
                        )
                        .width(Length::Shrink)
                    },
                    bouton_etoile(app, id.clone(), c.favori),
                    bouton_poubelle(app, id, label),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .padding(10)
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(Color { a: 0.05, ..fg }.into()),
                border: Border::default().rounded(10).width(1).color(edge),
                ..Default::default()
            });
            col = col.push(carte);
        }
        col.into()
    }

    fn vue_nouveau_compte(&self, app: &App) -> Element<'_, Msg> {
        let th = &app.cfg.general.theme;
        let r = crate::theme::readable(th.fg, th.bg, th.accent);
        let fg = color(r.fg, 1.0);
        let dim = color(r.dim, 1.0);
        let accent = color(th.accent, 1.0);
        let edge = color(r.edge, 1.0);
        let wrap = Msg::MymfaF;
        let saisie = |placeholder: &'static str, valeur: &str, champ: &'static str| {
            text_input(placeholder, valeur)
                .font(app.font)
                .size(13)
                .padding([8, 10])
                .width(Length::Fill)
                .style(move |_, status| {
                    let bord = match status {
                        text_input::Status::Focused { .. } => accent,
                        _ => edge,
                    };
                    text_input::Style {
                        background: Color { a: 0.06, ..fg }.into(),
                        border: Border::default().rounded(8).width(1).color(bord),
                        icon: fg,
                        placeholder: dim,
                        value: fg,
                        selection: Color { a: 0.35, ..accent },
                    }
                })
                .on_input(move |s| wrap(FMsg::NouveauChamp(champ, s)))
        };
        let mut col = column![
            text(crate::i18n::tr("Nouveau compte").to_string())
                .font(app.font)
                .size(14)
                .color(fg),
            saisie(
                "URI otpauth:// ou clé base32 (JBSWY3DP…)",
                &self.nouveau.texte,
                "texte",
            ),
            row![
                saisie("étiquette (optionnelle)", &self.nouveau.tag, "tag"),
                saisie(
                    "description (optionnelle)",
                    &self.nouveau.description,
                    "description"
                ),
            ]
            .spacing(8),
            row![
                bouton_simple(app, crate::i18n::tr("Vérifier"), FMsg::VerifierCompte),
                bouton_simple(app, crate::i18n::tr("Annuler"), FMsg::NouveauCompte(false)),
            ]
            .spacing(8),
        ]
        .spacing(8)
        .width(Length::Fill);

        if let Some(a) = &self.nouveau.apercu {
            let service = a.get("issuer").and_then(Value::as_str).unwrap_or("?");
            let compte = a.get("account").and_then(Value::as_str).unwrap_or("");
            let code = a.get("code").and_then(Value::as_str).unwrap_or("?");
            col = col.push(
                container(
                    row![
                        text(format!("{service} — {compte}"))
                            .font(app.font)
                            .size(13)
                            .color(fg),
                        iced::widget::Space::new().width(Length::Fill),
                        text(code).font(app.font).size(18).color(accent),
                        bouton_simple(
                            app,
                            crate::i18n::tr("Ajouter ce compte"),
                            FMsg::AjouterCompte
                        ),
                    ]
                    .spacing(10)
                    .align_y(Alignment::Center),
                )
                .padding(8)
                .width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(Color { a: 0.06, ..accent }.into()),
                    border: Border::default().rounded(8).width(1).color(accent),
                    ..Default::default()
                }),
            );
        }
        if let Some(e) = &self.nouveau.erreur {
            col = col.push(text(e.clone()).font(app.font).size(12).color(dim));
        }
        col.into()
    }

    /* ── Onglet Réglages : verrouillage, export, synchro Drive (MLG-47) ── */

    fn vue_reglages(&self, app: &App) -> Element<'_, Msg> {
        let th = &app.cfg.general.theme;
        let r = crate::theme::readable(th.fg, th.bg, th.accent);
        let fg = color(r.fg, 1.0);
        let dim = color(r.dim, 1.0);
        let edge = color(r.edge, 1.0);
        let warn = color(r.error, 1.0);
        let mut col = column![].spacing(12);

        // Synchro Drive.
        col = col.push(
            text(crate::i18n::tr("Synchronisation Google Drive").to_string())
                .font(app.font)
                .size(14)
                .color(fg),
        );
        let statut = self.statut_synchro.clone().unwrap_or(Value::Null);
        let connecte = statut
            .get("connecte")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let horloge = statut
            .get("horlogeVerifiee")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let revision = statut
            .get("modifieeLe")
            .and_then(Value::as_str)
            .unwrap_or("");
        let lignes = row![
            text(if connecte {
                crate::i18n::tr("Connecté à Google Drive.").to_string()
            } else {
                crate::i18n::tr("Non connecté.").to_string()
            })
            .font(app.font)
            .size(13)
            .color(if connecte { fg } else { dim }),
            text(if horloge {
                crate::i18n::tr("Horloge vérifiée (NTS).").to_string()
            } else {
                crate::i18n::tr("Horloge non vérifiée.").to_string()
            })
            .font(app.font)
            .size(13)
            .color(if horloge { fg } else { warn }),
            text(if revision.is_empty() {
                String::new()
            } else {
                crate::tr!("Dernière sauvegarde : {revision}", revision = revision).to_string()
            })
            .font(app.font)
            .size(13)
            .color(dim),
        ]
        .spacing(14)
        .wrap();
        col = col.push(lignes);
        col = col.push(
            row![
                if connecte {
                    bouton_simple(
                        app,
                        crate::i18n::tr("Synchroniser maintenant"),
                        FMsg::SyncNow,
                    )
                } else {
                    bouton_simple(
                        app,
                        crate::i18n::tr("Connecter Google Drive"),
                        FMsg::ConnecterDrive,
                    )
                },
                if connecte {
                    bouton_simple(app, crate::i18n::tr("Déconnecter"), FMsg::DeconnecterDrive)
                } else {
                    iced::widget::Space::new().into()
                },
            ]
            .spacing(8),
        );

        // Journal de synchro (dernières entrées).
        if let Some(j) = &self.journal {
            let entrees = j.get("entrees").and_then(Value::as_array);
            let Some(entrees) = entrees.filter(|e| !e.is_empty()) else {
                return col.into();
            };
            let mut jcol = column![].spacing(4);
            for e in entrees.iter().rev().take(8) {
                jcol = jcol.push(
                    row![
                        text(
                            e.get("at")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string()
                        )
                        .font(app.font)
                        .size(11)
                        .color(dim),
                        text(
                            e.get("texte")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string()
                        )
                        .font(app.font)
                        .size(12)
                        .color(fg),
                    ]
                    .spacing(10),
                );
            }
            col = col.push(
                container(jcol)
                    .padding(8)
                    .width(Length::Fill)
                    .style(move |_| container::Style {
                        background: Some(Color { a: 0.05, ..fg }.into()),
                        border: Border::default().rounded(8).width(1).color(edge),
                        ..Default::default()
                    }),
            );
        }
        col.into()
    }
}

fn f_meme_etiquette(f: &Filtre, t: &str) -> bool {
    matches!(f, Filtre::Etiquette(e) if e == t)
}

fn filtre_bouton(app: &App, actif: bool, t: &str, act: FMsg) -> Element<'static, Msg> {
    let th = &app.cfg.general.theme;
    let r = crate::theme::readable(th.fg, th.bg, th.accent);
    let fg = color(r.fg, 1.0);
    let accent = color(th.accent, 1.0);
    let wrap = Msg::MymfaF;
    button(
        text(t.to_string())
            .font(app.font)
            .size(12)
            .color(if actif { accent } else { fg }),
    )
    .padding([5, 10])
    .style(move |_, _| button::Style {
        background: Some(
            if actif {
                Color { a: 0.15, ..accent }
            } else {
                Color { a: 0.05, ..fg }
            }
            .into(),
        ),
        text_color: if actif { accent } else { fg },
        border: Border::default().rounded(999).width(1).color(if actif {
            accent
        } else {
            Color { a: 0.2, ..fg }
        }),
        ..Default::default()
    })
    .on_press(wrap(act))
    .into()
}

fn bouton_simple(app: &App, t: &str, act: FMsg) -> Element<'static, Msg> {
    let th = &app.cfg.general.theme;
    let r = crate::theme::readable(th.fg, th.bg, th.accent);
    let btn = color(r.button, 1.0);
    let on_btn = color(r.on_button, 1.0);
    let wrap = Msg::MymfaF;
    button(text(t.to_string()).font(app.font).size(13).color(on_btn))
        .padding([7, 12])
        .style(move |_, st| button::Style {
            background: Some(
                match st {
                    button::Status::Hovered | button::Status::Pressed => {
                        color(crate::theme::mix(r.button, r.on_button, 0.12), 1.0)
                    }
                    _ => btn,
                }
                .into(),
            ),
            text_color: on_btn,
            border: Border::default().rounded(8).width(1).color(btn),
            ..Default::default()
        })
        .on_press(wrap(act))
        .into()
}

fn bouton_etoile(app: &App, id: String, favori: bool) -> Element<'static, Msg> {
    let th = &app.cfg.general.theme;
    let r = crate::theme::readable(th.fg, th.bg, th.accent);
    let dim = color(r.dim, 1.0);
    let accent = color(th.accent, 1.0);
    let wrap = Msg::MymfaF;
    let marque = if favori { "★" } else { "☆" };
    let c = if favori { accent } else { dim };
    button(text(marque.to_string()).font(app.font).size(15).color(c))
        .padding(4)
        .style(|_, _| button::Style::default())
        .on_press(wrap(FMsg::Favori(id, !favori)))
        .into()
}

fn bouton_poubelle(app: &App, id: String, label: String) -> Element<'static, Msg> {
    let th = &app.cfg.general.theme;
    let r = crate::theme::readable(th.fg, th.bg, th.accent);
    let dim = color(r.dim, 1.0);
    let warn = color(r.error, 1.0);
    let wrap = Msg::MymfaF;
    button(text("🗑").font(app.font).size(13).color(dim))
        .padding(4)
        .style(move |_, st| button::Style {
            text_color: match st {
                button::Status::Hovered | button::Status::Pressed => warn,
                _ => dim,
            },
            ..Default::default()
        })
        .on_press(wrap(FMsg::SupprimerCompte(id, label)))
        .into()
}
