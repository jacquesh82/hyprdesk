//! Trousseau myMFA (MLG-44) : hyprdesk est l'interface du client Linux de
//! myMFA (dépôt mindlog.mymfa.clients, `linux/`), alternative à
//! gnome-keyring. Le démon `mymfa` tient le Secret Service des applications
//! et publie `today.mindlog.MyMfa1` sur le bus de session ; hyprdesk y
//! répond aux demandes de déverrouillage (signal `UnlockRequested`), propose
//! codes 2FA et mots de passe dans le lanceur (`:2fa`, `:mdp`), montre l'état
//! dans la barre et règle le tout dans Réglages → Mots de passe.
//!
//! Aucun mot de passe ne transite par hyprdesk hors de la saisie du coffre :
//! les codes et mots de passe sont copiés par le démon lui-même
//! (`CopyCode`, `CopyLogin`), le mot de passe marqué sensible et effacé
//! après 30 s.

use std::path::PathBuf;

pub const NAME: &str = "today.mindlog.MyMfa1";

#[zbus::proxy(
    interface = "today.mindlog.MyMfa1",
    default_service = "today.mindlog.MyMfa1",
    default_path = "/today/mindlog/MyMfa1"
)]
pub trait MyMfa1 {
    fn unlock(&self, password: &str) -> zbus::Result<()>;
    fn create_vault(&self, password: &str) -> zbus::Result<()>;
    fn install_vault(&self, path: &str, password: &str) -> zbus::Result<()>;
    fn lock(&self) -> zbus::Result<()>;
    #[allow(clippy::type_complexity)]
    fn codes(&self) -> zbus::Result<Vec<(String, String, String, String, String, u32, u32, bool)>>;
    fn search_logins(
        &self,
        query: &str,
    ) -> zbus::Result<Vec<(String, String, String, String, String)>>;
    fn copy_code(&self, id: &str) -> zbus::Result<()>;
    fn copy_login(&self, id: &str, field: &str) -> zbus::Result<()>;
    /// Tout le contenu du coffre ouvert, sans aucun secret (MLG-46).
    fn content(&self) -> zbus::Result<String>;
    /// Opération JSON du web (add_account, update_login, next_hotp…) : applique
    /// puis réécrit le coffre ; une opération refusée n'écrit rien.
    fn apply(&self, operation: &str) -> zbus::Result<String>;
    /// Aperçu d'un compte avant ajout : champs lus et code actuel. N'écrit rien.
    fn preview(&self, account: &str) -> zbus::Result<String>;
    /// Mot de passe d'un site, à la demande (comme « afficher » dans le web).
    fn reveal_password(&self, id: &str) -> zbus::Result<String>;
    /// Mot de passe généré : options `{"length","digits","symbols","words"}`.
    fn generate(&self, options: &str) -> zbus::Result<String>;
    /// Synchronisation Drive (LIN-06) : cycle immédiat et états.
    fn sync_now(&self) -> zbus::Result<String>;
    fn sync_status(&self) -> zbus::Result<String>;
    fn sync_journal(&self) -> zbus::Result<String>;
    fn drive_connect(&self) -> zbus::Result<String>;
    fn drive_disconnect(&self) -> zbus::Result<()>;
    fn restore_drive(&self, revision: &str, password: &str) -> zbus::Result<()>;
    #[zbus(property)]
    fn locked(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn vault_present(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn sealed(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn session_key(&self) -> zbus::Result<bool>;
    #[zbus(property)]
    fn secret_service(&self) -> zbus::Result<bool>;
    #[zbus(signal)]
    fn state_changed(&self, locked: bool) -> zbus::Result<()>;
    #[zbus(signal)]
    fn unlock_requested(&self, application: String) -> zbus::Result<()>;
    #[zbus(signal)]
    fn content_changed(&self) -> zbus::Result<()>;
}

/// État du trousseau, tel que hyprdesk le montre.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    /// Le client est installé (nom activable ou tenu sur le bus).
    pub installed: bool,
    pub vault: bool,
    pub locked: bool,
    /// Déverrouillage à la connexion en place (sceau de session).
    pub sealed: bool,
    /// Une clé de session (pam_mymfa) attend : le prochain déverrouillage,
    /// ou la création du coffre, la scellera.
    pub session_key: bool,
    /// Le démon tient org.freedesktop.secrets (sinon gnome-keyring, en général).
    pub secret_service: bool,
}

impl State {
    /// Mot pour la barre (fichier d'état lu par le module Lua).
    pub fn word(&self) -> &'static str {
        match self {
            s if !s.installed => "absent",
            s if !s.vault => "sans-coffre",
            s if s.locked => "verrouille",
            _ => "ouvert",
        }
    }
}

/// Code 2FA en cours.
#[derive(Debug, Clone, PartialEq)]
pub struct Code {
    pub id: String,
    pub issuer: String,
    pub account: String,
    pub code: String,
    pub remaining: u32,
}

impl Code {
    pub fn label(&self) -> String {
        match (self.issuer.is_empty(), self.account.is_empty()) {
            (false, false) => format!("{} — {}", self.issuer, self.account),
            (false, true) => self.issuer.clone(),
            _ => self.account.clone(),
        }
    }
}

/// Login d'un site (jamais son mot de passe).
#[derive(Debug, Clone, PartialEq)]
pub struct Login {
    pub id: String,
    pub title: String,
    pub host: String,
    pub username: String,
}

impl Login {
    pub fn label(&self) -> String {
        let site = if self.title.is_empty() {
            &self.host
        } else {
            &self.title
        };
        if self.username.is_empty() {
            site.clone()
        } else {
            format!("{site} — {}", self.username)
        }
    }
}

/// Mode du lanceur : `:2fa …` (codes) ou `:mdp …` (mots de passe), et la
/// requête qui suit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Codes,
    Logins,
}

pub fn query(q: &str) -> Option<(Mode, &str)> {
    let t = q.trim_start();
    for (prefix, mode) in [(":2fa", Mode::Codes), (":mdp", Mode::Logins)] {
        if let Some(rest) = t.strip_prefix(prefix)
            && (rest.is_empty() || rest.starts_with(' '))
        {
            return Some((mode, rest.trim()));
        }
    }
    None
}

/// Message lisible d'une erreur du démon (ses erreurs portent un texte
/// français affichable).
pub fn error_text(e: &zbus::Error) -> String {
    match e {
        zbus::Error::MethodError(_, Some(msg), _) => msg.clone(),
        zbus::Error::MethodError(name, None, _) => name.to_string(),
        other => other.to_string(),
    }
}

/// Message d'une erreur de création : un démon d'avant CreateVault /
/// InstallVault (binaire remplacé sans relance) est désigné comme tel.
fn create_error_text(e: &zbus::Error) -> String {
    match e {
        zbus::Error::MethodError(name, _, _)
            if name.as_str() == "org.freedesktop.DBus.Error.UnknownMethod" =>
        {
            crate::i18n::tr(
                "Le démon myMFA en service est trop ancien : mettez à jour le client (linux/scripts/installer.sh), puis réessayez.",
            )
            .to_string()
        }
        other => error_text(other),
    }
}

/// Mauvais mot de passe (`today.mindlog.MyMfa1.Error.WrongPassword`).
pub fn is_wrong_password(e: &zbus::Error) -> bool {
    matches!(e, zbus::Error::MethodError(name, _, _) if name.as_str().ends_with(".WrongPassword"))
}

async fn proxy() -> zbus::Result<MyMfa1Proxy<'static>> {
    let conn = zbus::Connection::session().await?;
    MyMfa1Proxy::new(&conn).await
}

/// Le client est installé : nom tenu, ou activable par le bus.
pub async fn installed(conn: &zbus::Connection) -> bool {
    let Ok(dbus) = zbus::fdo::DBusProxy::new(conn).await else {
        return false;
    };
    let Ok(name) = zbus::names::BusName::try_from(NAME) else {
        return false;
    };
    if dbus.name_has_owner(name).await.unwrap_or(false) {
        return true;
    }
    dbus.list_activatable_names()
        .await
        .map(|l| l.iter().any(|n| n.as_str() == NAME))
        .unwrap_or(false)
}

/// État courant (le démon démarre au besoin, par activation).
pub async fn state_of(conn: &zbus::Connection) -> State {
    if !installed(conn).await {
        return State::default();
    }
    let Ok(p) = MyMfa1Proxy::new(conn).await else {
        return State {
            installed: true,
            ..Default::default()
        };
    };
    State {
        installed: true,
        vault: p.vault_present().await.unwrap_or(false),
        locked: p.locked().await.unwrap_or(true),
        sealed: p.sealed().await.unwrap_or(false),
        session_key: p.session_key().await.unwrap_or(false),
        secret_service: p.secret_service().await.unwrap_or(false),
    }
}

pub async fn state() -> State {
    match zbus::Connection::session().await {
        Ok(c) => state_of(&c).await,
        Err(_) => State::default(),
    }
}

/// Déverrouille ; `Ok(scellé)` (le démon scelle au passage si une clé de
/// session attendait), `Err((mauvais_mot_de_passe, message))`. Le mot de
/// passe est effacé après l'envoi.
pub async fn unlock(mut password: String) -> Result<bool, (bool, String)> {
    let r = match proxy().await {
        Ok(p) => p.unlock(&password).await.map(|()| p),
        Err(e) => Err(e),
    };
    crate::lock::pam::wipe(&mut password);
    let p = r.map_err(|e| (is_wrong_password(&e), error_text(&e)))?;
    Ok(p.sealed().await.unwrap_or(false))
}

/// Longueur minimale du mot de passe d'un nouveau coffre (celle du web).
pub const MIN_LEN: usize = 10;

/// Longueur comptée comme le web (unités UTF-16, `password.length`).
pub fn length(password: &str) -> usize {
    password.encode_utf16().count()
}

/// Crée le coffre de cet ordinateur ; le démon l'ouvre aussitôt et le
/// scelle si la session vient de pam_mymfa. Rend `Ok(scellé)`.
pub async fn create_vault(mut password: String) -> Result<bool, String> {
    let r = match proxy().await {
        Ok(p) => p.create_vault(&password).await.map(|()| p),
        Err(e) => Err(e),
    };
    crate::lock::pam::wipe(&mut password);
    let p = r.map_err(|e| create_error_text(&e))?;
    Ok(p.sealed().await.unwrap_or(false))
}

/// Installe la sauvegarde du web `path` avec son mot de passe ; comme
/// `create_vault` ensuite.
pub async fn install_vault(path: PathBuf, mut password: String) -> Result<bool, String> {
    let r = match proxy().await {
        Ok(p) => p
            .install_vault(&path.to_string_lossy(), &password)
            .await
            .map(|()| p),
        Err(e) => Err(e),
    };
    crate::lock::pam::wipe(&mut password);
    let p = r.map_err(|e| create_error_text(&e))?;
    Ok(p.sealed().await.unwrap_or(false))
}

pub async fn lock() -> Result<(), String> {
    proxy()
        .await
        .map_err(|e| error_text(&e))?
        .lock()
        .await
        .map_err(|e| error_text(&e))
}

pub async fn codes() -> Result<Vec<Code>, String> {
    let p = proxy().await.map_err(|e| error_text(&e))?;
    let list = p.codes().await.map_err(|e| error_text(&e))?;
    Ok(list
        .into_iter()
        .map(
            |(id, issuer, account, _tag, code, remaining, _period, _fav)| Code {
                id,
                issuer,
                account,
                code,
                remaining,
            },
        )
        .collect())
}

pub async fn logins(q: String) -> Result<Vec<Login>, String> {
    let p = proxy().await.map_err(|e| error_text(&e))?;
    let list = p.search_logins(&q).await.map_err(|e| error_text(&e))?;
    Ok(list
        .into_iter()
        .map(|(id, title, host, _url, username)| Login {
            id,
            title,
            host,
            username,
        })
        .collect())
}

/// Codes en cours avec tous leurs champs (étiquette, période, favori) —
/// la fenêtre myMFA filtre dessus (MLG-46).
pub type CodeComplet = (String, String, String, String, String, u32, u32, bool);

pub async fn codes_pleines() -> Result<Vec<CodeComplet>, String> {
    proxy()
        .await
        .map_err(|e| error_text(&e))?
        .codes()
        .await
        .map_err(|e| error_text(&e))
}

pub async fn copy_code(id: String) -> Result<(), String> {
    proxy()
        .await
        .map_err(|e| error_text(&e))?
        .copy_code(&id)
        .await
        .map_err(|e| error_text(&e))
}

/// `field` : `password` ou `username`.
pub async fn copy_login(id: String, field: &'static str) -> Result<(), String> {
    proxy()
        .await
        .map_err(|e| error_text(&e))?
        .copy_login(&id, field)
        .await
        .map_err(|e| error_text(&e))
}

/// Tout le contenu du coffre ouvert, en JSON, sans aucun secret. Verrouillé :
/// `Err` (la fenêtre propose le déverrouillage).
pub async fn content() -> Result<serde_json::Value, String> {
    let p = proxy().await.map_err(|e| error_text(&e))?;
    let texte = p.content().await.map_err(|e| error_text(&e))?;
    serde_json::from_str(&texte).map_err(|e| e.to_string())
}

/// Applique une opération JSON du web (`{"op": "add_account", …}`) et rend la
/// réponse (`{"id": …}`, `{"counter","code"}`…).
pub async fn apply(operation: serde_json::Value) -> Result<serde_json::Value, String> {
    let p = proxy().await.map_err(|e| error_text(&e))?;
    let texte = p
        .apply(&operation.to_string())
        .await
        .map_err(|e| error_text(&e))?;
    serde_json::from_str(&texte).map_err(|e| e.to_string())
}

/// Aperçu d'un compte avant ajout : champs lus et code actuel. N'écrit rien.
pub async fn preview(account: serde_json::Value) -> Result<serde_json::Value, String> {
    let p = proxy().await.map_err(|e| error_text(&e))?;
    let texte = p
        .preview(&account.to_string())
        .await
        .map_err(|e| error_text(&e))?;
    serde_json::from_str(&texte).map_err(|e| e.to_string())
}

/// Mot de passe d'un site (RevealPassword), à la demande.
/// (MLG-46, tranche suivante : onglets Mots de passe et restauration.)
#[allow(dead_code)]
pub async fn reveal_password(id: &str) -> Result<String, String> {
    proxy()
        .await
        .map_err(|e| error_text(&e))?
        .reveal_password(id)
        .await
        .map_err(|e| error_text(&e))
}

/// Mot de passe généré : `{"password", "strength": {"level","bits"}}`.
/// (MLG-46, tranche suivante : onglets Mots de passe et restauration.)
#[allow(dead_code)]
pub async fn generate(options: serde_json::Value) -> Result<serde_json::Value, String> {
    let p = proxy().await.map_err(|e| error_text(&e))?;
    let texte = p
        .generate(&options.to_string())
        .await
        .map_err(|e| error_text(&e))?;
    serde_json::from_str(&texte).map_err(|e| e.to_string())
}

/// Statut de la synchronisation Drive.
pub async fn sync_status() -> Result<serde_json::Value, String> {
    let p = proxy().await.map_err(|e| error_text(&e))?;
    let texte = p.sync_status().await.map_err(|e| error_text(&e))?;
    serde_json::from_str(&texte).map_err(|e| e.to_string())
}

/// Lance un cycle de synchronisation maintenant, rend son statut.
pub async fn sync_now() -> Result<serde_json::Value, String> {
    let p = proxy().await.map_err(|e| error_text(&e))?;
    let texte = p.sync_now().await.map_err(|e| error_text(&e))?;
    serde_json::from_str(&texte).map_err(|e| e.to_string())
}

/// Journal de synchro (arrivées, départs, conflits, horloges suspectes).
pub async fn sync_journal() -> Result<serde_json::Value, String> {
    let p = proxy().await.map_err(|e| error_text(&e))?;
    let texte = p.sync_journal().await.map_err(|e| error_text(&e))?;
    serde_json::from_str(&texte).map_err(|e| e.to_string())
}

/// Connecte Google Drive : le démon ouvre le navigateur (boucle locale + PKCE).
pub async fn drive_connect() -> Result<serde_json::Value, String> {
    let p = proxy().await.map_err(|e| error_text(&e))?;
    let texte = p.drive_connect().await.map_err(|e| error_text(&e))?;
    serde_json::from_str(&texte).map_err(|e| e.to_string())
}

pub async fn drive_disconnect() -> Result<(), String> {
    proxy()
        .await
        .map_err(|e| error_text(&e))?
        .drive_disconnect()
        .await
        .map_err(|e| error_text(&e))
}

/// Restaure une version gardée en point de restauration (mot de passe demandé).
/// (MLG-46, tranche suivante : onglets Mots de passe et restauration.)
#[allow(dead_code)]
pub async fn restore_drive(revision: &str, mut password: String) -> Result<(), String> {
    let r = match proxy().await {
        Ok(p) => p.restore_drive(revision, &password).await,
        Err(e) => Err(e),
    };
    crate::lock::pam::wipe(&mut password);
    r.map_err(|e| error_text(&e))
}

/// Fichier d'état lu par le module de barre : `$XDG_RUNTIME_DIR/hyprdesk/mymfa`.
pub fn state_file() -> Option<PathBuf> {
    std::env::var_os("XDG_RUNTIME_DIR").map(|d| PathBuf::from(d).join("hyprdesk").join("mymfa"))
}

pub fn write_state(s: &State) {
    if let Some(f) = state_file() {
        if let Some(d) = f.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        let _ = std::fs::write(f, format!("{}\n", s.word()));
    }
}

/// Unit systemd du démon : `hyprdesk.conf` y coupe l'invite de repli
/// (pinentry) puisque hyprdesk répond lui-même aux demandes.
fn dropin() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))?;
    Some(base.join("systemd/user/mymfa.service.d/hyprdesk.conf"))
}

/// Pose une fois le complément d'unit (pris en compte au prochain
/// démarrage du démon). Renvoie `true` s'il vient d'être écrit.
pub fn ensure_dropin() -> bool {
    let Some(f) = dropin() else {
        return false;
    };
    if f.exists() {
        return false;
    }
    let body = "# Posé par hyprdesk : il répond lui-même aux demandes de déverrouillage\n\
                # (signal UnlockRequested), l'invite de repli du démon est inutile.\n\
                [Service]\nEnvironment=MYMFA_PINENTRY=none\n";
    if let Some(d) = f.parent()
        && std::fs::create_dir_all(d).is_ok()
        && std::fs::write(&f, body).is_ok()
    {
        let _ = std::process::Command::new("systemctl")
            .args(["--user", "daemon-reload"])
            .status();
        return true;
    }
    false
}

/// gnome-keyring encore actif côté systemd (socket non masquée).
pub fn gnome_keyring_in_use() -> bool {
    std::process::Command::new("systemctl")
        .args(["--user", "is-enabled", "gnome-keyring-daemon.socket"])
        .output()
        .map(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            let s = s.trim();
            !s.is_empty() && s != "masked" && s != "not-found"
        })
        .unwrap_or(false)
        || std::process::Command::new("pgrep")
            .args(["-x", "gnome-keyring-d"])
            .status()
            .is_ok_and(|s| s.success())
}

/// Programme qui tient `org.freedesktop.secrets` : (nom, pid). Le nom de
/// processus tronqué de gnome-keyring (`gnome-keyring-d`) est rendu entier.
pub fn secret_service_holder() -> Option<(String, u32)> {
    let out = std::process::Command::new("busctl")
        .args(["--user", "status", "org.freedesktop.secrets"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let field = |k: &str| {
        text.lines()
            .find_map(|l| l.strip_prefix(k))
            .map(|v| v.trim().to_string())
    };
    let pid = field("PID=")?.parse().ok()?;
    let comm = field("Comm=")?;
    let name = if comm == "gnome-keyring-d" {
        "gnome-keyring-daemon".to_string()
    } else {
        comm
    };
    Some((name, pid))
}

/// Demande à LYRA : pourquoi `holder` tient le Secret Service à la place de
/// myMFA, et comment le lui rendre sans perdre de secret.
pub fn lyra_prompt(holder: &str, s: &State) -> String {
    let vault = if !s.vault {
        crate::i18n::tr("n'existe pas encore (hyprdesk propose de le créer)")
    } else if s.locked {
        crate::i18n::tr("existe mais est verrouillé")
    } else {
        crate::i18n::tr("existe et est ouvert")
    };
    crate::tr!(
        "Dans ma session Hyprland, le trousseau des applications (org.freedesktop.secrets) est tenu par {holder} au lieu du démon myMFA (mymfa.service, client Linux de myMFA), qui attend en file. Le coffre myMFA {vault}. Trouve ce qui lance {holder} (socket et service systemd --user, PAM de l'écran de connexion /etc/pam.d/greetd, démarrage automatique XDG, hyprland.lua…) et explique-le-moi. Puis propose la bascule vers myMFA sans perdre de secret : coffre créé et ouvert, import des secrets existants (mymfa import-gnome-keyring), arrêt et masquage de {holder}, et vérification avec mymfa status. Demande-moi avant de modifier quoi que ce soit, et ne touche jamais à inputs.lua ni à monitors.lua.",
        holder = holder,
        vault = vault
    )
}

/// Reprise de gnome-keyring : import de ses secrets (par le Secret
/// Service, pendant qu'il le tient encore), puis arrêt et masquage de ses
/// units ; le démon mymfa, en file, reprend aussitôt le nom.
pub fn take_over_gnome_keyring() -> Result<(), String> {
    let out = std::process::Command::new("mymfa")
        .arg("import-gnome-keyring")
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        let e = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(if e.is_empty() {
            crate::i18n::tr("échec de l'import de gnome-keyring").to_string()
        } else {
            e
        });
    }
    let st = std::process::Command::new("systemctl")
        .args([
            "--user",
            "mask",
            "--now",
            "gnome-keyring-daemon.service",
            "gnome-keyring-daemon.socket",
        ])
        .status()
        .map_err(|e| e.to_string())?;
    if !st.success() {
        return Err(crate::i18n::tr("gnome-keyring n'a pas pu être arrêté.").to_string());
    }
    // Lancé hors systemd (démarrage automatique XDG) : arrêté aussi.
    let _ = std::process::Command::new("pkill")
        .args(["-x", "gnome-keyring-d"])
        .status();
    Ok(())
}

/// Retour à gnome-keyring : ses units démasquées et relancées (le démon
/// mymfa repasse en file).
pub fn give_back_to_gnome_keyring() -> Result<(), String> {
    let st = std::process::Command::new("systemctl")
        .args([
            "--user",
            "unmask",
            "gnome-keyring-daemon.service",
            "gnome-keyring-daemon.socket",
        ])
        .status()
        .map_err(|e| e.to_string())?;
    let _ = std::process::Command::new("systemctl")
        .args(["--user", "restart", "mymfa.service"])
        .status();
    let _ = std::process::Command::new("systemctl")
        .args(["--user", "start", "gnome-keyring-daemon.socket"])
        .status();
    if st.success() {
        Ok(())
    } else {
        Err(crate::i18n::tr("gnome-keyring n'a pas pu être rétabli.").to_string())
    }
}

/// Sauvegarde du web à installer (`mymfa-vault*.json` dans Téléchargements).
pub fn find_backup() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let dirs = [
        std::process::Command::new("xdg-user-dir")
            .arg("DOWNLOAD")
            .output()
            .ok()
            .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
            .filter(|p| p.is_absolute()),
        Some(home.join("Téléchargements")),
        Some(home.join("Downloads")),
    ];
    let mut found: Vec<(std::time::SystemTime, PathBuf)> = dirs
        .into_iter()
        .flatten()
        .filter_map(|d| std::fs::read_dir(d).ok())
        .flat_map(|rd| rd.flatten())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("mymfa-vault") && n.ends_with(".json"))
        })
        .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
        .collect();
    found.sort();
    found.pop().map(|(_, p)| p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_du_lanceur() {
        assert_eq!(query(":2fa"), Some((Mode::Codes, "")));
        assert_eq!(query(" :2fa git"), Some((Mode::Codes, "git")));
        assert_eq!(query(":mdp github"), Some((Mode::Logins, "github")));
        assert_eq!(query(":2fax"), None);
        assert_eq!(query("firefox"), None);
    }

    #[test]
    fn mots_de_la_barre() {
        assert_eq!(State::default().word(), "absent");
        let mut s = State {
            installed: true,
            ..Default::default()
        };
        assert_eq!(s.word(), "sans-coffre");
        s.vault = true;
        s.locked = true;
        assert_eq!(s.word(), "verrouille");
        s.locked = false;
        assert_eq!(s.word(), "ouvert");
    }

    #[test]
    fn libelles() {
        let c = Code {
            id: "1".into(),
            issuer: "GitHub".into(),
            account: "jacques".into(),
            code: "123456".into(),
            remaining: 12,
        };
        assert_eq!(c.label(), "GitHub — jacques");
        let l = Login {
            id: "2".into(),
            title: String::new(),
            host: "example.org".into(),
            username: "moi".into(),
        };
        assert_eq!(l.label(), "example.org — moi");
    }

    #[test]
    fn longueur_comme_le_web() {
        assert_eq!(length("motdepasse"), 10);
        assert_eq!(length("é"), 1);
        assert_eq!(length("🔐"), 2);
    }
}
