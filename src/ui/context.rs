//! Menu contextuel des icônes du bureau (MLG-19) : clic droit sur une icône.
//! Ouvrir ; application : épingler au dock (ou l'en retirer) ; pour un
//! fichier ou un dossier : ouvrir le dossier parent, renommer, mettre à la
//! corbeille (`gio trash`, spécification freedesktop de la corbeille),
//! copier le chemin ; icône ajoutée dans les Réglages : renommer, retirer du
//! bureau (le fichier reste) ; réglages des icônes.
//!
//! Le même menu sert à un widget (Réglages ouverts : le retirer, ses
//! réglages) et au fond du bureau (aligner les icônes, ajouter un widget,
//! changer de fond d'écran, réglages du bureau). Une carte du
//! thème au curseur, un capteur plein écran : un clic à côté ou Échap ferme.

use std::path::{Path, PathBuf};

use iced::widget::{button, column, container, mouse_area, text, text_input};
use iced::window::Id;
use iced::{Border, Color, Element, Length, Point, Shadow, Task, Vector};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::{App, Msg, color};
use crate::config::IconKind;

const WIDTH: f32 = 260.0;
/// Largeur de la carte des propriétés.
const PROPS_WIDTH: f32 = 400.0;
const ROW: f32 = 34.0;
const SHADOW: f32 = 16.0;
const INPUT_ID: &str = "hyprdesk-icon-rename";

/// Ce que vise le menu.
#[derive(Debug, Clone, PartialEq)]
pub enum MenuTarget {
    Icon,
    Widget(String),
    Desktop,
}

pub struct IconMenu {
    pub target: MenuTarget,
    pub win: Id,
    /// Surface du bureau où l'icône a été cliquée (son écran reçoit la fenêtre).
    pub desk: Id,
    pub catcher: Id,
    pub icon: usize,
    /// Fichier ou dossier de l'icône (sinon : application ou commande).
    pub path: Option<PathBuf>,
    /// Application (`.desktop`) de l'icône.
    pub app: Option<String>,
    /// Rang de l'icône parmi celles des Réglages (sinon : hyprdesk.lua,
    /// dossier Bureau…, qu'on ne retire ni ne renomme d'ici).
    pub own: Option<usize>,
    /// Renommage en cours : le nom saisi.
    pub rename: Option<String>,
    /// « Mettre à la corbeille » cliqué une fois : attend la confirmation.
    pub confirm: bool,
    pub error: Option<String>,
    /// « Propriétés » : libellé → valeur, à la place des entrées.
    pub props: Option<Vec<(String, String)>>,
}

#[derive(Debug, Clone)]
pub enum CMsg {
    Open,
    OpenFolder,
    Rename,
    RenameInput(String),
    RenameSubmit,
    Trash,
    CopyPath,
    /// Image : fond d'écran de tous les bureaux.
    Wallpaper,
    /// Widget : retiré du bureau ; ses réglages.
    RemoveWidget,
    WidgetSettings,
    /// Fond du bureau : icônes réalignées sur la grille ; un widget à ajouter ;
    /// un autre fond d'écran.
    AlignIcons,
    AddWidget,
    Wallpapers,
    /// Application : épinglée au dock, ou retirée.
    Pin,
    /// Icône des Réglages : retirée du bureau (le fichier, lui, reste).
    Remove,
    Settings,
    /// Détails utiles (fichier, application, widget, bureau) ; les copier.
    Properties,
    CopyProperties,
    Close,
    Done(Result<(), String>),
}

/// Image utilisable comme fond d'écran.
pub fn is_wallpaper(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| matches!(e.to_lowercase().as_str(), "png" | "jpg" | "jpeg" | "webp"))
}

/// Nouveau nom acceptable : non vide, sans « / », ni « . » ni « .. ».
pub fn valid_name(name: &str) -> bool {
    let n = name.trim();
    !n.is_empty() && !n.contains('/') && n != "." && n != ".." && !n.contains('\0')
}

/// Renomme dans le même dossier, sans écraser un fichier existant.
pub fn rename(path: &Path, name: &str) -> Result<PathBuf, String> {
    if !valid_name(name) {
        return Err(crate::i18n::tr("Nom invalide.").into());
    }
    let to = path
        .parent()
        .ok_or_else(|| crate::i18n::tr("Nom invalide.").to_string())?
        .join(name.trim());
    if to.exists() {
        return Err(crate::i18n::tr("Un fichier porte déjà ce nom.").into());
    }
    std::fs::rename(path, &to).map_err(|e| e.to_string())?;
    Ok(to)
}

fn trash(path: &Path) -> Result<(), String> {
    let o = std::process::Command::new("gio")
        .arg("trash")
        .arg(path)
        .output()
        .map_err(|e| e.to_string())?;
    if o.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&o.stderr).trim().to_string())
    }
}

/// Chemin raccourci (~ pour le dossier personnel).
fn home(p: &Path) -> String {
    let s = p.display().to_string();
    match std::env::var("HOME") {
        Ok(h) if !h.is_empty() && s.starts_with(&h) => format!("~{}", &s[h.len()..]),
        _ => s,
    }
}

/// Dossier Bureau de l'utilisateur (xdg-user-dir DESKTOP).
fn dirs_desktop() -> Option<PathBuf> {
    let o = std::process::Command::new("xdg-user-dir").arg("DESKTOP").output().ok()?;
    let p = PathBuf::from(String::from_utf8_lossy(&o.stdout).trim());
    p.is_dir().then_some(p)
}

/// Taille lisible : « 1,4 Mo (1 468 006 octets) ».
pub fn human_size(n: u64) -> String {
    let units = ["octets", "Ko", "Mo", "Go", "To"];
    let mut v = n as f64;
    let mut u = 0;
    while v >= 1024.0 && u < units.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{n} {}", crate::i18n::tr("octets"))
    } else {
        format!(
            "{} {} ({n} {})",
            format!("{v:.1}").replace('.', crate::i18n::tr(",")),
            crate::i18n::tr(units[u]),
            crate::i18n::tr("octets")
        )
    }
}

/// Droits à la manière de `ls -l` : « rw-r--r-- (644) ».
pub fn mode_string(mode: u32) -> String {
    let mut s = String::new();
    for shift in [6, 3, 0] {
        let b = (mode >> shift) & 7;
        s.push(if b & 4 != 0 { 'r' } else { '-' });
        s.push(if b & 2 != 0 { 'w' } else { '-' });
        s.push(if b & 1 != 0 { 'x' } else { '-' });
    }
    format!("{s} ({:o})", mode & 0o777)
}

/// Nom d'un utilisateur (/etc/passwd) ou d'un groupe (/etc/group).
fn id_name(file: &str, id: u32) -> String {
    std::fs::read_to_string(file)
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| {
                let f: Vec<&str> = l.split(':').collect();
                (f.get(2).and_then(|u| u.parse::<u32>().ok()) == Some(id)).then(|| f[0].to_string())
            })
        })
        .unwrap_or_else(|| id.to_string())
}

fn date(t: std::time::SystemTime) -> String {
    let d: chrono::DateTime<chrono::Local> = t.into();
    d.format_localized("%-d %B %Y, %H:%M", crate::lock::locale()).to_string()
}

fn run_out(cmd: &str, args: &[&str]) -> Option<String> {
    let o = std::process::Command::new(cmd).args(args).output().ok()?;
    let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
    (o.status.success() && !s.is_empty()).then_some(s)
}

/// Propriétés d'un fichier ou d'un dossier.
pub(super) fn path_properties(p: &Path) -> Vec<(String, String)> {
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    let tr = |s: &str| crate::i18n::tr(s).to_string();
    let mut v = Vec::new();
    v.push((
        tr("Nom"),
        p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
    ));
    let link = std::fs::symlink_metadata(p).is_ok_and(|m| m.file_type().is_symlink());
    let Ok(meta) = std::fs::metadata(p) else {
        v.push((tr("Emplacement"), p.parent().map(home).unwrap_or_default()));
        v.push((tr("État"), tr("introuvable (déplacé ou supprimé ?)")));
        return v;
    };
    let ps = p.display().to_string();
    let mime = if meta.is_dir() {
        None
    } else {
        run_out("xdg-mime", &["query", "filetype", &ps]).or_else(|| run_out("file", &["--mime-type", "-b", &ps]))
    };
    v.push((
        tr("Type"),
        match (&mime, meta.is_dir()) {
            (_, true) => tr("Dossier"),
            (Some(m), _) => crate::tr!("Fichier ({m})", m = m),
            (None, _) => tr("Fichier"),
        },
    ));
    v.push((tr("Emplacement"), p.parent().map(home).unwrap_or_default()));
    if link && let Ok(t) = std::fs::read_link(p) {
        v.push((tr("Lien vers"), home(&t)));
    }
    if meta.is_dir() {
        let n = std::fs::read_dir(p).map(|r| r.count()).unwrap_or(0);
        v.push((tr("Contenu"), crate::tr!("{n} éléments", n = n)));
    } else {
        v.push((tr("Taille"), human_size(meta.len())));
    }
    if let Ok(t) = meta.modified() {
        v.push((tr("Modifié"), date(t)));
    }
    if let Ok(t) = meta.created() {
        v.push((tr("Créé"), date(t)));
    }
    if let Ok(t) = meta.accessed() {
        v.push((tr("Ouvert"), date(t)));
    }
    v.push((tr("Droits"), mode_string(meta.permissions().mode())));
    v.push((tr("Propriétaire"), format!("{} / {}", id_name("/etc/passwd", meta.uid()), id_name("/etc/group", meta.gid()))));
    if let Some(m) = &mime
        && let Some(id) = run_out("xdg-mime", &["query", "default", m])
    {
        let name = crate::launch::find_desktop_file(id.trim_end_matches(".desktop"))
            .and_then(|f| std::fs::read_to_string(f).ok())
            .and_then(|s| crate::launch::parse_desktop_entry(&s, crate::i18n::code()))
            .map(|e| e.name)
            .unwrap_or(id);
        v.push((tr("S'ouvre avec"), name));
    }
    v
}

/// Hauteur de la carte des propriétés (valeurs longues sur plusieurs lignes).
fn props_height(props: &[(String, String)]) -> f32 {
    let per_line = ((PROPS_WIDTH - 28.0) / 7.2) as usize;
    let rows: f32 = props
        .iter()
        .map(|(_, v)| 16.0 + 18.0 * v.chars().count().div_ceil(per_line.max(1)).max(1) as f32 + 6.0)
        .sum();
    ROW + 6.0 + rows + 40.0
}

/// Après un changement dans le dossier Bureau : relire la config (icônes).
fn reload_after(r: Result<(), String>) -> Msg {
    match r {
        Ok(()) => Msg::ConfigChanged,
        Err(e) => Msg::IconMenu(CMsg::Done(Err(e))),
    }
}

impl App {
    /// Entrées du menu pour l'icône (libellés, messages).
    fn icon_menu_entries(&self, m: &IconMenu) -> Vec<(&'static str, CMsg)> {
        match &m.target {
            MenuTarget::Widget(id) => {
                let mut v = Vec::new();
                if self.settings.widgets.iter().any(|w| &w.id == id) {
                    v.push(("Retirer du bureau", CMsg::RemoveWidget));
                }
                v.push(("Réglages du widget…", CMsg::WidgetSettings));
                v.push(("Propriétés", CMsg::Properties));
                return v;
            }
            MenuTarget::Desktop => {
                let mut v = Vec::new();
                if self.cfg.plugin_on("icons") {
                    v.push(("Aligner les icônes", CMsg::AlignIcons));
                }
                if self.cfg.plugin_on("widgets") {
                    v.push(("Ajouter un widget…", CMsg::AddWidget));
                }
                if self.cfg.plugin_on("wallpaper") {
                    v.push(("Changer de fond d'écran…", CMsg::Wallpapers));
                }
                v.push(("Réglages du bureau…", CMsg::Settings));
                v.push(("Propriétés", CMsg::Properties));
                return v;
            }
            MenuTarget::Icon => {}
        }
        let mut v = vec![("Ouvrir", CMsg::Open)];
        if let Some(a) = &m.app
            && self.cfg.plugin_on("launcher")
        {
            v.push(if self.pinned().contains(a) {
                ("Retirer du dock", CMsg::Pin)
            } else {
                ("Épingler au dock", CMsg::Pin)
            });
        }
        if m.path.is_none() && m.own.is_some() {
            v.push(("Renommer…", CMsg::Rename));
        }
        if m.path.is_some() {
            v.push(("Ouvrir le dossier", CMsg::OpenFolder));
            v.push(("Renommer…", CMsg::Rename));
            v.push(if m.confirm {
                ("Confirmer : mettre à la corbeille", CMsg::Trash)
            } else {
                ("Mettre à la corbeille", CMsg::Trash)
            });
            v.push(("Copier le chemin", CMsg::CopyPath));
            if m.path.as_deref().is_some_and(is_wallpaper) {
                v.push(("Définir comme fond d'écran", CMsg::Wallpaper));
            }
        }
        if m.own.is_some() {
            v.push(("Retirer du bureau", CMsg::Remove));
        }
        v.push(("Réglages des icônes…", CMsg::Settings));
        v.push(("Propriétés", CMsg::Properties));
        v
    }

    fn icon_menu_target(&self) -> Option<MenuTarget> {
        self.icon_menu.as_ref().map(|m| m.target.clone())
    }

    /// Applications épinglées au dock (réglage en cours, sinon la config).
    fn pinned(&self) -> Vec<String> {
        self.settings
            .launcher_pinned
            .clone()
            .unwrap_or_else(|| self.cfg.general.launcher.pinned.clone())
    }

    fn icon_menu_height(&self, m: &IconMenu) -> f32 {
        if let Some(props) = &m.props {
            return props_height(props);
        }
        let rows = self.icon_menu_entries(m).len() as f32 + 1.0;
        rows * ROW + 24.0 + if m.rename.is_some() { 48.0 } else { 0.0 }
    }

    /// Clic droit sur l'icône `idx` de la surface `desk`.
    pub(super) fn open_icon_menu(&mut self, desk: Id, idx: usize) -> Task<Msg> {
        self.open_menu(desk, MenuTarget::Icon, idx)
    }

    /// Clic droit sur un widget (Réglages ouverts).
    pub(super) fn open_widget_menu(&mut self, desk: Id, id: String) -> Task<Msg> {
        self.open_menu(desk, MenuTarget::Widget(id), usize::MAX)
    }

    /// Clic droit sur le fond du bureau.
    pub(super) fn open_desk_menu(&mut self, desk: Id) -> Task<Msg> {
        self.open_menu(desk, MenuTarget::Desktop, usize::MAX)
    }

    fn open_menu(&mut self, desk: Id, target: MenuTarget, idx: usize) -> Task<Msg> {
        self.open_menu_with(desk, target, idx, None)
    }

    fn open_menu_with(
        &mut self,
        desk: Id,
        target: MenuTarget,
        idx: usize,
        props: Option<Vec<(String, String)>>,
    ) -> Task<Msg> {
        let mut tasks = vec![self.close_icon_menu()];
        let Some(monitor) = self.surfaces.get(&desk).cloned() else {
            return Task::batch(tasks);
        };
        let def = match target {
            MenuTarget::Icon => match self.cfg.icons.get(idx) {
                Some(d) => Some(d),
                None => return Task::batch(tasks),
            },
            _ => None,
        };
        let path = def.and_then(|d| match &d.kind {
            IconKind::Path(p) => Some(p.clone()),
            _ => None,
        });
        let app = def.and_then(|d| match &d.kind {
            IconKind::App(a) => Some(a.clone()),
            _ => None,
        });
        let own = def.and_then(|def| {
            self.settings
                .icons
                .iter()
                .position(|i| i.id == def.id && i.sel == def.sel)
        });
        let at = self
            .cursor
            .get(&desk)
            .copied()
            .unwrap_or(Point::new(40.0, 40.0));
        let size = self.sizes.get(&desk).copied();
        let (catcher, win) = (Id::unique(), Id::unique());
        let menu = IconMenu {
            target,
            win,
            desk,
            catcher,
            icon: idx,
            path,
            app,
            own,
            rename: None,
            confirm: false,
            error: None,
            props,
        };
        // Taille fixe (renommage compris) : la carte ne change pas de surface.
        let h = self.icon_menu_height(&menu) + 48.0 + 2.0 * SHADOW;
        let w = if menu.props.is_some() { PROPS_WIDTH } else { WIDTH } + 2.0 * SHADOW;
        let (mut x, mut y) = (at.x - SHADOW, at.y - SHADOW);
        if let Some(s) = size {
            x = x.min(s.width - w).max(0.0);
            y = y.min(s.height - h).max(0.0);
        }
        tasks.push(Task::done(Msg::NewLayerShell {
            settings: NewLayerShellSettings {
                size: Some((0, 0)),
                layer: Layer::Top,
                anchor: LsAnchor::Top | LsAnchor::Bottom | LsAnchor::Left | LsAnchor::Right,
                exclusive_zone: Some(-1),
                keyboard_interactivity: KeyboardInteractivity::None,
                output_option: OutputOption::OutputName(monitor.clone()),
                namespace: Some("hyprdesk-catcher".into()),
                ..Default::default()
            },
            id: catcher,
        }));
        tasks.push(Task::done(Msg::NewLayerShell {
            settings: NewLayerShellSettings {
                size: Some((w as u32, h as u32)),
                layer: Layer::Overlay,
                anchor: LsAnchor::Top | LsAnchor::Left,
                exclusive_zone: Some(-1),
                margin: Some((y as i32, 0, 0, x as i32)),
                keyboard_interactivity: KeyboardInteractivity::OnDemand,
                output_option: OutputOption::OutputName(monitor),
                namespace: Some("hyprdesk-icon-menu".into()),
                ..Default::default()
            },
            id: win,
        }));
        self.icon_menu = Some(menu);
        Task::batch(tasks)
    }

    pub(super) fn close_icon_menu(&mut self) -> Task<Msg> {
        match self.icon_menu.take() {
            Some(m) => Task::batch([iced::window::close(m.win), iced::window::close(m.catcher)]),
            None => Task::none(),
        }
    }

    pub(super) fn icon_menu_update(&mut self, m: CMsg) -> Task<Msg> {
        let Some(menu) = self.icon_menu.as_mut() else {
            return Task::none();
        };
        let path = menu.path.clone();
        let idx = menu.icon;
        let own = menu.own;
        match m {
            CMsg::Open => {
                let desk = self.icon_menu.as_ref().map(|m| m.desk);
                let close = self.close_icon_menu();
                Task::batch([close, self.launch_from(desk, idx)])
            }
            CMsg::OpenFolder => {
                let close = self.close_icon_menu();
                let dir = path
                    .as_deref()
                    .and_then(Path::parent)
                    .map(Path::to_path_buf);
                let open = match dir {
                    Some(d) => self.run_from_desktop(
                        format!(
                            "xdg-open {}",
                            crate::launch::sh_quote(&d.display().to_string())
                        ),
                        true,
                    ),
                    None => Task::none(),
                };
                Task::batch([close, open])
            }
            CMsg::Rename => {
                // Fichier : son nom ; autre icône : son libellé affiché.
                let label = self.icons.get(idx).map(|r| r.label.clone()).unwrap_or_default();
                let Some(menu) = self.icon_menu.as_mut() else {
                    return Task::none();
                };
                menu.rename = Some(
                    path.as_deref()
                        .and_then(Path::file_name)
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or(label),
                );
                menu.confirm = false;
                iced::widget::operation::focus(INPUT_ID)
            }
            CMsg::RenameInput(s) => {
                menu.rename = Some(s);
                menu.error = None;
                Task::none()
            }
            CMsg::RenameSubmit => {
                // Icône des Réglages (application, commande) : son libellé.
                if path.is_none() {
                    let (Some(i), Some(name)) = (own, menu.rename.clone()) else {
                        return Task::none();
                    };
                    let name = name.trim().to_string();
                    if let Some(ic) = self.settings.icons.get_mut(i) {
                        ic.label = (!name.is_empty()).then_some(name);
                    }
                    let close = self.close_icon_menu();
                    return Task::batch([close, self.commit()]);
                }
                let (Some(p), Some(name)) = (path, menu.rename.clone()) else {
                    return Task::none();
                };
                match rename(&p, &name) {
                    Ok(_) => {
                        let close = self.close_icon_menu();
                        Task::batch([close, Task::done(Msg::ConfigChanged)])
                    }
                    Err(e) => {
                        menu.error = Some(e);
                        Task::none()
                    }
                }
            }
            CMsg::Trash => {
                if !menu.confirm {
                    menu.confirm = true;
                    return Task::none();
                }
                let Some(p) = path else {
                    return Task::none();
                };
                let close = self.close_icon_menu();
                let work = Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || trash(&p))
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    reload_after,
                );
                Task::batch([close, work])
            }
            CMsg::CopyPath => {
                let close = self.close_icon_menu();
                if let Some(p) = path {
                    let _ = std::process::Command::new("wl-copy")
                        .arg(p.display().to_string())
                        .spawn();
                }
                close
            }
            CMsg::Wallpaper => {
                let close = self.close_icon_menu();
                let Some(p) = path.filter(|p| is_wallpaper(p)) else {
                    return close;
                };
                // Tous les bureaux, en gardant l'ajustement du fond actuel.
                let sel = crate::config::Selector::default();
                let fit = self
                    .settings
                    .wallpaper(&sel)
                    .map(|w| w.fit)
                    .unwrap_or_default();
                self.settings
                    .set_wallpaper(sel, crate::config::WallSource::Path(p), fit);
                Task::batch([close, self.commit()])
            }
            CMsg::Pin => {
                let close = self.close_icon_menu();
                let Some(a) = self.cfg.icons.get(idx).and_then(|d| match &d.kind {
                    IconKind::App(a) => Some(a.clone()),
                    _ => None,
                }) else {
                    return close;
                };
                let mut l = self.pinned();
                if let Some(at) = l.iter().position(|x| x == &a) {
                    l.remove(at);
                } else {
                    l.push(a);
                }
                self.settings.launcher_pinned = Some(l);
                Task::batch([close, self.commit()])
            }
            CMsg::RemoveWidget => {
                let target = self.icon_menu_target();
                let close = self.close_icon_menu();
                let Some(MenuTarget::Widget(id)) = target else {
                    return close;
                };
                if let Some(p) = self.panel.as_mut()
                    && p.widgets.widget_sel.as_deref() == Some(id.as_str())
                {
                    p.widgets.widget_sel = None;
                }
                self.settings.remove_widget(&id);
                Task::batch([close, self.commit()])
            }
            CMsg::WidgetSettings => {
                let target = self.icon_menu_target();
                let close = self.close_icon_menu();
                if let Some(MenuTarget::Widget(id)) = target
                    && let Some(p) = self.panel.as_mut()
                {
                    p.select_widget(&id);
                }
                Task::batch([
                    close,
                    Task::done(Msg::Control(super::control::Command::Settings(
                        "widgets/bureau".into(),
                    ))),
                ])
            }
            CMsg::AlignIcons => {
                // Positions mémorisées de ce bureau oubliées : les icônes
                // reprennent la grille, dans l'ordre.
                let desk = self.icon_menu.as_ref().map(|m| m.desk);
                let close = self.close_icon_menu();
                if let Some(m) = desk.and_then(|d| self.monitor(d)).cloned() {
                    self.layout.clear_desktop(&m.name, &m.workspace);
                    self.layout.save();
                }
                close
            }
            CMsg::AddWidget => {
                let close = self.close_icon_menu();
                Task::batch([
                    close,
                    Task::done(Msg::Control(super::control::Command::Settings(
                        "widgets/disponibles".into(),
                    ))),
                ])
            }
            CMsg::Wallpapers => {
                let close = self.close_icon_menu();
                Task::batch([
                    close,
                    Task::done(Msg::Control(super::control::Command::Settings(
                        "wallpaper".into(),
                    ))),
                ])
            }
            CMsg::Remove => {
                let close = self.close_icon_menu();
                let Some(i) = own else {
                    return close;
                };
                self.settings.remove_icon(i);
                self.selected = None;
                Task::batch([close, self.commit()])
            }
            CMsg::Settings => {
                let page = match self.icon_menu_target() {
                    Some(MenuTarget::Desktop) => "desktop",
                    _ => "icons",
                };
                let close = self.close_icon_menu();
                Task::batch([
                    close,
                    Task::done(Msg::Control(super::control::Command::Settings(page.into()))),
                ])
            }
            CMsg::Properties => {
                let (desk, target) = (menu.desk, menu.target.clone());
                let props = self.properties(desk, &target, idx, path.as_deref());
                self.open_menu_with(desk, target, idx, Some(props))
            }
            CMsg::CopyProperties => {
                let text = menu
                    .props
                    .as_ref()
                    .map(|p| {
                        p.iter()
                            .map(|(k, v)| format!("{k} : {v}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
                let close = self.close_icon_menu();
                if let Ok(mut c) = std::process::Command::new("wl-copy")
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                {
                    use std::io::Write;
                    if let Some(i) = c.stdin.as_mut() {
                        let _ = i.write_all(text.as_bytes());
                    }
                    drop(c.stdin.take());
                    let _ = c.wait();
                }
                close
            }
            CMsg::Close => self.close_icon_menu(),
            CMsg::Done(r) => {
                if let Err(e) = r {
                    tracing::warn!("{}", crate::tr!("corbeille : {e}", e = e));
                    super::hypr::notify(0, 5000, &crate::tr!("Corbeille : {e}", e = e));
                }
                Task::none()
            }
        }
    }

    /// Détails utiles de ce que vise le menu.
    fn properties(
        &self,
        desk: Id,
        target: &MenuTarget,
        idx: usize,
        path: Option<&Path>,
    ) -> Vec<(String, String)> {
        let tr = |s: &str| crate::i18n::tr(s).to_string();
        let yes_no = |b: bool| tr(if b { "oui" } else { "non" });
        let mut v: Vec<(String, String)> = Vec::new();
        match target {
            MenuTarget::Icon => {
                let def = self.cfg.icons.get(idx);
                if let Some(p) = path {
                    v.extend(path_properties(p));
                } else if let Some(d) = def {
                    match &d.kind {
                        IconKind::App(a) => {
                            let file = crate::launch::find_desktop_file(a);
                            let entry = file
                                .as_ref()
                                .and_then(|f| std::fs::read_to_string(f).ok())
                                .and_then(|s| {
                                    crate::launch::parse_desktop_entry(&s, crate::i18n::code())
                                });
                            if let Some(e) = &entry {
                                v.push((tr("Nom"), e.name.clone()));
                            }
                            v.push((tr("Type"), tr("Application")));
                            v.push((tr("Identifiant"), a.clone()));
                            match &file {
                                Some(f) => v.push((tr("Fichier"), home(f))),
                                None => v.push((tr("Fichier"), tr("introuvable (application désinstallée ?)"))),
                            }
                            if let Some(e) = &entry {
                                v.push((tr("Commande"), e.exec.clone()));
                                if let Some(w) = &e.wm_class {
                                    v.push((tr("Classe de fenêtre"), w.clone()));
                                }
                                if !e.categories.is_empty() {
                                    v.push((tr("Catégories"), e.categories.join(", ")));
                                }
                                v.push((tr("Dans un terminal"), yes_no(e.terminal)));
                            }
                            v.push((tr("Épinglée au dock"), yes_no(self.pinned().contains(a))));
                        }
                        IconKind::Exec { cmd, terminal } => {
                            v.push((tr("Type"), tr("Commande")));
                            v.push((tr("Commande"), cmd.clone()));
                            v.push((tr("Dans un terminal"), yes_no(*terminal)));
                        }
                        IconKind::Path(_) => {}
                    }
                }
                if let Some(d) = def {
                    if let Some(l) = &d.label {
                        v.push((tr("Libellé"), l.clone()));
                    }
                    if let Some(i) = &d.icon {
                        v.push((tr("Icône"), i.clone()));
                    }
                    v.push((
                        tr("Déclarée"),
                        tr(match d.origin {
                            crate::config::Origin::Settings => "dans les Réglages",
                            crate::config::Origin::User => "dans hyprdesk.lua (ou dossier Bureau)",
                        }),
                    ));
                }
            }
            MenuTarget::Widget(id) => {
                let w = self.cfg.widgets.iter().find(|w| &w.id == id);
                let ty = w.map(|w| w.type_id.clone()).unwrap_or_default();
                let name = self
                    .cfg
                    .catalog
                    .iter()
                    .find(|t| t.id == ty)
                    .map(|t| t.name.clone())
                    .unwrap_or_else(|| ty.clone());
                v.push((tr("Widget"), name));
                v.push((tr("Type"), ty));
                v.push((tr("Identifiant"), id.clone()));
                v.push((
                    tr("Déclaré"),
                    tr(if self.settings.widgets.iter().any(|w| &w.id == id) {
                        "dans les Réglages"
                    } else {
                        "dans hyprdesk.lua"
                    }),
                ));
            }
            MenuTarget::Desktop => {
                if let Some(m) = self.monitor(desk) {
                    v.push((
                        tr("Écran"),
                        format!(
                            "{} — {}×{} @ {}",
                            m.name,
                            (m.width as f32 * m.scale).round(),
                            (m.height as f32 * m.scale).round(),
                            m.scale
                        ),
                    ));
                    v.push((tr("Workspace"), m.workspace.clone()));
                    v.push((tr("Mode"), self.monitor_mode(&m.name).as_str().to_string()));
                    let d = crate::config::resolve::resolve(&self.cfg, &m.name, &m.workspace);
                    let wall = match d.wallpaper.map(|w| (&w.source, w.span)) {
                        Some((crate::config::WallSource::Path(p), span)) => {
                            let mut s = home(p);
                            if span {
                                s += &format!(" ({})", tr("étalé sur les écrans"));
                            }
                            s
                        }
                        Some((crate::config::WallSource::Rotate { dir, interval, .. }, _)) => {
                            crate::tr!(
                                "rotation dans {d}, toutes les {n} min",
                                d = home(dir),
                                n = interval.as_secs() / 60
                            )
                        }
                        Some((crate::config::WallSource::Color(c), _)) => {
                            crate::tr!("couleur {c}", c = format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2))
                        }
                        None => tr("aucun (couleur du thème)"),
                    };
                    v.push((tr("Fond d'écran"), wall));
                    v.push((tr("Thème"), self.cfg.general.theme.name.clone()));
                    v.push((tr("Icônes"), self.icons.len().to_string()));
                    v.push((tr("Widgets"), d.widgets.len().to_string()));
                }
                if let Some(dir) = dirs_desktop() {
                    v.push((tr("Dossier Bureau"), home(&dir)));
                }
            }
        }
        v
    }

    /// Carte des propriétés : libellé au-dessus de sa valeur, puis Copier / Fermer.
    fn view_properties<'a>(&'a self, name: &str, props: &'a [(String, String)]) -> Element<'a, Msg> {
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let accent = color(th.accent, 1.0);
        let bg = color(
            th.bg,
            th.effect(crate::config::Place::Menus).opacity.max(0.97),
        );
        let mut col = column![
            text(crate::tr!("Propriétés — {n}", n = name))
                .font(self.title_font())
                .size(14)
                .color(fg)
                .height(ROW)
        ]
        .spacing(6);
        for (k, v) in props {
            col = col.push(
                column![
                    text(k.clone()).font(self.font).size(11).color(dim),
                    text(v.clone())
                        .font(self.font)
                        .size(13)
                        .color(fg)
                        .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
                ]
                .spacing(1),
            );
        }
        let btn = |label: &'static str, msg: CMsg| {
            button(text(crate::i18n::tr(label)).font(self.font).size(13).color(fg))
                .padding([5, 12])
                .style(move |_, st| button::Style {
                    background: Some(
                        Color {
                            a: if matches!(st, button::Status::Hovered) { 0.22 } else { 0.12 },
                            ..accent
                        }
                        .into(),
                    ),
                    border: Border::default().rounded(8),
                    text_color: fg,
                    ..Default::default()
                })
                .on_press(Msg::IconMenu(msg))
        };
        col = col.push(
            iced::widget::row![
                iced::widget::Space::new().width(Length::Fill),
                btn("Copier", CMsg::CopyProperties),
                btn("Fermer", CMsg::Close),
            ]
            .spacing(8),
        );
        let card = container(col)
            .padding(14)
            .width(PROPS_WIDTH)
            .style(move |_| container::Style {
                background: Some(bg.into()),
                border: Border::default()
                    .rounded(super::design::RADIUS)
                    .width(1)
                    .color(Color { a: 0.12, ..fg }),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
                    offset: Vector::new(0.0, 6.0),
                    blur_radius: 16.0,
                },
                text_color: Some(fg),
                ..Default::default()
            });
        container(card).padding(SHADOW).into()
    }

    pub(super) fn view_icon_catcher(&self) -> Element<'_, Msg> {
        mouse_area(
            container(iced::widget::Space::new())
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(Msg::IconMenu(CMsg::Close))
        .on_right_press(Msg::IconMenu(CMsg::Close))
        .into()
    }

    pub(in crate::ui) fn view_icon_menu(&self) -> Element<'_, Msg> {
        let Some(m) = &self.icon_menu else {
            return iced::widget::Space::new().into();
        };
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let accent = color(th.accent, 1.0);
        let bg = color(
            th.bg,
            th.effect(crate::config::Place::Menus).opacity.max(0.97),
        );
        let name = match &m.target {
            MenuTarget::Icon => self
                .icons
                .get(m.icon)
                .map(|r| r.label.clone())
                .unwrap_or_default(),
            MenuTarget::Widget(id) => {
                let ty = self
                    .cfg
                    .widgets
                    .iter()
                    .find(|w| &w.id == id)
                    .map(|w| w.type_id.clone())
                    .unwrap_or_default();
                let name = self
                    .cfg
                    .catalog
                    .iter()
                    .find(|t| t.id == ty)
                    .map(|t| t.name.clone())
                    .unwrap_or(ty);
                if self.settings.widgets.iter().any(|w| &w.id == id) {
                    name
                } else {
                    crate::tr!("{n} — déclaré dans hyprdesk.lua", n = name)
                }
            }
            MenuTarget::Desktop => crate::i18n::tr("Bureau").to_string(),
        };
        if let Some(props) = &m.props {
            return self.view_properties(&name, props);
        }
        let mut col =
            column![text(name).font(self.font).size(12).color(dim).height(ROW)].spacing(0);
        for (label, msg) in self.icon_menu_entries(m) {
            let danger = matches!(msg, CMsg::Trash) && m.confirm;
            col =
                col.push(
                    button(text(crate::i18n::tr(label)).font(self.font).size(14).color(
                        if danger {
                            Color::from_rgb8(0xff, 0x8a, 0x80)
                        } else {
                            fg
                        },
                    ))
                    .padding([6, 10])
                    .width(Length::Fill)
                    .height(ROW)
                    .style(move |_, st| button::Style {
                        background: matches!(st, button::Status::Hovered)
                            .then(|| Color { a: 0.14, ..accent }.into()),
                        border: Border::default().rounded(8),
                        text_color: fg,
                        ..Default::default()
                    })
                    .on_press(Msg::IconMenu(msg)),
                );
        }
        if let Some(r) = &m.rename {
            col = col.push(
                text_input(crate::i18n::tr("Nouveau nom"), r)
                    .id(INPUT_ID)
                    .on_input(|s| Msg::IconMenu(CMsg::RenameInput(s)))
                    .on_submit(Msg::IconMenu(CMsg::RenameSubmit))
                    .font(self.font)
                    .size(14)
                    .padding(8),
            );
        }
        if let Some(e) = &m.error {
            col = col.push(
                text(e.clone())
                    .font(self.font)
                    .size(12)
                    .color(Color::from_rgb8(0xff, 0xb8, 0x6c)),
            );
        }
        let card = container(col)
            .padding(12)
            .width(WIDTH)
            .style(move |_| container::Style {
                background: Some(bg.into()),
                border: Border::default()
                    .rounded(super::design::RADIUS)
                    .width(1)
                    .color(Color { a: 0.12, ..fg }),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
                    offset: Vector::new(0.0, 6.0),
                    blur_radius: 16.0,
                },
                text_color: Some(fg),
                ..Default::default()
            });
        container(card).padding(SHADOW).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_de_fond() {
        assert!(is_wallpaper(Path::new("/a/b.JPG")));
        assert!(is_wallpaper(Path::new("x.webp")));
        assert!(!is_wallpaper(Path::new("notes.txt")));
        assert!(!is_wallpaper(Path::new("dossier")));
    }

    #[test]
    fn tailles_et_droits() {
        assert_eq!(mode_string(0o100644), "rw-r--r-- (644)");
        assert_eq!(mode_string(0o40755), "rwxr-xr-x (755)");
        assert!(human_size(512).starts_with("512 "));
        assert!(human_size(1_468_006).contains("1 468 006") || human_size(1_468_006).contains("1468006"));
    }

    #[test]
    fn noms_valides() {
        assert!(valid_name("Rapport 2026.pdf"));
        assert!(!valid_name("  "));
        assert!(!valid_name("a/b"));
        assert!(!valid_name(".."));
    }

    #[test]
    fn renommer_sans_ecraser() {
        let dir = std::env::temp_dir().join(format!("hyprdesk-renommer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), "a").unwrap();
        std::fs::write(dir.join("b.txt"), "b").unwrap();
        assert!(rename(&dir.join("a.txt"), "b.txt").is_err());
        let to = rename(&dir.join("a.txt"), " c.txt ").unwrap();
        assert_eq!(to, dir.join("c.txt"));
        assert!(to.exists() && !dir.join("a.txt").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
