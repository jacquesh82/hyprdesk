//! Démon de notifications freedesktop (`org.freedesktop.Notifications`,
//! spécification 1.2) : les applications envoient leurs notifications à
//! hyprdesk, qui les affiche (bulles, centre de notifications) et renvoie
//! les actions choisies et les fermetures.
//!
//! Le nom n'est pris que s'il est libre (`DoNotQueue`) : un démon déjà en
//! place (wayle, mako, dunst, swaync…) reste maître, hyprdesk n'insiste pas.
//!
//! Tranche A (MLG-6) : le serveur et son état ; l'interface (bulles, module
//! de la barre, historique, « Ne pas déranger ») le branche à la tranche B.
#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use zbus::object_server::SignalEmitter;
use zbus::zvariant::OwnedValue;

pub const NAME: &str = "org.freedesktop.Notifications";
pub const PATH: &str = "/org/freedesktop/Notifications";
/// Notifications gardées pour l'historique.
pub const HISTORY: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Urgency {
    Low,
    #[default]
    Normal,
    Critical,
}

/// Raison de fermeture (spécification) : expirée, fermée par l'utilisateur,
/// par `CloseNotification`, autre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    Expired = 1,
    Dismissed = 2,
    Closed = 3,
    Undefined = 4,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Notification {
    pub id: u32,
    pub app: String,
    /// Icône : nom du thème ou chemin (`app_icon`, sinon les hints image).
    pub icon: String,
    pub summary: String,
    pub body: String,
    /// (clé, libellé) ; `default` : action d'un clic sur la notification.
    pub actions: Vec<(String, String)>,
    pub urgency: Urgency,
    /// Délai demandé (ms) : -1 au choix du serveur, 0 jamais.
    pub timeout_ms: i32,
    /// Reste après une action (`resident`).
    pub resident: bool,
    /// Pas d'historique (`transient`).
    pub transient: bool,
    /// Application d'origine (`desktop-entry`).
    pub desktop_entry: Option<String>,
    pub category: Option<String>,
}

/// Ce que le serveur signale à l'interface.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// Nouvelle notification, ou remplacement (même id).
    Shown(Notification),
    /// Fermée par l'application (`CloseNotification`).
    Closed(u32),
}

fn hint_str(h: &HashMap<String, OwnedValue>, k: &str) -> Option<String> {
    h.get(k)
        .and_then(|v| String::try_from(v.try_clone().ok()?).ok())
        .filter(|s| !s.is_empty())
}

fn hint_bool(h: &HashMap<String, OwnedValue>, k: &str) -> bool {
    h.get(k)
        .and_then(|v| bool::try_from(v.try_clone().ok()?).ok())
        .unwrap_or(false)
}

/// Urgence : octet 0, 1, 2 (certaines applications envoient un autre entier).
fn hint_urgency(h: &HashMap<String, OwnedValue>) -> Urgency {
    let n = h
        .get("urgency")
        .and_then(|v| {
            let byte = u8::try_from(v.try_clone().ok()?).ok().map(i64::from);
            byte.or_else(|| i32::try_from(v.try_clone().ok()?).ok().map(i64::from))
        })
        .unwrap_or(1);
    match n {
        0 => Urgency::Low,
        2 => Urgency::Critical,
        _ => Urgency::Normal,
    }
}

/// Liste plate [clé, libellé, clé, libellé…] → paires (une clé sans libellé
/// en fin de liste est ignorée).
pub fn parse_actions(list: &[String]) -> Vec<(String, String)> {
    list.as_chunks::<2>()
        .0
        .iter()
        .map(|[k, l]| (k.clone(), l.clone()))
        .collect()
}

/// Construit la notification reçue par `Notify`.
#[allow(clippy::too_many_arguments)]
pub fn build(
    id: u32,
    app: &str,
    app_icon: &str,
    summary: &str,
    body: &str,
    actions: &[String],
    hints: &HashMap<String, OwnedValue>,
    timeout_ms: i32,
) -> Notification {
    let icon = Some(app_icon.to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| hint_str(hints, "image-path"))
        .or_else(|| hint_str(hints, "image_path"))
        .unwrap_or_default();
    Notification {
        id,
        app: app.to_string(),
        icon,
        summary: summary.to_string(),
        body: body.to_string(),
        actions: parse_actions(actions),
        urgency: hint_urgency(hints),
        timeout_ms,
        resident: hint_bool(hints, "resident"),
        transient: hint_bool(hints, "transient"),
        desktop_entry: hint_str(hints, "desktop-entry"),
        category: hint_str(hints, "category"),
    }
}

/// Notifications ouvertes et historique, partagés avec l'interface.
#[derive(Debug, Default)]
pub struct Store {
    /// Les plus récentes en dernier.
    pub list: Vec<Notification>,
}

impl Store {
    /// Ajoute ou remplace (même id), en gardant au plus `HISTORY`.
    pub fn put(&mut self, n: Notification) {
        if let Some(old) = self.list.iter_mut().find(|o| o.id == n.id) {
            *old = n;
            return;
        }
        self.list.push(n);
        let extra = self.list.len().saturating_sub(HISTORY);
        self.list.drain(..extra);
    }

    pub fn remove(&mut self, id: u32) -> bool {
        let before = self.list.len();
        self.list.retain(|n| n.id != id);
        before != self.list.len()
    }
}

/// Le serveur publié sur le bus de session.
pub struct Server {
    next: AtomicU32,
    pub store: Arc<Mutex<Store>>,
    tx: tokio::sync::mpsc::UnboundedSender<Event>,
}

impl Server {
    pub fn new(tx: tokio::sync::mpsc::UnboundedSender<Event>) -> Server {
        Server {
            next: AtomicU32::new(1),
            store: Default::default(),
            tx,
        }
    }

    fn store(&self) -> std::sync::MutexGuard<'_, Store> {
        self.store.lock().unwrap_or_else(|e| e.into_inner())
    }
}

#[zbus::interface(name = "org.freedesktop.Notifications")]
impl Server {
    #[allow(clippy::too_many_arguments)]
    async fn notify(
        &self,
        app_name: String,
        replaces_id: u32,
        app_icon: String,
        summary: String,
        body: String,
        actions: Vec<String>,
        hints: HashMap<String, OwnedValue>,
        expire_timeout: i32,
    ) -> u32 {
        // Remplacement seulement d'une notification connue ; sinon, nouvel id.
        let id = if replaces_id != 0 && self.store().list.iter().any(|n| n.id == replaces_id) {
            replaces_id
        } else {
            self.next.fetch_add(1, Ordering::Relaxed)
        };
        let n = build(
            id,
            &app_name,
            &app_icon,
            &summary,
            &body,
            &actions,
            &hints,
            expire_timeout,
        );
        self.store().put(n.clone());
        let _ = self.tx.send(Event::Shown(n));
        id
    }

    async fn close_notification(
        &self,
        id: u32,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> zbus::fdo::Result<()> {
        if self.store().remove(id) {
            let _ = self.tx.send(Event::Closed(id));
            Self::notification_closed(&emitter, id, Reason::Closed as u32).await?;
        }
        Ok(())
    }

    fn get_capabilities(&self) -> Vec<String> {
        ["actions", "body", "icon-static", "persistence"]
            .map(String::from)
            .to_vec()
    }

    fn get_server_information(&self) -> (String, String, String, String) {
        (
            "hyprdesk".into(),
            "LeLab".into(),
            env!("CARGO_PKG_VERSION").into(),
            "1.2".into(),
        )
    }

    #[zbus(signal)]
    pub async fn notification_closed(
        emitter: &SignalEmitter<'_>,
        id: u32,
        reason: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn action_invoked(
        emitter: &SignalEmitter<'_>,
        id: u32,
        action_key: &str,
    ) -> zbus::Result<()>;
}

/// Serveur en place : connexion, état partagé, événements pour l'interface.
pub struct Handle {
    pub conn: zbus::Connection,
    pub store: Arc<Mutex<Store>>,
    pub events: tokio::sync::mpsc::UnboundedReceiver<Event>,
}

/// Publie le serveur et prend le nom s'il est libre ; `Ok(None)` si un autre
/// démon de notifications est déjà en place.
pub async fn start(conn: zbus::Connection) -> zbus::Result<Option<Handle>> {
    let (tx, events) = tokio::sync::mpsc::unbounded_channel();
    let server = Server::new(tx);
    let store = server.store.clone();
    conn.object_server().at(PATH, server).await?;
    let r = conn
        .request_name_with_flags(NAME, zbus::fdo::RequestNameFlags::DoNotQueue.into())
        .await;
    match r {
        Ok(zbus::fdo::RequestNameReply::PrimaryOwner)
        | Ok(zbus::fdo::RequestNameReply::AlreadyOwner) => Ok(Some(Handle {
            conn,
            store,
            events,
        })),
        Ok(_) | Err(zbus::Error::NameTaken) => {
            let _ = conn.object_server().remove::<Server, _>(PATH).await;
            Ok(None)
        }
        Err(e) => Err(e),
    }
}

impl Handle {
    fn emitter(&self) -> zbus::Result<SignalEmitter<'_>> {
        SignalEmitter::new(&self.conn, PATH)
    }

    /// L'utilisateur choisit une action ; la notification se ferme sauf
    /// si elle est `resident`.
    pub async fn invoke(&self, id: u32, key: &str) -> zbus::Result<()> {
        let e = self.emitter()?;
        Server::action_invoked(&e, id, key).await?;
        let resident = self
            .store
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .list
            .iter()
            .any(|n| n.id == id && n.resident);
        if !resident {
            self.dismiss(id, Reason::Dismissed).await?;
        }
        Ok(())
    }

    /// Ferme une notification (expirée, ou par l'utilisateur).
    pub async fn dismiss(&self, id: u32, reason: Reason) -> zbus::Result<()> {
        let known = self
            .store
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(id);
        if known {
            let e = self.emitter()?;
            Server::notification_closed(&e, id, reason as u32).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hints(pairs: &[(&str, OwnedValue)]) -> HashMap<String, OwnedValue> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.try_clone().unwrap()))
            .collect()
    }

    #[test]
    fn notification_construite() {
        let h = hints(&[
            ("urgency", OwnedValue::from(2u8)),
            ("resident", OwnedValue::from(true)),
            (
                "desktop-entry",
                OwnedValue::try_from(zbus::zvariant::Value::from("firefox")).unwrap(),
            ),
            (
                "image-path",
                OwnedValue::try_from(zbus::zvariant::Value::from("/tmp/a.png")).unwrap(),
            ),
        ]);
        let acts = [
            "default".to_string(),
            "Ouvrir".into(),
            "reply".into(),
            "Répondre".into(),
            "orphelin".into(),
        ];
        let n = build(7, "Firefox", "", "Titre", "Corps", &acts, &h, -1);
        assert_eq!(n.urgency, Urgency::Critical);
        assert!(n.resident && !n.transient);
        assert_eq!(n.icon, "/tmp/a.png");
        assert_eq!(n.desktop_entry.as_deref(), Some("firefox"));
        assert_eq!(
            n.actions,
            vec![
                ("default".into(), "Ouvrir".into()),
                ("reply".into(), "Répondre".into())
            ]
        );
        let n = build(
            8,
            "x",
            "dialog-information",
            "s",
            "b",
            &[],
            &HashMap::new(),
            0,
        );
        assert_eq!(
            (n.urgency, n.icon.as_str()),
            (Urgency::Normal, "dialog-information")
        );
    }

    #[test]
    fn historique_borne_et_remplacement() {
        let mut s = Store::default();
        for id in 1..=(HISTORY as u32 + 5) {
            s.put(build(id, "a", "", "s", "b", &[], &HashMap::new(), -1));
        }
        assert_eq!(s.list.len(), HISTORY);
        assert_eq!(s.list.first().unwrap().id, 6);
        let last = s.list.last().unwrap().id;
        s.put(build(
            last,
            "a",
            "",
            "remplacée",
            "b",
            &[],
            &HashMap::new(),
            -1,
        ));
        assert_eq!(s.list.len(), HISTORY);
        assert_eq!(s.list.last().unwrap().summary, "remplacée");
        assert!(s.remove(last) && !s.remove(last));
    }

    async fn notify(app: &zbus::Connection, replaces: u32, summary: &str) -> u32 {
        let body = (
            "essai",
            replaces,
            "",
            summary,
            "corps",
            vec!["default", "Ouvrir"],
            HashMap::<&str, zbus::zvariant::Value>::new(),
            -1i32,
        );
        app.call_method(Some(NAME), PATH, Some(NAME), "Notify", &body)
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap()
    }

    #[tokio::test]
    #[ignore = "exécuter sous dbus-run-session, jamais sur le bus de la session utilisateur"]
    async fn serveur_sur_le_bus() {
        let conn = zbus::Connection::session().await.unwrap();
        let mut h = start(conn).await.unwrap().expect("nom libre");
        let app = zbus::Connection::session().await.unwrap();
        let call = |m: &'static str| app.call_method(Some(NAME), PATH, Some(NAME), m, &());
        let info: (String, String, String, String) = call("GetServerInformation")
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        assert_eq!(info.0, "hyprdesk");
        let caps: Vec<String> = call("GetCapabilities")
            .await
            .unwrap()
            .body()
            .deserialize()
            .unwrap();
        assert!(caps.contains(&"actions".to_string()));
        let id: u32 = notify(&app, 0, "un").await;
        let Some(Event::Shown(n)) = h.events.recv().await else {
            panic!()
        };
        assert_eq!((n.id, n.summary.as_str()), (id, "un"));
        let again: u32 = notify(&app, id, "deux").await;
        assert_eq!(again, id, "remplacement");
        let Some(Event::Shown(n)) = h.events.recv().await else {
            panic!()
        };
        assert_eq!(n.summary, "deux");
        app.call_method(Some(NAME), PATH, Some(NAME), "CloseNotification", &(id,))
            .await
            .unwrap();
        assert_eq!(h.events.recv().await, Some(Event::Closed(id)));
        assert!(h.store.lock().unwrap().list.is_empty());
        // Un second serveur trouve le nom pris : il s'efface.
        let other = zbus::Connection::session().await.unwrap();
        assert!(start(other).await.unwrap().is_none());
        // Action puis fermeture (non résidente).
        let id: u32 = notify(&app, 0, "trois").await;
        let _ = h.events.recv().await;
        h.invoke(id, "default").await.unwrap();
        assert!(h.store.lock().unwrap().list.is_empty());
    }
}
