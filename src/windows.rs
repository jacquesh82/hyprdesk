//! Acteur séquentiel : aucun IPC de déplacement ne bloque le rendu graphique.
//!
//! Plein bureau (MLG-55) : les fenêtres du workspace passent en flottant,
//! organisées sur l'écran ou à la place qu'elles avaient la dernière fois
//! sur ce workspace ; en quittant le mode, leurs positions sont mémorisées et
//! elles reprennent leur configuration d'origine.
use std::collections::{HashMap, HashSet};

use crate::hypr;
use tokio::sync::oneshot;

enum Request {
    Restore(Option<String>, oneshot::Sender<Result<(), String>>),
    /// Rend une seule fenêtre (par adresse), les autres restent de côté.
    RestoreOne(String, oneshot::Sender<Result<(), String>>),
    Submap(bool, oneshot::Sender<Result<(), String>>),
    /// Plein bureau : ces fenêtres passent en flottant (taille, centrées) ;
    /// `launched` : ouvertes depuis le bureau (déjà flottantes à leur lancement).
    Float(
        Vec<String>,
        (u32, u32),
        bool,
        oneshot::Sender<Result<(), String>>,
    ),
    /// Plein bureau sur un workspace : ses fenêtres passent en flottant,
    /// à la place mémorisée pour ce workspace, sinon organisées dans la zone
    /// utile de l'écran.
    Arrange(String, hypr::Rect, oneshot::Sender<Result<(), String>>),
    /// Sortie du plein bureau : positions mémorisées par workspace, puis les
    /// fenêtres qu'il a fait flotter reprennent leur configuration d'origine.
    Release(oneshot::Sender<Result<(), String>>),
    /// Bureau affiché ou arrêt : elles reviennent.
    Unpark(oneshot::Sender<Result<(), String>>),
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
            let mut desk = Floating::default();
            while let Ok(request) = rx.recv() {
                match request {
                    Request::Float(addresses, size, launched, reply) => {
                        let _ = reply.send(desk.float(addresses, size, launched));
                    }
                    Request::Arrange(workspace, area, reply) => {
                        let _ = reply.send(desk.arrange(&workspace, area));
                    }
                    Request::Release(reply) => {
                        let _ = reply.send(desk.release());
                    }
                    Request::Unpark(reply) => {
                        let _ = reply.send(hypr::unpark_desk_windows());
                    }
                    Request::Submap(active, reply) => {
                        hypr::set_submap(active.then_some(hypr::SUBMAP));
                        let _ = reply.send(Ok(()));
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
                Request::Restore(_, reply)
                | Request::RestoreOne(_, reply)
                | Request::Submap(_, reply)
                | Request::Float(_, _, _, reply)
                | Request::Arrange(_, _, reply)
                | Request::Release(reply)
                | Request::Unpark(reply) => {
                    let _ = reply.send(Err(crate::i18n::tr(
                        "gestionnaire de fenêtres occupé ou arrêté",
                    )
                    .into()));
                }
            }
        }
        rx
    }

    pub fn restore(&self, monitor: Option<&str>) -> oneshot::Receiver<Result<(), String>> {
        self.enqueue(|reply| Request::Restore(monitor.map(str::to_owned), reply))
    }

    pub fn restore_window(&self, address: &str) -> oneshot::Receiver<Result<(), String>> {
        self.enqueue(|reply| Request::RestoreOne(address.to_owned(), reply))
    }

    pub fn float(
        &self,
        addresses: Vec<String>,
        size: (u32, u32),
        launched: bool,
    ) -> oneshot::Receiver<Result<(), String>> {
        self.enqueue(|reply| Request::Float(addresses, size, launched, reply))
    }

    pub fn arrange(
        &self,
        workspace: String,
        area: hypr::Rect,
    ) -> oneshot::Receiver<Result<(), String>> {
        self.enqueue(|reply| Request::Arrange(workspace, area, reply))
    }

    pub fn release(&self) -> oneshot::Receiver<Result<(), String>> {
        self.enqueue(Request::Release)
    }

    pub fn unpark(&self) -> oneshot::Receiver<Result<(), String>> {
        self.enqueue(Request::Unpark)
    }

    pub fn submap(&self, active: bool) -> oneshot::Receiver<Result<(), String>> {
        self.enqueue(|reply| Request::Submap(active, reply))
    }
}

/// Écart entre les fenêtres organisées en plein bureau.
const GAP: i32 = 16;

/// Fenêtres du plein bureau (état de l'acteur).
#[derive(Default)]
struct Floating {
    /// Passées en flottant par le plein bureau : en mosaïque à la sortie.
    floated: HashSet<String>,
    /// Fenêtres affichées sur le bureau, et leur workspace.
    managed: HashMap<String, String>,
    /// Positions au moment de quitter le plein bureau, par workspace.
    memory: HashMap<String, HashMap<String, hypr::Rect>>,
}

impl Floating {
    fn clients() -> Result<Vec<hypr::Geometry>, String> {
        hyprpanel_core::ipc::query_json("clients")
            .map(|v| hypr::geometries(&v))
            .map_err(|e| e.to_string())
    }

    fn float(
        &mut self,
        addresses: Vec<String>,
        size: (u32, u32),
        launched: bool,
    ) -> Result<(), String> {
        let changed = hypr::float_windows(&addresses, size)?;
        if let Ok(list) = Self::clients() {
            for g in list.into_iter().filter(|g| addresses.contains(&g.address)) {
                self.managed.insert(g.address, g.workspace);
            }
        }
        self.floated
            .extend(if launched { addresses } else { changed });
        Ok(())
    }

    fn arrange(&mut self, workspace: &str, area: hypr::Rect) -> Result<(), String> {
        let list: Vec<hypr::Geometry> = Self::clients()?
            .into_iter()
            .filter(|g| g.workspace == workspace)
            .collect();
        let remembered = self.memory.get(workspace).cloned().unwrap_or_default();
        let mut errors = Vec::new();
        let mut fresh = Vec::new();
        for g in &list {
            self.managed.insert(g.address.clone(), g.workspace.clone());
            if !g.floating {
                if let Err(e) = hypr::set_floating(&g.address, true) {
                    errors.push(e);
                    continue;
                }
                self.floated.insert(g.address.clone());
            }
            match remembered.get(&g.address) {
                Some(rect) => {
                    if let Err(e) = hypr::place(&g.address, *rect) {
                        errors.push(e);
                    }
                }
                // Déjà flottante sans place mémorisée : elle garde la sienne.
                None if !g.floating => fresh.push(g.address.clone()),
                None => {}
            }
        }
        for (a, rect) in fresh.iter().zip(hypr::arrange(fresh.len(), area, GAP)) {
            if let Err(e) = hypr::place(a, rect) {
                errors.push(e);
            }
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join(" ; "))
        }
    }

    fn release(&mut self) -> Result<(), String> {
        let list = Self::clients().unwrap_or_default();
        // Positions mémorisées, par workspace, pour le prochain plein bureau.
        let managed = std::mem::take(&mut self.managed);
        for (address, workspace) in &managed {
            if let Some(g) = list
                .iter()
                .find(|g| &g.address == address && &g.workspace == workspace && g.floating)
            {
                self.memory
                    .entry(workspace.clone())
                    .or_default()
                    .insert(address.clone(), g.rect);
            }
        }
        // Fenêtres fermées depuis : oubliées.
        for places in self.memory.values_mut() {
            places.retain(|a, _| list.iter().any(|g| &g.address == a));
        }
        self.memory.retain(|_, places| !places.is_empty());
        let floated: Vec<String> = self.floated.drain().collect();
        hypr::tile_windows(&floated)
    }
}
