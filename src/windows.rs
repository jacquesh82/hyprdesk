//! Acteur séquentiel : aucun IPC de déplacement ne bloque le rendu graphique.
//!
//! Plein bureau (MLG-55) : les fenêtres du workspace passent en flottant,
//! organisées sur l'écran ou à la place qu'elles avaient la dernière fois
//! sur ce workspace ; en quittant le mode, leurs positions sont mémorisées et
//! elles reprennent leur configuration d'origine.
//!
//! Une fenêtre que l'utilisateur a déplacée ou redimensionnée (elle n'est
//! plus là où hyprdesk l'a posée) n'est plus jamais réorganisée : sa place
//! devient une préférence, par workspace et par application, gardée d'une
//! session à l'autre (`~/.local/state/hyprdesk/window-places.json`) ;
//! Réglages → Bureau permet de les oublier.
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
    /// fenêtres qu'il a fait flotter reprennent leur configuration d'origine ;
    /// `Some(workspaces)` : seulement celles de ces workspaces (un écran).
    Release(Option<Vec<String>>, oneshot::Sender<Result<(), String>>),
    /// Bureau affiché ou arrêt : elles reviennent.
    Unpark(oneshot::Sender<Result<(), String>>),
    /// Oublie les places choisies par l'utilisateur (Réglages → Bureau).
    Forget(oneshot::Sender<Result<(), String>>),
}

/// Fichier des places choisies par l'utilisateur en plein bureau.
pub fn places_path() -> std::path::PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join(".local/state")
        })
        .join("hyprdesk")
        .join("window-places.json")
}

/// Places choisies : « workspace|classe » → (x, y, largeur, hauteur).
pub fn load_places() -> HashMap<String, hypr::Rect> {
    std::fs::read_to_string(places_path())
        .ok()
        .and_then(|s| serde_json::from_str::<HashMap<String, [i32; 4]>>(&s).ok())
        .map(|m| {
            m.into_iter()
                .map(|(k, r)| (k, (r[0], r[1], r[2], r[3])))
                .collect()
        })
        .unwrap_or_default()
}

fn save_places(places: &HashMap<String, hypr::Rect>) {
    let m: HashMap<&String, [i32; 4]> = places
        .iter()
        .map(|(k, r)| (k, [r.0, r.1, r.2, r.3]))
        .collect();
    let path = places_path();
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if let Ok(s) = serde_json::to_string_pretty(&m) {
        let _ = std::fs::write(path, s);
    }
}

/// Nombre de places choisies mémorisées (Réglages → Bureau).
pub fn places_count() -> usize {
    load_places().len()
}

fn place_key(workspace: &str, class: &str) -> String {
    format!("{workspace}|{class}")
}

/// Deux positions qui diffèrent assez pour venir de l'utilisateur
/// (Hyprland arrondit et applique ses tailles minimales).
fn moved(a: hypr::Rect, b: hypr::Rect) -> bool {
    let d = |x: i32, y: i32| (x - y).abs() > 8;
    d(a.0, b.0) || d(a.1, b.1) || d(a.2, b.2) || d(a.3, b.3)
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
                    Request::Release(scope, reply) => {
                        let _ = reply.send(desk.release(scope.as_deref()));
                    }
                    Request::Unpark(reply) => {
                        let _ = reply.send(hypr::unpark_desk_windows());
                    }
                    Request::Forget(reply) => {
                        let _ = reply.send(desk.forget());
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
                | Request::Release(_, reply)
                | Request::Unpark(reply)
                | Request::Forget(reply) => {
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
        self.enqueue(|reply| Request::Release(None, reply))
    }

    /// Comme `release`, pour les fenêtres de ces workspaces seulement.
    pub fn release_workspaces(
        &self,
        workspaces: Vec<String>,
    ) -> oneshot::Receiver<Result<(), String>> {
        self.enqueue(|reply| Request::Release(Some(workspaces), reply))
    }

    pub fn unpark(&self) -> oneshot::Receiver<Result<(), String>> {
        self.enqueue(Request::Unpark)
    }

    /// Oublie les places choisies (fichier et mémoire de la session).
    pub fn forget(&self) -> oneshot::Receiver<Result<(), String>> {
        self.enqueue(Request::Forget)
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
    /// Dernière place donnée par hyprdesk à chaque fenêtre : une fenêtre
    /// ailleurs a été déplacée par l'utilisateur.
    applied: HashMap<String, hypr::Rect>,
    /// Fenêtres déplacées par l'utilisateur : plus jamais réorganisées.
    touched: HashSet<String>,
    /// Places choisies (fichier), lues au premier besoin.
    places: Option<HashMap<String, hypr::Rect>>,
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
        // Hyprland les centre un instant après : leur place réelle ensuite.
        std::thread::sleep(std::time::Duration::from_millis(120));
        if let Ok(list) = Self::clients() {
            for g in list.into_iter().filter(|g| addresses.contains(&g.address)) {
                self.applied.insert(g.address.clone(), g.rect);
                self.managed.insert(g.address, g.workspace);
            }
        }
        self.floated
            .extend(if launched { addresses } else { changed });
        Ok(())
    }

    /// Fenêtres qui ne sont plus là où hyprdesk les a posées : déplacées
    /// par l'utilisateur. Plus jamais réorganisées ; leur place devient une
    /// préférence (workspace, application).
    fn note_user_moves(&mut self, list: &[hypr::Geometry]) {
        let mut changed = false;
        for g in list.iter().filter(|g| g.floating) {
            let Some(was) = self.applied.get(&g.address) else {
                continue;
            };
            if moved(*was, g.rect) {
                self.touched.insert(g.address.clone());
                self.applied.insert(g.address.clone(), g.rect);
                self.places
                    .get_or_insert_with(load_places)
                    .insert(place_key(&g.workspace, &g.class), g.rect);
                changed = true;
            }
        }
        if changed && let Some(p) = &self.places {
            save_places(p);
        }
    }

    fn forget(&mut self) -> Result<(), String> {
        self.touched.clear();
        self.memory.clear();
        self.places = Some(HashMap::new());
        match std::fs::remove_file(places_path()) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }

    fn arrange(&mut self, workspace: &str, area: hypr::Rect) -> Result<(), String> {
        let all = Self::clients()?;
        self.note_user_moves(&all);
        let list: Vec<hypr::Geometry> = all
            .into_iter()
            .filter(|g| g.workspace == workspace)
            .collect();
        let remembered = self.memory.get(workspace).cloned().unwrap_or_default();
        let places = self.places.get_or_insert_with(load_places).clone();
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
            // Déplacée par l'utilisateur : on n'y touche plus.
            if self.touched.contains(&g.address) {
                continue;
            }
            // Sa place choisie (workspace, application), sinon celle de la
            // dernière fois, sinon l'organisation automatique.
            let target = places
                .get(&place_key(workspace, &g.class))
                .or_else(|| remembered.get(&g.address))
                .copied();
            match target {
                Some(rect) => {
                    if let Err(e) = hypr::place(&g.address, rect) {
                        errors.push(e);
                    }
                    self.applied.insert(g.address.clone(), rect);
                }
                // Déjà flottante sans place connue : elle garde la sienne.
                None if !g.floating => fresh.push(g.address.clone()),
                None => {
                    self.applied.insert(g.address.clone(), g.rect);
                }
            }
        }
        for (a, rect) in fresh.iter().zip(hypr::arrange(fresh.len(), area, GAP)) {
            if let Err(e) = hypr::place(a, rect) {
                errors.push(e);
            }
            // Place demandée (Hyprland l'applique un instant après).
            self.applied.insert(a.clone(), rect);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join(" ; "))
        }
    }

    fn release(&mut self, scope: Option<&[String]>) -> Result<(), String> {
        let list = Self::clients().unwrap_or_default();
        // Déplacées depuis : préférences retenues avant de quitter.
        self.note_user_moves(&list);
        let in_scope = |ws: &str| scope.is_none_or(|s| s.iter().any(|w| w == ws));
        // Positions mémorisées, par workspace, pour le prochain plein bureau.
        let (managed, keep): (HashMap<String, String>, HashMap<String, String>) =
            std::mem::take(&mut self.managed)
                .into_iter()
                .partition(|(_, ws)| in_scope(ws));
        self.managed = keep;
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
        // Fenêtres que le plein bureau a fait flotter, sur ces workspaces.
        let (floated, keep): (HashSet<String>, HashSet<String>) = std::mem::take(&mut self.floated)
            .into_iter()
            .partition(|a| {
                scope.is_none()
                    || list
                        .iter()
                        .find(|g| &g.address == a)
                        .is_some_and(|g| in_scope(&g.workspace))
            });
        self.floated = keep;
        let floated: Vec<String> = floated.into_iter().collect();
        hypr::tile_windows(&floated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deplacement_par_l_utilisateur() {
        // Arrondis de Hyprland : pas un déplacement ; au-delà : oui.
        assert!(!moved((100, 100, 800, 600), (104, 98, 803, 600)));
        assert!(moved((100, 100, 800, 600), (300, 100, 800, 600)));
        assert!(moved((100, 100, 800, 600), (100, 100, 1000, 600)));
        assert_eq!(place_key("3", "kitty"), "3|kitty");
    }
}
