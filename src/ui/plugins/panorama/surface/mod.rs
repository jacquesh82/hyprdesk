//! Panorama (MLG-164) : surface plein écran tactile des écrans bandeau.
//!
//! - une surface layer-shell par écran au rôle Panorama (calque Top, jamais
//!   de focus clavier : toucher le Panorama ne prend pas le clavier à la
//!   fenêtre de l'écran principal) ;
//! - des pages qu'on fait glisser (`swipe`), au doigt ou à la souris ;
//! - escamotable : une poignée sur le bord du bas la fait revenir ;
//! - proposée d'elle-même à la détection d'un écran candidat
//!   (`crate::panorama::detect`), une fois par écran.

mod dashboard;
mod drag;
mod flux;
mod fx;
mod halo;
mod hold;
mod instruments;
mod modules;
mod panes;
mod side;
mod swipe;

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use iced::widget::{Space, button, column, container, image, row, text};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Task};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use crate::panorama::detect::{self, Offer, Screen};
use crate::panorama::layout::{self, Orientation, Zones};
use crate::ui::{App, Msg, color};
use crate::widgets::media::Status;

/// Poignée de l'écran escamoté.
const HANDLE_W: u32 = 180;
const HANDLE_H: u32 = 26;
/// Clé de page et d'état de la fenêtre d'aperçu (pas un écran).
pub const PREVIEW: &str = "\u{0}aperçu";
/// app_id de la fenêtre d'aperçu (règle de Hyprland : flottante, centrée).
pub const PREVIEW_APP_ID: &str = "hyprdesk-panorama-preview";
/// Format simulé par défaut (Xeneon Edge et ses semblables).
pub const PREVIEW_SIZE: (u32, u32) = (2560, 720);
/// Densité simulée d'un écran bandeau de 14,5″ (≈ 183 ppp), en px/mm.
const PREVIEW_PX_PER_MM: f32 = 7.4;
/// Part maximale de l'écran courant occupée par l'aperçu.
const PREVIEW_FIT: f32 = 0.85;

/// Fenêtre d'aperçu : le Panorama dans une fenêtre flottante.
#[derive(Debug, Clone, Copy)]
pub struct Preview {
    pub win: Id,
    /// Format simulé (pixels logiques de l'écran bandeau).
    pub virtual_size: (u32, u32),
    /// Taille de la fenêtre / taille simulée (≤ 1 : l'aperçu tient à l'écran).
    pub scale: f32,
}

/// Taille de la fenêtre pour un format simulé sur un écran `screen` :
/// le format lui-même s'il tient dans 85 % de l'écran, réduit sinon.
pub fn preview_fit(virtual_size: (u32, u32), screen: (u32, u32)) -> ((u32, u32), f32) {
    let (vw, vh) = (virtual_size.0.max(1) as f32, virtual_size.1.max(1) as f32);
    let (sw, sh) = (screen.0.max(1) as f32, screen.1.max(1) as f32);
    let scale = (PREVIEW_FIT * sw / vw).min(PREVIEW_FIT * sh / vh).min(1.0);
    (
        ((vw * scale).round() as u32, (vh * scale).round() as u32),
        scale,
    )
}

/// `WxH` (`2560x720`, `720×2560`).
pub fn parse_size(s: &str) -> Option<(u32, u32)> {
    let (w, h) = s.split_once(['x', 'X', '×'])?;
    let (w, h) = (w.trim().parse().ok()?, h.trim().parse().ok()?);
    ((200..=8000).contains(&w) && (200..=8000).contains(&h)).then_some((w, h))
}

fn preview_title(size: (u32, u32)) -> String {
    crate::tr!("Panorama — aperçu {w}×{h}", w = size.0, h = size.1)
}

/// Règle de Hyprland de la fenêtre d'aperçu (remplace la précédente).
fn preview_rule_lua(w: u32, h: u32) -> String {
    format!(
        "if __hyprdesk_panorama_preview then pcall(function() __hyprdesk_panorama_preview:set_enabled(false) end) end\n\
         __hyprdesk_panorama_preview = hl.window_rule({{ name = \"hyprdesk-panorama-preview\", \
         match = {{ class = \"^({PREVIEW_APP_ID})$\" }}, float = true, center = true, size = \"{w} {h}\" }})\n"
    )
}

/// Relevés (lecteur, système) tant qu'un Panorama est affiché.
pub const TICK: Duration = Duration::from_secs(1);

/// Une vidéo est posée dans le Panorama (fenêtre d'application ou fenêtre
/// empruntée) et un lecteur joue.
pub(in crate::ui) fn video_playing(
    video: Option<&panes::Video>,
    media: Option<&crate::widgets::media::Status>,
) -> bool {
    video.is_some_and(|v| v.address.is_some() || v.borrowed.is_some())
        && media.is_some_and(|m| m.playing)
}

/// Vidéo posée dans sa zone sans navigateur : aperçu isolé (`hyprdesk
/// preview`), pour voir le trou et le voile qui l'entoure.
pub(in crate::ui) fn preview_video(name: &str) -> panes::Video {
    panes::Video {
        name: name.into(),
        url: String::new(),
        prefix: String::new(),
        address: Some("0xpreview".into()),
        known: Vec::new(),
        error: None,
        placed: None,
        focus_back: None,
        borrowed: None,
    }
}

/// Module de barre de LYRA vocale (`assets/widgets/integres/voice.lua`).
pub const VOICE_WIDGET: &str = "voice";

const ICONS: &[&str] = &[
    "media-playback-start-symbolic",
    "media-playback-pause-symbolic",
    "media-skip-forward-symbolic",
    "media-skip-backward-symbolic",
    "audio-x-generic-symbolic",
    "go-down-symbolic",
    "go-up-symbolic",
    "window-close-symbolic",
];

#[derive(Debug, Default)]
pub struct State {
    /// Surface → écran.
    pub surfaces: HashMap<Id, String>,
    /// Poignée (Panorama escamoté) → écran.
    pub handles: HashMap<Id, String>,
    /// Carte de proposition → offre.
    pub offers: HashMap<Id, Offer>,
    /// Écrans escamotés (cette session).
    pub hidden: HashSet<String>,
    /// Page affichée, par écran.
    pub page: HashMap<String, usize>,
    /// Écrans tels que Hyprland les décrit (EDID, taille physique).
    pub screens: Vec<Screen>,
    /// Périphériques tactiles.
    pub touch: Vec<String>,
    /// Écrans (identifiant EDID) déjà proposés cette session.
    pub offered: HashSet<String>,
    pub media: Option<Status>,
    pub sys: Vec<crate::widgets::SysLine>,
    /// Dernier relevé de chaque mesure (tuiles `metric:<id>`).
    pub sys_ids: HashMap<String, crate::widgets::SysLine>,
    /// Dernier toucher (la veille attend `sleep_minutes` sans toucher ni lecture).
    pub last_press: Option<Instant>,
    pub cover: Option<(String, image::Handle)>,
    /// Fenêtre d'aperçu ouverte.
    pub preview: Option<Preview>,
    /// Vidéo du widget Vidéo (fenêtre de navigateur posée dans sa zone).
    pub video: Option<panes::Video>,
    /// Inhibiteur logind `idle` tenu pendant qu'une vidéo du Panorama joue
    /// (MLG-249) : une fenêtre empruntée (PiP d'un navigateur) ne retient
    /// pas elle-même la veille.
    pub idle_hold: Option<std::process::Child>,
    pub capture_watch: crate::panorama::capture::Watch,
    pub capture_busy: bool,
    pub capture_host: Option<Id>,
    /// Carte « Téléphone » de la zone vidéo (QR de la page du cast, MLG-217).
    pub cast_card: bool,
    /// Lancements abandonnés avant que leur fenêtre n'apparaisse : elle sera
    /// fermée à son arrivée (une seule source à la fois).
    pub strays: Vec<panes::Stray>,
    /// Réglage du cadrage de la source ouverte (colonne des services).
    pub framing: bool,
    /// Connexion lue dans le navigateur, par adresse de source (`auth`) ;
    /// dernier relevé.
    pub signed: HashMap<String, Option<bool>>,
    pub signed_at: Option<Instant>,
    /// Liste de sources réglée dans les Réglages : video, music, radio.
    pub sources_kind: &'static str,
    /// LYRA vocale : état affiché par le bouton micro.
    pub voice: VoiceUi,
    /// Dernier état écrit pour le module de barre `voice` (`voice_publish`).
    pub voice_written: String,
    /// Accord demandé par LYRA pendant une demande vocale : (demande, langue).
    pub voice_confirm: Option<(String, String)>,
    /// Micros de PipeWire : (nœud, libellé, par défaut).
    pub voice_mics: Vec<(String, String, bool)>,
    /// Dernière phrase entendue par le micro (diagnostic).
    pub voice_heard: Option<String>,
    /// Surfaces du halo de LYRA vocale (écran de chacune).
    pub halo: HashMap<Id, String>,
    pub halo_since: Option<Instant>,
    /// Zone Panorama dont la surface est posée (pour la refaire si elle change).
    pub zone_applied: Option<crate::panorama::config::Zone>,
    /// Dernier rattachement du tactile vérifié : (écran visé, écrans branchés).
    pub touch_checked: Option<(String, Vec<String>)>,
    /// Webradio jouée par mpv : (station, flux) ; en pause.
    pub radio: Option<(String, String)>,
    pub radio_paused: bool,
    /// Niveaux du son (global, graves, médiums, aigus) et leur historique.
    pub levels: [f32; 4],
    pub level_hist: std::collections::VecDeque<f32>,
    /// Dernier relevé des niveaux (défilement continu du flux).
    pub levels_at: Option<Instant>,
    /// Niveaux très lissés du fond, hauteur courante de l'onde, images.
    pub bg_levels: [f32; 4],
    pub wave: f32,
    pub flux_frames: u32,
    pub flux_t0: FluxClock,
    /// Dernier relevé des mesures (défilement continu des courbes).
    pub sys_at: Option<Instant>,
    /// Gestionnaire des tâches ouvert (tuile de mesure touchée).
    pub tasks: Option<TasksView>,
    /// Fenêtres du lecteur laissées par un hyprdesk précédent : vérifiées
    /// une fois (reprises, ou fermées).
    pub orphans_checked: bool,
    /// Sources (adresses) dont l'icône a déjà été demandée à SmartUI cette
    /// session : une source sans icône est dessinée une fois, d'elle-même.
    pub icons_tried: Vec<String>,
    /// Fenêtres du lecteur déjà passées (ou essayées) en vidéo seule : une
    /// fois par fenêtre, la personne peut en sortir.
    pub autofull_done: Vec<String>,
    /// Onglet de la colonne du profil Musique : playlist ou radio.
    pub music_tab: &'static str,
    /// Dernière recherche de radios : (demande, stations) ; en cours.
    pub radio_results: Option<(String, Vec<crate::panorama::config::Source>)>,
    pub radio_busy: bool,
    pub radio_error: Option<String>,
    /// La radio lancée se connecte encore (tampon de mpv pas rempli).
    pub radio_connecting: bool,
    /// Logos des radios favorites déjà cherchés (une fois par session).
    pub radio_logos_tried: bool,
    /// Zone de saisie posée sur chaque surface : ses trous (vidéo).
    pub regions: HashMap<Id, Vec<layout::Rect>>,
    /// Position globale de la fenêtre d'aperçu (la vidéo s'y pose quand
    /// aucun écran Panorama n'a de zone Vidéo).
    pub preview_at: Option<(i32, i32)>,
    /// Réglages : nouvelle source (nom, adresse) en cours de saisie.
    pub draft: (String, String),
    /// Réglages : recherche de radio en cours de saisie.
    pub radio_query: String,
    /// Réglages : navigateur en cours de saisie.
    pub browser_draft: Option<String>,
    pub design_drafts: HashMap<String, String>,
    pub design_error: Option<String>,
    /// Sélecteur de couleur ouvert, et la clé de son réglage (HIG SEL-02).
    pub design_picker: Option<(String, crate::ui::color_picker::Picker)>,
    pub selected_tile: usize,
    pub shortcut_draft: (String, String, String),
    /// Réglages → Panorama : disposition dont on règle les zones.
    pub edit_layout: Option<crate::panorama::config::Layout>,
    /// Composition en cours de réglage : vue par défaut (`None`) ou page.
    pub edit_board: Option<usize>,
    /// Noms de page en cours de saisie (Réglages).
    pub page_names: HashMap<usize, String>,
    /// Écran où le choix de la disposition est ouvert (geste de bas en haut).
    pub picker: Option<String>,
    /// Écran où les notifications sont ouvertes (geste du bord droit).
    pub notifs: Option<String>,
    /// Bloc des raccourcis : 0 les touches, 1 les fenêtres ouvertes.
    pub shortcuts_page: usize,
    /// Tuile Hue : pont appairé, pièces, dernière erreur, dernier relevé.
    pub hue_paired: bool,
    pub hue: Vec<crate::panorama::modules::hue::Room>,
    pub hue_error: Option<String>,
    pub hue_at: Option<Instant>,
    /// Premier relevé du pont reçu (avant : « Lecture… »).
    pub hue_read: bool,
    /// Curseur de luminosité tenu : (lampe ? sinon pièce), identifiant, valeur.
    pub hue_drag: Option<(bool, String, u8)>,
    /// Lampe désignée dans la tuile Hue.
    pub hue_focus: Option<String>,
    /// Bande des modules : catalogue ouvert (ajouter, retirer).
    pub modules_edit: bool,
    /// Tuile Notes : échec du dernier lancement.
    pub notes_error: Option<String>,
    /// Réglages : `hyprdesk panorama icons` en cours.
    pub icons_job: Option<String>,
    /// Compte rendu de la dernière génération d'icônes.
    pub icons_report: Option<Result<String, String>>,
    pub history: HashMap<String, std::collections::VecDeque<f32>>,
    pub wallpaper: Option<(String, image::Handle, iced::Size)>,
    pub wallpaper_requested: String,
    /// Profil Système : état des mises à jour relu, relevé déjà demandé.
    pub updates: Option<crate::updates::Snapshot>,
    pub updates_asked: bool,
    /// Touche lourde armée (éteindre, redémarrer) : (rang, depuis).
    pub confirm: Option<(usize, Instant)>,
    /// Côte à côte des profils Vidéo et Musique (média | zone d'accueil).
    pub split: Option<side::SideBySide>,
}

/// Gestionnaire des tâches du Panorama : sur quel écran, trié sur quoi.
#[derive(Debug)]
pub struct TasksView {
    pub key: String,
    pub sort: &'static str,
    /// Mesure affichée à gauche (celle de la tuile, puis celle du tri).
    pub metric: String,
    pub list: Vec<crate::panorama::tasks::Proc>,
    pub sampler: std::sync::Arc<std::sync::Mutex<crate::panorama::tasks::Sampler>>,
    /// Programme dont l'arrêt attend un second appui.
    pub kill: Option<i32>,
    pub busy: bool,
}

/// Horloge de l'animation « flux vocal » (départ au lancement).
#[derive(Debug)]
pub struct FluxClock(Instant);

impl Default for FluxClock {
    fn default() -> Self {
        FluxClock(Instant::now())
    }
}

impl FluxClock {
    pub fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }
}

/// État de LYRA vocale (bouton micro du Panorama).
#[derive(Debug, Clone, Default, PartialEq)]
pub enum VoiceUi {
    #[default]
    Idle,
    /// Installation de Piper et Vosk (étape en cours).
    Installing(String),
    /// À l'écoute du mot « Lyra ».
    Listening,
    /// « Lyra » entendu : la demande suit.
    Awake,
    /// Demande envoyée à LYRA.
    Thinking,
    /// LYRA répond à voix haute.
    Speaking,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum PMsg {
    Shortcut(usize),
    /// Côte à côte : toggle, on, off, swap, pick, done, release.
    Side(&'static str),
    /// Zone d'accueil : module ajouté ou retiré.
    SideModule(String),
    /// Zone d'accueil : fenêtre (adresse) à ramener.
    SideWindow(String),
    /// Fenêtre ramenée, ou pourquoi elle ne l'a pas été.
    SideTaken(Result<crate::panorama::guest::Guest, String>),
    /// Fin d'une pose ou d'une restitution de la fenêtre invitée.
    SideDone(Result<(), String>),
    /// Préréglage de la zone d'accueil (rang 0 à 4) : appuyé (true) ou relâché.
    SidePreset(usize, bool),
    /// Fin du délai d'appui long : (rang, appui).
    SideHold(usize, u64),
    /// Module de la zone d'accueil en édition (appui long), ou fin d'édition.
    SideEdit(Option<usize>),
    /// Module déplacé d'un cran (avant : -1, après : +1).
    SideMove(usize, i32),
    /// Module retiré de la zone d'accueil.
    SideRemove(usize),
    /// Glisser dans la zone d'accueil : préréglage suivant (+1) ou précédent.
    SideSwipe(i32),
    /// Pièces Hue relues : pont appairé, pièces ou erreur.
    HueRooms(
        bool,
        Result<Vec<crate::panorama::modules::hue::Room>, String>,
    ),
    /// Pièce Hue : allumer / éteindre, luminosité.
    HueSet(String, Option<bool>, Option<u8>),
    /// Lampe Hue : allumer / éteindre, luminosité.
    HueLight(String, Option<bool>, Option<u8>),
    /// Curseur de luminosité d'une pièce : en cours, puis relâché.
    HueDrag(String, u8),
    HueLightDrag(String, u8),
    HueRelease,
    /// Lampe désignée (survol, appui) : son nom et son curseur.
    HueFocus(Option<String>),
    /// Bande des modules : catalogue ouvert ou fermé ; module ajouté/retiré.
    ModulesEdit(bool),
    ModuleToggle(String),
    /// Tuile Notes : ouvre l'éditeur.
    NotesOpen,
    NotesOpened(Result<(), String>),
    /// Bloc des raccourcis : page des touches (0) ou des fenêtres (1).
    ShortcutsPage(usize),
    /// Fenêtre (adresse) envoyée sur le workspace actif de cet écran.
    SendWindow(String, String),
    /// Profil du bloc de raccourcis (vidéo, audio, télécommande, système).
    Profile(String),
    /// Cadrage de la source ouverte : start (bascule), zoom+, zoom-, left,
    /// right, up, down, reset, done.
    Frame(&'static str),
    /// Fenêtres étiquetées du lecteur trouvées au démarrage : (adresse, classe).
    Orphans(Vec<(String, String)>),
    /// LYRA vocale : événement de l'écoute.
    Voice(crate::voice::Event),
    /// Bouton micro : active (installe au besoin) ou coupe LYRA vocale.
    VoiceToggle,
    VoiceSetup(Result<(), String>),
    /// Réponse dite : l'écoute reprend.
    VoiceDone,
    /// Image suivante des courbes (défilement continu).
    GraphFrame,
    /// Niveaux du son (barre « flux vocal »).
    Levels(crate::panorama::radio_play::Levels),
    /// Radio : pause (true) ou lecture.
    RadioPaused(Option<bool>),
    /// Pastille « en cours » : lecture / pause de ce qui joue.
    NowPlayingToggle,
    /// Tuile de mesure touchée : gestionnaire des tâches (écran, mesure).
    TasksOpen(String, String),
    TasksSort(&'static str),
    TasksClose,
    TasksData(Vec<crate::panorama::tasks::Proc>),
    /// Terminer : premier appui arme, second envoie SIGTERM.
    TaskKill(i32),
    /// Relit la liste des micros ; liste relue.
    MicsRefresh,
    Mics(Vec<(String, String, bool)>),
    /// Fait parler LYRA (essai de la voix, Réglages).
    VoiceTest,
    /// Question dite : l'écoute reprend, la phrase suivante est la réponse.
    VoiceArm,
    /// Cookies du navigateur relus (connexions aux plateformes).
    Signed(Option<std::collections::HashSet<(String, String)>>),
    /// Fin du dessin automatique des icônes manquantes (SmartUI).
    IconsAuto(Result<String, String>),
    /// Ouvre une adresse dans le lecteur : (nom, adresse).
    PlayUrl(String, String),
    /// Onglet de la colonne Musique : playlist, radio.
    MusicTab(&'static str),
    /// Recherche de radios (Radio Browser) ; `play` : lancer la première.
    RadioSearch(String, bool),
    RadioFound(
        String,
        bool,
        Result<Vec<crate::panorama::config::Source>, String>,
    ),
    /// Ajoute (ou retire) une radio des favoris : (nom, flux).
    RadioFav(String, String),
    /// Suivi de la connexion d'une radio : (flux, essai).
    RadioPoll(String, u32),
    RadioStatus(String, u32, crate::panorama::radio_play::Status),
    /// Lecture impossible : (flux, raison).
    RadioFailed(String, String),
    /// Cherche les logos des radios favorites qui n'en ont pas ; `true` :
    /// même déjà cherchés cette session.
    RadioLogos(bool),
    RadioLogosFound(Vec<(String, String)>),
    /// Fin de la mise en vidéo seule automatique : (adresse de la source,
    /// méthode qui a marché, à mémoriser).
    AutoFull(String, Result<Option<String>, String>),
    ShortcutResult(Result<(), String>),
    Wallpaper(String, Option<iced::Size>),
    /// Écrans et tactiles relus.
    Detected(Vec<Screen>, Vec<String>),
    Swipe(String, i32),
    /// Geste depuis le bord bas (+1, choix de la disposition) ou le bord
    /// haut (-1 : ferme ce qui est ouvert, sinon masque le Panorama).
    Cross(String, i32),
    /// Ferme le choix de la disposition.
    PickerClose,
    /// Notifications (geste depuis le bord droit) : ouvre sur cet écran,
    /// `None` ferme.
    Notifs(Option<String>),
    /// Efface l'historique des notifications.
    NotifsClear,
    Goto(String, usize),
    Hide(String),
    Show(String),
    /// Basculer (ligne de commande : sans écran, le premier Panorama).
    Toggle(Option<String>),
    Accept(Id),
    Decline(Id),
    Later(Id),
    Tick,
    Media(Option<Status>),
    /// Mesures relevées : (identifiant, ligne).
    Sys(Vec<(String, crate::widgets::SysLine)>),
    /// Commande MPRIS (`PlayPause`, `Next`, `Previous`).
    Control(&'static str),
    /// État des mises à jour relu (profil Système).
    Updates(Option<crate::updates::Snapshot>),
    /// `hyprdesk updates --refresh | --upgrade`, lancé par Hyprland (survit
    /// à un redémarrage de hyprdesk).
    UpdatesRun(&'static str),
    Pressed,
    Touch(Result<(), String>),
    /// Ouvre l'aperçu au format donné (taille simulée), ou le ferme s'il
    /// est ouvert au même format.
    Preview((u32, u32)),
    ClosePreview,
    /// Disposition : une page, 2 ou 3 zones.
    Layout(crate::panorama::config::Layout),
    /// Widget suivant (+1) ou précédent (-1) de la zone.
    Pane(usize, i32),
    /// Ouvre la source vidéo n° (liste `sources`).
    VideoOpen(usize),
    /// Recherche de la fenêtre lancée (essai n°).
    VideoPoll(u32),
    VideoFound(Option<String>),
    VideoPlaced(Result<(), String>),
    VideoClose,
    /// Carte « Téléphone » de la zone vidéo montrée ou non.
    CastCard(bool),
    CaptureScan(Option<(serde_json::Value, Vec<crate::widgets::media::Status>)>),
    CapturePlaced(String, Result<(), String>),
    PreviewAt(Option<(i32, i32)>),
}

fn wrap(m: PMsg) -> Msg {
    Msg::Panorama(m)
}

/// Écrans et tactiles décrits par Hyprland (bloquant).
fn detect_now() -> (Vec<Screen>, Vec<String>) {
    use hyprpanel_core::ipc::query_json;
    let screens = query_json("monitors")
        .map(|v| detect::parse_screens(&v))
        .unwrap_or_default();
    let touch = query_json("devices")
        .map(|v| detect::parse_touch(&v))
        .unwrap_or_default();
    (screens, touch)
}

pub fn detect_task() -> Task<Msg> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(detect_now)
                .await
                .unwrap_or_default()
        },
        |(s, t)| wrap(PMsg::Detected(s, t)),
    )
}

pub(in crate::ui) fn fmt_time(secs: i64) -> String {
    let s = secs.max(0);
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

impl App {
    fn panorama_font(&self) -> iced::Font {
        let family = &self.cfg.general.panorama.design.font_family;
        if family.is_empty() {
            self.font
        } else {
            iced::Font::with_name(crate::ui::intern_font(family))
        }
    }
    fn panorama_palette(&self) -> (Color, Color, Color) {
        let th = &self.cfg.general.theme;
        let d = &self.cfg.general.panorama.design;
        let c = |s: &str, fallback| color(crate::config::Rgba::parse(s).unwrap_or(fallback), 1.0);
        (
            c(&d.background, th.bg),
            c(&d.foreground, th.fg),
            c(&d.accent, th.accent),
        )
    }
    fn panorama_motion(&self) -> Duration {
        if self.cfg.general.transition == crate::config::Transition::None {
            Duration::ZERO
        } else {
            Duration::from_millis(160)
        }
    }

    fn panorama_feedback<'a>(&self, content: impl Into<Element<'a, Msg>>) -> Element<'a, Msg> {
        crate::ui::kinetic::magnet(
            content,
            crate::ui::plugins::bar::style::convexe::Edge::Bottom,
            self.panorama_palette().2,
            self.panorama_motion(),
        )
    }

    /// Configuration du Panorama en vigueur (réglages en cours, sinon chargée).
    pub(in crate::ui) fn panorama_cfg(&self) -> crate::panorama::Config {
        self.settings
            .panorama
            .clone()
            .unwrap_or_else(|| self.cfg.general.panorama.clone())
    }

    /// Écrans branchés au rôle Panorama (connecteurs).
    pub(in crate::ui) fn panorama_monitors(&self) -> Vec<String> {
        if !self.cfg.plugin_on("panorama") {
            return Vec::new();
        }
        let c = &self.cfg.general.panorama;
        self.monitors
            .iter()
            .filter(|m| {
                c.zone.as_ref().is_some_and(|z| z.screen == m.name)
                    || c.screens.iter().any(|e| e == &m.name)
                    || self
                        .panorama
                        .screens
                        .iter()
                        .find(|s| s.name == m.name)
                        .is_some_and(|s| c.has(s))
            })
            .map(|m| m.name.clone())
            .collect()
    }

    pub(in crate::ui) fn is_panorama(&self, monitor: &str) -> bool {
        // Écran entier seulement : un écran qui porte une zone Panorama garde
        // sa barre et son bureau.
        self.cfg
            .general
            .panorama
            .zone
            .as_ref()
            .is_none_or(|z| z.screen != monitor)
            && self.panorama_monitors().iter().any(|m| m == monitor)
    }

    /// Crée ou ferme les surfaces et poignées selon la config et les écrans.
    pub(in crate::ui) fn sync_panorama(&mut self) -> Task<Msg> {
        let want = self.panorama_monitors();
        let mut tasks: Vec<Task<Msg>> = Vec::new();
        // Paquet coupé : plus de lecture (vidéo, radio) ni d'aperçu ; les
        // surfaces se ferment plus bas (`want` est vide).
        if !self.cfg.plugin_on("panorama") {
            if self.panorama.video.is_some() || self.panorama.radio.is_some() {
                tasks.push(self.panorama_update(PMsg::VideoClose));
            }
            if let Some(p) = self.panorama.preview.take() {
                tasks.push(iced::window::close(p.win));
            }
        }
        let wallpaper = self.cfg.general.panorama.design.wallpaper.clone();
        if wallpaper != self.panorama.wallpaper_requested {
            self.panorama.wallpaper_requested = wallpaper.clone();
            self.panorama.wallpaper = None;
            if !wallpaper.is_empty() {
                let path = crate::config::lua::expand_path(
                    &wallpaper,
                    self.config_path
                        .parent()
                        .unwrap_or(std::path::Path::new(".")),
                );
                tasks.push(Task::perform(
                    async move {
                        let size = tokio::task::spawn_blocking(move || {
                            ::image::image_dimensions(path).ok()
                        })
                        .await
                        .ok()
                        .flatten()
                        .map(|(w, h)| iced::Size::new(w as f32, h as f32));
                        (wallpaper, size)
                    },
                    |(p, s)| wrap(PMsg::Wallpaper(p, s)),
                ));
            }
        }
        // Le tactile suit l'écran Panorama même quand son connecteur change
        // de nom : rattaché à un écran absent, Hyprland met le toucher sur
        // l'écran qui a le focus, et le Panorama ne répond plus qu'avec lui.
        if !self.panorama.touch.is_empty()
            && let Some(target) =
                crate::panorama::touch_target(&want, &self.panorama.screens, &self.panorama.touch)
        {
            let connected: Vec<String> = self.monitors.iter().map(|m| m.name.clone()).collect();
            let key = (target.clone(), connected.clone());
            if self.panorama.touch_checked.as_ref() != Some(&key) {
                self.panorama.touch_checked = Some(key);
                let path = self.config_path.clone();
                tasks.push(Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::panorama::keep_touch(&path, &target, &connected).map(|moved| {
                                if moved {
                                    tracing::info!(
                                        "{}",
                                        crate::tr!(
                                            "Panorama : tactile rattaché à {p0}",
                                            p0 = target
                                        )
                                    );
                                }
                            })
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| wrap(PMsg::Touch(r)),
                ));
            }
        }
        let shown: Vec<String> = want
            .iter()
            .filter(|m| !self.panorama.hidden.contains(*m))
            .cloned()
            .collect();
        let hidden: Vec<String> = want
            .iter()
            .filter(|m| self.panorama.hidden.contains(*m))
            .cloned()
            .collect();
        for (id, m) in self.panorama.surfaces.clone() {
            if !shown.contains(&m) {
                self.panorama.surfaces.remove(&id);
                tasks.push(iced::window::close(id));
            }
        }
        for (id, m) in self.panorama.handles.clone() {
            if !hidden.contains(&m) {
                self.panorama.handles.remove(&id);
                tasks.push(iced::window::close(id));
            }
        }
        // Rôle donné autrement (Réglages, ligne de commande) ou écran parti :
        // la proposition n'a plus lieu d'être.
        for (id, o) in self.panorama.offers.clone() {
            if want.contains(&o.screen) || !self.monitors.iter().any(|m| m.name == o.screen) {
                self.panorama.offers.remove(&id);
                tasks.push(iced::window::close(id));
            }
        }
        // Zone Panorama changée (côté, taille, écran) : sa surface est refaite.
        let zone = self.cfg.general.panorama.zone.clone();
        if self.panorama.zone_applied != zone {
            for (id, m) in self.panorama.surfaces.clone() {
                let was = self
                    .panorama
                    .zone_applied
                    .as_ref()
                    .is_some_and(|z| z.screen == m);
                let now = zone.as_ref().is_some_and(|z| z.screen == m);
                if was || now {
                    self.panorama.surfaces.remove(&id);
                    tasks.push(iced::window::close(id));
                }
            }
            self.panorama.zone_applied = zone.clone();
        }
        for m in shown {
            if self.panorama.surfaces.values().any(|s| s == &m) {
                continue;
            }
            let id = Id::unique();
            self.panorama.surfaces.insert(id, m.clone());
            self.panorama.last_press.get_or_insert_with(Instant::now);
            tracing::info!("{}", crate::tr!("Panorama sur {p0}", p0 = m.clone()));
            // Écran entier, ou bande collée à un bord : sa zone exclusive
            // tient les fenêtres (même maximisées) hors du Panorama.
            let (anchor, size, exclusive) = match zone.as_ref().filter(|z| z.screen == m) {
                Some(z) => {
                    let mon = self.monitors.iter().find(|x| x.name == m);
                    let (w, h) =
                        mon.map_or((1920.0, 1080.0), |x| (x.width as f32, x.height as f32));
                    let t = z.thickness(w, h) as u32;
                    match z.side.as_str() {
                        "top" => (LsAnchor::Top | LsAnchor::Left | LsAnchor::Right, (0, t), t),
                        "left" => (LsAnchor::Left | LsAnchor::Top | LsAnchor::Bottom, (t, 0), t),
                        "right" => (
                            LsAnchor::Right | LsAnchor::Top | LsAnchor::Bottom,
                            (t, 0),
                            t,
                        ),
                        _ => (
                            LsAnchor::Bottom | LsAnchor::Left | LsAnchor::Right,
                            (0, t),
                            t,
                        ),
                    }
                }
                None => (
                    LsAnchor::Top | LsAnchor::Bottom | LsAnchor::Left | LsAnchor::Right,
                    (0, 0),
                    0,
                ),
            };
            tasks.push(Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some(size),
                    layer: Layer::Top,
                    anchor,
                    exclusive_zone: Some(if exclusive > 0 { exclusive as i32 } else { -1 }),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    output_option: OutputOption::OutputName(m),
                    namespace: Some(crate::panorama::NAMESPACE.into()),
                    ..Default::default()
                },
                id,
            }));
        }
        for m in hidden {
            if self.panorama.handles.values().any(|s| s == &m) {
                continue;
            }
            let id = Id::unique();
            self.panorama.handles.insert(id, m.clone());
            tasks.push(Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some((HANDLE_W, HANDLE_H)),
                    layer: Layer::Top,
                    anchor: LsAnchor::Bottom,
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    output_option: OutputOption::OutputName(m),
                    namespace: Some(crate::panorama::NAMESPACE.into()),
                    ..Default::default()
                },
                id,
            }));
        }
        tasks.push(self.panorama_want_icons());
        Task::batch(tasks)
    }

    /// Résout les icônes du bandeau s'il est affiché (surface ou aperçu).
    /// Rappelé après chaque relecture des icônes (`Msg::IconsResolved`), qui
    /// vide `named_icons` : sans cela, les tuiles retombaient sur l'initiale
    /// jusqu'au prochain `sync_panorama`.
    pub(in crate::ui) fn panorama_want_icons(&mut self) -> Task<Msg> {
        if self.panorama.surfaces.is_empty() && self.panorama.preview.is_none() {
            return Task::none();
        }
        let tasks: Vec<Task<Msg>> = self
            .panorama_icon_names()
            .iter()
            .map(|n| self.want_icon(n))
            .collect();
        Task::batch(tasks)
    }

    /// Demande entendue : envoyée à LYRA (son tiroir s'ouvre), la réponse
    /// sera dite dans la langue de la demande.
    fn voice_ask(&mut self, lang: String, text: String) -> Task<Msg> {
        // LYRA attend un accord : « oui » ou « non ».
        if let Some((id, asked)) = self.panorama.voice_confirm.clone() {
            let t = text.to_lowercase();
            let has = |words: &[&str]| words.iter().any(|w| t.split_whitespace().any(|x| x == *w));
            let yes = has(&[
                "oui", "ouais", "ok", "okay", "d'accord", "vas-y", "accepte", "confirme", "yes",
                "yeah", "sure", "go",
            ]);
            let no = has(&["non", "annule", "refuse", "no", "nope", "cancel"]);
            if yes != no {
                self.panorama.voice_confirm = None;
                self.panorama.voice = VoiceUi::Thinking;
                return self.chat_update(crate::ui::chat::ChatMsg::Confirm(id, yes));
            }
            let again = if asked == "en" {
                "Sorry, yes or no?"
            } else {
                "Je n'ai pas compris : oui ou non ?"
            };
            return self.voice_say_then_arm(again.into(), asked);
        }
        self.panorama.voice = VoiceUi::Thinking;
        let prompt = if lang == "en" {
            format!(
                "🎙 {text}\n\n(Voice request, possibly misheard: act if needed with hyprdesk commands, then answer in one or two short spoken sentences, in English, without formatting.)"
            )
        } else {
            format!(
                "🎙 {text}\n\n(Demande vocale, peut-être mal entendue : agis au besoin avec les commandes hyprdesk, puis réponds en une ou deux phrases courtes à dire à voix haute, en français, sans mise en forme.)"
            )
        };
        match self.ask_assistant(prompt, "LYRA vocale", true) {
            Some(t) => {
                let session = self.chat_session;
                if let Some(c) = self.chat.as_mut().and_then(|c| c.by_session(session)) {
                    c.speak = Some(lang);
                }
                t
            }
            None => self.voice_say(
                crate::i18n::tr(
                    "LYRA n'est pas disponible : activez l'assistant dans les Réglages.",
                )
                .into(),
                lang,
            ),
        }
    }

    /// LYRA demande un accord pendant une demande vocale : la question est
    /// dite, puis la réponse (« oui », « non ») est écoutée sans « Lyra ».
    pub(in crate::ui) fn voice_confirm(
        &mut self,
        id: String,
        command: String,
        reason: String,
        lang: String,
    ) -> Task<Msg> {
        self.panorama.voice_confirm = Some((id, lang.clone()));
        let what = if reason.trim().is_empty() {
            command
        } else {
            reason
        };
        let text = if lang == "en" {
            format!("I need your approval: {what}. Yes or no?")
        } else {
            format!("J'ai besoin de votre accord : {what}. Oui ou non ?")
        };
        self.voice_say_then_arm(text, lang)
    }

    /// Dit `text`, puis écoute la phrase suivante sans attendre « Lyra ».
    fn voice_say_then_arm(&mut self, text: String, lang: String) -> Task<Msg> {
        crate::voice::control("pause");
        self.panorama.voice = VoiceUi::Speaking;
        Task::perform(
            async move {
                let _ =
                    tokio::task::spawn_blocking(move || crate::voice::speak(&text, &lang)).await;
                tokio::time::sleep(Duration::from_millis(300)).await;
            },
            |_| wrap(PMsg::VoiceArm),
        )
    }

    /// Dit `text` (Piper) ; l'écoute est suspendue le temps de parler.
    pub(in crate::ui) fn voice_say(&mut self, text: String, lang: String) -> Task<Msg> {
        crate::voice::control("pause");
        self.panorama.voice = VoiceUi::Speaking;
        Task::perform(
            async move {
                let _ =
                    tokio::task::spawn_blocking(move || crate::voice::speak(&text, &lang)).await;
                // L'écho de la pièce se tait avant de réécouter.
                tokio::time::sleep(Duration::from_millis(300)).await;
            },
            |_| wrap(PMsg::VoiceDone),
        )
    }

    /// Relevé des programmes (gestionnaire des tâches ouvert), en arrière-
    /// plan ; la carte NVIDIA n'est interrogée que pour trier sur GPU/VRAM.
    /// Touche lourde armée il y a moins de 4 s.
    pub(in crate::ui) fn panorama_armed(&self, i: usize) -> bool {
        self.panorama
            .confirm
            .is_some_and(|(j, at)| j == i && at.elapsed() < Duration::from_secs(4))
    }

    /// Relit l'état des mises à jour (profil Système affiché).
    fn panorama_updates_read(&self) -> Task<Msg> {
        if self.cfg.general.panorama.design.shortcut_profile != "power" {
            return Task::none();
        }
        Task::perform(
            async {
                tokio::task::spawn_blocking(crate::updates::snapshot)
                    .await
                    .ok()
                    .flatten()
            },
            |s| wrap(PMsg::Updates(s)),
        )
    }

    /// Recherche (`--refresh`), mise à jour (`--upgrade`) ou détail du
    /// dernier échec (`--report`), sauf si une tâche tourne déjà ; l'état
    /// s'affiche au relevé suivant.
    fn panorama_updates_run(&mut self, arg: &'static str) -> Task<Msg> {
        self.panorama.updates_asked = true;
        if self
            .panorama
            .updates
            .as_ref()
            .is_some_and(|u| matches!(u.state.as_str(), "checking" | "running"))
        {
            return Task::none();
        }
        if let Some(u) = self.panorama.updates.as_mut()
            && arg != "--report"
        {
            u.state = if arg == "--upgrade" {
                "running"
            } else {
                "checking"
            }
            .into();
        }
        let command = format!("hyprdesk updates {arg}");
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    crate::hypr::exec(&command).map_err(|e| e.to_string())
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
            },
            |r| wrap(PMsg::ShortcutResult(r)),
        )
    }

    fn panorama_tasks_refresh(&mut self) -> Task<Msg> {
        let Some(t) = self.panorama.tasks.as_mut().filter(|t| !t.busy) else {
            return Task::none();
        };
        t.busy = true;
        let sampler = t.sampler.clone();
        let gpu = matches!(t.sort, "gpu" | "vram");
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let mut s = sampler.lock().unwrap_or_else(|e| e.into_inner());
                    crate::panorama::tasks::sample(&mut s, gpu)
                })
                .await
                .unwrap_or_default()
            },
            |l| wrap(PMsg::TasksData(l)),
        )
    }

    /// Une image du flux vocal : le fond suit des niveaux très lissés
    /// (environ 1 s), l'onde avance d'un cran toutes les deux images avec
    /// une hauteur qui glisse vers sa cible.
    fn panorama_flux_frame(&mut self) {
        let p = &mut self.panorama;
        for (s, v) in p.bg_levels.iter_mut().zip(p.levels) {
            *s += (v - *s) * 0.035;
        }
        p.wave += (p.levels[0] - p.wave) * 0.18;
        p.flux_frames = p.flux_frames.wrapping_add(1);
        if p.flux_frames.is_multiple_of(2) {
            p.level_hist.push_back(p.wave);
            while p.level_hist.len() > 160 {
                p.level_hist.pop_front();
            }
            p.levels_at = Some(Instant::now());
        }
    }

    /// Avancée vers le relevé suivant (0 à 1), pour faire défiler les
    /// courbes en continu.
    pub(in crate::ui) fn panorama_graph_phase(&self) -> f32 {
        self.panorama.sys_at.map_or(1.0, |t| {
            (t.elapsed().as_secs_f32() / TICK.as_secs_f32()).min(1.0)
        })
    }

    /// Des courbes sont à l'écran (mesures en graphique) : le Panorama est
    /// redessiné en continu pour les faire défiler.
    pub(in crate::ui) fn panorama_graphs_shown(&self) -> bool {
        if self.panorama_asleep() {
            return false;
        }
        let graphs = |w: &str| {
            crate::panorama::design::metric_of(w).is_some_and(|(_, s)| s == "graphs")
                || (w == "system" && self.cfg.general.panorama.design.system_style == "graphs")
        };
        self.panorama_surfs().iter().any(|s| {
            !self.panorama.hidden.contains(&s.key)
                && self.panorama_geo(s).panes.iter().any(|p| graphs(&p.widget))
        })
    }

    /// Sources du lecteur visibles (profil Musique : services de musique ;
    /// sinon vidéo) : sans compte, connectées, ou forcées dans les Réglages.
    pub(in crate::ui) fn panorama_sources(&self) -> Vec<crate::panorama::config::Source> {
        self.cfg
            .general
            .panorama
            .active_sources()
            .iter()
            .filter(|s| {
                crate::panorama::auth::visible(
                    s,
                    self.panorama.signed.get(&s.url).copied().flatten(),
                )
            })
            .cloned()
            .collect()
    }

    /// Relit les connexions (cookies du navigateur) toutes les 60 s.
    fn panorama_check_signed(&mut self) -> Task<Msg> {
        if self
            .panorama
            .signed_at
            .is_some_and(|t| t.elapsed() < Duration::from_secs(60))
        {
            return Task::none();
        }
        self.panorama.signed_at = Some(Instant::now());
        let mics = Task::done(wrap(PMsg::MicsRefresh));
        let browser = crate::panorama::video::browser(&self.cfg.general.panorama.browser)
            .map(|(cmd, _)| cmd)
            .unwrap_or_default();
        Task::batch([
            mics,
            Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || crate::panorama::auth::cookies(&browser))
                        .await
                        .ok()
                        .flatten()
                },
                |k| wrap(PMsg::Signed(k)),
            ),
        ])
    }

    /// Lecture commencée dans le lecteur (profils Vidéo et Musique) : la
    /// vidéo passe seule en plein écran, dans sa zone (`autofull`), une fois
    /// par fenêtre ; la méthode trouvée est mémorisée pour la source.
    fn panorama_auto_full(&mut self) -> Task<Msg> {
        if !matches!(
            self.cfg.general.panorama.design.shortcut_profile.as_str(),
            "video" | "audio"
        ) {
            return Task::none();
        }
        let Some(v) = self.panorama.video.as_ref() else {
            return Task::none();
        };
        let Some(addr) = v.address.clone() else {
            return Task::none();
        };
        if v.borrowed.is_some() || self.panorama.autofull_done.contains(&addr) {
            return Task::none();
        }
        // Le lecteur (MPRIS) qui joue est celui du navigateur.
        let playing = self.panorama.media.as_ref().is_some_and(|m| {
            m.playing && (v.prefix.is_empty() || m.player.to_lowercase().contains(&v.prefix))
        });
        if !playing {
            return Task::none();
        }
        self.panorama.autofull_done.push(addr.clone());
        let url = v.url.clone();
        let known = self
            .panorama_current_source()
            .and_then(|s| crate::panorama::autofull::Method::parse(&s.fs));
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    crate::panorama::autofull::run(&addr, known).map(|m| m.map(|m| m.id()))
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
            },
            move |r| wrap(PMsg::AutoFull(url.clone(), r)),
        )
    }

    /// Sources vidéo ou musique sans icône : SmartUI les dessine d'elle-même,
    /// en arrière-plan (`hyprdesk panorama icons sources`), une fois par
    /// session et par source ; chaque icône s'affiche dès qu'elle est prête.
    fn panorama_auto_icons(&mut self) -> Task<Msg> {
        if self.panorama.icons_job.is_some() {
            return Task::none();
        }
        let c = &self.cfg.general.panorama;
        let missing: Vec<String> = c
            .sources
            .iter()
            .chain(&c.music_sources)
            .filter(|s| s.icon.is_empty() && !self.panorama.icons_tried.contains(&s.url))
            .map(|s| s.url.clone())
            .collect();
        if missing.is_empty() {
            return Task::none();
        }
        self.panorama.icons_tried.extend(missing);
        self.panorama.icons_job = Some(String::new());
        let config = self.config_path.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
                    let out = std::process::Command::new(exe)
                        .arg("--config")
                        .arg(&config)
                        .args(["panorama", "icons", "sources"])
                        .output()
                        .map_err(|e| e.to_string())?;
                    let text = |b: &[u8]| String::from_utf8_lossy(b).trim().to_string();
                    let last = text(&out.stdout).lines().last().unwrap_or("").to_string();
                    if out.status.success() {
                        Ok(last)
                    } else {
                        Err(format!("{last}\n{}", text(&out.stderr)).trim().to_string())
                    }
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
            },
            |r| wrap(PMsg::IconsAuto(r)),
        )
    }

    /// Icônes à résoudre pour le bandeau : commandes, touches de chaque
    /// profil, raccourcis de la personne, sources vidéo et musique.
    fn panorama_icon_names(&self) -> Vec<String> {
        let c = &self.cfg.general.panorama;
        let mut out: Vec<String> = ICONS.iter().map(|n| n.to_string()).collect();
        // Cadrage du lecteur, Lecture/Pause.
        out.extend(
            [
                "zoom-in-symbolic",
                "zoom-out-symbolic",
                "view-refresh-symbolic",
                "object-select-symbolic",
                "media-playback-pause-symbolic",
            ]
            .map(String::from),
        );
        for id in crate::panorama::design::PROFILES {
            out.extend(
                crate::panorama::design::profile_shortcuts(id)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|s| s.icon),
            );
        }
        out.extend(c.design.shortcuts.iter().map(|s| s.icon.clone()));
        out.extend(
            c.sources
                .iter()
                .chain(&c.music_sources)
                .chain(&c.radios)
                .chain(self.panorama.radio_results.iter().flat_map(|(_, l)| l))
                .map(|s| s.icon.clone())
                .filter(|i| !i.is_empty()),
        );
        out.sort();
        out.dedup();
        out
    }

    /// Propositions à afficher (écrans candidats nouveaux).
    fn panorama_offers(&mut self) -> Task<Msg> {
        if !self.cfg.plugin_on("panorama") {
            return Task::none();
        }
        let c = self.panorama_cfg();
        let list = detect::offers(
            &self.panorama.screens,
            &self.panorama.touch,
            &c.screens,
            &c.declined,
        );
        let mut tasks = Vec::new();
        for o in list {
            let key = if o.edid.is_empty() {
                o.screen.clone()
            } else {
                o.edid.clone()
            };
            if !self.panorama.offered.insert(key) {
                continue;
            }
            let Some(m) = self.monitors.iter().find(|m| m.name == o.screen) else {
                continue;
            };
            let w = (m.width.saturating_sub(32)).min(900);
            let h = (m.height.saturating_sub(32)).min(260);
            // Un tactile sûr : il est rattaché tout de suite, pour qu'on
            // puisse répondre au doigt.
            if let Some(_t) = &o.touch {
                let (path, out) = (self.config_path.clone(), o.screen.clone());
                tasks.push(Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::panorama::map_touch(&path, Some(&out))
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| wrap(PMsg::Touch(r)),
                ));
            }
            let id = Id::unique();
            tasks.push(Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some((w, h)),
                    layer: Layer::Overlay,
                    anchor: LsAnchor::empty(),
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    output_option: OutputOption::OutputName(o.screen.clone()),
                    namespace: Some(crate::panorama::NAMESPACE.into()),
                    ..Default::default()
                },
                id,
            }));
            self.panorama.offers.insert(id, o);
        }
        Task::batch(tasks)
    }

    /// Enregistre une nouvelle configuration du Panorama.
    fn panorama_save(&mut self, c: crate::panorama::Config) -> Task<Msg> {
        self.settings.panorama = Some(c);
        self.commit()
    }

    /// Traite un message, puis remet zone de saisie et fenêtre vidéo à leur
    /// place (sans rien envoyer si rien n'a bougé).
    pub(in crate::ui) fn panorama_update(&mut self, m: PMsg) -> Task<Msg> {
        let t = self.panorama_update_inner(m);
        Task::batch([t, self.sync_pip(), self.sync_halo()])
    }

    /// Halo de LYRA vocale : affiché pendant la conversation (« Lyra »
    /// entendu, réflexion, réponse) sur les écrans choisis (Réglages).
    pub(in crate::ui) fn sync_halo(&mut self) -> Task<Msg> {
        let c = &self.cfg.general.panorama;
        let talking = c.voice
            && matches!(
                self.panorama.voice,
                VoiceUi::Awake | VoiceUi::Thinking | VoiceUi::Speaking
            );
        let want: Vec<String> = if !talking {
            Vec::new()
        } else {
            match c.voice_halo.as_str() {
                "off" => Vec::new(),
                "panorama" => self.panorama_monitors(),
                _ => self.monitors.iter().map(|m| m.name.clone()).collect(),
            }
        };
        let mut tasks = Vec::new();
        let gone: Vec<Id> = self
            .panorama
            .halo
            .iter()
            .filter(|(_, m)| !want.contains(m))
            .map(|(id, _)| *id)
            .collect();
        for id in gone {
            self.panorama.halo.remove(&id);
            tasks.push(iced::window::close(id));
        }
        for m in want {
            if self.panorama.halo.values().any(|x| *x == m) {
                continue;
            }
            let id = Id::unique();
            self.panorama.halo.insert(id, m.clone());
            self.panorama.halo_since.get_or_insert_with(Instant::now);
            tasks.push(Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: None,
                    layer: Layer::Overlay,
                    anchor: LsAnchor::Top | LsAnchor::Bottom | LsAnchor::Left | LsAnchor::Right,
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::None,
                    output_option: OutputOption::OutputName(m),
                    namespace: Some("hyprdesk-lyra-halo".into()),
                    ..Default::default()
                },
                id,
            }));
            // Aucune zone de saisie : clics et toucher passent au travers.
            tasks.push(Task::done(Msg::SetInputRegion {
                id,
                callback: iced_layershell::actions::ActionCallback::new(|_region| {}),
            }));
        }
        if self.panorama.halo.is_empty() {
            self.panorama.halo_since = None;
        }
        Task::batch(tasks)
    }

    fn panorama_update_inner(&mut self, m: PMsg) -> Task<Msg> {
        match m {
            PMsg::Wallpaper(path, size) => {
                if path == self.panorama.wallpaper_requested {
                    if let Some(size) = size {
                        let full = crate::config::lua::expand_path(
                            &path,
                            self.config_path
                                .parent()
                                .unwrap_or(std::path::Path::new(".")),
                        );
                        self.panorama.wallpaper =
                            Some((path, image::Handle::from_path(full), size));
                    } else {
                        self.panorama.design_error = Some(crate::tr!(
                            "Image introuvable ou illisible : {path}",
                            path = path
                        ));
                    }
                }
                Task::none()
            }
            PMsg::ShortcutsPage(i) => {
                self.panorama.shortcuts_page = i;
                self.panorama.last_press = Some(Instant::now());
                Task::none()
            }
            PMsg::SendWindow(address, monitor) => {
                self.panorama.last_press = Some(Instant::now());
                let Some(ws) = self
                    .monitors
                    .iter()
                    .find(|m| m.name == monitor)
                    .map(|m| {
                        if m.base.is_empty() {
                            m.workspace.clone()
                        } else {
                            m.base.clone()
                        }
                    })
                    .filter(|w| !w.is_empty() && !w.starts_with("special:"))
                else {
                    return Task::none();
                };
                Task::perform(
                    async move {
                        let _ = tokio::task::spawn_blocking(move || {
                            crate::hypr::move_window(&address, &ws)
                        })
                        .await;
                    },
                    |_| Msg::Noop,
                )
            }
            PMsg::Shortcut(i) => {
                self.panorama.last_press = Some(Instant::now());
                let list = self.cfg.general.panorama.design.active_shortcuts();
                let Some(s) = list.get(i) else {
                    return Task::none();
                };
                let command = s.command.clone();
                drop(list);
                // Éteindre, redémarrer : le premier appui arme, le second lance.
                if crate::panorama::design::needs_confirm(&command) && !self.panorama_armed(i) {
                    self.panorama.confirm = Some((i, Instant::now()));
                    return Task::none();
                }
                self.panorama.confirm = None;
                // Côte à côte : sans passer par la ligne de commande.
                if command.trim() == "hyprdesk panorama split" {
                    return self.panorama_side_update(PMsg::Side("toggle"));
                }
                // Mise à jour : son avancement s'affiche aussitôt.
                if command.trim() == "hyprdesk updates --upgrade" {
                    return self.panorama_updates_run("--upgrade");
                }
                // Webradio (mpv) : Lecture/Pause la pilote directement.
                if self.panorama.radio.is_some() && command.trim() == "hyprdesk media play-pause" {
                    return Task::perform(
                        async {
                            tokio::task::spawn_blocking(crate::panorama::radio_play::toggle_pause)
                                .await
                                .ok()
                                .flatten()
                        },
                        |p| wrap(PMsg::RadioPaused(p)),
                    );
                }
                // Profils Vidéo et Musique : les touches vont au lecteur ; son
                // plein écran (vidéo seule) est mémorisé pour la source.
                let player = matches!(
                    self.cfg.general.panorama.design.shortcut_profile.as_str(),
                    "video" | "audio"
                );
                if player
                    && let Some(key) = command.trim().strip_prefix("hyprdesk key ")
                    && let Some(v) = self.panorama.video.as_ref()
                    && let Some(addr) = v.address.clone()
                {
                    let key = key.trim().to_string();
                    return Task::batch([Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                crate::panorama::video::send_key(&addr, &key)
                            })
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                        },
                        |r| wrap(PMsg::ShortcutResult(r)),
                    )]);
                }
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::hypr::exec(&command).map_err(|e| e.to_string())
                        })
                        .await
                        .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| wrap(PMsg::ShortcutResult(r)),
                )
            }
            PMsg::VoiceToggle => {
                self.panorama.last_press = Some(Instant::now());
                if self.cfg.general.panorama.voice {
                    self.panorama.voice = VoiceUi::Idle;
                    let mut c = self.panorama_cfg();
                    c.voice = false;
                    return self.panorama_save(c);
                }
                if crate::voice::ready() {
                    self.panorama.voice = VoiceUi::Listening;
                    let mut c = self.panorama_cfg();
                    c.voice = true;
                    return self.panorama_save(c);
                }
                if matches!(self.panorama.voice, VoiceUi::Installing(_)) {
                    return Task::none();
                }
                // Première fois : Piper et Vosk s'installent (~350 Mo).
                self.panorama.voice =
                    VoiceUi::Installing(crate::i18n::tr("Installation de la voix…").into());
                Task::perform(
                    async {
                        tokio::task::spawn_blocking(|| crate::voice::setup(&mut |_| {}))
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                    },
                    |r| wrap(PMsg::VoiceSetup(r)),
                )
            }
            PMsg::VoiceSetup(r) => match r {
                Ok(()) => {
                    self.panorama.voice = VoiceUi::Listening;
                    let mut c = self.panorama_cfg();
                    c.voice = true;
                    self.panorama_save(c)
                }
                Err(e) => {
                    self.panorama.voice = VoiceUi::Error(e);
                    Task::none()
                }
            },
            PMsg::Voice(e) => match e {
                crate::voice::Event::Ready => {
                    if !matches!(self.panorama.voice, VoiceUi::Speaking | VoiceUi::Thinking) {
                        self.panorama.voice = VoiceUi::Listening;
                    }
                    Task::none()
                }
                crate::voice::Event::Wake => {
                    self.panorama.voice = VoiceUi::Awake;
                    self.panorama.last_press = Some(Instant::now());
                    Task::none()
                }
                crate::voice::Event::Error(m) => {
                    self.panorama.voice = VoiceUi::Error(m);
                    Task::none()
                }
                crate::voice::Event::Heard(lang, text) => self.voice_ask(lang, text),
                crate::voice::Event::Partial(t) => {
                    self.panorama.voice_heard = Some(t);
                    Task::none()
                }
            },
            PMsg::GraphFrame => {
                if self.panorama.radio.is_some() {
                    self.panorama_flux_frame();
                }
                Task::none()
            }
            PMsg::Levels(l) => {
                // Cible seulement : l'onde avance à cadence régulière (images),
                // les mesures arrivant par paquets.
                self.panorama.levels = l;
                while self.panorama.level_hist.len() > 160 {
                    self.panorama.level_hist.pop_front();
                }
                Task::none()
            }
            PMsg::NowPlayingToggle => {
                self.panorama.last_press = Some(Instant::now());
                if self.panorama.radio.is_some() {
                    return Task::perform(
                        async {
                            tokio::task::spawn_blocking(crate::panorama::radio_play::toggle_pause)
                                .await
                                .ok()
                                .flatten()
                        },
                        |p| wrap(PMsg::RadioPaused(p)),
                    );
                }
                self.panorama_update(PMsg::Control("PlayPause"))
            }
            PMsg::RadioPaused(p) => {
                if let Some(p) = p {
                    self.panorama.radio_paused = p;
                }
                Task::none()
            }
            PMsg::TasksOpen(key, metric) => {
                self.panorama.last_press = Some(Instant::now());
                self.panorama.tasks = Some(TasksView {
                    key,
                    sort: crate::panorama::tasks::sort_key(&metric),
                    metric: metric.clone(),
                    list: Vec::new(),
                    sampler: Default::default(),
                    kill: None,
                    busy: false,
                });
                self.panorama_tasks_refresh()
            }
            PMsg::TasksSort(k) => {
                self.panorama.last_press = Some(Instant::now());
                if let Some(t) = self.panorama.tasks.as_mut() {
                    t.sort = k;
                    t.kill = None;
                    t.metric = match k {
                        "mem" => "ram",
                        "gpu" => "gpu",
                        "vram" => "vram",
                        "io" => "disk",
                        _ => "cpu",
                    }
                    .into();
                    crate::panorama::tasks::sort(&mut t.list, k);
                }
                self.panorama_tasks_refresh()
            }
            PMsg::TasksClose => {
                self.panorama.tasks = None;
                Task::none()
            }
            PMsg::TasksData(mut l) => {
                if let Some(t) = self.panorama.tasks.as_mut() {
                    crate::panorama::tasks::sort(&mut l, t.sort);
                    t.list = l;
                    t.busy = false;
                }
                Task::none()
            }
            PMsg::TaskKill(pid) => {
                self.panorama.last_press = Some(Instant::now());
                let Some(t) = self.panorama.tasks.as_mut() else {
                    return Task::none();
                };
                if t.kill != Some(pid) {
                    t.kill = Some(pid);
                    return Task::none();
                }
                t.kill = None;
                if let Err(e) = crate::panorama::tasks::terminate(pid) {
                    tracing::warn!("Panorama : terminer {pid} : {e}");
                }
                self.panorama_tasks_refresh()
            }
            PMsg::MicsRefresh => Task::perform(
                async {
                    tokio::task::spawn_blocking(crate::voice::microphones)
                        .await
                        .unwrap_or_default()
                },
                |l| wrap(PMsg::Mics(l)),
            ),
            PMsg::Mics(l) => {
                self.panorama.voice_mics = l;
                Task::none()
            }
            PMsg::VoiceTest => {
                if !crate::voice::ready() {
                    return Task::none();
                }
                self.voice_say(
                    crate::i18n::tr("Bonjour, je suis Lyra. Dites mon nom, puis votre demande.")
                        .into(),
                    "fr".into(),
                )
            }
            PMsg::VoiceArm => {
                crate::voice::control("resume");
                crate::voice::control("arm");
                self.panorama.voice = VoiceUi::Awake;
                Task::none()
            }
            PMsg::VoiceDone => {
                crate::voice::control("resume");
                if self.cfg.general.panorama.voice {
                    self.panorama.voice = VoiceUi::Listening;
                }
                Task::none()
            }
            PMsg::Signed(cookies) => {
                let c = &self.cfg.general.panorama;
                // Service web de l'onglet Webradio (TuneIn…) compris.
                let web = crate::panorama::radio::SERVICES
                    .iter()
                    .filter_map(|s| s.2.map(String::from));
                self.panorama.signed = match cookies {
                    Some(k) => c
                        .sources
                        .iter()
                        .chain(&c.music_sources)
                        .chain(&c.radios)
                        .map(|s| s.url.clone())
                        .chain(web)
                        .map(|u| {
                            let on = crate::panorama::auth::signed_in(&k, &u);
                            (u, on)
                        })
                        .collect(),
                    None => HashMap::new(),
                };
                Task::none()
            }
            PMsg::IconsAuto(r) => {
                self.panorama.icons_job = None;
                if let Err(e) = &r {
                    tracing::warn!("Panorama : icônes SmartUI : {e}");
                }
                self.panorama.icons_report = Some(r);
                Task::none()
            }
            PMsg::Updates(s) => {
                self.panorama.updates = s;
                // Jamais relevé : une recherche, une fois par session.
                if !self.panorama.updates_asked
                    && self
                        .panorama
                        .updates
                        .as_ref()
                        .is_none_or(|u| u.checked == 0 && u.state == "idle")
                {
                    return self.panorama_updates_run("--refresh");
                }
                Task::none()
            }
            PMsg::UpdatesRun(arg) => self.panorama_updates_run(arg),
            PMsg::ShortcutResult(r) => {
                self.panorama.design_error = r.err();
                Task::none()
            }
            m @ (PMsg::Side(_)
            | PMsg::SideModule(_)
            | PMsg::SideWindow(_)
            | PMsg::SideTaken(_)
            | PMsg::SideDone(_)
            | PMsg::SidePreset(..)
            | PMsg::SideHold(..)
            | PMsg::SideEdit(_)
            | PMsg::SideMove(..)
            | PMsg::SideRemove(_)
            | PMsg::SideSwipe(_)) => self.panorama_side_update(m),
            m @ (PMsg::Layout(_)
            | PMsg::Profile(_)
            | PMsg::AutoFull(..)
            | PMsg::PlayUrl(..)
            | PMsg::MusicTab(_)
            | PMsg::RadioSearch(..)
            | PMsg::RadioFound(..)
            | PMsg::RadioFav(..)
            | PMsg::RadioPoll(..)
            | PMsg::RadioStatus(..)
            | PMsg::RadioFailed(..)
            | PMsg::RadioLogos(_)
            | PMsg::RadioLogosFound(_)
            | PMsg::Frame(_)
            | PMsg::Orphans(_)
            | PMsg::Pane(..)
            | PMsg::VideoOpen(_)
            | PMsg::VideoPoll(_)
            | PMsg::VideoFound(_)
            | PMsg::VideoPlaced(_)
            | PMsg::CaptureScan(_)
            | PMsg::CapturePlaced(..)
            | PMsg::VideoClose
            | PMsg::CastCard(_)) => self.panorama_panes_update(m),
            PMsg::PreviewAt(at) => {
                self.panorama.preview_at = at;
                Task::none()
            }
            PMsg::Detected(screens, touch) => {
                self.panorama.screens = screens;
                self.panorama.touch = touch;
                let offers = self.panorama_offers();
                Task::batch([self.sync_bars(), self.sync_panorama(), offers])
            }
            PMsg::Cross(mon, dir) => {
                self.panorama.last_press = Some(Instant::now());
                if dir > 0 {
                    self.panorama.notifs = None;
                    self.panorama.picker = Some(mon);
                    Task::none()
                } else if self.panorama.picker.take().is_some()
                    || self.panorama.notifs.take().is_some()
                {
                    Task::none()
                } else {
                    self.panorama_update(PMsg::Hide(mon))
                }
            }
            m @ (PMsg::HueRooms(..)
            | PMsg::HueSet(..)
            | PMsg::HueLight(..)
            | PMsg::HueDrag(..)
            | PMsg::HueLightDrag(..)
            | PMsg::HueFocus(_)
            | PMsg::HueRelease
            | PMsg::ModulesEdit(_)
            | PMsg::ModuleToggle(_)
            | PMsg::NotesOpen
            | PMsg::NotesOpened(_)) => self.panorama_modules_update(m),
            PMsg::PickerClose => {
                self.panorama.picker = None;
                Task::none()
            }
            PMsg::Notifs(mon) => {
                self.panorama.last_press = Some(Instant::now());
                // Le volet des notifications qui s'ouvre lit tout (HIG SYS-02).
                let opened = mon.is_some();
                if opened {
                    self.panorama.picker = None;
                }
                self.panorama.notifs = mon;
                if opened {
                    self.toasts_update(crate::ui::toasts::TMsg::Read)
                } else {
                    Task::none()
                }
            }
            PMsg::NotifsClear => {
                self.toasts.command("clear");
                Task::none()
            }
            PMsg::Swipe(mon, dir) => {
                let n = self.panorama_carousel() as i64;
                let p = self.panorama.page.entry(mon).or_insert(0);
                *p = (*p as i64 + dir as i64).rem_euclid(n) as usize;
                Task::none()
            }
            PMsg::Goto(mon, i) => {
                self.panorama.page.insert(mon, i);
                Task::none()
            }
            PMsg::Hide(mon) if mon == PREVIEW => self.panorama_update(PMsg::ClosePreview),
            PMsg::Hide(mon) => {
                self.panorama.hidden.insert(mon);
                self.sync_panorama()
            }
            PMsg::Show(mon) => {
                self.panorama.hidden.remove(&mon);
                self.panorama.last_press = Some(Instant::now());
                // La surface revient sur son volet des notifications ouvert :
                // la liste se montre, tout est lu (HIG SYS-02).
                let read = if self.panorama.notifs.is_some() {
                    self.toasts_update(crate::ui::toasts::TMsg::Read)
                } else {
                    Task::none()
                };
                Task::batch([self.sync_panorama(), read])
            }
            PMsg::Toggle(mon) => {
                let list = self.panorama_monitors();
                let Some(mon) = mon.or_else(|| list.first().cloned()) else {
                    return Task::none();
                };
                if self.panorama.hidden.contains(&mon) {
                    self.panorama_update(PMsg::Show(mon))
                } else {
                    self.panorama_update(PMsg::Hide(mon))
                }
            }
            PMsg::Accept(id) => {
                let Some(o) = self.panorama.offers.remove(&id) else {
                    return Task::none();
                };
                let mut c = self.panorama_cfg();
                let key = if o.edid.is_empty() {
                    o.screen.clone()
                } else {
                    o.edid.clone()
                };
                c.declined.retain(|d| d != &key);
                if !c.screens.contains(&key) {
                    c.screens.push(key);
                }
                // Tactile présent : il suit l'écran choisi (« Oui » touché ici).
                let touch = if self.panorama.touch.is_empty() {
                    Task::none()
                } else {
                    let (path, out) = (self.config_path.clone(), o.screen.clone());
                    Task::perform(
                        async move {
                            tokio::task::spawn_blocking(move || {
                                crate::panorama::map_touch(&path, Some(&out))
                            })
                            .await
                            .unwrap_or_else(|e| Err(e.to_string()))
                        },
                        |r| wrap(PMsg::Touch(r)),
                    )
                };
                Task::batch([iced::window::close(id), self.panorama_save(c), touch])
            }
            PMsg::Decline(id) => {
                let Some(o) = self.panorama.offers.remove(&id) else {
                    return Task::none();
                };
                let mut c = self.panorama_cfg();
                let key = if o.edid.is_empty() { o.screen } else { o.edid };
                if !c.declined.contains(&key) {
                    c.declined.push(key);
                }
                Task::batch([iced::window::close(id), self.panorama_save(c)])
            }
            PMsg::Later(id) => {
                self.panorama.offers.remove(&id);
                iced::window::close(id)
            }
            PMsg::Tick => {
                if !self.panorama_ticking() {
                    return Task::none();
                }
                self.panorama_video_gone();
                self.panorama_side_gone();
                let mut strays = Task::batch([
                    self.panorama_tasks_refresh(),
                    self.panorama_close_strays(),
                    self.panorama_check_signed(),
                    self.panorama_auto_icons(),
                    self.panorama_auto_full(),
                    self.panorama_capture_scan(),
                    self.panorama_anchor(),
                    self.panorama_side_anchor(),
                    self.panorama_updates_read(),
                    self.panorama_hue_tick(),
                ]);
                if !self.panorama.orphans_checked {
                    self.panorama.orphans_checked = true;
                    // Radio en cours avant un redémarrage de hyprdesk : reprise.
                    if self.panorama.radio.is_none()
                        && self.panorama.video.is_none()
                        && let Some((name, url)) = crate::panorama::radio_play::to_resume()
                    {
                        strays = Task::batch([strays, Task::done(wrap(PMsg::PlayUrl(name, url)))]);
                    }
                    strays = Task::batch([
                        strays,
                        Task::perform(
                            async {
                                tokio::task::spawn_blocking(|| {
                                    // Captures d'une session précédente : rendues
                                    // à la surveillance, qui les reprend.
                                    let n = crate::panorama::capture::release_stale();
                                    if n > 0 {
                                        tracing::info!("Panorama : {n} capture(s) reprise(s)");
                                    }
                                    crate::panorama::video::tagged()
                                })
                                .await
                                .unwrap_or_default()
                            },
                            |l| wrap(PMsg::Orphans(l)),
                        ),
                    ]);
                }
                // Vidéo posée dans l'aperçu : la fenêtre suit la sienne.
                // (dès l'ouverture : la position est connue quand elle apparaît).
                let follow = if self.panorama.preview.is_some() && self.panorama.video.is_some() {
                    Task::perform(
                        async {
                            tokio::task::spawn_blocking(|| {
                                let c = hyprpanel_core::ipc::query_json("clients").ok()?;
                                c.as_array()?.iter().find_map(|w| {
                                    (w["class"].as_str() == Some(PREVIEW_APP_ID)).then(|| {
                                        (
                                            w["at"][0].as_i64().unwrap_or(0) as i32,
                                            w["at"][1].as_i64().unwrap_or(0) as i32,
                                        )
                                    })
                                })
                            })
                            .await
                            .ok()
                            .flatten()
                        },
                        |at| wrap(PMsg::PreviewAt(at)),
                    )
                } else {
                    Task::none()
                };
                let media = Task::batch([
                    follow,
                    strays,
                    Task::perform(crate::widgets::media::status(), |s| wrap(PMsg::Media(s))),
                ]);
                let wants_sys = (self.cfg.general.panorama.layout
                    == crate::panorama::config::Layout::Dashboard
                    && self
                        .cfg
                        .general
                        .panorama
                        .design
                        .active_tiles(1600.0, 450.0)
                        .iter()
                        .map(|t| t.widget.clone())
                        // Bande des modules du profil Raccourcis.
                        .chain(
                            if self.cfg.general.panorama.design.shortcut_profile == "system" {
                                self.cfg.general.panorama.design.modules()
                            } else {
                                Default::default()
                            },
                        )
                        .any(|w| {
                            w == "system" || crate::panorama::design::metric_of(&w).is_some()
                        }))
                    || self
                        .panorama
                        .surfaces
                        .values()
                        .map(String::as_str)
                        .chain(self.panorama.preview.map(|_| PREVIEW))
                        .any(|m| {
                            self.panorama_page_at(m).is_some_and(|p| {
                                p.tiles.iter().any(|t| {
                                    t.widget == "system"
                                        || crate::panorama::design::metric_of(&t.widget).is_some()
                                })
                            })
                        })
                    || (self.cfg.general.panorama.layout
                        != crate::panorama::config::Layout::Single
                        && self
                            .cfg
                            .general
                            .panorama
                            .panes_of(self.cfg.general.panorama.layout)
                            .iter()
                            .any(|p| p == "system"));
                if !wants_sys {
                    return media;
                }
                // Mesures du widget Système, et de chaque tuile de mesure
                // (composition et pages).
                let c = &self.cfg.general.panorama;
                let mut ids: Vec<String> = c
                    .design
                    .metrics
                    .split(',')
                    .map(|m| m.trim().to_string())
                    .filter(|m| !m.is_empty())
                    .collect();
                for t in c
                    .design
                    .tiles
                    .iter()
                    .chain(c.pages.iter().flat_map(|p| &p.tiles))
                {
                    if let Some((m, _)) = crate::panorama::design::metric_of(&t.widget)
                        && !ids.iter().any(|i| i == m)
                    {
                        ids.push(m.to_string());
                    }
                }
                let sys = Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            crate::panorama::telemetry::sample_ids(&ids)
                        })
                        .await
                        .unwrap_or_default()
                    },
                    |l| wrap(PMsg::Sys(l)),
                );
                Task::batch([media, sys])
            }
            PMsg::Media(s) => {
                // Pochette locale (file://) : chargée une fois par adresse.
                let art = s
                    .as_ref()
                    .and_then(|s| s.art.strip_prefix("file://"))
                    .map(str::to_string);
                match art {
                    Some(path) if self.panorama.cover.as_ref().map(|c| &c.0) != Some(&path) => {
                        let h = image::Handle::from_path(&path);
                        self.panorama.cover = Some((path, h));
                    }
                    None => self.panorama.cover = None,
                    _ => {}
                }
                self.panorama.media = s;
                self.panorama_sync_idle();
                Task::none()
            }
            PMsg::Sys(l) => {
                let limit = self.cfg.general.panorama.design.history_seconds as usize;
                for (_, v) in &l {
                    if let Some(r) = v.ratio {
                        let h = self.panorama.history.entry(v.label.clone()).or_default();
                        h.push_back(r);
                        // Deux relevés de plus que la largeur : le plus
                        // ancien sort du graphique avant d'être oublié.
                        while h.len() > limit + 2 {
                            h.pop_front();
                        }
                    }
                }
                // Widget Système : ses mesures, dans l'ordre réglé.
                let wanted: Vec<&str> = self
                    .cfg
                    .general
                    .panorama
                    .design
                    .metrics
                    .split(',')
                    .map(str::trim)
                    .collect();
                self.panorama.sys = wanted
                    .iter()
                    .filter_map(|w| l.iter().find(|(id, _)| id == w).map(|(_, v)| v.clone()))
                    .collect();
                self.panorama.sys_ids = l.into_iter().collect();
                self.panorama.sys_at = Some(Instant::now());
                Task::none()
            }
            PMsg::Control(method) => {
                let Some(bus) = self.panorama.media.as_ref().map(|s| s.bus.clone()) else {
                    return Task::none();
                };
                if method == "PlayPause"
                    && let Some(s) = self.panorama.media.as_mut()
                {
                    s.playing = !s.playing;
                }
                Task::perform(crate::widgets::media::media_control(bus, method), |_| {
                    wrap(PMsg::Tick)
                })
            }
            PMsg::Pressed => {
                self.panorama.last_press = Some(Instant::now());
                Task::none()
            }
            PMsg::Preview(size) => {
                if let Some(p) = self.panorama.preview.take() {
                    let close = iced::window::close(p.win);
                    if p.virtual_size == size {
                        return close;
                    }
                    return Task::batch([close, self.panorama_open_preview(size)]);
                }
                self.panorama_open_preview(size)
            }
            PMsg::ClosePreview => match self.panorama.preview.take() {
                Some(p) => iced::window::close(p.win),
                None => Task::none(),
            },
            PMsg::Touch(r) => {
                if let Err(e) = r {
                    tracing::warn!(
                        "{}",
                        crate::tr!("Panorama : tactile non rattaché : {e}", e = e)
                    );
                }
                Task::none()
            }
        }
    }

    /// `hyprdesk panorama <toggle|show|hide|page <n|id>|detect>`.
    pub(in crate::ui) fn panorama_command(&mut self, a: &str) -> Task<Msg> {
        let mut it = a.split_whitespace();
        let verb = it.next().unwrap_or("toggle");
        // Paquet coupé : ses commandes ne font rien, sauf LYRA vocale, que le
        // module de barre `voice` sert aussi.
        if verb != "voice" && !self.cfg.plugin_on("panorama") {
            tracing::warn!(
                "{}",
                crate::tr!(
                    "hyprdesk panorama {v} : paquet Panorama coupé (Réglages → Plugins → Installés)",
                    v = verb
                )
            );
            return Task::none();
        }
        let first = self.panorama_monitors().into_iter().next();
        match (verb, first) {
            ("detect", _) => {
                // Nouvelle passe : les écrans déjà proposés peuvent l'être encore.
                self.panorama.offered.clear();
                detect_task()
            }
            // Côte à côte : split [toggle|on|off|swap].
            ("split", _) => match it.next().unwrap_or("toggle") {
                "toggle" => self.panorama_update(PMsg::Side("toggle")),
                "on" => self.panorama_update(PMsg::Side("on")),
                "off" => self.panorama_update(PMsg::Side("off")),
                "swap" => self.panorama_update(PMsg::Side("swap")),
                _ => Task::none(),
            },
            ("layout", _) => match it.next().and_then(crate::panorama::config::Layout::parse) {
                Some(l) => self.panorama_update(PMsg::Layout(l)),
                None => Task::none(),
            },
            ("video", _) => {
                let want = it.collect::<Vec<_>>().join(" ");
                if want == "close" || want == "stop" {
                    return self.panorama_update(PMsg::VideoClose);
                }
                let sources = self.panorama_sources();
                let i = want
                    .parse::<usize>()
                    .ok()
                    .map(|n| n.saturating_sub(1))
                    .or_else(|| {
                        let w = want.to_lowercase();
                        sources.iter().position(|s| {
                            s.name.to_lowercase() == w
                                || crate::panorama::video::host(&s.url).contains(&w)
                        })
                    });
                match i {
                    Some(i) if i < sources.len() => self.panorama_update(PMsg::VideoOpen(i)),
                    _ => Task::none(),
                }
            }
            // LYRA : ouvrir une adresse, chercher une radio, chercher de la
            // musique sur la plateforme active.
            ("open", _) => {
                let url = it.next().unwrap_or("").to_string();
                if !(url.starts_with("https://") || url.starts_with("http://")) {
                    return Task::none();
                }
                let name = it.collect::<Vec<_>>().join(" ");
                // Sans nom (lien partagé depuis le téléphone) : celui de la
                // source du même site, sinon le site.
                let name = if name.is_empty() {
                    let host = crate::panorama::video::host(&url);
                    let bare = |h: &str| h.trim_start_matches("www.").to_string();
                    let c = &self.cfg.general.panorama;
                    c.sources
                        .iter()
                        .chain(&c.music_sources)
                        .find(|s| bare(&crate::panorama::video::host(&s.url)) == bare(&host))
                        .map(|s| s.name.clone())
                        .unwrap_or(host)
                } else {
                    name
                };
                self.panorama_update(PMsg::PlayUrl(name, url))
            }
            // Gestionnaire des tâches : tasks <cpu|ram|gpu|vram|disk> | close.
            ("tasks", first) => match it.next().unwrap_or("cpu") {
                "close" => self.panorama_update(PMsg::TasksClose),
                m => match first {
                    Some(key) => self.panorama_update(PMsg::TasksOpen(key, m.to_string())),
                    None => Task::none(),
                },
            },
            ("radio", _) => {
                let q = it.collect::<Vec<_>>().join(" ");
                if q.is_empty() {
                    return Task::none();
                }
                self.panorama_update(PMsg::RadioSearch(q, true))
            }
            ("music", _) => {
                let q = it.collect::<Vec<_>>().join(" ");
                // Plateforme active : celle du lecteur, sinon la première
                // des services de musique.
                let c = &self.cfg.general.panorama;
                let base = self
                    .panorama
                    .video
                    .as_ref()
                    .map(|v| v.url.clone())
                    .filter(|u| crate::panorama::radio::search_url(u, "x").is_some())
                    .or_else(|| c.music_sources.first().map(|s| s.url.clone()));
                let Some(base) = base else {
                    return Task::none();
                };
                let name = c
                    .music_sources
                    .iter()
                    .chain(&c.sources)
                    .find(|s| {
                        crate::panorama::video::host(&s.url) == crate::panorama::video::host(&base)
                    })
                    .map(|s| s.name.clone())
                    .unwrap_or_else(|| crate::panorama::video::host(&base));
                match crate::panorama::radio::search_url(&base, &q) {
                    Some(url) if !q.is_empty() => {
                        self.panorama.music_tab = "playlist";
                        self.panorama_update(PMsg::PlayUrl(name, url))
                    }
                    _ => Task::none(),
                }
            }
            // LYRA vocale : on, off, toggle ; ask <fr|en> <texte> simule une
            // demande entendue (réponse dite).
            ("voice", _) if !self.cfg.plugin_on("assistant") => Task::none(),
            ("voice", _) => match it.next().unwrap_or("toggle") {
                "ask" => {
                    let lang = it.next().unwrap_or("fr").to_string();
                    let text = it.collect::<Vec<_>>().join(" ");
                    if text.is_empty() {
                        return Task::none();
                    }
                    self.panorama_update(PMsg::Voice(crate::voice::Event::Heard(lang, text)))
                }
                "on" if self.cfg.general.panorama.voice => Task::none(),
                "off" if !self.cfg.general.panorama.voice => Task::none(),
                _ => self.panorama_update(PMsg::VoiceToggle),
            },
            ("preview", _) => {
                let size = it.next().and_then(parse_size).unwrap_or(PREVIEW_SIZE);
                self.panorama_update(PMsg::Preview(size))
            }
            ("toggle", m) => self.panorama_update(PMsg::Toggle(m)),
            ("show", Some(m)) => self.panorama_update(PMsg::Show(m)),
            ("hide", Some(m)) => self.panorama_update(PMsg::Hide(m)),
            ("page", Some(m)) => {
                let want = it.next().unwrap_or("");
                let pages = &self.cfg.general.panorama.pages;
                let i = want
                    .parse::<usize>()
                    .ok()
                    .map(|n| n.saturating_sub(1))
                    .or_else(|| pages.iter().position(|p| p.name.eq_ignore_ascii_case(want)));
                match i {
                    Some(i) if i < pages.len() => {
                        let i = i + self.panorama_offset();
                        self.panorama.hidden.remove(&m);
                        self.panorama.page.insert(m, i);
                        self.sync_panorama()
                    }
                    _ => Task::none(),
                }
            }
            _ => Task::none(),
        }
    }

    /// Ouvre la fenêtre d'aperçu (règle flottante posée d'abord).
    fn panorama_open_preview(&mut self, size: (u32, u32)) -> Task<Msg> {
        let screen = self
            .monitors
            .iter()
            .find(|m| m.focused)
            .or(self.monitors.first())
            .map(|m| (m.width, m.height))
            .unwrap_or((1920, 1080));
        let ((w, h), scale) = preview_fit(size, screen);
        let win = Id::unique();
        self.panorama.preview = Some(Preview {
            win,
            virtual_size: size,
            scale,
        });
        self.panorama.last_press = Some(Instant::now());
        let lua = preview_rule_lua(w, h);
        let mut tasks: Vec<Task<Msg>> = self
            .panorama_icon_names()
            .iter()
            .map(|n| self.want_icon(n))
            .collect();
        tasks.push(Task::perform(
            async move {
                let _ = tokio::task::spawn_blocking(move || hyprpanel_core::ipc::eval(&lua)).await;
            },
            move |_| Msg::NewBaseWindow {
                settings: iced_layershell::actions::IcedXdgWindowSettings {
                    title: Some(preview_title(size)),
                    app_id: Some(PREVIEW_APP_ID.into()),
                    size: Some((w, h)),
                    client_side_decorations: false,
                },
                id: win,
            },
        ));
        tasks.push(Task::done(wrap(PMsg::Tick)));
        Task::batch(tasks)
    }

    /// Titre de la fenêtre d'aperçu (aucun pour les autres surfaces).
    pub(in crate::ui) fn panorama_title(&self, id: Id) -> Option<String> {
        self.panorama
            .preview
            .filter(|p| p.win == id)
            .map(|p| preview_title(p.virtual_size))
    }

    /// Relevés utiles : un Panorama ou son aperçu est affiché.
    pub(in crate::ui) fn panorama_ticking(&self) -> bool {
        !self.panorama.surfaces.is_empty() || self.panorama.preview.is_some()
    }

    /// LYRA vocale peut écouter : réglée (`panorama.voice`), LYRA active
    /// (paquet Assistant), et visible quelque part — le Panorama, ou le
    /// module de barre `voice`. Sans endroit où voir l'écoute, le micro reste
    /// fermé.
    pub(in crate::ui) fn voice_allowed(&self) -> bool {
        self.cfg.general.panorama.voice
            && self.cfg.plugin_on("assistant")
            && (self.cfg.plugin_on("panorama") || self.voice_in_bar())
    }

    /// Le module de barre `voice` est posé dans la barre.
    fn voice_in_bar(&self) -> bool {
        let ids: Vec<&str> = self
            .cfg
            .widgets
            .iter()
            .filter(|w| w.type_id == VOICE_WIDGET)
            .map(|w| w.id.as_str())
            .collect();
        !ids.is_empty()
            && ids
                .iter()
                .any(|id| self.bar_has(&crate::config::BarItem::Widget((*id).to_string())))
    }

    /// État de LYRA vocale pour le module de barre `voice`, écrit dans
    /// `$XDG_RUNTIME_DIR/hyprdesk/voix` quand il change : `state=` absent
    /// (LYRA coupée), off, installing, listening, awake, thinking, speaking,
    /// error ; `detail=` phrase entendue ou erreur ; `settings=` page de
    /// ses réglages.
    pub(in crate::ui) fn voice_publish(&mut self) {
        let on = self.voice_allowed();
        let (state, detail) = if !self.cfg.plugin_on("assistant") {
            ("absent", String::new())
        } else {
            match &self.panorama.voice {
                VoiceUi::Installing(step) => ("installing", step.clone()),
                VoiceUi::Error(e) => ("error", e.clone()),
                _ if !on => ("off", String::new()),
                VoiceUi::Idle | VoiceUi::Listening => ("listening", String::new()),
                VoiceUi::Awake => (
                    "awake",
                    self.panorama.voice_heard.clone().unwrap_or_default(),
                ),
                VoiceUi::Thinking => (
                    "thinking",
                    self.panorama.voice_heard.clone().unwrap_or_default(),
                ),
                VoiceUi::Speaking => ("speaking", String::new()),
            }
        };
        // Réglages de LYRA vocale : IA → Voix.
        let settings = "assistant/voix";
        let detail = detail.replace('\n', " ");
        let text = format!("state={state}\ndetail={detail}\nsettings={settings}\n");
        if text == self.panorama.voice_written {
            return;
        }
        let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR") else {
            return;
        };
        let dir = std::path::PathBuf::from(dir).join("hyprdesk");
        let _ = std::fs::create_dir_all(&dir);
        if std::fs::write(dir.join("voix"), &text).is_ok() {
            self.panorama.voice_written = text;
        }
    }

    /// Surface fermée par le compositeur (écran débranché…) ou fenêtre
    /// d'aperçu fermée par l'utilisateur.
    pub(in crate::ui) fn panorama_closed(&mut self, id: Id) -> bool {
        if self.panorama.preview.is_some_and(|p| p.win == id) {
            self.panorama.preview = None;
            return true;
        }
        self.panorama.surfaces.remove(&id).is_some()
            || self.panorama.handles.remove(&id).is_some()
            || self.panorama.offers.remove(&id).is_some()
    }

    pub(in crate::ui) fn panorama_owns(&self, id: Id) -> bool {
        self.panorama.preview.is_some_and(|p| p.win == id)
            || self.panorama.surfaces.contains_key(&id)
            || self.panorama.handles.contains_key(&id)
            || self.panorama.offers.contains_key(&id)
            || self.panorama.halo.contains_key(&id)
    }

    /// Vue composée en tête du carrousel (découpage, composition libre) :
    /// 1, sinon 0 (une page à la fois).
    fn panorama_offset(&self) -> usize {
        usize::from(self.cfg.general.panorama.layout != crate::panorama::config::Layout::Single)
    }

    /// Entrées du carrousel d'un écran : la vue composée, puis les pages.
    pub(in crate::ui) fn panorama_carousel(&self) -> usize {
        self.cfg.general.panorama.pages.len().max(1) + self.panorama_offset()
    }

    /// Disposition affichée sur l'écran `key` : la configurée sur la vue
    /// composée (entrée 0), une page à la fois sur les pages qui suivent.
    pub(in crate::ui) fn panorama_layout_at(&self, key: &str) -> crate::panorama::config::Layout {
        let i = self.panorama.page.get(key).copied().unwrap_or(0);
        if i >= self.panorama_offset() {
            crate::panorama::config::Layout::Single
        } else {
            self.cfg.general.panorama.layout
        }
    }

    /// Page affichée sur un écran, hors vue par défaut.
    pub(in crate::ui) fn panorama_page_at(
        &self,
        monitor: &str,
    ) -> Option<&crate::panorama::design::Page> {
        let pages = &self.cfg.general.panorama.pages;
        let i = self
            .panorama
            .page
            .get(monitor)
            .copied()
            .unwrap_or(0)
            .saturating_sub(self.panorama_offset());
        pages.get(i.min(pages.len().saturating_sub(1)))
    }

    /// Veille du système retenue pendant qu'une vidéo du Panorama joue
    /// (MLG-249). La fenêtre posée dans la zone est souvent empruntée (PiP
    /// d'un navigateur dont l'onglet d'origine n'est plus à l'écran) : son
    /// navigateur ne retient pas la veille, hypridle verrouillait en pleine
    /// lecture. hyprdesk tient alors un inhibiteur logind `idle` (respecté
    /// par hypridle : `ignore_systemd_inhibit = false`), relâché à la pause,
    /// à l'arrêt ou au départ de la vidéo ; il meurt avec le démon
    /// (`tail --pid`), jamais orphelin. Une vidéo en pause n'empêche ni la
    /// veille ni le verrouillage.
    pub(in crate::ui) fn panorama_sync_idle(&mut self) {
        let want = !self.preview
            && video_playing(self.panorama.video.as_ref(), self.panorama.media.as_ref());
        // Inhibiteur tombé tout seul (logind relancé…) : à reprendre.
        if let Some(c) = self.panorama.idle_hold.as_mut()
            && !matches!(c.try_wait(), Ok(None))
        {
            self.panorama.idle_hold = None;
        }
        match (want, self.panorama.idle_hold.take()) {
            (true, Some(c)) => self.panorama.idle_hold = Some(c),
            (true, None) => {
                let pid = std::process::id().to_string();
                match std::process::Command::new("systemd-inhibit")
                    .args([
                        "--what=idle",
                        "--who=hyprdesk",
                        "--why=Vidéo du Panorama en lecture",
                        "--mode=block",
                        "tail",
                        &format!("--pid={pid}"),
                        "-f",
                        "/dev/null",
                    ])
                    .stdin(std::process::Stdio::null())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                {
                    Ok(c) => self.panorama.idle_hold = Some(c),
                    Err(e) => tracing::warn!(
                        "{}",
                        crate::tr!("veille retenue pour la vidéo : {e}", e = e)
                    ),
                }
            }
            (false, Some(mut c)) => {
                let _ = c.kill();
                let _ = c.wait();
            }
            (false, None) => {}
        }
    }

    /// Veille : rien ne joue et personne n'a touché depuis `sleep_minutes`.
    fn panorama_asleep(&self) -> bool {
        let mins = self.cfg.general.panorama.sleep_minutes;
        if mins == 0 || self.panorama.media.as_ref().is_some_and(|m| m.playing) {
            return false;
        }
        self.panorama
            .last_press
            .is_none_or(|t| t.elapsed() >= Duration::from_secs(mins as u64 * 60))
    }

    pub(in crate::ui) fn view_panorama(&self, id: Id) -> Element<'_, Msg> {
        if let Some(m) = self.panorama.handles.get(&id) {
            return self.view_panorama_handle(m);
        }
        if let Some(o) = self.panorama.offers.get(&id) {
            return self.view_panorama_offer(id, o);
        }
        if self.panorama.halo.contains_key(&id) {
            let c = &self.cfg.general.panorama;
            let (from, to) = halo::colors(&c.voice_halo_color, self.panorama_palette().2);
            let t = self
                .panorama
                .halo_since
                .map_or(0.0, |s| s.elapsed().as_secs_f32());
            let strength = match self.panorama.voice {
                VoiceUi::Speaking => 1.0,
                VoiceUi::Thinking => 0.75,
                _ => 0.9,
            };
            return halo::view(from, to, t, strength);
        }
        self.view_panorama_surface(id)
    }

    fn panorama_dots<'a>(
        &self,
        n: usize,
        cur: usize,
        mon: &str,
        fg: Color,
        accent: Color,
    ) -> Element<'a, Msg> {
        let mon = mon.to_string();
        Self::panorama_pager(n, cur, fg, accent, move |i| {
            wrap(PMsg::Goto(mon.clone(), i))
        })
    }

    /// Points de pages : la page affichée en trait, les autres en point ;
    /// un appui y mène.
    pub(in crate::ui) fn panorama_pager<'a>(
        n: usize,
        cur: usize,
        fg: Color,
        accent: Color,
        go: impl Fn(usize) -> Msg,
    ) -> Element<'a, Msg> {
        let items: Vec<Element<'a, Msg>> = (0..n)
            .map(|i| {
                let on = i == cur;
                button(
                    container(Space::new().width(if on { 26 } else { 10 }).height(10)).style(
                        move |_| container::Style {
                            background: Some(
                                if on { accent } else { Color { a: 0.35, ..fg } }.into(),
                            ),
                            border: Border::default().rounded(999),
                            ..Default::default()
                        },
                    ),
                )
                .padding(10)
                .on_press(go(i))
                .style(|_, _| button::Style::default())
                .into()
            })
            .collect();
        row(items).spacing(2).into()
    }

    fn panorama_button<'a>(&self, icon: &str, size: f32, fg: Color, msg: Msg) -> Element<'a, Msg> {
        let accent = self.panorama_palette().2;
        let primary = icon.starts_with("media-playback-");
        let ink = if primary {
            crate::ui::design::on_accent(accent, fg, color(self.cfg.general.theme.bg, 1.0))
        } else {
            fg
        };
        let glyph = self
            .icon_named(icon, size * 0.45, ink)
            .unwrap_or_else(|| text("•").size(size * 0.4).color(fg).into());
        self.panorama_feedback(
            button(
                container(glyph)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .center_x(Length::Fill)
                    .center_y(Length::Fill),
            )
            .width(size)
            .height(size)
            .on_press(msg)
            .style(move |_, st| button::Style {
                background: Some(
                    Color {
                        a: if primary {
                            1.0
                        } else {
                            match st {
                                button::Status::Pressed => 0.28,
                                button::Status::Hovered => 0.16,
                                _ => 0.08,
                            }
                        },
                        ..if primary { accent } else { fg }
                    }
                    .into(),
                ),
                border: Border {
                    color: fx::alpha(if primary { accent } else { fg }, 0.22),
                    width: 1.0,
                    radius: (size / 2.0).into(),
                },
                shadow: iced::Shadow {
                    color: fx::alpha(accent, if primary { 0.24 } else { 0.0 }),
                    offset: iced::Vector::new(0.0, 4.0),
                    blur_radius: 18.0,
                },
                ..Default::default()
            }),
        )
    }

    fn panorama_media(&self, z: &Zones, fg: Color) -> Element<'_, Msg> {
        let accent = self.panorama_palette().2;
        let dim = Color { a: 0.6, ..fg };
        let art_side = z.art.w;
        let bg = self.panorama_palette().0;
        let face = (art_side - 16.0).max(1.0);
        let art: Element<'_, Msg> = match &self.panorama.cover {
            Some((_, h)) => image(h.clone())
                .width(face)
                .height(face)
                .content_fit(iced::ContentFit::Cover)
                .border_radius(18.0)
                .into(),
            None => fx::album(face, bg, accent),
        };
        let art = self.panorama_feedback(
            container(art)
                .padding(8)
                .width(art_side)
                .height(art_side)
                .style(move |_| fx::glass(bg, fg, accent, 24.0))
                .clip(true),
        );
        let tgt = z.target;
        let short = z.unit;
        let (title, sub, playing, pos, len) = match &self.panorama.media {
            Some(s) => (
                if s.title.is_empty() {
                    s.player.clone()
                } else {
                    s.title.clone()
                },
                [s.artist.as_str(), s.album.as_str()]
                    .iter()
                    .filter(|x| !x.is_empty())
                    .copied()
                    .collect::<Vec<_>>()
                    .join(" · "),
                s.playing,
                s.position,
                s.length,
            ),
            None => (
                crate::i18n::tr("Rien ne joue").to_string(),
                crate::i18n::tr("Lancez un lecteur (Spotify, Firefox, mpv…)").to_string(),
                false,
                None,
                None,
            ),
        };
        let progress: Element<'_, Msg> = match (pos, len) {
            // Un direct annonce parfois une durée absurde : rien plutôt que faux.
            (Some(p), Some(l)) if l > 0 && l < 48 * 3600 && (0..=l).contains(&p) => {
                let r = (p as f32 / l as f32).clamp(0.0, 1.0);
                let filled = (r * 1000.0) as u16;
                let bar = |portion: u16, c: Option<Color>| -> Element<'_, Msg> {
                    container(Space::new())
                        .width(Length::FillPortion(portion.max(1)))
                        .height(8)
                        .style(move |_| container::Style {
                            background: c.map(Into::into),
                            border: Border::default().rounded(4),
                            ..Default::default()
                        })
                        .into()
                };
                column![
                    row![
                        bar(filled, Some(accent)),
                        bar(1000 - filled, Some(Color { a: 0.15, ..fg }))
                    ],
                    row![
                        text(fmt_time(p)).size(short * 0.045).color(dim),
                        Space::new().width(Length::Fill),
                        text(fmt_time(l)).size(short * 0.045).color(dim)
                    ]
                ]
                .spacing(6)
                .into()
            }
            _ => Space::new().into(),
        };
        let controls = row![
            self.panorama_button(
                "media-skip-backward-symbolic",
                tgt,
                fg,
                wrap(PMsg::Control("Previous"))
            ),
            self.panorama_button(
                if playing {
                    "media-playback-pause-symbolic"
                } else {
                    "media-playback-start-symbolic"
                },
                tgt * 1.35,
                fg,
                wrap(PMsg::Control("PlayPause"))
            ),
            self.panorama_button(
                "media-skip-forward-symbolic",
                tgt,
                fg,
                wrap(PMsg::Control("Next"))
            ),
        ]
        .spacing(tgt * 0.4)
        .align_y(Alignment::Center);
        let info = column![
            container(
                row![
                    container(Space::new())
                        .width(6)
                        .height(6)
                        .style(move |_| container::Style {
                            background: Some(if playing { accent } else { dim }.into()),
                            border: Border::default().rounded(3),
                            ..Default::default()
                        }),
                    text(crate::i18n::tr(if playing {
                        "En lecture"
                    } else {
                        "Média"
                    }))
                    .size((short * 0.032).clamp(11.0, 20.0))
                    .color(dim)
                    .font(self.panorama_font())
                ]
                .spacing(9)
                .align_y(Alignment::Center)
            )
            .padding([6, 10])
            .style(move |_| fx::glass(bg, fg, accent, 20.0)),
            text(title)
                .size((short * 0.095).clamp(20.0, 64.0))
                .color(fg)
                .font(iced::Font {
                    weight: iced::font::Weight::Semibold,
                    ..self.panorama_font()
                })
                .wrapping(text::Wrapping::None),
            text(sub)
                .size((short * 0.045).clamp(13.0, 30.0))
                .color(dim)
                .font(self.panorama_font())
                .wrapping(text::Wrapping::None),
            Space::new().height(Length::Fill),
            progress,
            Space::new().height(z.pad / 2.0),
            controls,
        ]
        .spacing(8)
        .width(z.main.w)
        .height(z.main.h)
        .clip(true);
        let side: Element<'_, Msg> = match (z.side, &self.panorama.media) {
            (Some(r), Some(s)) => column![
                text(crate::i18n::tr("Lecteur"))
                    .size(short * 0.04)
                    .color(dim)
                    .font(self.panorama_font()),
                text(s.player.split('.').next().unwrap_or(&s.player).to_string())
                    .size(short * 0.06)
                    .color(fg)
                    .font(self.panorama_font()),
            ]
            .spacing(6)
            .width(r.w)
            .height(r.h)
            .into(),
            (Some(r), None) => Space::new().width(r.w).height(r.h).into(),
            (None, _) => Space::new().width(0).into(),
        };
        let pad = z.pad;
        if z.compact {
            if z.art.w < 1.0 {
                return container(info).padding(pad).into();
            }
            return column![art, info].spacing(pad).padding(pad).into();
        }
        match z.orientation {
            Orientation::Landscape => row![art, info, side]
                .spacing(pad)
                .padding(pad)
                .align_y(Alignment::Start)
                .into(),
            Orientation::Portrait => column![art, info, side]
                .spacing(pad)
                .padding(pad)
                .align_x(Alignment::Center)
                .into(),
        }
    }

    fn panorama_system(
        &self,
        z: &Zones,
        fg: Color,
        size: iced::Size,
        reserve_controls: bool,
    ) -> Element<'_, Msg> {
        let accent = self.panorama_palette().2;
        let dim = Color { a: 0.6, ..fg };
        let short = z.unit;
        let columns = if z.compact {
            2
        } else if z.orientation == Orientation::Landscape {
            self.panorama.sys.len().max(1)
        } else {
            1
        };
        let rows = if z.compact {
            self.panorama.sys.len().div_ceil(2).max(1)
        } else if z.orientation == Orientation::Portrait {
            self.panorama.sys.len().max(1)
        } else {
            1
        };
        let footer = if reserve_controls { z.target } else { 0.0 };
        let card_width = (size.width - z.pad * (columns as f32 + 1.0)) / columns as f32;
        let card_height = (size.height - z.pad * (rows as f32 + 1.0) - footer) / rows as f32;
        let gauge_height = if card_height < 110.0 { 5.0 } else { 10.0 };
        let cards: Vec<Element<'_, Msg>> = self
            .panorama
            .sys
            .iter()
            .map(|l| {
                let bg = self.panorama_palette().0;
                let gauge: Element<'_, Msg> = match l.ratio {
                    Some(r) => {
                        let f = (r.clamp(0.0, 1.0) * 1000.0) as u16;
                        row![
                            container(Space::new())
                                .width(Length::FillPortion(f.max(1)))
                                .height(gauge_height)
                                .style(move |_| container::Style {
                                    background: Some(accent.into()),
                                    border: Border::default().rounded(5),
                                    ..Default::default()
                                }),
                            container(Space::new())
                                .width(Length::FillPortion((1000 - f).max(1)))
                                .height(gauge_height)
                                .style(move |_| container::Style {
                                    background: Some(Color { a: 0.15, ..fg }.into()),
                                    border: Border::default().rounded(5),
                                    ..Default::default()
                                }),
                        ]
                        .into()
                    }
                    None => Space::new().into(),
                };
                self.panorama_feedback(
                    container(
                        column![
                            text(l.label.clone())
                                .size((short * 0.05).min(card_height * 0.18).clamp(11.0, 24.0))
                                .color(dim)
                                .font(iced::Font {
                                    weight: iced::font::Weight::Semibold,
                                    ..self.panorama_font()
                                }),
                            text(l.value.clone())
                                .size(
                                    (short * 0.09)
                                        .min(card_width * 0.11)
                                        .min(card_height * 0.28)
                                        // Une valeur longue (« 128.0 GiB / 512.0 GiB »)
                                        // rapetisse pour tenir dans la carte au lieu
                                        // d'être coupée (≈ 0,6 em par caractère).
                                        .min(
                                            (card_width
                                                - 2.0 * z.pad.min((card_height * 0.14).max(4.0)))
                                                / (l.value.chars().count().max(1) as f32 * 0.6),
                                        )
                                        .clamp(11.0, 42.0)
                                )
                                .color(fg)
                                .wrapping(text::Wrapping::None)
                                .font(self.panorama_font()),
                            Space::new().height(Length::Fill),
                            gauge,
                        ]
                        .spacing((card_height * 0.05).clamp(2.0, 6.0)),
                    )
                    .padding(z.pad.min((card_height * 0.14).max(4.0)))
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .style(move |_| fx::glass(bg, fg, accent, 20.0)),
                )
            })
            .collect();
        let pad = z.pad;
        let body: Element<'_, Msg> = if z.compact {
            // Zone presque carrée : une grille de deux colonnes.
            let mut col = column![].spacing(pad);
            let mut it = cards.into_iter().peekable();
            while it.peek().is_some() {
                let mut line = row![].spacing(pad);
                for _ in 0..2 {
                    line = line.push(
                        it.next()
                            .unwrap_or_else(|| Space::new().width(Length::Fill).into()),
                    );
                }
                col = col.push(line);
            }
            col.into()
        } else {
            match z.orientation {
                Orientation::Landscape => row(cards).spacing(pad).into(),
                Orientation::Portrait => column(cards).spacing(pad).into(),
            }
        };
        container(body)
            .padding(iced::Padding {
                top: pad,
                left: pad,
                right: pad,
                bottom: pad + footer,
            })
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn panorama_clock(&self, z: &Zones, fg: Color) -> Element<'_, Msg> {
        let d = &self.cfg.general.panorama.design;
        if d.clock_style != "digital"
            || self.cfg.general.panorama.layout == crate::panorama::config::Layout::Dashboard
            || d.seconds
            || !d.date
            || d.text_scale != 1.0
        {
            return instruments::clock(
                &d.clock_style,
                d.seconds,
                d.date,
                fg,
                self.panorama_palette().2,
                self.panorama_font(),
                d.text_scale,
            );
        }
        let dim = Color { a: 0.6, ..fg };
        let short = z.unit;
        let now = chrono::Local::now();
        let date = now
            .format_localized("%A %e %B", crate::i18n::chrono_locale())
            .to_string();
        let mut col = column![
            text(now.format("%H:%M").to_string())
                .size(short * 0.42)
                .color(fg)
                .font(self.panorama_font()),
            text(date)
                .size(short * 0.08)
                .color(dim)
                .font(self.panorama_font()),
        ]
        .align_x(Alignment::Center)
        .spacing(4);
        if let Some(s) = self.panorama.media.as_ref().filter(|s| !s.title.is_empty()) {
            col = col.push(
                text(if s.artist.is_empty() {
                    s.title.clone()
                } else {
                    format!("{} — {}", s.title, s.artist)
                })
                .size(short * 0.05)
                .color(dim)
                .font(self.panorama_font()),
            );
        }
        container(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into()
    }

    fn view_panorama_handle(&self, mon: &str) -> Element<'_, Msg> {
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let bg = color(th.bg, 0.92);
        button(
            container(
                row![
                    self.icon_named("go-up-symbolic", 14.0, fg)
                        .unwrap_or_else(|| text("▴")
                            .size(crate::ui::tokens::ts(13.0))
                            .color(fg)
                            .into()),
                    text("Panorama")
                        .size(crate::ui::tokens::ts(13.0))
                        .color(fg)
                        .font(self.panorama_font())
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .center_x(Length::Fill)
            .center_y(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .on_press(wrap(PMsg::Show(mon.to_string())))
        .style(move |_, _| button::Style {
            background: Some(bg.into()),
            border: Border {
                radius: iced::border::Radius {
                    top_left: 13.0,
                    top_right: 13.0,
                    bottom_right: 0.0,
                    bottom_left: 0.0,
                },
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
    }

    fn view_panorama_offer(&self, id: Id, o: &Offer) -> Element<'_, Msg> {
        let th = &self.cfg.general.theme;
        let fg = color(th.fg, 1.0);
        let accent = self.panorama_palette().2;
        let bg = color(th.bg, 0.97);
        let dim = Color { a: 0.7, ..fg };
        let what = if o.touch.is_some() {
            crate::i18n::tr(
                "Écran tactile allongé détecté. En faire un Panorama : lecteur multimédia, système et horloge, pilotés au doigt ?",
            )
        } else if o.ambiguous {
            crate::i18n::tr(
                "Écran allongé détecté. En faire un Panorama ? Touchez « Oui » sur l'écran tactile : le tactile y sera rattaché.",
            )
        } else {
            crate::i18n::tr(
                "Écran allongé détecté. En faire un Panorama : lecteur multimédia, système et horloge, en plein écran ?",
            )
        };
        let btn = |label: &'static str, primary: bool, msg: Msg| -> Element<'_, Msg> {
            button(
                text(crate::i18n::tr(label))
                    .size(crate::ui::tokens::ts(17.0))
                    .font(self.panorama_font())
                    .color(if primary { Color::BLACK } else { fg }),
            )
            .padding([14, 26])
            .on_press(msg)
            .style(move |_, st| button::Style {
                background: Some(
                    if primary {
                        accent
                    } else {
                        Color {
                            a: if matches!(st, button::Status::Hovered | button::Status::Pressed) {
                                0.18
                            } else {
                                0.08
                            },
                            ..fg
                        }
                    }
                    .into(),
                ),
                border: Border::default().rounded(999),
                ..Default::default()
            })
            .into()
        };
        container(
            column![
                text(crate::i18n::tr("Panorama"))
                    .size(crate::ui::tokens::ts(26.0))
                    .font(self.panorama_font())
                    .color(fg),
                text(what)
                    .size(crate::ui::tokens::ts(15.0))
                    .font(self.panorama_font())
                    .color(dim),
                Space::new().height(Length::Fill),
                row![
                    btn("Oui, en faire un Panorama", true, wrap(PMsg::Accept(id))),
                    btn("Plus tard", false, wrap(PMsg::Later(id))),
                    btn("Non merci", false, wrap(PMsg::Decline(id))),
                ]
                .spacing(12),
            ]
            .spacing(10),
        )
        .padding(24)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(move |_| container::Style {
            background: Some(bg.into()),
            border: Border::default()
                .rounded(22)
                .width(1)
                .color(Color { a: 0.15, ..fg }),
            ..Default::default()
        })
        .into()
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn veille_retenue_seulement_pour_une_vidéo_en_lecture() {
        // MLG-249 : une PiP empruntée ne retient pas la veille elle-même.
        let mut v = preview_video("PiP");
        let joue = crate::widgets::media::Status {
            playing: true,
            ..Default::default()
        };
        let pause = crate::widgets::media::Status::default();
        assert!(video_playing(Some(&v), Some(&joue)));
        assert!(
            !video_playing(Some(&v), Some(&pause)),
            "pause : la veille reprend"
        );
        assert!(!video_playing(None, Some(&joue)), "musique seule : rien");
        v.address = None;
        assert!(
            !video_playing(Some(&v), Some(&joue)),
            "fenêtre pas encore posée"
        );
    }

    use super::*;

    #[test]
    fn apercu_tient_a_l_ecran() {
        // 2560×720 sur un 2560×1440 : réduit à 85 % de la largeur.
        let ((w, h), s) = preview_fit((2560, 720), (2560, 1440));
        assert_eq!((w, h), (2176, 612));
        assert!((s - 0.85).abs() < 1e-3);
        // Assez petit : taille réelle.
        assert_eq!(preview_fit((1280, 400), (2560, 1440)), ((1280, 400), 1.0));
        // Portrait : limité par la hauteur.
        let ((w, h), _) = preview_fit((720, 2560), (2560, 1440));
        assert_eq!(h, 1224);
        assert!((w as f32 / h as f32 - 720.0 / 2560.0).abs() < 0.01);
    }

    #[test]
    fn format_lu() {
        assert_eq!(parse_size("2560x720"), Some((2560, 720)));
        assert_eq!(parse_size("720×2560"), Some((720, 2560)));
        assert_eq!(parse_size("10x10"), None);
        assert_eq!(parse_size("abc"), None);
    }

    #[test]
    fn regle_flottante() {
        let l = preview_rule_lua(1280, 360);
        assert!(l.contains(r#"class = "^(hyprdesk-panorama-preview)$""#));
        assert!(l.contains(r#"float = true, center = true, size = "1280 360""#));
        assert!(l.contains("set_enabled(false)"));
    }

    /// HIG SYS-02 : le volet des notifications qui s'ouvre lit tout ; fermé,
    /// il ne lit rien.
    #[test]
    fn le_volet_des_notifications_qui_s_ouvre_lit_tout() {
        let (mut app, _) = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-panorama-tests/config.lua"),
            true,
        );
        app.toasts.history = (1..=2)
            .map(|id| {
                crate::notifications::build(id, "a", "", "s", "", &[], &Default::default(), -1)
            })
            .collect();
        let _ = app.panorama_update(PMsg::Notifs(None));
        assert_eq!(app.toasts.unread(), 2, "le volet fermé ne lit rien");
        let _ = app.panorama_update(PMsg::Notifs(Some("A".into())));
        assert_eq!(app.panorama.notifs.as_deref(), Some("A"));
        assert_eq!(app.toasts.unread(), 0, "le volet ouvert lit tout");
    }
}
