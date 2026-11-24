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
//!
//! Il est dessiné par le composant commun des menus (`super::menu`, MLG-306) :
//! trois groupes au plus, une icône par entrée, corbeille en rouge, flèches et
//! Entrée au clavier. La même carte sert au dock et à la grille du Lanceur
//! (une application : ses fenêtres, ses actions `.desktop`, l'épingler, la
//! fermer ; MLG-307, MLG-308), aux champs de saisie (Couper, Copier, Coller,
//! Tout sélectionner ; MLG-309) et au partage d'un fichier (MLG-310).

use crate::ui::tokens::Shown as _;
use std::path::{Path, PathBuf};

use iced::widget::{button, column, container, mouse_area, text, text_input};
use iced::window::Id;
use iced::{Border, Color, Element, Length, Point, Shadow, Task, Vector};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::menu::{Item, Line, groups};
use super::{App, Msg, color};
use crate::config::IconKind;

const WIDTH: f32 = 260.0;
/// Largeur d'un menu d'application (titres de fenêtres).
const APP_WIDTH: f32 = 300.0;
/// Largeur du menu d'édition d'un champ.
const EDIT_WIDTH: f32 = 220.0;
/// Largeur de la carte des propriétés.
const PROPS_WIDTH: f32 = 400.0;
const ROW: f32 = 34.0;
pub(in crate::ui) const SHADOW: f32 = 16.0;
/// Marge intérieure de la carte d'un menu.
const PAD: f32 = 6.0;
const INPUT_ID: &str = "hyprdesk-icon-rename";
/// Défilement des lignes d'une carte plafonnée à l'écran (HIG MENU-02).
const SCROLL: &str = "hyprdesk-icon-menu-list";

/// Hauteur de la surface d'une carte de `full` pixels dans un écran de
/// `screen` : toute la carte, ou l'écran quand elle le dépasse. Ses lignes
/// défilent alors dans la carte, comme dans les menus de la barre.
fn window_height(full: f32, screen: f32) -> (f32, bool) {
    if full > screen {
        (screen, true)
    } else {
        (full, false)
    }
}

/// Décalage qui rend visible la ligne `[top, top + row]` d'un contenu de
/// `content` pixels montré dans une fenêtre de `view` pixels, depuis le
/// décalage `from` : le plus court, sans sortir du contenu.
fn follow(from: f32, view: f32, content: f32, top: f32, row: f32) -> f32 {
    let to = if top + row > from + view {
        top + row - view
    } else if top < from {
        top
    } else {
        from
    };
    to.clamp(0.0, (content - view).max(0.0))
}

/// Ce que vise le menu.
#[derive(Debug, Clone, PartialEq)]
pub enum MenuTarget {
    Icon,
    Widget(String),
    Desktop,
    /// Application du dock ou de la grille du Lanceur (MLG-307, MLG-308).
    App(AppRef),
    /// Champ de saisie sous le curseur (MLG-309).
    Edit(EditRef),
}

/// Application visée par un menu du dock ou de la grille du Lanceur.
#[derive(Debug, Clone, PartialEq)]
pub struct AppRef {
    /// Identifiant `.desktop` (épinglée, grille, ou retrouvé par la classe).
    pub id: Option<String>,
    /// Classe de fenêtre (application ouverte).
    pub class: Option<String>,
    pub name: String,
}

/// Champ de saisie visé par le menu d'édition.
#[derive(Debug, Clone, PartialEq)]
pub struct EditRef {
    /// Surface qui porte le champ : la commande y est rejouée.
    pub surface: Id,
    /// Une sélection à copier (faux dans un champ masqué).
    pub can_copy: bool,
    /// Le champ accepte la saisie.
    pub editable: bool,
    /// Le presse-papiers a du texte.
    pub can_paste: bool,
}

/// Destination de « Partager… » (MLG-310).
#[derive(Debug, Clone, PartialEq)]
pub enum Share {
    /// Fichier copié (`text/uri-list`) : à coller dans une autre application.
    Copy,
    /// Pièce jointe d'un nouveau courriel (`xdg-email --attach`).
    Mail,
    /// Téléphone appairé par KDE Connect.
    Phone { id: String, name: String },
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
    /// Écran du menu et coin haut-gauche de sa surface (ombre comprise).
    pub monitor: String,
    pub origin: Point,
    /// Entrée choisie au clavier (rang dans les lignes).
    pub hover: Option<usize>,
    /// La carte dépasse l'écran : la surface est plafonnée et ses lignes
    /// défilent (HIG MENU-02).
    pub scrolls: bool,
    /// Carte plafonnée : hauteur visible des lignes, et décalage du
    /// défilement que le clavier a demandé en dernier.
    pub viewport: f32,
    pub offset: f32,
    /// Entrée `.desktop` de l'application visée : ses actions, son terminal.
    pub entry: Option<crate::launch::DesktopEntry>,
    /// « Partager… » : les destinations, à la place des entrées.
    pub share: Option<Vec<Share>>,
}

impl IconMenu {
    /// Menu sans surface encore (placé par `spawn_menu`).
    pub fn new(target: MenuTarget, desk: Id) -> Self {
        Self {
            target,
            win: Id::unique(),
            desk,
            catcher: Id::unique(),
            icon: usize::MAX,
            path: None,
            app: None,
            own: None,
            rename: None,
            confirm: false,
            error: None,
            props: None,
            monitor: String::new(),
            origin: Point::ORIGIN,
            hover: None,
            scrolls: false,
            viewport: 0.0,
            offset: 0.0,
            entry: None,
            share: None,
        }
    }

    /// Carte de `full` pixels sur un écran de `screen` : hauteur de sa
    /// surface — l'écran quand la carte le dépasse, ses lignes défilent
    /// alors (HIG MENU-02). Sans écran connu, la carte garde sa taille.
    pub(in crate::ui) fn cap(&mut self, full: f32, screen: Option<f32>) -> f32 {
        let (h, scrolls) = match screen {
            Some(screen) => window_height(full, screen),
            None => (full, false),
        };
        self.scrolls = scrolls;
        self.viewport = if scrolls {
            h - 2.0 * (SHADOW + PAD)
        } else {
            0.0
        };
        self.offset = 0.0;
        h
    }
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
    /// Dock, grille : une fenêtre de l'application au premier plan.
    Focus(String),
    /// Application lancée (Ouvrir, Nouvelle fenêtre).
    Launch,
    /// Action de l'application (`[Desktop Action]`), par son identifiant.
    Action(String),
    /// Fenêtres de l'application fermées.
    CloseWindows,
    /// « Partager… », puis la destination choisie.
    Share,
    ShareTo(Share),
    /// Champ de saisie : commande d'édition rejouée dans sa surface.
    Edit(super::edit::Command),
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
        return Err(crate::i18n::tr("Choisissez un nom non vide, sans barre oblique (/)").into());
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

/// Texte coupé à `n` caractères, « … » au bout.
fn clip(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(n.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}

/// `file://` d'un chemin, octets réservés encodés (RFC 3986).
fn file_uri(p: &Path) -> String {
    use std::os::unix::ffi::OsStrExt;
    let mut out = String::from("file://");
    for &b in p.as_os_str().as_bytes() {
        if b.is_ascii_alphanumeric() || b"/-._~".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
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
    let o = std::process::Command::new("xdg-user-dir")
        .arg("DESKTOP")
        .output()
        .ok()?;
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
    d.format_localized("%-d %B %Y, %H:%M", crate::lock::locale())
        .to_string()
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
        p.file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
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
        run_out("xdg-mime", &["query", "filetype", &ps])
            .or_else(|| run_out("file", &["--mime-type", "-b", &ps]))
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
    v.push((
        tr("Propriétaire"),
        format!(
            "{} / {}",
            id_name("/etc/passwd", meta.uid()),
            id_name("/etc/group", meta.gid())
        ),
    ));
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

/// Destinations de partage utilisables pour ce chemin (MLG-310) : copier
/// (toujours) ; un courriel (fichier, `xdg-email` présent) ; un téléphone
/// appairé par KDE Connect (`kdeconnect-cli` présent, appareil joignable).
fn share_targets(path: Option<&Path>) -> Vec<Share> {
    let mut v = vec![Share::Copy];
    let file = path.is_some_and(Path::is_file);
    if file && crate::launch::executable("xdg-email") {
        v.push(Share::Mail);
    }
    if file && crate::launch::executable("kdeconnect-cli") {
        let out = std::process::Command::new("kdeconnect-cli")
            .args(["--list-available", "--id-name-only"])
            .output();
        if let Ok(o) = out {
            v.extend(kdeconnect_devices(&String::from_utf8_lossy(&o.stdout)));
        }
    }
    v
}

/// Lignes « <id> <nom> » de `kdeconnect-cli --id-name-only`.
fn kdeconnect_devices(out: &str) -> Vec<Share> {
    out.lines()
        .filter_map(|l| {
            let (id, name) = l.trim().split_once(' ')?;
            Some(Share::Phone {
                id: id.to_string(),
                name: name.trim().to_string(),
            })
        })
        .collect()
}

/// Envoie le fichier à sa destination.
fn share(p: &Path, t: &Share) -> Result<(), String> {
    use std::process::{Command, Stdio};
    match t {
        Share::Copy => {
            let mut c = Command::new("wl-copy")
                .args(["--type", "text/uri-list"])
                .stdin(Stdio::piped())
                .spawn()
                .map_err(|e| e.to_string())?;
            if let Some(i) = c.stdin.as_mut() {
                use std::io::Write;
                i.write_all(format!("{}\r\n", file_uri(p)).as_bytes())
                    .map_err(|e| e.to_string())?;
            }
            drop(c.stdin.take());
            c.wait().map_err(|e| e.to_string())?;
        }
        Share::Mail => {
            Command::new("xdg-email")
                .arg("--attach")
                .arg(p)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        Share::Phone { id, .. } => {
            Command::new("kdeconnect-cli")
                .args(["--device", id, "--share"])
                .arg(p)
                .spawn()
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

impl App {
    /// Lignes du menu : trois groupes au plus, les plus utiles en tête,
    /// l'action destructive en dernier (HIG Context menus).
    pub(super) fn icon_menu_lines(&self, m: &IconMenu) -> Vec<Line<CMsg>> {
        let tr = |s: &str| crate::i18n::tr(s).to_string();
        if let Some(targets) = &m.share {
            return self.share_lines(m, targets);
        }
        match &m.target {
            MenuTarget::Widget(id) => {
                let own = self.settings.widgets.iter().any(|w| &w.id == id);
                groups(vec![
                    vec![
                        Item::new(
                            tr("Réglages du widget…"),
                            "preferences-system-symbolic",
                            CMsg::WidgetSettings,
                        )
                        .into(),
                        Item::new(
                            tr("Propriétés"),
                            "document-properties-symbolic",
                            CMsg::Properties,
                        )
                        .into(),
                    ],
                    if own {
                        vec![
                            Item::new(
                                tr("Retirer du bureau"),
                                "list-remove-symbolic",
                                CMsg::RemoveWidget,
                            )
                            .into(),
                        ]
                    } else {
                        Vec::new()
                    },
                ])
            }
            MenuTarget::Desktop => {
                let mut add: Vec<Line<CMsg>> = Vec::new();
                if self.cfg.plugin_on("widgets") {
                    add.push(
                        Item::new(
                            tr("Ajouter un widget…"),
                            "list-add-symbolic",
                            CMsg::AddWidget,
                        )
                        .into(),
                    );
                }
                if self.cfg.plugin_on("wallpaper") {
                    add.push(
                        Item::new(
                            tr("Changer de fond d'écran…"),
                            "preferences-desktop-wallpaper-symbolic",
                            CMsg::Wallpapers,
                        )
                        .into(),
                    );
                }
                groups(vec![
                    if self.cfg.plugin_on("icons") {
                        vec![
                            Item::new(
                                tr("Aligner les icônes"),
                                "view-grid-symbolic",
                                CMsg::AlignIcons,
                            )
                            .into(),
                        ]
                    } else {
                        Vec::new()
                    },
                    add,
                    vec![
                        Item::new(
                            tr("Réglages du bureau…"),
                            "preferences-system-symbolic",
                            CMsg::Settings,
                        )
                        .into(),
                        Item::new(
                            tr("Propriétés"),
                            "document-properties-symbolic",
                            CMsg::Properties,
                        )
                        .into(),
                    ],
                ])
            }
            MenuTarget::App(a) => self.app_lines(m, a),
            MenuTarget::Edit(e) => groups(vec![
                vec![
                    Item::new(
                        tr("Couper"),
                        "edit-cut-symbolic",
                        CMsg::Edit(super::edit::Command::Cut),
                    )
                    .enabled(e.can_copy && e.editable)
                    .into(),
                    Item::new(
                        tr("Copier"),
                        "edit-copy-symbolic",
                        CMsg::Edit(super::edit::Command::Copy),
                    )
                    .enabled(e.can_copy)
                    .into(),
                    Item::new(
                        tr("Coller"),
                        "edit-paste-symbolic",
                        CMsg::Edit(super::edit::Command::Paste),
                    )
                    .enabled(e.can_paste && e.editable)
                    .into(),
                ],
                vec![
                    Item::new(
                        tr("Tout sélectionner"),
                        "edit-select-all-symbolic",
                        CMsg::Edit(super::edit::Command::SelectAll),
                    )
                    .into(),
                ],
            ]),
            MenuTarget::Icon => {
                let mut open: Vec<Line<CMsg>> =
                    vec![Item::new(tr("Ouvrir"), "document-open-symbolic", CMsg::Open).into()];
                if m.path.is_some() {
                    open.push(
                        Item::new(
                            tr("Ouvrir le dossier"),
                            "folder-open-symbolic",
                            CMsg::OpenFolder,
                        )
                        .into(),
                    );
                }
                open.extend(self.action_lines(m));
                let mut keep: Vec<Line<CMsg>> = Vec::new();
                if m.path.is_some() || m.own.is_some() {
                    keep.push(
                        Item::new(tr("Renommer…"), "document-edit-symbolic", CMsg::Rename).into(),
                    );
                }
                if m.path.is_some() {
                    keep.push(Item::new(tr("Partager…"), "send-to-symbolic", CMsg::Share).into());
                    keep.push(
                        Item::new(tr("Copier le chemin"), "edit-copy-symbolic", CMsg::CopyPath)
                            .into(),
                    );
                }
                if m.path.as_deref().is_some_and(is_wallpaper) {
                    keep.push(
                        Item::new(
                            tr("Définir comme fond d'écran"),
                            "preferences-desktop-wallpaper-symbolic",
                            CMsg::Wallpaper,
                        )
                        .into(),
                    );
                }
                if let Some(a) = &m.app {
                    keep.extend(self.pin_line(a));
                }
                keep.push(
                    Item::new(
                        tr("Propriétés"),
                        "document-properties-symbolic",
                        CMsg::Properties,
                    )
                    .into(),
                );
                keep.push(
                    Item::new(
                        tr("Réglages des icônes…"),
                        "preferences-system-symbolic",
                        CMsg::Settings,
                    )
                    .into(),
                );
                let mut remove: Vec<Line<CMsg>> = Vec::new();
                if m.own.is_some() {
                    remove.push(
                        Item::new(
                            tr("Retirer du bureau"),
                            "list-remove-symbolic",
                            CMsg::Remove,
                        )
                        .into(),
                    );
                }
                if m.path.is_some() {
                    remove.push(
                        Item::new(
                            if m.confirm {
                                tr("Confirmer : mettre à la corbeille")
                            } else {
                                tr("Mettre à la corbeille")
                            },
                            "user-trash-symbolic",
                            CMsg::Trash,
                        )
                        .danger(true)
                        .into(),
                    );
                }
                groups(vec![open, keep, remove])
            }
        }
    }

    /// Épingler au dock, ou l'en retirer (plugin Dock actif).
    fn pin_line(&self, app: &str) -> Option<Line<CMsg>> {
        let tr = |s: &str| crate::i18n::tr(s).to_string();
        self.cfg.plugin_on("dock").then(|| {
            if self.pinned().iter().any(|p| p == app) {
                Item::new(tr("Retirer du dock"), "list-remove-symbolic", CMsg::Pin).into()
            } else {
                Item::new(tr("Épingler au dock"), "view-pin-symbolic", CMsg::Pin).into()
            }
        })
    }

    /// Actions `.desktop` de l'application visée (MLG-308).
    fn action_lines(&self, m: &IconMenu) -> Vec<Line<CMsg>> {
        m.entry
            .iter()
            .flat_map(|e| e.actions.iter())
            .map(|a| {
                Item::new(
                    a.name.clone(),
                    a.icon
                        .as_deref()
                        .filter(|i| !i.contains('/'))
                        .map(|i| format!("{}-symbolic", i.trim_end_matches("-symbolic")))
                        .as_deref()
                        .unwrap_or("application-x-executable-symbolic"),
                    CMsg::Action(a.id.clone()),
                )
                .into()
            })
            .collect()
    }

    /// Fenêtres ouvertes de l'application, la plus récente d'abord.
    pub(super) fn app_windows(&self, a: &AppRef) -> Vec<&crate::hypr::Client> {
        self.clients
            .iter()
            .filter(|c| {
                a.class.as_deref() == Some(c.class.as_str())
                    || a.id
                        .as_deref()
                        .is_some_and(|id| self.app_matches(id, &c.class))
            })
            .collect()
    }

    /// Menu d'une application (dock, grille) : ses fenêtres ; l'ouvrir et ses
    /// actions ; l'épingler, fermer ses fenêtres (HIG Dock menus).
    fn app_lines(&self, m: &IconMenu, a: &AppRef) -> Vec<Line<CMsg>> {
        let tr = |s: &str| crate::i18n::tr(s).to_string();
        let wins = self.app_windows(a);
        let active = self.active_window.as_ref();
        // Espace affiché de l'écran du menu : les autres sont nommés.
        let here = self
            .monitors
            .iter()
            .find(|x| x.name == m.monitor)
            .map(|x| x.workspace.as_str());
        let windows: Vec<Line<CMsg>> = wins
            .iter()
            .map(|c| {
                let title = if c.title.is_empty() {
                    c.class.clone()
                } else {
                    c.title.clone()
                };
                let place = if c.workspace == crate::hypr::MINIMIZED {
                    Some(tr("réduite"))
                } else if here.is_some_and(|h| h != c.workspace) && !c.workspace.is_empty() {
                    Some(crate::tr!("espace {w}", w = c.workspace))
                } else {
                    None
                };
                Item::new(clip(&title, 34), "", CMsg::Focus(c.address.clone()))
                    .checked(active.is_some_and(|(cl, t)| cl == &c.class && t == &c.title))
                    .detail(place)
                    .into()
            })
            .collect();
        let mut open: Vec<Line<CMsg>> = Vec::new();
        let own_new = m
            .entry
            .as_ref()
            .is_some_and(|e| e.actions.iter().any(|x| x.id == "new-window"));
        if a.id.is_some() {
            if wins.is_empty() {
                open.push(Item::new(tr("Ouvrir"), "document-open-symbolic", CMsg::Launch).into());
            } else if !own_new {
                open.push(
                    Item::new(tr("Nouvelle fenêtre"), "window-new-symbolic", CMsg::Launch).into(),
                );
            }
        }
        open.extend(self.action_lines(m));
        let mut keep: Vec<Line<CMsg>> = Vec::new();
        if let Some(id) = &a.id {
            keep.extend(self.pin_line(id));
        }
        match wins.len() {
            0 => {}
            1 => keep.push(
                Item::new(
                    tr("Fermer la fenêtre"),
                    "window-close-symbolic",
                    CMsg::CloseWindows,
                )
                .danger(true)
                .into(),
            ),
            n => keep.push(
                Item::new(
                    crate::tr!("Fermer les {n} fenêtres", n = n),
                    "window-close-symbolic",
                    CMsg::CloseWindows,
                )
                .danger(true)
                .into(),
            ),
        }
        groups(vec![windows, open, keep])
    }

    /// Destinations de « Partager… » : seules celles qu'on peut utiliser.
    fn share_lines(&self, m: &IconMenu, targets: &[Share]) -> Vec<Line<CMsg>> {
        let tr = |s: &str| crate::i18n::tr(s).to_string();
        let dir = m.path.as_deref().is_some_and(Path::is_dir);
        targets
            .iter()
            .map(|t| {
                let (label, icon) = match t {
                    Share::Copy if dir => (tr("Copier le dossier"), "edit-copy-symbolic"),
                    Share::Copy => (tr("Copier le fichier"), "edit-copy-symbolic"),
                    Share::Mail => (tr("Envoyer par e-mail…"), "mail-send-symbolic"),
                    Share::Phone { name, .. } => (
                        crate::tr!("Envoyer à {name}", name = name),
                        "phone-symbolic",
                    ),
                };
                Item::new(label, icon, CMsg::ShareTo(t.clone())).into()
            })
            .collect()
    }

    /// Titre de la carte (vide : pas de titre).
    fn icon_menu_title(&self, m: &IconMenu) -> String {
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
            MenuTarget::App(a) => a.name.clone(),
            MenuTarget::Edit(_) => String::new(),
        };
        if m.share.is_some() && !name.is_empty() {
            crate::tr!("Partager « {n} »", n = clip(&name, 28))
        } else {
            clip(&name, 40)
        }
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

    fn icon_menu_width(m: &IconMenu) -> f32 {
        if m.props.is_some() {
            return PROPS_WIDTH;
        }
        match m.target {
            MenuTarget::App(_) => APP_WIDTH,
            MenuTarget::Edit(_) => EDIT_WIDTH,
            _ => WIDTH,
        }
    }

    pub(in crate::ui) fn icon_menu_height(&self, m: &IconMenu) -> f32 {
        if let Some(props) = &m.props {
            return props_height(props);
        }
        let title = if self.icon_menu_title(m).is_empty() {
            0.0
        } else {
            super::menu::HEADER
        };
        title + super::menu::height(&self.icon_menu_lines(m)) + 2.0 * PAD
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

    /// Menu d'une surface du bureau, au curseur.
    fn open_menu_with(
        &mut self,
        desk: Id,
        target: MenuTarget,
        idx: usize,
        props: Option<Vec<(String, String)>>,
    ) -> Task<Msg> {
        let Some(monitor) = self.surfaces.get(&desk).cloned() else {
            return self.close_icon_menu();
        };
        let def = match target {
            MenuTarget::Icon => match self.cfg.icons.get(idx) {
                Some(d) => Some(d),
                None => return self.close_icon_menu(),
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
        let mut menu = IconMenu::new(target, desk);
        menu.icon = idx;
        menu.entry = app.as_deref().and_then(crate::launch::desktop_entry);
        menu.path = path;
        menu.app = app;
        menu.own = own;
        menu.props = props;
        menu.monitor = monitor;
        self.spawn_menu(menu, at, false)
    }

    /// Ouvre la carte : au curseur (`exact` faux : son coin en `at`, ombre
    /// comprise), ou surface posée en `at` ; dans l'écran, toujours.
    pub(super) fn spawn_menu(&mut self, mut menu: IconMenu, at: Point, exact: bool) -> Task<Msg> {
        let mut tasks = vec![self.close_icon_menu()];
        // Taille fixe (renommage compris) : la carte ne change pas de surface.
        // Icône : place réservée au champ de renommage et à son erreur.
        let room = if menu.target == MenuTarget::Icon && menu.props.is_none() {
            48.0 + 30.0
        } else {
            0.0
        };
        let full = self.icon_menu_height(&menu) + room + 2.0 * SHADOW;
        let w = Self::icon_menu_width(&menu) + 2.0 * SHADOW;
        let (mut x, mut y) = if exact {
            (at.x, at.y)
        } else {
            (at.x - SHADOW, at.y - SHADOW)
        };
        // Carte plus haute que l'écran (dock : toutes ses fenêtres, toutes ses
        // actions) : la surface est plafonnée et le contenu défile (HIG MENU-02).
        let screen = self.monitors.iter().find(|m| m.name == menu.monitor);
        let h = menu.cap(full, screen.map(|m| m.height as f32));
        if let Some(m) = screen {
            x = x.min(m.width as f32 - w).max(0.0);
            y = y.min(m.height as f32 - h).max(0.0);
        }
        menu.origin = Point::new(x, y);
        menu.win = Id::unique();
        menu.catcher = Id::unique();
        // Dock, grille, champs : leur surface peut être au-dessus de la couche
        // « Top » ; le capteur passe alors par-dessus elle.
        let over = matches!(menu.target, MenuTarget::App(_) | MenuTarget::Edit(_));
        let monitor = menu.monitor.clone();
        for name in super::menu::icons(&self.icon_menu_lines(&menu)) {
            tasks.push(self.want_icon(&name));
        }
        tasks.push(Task::done(Msg::NewLayerShell {
            settings: NewLayerShellSettings {
                size: Some((0, 0)),
                layer: if over { Layer::Overlay } else { Layer::Top },
                anchor: LsAnchor::Top | LsAnchor::Bottom | LsAnchor::Left | LsAnchor::Right,
                exclusive_zone: Some(-1),
                keyboard_interactivity: KeyboardInteractivity::None,
                output_option: OutputOption::OutputName(monitor.clone()),
                namespace: Some("hyprdesk-catcher".into()),
                ..Default::default()
            },
            id: menu.catcher,
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
            id: menu.win,
        }));
        self.icon_menu = Some(menu);
        Task::batch(tasks)
    }

    /// Même menu, autre page (propriétés, partage), au même endroit.
    fn reopen_icon_menu(&mut self, menu: IconMenu) -> Task<Msg> {
        let at = menu.origin;
        self.spawn_menu(menu, at, true)
    }

    /// Écran et position (logique, dans l'écran) du pointeur, demandés à
    /// Hyprland : pour les surfaces qui ne couvrent pas l'écran.
    pub(super) fn pointer_on_screen(&self) -> Option<(String, Point)> {
        let (x, y) = crate::hypr::cursor_pos()?;
        let m = self.monitors.iter().find(|m| {
            x >= m.x && y >= m.y && x < m.x + m.width as f32 && y < m.y + m.height as f32
        })?;
        Some((m.name.clone(), Point::new(x - m.x, y - m.y)))
    }

    /// Menu ouvert sur la grille du Lanceur.
    pub(super) fn grid_menu_open(&self) -> bool {
        let grid = self.search.as_ref().map(|s| s.win);
        self.icon_menu
            .as_ref()
            .is_some_and(|m| Some(m.desk) == grid && matches!(m.target, MenuTarget::App(_)))
    }

    /// Après une application ouverte depuis le menu de la grille : la grille
    /// se referme, comme après un clic sur sa tuile.
    fn leave_grid(&mut self, from_grid: bool) -> Task<Msg> {
        if from_grid && self.search.is_some() {
            self.close_search()
        } else {
            Task::none()
        }
    }

    /// Clic droit sur une application de la grille du Lanceur (MLG-308).
    pub(super) fn open_grid_menu(&mut self, id: String) -> Task<Msg> {
        let Some(surface) = self.search.as_ref().map(|s| s.win) else {
            return Task::none();
        };
        let Some((monitor, at)) = self.pointer_on_screen() else {
            return Task::none();
        };
        let name = self
            .apps
            .iter()
            .find(|a| a.id == id)
            .map_or_else(|| id.clone(), |a| a.name.clone());
        let mut menu = IconMenu::new(
            MenuTarget::App(AppRef {
                id: Some(id.clone()),
                class: None,
                name,
            }),
            surface,
        );
        menu.entry = crate::launch::desktop_entry(&id);
        menu.monitor = monitor;
        self.spawn_menu(menu, at, false)
    }

    /// Clic droit sur un bouton du dock (MLG-307) : le menu de son
    /// application, posé contre le bouton comme sa bulle.
    pub(super) fn open_dock_menu(&mut self, monitor: &str, key: &str) -> Task<Msg> {
        let class_app = |class: &str| {
            self.apps
                .iter()
                .find(|a| self.app_matches(&a.id, class))
                .map(|a| a.id.clone())
        };
        let (id, class) = if let Some(id) = key.strip_prefix("dock-pin:") {
            (Some(id.to_string()), None)
        } else if let Some(class) = key.strip_prefix("dock-app:") {
            (class_app(class), Some(class.to_string()))
        } else if let Some(address) = key.strip_prefix("dock:") {
            let Some(c) = self.clients.iter().find(|c| c.address == address) else {
                return Task::none();
            };
            (class_app(&c.class), Some(c.class.clone()))
        } else {
            return Task::none();
        };
        let name = id
            .as_deref()
            .and_then(|id| self.apps.iter().find(|a| a.id == id))
            .map(|a| a.name.clone())
            .or_else(|| class.clone())
            .unwrap_or_default();
        let surface = self
            .launcher_surfaces
            .iter()
            .find(|(_, s)| s.monitor == monitor)
            .map(|(id, _)| *id)
            .unwrap_or_else(Id::unique);
        let mut menu = IconMenu::new(MenuTarget::App(AppRef { id, class, name }), surface);
        menu.entry = match &menu.target {
            MenuTarget::App(a) => a.id.as_deref().and_then(crate::launch::desktop_entry),
            _ => None,
        };
        menu.monitor = monitor.to_string();
        let size = iced::Size::new(
            Self::icon_menu_width(&menu) + 2.0 * SHADOW,
            self.icon_menu_height(&menu) + 2.0 * SHADOW,
        );
        let Some(at) = self.dock_menu_origin(monitor, key, size, SHADOW) else {
            return Task::none();
        };
        let close = self.close_launcher_popup();
        Task::batch([close, self.spawn_menu(menu, at, true)])
    }

    /// Clavier dans un menu ouvert : flèches et Début/Fin choisissent une
    /// entrée, Entrée (ou Espace) la prend, Échap ferme. Tab avance, Maj+Tab
    /// recule comme la flèche haut (HIG ENT-05). Le champ de renommage garde
    /// ses touches. Dans une carte plafonnée à l'écran, l'entrée choisie
    /// reste visible.
    pub(super) fn icon_menu_key(
        &mut self,
        key: &iced::keyboard::Key,
        shift: bool,
    ) -> Option<Task<Msg>> {
        use iced::keyboard::{Key, key::Named};
        use iced::widget::operation::{AbsoluteOffset, scroll_to};
        let m = self.icon_menu.as_ref()?;
        if m.rename.is_some() || m.props.is_some() {
            return None;
        }
        let lines = self.icon_menu_lines(m);
        let head = if self.icon_menu_title(m).is_empty() {
            0.0
        } else {
            super::menu::HEADER
        };
        let scrolls = m.scrolls;
        let hover = m.hover;
        let next = match key {
            Key::Named(Named::Escape) => return Some(self.close_icon_menu()),
            Key::Named(Named::Tab) => super::menu::step(&lines, hover, !shift),
            Key::Named(Named::ArrowDown) => super::menu::step(&lines, hover, true),
            Key::Named(Named::ArrowUp) => super::menu::step(&lines, hover, false),
            Key::Named(Named::Home | Named::PageUp) => super::menu::edge(&lines, true),
            Key::Named(Named::End | Named::PageDown) => super::menu::edge(&lines, false),
            Key::Named(Named::Enter | Named::Space) => {
                let msg = hover.and_then(|i| match lines.get(i) {
                    Some(Line::Item(it)) if it.enabled => Some(it.msg.clone()),
                    _ => None,
                });
                return Some(match msg {
                    Some(msg) => self.icon_menu_update(msg),
                    None => Task::none(),
                });
            }
            _ => return None,
        };
        let Some(m) = self.icon_menu.as_mut() else {
            return Some(Task::none());
        };
        m.hover = next;
        // Carte plafonnée : le défilement suit l'entrée choisie.
        if let (true, Some(i)) = (scrolls, next) {
            let top = super::menu::height;
            m.offset = follow(
                m.offset,
                m.viewport,
                head + top(&lines),
                head + top(&lines[..i]),
                top(&lines[i..i + 1]),
            );
            return Some(scroll_to(
                SCROLL,
                AbsoluteOffset {
                    x: 0.0,
                    y: m.offset,
                },
            ));
        }
        Some(Task::none())
    }

    /// Clic droit dans un champ de saisie (MLG-309). Pas dans le champ de
    /// renommage du menu lui-même : le menu se fermerait sur lui.
    pub(super) fn open_edit_menu(&mut self, surface: Id, can_copy: bool) -> Task<Msg> {
        if self.icon_menu.as_ref().is_some_and(|m| m.win == surface) {
            return Task::none();
        }
        let Some((monitor, at)) = self.pointer_on_screen() else {
            return Task::none();
        };
        let mut menu = IconMenu::new(
            MenuTarget::Edit(EditRef {
                surface,
                can_copy,
                editable: true,
                can_paste: super::edit::clipboard_has_text(),
            }),
            surface,
        );
        menu.monitor = monitor;
        self.spawn_menu(menu, at, false)
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
                let label = self
                    .icons
                    .get(idx)
                    .map(|r| r.label.clone())
                    .unwrap_or_default();
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
                let app = match &menu.target {
                    MenuTarget::App(a) => a.id.clone(),
                    _ => menu.app.clone(),
                };
                let close = self.close_icon_menu();
                let Some(a) = app else {
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
            CMsg::Focus(address) => {
                let grid = self.grid_menu_open();
                let close = self.close_icon_menu();
                let leave = self.leave_grid(grid);
                Task::batch([close, leave, Task::done(Msg::FocusWindow(address))])
            }
            CMsg::Launch => {
                let id = match &menu.target {
                    MenuTarget::App(a) => a.id.clone(),
                    _ => None,
                };
                let grid = self.grid_menu_open();
                let close = self.close_icon_menu();
                let leave = self.leave_grid(grid);
                match id {
                    Some(id) => {
                        if grid {
                            // Comme une tuile : le lancement compte pour les suggestions.
                            let rec = id.clone();
                            std::thread::spawn(move || crate::search::record_launch(&rec));
                        }
                        Task::batch([close, leave, Task::done(Msg::LauncherLaunch(id))])
                    }
                    None => Task::batch([close, leave]),
                }
            }
            CMsg::Action(action) => {
                let id = match &menu.target {
                    MenuTarget::App(a) => a.id.clone(),
                    _ => menu.app.clone(),
                };
                let command = menu.entry.as_ref().and_then(|e| {
                    let a = e.actions.iter().find(|a| a.id == action)?;
                    Some(crate::launch::action_command(
                        e,
                        a,
                        &self.cfg.general.terminal,
                    ))
                });
                let grid = self.grid_menu_open();
                let close = self.close_icon_menu();
                let Some(command) = command else {
                    return close;
                };
                let close = Task::batch([close, self.leave_grid(grid)]);
                // Retour du dock (rebond) quand l'application est connue et
                // qu'aucun lancement n'est déjà suivi ; sinon, lancée telle quelle.
                match id.filter(|id| !self.launcher_runtime.launches.contains_key(id)) {
                    Some(id) => {
                        let feedback = self.launcher_launch_started(id.clone());
                        let token = self.launcher_runtime.launches[&id].started;
                        Task::batch([
                            close,
                            feedback,
                            Task::done(Msg::LauncherResolved(id, token, command)),
                        ])
                    }
                    None => Task::batch([close, self.run_from_desktop(command, true)]),
                }
            }
            CMsg::CloseWindows => {
                let addresses: Vec<String> = match &menu.target {
                    MenuTarget::App(a) => {
                        let a = a.clone();
                        self.app_windows(&a)
                            .into_iter()
                            .map(|c| c.address.clone())
                            .collect()
                    }
                    _ => Vec::new(),
                };
                let close = self.close_icon_menu();
                let work = Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            for a in addresses {
                                let _ = crate::hypr::close_window(&a);
                            }
                        })
                        .await
                    },
                    |_| Msg::Noop,
                );
                Task::batch([close, work])
            }
            CMsg::Share => {
                let Some(mut m) = self.icon_menu.take() else {
                    return Task::none();
                };
                let close =
                    Task::batch([iced::window::close(m.win), iced::window::close(m.catcher)]);
                m.share = Some(share_targets(m.path.as_deref()));
                m.hover = None;
                m.confirm = false;
                Task::batch([close, self.reopen_icon_menu(m)])
            }
            CMsg::ShareTo(t) => {
                let close = self.close_icon_menu();
                let Some(p) = path else {
                    return close;
                };
                if let Err(e) = share(&p, &t) {
                    super::hypr::notify(0, 5000, &crate::tr!("Partage : {e}", e = e));
                }
                close
            }
            CMsg::Edit(cmd) => {
                let surface = match &menu.target {
                    MenuTarget::Edit(e) => Some(e.surface),
                    _ => None,
                };
                let close = self.close_icon_menu();
                match surface {
                    Some(s) => Task::batch([close, super::edit::run(s, cmd)]),
                    None => close,
                }
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
                                None => v.push((
                                    tr("Fichier"),
                                    tr("introuvable (application désinstallée ?)"),
                                )),
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
            // Pas de « Propriétés » dans ces menus.
            MenuTarget::App(_) | MenuTarget::Edit(_) => {}
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
                            crate::tr!(
                                "couleur {c}",
                                c = format!("#{:02x}{:02x}{:02x}", c.0, c.1, c.2)
                            )
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
    fn view_properties<'a>(
        &'a self,
        name: &str,
        props: &'a [(String, String)],
        scrolls: bool,
    ) -> Element<'a, Msg> {
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let dim = Color { a: 0.6, ..fg };
        let accent = color(th.accent, 1.0);
        let bg = color(
            th.bg,
            th.shown(crate::config::Place::Menus).opacity.max(0.97),
        );
        let mut col = column![
            text(crate::tr!("Propriétés — {n}", n = name))
                .font(self.title_font())
                .size(crate::ui::tokens::ts(14.0))
                .color(fg)
                .height(ROW)
        ]
        .spacing(6);
        let mut list = column![].spacing(6);
        for (k, v) in props {
            list = list.push(
                column![
                    text(k.clone())
                        .font(self.font)
                        .size(crate::ui::tokens::ts(11.0))
                        .color(dim),
                    text(v.clone())
                        .font(self.font)
                        .size(crate::ui::tokens::ts(13.0))
                        .color(fg)
                        .wrapping(iced::widget::text::Wrapping::WordOrGlyph),
                ]
                .spacing(1),
            );
        }
        // Carte plafonnée à l'écran : les propriétés défilent, le titre et les
        // commandes restent (HIG MENU-02).
        let list: Element<'_, Msg> = if scrolls {
            iced::widget::scrollable(list)
                .id(SCROLL)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            list.into()
        };
        col = col.push(list);
        let btn = |label: &'static str, msg: CMsg| {
            button(
                text(crate::i18n::tr(label))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
                    .color(fg),
            )
            .padding([5, 12])
            .style(move |_, st| button::Style {
                background: Some(
                    Color {
                        a: if matches!(st, button::Status::Hovered) {
                            0.22
                        } else {
                            0.12
                        },
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
            .height(if scrolls {
                Length::Fill
            } else {
                Length::Shrink
            })
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
        let card = container(card).padding(SHADOW);
        if scrolls {
            card.height(Length::Fill).into()
        } else {
            card.into()
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
        let bg = color(
            th.bg,
            th.shown(crate::config::Place::Menus).opacity.max(0.97),
        );
        let name = self.icon_menu_title(m);
        if let Some(props) = &m.props {
            return self.view_properties(&name, props, m.scrolls);
        }
        let ink = self.menu_ink(1.0);
        let mut col = column![].spacing(0);
        if !name.is_empty() {
            col = col.push(self.menu_header(ink, name));
        }
        col = col.push(self.menu_lines(ink, &self.icon_menu_lines(m), m.hover, Msg::IconMenu));
        // Carte plafonnée à l'écran : les lignes défilent dans la carte, qui
        // garde son coin et son ombre ; le renommage, lui, reste en bas.
        let scrolls = m.scrolls;
        let mut col = if scrolls {
            column![
                iced::widget::scrollable(col)
                    .id(SCROLL)
                    .width(Length::Fill)
                    .height(Length::Fill)
            ]
            .spacing(0)
        } else {
            col
        };
        if let Some(r) = &m.rename {
            col = col.push(
                text_input(crate::i18n::tr("Nouveau nom"), r)
                    .id(INPUT_ID)
                    .on_input(|s| Msg::IconMenu(CMsg::RenameInput(s)))
                    .on_submit(Msg::IconMenu(CMsg::RenameSubmit))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(14.0))
                    .padding(8),
            );
        }
        if let Some(e) = &m.error {
            col = col.push(
                text(e.clone())
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(crate::ui::tone::warning()),
            );
        }
        let card = container(col)
            .padding(PAD)
            .width(Self::icon_menu_width(m))
            .height(if scrolls {
                Length::Fill
            } else {
                Length::Shrink
            })
            .style(move |_| container::Style {
                background: Some(bg.into()),
                border: Border::default()
                    // Coins concentriques : ceux des lignes plus la marge.
                    .rounded(super::design::SMALL_RADIUS + PAD)
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
        let card = container(card).padding(SHADOW);
        if scrolls {
            card.height(Length::Fill).into()
        } else {
            card.into()
        }
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
        assert!(
            human_size(1_468_006).contains("1 468 006")
                || human_size(1_468_006).contains("1468006")
        );
    }

    #[test]
    fn noms_valides() {
        assert!(valid_name("Rapport 2026.pdf"));
        assert!(!valid_name("  "));
        assert!(!valid_name("a/b"));
        assert!(!valid_name(".."));
    }

    fn ecran(height: u32) -> crate::hypr::Monitor {
        crate::hypr::Monitor {
            dpms_off: false,
            name: "A".into(),
            width: 1920,
            height,
            workspace: "1".into(),
            base: String::new(),
            focused: true,
            scale: 1.0,
            reserved: (0.0, 0.0, 0.0, 0.0),
            x: 0.0,
            y: 0.0,
        }
    }

    /// Le dock : `n` fenêtres d'une même application.
    fn dock_et_ses_fenetres(n: usize) -> (App, IconMenu) {
        let mut app = app();
        app.monitors = vec![ecran(1080)];
        app.clients = (0..n)
            .map(|k| crate::hypr::Client {
                address: format!("0x{k:x}"),
                class: "firefox".into(),
                title: format!("Fenêtre {k}"),
                workspace: "1".into(),
                monitor: "A".into(),
            })
            .collect();
        let mut m = IconMenu::new(
            MenuTarget::App(AppRef {
                id: Some("firefox".into()),
                class: None,
                name: "Firefox".into(),
            }),
            Id::unique(),
        );
        m.monitor = "A".into();
        (app, m)
    }

    /// Une carte plus haute que l'écran (dock : toutes ses fenêtres) est
    /// plafonnée à l'écran et défile ; une carte qui tient garde sa taille
    /// et sa surface fixe (HIG MENU-02).
    #[test]
    fn carte_plafonnee_a_l_ecran() {
        assert_eq!(window_height(400.0, 1080.0), (400.0, false));
        assert_eq!(window_height(2000.0, 1080.0), (1080.0, true));

        let (mut app, m) = dock_et_ses_fenetres(2);
        let _ = app.spawn_menu(m, Point::new(10.0, 10.0), false);
        let menu = app.icon_menu.as_ref().unwrap();
        assert!(!menu.scrolls && menu.viewport == 0.0);

        let (mut app, m) = dock_et_ses_fenetres(80);
        let _ = app.spawn_menu(m, Point::new(10.0, 10.0), false);
        let menu = app.icon_menu.as_ref().unwrap();
        assert!(menu.scrolls);
        // La surface occupe l'écran ; la carte y garde son ombre et sa marge.
        assert!((menu.viewport - (1080.0 - 2.0 * (SHADOW + PAD))).abs() < 0.5);
        assert!(menu.origin.y >= 0.0 && menu.origin.y <= 2.0 * SHADOW);
    }

    #[test]
    fn le_defilement_suite_l_entre_choisie() {
        // Déjà visible : rien ne bouge.
        assert_eq!(follow(0.0, 300.0, 900.0, 100.0, 30.0), 0.0);
        assert_eq!(follow(600.0, 300.0, 900.0, 610.0, 30.0), 600.0);
        // Sous la fenêtre : elle en devient le bas ; au-dessus : le haut.
        assert_eq!(follow(0.0, 300.0, 900.0, 290.0, 30.0), 20.0);
        assert_eq!(follow(600.0, 300.0, 900.0, 590.0, 30.0), 590.0);
        // Jamais au-delà du contenu.
        assert_eq!(follow(0.0, 300.0, 900.0, 880.0, 30.0), 600.0);
        assert_eq!(follow(600.0, 300.0, 900.0, 0.0, 30.0), 0.0);
    }

    /// Tab avance, Maj+Tab recule comme la flèche haut (HIG ENT-05) ; dans
    /// une carte plafonnée, le défilement mène l'entrée choisie.
    #[test]
    fn maj_tab_recule_et_la_carte_suite() {
        use super::super::menu::{HEADER, height};
        use iced::keyboard::key::Named;
        let key = iced::keyboard::Key::Named;
        let hover = |a: &App| a.icon_menu.as_ref().and_then(|m| m.hover);

        let (mut app, m) = dock_et_ses_fenetres(2);
        let _ = app.spawn_menu(m, Point::new(10.0, 10.0), false);
        let _ = app.icon_menu_key(&key(Named::Tab), false);
        assert_eq!(hover(&app), Some(0));
        let _ = app.icon_menu_key(&key(Named::Tab), false);
        assert_eq!(hover(&app), Some(1));
        let _ = app.icon_menu_key(&key(Named::Tab), true);
        assert_eq!(hover(&app), Some(0));
        let up = app.icon_menu_key(&key(Named::Tab), true).is_some();
        let last = {
            let menu = app.icon_menu.as_ref().unwrap();
            super::super::menu::edge(&app.icon_menu_lines(menu), false)
        };
        assert!(up && hover(&app) == last, "Maj+Tab boucle sur la dernière");

        let (mut app, m) = dock_et_ses_fenetres(80);
        let _ = app.spawn_menu(m, Point::new(10.0, 10.0), false);
        let _ = app.icon_menu_key(&key(Named::ArrowDown), false);
        let _ = app.icon_menu_key(&key(Named::ArrowDown), false);
        let menu = app.icon_menu.as_ref().unwrap();
        assert_eq!(menu.hover, Some(1));
        assert_eq!(menu.offset, 0.0, "les premières entrées sont visibles");
        let _ = app.icon_menu_key(&key(Named::End), false);
        let menu = app.icon_menu.as_ref().unwrap();
        let lines = app.icon_menu_lines(menu);
        let head = if app.icon_menu_title(menu).is_empty() {
            0.0
        } else {
            HEADER
        };
        assert_eq!(menu.offset, head + height(&lines) - menu.viewport);
    }

    fn app() -> App {
        App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-menus-tests/config.lua"),
            true,
        )
        .0
    }

    fn items(lines: &[Line<CMsg>]) -> Vec<&super::super::menu::Item<CMsg>> {
        lines
            .iter()
            .filter_map(|l| match l {
                Line::Item(i) => Some(i),
                Line::Separator => None,
            })
            .collect()
    }

    /// Fichier du bureau : trois groupes au plus, une icône partout, la
    /// corbeille en dernier et en rouge ; « Partager… » proposé.
    #[test]
    fn menu_d_un_fichier() {
        let app = app();
        let mut m = IconMenu::new(MenuTarget::Icon, Id::unique());
        m.path = Some("/tmp/hyprdesk-menus-tests/Rapport.pdf".into());
        let lines = app.icon_menu_lines(&m);
        let seps = lines
            .iter()
            .filter(|l| matches!(l, Line::Separator))
            .count();
        assert!(seps <= 2, "{seps} séparateurs");
        assert!(!matches!(lines.first(), Some(Line::Separator)));
        let it = items(&lines);
        assert!(it.iter().all(|i| i.icon.is_some()));
        assert!(
            it.last()
                .is_some_and(|i| i.danger && matches!(i.msg, CMsg::Trash))
        );
        assert!(it.iter().any(|i| matches!(i.msg, CMsg::Share)));
        assert_eq!(it.iter().filter(|i| i.danger).count(), 1);
    }

    /// Application du dock avec deux fenêtres : elles d'abord, la sienne au
    /// premier plan cochée ; ses actions (sa propre « Nouvelle fenêtre » : pas
    /// de doublon) ; puis fermer les deux, en rouge.
    #[test]
    fn menu_du_dock() {
        let mut app = app();
        let c = |a: &str, t: &str, ws: &str| crate::hypr::Client {
            address: a.into(),
            class: "firefox".into(),
            title: t.into(),
            workspace: ws.into(),
            monitor: "A".into(),
        };
        app.clients = vec![c("0x1", "Un", "1"), c("0x2", "Deux", "2")];
        app.active_window = Some(("firefox".into(), "Deux".into()));
        let mut m = IconMenu::new(
            MenuTarget::App(AppRef {
                id: Some("firefox".into()),
                class: None,
                name: "Firefox".into(),
            }),
            Id::unique(),
        );
        m.entry = crate::launch::parse_desktop_entry(
            "[Desktop Entry]\nName=Firefox\nExec=firefox\nActions=new-window;private;\n\
             [Desktop Action new-window]\nName=Nouvelle fenêtre\nExec=firefox --new-window\n\
             [Desktop Action private]\nName=Privée\nExec=firefox --private-window\n",
            "fr",
        );
        let lines = app.icon_menu_lines(&m);
        let it = items(&lines);
        assert!(matches!(&it[0].msg, CMsg::Focus(a) if a == "0x1") && !it[0].checked);
        assert!(matches!(&it[1].msg, CMsg::Focus(a) if a == "0x2") && it[1].checked);
        assert!(!it.iter().any(|i| matches!(i.msg, CMsg::Launch)));
        assert_eq!(
            it.iter()
                .filter(|i| matches!(i.msg, CMsg::Action(_)))
                .count(),
            2
        );
        assert!(
            it.last()
                .is_some_and(|i| i.danger && matches!(i.msg, CMsg::CloseWindows))
        );
        // Application fermée : « Ouvrir », rien à fermer.
        app.clients.clear();
        let it2 = app.icon_menu_lines(&m);
        let it2 = items(&it2);
        assert!(matches!(it2[0].msg, CMsg::Launch));
        assert!(!it2.iter().any(|i| matches!(i.msg, CMsg::CloseWindows)));
    }

    /// Champ de saisie : Couper et Copier grisés sans sélection, Coller sans
    /// texte à coller ; Tout sélectionner toujours.
    #[test]
    fn menu_d_edition() {
        let app = app();
        let edit = |can_copy, can_paste| {
            let m = IconMenu::new(
                MenuTarget::Edit(EditRef {
                    surface: Id::unique(),
                    can_copy,
                    editable: true,
                    can_paste,
                }),
                Id::unique(),
            );
            items(&app.icon_menu_lines(&m))
                .iter()
                .map(|i| i.enabled)
                .collect::<Vec<_>>()
        };
        assert_eq!(edit(false, true), [false, false, true, true]);
        assert_eq!(edit(true, false), [true, true, false, true]);
    }

    #[test]
    fn partage() {
        // Un dossier se copie, il ne part pas en pièce jointe.
        assert_eq!(share_targets(Some(Path::new("/tmp"))), [Share::Copy]);
        assert_eq!(
            kdeconnect_devices("abc123 Pixel 8\n\n"),
            [Share::Phone {
                id: "abc123".into(),
                name: "Pixel 8".into()
            }]
        );
        assert_eq!(
            file_uri(Path::new("/home/u/Mes notes/été #1.txt")),
            "file:///home/u/Mes%20notes/%C3%A9t%C3%A9%20%231.txt"
        );
        assert_eq!(clip("abcdef", 4), "abc…");
        assert_eq!(clip("abc", 4), "abc");
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
