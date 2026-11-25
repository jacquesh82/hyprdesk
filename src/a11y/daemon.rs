//! Démarrage du pont dans le démon : il ne sert l'arbre que si un lecteur
//! d'écran le demande.
//!
//! `org.a11y.Bus.GetAddress` lance toujours le bus d'accessibilité : il ne dit
//! pas si un lecteur écoute. Le critère est `org.a11y.Status` (`IsEnabled`,
//! `ScreenReaderEnabled`), propriétés de `org.a11y.Bus` à `/org/a11y/bus`,
//! suivies en continu (Orca démarre souvent après le démon). Tant que ce
//! service n'a pas de propriétaire, rien n'est lu ni lancé : aucun coût sans
//! lecteur d'écran.

use std::time::Duration;

use iced::futures::StreamExt;
use zbus::zvariant::OwnedValue;

use super::bus::{self, Handle, REGISTRY};
use super::model::{Node, Role};

/// Service du bus d'accessibilité, sur le bus de session.
const A11Y_BUS: &str = "org.a11y.Bus";
const A11Y_PATH: &str = "/org/a11y/bus";
const STATUS: &str = "org.a11y.Status";

/// Où en est le pont.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Aucune connexion au bus d'accessibilité.
    Off,
    /// Objets servis, pas (ou plus) inscrits auprès du registre.
    Served,
    /// Objets servis et inscrits.
    Embedded,
}

/// Prochain geste du pont.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// Rien à faire.
    Stay,
    /// Se connecter au bus d'accessibilité et servir l'arbre.
    Connect,
    /// S'inscrire auprès du registre (`Socket.Embed`).
    Embed,
    /// Le registre est parti : l'inscription est perdue, les objets restent.
    Lost,
    /// Plus de lecteur d'écran : se désinscrire, retirer les objets, couper.
    Disconnect,
}

/// Décision pure : `enabled` (un lecteur d'écran est demandé) et `registry`
/// (le registre a un propriétaire sur le bus d'accessibilité).
pub fn next(phase: Phase, enabled: bool, registry: bool) -> Step {
    match (phase, enabled, registry) {
        (Phase::Off, false, _) => Step::Stay,
        (Phase::Off, true, _) => Step::Connect,
        (_, false, _) => Step::Disconnect,
        (Phase::Served, true, true) => Step::Embed,
        (Phase::Served, true, false) => Step::Stay,
        (Phase::Embedded, true, true) => Step::Stay,
        (Phase::Embedded, true, false) => Step::Lost,
    }
}

/// Le registre a changé de propriétaire (parti, revenu ou remplacé) :
/// l'inscription qu'il tenait est perdue ; [`next`] la refait s'il est là.
pub fn on_registry_change(phase: Phase) -> Phase {
    match phase {
        Phase::Embedded => Phase::Served,
        p => p,
    }
}

/// Arbre servi : pour l'instant la seule application ; les surfaces s'y
/// ajoutent aux étapes suivantes du plan.
fn tree() -> Node {
    Node::new(Role::Application, "hyprdesk")
}

/// Abonnement du démon : suit `org.a11y.Status` et tient le pont. Ne rend
/// aucun message.
pub fn stream() -> impl iced::futures::Stream<Item = ()> {
    iced::stream::channel(
        1,
        async move |_out: iced::futures::channel::mpsc::Sender<()>| {
            run().await;
        },
    )
}

async fn run() {
    let Ok(session) = zbus::Connection::session().await else {
        return;
    };
    let env = std::env::var("AT_SPI_BUS_ADDRESS")
        .ok()
        .filter(|a| !a.is_empty());
    run_on(session, env).await;
}

/// La boucle du pont, sur le bus de session `session` ; `env` est
/// `AT_SPI_BUS_ADDRESS`, s'il est posé.
async fn run_on(session: zbus::Connection, env: Option<String>) {
    let Ok(dbus) = zbus::fdo::DBusProxy::new(&session).await else {
        return;
    };
    let Ok(owners) = dbus
        .receive_name_owner_changed_with_args(&[(0, A11Y_BUS)])
        .await
    else {
        return;
    };
    let Ok(changes) = crate::sources::properties_changed(&session, A11Y_BUS, A11Y_PATH).await
    else {
        return;
    };
    let mut wake = iced::futures::stream::select(owners.map(|_| ()), changes.map(|_| ()));
    let mut phase = Phase::Off;
    let mut handle: Option<Handle> = None;
    let mut registry_events: Option<
        std::pin::Pin<Box<dyn iced::futures::Stream<Item = ()> + Send>>,
    > = None;
    loop {
        let enabled = enabled(&session, &dbus).await;
        let registry = match &handle {
            Some(h) => has_owner(&h.conn, REGISTRY).await,
            None => false,
        };
        match next(phase, enabled, registry) {
            Step::Stay => {
                let woke = match registry_events.as_mut() {
                    Some(r) => tokio::select! {
                        w = wake.next() => Wake::Status(w.is_some()),
                        r = r.next() => Wake::Registry(r.is_some()),
                    },
                    None => Wake::Status(wake.next().await.is_some()),
                };
                match woke {
                    Wake::Status(false) => return,
                    Wake::Status(true) => {}
                    Wake::Registry(true) => {
                        phase = on_registry_change(phase);
                        if phase == Phase::Served
                            && let Some(h) = handle.as_mut()
                        {
                            h.lost();
                        }
                    }
                    // Le bus d'accessibilité s'est fermé : tout est à refaire,
                    // sans s'emballer s'il se referme aussitôt.
                    Wake::Registry(false) => {
                        handle = None;
                        registry_events = None;
                        phase = Phase::Off;
                        pause(&mut wake).await;
                    }
                }
            }
            Step::Connect => match connect(&session, env.clone()).await {
                Ok(conn) => {
                    let Some(watch) = registry_watch(&conn).await else {
                        // Sans suivi du registre, une inscription ne serait
                        // jamais refaite : nouvel essai plus tard.
                        pause(&mut wake).await;
                        continue;
                    };
                    match bus::serve(conn, &tree()).await {
                        Ok(h) => {
                            tracing::info!(
                                "{}",
                                crate::i18n::tr("accessibilité : arbre servi aux lecteurs d'écran")
                            );
                            registry_events = Some(watch);
                            handle = Some(h);
                            phase = Phase::Served;
                        }
                        Err(e) => {
                            tracing::warn!("{}", crate::tr!("accessibilité : {e}", e = e));
                            pause(&mut wake).await;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("{}", crate::tr!("accessibilité : {e}", e = e));
                    pause(&mut wake).await;
                }
            },
            Step::Embed => {
                if let Some(h) = handle.as_mut() {
                    match h.embed().await {
                        Ok(()) => phase = Phase::Embedded,
                        Err(e) => {
                            tracing::warn!("{}", crate::tr!("accessibilité : {e}", e = e));
                            pause(&mut wake).await;
                        }
                    }
                }
            }
            Step::Lost => {
                if let Some(h) = handle.as_mut() {
                    h.lost();
                }
                phase = Phase::Served;
            }
            Step::Disconnect => {
                if let Some(mut h) = handle.take() {
                    if phase == Phase::Embedded {
                        let _ = h.unembed().await;
                    }
                    h.stop().await;
                }
                registry_events = None;
                phase = Phase::Off;
            }
        }
    }
}

/// Ce qui réveille la boucle.
enum Wake {
    /// `org.a11y.Bus` ou `org.a11y.Status` a changé (`false` : flux fini).
    Status(bool),
    /// Le registre est arrivé ou parti (`false` : bus d'accessibilité fermé).
    Registry(bool),
}

/// Après un échec : attendre un changement, une minute au plus, sans
/// s'emballer.
async fn pause(wake: &mut (impl iced::futures::Stream<Item = ()> + Unpin)) {
    let _ = tokio::time::timeout(Duration::from_secs(60), wake.next()).await;
}

/// Un lecteur d'écran est demandé : `org.a11y.Bus` a un propriétaire (le
/// lire ne le lance donc pas) et `IsEnabled` ou `ScreenReaderEnabled` vaut
/// vrai.
async fn enabled(session: &zbus::Connection, dbus: &zbus::fdo::DBusProxy<'_>) -> bool {
    if !has_owner_with(dbus, A11Y_BUS).await {
        return false;
    }
    // Sans activation : si le service part entre-temps, la lecture échoue
    // au lieu de le relancer.
    let Ok(props) = zbus::proxy::Builder::<zbus::Proxy<'_>>::new(session)
        .destination(A11Y_BUS)
        .and_then(|b| b.path(A11Y_PATH))
        .and_then(|b| b.interface("org.freedesktop.DBus.Properties"))
    else {
        return false;
    };
    let Ok(props) = props
        .cache_properties(zbus::proxy::CacheProperties::No)
        .build()
        .await
    else {
        return false;
    };
    for p in ["IsEnabled", "ScreenReaderEnabled"] {
        let v: Option<OwnedValue> = props
            .call_with_flags(
                "Get",
                zbus::proxy::MethodFlags::NoAutoStart.into(),
                &(STATUS, p),
            )
            .await
            .ok()
            .flatten();
        if v.and_then(|v| bool::try_from(v).ok()) == Some(true) {
            return true;
        }
    }
    false
}

async fn has_owner_with(dbus: &zbus::fdo::DBusProxy<'_>, name: &str) -> bool {
    match zbus::names::BusName::try_from(name) {
        Ok(n) => dbus.name_has_owner(n).await.unwrap_or(false),
        Err(_) => false,
    }
}

async fn has_owner(conn: &zbus::Connection, name: &str) -> bool {
    match zbus::fdo::DBusProxy::new(conn).await {
        Ok(d) => has_owner_with(&d, name).await,
        Err(_) => false,
    }
}

/// Adresse du bus d'accessibilité : `AT_SPI_BUS_ADDRESS` si elle est posée,
/// sinon `org.a11y.Bus.GetAddress`.
async fn connect(
    session: &zbus::Connection,
    env: Option<String>,
) -> zbus::Result<zbus::Connection> {
    let address = match env {
        Some(a) => a,
        None => session
            .call_method(Some(A11Y_BUS), A11Y_PATH, Some(A11Y_BUS), "GetAddress", &())
            .await?
            .body()
            .deserialize::<String>()?,
    };
    zbus::connection::Builder::address(address.as_str())?
        .build()
        .await
}

/// Arrivées et départs du registre, sur le bus d'accessibilité.
async fn registry_watch(
    conn: &zbus::Connection,
) -> Option<std::pin::Pin<Box<dyn iced::futures::Stream<Item = ()> + Send>>> {
    let dbus = zbus::fdo::DBusProxy::new(conn).await.ok()?;
    let s = dbus
        .receive_name_owner_changed_with_args(&[(0, REGISTRY)])
        .await
        .ok()?;
    Some(Box::pin(s.map(|_| ())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pont_seulement_si_un_lecteur_le_demande() {
        // Sans lecteur d'écran : rien, quel que soit le registre.
        assert_eq!(next(Phase::Off, false, false), Step::Stay);
        assert_eq!(next(Phase::Off, false, true), Step::Stay);
        // Un lecteur arrive : connexion, puis inscription.
        assert_eq!(next(Phase::Off, true, true), Step::Connect);
        assert_eq!(next(Phase::Served, true, true), Step::Embed);
        assert_eq!(next(Phase::Embedded, true, true), Step::Stay);
    }

    #[test]
    fn registre_parti_puis_revenu() {
        assert_eq!(next(Phase::Embedded, true, false), Step::Lost);
        assert_eq!(next(Phase::Served, true, false), Step::Stay);
        assert_eq!(next(Phase::Served, true, true), Step::Embed);
    }

    #[test]
    fn registre_remplace_d_un_coup() {
        // Un seul `NameOwnerChanged` (mort et renaissance) : l'inscription
        // est perdue, `next` la refait.
        let p = on_registry_change(Phase::Embedded);
        assert_eq!(p, Phase::Served);
        assert_eq!(next(p, true, true), Step::Embed);
        assert_eq!(on_registry_change(Phase::Off), Phase::Off);
    }

    #[test]
    fn lecteur_coupe() {
        assert_eq!(next(Phase::Embedded, false, true), Step::Disconnect);
        assert_eq!(next(Phase::Served, false, false), Step::Disconnect);
    }

    /// Faux `org.a11y.Bus` : l'adresse rendue est celle du bus de test.
    struct FakeBus {
        address: String,
    }

    #[zbus::interface(name = "org.a11y.Bus")]
    impl FakeBus {
        fn get_address(&self) -> String {
            self.address.clone()
        }
    }

    struct FakeStatus {
        enabled: bool,
    }

    #[zbus::interface(name = "org.a11y.Status")]
    impl FakeStatus {
        #[zbus(property)]
        fn is_enabled(&self) -> bool {
            self.enabled
        }

        #[zbus(property)]
        fn screen_reader_enabled(&self) -> bool {
            false
        }
    }

    /// Attend que `slot` soit rempli, cinq secondes au plus.
    async fn until<T: Clone>(slot: &std::sync::Mutex<Option<T>>) -> Option<T> {
        for _ in 0..100 {
            if let Some(v) = slot.lock().unwrap().clone() {
                return Some(v);
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        None
    }

    #[tokio::test]
    #[ignore = "exécuter sous dbus-run-session, jamais sur le bus de la session utilisateur"]
    async fn pont_suit_le_lecteur_d_ecran() {
        use crate::a11y::bus::tests::FakeRegistry;
        use crate::a11y::model::ROOT_PATH;
        use std::sync::{Arc, Mutex};

        let address = std::env::var("DBUS_SESSION_BUS_ADDRESS").unwrap();
        // Le registre et le bus d'accessibilité, sur le bus de test.
        let reg = zbus::Connection::session().await.unwrap();
        let seen: Arc<Mutex<Option<bus::Ref>>> = Default::default();
        let gone: Arc<Mutex<Option<bus::Ref>>> = Default::default();
        reg.object_server()
            .at(
                ROOT_PATH,
                FakeRegistry {
                    seen: seen.clone(),
                    gone: gone.clone(),
                    name: reg.unique_name().unwrap().to_string(),
                },
            )
            .await
            .unwrap();
        reg.request_name(REGISTRY).await.unwrap();
        let status = zbus::Connection::session().await.unwrap();
        status
            .object_server()
            .at(A11Y_PATH, FakeBus { address })
            .await
            .unwrap();
        status
            .object_server()
            .at(A11Y_PATH, FakeStatus { enabled: false })
            .await
            .unwrap();

        // Aucun lecteur : le pont ne sert rien.
        let session = zbus::Connection::session().await.unwrap();
        tokio::spawn(run_on(session, None));
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert!(seen.lock().unwrap().is_none());

        // `org.a11y.Bus` arrive, lecteur demandé : le pont s'inscrit.
        let iface = status
            .object_server()
            .interface::<_, FakeStatus>(A11Y_PATH)
            .await
            .unwrap();
        iface.get_mut().await.enabled = true;
        status.request_name(A11Y_BUS).await.unwrap();
        let plug = until(&seen).await.expect("Embed reçu");
        assert_eq!(plug.1.as_str(), ROOT_PATH);

        // Le registre renaît sur une autre connexion : nouvel `Embed`.
        reg.release_name(REGISTRY).await.unwrap();
        drop(reg);
        let again: Arc<Mutex<Option<bus::Ref>>> = Default::default();
        let reg2 = zbus::Connection::session().await.unwrap();
        reg2.object_server()
            .at(
                ROOT_PATH,
                FakeRegistry {
                    seen: again.clone(),
                    gone: gone.clone(),
                    name: reg2.unique_name().unwrap().to_string(),
                },
            )
            .await
            .unwrap();
        reg2.request_name(REGISTRY).await.unwrap();
        assert_eq!(until(&again).await, Some(plug.clone()), "nouvel Embed");

        // Lecteur coupé : `PropertiesChanged`, le pont se désinscrit.
        iface.get_mut().await.enabled = false;
        iface
            .get()
            .await
            .is_enabled_changed(iface.signal_emitter())
            .await
            .unwrap();
        assert_eq!(until(&gone).await, Some(plug), "Unembed reçu");
    }
}
