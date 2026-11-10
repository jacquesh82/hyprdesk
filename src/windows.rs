//! Acteur séquentiel : aucun IPC de déplacement ne bloque le rendu graphique.
use crate::hypr::{self, Stashed};
use tokio::sync::oneshot;

enum Request {
    Stash(Vec<Stashed>, oneshot::Sender<Result<(), String>>),
    Restore(Option<String>, oneshot::Sender<Result<(), String>>),
    /// Rend une seule fenêtre (par adresse), les autres restent de côté.
    RestoreOne(String, oneshot::Sender<Result<(), String>>),
    Submap(bool, oneshot::Sender<Result<(), String>>),
}

pub struct WindowManager(std::sync::mpsc::SyncSender<Request>);

impl WindowManager {
    pub fn disabled() -> Self {
        let (tx, _) = std::sync::mpsc::sync_channel(1);
        Self(tx)
    }
    pub fn spawn() -> Self {
        let (tx, rx) = std::sync::mpsc::sync_channel(64);
        std::thread::Builder::new().name("hyprdesk-windows".into()).spawn(move || {
            let mut stash = match hypr::load_stash() {
                Ok(stash) => stash,
                Err(e) => {
                    tracing::error!("{}", crate::tr!("journal des fenêtres conservé, gestionnaire suspendu : {e}", e = e));
                    return;
                }
            };
            while let Ok(request) = rx.recv() {
                match request {
                    Request::Submap(active, reply) => {
                        hypr::set_submap(active.then_some(hypr::SUBMAP));
                        let _ = reply.send(Ok(()));
                    }
                    Request::Stash(picked, reply) => {
                        let mut next = stash.clone();
                        // Une adresse de retour vide ou égale au workspace de côté rendrait la fenêtre introuvable.
                        let picked: Vec<_> = picked.into_iter().filter(|s| !s.workspace.is_empty() && s.workspace != hypr::STASH && !stash.iter().any(|o| o.address == s.address)).collect();
                        next.extend(picked.iter().cloned());
                        // Ne jamais déplacer une fenêtre dont la destination de retour n'est pas enregistrée.
                        let result = hypr::save_stash(&next).map_err(|e| e.to_string()).and_then(|()| {
                            stash = next;
                            let mut errors = Vec::new();
                            for s in picked {
                                if let Err(e) = hypr::move_window(&s.address, hypr::STASH) { errors.push(e.to_string()); }
                            }
                            hypr::close_stash_view();
                            if errors.is_empty() { Ok(()) } else { Err(errors.join(" ; ")) }
                        });
                        let _ = reply.send(result);
                    }
                    Request::RestoreOne(address, reply) => {
                        let (back, mut keep): (Vec<_>, Vec<_>) = stash.iter().cloned().partition(|s| s.address == address);
                        let pending = hypr::restore(&back);
                        let unresolved = pending.len();
                        keep.extend(pending);
                        stash = keep;
                        let result = hypr::save_stash(&stash).map_err(|e| e.to_string()).and_then(|()| {
                            if unresolved == 0 { Ok(()) } else { Err(crate::tr!("fenêtre {address} en attente de restitution", address = address)) }
                        });
                        let _ = reply.send(result);
                    }
                    Request::Restore(monitor, reply) => {
                        let (back, mut keep): (Vec<_>, Vec<_>) = stash.iter().cloned().partition(|s| monitor.as_ref().is_none_or(|m| &s.monitor == m));
                        let pending = hypr::restore(&back);
                        let unresolved = pending.len();
                        keep.extend(pending);
                        stash = keep;
                        let result = hypr::save_stash(&stash).map_err(|e| e.to_string()).and_then(|()| {
                            if unresolved == 0 { Ok(()) } else { Err(crate::tr!("{unresolved} fenêtre(s) en attente de restitution ; journal conservé", unresolved = unresolved)) }
                        });
                        let _ = reply.send(result);
                    }
                }
            }
        }).expect("thread de gestion des fenêtres");
        Self(tx)
    }

    fn enqueue(
        &self,
        request: impl FnOnce(oneshot::Sender<Result<(), String>>) -> Request,
    ) -> oneshot::Receiver<Result<(), String>> {
        let (tx, rx) = oneshot::channel();
        if let Err(e) = self.0.try_send(request(tx)) {
            let (std::sync::mpsc::TrySendError::Full(request)
            | std::sync::mpsc::TrySendError::Disconnected(request)) = e;
            match request {
                Request::Stash(_, reply)
                | Request::Restore(_, reply)
                | Request::RestoreOne(_, reply)
                | Request::Submap(_, reply) => {
                    let _ = reply.send(Err(crate::i18n::tr(
                        "gestionnaire de fenêtres occupé ou arrêté",
                    )
                    .into()));
                }
            }
        }
        rx
    }

    pub fn stash(&self, picked: Vec<Stashed>) -> oneshot::Receiver<Result<(), String>> {
        self.enqueue(|reply| Request::Stash(picked, reply))
    }

    pub fn restore(&self, monitor: Option<&str>) -> oneshot::Receiver<Result<(), String>> {
        self.enqueue(|reply| Request::Restore(monitor.map(str::to_owned), reply))
    }

    pub fn restore_window(&self, address: &str) -> oneshot::Receiver<Result<(), String>> {
        self.enqueue(|reply| Request::RestoreOne(address.to_owned(), reply))
    }

    pub fn submap(&self, active: bool) -> oneshot::Receiver<Result<(), String>> {
        self.enqueue(|reply| Request::Submap(active, reply))
    }
}
