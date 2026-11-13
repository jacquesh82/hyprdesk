//! Fenêtre myMFA (MLG-46) : codes 2FA et réglages (dont la synchronisation
//! Drive, MLG-47), au style de hyprdesk, dans une fenêtre d'application
//! ordinaire (MLG-103) : `hyprdesk mymfa window`, processus à part que le
//! démon fait lancer par Hyprland, flottante et centrée, déplaçable et rangée
//! comme les autres. Le démon n'héberge plus ces vues que dans l'aperçu.
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
use serde_json::Value;

use super::{App, Msg, color};
use crate::mymfa;

const WIDTH: f32 = 860.0;
const HEIGHT: f32 = 600.0;
const MARGE: f32 = 18.0;
/// Le mot de passe révélé quitte la fenêtre après ce délai (comme la copie
/// sensible du démon, 30 s).
const REVEAL_S: u64 = 30;

/// Onglet affiché.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Onglet {
    Codes,
    MotsDePasse,
    Activite,
    TableauDeBord,
    Reglages,
}

impl Onglet {
    fn titre(self) -> String {
        crate::i18n::tr(match self {
            Onglet::Codes => "Codes 2FA",
            Onglet::MotsDePasse => "Mots de passe",
            Onglet::Activite => "Activité",
            Onglet::TableauDeBord => "Tableau de bord",
            Onglet::Reglages => "Réglages",
        })
        .to_string()
    }
    const TOUS: [Onglet; 5] = [
        Onglet::Codes,
        Onglet::MotsDePasse,
        Onglet::Activite,
        Onglet::TableauDeBord,
        Onglet::Reglages,
    ];
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
    /// `Content` refusé : le message, et si le coffre est verrouillé.
    ContenuRefuse(String, bool),
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
    /// Mots de passe (MLG-46).
    VoirSite(Option<String>),
    Reveal(String),
    Revele(Result<String, String>),
    CopierLogin(String, &'static str),
    FavoriLogin(String, bool),
    FormulaireSite(bool, Option<String>),
    ChampSite(&'static str, String),
    Generer,
    Genere(Result<String, String>),
    Longueur(u16),
    Bascule(&'static str),
    EnregistrerSite,
    SupprimerLogin(String),
    SupprimerPasskey(String),
    /// Tableau de bord (MLG-46).
    Audit(Result<Value, String>),
    /// Scanner : image et capture écran.
    DecoderImage,
    ImageDecode(Result<String, String>),
    CapturerEcran,
    Capture(Result<String, String>),
    CheminImage(String),
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

/// Ce que la fenêtre demande à son hôte, au-delà d'elle-même.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sortie {
    Fermer,
    /// Coffre verrouillé entre-temps : la carte de déverrouillage prend le relais.
    Deverrouiller,
}

/// Fenêtre ouverte.
pub struct Fenetre {
    pub win: Id,
    /// Posé par `update`, lu (et vidé) par l'hôte.
    pub sortie: Option<Sortie>,
    pub onglet: Onglet,
    /// Contenu complet (`Content`), sans aucun secret.
    pub contenu: Option<Value>,
    /// Codes en cours (`Codes` du démon).
    pub codes: Vec<CodeVue>,
    pub recherche: String,
    pub filtre: Filtre,
    /// Codes masqués (l'état, cohérent dans la fenêtre).
    pub masque: bool,
    /// Site ouvert en détail (Mots de passe) et son mot de passe révélé.
    pub detail: Option<String>,
    pub revele: String,
    pub revele_vivant: bool,
    /// Formulaire d'un site (vide : création).
    pub formulaire: Option<FormulaireSite>,
    pub generateur: Generateur,
    /// Nouveau compte 2FA (formulaire du code, pas encore du scanner d'image).
    pub nouveau: NouveauCompte,
    pub statut_synchro: Option<Value>,
    pub journal: Option<Value>,
    /// Santé des mots de passe (Audit du démon).
    pub audit: Option<Value>,
    /// Chemin d'image à décoder (Scanner du nouveau compte).
    pub image_chemin: String,
    pub message: Option<(bool, String)>,
    pub occupe: bool,
    pub tiques: u32,
}

/// Formulaire d'un site (création ou modification).
#[derive(Debug, Clone, Default)]
pub struct FormulaireSite {
    pub id: String, // vide = création
    pub origin: String,
    pub username: String,
    pub password: String,
    pub title: String,
    pub note: String,
    pub tag: String,
}

/// État du générateur.
#[derive(Debug, Clone)]
pub struct Generateur {
    pub longueur: u16,
    pub chiffres: bool,
    pub symboles: bool,
    pub phrase: bool,
}

impl Default for Generateur {
    fn default() -> Self {
        Self {
            longueur: 20,
            chiffres: true,
            symboles: true,
            phrase: false,
        }
    }
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
            sortie: None,
            onglet: Onglet::Codes,
            contenu: None,
            codes: Vec::new(),
            recherche: String::new(),
            filtre: Filtre::Tous,
            masque: false,
            detail: None,
            revele: String::new(),
            revele_vivant: false,
            formulaire: None,
            generateur: Generateur::default(),
            nouveau: NouveauCompte::default(),
            statut_synchro: None,
            journal: None,
            audit: None,
            image_chemin: String::new(),
            message: None,
            occupe: false,
            tiques: 0,
        }
    }

    /// Efface les secrets tenus (mot de passe révélé, formulaire).
    pub fn effacer(&mut self) {
        crate::lock::pam::wipe(&mut self.revele);
        if let Some(form) = self.formulaire.as_mut() {
            crate::lock::pam::wipe(&mut form.password);
        }
    }

    fn dit_message(&mut self, erreur: bool, texte: String) {
        self.message = Some((erreur, texte));
        self.occupe = false;
    }

    /// Logins du contenu, du plus récemment utilisé au plus ancien.
    fn logins_liste(&self) -> Vec<Value> {
        let mut l = self
            .contenu
            .as_ref()
            .and_then(|c| c.get("logins"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let cle = |v: &Value| -> String {
            v.get("usedAt")
                .or_else(|| v.get("updatedAt"))
                .or_else(|| v.get("createdAt"))
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        l.sort_by_key(|v| std::cmp::Reverse(cle(v)));
        l
    }

    fn texte_de(v: &Value, c: &str) -> String {
        v.get(c).and_then(Value::as_str).unwrap_or("").to_string()
    }

    fn champ_bool(v: &Value, c: &str) -> bool {
        v.get(c).and_then(Value::as_bool).unwrap_or(false)
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
        "logins": [
            { "id": "l1", "origin": "https://github.com", "host": "github.com", "username": "jacques",
              "title": "", "note": "clé de secours dans le coffre", "tag": "travail", "favorite": true,
              "hasPassword": true, "passwordLength": 20,
              "createdAt": "2026-11-01T10:00:00.000Z", "updatedAt": "2026-11-10T09:00:00.000Z",
              "usedAt": "2026-11-11T08:30:00.000Z", "passwordChangedAt": "2026-11-10T09:00:00.000Z" },
            { "id": "l2", "origin": "https://www.ovh.com", "host": "ovh.com", "username": "moi@example.net",
              "title": "OVHcloud", "note": "", "tag": "", "favorite": false,
              "hasPassword": true, "passwordLength": 16,
              "createdAt": "2026-10-02T10:00:00.000Z", "updatedAt": "2026-10-02T10:00:00.000Z",
              "usedAt": "2026-11-09T14:00:00.000Z", "passwordChangedAt": "2026-10-02T10:00:00.000Z" },
        ],
        "passkeys": [
            { "id": "p1", "credentialId": "abc", "rpId": "github.com", "rpName": "GitHub",
              "userName": "jacques", "userDisplayName": "Jacques" },
        ],
        "tags": ["travail"],
        "history": [
            { "id": "h1", "at": "2026-11-11T08:30:00.000Z", "action": "update", "label": { "issuer": "GitHub", "account": "jacques" } },
            { "id": "h2", "at": "2026-11-11T08:29:00.000Z", "action": "create", "label": { "issuer": "OVH", "account": "moi@example.net" } },
            { "id": "h3", "at": "2026-11-09T14:00:00.000Z", "action": "delete", "label": { "issuer": "Test", "account": "" } },
        ],
    })
}

/// Ce que les vues de la fenêtre prennent de hyprdesk : thème et polices.
/// Le processus de la fenêtre d'application le tire de la config seule.
pub struct Habillage {
    pub cfg: std::sync::Arc<crate::config::Config>,
    pub font: iced::Font,
    titre: iced::Font,
    /// Fenêtre d'application (pas de carte arrondie : Hyprland décore).
    normale: bool,
}

impl Habillage {
    fn title_font(&self) -> iced::Font {
        self.titre
    }

    /// Habillage tiré d'une config, comme hyprdesk le fait de la sienne.
    fn de_config(cfg: crate::config::Config) -> Self {
        let famille = match &cfg.general.font {
            Some(f) => super::intern_font(f),
            None => crate::fonts::FAMILY,
        };
        Self {
            font: match &cfg.general.font {
                Some(f) => iced::Font::with_name(super::intern_font(f)),
                None => crate::fonts::DEFAULT,
            },
            titre: iced::Font {
                family: iced::font::Family::Name(famille),
                weight: iced::font::Weight::Bold,
                ..iced::Font::DEFAULT
            },
            cfg: std::sync::Arc::new(cfg),
            normale: true,
        }
    }
}

/// Classe (app_id) de la fenêtre d'application myMFA.
pub const APP_ID: &str = "hyprdesk-mymfa";

/// Commande qui ouvre la fenêtre d'application (ce binaire, même config).
fn commande_fenetre(config: &std::path::Path) -> String {
    let exe = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "hyprdesk".into());
    format!(
        "{} --config {} mymfa window",
        crate::launch::sh_quote(&exe),
        crate::launch::sh_quote(&config.to_string_lossy())
    )
}

/// Adresse de la fenêtre myMFA déjà ouverte, s'il y en a une.
fn fenetre_ouverte() -> Option<String> {
    let clients = hyprpanel_core::ipc::query_json("clients").ok()?;
    clients.as_array()?.iter().find_map(|c| {
        (c["class"].as_str() == Some(APP_ID))
            .then(|| c["address"].as_str().map(str::to_string))
            .flatten()
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
        // Fenêtre d'application (xdg-toplevel) dans son propre processus,
        // lancée par Hyprland : flottante et centrée, elle se déplace, se range
        // et passe derrière les autres comme toute application. Déjà ouverte :
        // elle revient au premier plan.
        let cmd = commande_fenetre(&self.config_path);
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    if let Some(adresse) = fenetre_ouverte() {
                        return crate::hypr::focus_window(&adresse).map_err(|e| e.to_string());
                    }
                    crate::hypr::exec_floating(
                        &cmd,
                        (WIDTH + 2.0 * MARGE) as u32,
                        (HEIGHT + 2.0 * MARGE) as u32,
                        None,
                    )
                    .map_err(|e| e.to_string())
                })
                .await
                .map_err(|e| e.to_string())
                .and_then(|r| r)
            },
            |r| {
                if let Err(e) = r {
                    tracing::warn!("{}", crate::tr!("fenêtre myMFA : {e}", e = e));
                }
                Msg::Noop
            },
        )
    }

    /// Ferme la fenêtre : état effacé ET surface détruite. Sans la seconde, une
    /// surface vide resterait à l'écran en gardant le clavier (exclusif).
    pub fn mfa_fenetre_close(&mut self) -> Task<Msg> {
        match self.mfa_fenetre_oublier() {
            Some(win) => iced::window::close(win),
            None => Task::none(),
        }
    }

    /// Efface l'état de la fenêtre (secrets compris) et rend sa surface, sans
    /// la fermer : pour une surface que le compositeur a déjà fermée.
    pub fn mfa_fenetre_oublier(&mut self) -> Option<Id> {
        let mut f = self.mfa_fenetre.take()?;
        f.effacer();
        Some(f.win)
    }

    fn habillage_mfa(&self, normale: bool) -> Habillage {
        Habillage {
            cfg: self.cfg.clone(),
            font: self.font,
            titre: self.title_font(),
            normale,
        }
    }

    pub fn view_mfa_fenetre(&self) -> Element<'_, Msg> {
        match &self.mfa_fenetre {
            Some(f) => f.view(&self.habillage_mfa(false)),
            None => iced::widget::Space::new().into(),
        }
    }

    pub fn mfa_fenetre_update(&mut self, m: FMsg) -> Task<Msg> {
        let Some(f) = self.mfa_fenetre.as_mut() else {
            return Task::none();
        };
        let t = f.update(m);
        match f.sortie.take() {
            None => t,
            Some(Sortie::Fermer) => Task::batch([t, self.mfa_fenetre_close()]),
            Some(Sortie::Deverrouiller) => Task::batch([
                t,
                self.mfa_fenetre_close(),
                self.mfa_update(crate::ui::mymfa::MMsg::Ask),
            ]),
        }
    }
}

impl Fenetre {
    /// Un message de la fenêtre. Ce qui la dépasse (fermer, laisser la carte
    /// de déverrouillage prendre le relais) est posé dans `sortie`, que l'hôte
    /// lit ensuite : hyprdesk, ou le processus de la fenêtre d'application.
    pub fn update(&mut self, m: FMsg) -> Task<Msg> {
        let f = &mut *self;
        match m {
            FMsg::Fermer => {
                f.sortie = Some(Sortie::Fermer);
                Task::none()
            }
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
                    Task::perform(mymfa::content(), |r| Msg::MymfaF(FMsg::Contenu(r)))
                        .chain(Task::perform(mymfa::sync_status(), |r| {
                            Msg::MymfaF(FMsg::StatutSynchro(r))
                        }))
                        .chain(Task::perform(mymfa::audit(), |r| {
                            Msg::MymfaF(FMsg::Audit(r))
                        }))
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
                // Refus : coffre verrouillé entre-temps, ou autre chose (démon
                // trop ancien pour `Content`…). L'état du démon tranche.
                Err(e) => Task::perform(mymfa::state(), move |s| {
                    Msg::MymfaF(FMsg::ContenuRefuse(e.clone(), s.vault && s.locked))
                }),
            },
            FMsg::ContenuRefuse(e, verrouille) => {
                if verrouille {
                    // La carte de déverrouillage prend le relais.
                    f.sortie = Some(Sortie::Deverrouiller);
                    Task::none()
                } else {
                    // Coffre ouvert : la fenêtre reste et dit pourquoi elle est vide.
                    f.dit_message(true, e);
                    Task::none()
                }
            }
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
            FMsg::VoirSite(id) => {
                // Changer de détail efface un mot de passe révélé qui traîne.
                if f.revele_vivant {
                    crate::lock::pam::wipe(&mut f.revele);
                    f.revele.clear();
                    f.revele_vivant = false;
                }
                f.detail = match (&f.detail, id) {
                    (_, None) => None,
                    (Some(d), Some(i)) if *d == *i => None,
                    (_, Some(i)) => Some(i),
                };
                Task::none()
            }
            FMsg::Reveal(id) => Task::perform(
                {
                    let id = id.clone();
                    async move { mymfa::reveal_password(&id).await }
                },
                |r| Msg::MymfaF(FMsg::Revele(r)),
            ),
            FMsg::Revele(r) => match r {
                Ok(mot) => {
                    crate::lock::pam::wipe(&mut f.revele);
                    f.revele = mot;
                    f.revele_vivant = true;
                    // Effacé après 30 s, comme la copie sensible du démon.
                    Task::perform(
                        tokio::time::sleep(std::time::Duration::from_secs(REVEAL_S)),
                        |()| Msg::MymfaF(FMsg::Revele(Err(String::new()))),
                    )
                }
                Err(e) if e.is_empty() => {
                    // Délai écoulé.
                    crate::lock::pam::wipe(&mut f.revele);
                    f.revele.clear();
                    f.revele_vivant = false;
                    Task::none()
                }
                Err(e) => Task::done(Msg::MymfaF(FMsg::Message(true, e))),
            },
            FMsg::CopierLogin(id, champ) => Task::perform(mymfa::copy_login(id, champ), |r| {
                Msg::MymfaF(match r {
                    Ok(()) => FMsg::Message(false, tr_("Copié.")),
                    Err(e) => FMsg::Message(true, e),
                })
            }),
            FMsg::FavoriLogin(id, v) => Task::perform(
                {
                    let id = id.clone();
                    async move {
                        mymfa::apply(serde_json::json!({
                            "op": "update_login", "id": id, "patch": { "favorite": v }
                        }))
                        .await
                    }
                },
                |r| Msg::MymfaF(message(r.map(|_| tr_("Modifié.")))),
            )
            .chain(Task::perform(mymfa::content(), |r| {
                Msg::MymfaF(FMsg::Contenu(r))
            })),
            FMsg::FormulaireSite(ouvert, id) => {
                f.formulaire = ouvert.then(|| {
                    // Édition : préremplir depuis le contenu (sans le mot de passe).
                    let mut form = FormulaireSite {
                        id: id.clone().unwrap_or_default(),
                        ..Default::default()
                    };
                    if let Some(i) = &id {
                        for l in f.logins_liste() {
                            if mymfa_fenetre_champ(&l, "id") == *i {
                                for (c, k) in [
                                    ("origin", "origin"),
                                    ("username", "username"),
                                    ("title", "title"),
                                    ("note", "note"),
                                    ("tag", "tag"),
                                ] {
                                    if let Some(v) = l.get(k).and_then(Value::as_str) {
                                        match c {
                                            "origin" => form.origin = v.to_string(),
                                            "username" => form.username = v.to_string(),
                                            "title" => form.title = v.to_string(),
                                            "note" => form.note = v.to_string(),
                                            "tag" => form.tag = v.to_string(),
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                    }
                    form
                });
                Task::none()
            }
            FMsg::ChampSite(champ, v) => {
                if let Some(form) = f.formulaire.as_mut() {
                    let mut v = v;
                    match champ {
                        "origin" => form.origin = std::mem::take(&mut v),
                        "username" => form.username = std::mem::take(&mut v),
                        "password" => form.password = std::mem::take(&mut v),
                        "title" => form.title = std::mem::take(&mut v),
                        "note" => form.note = std::mem::take(&mut v),
                        "tag" => form.tag = std::mem::take(&mut v),
                        _ => {}
                    }
                }
                Task::none()
            }
            FMsg::Generer => {
                let g = f.generateur.clone();
                Task::perform(
                    async move {
                        mymfa::generate(serde_json::json!({
                            "length": g.longueur,
                            "digits": g.chiffres,
                            "symbols": g.symboles,
                            "words": g.phrase,
                        }))
                        .await
                    },
                    |r| {
                        Msg::MymfaF(FMsg::Genere(
                            r.map(|v| v.as_str().unwrap_or_default().to_string()),
                        ))
                    },
                )
            }
            FMsg::Genere(r) => {
                match r {
                    Ok(mot) => {
                        if let Some(form) = f.formulaire.as_mut() {
                            crate::lock::pam::wipe(&mut form.password);
                            form.password = mot;
                        }
                    }
                    Err(e) => f.dit_message(true, e),
                }
                Task::none()
            }
            FMsg::Longueur(n) => {
                f.generateur.longueur = n;
                Task::none()
            }
            FMsg::Bascule(k) => {
                match k {
                    "chiffres" => f.generateur.chiffres = !f.generateur.chiffres,
                    "symboles" => f.generateur.symboles = !f.generateur.symboles,
                    "phrase" => f.generateur.phrase = !f.generateur.phrase,
                    _ => {}
                }
                Task::none()
            }
            FMsg::EnregistrerSite => {
                if f.occupe {
                    return Task::none();
                }
                let Some(form) = f.formulaire.clone() else {
                    return Task::none();
                };
                if form.password.is_empty() {
                    f.dit_message(true, tr_("Le mot de passe ne peut pas être vide."));
                    return Task::none();
                }
                f.occupe = true;
                let creation = form.id.is_empty();
                let op = if creation {
                    serde_json::json!({
                        "op": "add_login", "origin": form.origin, "username": form.username,
                        "password": form.password, "title": form.title, "note": form.note, "tag": form.tag,
                    })
                } else {
                    serde_json::json!({
                        "op": "update_login", "id": form.id, "patch": {
                            "origin": form.origin, "username": form.username, "password": form.password,
                            "title": form.title, "note": form.note, "tag": form.tag,
                        }
                    })
                };
                let mut mdp = form.password.clone();
                Task::perform(mymfa::apply(op), move |r| {
                    crate::lock::pam::wipe(&mut mdp);
                    Msg::MymfaF(message(r.map(|_| {
                        if creation {
                            tr_("Site enregistré.")
                        } else {
                            tr_("Site mis à jour.")
                        }
                    })))
                })
                .chain(
                    Task::perform(mymfa::content(), |r| Msg::MymfaF(FMsg::Contenu(r)))
                        .chain(Task::done(Msg::MymfaF(FMsg::FormulaireSite(false, None)))),
                )
            }
            FMsg::SupprimerLogin(id) => Task::perform(
                {
                    let id = id.clone();
                    async move {
                        mymfa::apply(serde_json::json!({ "op": "remove_login", "id": id })).await
                    }
                },
                |r| Msg::MymfaF(message(r.map(|_| tr_("Site supprimé.")))),
            )
            .chain(Task::perform(mymfa::content(), |r| {
                Msg::MymfaF(FMsg::Contenu(r))
            })),
            FMsg::SupprimerPasskey(id) => Task::perform(
                {
                    let id = id.clone();
                    async move {
                        mymfa::apply(serde_json::json!({ "op": "remove_passkey", "id": id })).await
                    }
                },
                |r| Msg::MymfaF(message(r.map(|_| tr_("Passkey supprimée.")))),
            )
            .chain(Task::perform(mymfa::content(), |r| {
                Msg::MymfaF(FMsg::Contenu(r))
            })),
            FMsg::Audit(r) => {
                if let Ok(a) = r {
                    f.audit = Some(a);
                }
                Task::none()
            }
            FMsg::CheminImage(v) => {
                f.image_chemin = v;
                Task::none()
            }
            FMsg::DecoderImage => {
                let chemin = f.image_chemin.clone();
                Task::perform(
                    async move { tokio::task::spawn_blocking(move || decoder_image(&chemin)).await },
                    |r| {
                        Msg::MymfaF(FMsg::ImageDecode(match r {
                            Ok(interne) => interne,
                            Err(e) => Err(e.to_string()),
                        }))
                    },
                )
            }
            FMsg::ImageDecode(r) => match r {
                Ok(texte) => {
                    f.image_chemin.clear();
                    f.nouveau.texte = texte;
                    Task::done(Msg::MymfaF(FMsg::VerifierCompte))
                }
                Err(e) => Task::done(Msg::MymfaF(FMsg::Message(true, e))),
            },
            FMsg::CapturerEcran => Task::perform(
                async {
                    tokio::process::Command::new("sh")
                        .arg("-c")
                        .arg("grim -g \"$(slurp)\" /tmp/hyprdesk-qr.png && echo ok")
                        .output()
                        .await
                },
                |r| {
                    let ok = r.map(|o| o.status.success()).unwrap_or(false);
                    if ok {
                        Msg::MymfaF(FMsg::Capture(Ok("/tmp/hyprdesk-qr.png".into())))
                    } else {
                        Msg::MymfaF(FMsg::Capture(Err(crate::i18n::tr(
                            "Capture interrompue (sélection annulée ?).",
                        )
                        .to_string())))
                    }
                },
            ),
            FMsg::Capture(r) => match r {
                Ok(chemin) => {
                    f.image_chemin = chemin;
                    Task::done(Msg::MymfaF(FMsg::DecoderImage))
                }
                Err(e) => Task::done(Msg::MymfaF(FMsg::Message(true, e))),
            },
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
    fn view(&self, app: &Habillage) -> Element<'_, Msg> {
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
            Onglet::MotsDePasse => self.vue_sites(app),
            Onglet::Activite => self.vue_activite(app),
            Onglet::TableauDeBord => self.vue_tableau(app),
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

        // Carte arrondie et bordée en surface hyprdesk ; fenêtre d'application,
        // Hyprland pose déjà bordure et coins.
        let (rayon, trait_) = if app.normale { (0.0, 0.0) } else { (14.0, 1.0) };
        container(cadre)
            .padding(MARGE)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| container::Style {
                background: Some(bg.into()),
                text_color: Some(fg),
                border: Border::default().rounded(rayon).width(trait_).color(edge),
                ..Default::default()
            })
            .into()
    }

    /// Petit bouton discret (fermer, verrouiller…).
    fn bouton_doux(app: &Habillage, t: &str, act: FMsg) -> Element<'static, Msg> {
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

    fn vue_codes(&self, app: &Habillage) -> Element<'_, Msg> {
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

    fn vue_nouveau_compte(&self, app: &Habillage) -> Element<'_, Msg> {
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
            container(iced::widget::Space::new().height(1))
                .width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(edge.into()),
                    ..Default::default()
                }),
            text(crate::i18n::tr("Ou depuis un code QR :").to_string())
                .font(app.font)
                .size(12)
                .color(dim),
            row![
                text_input(crate::i18n::tr("chemin de l'image…"), &self.image_chemin,)
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
                    .on_input(move |s| wrap(FMsg::CheminImage(s))),
                bouton_simple(app, crate::i18n::tr("Décoder l'image"), FMsg::DecoderImage),
                bouton_simple(
                    app,
                    crate::i18n::tr("Capturer l'écran"),
                    FMsg::CapturerEcran
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
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

    /* ── Onglet Mots de passe (MLG-46) ── */

    fn vue_sites(&self, app: &Habillage) -> Element<'_, Msg> {
        let th = &app.cfg.general.theme;
        let r = crate::theme::readable(th.fg, th.bg, th.accent);
        let fg = color(r.fg, 1.0);
        let dim = color(r.dim, 1.0);
        let accent = color(th.accent, 1.0);
        let edge = color(r.edge, 1.0);
        let ok = color(r.focus, 1.0);
        let warn = color(r.error, 1.0);
        let wrap = Msg::MymfaF;
        let mut col = column![].spacing(10);

        // Recherche partagée + nouveau site.
        col = col.push(
            row![
                text_input(
                    crate::i18n::tr("Rechercher (site, identifiant, étiquette)…"),
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
                    crate::i18n::tr("Nouveau site"),
                    FMsg::FormulaireSite(true, None)
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );

        // Formulaire (création ou édition).
        if let Some(form) = &self.formulaire {
            col = col.push(self.vue_formulaire(app, form));
        }

        // Liste des sites, du plus récemment utilisé au plus ancien.
        let q = self.recherche.to_lowercase();
        let mut montres = 0;
        for l in self.logins_liste() {
            let id = Self::texte_de(&l, "id");
            let titre = Self::texte_de(&l, "title");
            let hote = Self::texte_de(&l, "host");
            let user = Self::texte_de(&l, "username");
            let tag = Self::texte_de(&l, "tag");
            let site = if titre.is_empty() {
                hote.clone()
            } else {
                titre.clone()
            };
            let correspond = q.is_empty()
                || site.to_lowercase().contains(&q)
                || user.to_lowercase().contains(&q)
                || tag.to_lowercase().contains(&q);
            if !correspond {
                continue;
            }
            montres += 1;
            let ouvert = self.detail.as_deref() == Some(id.as_str());
            let favori = Self::champ_bool(&l, "favorite");
            let ligne = container(
                row![
                    button(
                        column![
                            text(site.clone()).font(app.font).size(14).color(fg),
                            text(if user.is_empty() {
                                if tag.is_empty() {
                                    String::new()
                                } else {
                                    format!("#{tag}")
                                }
                            } else if tag.is_empty() {
                                user.clone()
                            } else {
                                format!("{user} · #{tag}")
                            })
                            .font(app.font)
                            .size(11)
                            .color(dim),
                        ]
                        .spacing(2)
                        .width(Length::FillPortion(6)),
                    )
                    .padding(0)
                    .style(|_, _| button::Style::default())
                    .on_press(wrap(FMsg::VoirSite(Some(id.clone())))),
                    iced::widget::Space::new().width(Length::FillPortion(1)),
                    iced::widget::Space::new().width(Length::FillPortion(1)),
                    bouton_etoile_login(app, id.clone(), favori),
                    bouton_poubelle_login(app, id, site.clone()),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .padding(10)
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(
                    if ouvert {
                        Color { a: 0.10, ..accent }
                    } else {
                        Color { a: 0.05, ..fg }
                    }
                    .into(),
                ),
                border: Border::default().rounded(10).width(1).color(if ouvert {
                    accent
                } else {
                    edge
                }),
                ..Default::default()
            });
            col = col.push(ligne);
            if !ouvert {
                continue;
            }

            // ── Détail du site : identifiant, mot de passe, note, passkeys.
            let detail = self.vue_detail_site(app, &l);
            col = col.push(container(detail).padding([0, 10]).width(Length::Fill));
            let _ = ok;
            let _ = warn;
        }
        if montres == 0 && self.formulaire.is_none() {
            col = col.push(
                text(
                    crate::i18n::tr("Aucun site : enregistrez-en un (« Nouveau site »).")
                        .to_string(),
                )
                .font(app.font)
                .size(13)
                .color(dim),
            );
        }
        col.into()
    }

    /// Détail d'un site : identifiant, mot de passe (révélation 30 s), note,
    /// étiquette, passkeys du site.
    fn vue_detail_site(&self, app: &Habillage, l: &Value) -> Element<'static, Msg> {
        let th = &app.cfg.general.theme;
        let r = crate::theme::readable(th.fg, th.bg, th.accent);
        let fg = color(r.fg, 1.0);
        let dim = color(r.dim, 1.0);
        let wrap = Msg::MymfaF;
        let id = Self::texte_de(l, "id");
        let user = Self::texte_de(l, "username");
        let note = Self::texte_de(l, "note");
        let host = Self::texte_de(l, "host");
        let a_mdp = Self::champ_bool(l, "hasPassword");
        let mut col = column![].spacing(6);

        // Identifiant.
        if !user.is_empty() {
            col = col.push(
                row![
                    text(crate::i18n::tr("Identifiant").to_string())
                        .font(app.font)
                        .size(12)
                        .color(dim)
                        .width(Length::Fixed(110.0)),
                    text(user.clone()).font(app.font).size(13).color(fg),
                    iced::widget::Space::new().width(Length::Fill),
                    bouton_simple(
                        app,
                        crate::i18n::tr("Copier l'identifiant"),
                        FMsg::CopierLogin(id.clone(), "username")
                    ),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        // Mot de passe : révélé 30 s, sinon à la demande (jamais en clair d'office).
        if a_mdp {
            let montre = self.detail.as_deref() == Some(id.as_str()) && self.revele_vivant;
            col = col.push(
                row![
                    text(crate::i18n::tr("Mot de passe").to_string())
                        .font(app.font)
                        .size(12)
                        .color(dim)
                        .width(Length::Fixed(110.0)),
                    text(if montre {
                        format!("{} (30 s)", "*".repeat(self.revele.chars().count().min(24)))
                    } else {
                        "••••••••".to_string()
                    })
                    .font(app.font)
                    .size(13)
                    .color(if montre { fg } else { dim }),
                    iced::widget::Space::new().width(Length::Fill),
                    if montre {
                        bouton_simple(
                            app,
                            crate::i18n::tr("Cacher"),
                            FMsg::Revele(Err(String::new())),
                        )
                    } else {
                        bouton_simple(app, crate::i18n::tr("Afficher"), FMsg::Reveal(id.clone()))
                    },
                    bouton_simple(
                        app,
                        crate::i18n::tr("Copier"),
                        FMsg::CopierLogin(id.clone(), "password")
                    ),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
        // Note.
        if !note.is_empty() {
            col = col.push(
                row![
                    text(crate::i18n::tr("Note").to_string())
                        .font(app.font)
                        .size(12)
                        .color(dim)
                        .width(Length::Fixed(110.0)),
                    text(note).font(app.font).size(13).color(fg),
                ]
                .spacing(8),
            );
        }
        // Étiquette et adresse.
        let tag = Self::texte_de(l, "tag");
        let origin = Self::texte_de(l, "origin");
        col = col.push(
            row![
                text(if tag.is_empty() {
                    origin.clone()
                } else {
                    format!("{origin} · #{tag}")
                })
                .font(app.font)
                .size(12)
                .color(dim),
                iced::widget::Space::new().width(Length::Fill),
                bouton_simple(
                    app,
                    crate::i18n::tr("Modifier"),
                    FMsg::FormulaireSite(true, Some(id.clone()))
                ),
            ]
            .spacing(8),
        );
        // Code 2FA du même site (service dont le nom ou l'adresse correspond).
        let comptes = self.comptes_apercu();
        let mut compte_lie: Option<&Value> = None;
        for a in &comptes {
            let issuer = Self::texte_de(a, "issuer");
            if !host.is_empty()
                && !issuer.is_empty()
                && (host.contains(&issuer.to_lowercase()) || issuer.to_lowercase().contains(&host))
            {
                compte_lie = Some(a);
                break;
            }
        }
        if let Some(a) = compte_lie {
            col = col.push(
                row![
                    text(crate::i18n::tr("Code 2FA du site").to_string())
                        .font(app.font)
                        .size(12)
                        .color(dim)
                        .width(Length::Fixed(110.0)),
                    text(format!(
                        "{} — {}",
                        Self::texte_de(a, "issuer"),
                        Self::texte_de(a, "account")
                    ))
                    .font(app.font)
                    .size(13)
                    .color(fg),
                ]
                .spacing(8),
            );
        }
        // Passkeys du site.
        for p in self
            .contenu
            .as_ref()
            .and_then(|c| c.get("passkeys"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let rp = Self::texte_de(p, "rpId");
            if !host.is_empty() && !rp.is_empty() && !host.ends_with(&rp) && !rp.ends_with(&host) {
                continue;
            }
            let pid = Self::texte_de(p, "id");
            let nom = {
                let n = Self::texte_de(p, "userName");
                let d = Self::texte_de(p, "userDisplayName");
                if n.is_empty() { d } else { n }
            };
            col = col.push(
                row![
                    text(format!("🔑 {} — {}", rp, nom))
                        .font(app.font)
                        .size(12)
                        .color(fg),
                    iced::widget::Space::new().width(Length::Fill),
                    bouton_simple(
                        app,
                        crate::i18n::tr("Supprimer la passkey"),
                        FMsg::SupprimerPasskey(pid)
                    ),
                ]
                .spacing(8),
            );
        }
        let _ = wrap;
        col.into()
    }

    fn vue_formulaire(&self, app: &Habillage, form: &FormulaireSite) -> Element<'static, Msg> {
        let th = &app.cfg.general.theme;
        let r = crate::theme::readable(th.fg, th.bg, th.accent);
        let fg = color(r.fg, 1.0);
        let dim = color(r.dim, 1.0);
        let accent = color(th.accent, 1.0);
        let edge = color(r.edge, 1.0);
        let ok = color(r.focus, 1.0);
        let warn = color(r.error, 1.0);
        let wrap = Msg::MymfaF;
        let creation = form.id.is_empty();
        let champ = |placeholder: &'static str, valeur: &str, nom: &'static str, secret: bool| {
            let mut t = text_input(placeholder, valeur)
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
                .on_input(move |s| wrap(FMsg::ChampSite(nom, s)));
            if secret {
                t = t.secure(secret);
            }
            t
        };
        let (niveau, bits) = solidite(&form.password);
        let jauge = if form.password.is_empty() {
            None
        } else {
            let (c, t) = match niveau {
                1 => (warn, crate::i18n::tr("faible").to_string()),
                2 => (dim, crate::i18n::tr("moyen").to_string()),
                3 => (accent, crate::i18n::tr("bon").to_string()),
                _ => (ok, crate::i18n::tr("excellent").to_string()),
            };
            Some(
                row![
                    text(crate::i18n::tr("Solidité").to_string())
                        .font(app.font)
                        .size(11)
                        .color(dim),
                    container(
                        container(iced::widget::Space::new().width(Length::Fill))
                            .width(Length::Fill)
                            .style(move |_| container::Style {
                                background: Some(c.into()),
                                ..Default::default()
                            }),
                    )
                    .width(match niveau {
                        1 => Length::FillPortion(1),
                        2 => Length::FillPortion(2),
                        3 => Length::FillPortion(3),
                        _ => Length::FillPortion(4),
                    })
                    .height(4),
                    text(format!("{t} · {bits} bits"))
                        .font(app.font)
                        .size(11)
                        .color(c),
                ]
                .spacing(8)
                .align_y(Alignment::Center)
                .width(Length::Fill),
            )
        };
        let gene = &self.generateur;
        let mut col = column![
            text(if creation {
                crate::i18n::tr("Nouveau site").to_string()
            } else {
                crate::i18n::tr("Modifier le site").to_string()
            })
            .font(app.font)
            .size(14)
            .color(fg),
            champ(
                "https://exemple.fr (adresse)",
                &form.origin,
                "origin",
                false
            ),
            row![
                champ("identifiant", &form.username, "username", false),
                champ("titre (optionnel)", &form.title, "title", false),
            ]
            .spacing(8),
            champ("mot de passe", &form.password, "password", true),
        ]
        .spacing(6)
        .width(Length::Fill);
        if let Some(j) = jauge {
            col = col.push(j);
        }
        // Générateur : longueur, chiffres, symboles, phrase.
        col = col.push(
            row![
                text(crate::i18n::tr("Générer :").to_string())
                    .font(app.font)
                    .size(12)
                    .color(dim),
                bouton_simple(
                    app,
                    &format!("− {}", gene.longueur),
                    FMsg::Longueur(gene.longueur.saturating_sub(4).max(8))
                ),
                text(gene.longueur.to_string())
                    .font(app.font)
                    .size(12)
                    .color(fg),
                bouton_simple(
                    app,
                    &format!("+ {}", gene.longueur + 4),
                    FMsg::Longueur((gene.longueur + 4).min(64))
                ),
                bascule(app, "chiffres", gene.chiffres, crate::i18n::tr("123")),
                bascule(app, "symboles", gene.symboles, crate::i18n::tr("#$&")),
                bascule(app, "phrase", gene.phrase, crate::i18n::tr("phrase")),
                bouton_simple(app, crate::i18n::tr("Générer"), FMsg::Generer),
            ]
            .spacing(6)
            .align_y(Alignment::Center)
            .wrap(),
        );
        col = col.push(
            row![
                champ("note (optionnelle)", &form.note, "note", false),
                champ("étiquette (optionnelle)", &form.tag, "tag", false),
            ]
            .spacing(8),
        );
        col = col.push(
            row![
                bouton_simple(
                    app,
                    &if creation {
                        crate::i18n::tr("Enregistrer le site").to_string()
                    } else {
                        crate::i18n::tr("Mettre à jour").to_string()
                    },
                    FMsg::EnregistrerSite,
                ),
                bouton_simple(
                    app,
                    crate::i18n::tr("Annuler"),
                    FMsg::FormulaireSite(false, None)
                ),
            ]
            .spacing(8),
        );
        container(col)
            .padding(10)
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(Color { a: 0.08, ..accent }.into()),
                border: Border::default().rounded(10).width(1).color(accent),
                ..Default::default()
            })
            .into()
    }

    /// Comptes 2FA du contenu (aperçu sans secret).
    fn comptes_apercu(&self) -> Vec<Value> {
        self.contenu
            .as_ref()
            .and_then(|c| c.get("accounts"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
    }

    /* ── Onglet Activité (MLG-46) ── */

    fn vue_activite(&self, app: &Habillage) -> Element<'_, Msg> {
        let th = &app.cfg.general.theme;
        let r = crate::theme::readable(th.fg, th.bg, th.accent);
        let fg = color(r.fg, 1.0);
        let dim = color(r.dim, 1.0);
        let edge = color(r.edge, 1.0);
        let mut col = column![].spacing(8);
        let vide = self
            .contenu
            .as_ref()
            .and_then(|c| c.get("history"))
            .and_then(Value::as_array)
            .map(|h| h.is_empty())
            .unwrap_or(true);
        if vide {
            col = col.push(
                text(crate::i18n::tr("Aucune activité enregistrée.").to_string())
                    .font(app.font)
                    .size(13)
                    .color(dim),
            );
            return col.into();
        }
        let entrees = self
            .contenu
            .as_ref()
            .and_then(|c| c.get("history"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        // Groupées par jour, du plus récent au plus ancien (l'historique du
        // web est déjà trié décroissant, on regarde la date de chaque entrée).
        let mut jour_courant = String::new();
        for e in &entrees {
            let at = Self::texte_de(e, "at");
            let jour = at.get(..10).unwrap_or("").to_string();
            if jour != jour_courant {
                jour_courant = jour.clone();
                let libelle = if jour
                    == self
                        .contenu
                        .as_ref()
                        .map(|_| aujourd_hui())
                        .unwrap_or_default()
                {
                    crate::i18n::tr("Aujourd'hui").to_string()
                } else {
                    jour.clone()
                };
                col = col.push(text(libelle).font(app.font).size(12).color(dim));
            }
            let action = Self::texte_de(e, "action");
            let label = e
                .get("label")
                .map(|l| {
                    let issuer = l.get("issuer").and_then(Value::as_str).unwrap_or("");
                    let compte = l.get("account").and_then(Value::as_str).unwrap_or("");
                    match (issuer.is_empty(), compte.is_empty()) {
                        (false, false) => format!("{issuer} — {compte}"),
                        (false, true) => issuer.to_string(),
                        _ => compte.to_string(),
                    }
                })
                .unwrap_or_default();
            col = col.push(
                container(
                    row![
                        text(at.get(11..16).unwrap_or("").to_string())
                            .font(app.font)
                            .size(12)
                            .color(dim)
                            .width(Length::Fixed(46.0)),
                        text(Self::verbe_site(&action))
                            .font(app.font)
                            .size(13)
                            .color(fg)
                            .width(Length::Fixed(90.0)),
                        text(label).font(app.font).size(13).color(fg),
                    ]
                    .spacing(10),
                )
                .padding([4, 8])
                .width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(Color { a: 0.04, ..fg }.into()),
                    border: Border::default().rounded(6).width(1).color(edge),
                    ..Default::default()
                }),
            );
        }
        col.into()
    }

    fn verbe_site(action: &str) -> String {
        crate::i18n::tr(match action {
            "create" => "création",
            "update" => "modification",
            "delete" => "suppression",
            "import" => "import",
            "security" => "sécurité",
            "hotp" => "code HOTP",
            autre => autre,
        })
        .to_string()
    }

    /* ── Onglet Tableau de bord (MLG-46) : chiffres, usage, santé ── */

    fn vue_tableau(&self, app: &Habillage) -> Element<'_, Msg> {
        let th = &app.cfg.general.theme;
        let r = crate::theme::readable(th.fg, th.bg, th.accent);
        let fg = color(r.fg, 1.0);
        let dim = color(r.dim, 1.0);
        let accent = color(th.accent, 1.0);
        let edge = color(r.edge, 1.0);
        let warn = color(r.error, 1.0);
        let mut col = column![].spacing(12);

        // ── Chiffres clés.
        let comptes = self
            .contenu
            .as_ref()
            .and_then(|c| c.get("accounts"))
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        let logins = self
            .contenu
            .as_ref()
            .and_then(|c| c.get("logins"))
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        let cles = self
            .contenu
            .as_ref()
            .and_then(|c| c.get("passkeys"))
            .and_then(Value::as_array)
            .map(Vec::len)
            .unwrap_or(0);
        let chiffres = row![
            self.chiffre_carte(app, &comptes.to_string(), crate::i18n::tr("codes 2FA")),
            self.chiffre_carte(
                app,
                &logins.to_string(),
                crate::i18n::tr("sites enregistrés")
            ),
            self.chiffre_carte(app, &cles.to_string(), crate::i18n::tr("passkeys")),
        ]
        .spacing(8);
        col = col.push(chiffres);

        // ── Usage sur 30 jours : entrées d'historique par jour.
        col = col.push(
            text(crate::i18n::tr("Usage sur 30 jours").to_string())
                .font(app.font)
                .size(14)
                .color(fg),
        );
        let histoire = self
            .contenu
            .as_ref()
            .and_then(|c| c.get("history"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let mut par_jour: std::collections::BTreeMap<String, u32> =
            std::collections::BTreeMap::new();
        for e in &histoire {
            if let Some(jour) = Self::texte_de(e, "at").get(..10) {
                *par_jour.entry(jour.to_string()).or_insert(0) += 1;
            }
        }
        let jours: Vec<(String, u32)> = par_jour.into_iter().collect();
        let sommet = jours.iter().map(|(_, n)| *n).max().unwrap_or(1).max(1);
        let mut barres = row![].spacing(3).align_y(Alignment::End);
        let largeur = jours.len();
        for (jour, n) in jours.iter().rev().take(30).rev() {
            let hauteur = 6.0 + 44.0 * (*n as f32 / sommet as f32);
            barres = barres.push(
                container(
                    column![
                        container(iced::widget::Space::new().height(hauteur))
                            .width(Length::Fill)
                            .style(move |_| container::Style {
                                background: Some(accent.into()),
                                ..Default::default()
                            }),
                        text(jour.get(8..10).unwrap_or("").to_string())
                            .font(app.font)
                            .size(9)
                            .color(dim),
                    ]
                    .spacing(2),
                )
                .width(Length::FillPortion(largeur.max(1) as u16)),
            );
        }
        if jours.is_empty() {
            barres = barres.push(
                text(crate::i18n::tr("Aucun usage enregistré sur 30 jours.").to_string())
                    .font(app.font)
                    .size(12)
                    .color(dim),
            );
        }
        col = col.push(
            container(barres)
                .padding(8)
                .width(Length::Fill)
                .style(move |_| container::Style {
                    background: Some(Color { a: 0.05, ..fg }.into()),
                    border: Border::default().rounded(8).width(1).color(edge),
                    ..Default::default()
                }),
        );

        // ── Santé des mots de passe (Audit du démon : faibles, réutilisés,
        // à renouveler — jamais de mot de passe dans la réponse).
        col = col.push(
            text(crate::i18n::tr("Santé des mots de passe").to_string())
                .font(app.font)
                .size(14)
                .color(fg),
        );
        let Some(a) = &self.audit else {
            col = col.push(
                text(crate::i18n::tr("Chargement de l'audit…").to_string())
                    .font(app.font)
                    .size(12)
                    .color(dim),
            );
            return col.into();
        };
        let section_sante =
            |titre: String, vide: String, items: Vec<Value>, cle_label: &str| -> Element<'_, Msg> {
                let mut bloc = column![text(titre).font(app.font).size(13).color(warn),].spacing(4);
                if items.is_empty() {
                    bloc = bloc.push(text(vide).font(app.font).size(12).color(dim));
                }
                for item in &items {
                    bloc = bloc.push(
                        text(Self::texte_de(item, cle_label))
                            .font(app.font)
                            .size(12)
                            .color(fg),
                    );
                }
                bloc.into()
            };
        let faibles = a
            .get("faibles")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let reutilises = a
            .get("reutilises")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let a_renouveller = a
            .get("aRenouveler")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if faibles.is_empty() && reutilises.is_empty() && a_renouveller.is_empty() {
            col = col.push(
                text(
                    crate::i18n::tr(
                        "Aucun problème détecté : mots de passe forts, uniques et récents.",
                    )
                    .to_string(),
                )
                .font(app.font)
                .size(12)
                .color(ok_vert(app)),
            );
        }
        if !faibles.is_empty() {
            col = col.push(section_sante(
                crate::tr!("Faibles ({n})", n = faibles.len()).to_string(),
                String::new(),
                faibles,
                "label",
            ));
        }
        if !reutilises.is_empty() {
            let mut bloc = column![
                text(crate::tr!("Réutilisés ({n})", n = reutilises.len()).to_string())
                    .font(app.font)
                    .size(13)
                    .color(warn)
            ]
            .spacing(4);
            for groupe in &reutilises {
                let sites = groupe
                    .get("sites")
                    .and_then(Value::as_array)
                    .map(|s| {
                        s.iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(" · ")
                    })
                    .unwrap_or_default();
                bloc = bloc.push(text(sites).font(app.font).size(12).color(fg));
            }
            col = col.push(bloc);
        }
        if !a_renouveller.is_empty() {
            col = col.push(section_sante(
                crate::tr!("À renouveler ({n})", n = a_renouveller.len()).to_string(),
                String::new(),
                a_renouveller,
                "label",
            ));
        }
        col = col.push(
            text(crate::i18n::tr("Fuites (Have I Been Pwned) : vérification côté web en attendant le port au démon.").to_string())
                .font(app.font)
                .size(11)
                .color(dim),
        );
        col.into()
    }

    fn chiffre_carte(&self, app: &Habillage, nombre: &str, libelle: &str) -> Element<'_, Msg> {
        let th = &app.cfg.general.theme;
        let r = crate::theme::readable(th.fg, th.bg, th.accent);
        let fg = color(r.fg, 1.0);
        let dim = color(r.dim, 1.0);
        let edge = color(r.edge, 1.0);
        container(
            column![
                text(nombre.to_string())
                    .font(app.title_font())
                    .size(26)
                    .color(fg),
                text(libelle.to_string()).font(app.font).size(11).color(dim),
            ]
            .spacing(2),
        )
        .padding(10)
        .width(Length::FillPortion(3))
        .style(move |_| container::Style {
            background: Some(Color { a: 0.05, ..fg }.into()),
            border: Border::default().rounded(10).width(1).color(edge),
            ..Default::default()
        })
        .into()
    }

    /* ── Onglet Réglages : verrouillage, export, synchro Drive (MLG-47) ── */

    fn vue_reglages(&self, app: &Habillage) -> Element<'_, Msg> {
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

fn mymfa_fenetre_champ(v: &Value, c: &str) -> String {
    v.get(c).and_then(Value::as_str).unwrap_or("").to_string()
}

/// Date du jour (UTC), pour le groupement d'activité.
fn aujourd_hui() -> String {
    mymfa_core_date()
}

fn mymfa_core_date() -> String {
    // Le démon et le web horodatent en ISO UTC : la date est les 10 premiers
    // caractères de l'instant ISO courant.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    let jours = secs.div_euclid(86_400);
    // Algorithme de Howard Hinnant (civil_from_days), comme mymfa-core.
    let z = jours + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

fn bascule(app: &Habillage, nom: &'static str, actif: bool, t: &str) -> Element<'static, Msg> {
    let th = &app.cfg.general.theme;
    let r = crate::theme::readable(th.fg, th.bg, th.accent);
    let fg = color(r.fg, 1.0);
    let accent = color(th.accent, 1.0);
    let edge = color(r.edge, 1.0);
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
        border: Border::default()
            .rounded(999)
            .width(1)
            .color(if actif { accent } else { edge }),
        ..Default::default()
    })
    .on_press(wrap(FMsg::Bascule(nom)))
    .into()
}

fn bouton_etoile_login(app: &Habillage, id: String, favori: bool) -> Element<'static, Msg> {
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
        .on_press(wrap(FMsg::FavoriLogin(id, !favori)))
        .into()
}

fn bouton_poubelle_login(app: &Habillage, id: String, _site: String) -> Element<'static, Msg> {
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
        .on_press(wrap(FMsg::SupprimerLogin(id)))
        .into()
}

/// Solidité d'un mot de passe (0-4, bits), réplique de l'heuristique du web
/// et du démon (`password_strength`) : pour guider la saisie, pas un oracle.
pub fn solidite(mot: &str) -> (u8, u32) {
    if mot.is_empty() {
        return (0, 0);
    }
    let mut pool: f64 = 0.0;
    if mot.chars().any(|c| c.is_ascii_lowercase()) {
        pool += 26.0;
    }
    if mot.chars().any(|c| c.is_ascii_uppercase()) {
        pool += 26.0;
    }
    if mot.chars().any(|c| c.is_ascii_digit()) {
        pool += 10.0;
    }
    if mot.chars().any(|c| !c.is_ascii_alphanumeric()) {
        pool += 33.0;
    }
    let mut bits = mot.chars().count() as f64 * pool.max(1.0).log2();
    let triple = mot
        .chars()
        .zip(mot.chars().skip(1))
        .zip(mot.chars().skip(2))
        .any(|((a, b), c)| a == b && b == c);
    if triple {
        bits *= 0.75;
    }
    let bas = mot.to_lowercase();
    for commun in [
        "password",
        "motdepasse",
        "azerty",
        "qwerty",
        "123456",
        "12345678",
        "admin",
        "bonjour",
        "soleil",
        "welcome",
    ] {
        if bas.contains(commun) {
            bits = bits.min(24.0);
        }
    }
    let b: Vec<u32> = mot.chars().map(|c| c as u32).collect();
    let suite = b.windows(3).any(|w| w[0] + 1 == w[1] && w[1] + 1 == w[2]);
    if suite {
        bits *= 0.7;
    }
    let niveau = if bits < 36.0 {
        1
    } else if bits < 60.0 {
        2
    } else if bits < 80.0 {
        3
    } else {
        4
    };
    (niveau, bits.round() as u32)
}

/// Décode le premier QR d'une image (chemin local) : renvoie son contenu.
fn decoder_image(chemin: &str) -> Result<String, String> {
    let img = image::open(chemin).map_err(|e| format!("image illisible : {e}"))?;
    let luma = img.to_luma8();
    let mut preparee = rqrr::PreparedImage::prepare(luma);
    for grille in preparee.detect_grids() {
        if let Ok((_, contenu)) = grille.decode() {
            return Ok(contenu);
        }
    }
    Err(crate::i18n::tr("Aucun code QR trouvé dans cette image.").to_string())
}

fn ok_vert(app: &Habillage) -> Color {
    let th = &app.cfg.general.theme;
    color(crate::theme::readable(th.fg, th.bg, th.accent).focus, 1.0)
}

fn f_meme_etiquette(f: &Filtre, t: &str) -> bool {
    matches!(f, Filtre::Etiquette(e) if e == t)
}

fn filtre_bouton(app: &Habillage, actif: bool, t: &str, act: FMsg) -> Element<'static, Msg> {
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

fn bouton_simple(app: &Habillage, t: &str, act: FMsg) -> Element<'static, Msg> {
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

fn bouton_etoile(app: &Habillage, id: String, favori: bool) -> Element<'static, Msg> {
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

fn bouton_poubelle(app: &Habillage, id: String, label: String) -> Element<'static, Msg> {
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

/* ───── Fenêtre d'application (processus à part) ─────────────────────────── */

/// La fenêtre myMFA hors de hyprdesk : mêmes vues, même logique, dans une
/// fenêtre d'application ordinaire.
struct Application {
    fenetre: Fenetre,
    hab: Habillage,
    theme: iced::Theme,
}

impl Application {
    fn boot(config_path: &std::path::Path) -> (Self, Task<Msg>) {
        let cfg = crate::config::lua::load_settings_only(config_path).unwrap_or_default();
        let theme = super::make_theme(&cfg);
        (
            Self {
                fenetre: Fenetre::neuf(Id::unique()),
                hab: Habillage::de_config(cfg),
                theme,
            },
            Task::done(Msg::MymfaF(FMsg::Tic)),
        )
    }

    fn update(&mut self, m: Msg) -> Task<Msg> {
        let Msg::MymfaF(m) = m else {
            return Task::none();
        };
        let t = self.fenetre.update(m);
        match self.fenetre.sortie.take() {
            None => t,
            Some(sortie) => {
                self.fenetre.effacer();
                if sortie == Sortie::Deverrouiller {
                    // Coffre verrouillé : la carte de hyprdesk le demande.
                    let _ = crate::control::send(crate::control::Command::Mymfa("unlock".into()));
                }
                iced::exit()
            }
        }
    }

    fn view(&self) -> Element<'_, Msg> {
        self.fenetre.view(&self.hab)
    }
}

/// `hyprdesk mymfa window` : la fenêtre myMFA en application (xdg-toplevel,
/// classe `hyprdesk-mymfa`), que hyprdesk fait lancer par Hyprland. Une seule
/// par session : une seconde ramène la première au premier plan.
pub fn run(config_path: std::path::PathBuf) -> iced::Result {
    let verrou = std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("hyprdesk-mymfa.lock");
    let Ok(_garde) = crate::single::acquire(&verrou) else {
        if let Some(adresse) = fenetre_ouverte() {
            let _ = crate::hypr::focus_window(&adresse);
        }
        return Ok(());
    };
    iced::application(
        move || Application::boot(&config_path),
        Application::update,
        Application::view,
    )
    .settings(iced::Settings {
        fonts: crate::fonts::bundled(),
        default_font: crate::fonts::DEFAULT,
        ..Default::default()
    })
    .title(|_: &Application| crate::i18n::tr("myMFA").to_string())
    .window(iced::window::Settings {
        size: iced::Size::new(WIDTH + 2.0 * MARGE, HEIGHT + 2.0 * MARGE),
        min_size: Some(iced::Size::new(640.0, 420.0)),
        platform_specific: iced::window::settings::PlatformSpecific {
            application_id: APP_ID.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .theme(|a: &Application| a.theme.clone())
    .run()
}
