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
        };
        // Taille fixe (renommage compris) : la carte ne change pas de surface.
        let h = self.icon_menu_height(&menu) + 48.0 + 2.0 * SHADOW;
        let w = WIDTH + 2.0 * SHADOW;
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
