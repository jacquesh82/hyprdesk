//! Écran Réglages (⚙) : un panneau par-dessus le bureau, sur l'écran actif.
//!
//! Chaque changement modifie `App::settings`, puis est enregistré dans
//! `hyprdesk/settings.lua` (Lua généré) et rechargé comme n'importe quelle
//! modification de config. Les curseurs prévisualisent pendant le glissé et
//! n'enregistrent qu'au relâchement.

use crate::ui::tokens::Shown as _;
use std::sync::Arc;

use crate::i18n::text;
use iced::widget::{
    Space, button, column, container, mouse_area, row, rule, scrollable, toggler, tooltip,
};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Task};

use super::plugins::{self, PluginMsg};
use super::{App, Msg, color};
use crate::config::Rgba;
use crate::launch::{self};

/// Onglet du tiroir : l'identifiant d'un paquet de réglages (`plugins::BUILTIN`).
pub type Tab = &'static str;

/// Identifiant du champ de recherche des Réglages : il reprend le focus après
/// un effacement.
const SEARCH_ID: &str = "hyprdesk-panel-search";

/// Identifiant du champ de la valeur d'un curseur (HIG SEL-08) : un seul
/// champ à la fois, il prend le focus à l'ouverture.
const SLIDER_ID: &str = "hyprdesk-panel-slider";

/// Largeur du champ de la valeur d'un curseur : le nombre, aligné à droite
/// (HIG SEL-08).
const SLIDER_FIELD_W: f32 = 72.0;

/// Taille de référence des bornes sous un curseur des Réglages (HIG SEL-09) :
/// la légende, comme les aides du tiroir.
const SCALE_SIZE: f32 = 11.0;
/// Écart du curseur à son échelle (px).
const SCALE_GAP: f32 = 4.0;
/// Hauteur d'une graduation (px).
const TICK_H: f32 = 4.0;
/// Rayon de la poignée d'un curseur du tiroir (style par défaut d'iced) :
/// la poignée parcourt le rail de `r` à `largeur - r`, les graduations aussi.
const HANDLE_R: f32 = 7.0;

/// Hauteur d'une ligne de bornes sous un curseur (HIG SEL-09).
pub(in crate::ui) fn scale_line_h() -> f32 {
    (crate::ui::tokens::ts(SCALE_SIZE) * 1.3).round()
}

/// Nombre de pas d'un curseur gradué : dix au plus (HIG SEL-09), sinon aucun.
fn graduations(range: (f64, f64), step: f64) -> Option<usize> {
    let n = (range.1 - range.0).abs() / step;
    let entier = n.round();
    (step > 0.0 && n.is_finite() && (n - entier).abs() < 1e-6 && (1.0..=10.0).contains(&entier))
        .then_some(entier as usize)
}

/// Hauteur que l'échelle ajoute sous un curseur (bornes, et graduations s'il
/// en a) : la valeur et les voisins d'une rangée centrée la gardent sous eux
/// pour rester sur la ligne du curseur.
pub(in crate::ui) fn slider_band(range: (f64, f64), step: f64) -> f32 {
    let ticks = graduations(range, step).map_or(0.0, |_| TICK_H + SCALE_GAP);
    scale_line_h() + SCALE_GAP + ticks
}

/// Un voisin d'un curseur dans une rangée centrée (HIG SEL-09) : libellé,
/// bouton, interrupteur… garde sous lui la place de l'échelle et reste ainsi
/// sur la ligne du curseur.
pub(in crate::ui) fn beside_slider<'a, M: 'a>(
    spec: &SliderSpec,
    el: impl Into<Element<'a, M>>,
) -> Element<'a, M> {
    container(el)
        .padding(iced::Padding {
            bottom: slider_band(spec.range, spec.step),
            ..iced::Padding::ZERO
        })
        .into()
}

/// Messages du tiroir : ceux de la coquille, et ceux des paquets (enveloppés).
#[derive(Debug, Clone)]
pub enum PanelMsg {
    /// Affiche le paquet d'identifiant donné.
    Tab(Tab),
    /// Affiche un onglet du paquet (paquet, onglet) ; retenu pour la suite.
    SubTab(Tab, &'static str),
    /// Configuration avancée (tous les détails) ou de base (l'essentiel).
    Advanced(bool),
    /// Section avancée dépliée ou repliée : sa clé (HIG LAY-01).
    AdvToggle(String),
    /// Visite guidée.
    Tour(super::tour::TourMsg),
    Close,
    /// Fin d'un glissé de curseur : on enregistre.
    Commit,
    /// La valeur d'un curseur devient un champ, qui prend le focus (HIG SEL-08).
    SliderEdit(Box<SliderEdit>),
    /// Saisie en cours dans le champ de la valeur d'un curseur.
    SliderDraft(String),
    /// Entrée dans le champ : la valeur lue est posée, puis enregistrée.
    SliderSubmit,
    /// Le champ de la valeur se referme sans rien changer (Échap, autre page).
    SliderClose,
    /// Message d'un paquet de réglages.
    Plugin(PluginMsg),
    /// Le clavier vient au volet : son premier contrôle prend le focus (HIG ENT-02).
    FocusFirst,
    /// Recherche d'un réglage (texte tapé).
    Search(String),
    /// Résultat choisi : paquet et onglet.
    SearchOpen(Tab, Option<&'static str>),
    /// Retour à la liste des résultats.
    SearchBack,
    SearchClear,
    /// « Par défaut… » d'une page : demande, puis confirmation (MLG-202).
    Defaults(Tab),
    DefaultsCancel,
    DefaultsDone(Tab, Result<String, String>),
}

pub struct Panel {
    /// Surface du bureau de l'écran visé (contexte : écran, workspace, aperçu).
    pub win: Id,
    /// Surface propre du tiroir, au-dessus des fenêtres.
    pub surface: Id,
    pub tab: Tab,
    // État de chaque paquet intégré.
    pub desktop: plugins::desktop::State,
    pub keys: plugins::keys::State,
    pub icons: plugins::icons::State,
    pub wallpaper: plugins::wallpaper::State,
    pub widgets: plugins::widgets::State,
    pub screens: plugins::screens::State,
    pub workspaces: plugins::workspaces::State,
    /// Étape de la visite guidée en cours.
    pub tour: Option<usize>,
    pub layouts: plugins::layouts::State,
    pub animations: plugins::animations::State,
    pub sound: plugins::sound::State,
    pub energy: plugins::energy::State,
    pub panorama_modules: plugins::panorama::modules::State,
    pub performance: plugins::performance::State,
    pub network: plugins::network::State,
    pub bluetooth: plugins::bluetooth::State,
    pub datetime: plugins::datetime::State,
    pub notifications: plugins::notifications::State,
    pub input: plugins::input::State,
    pub autostart: plugins::autostart::State,
    pub hyprconfig: plugins::hyprconfig::State,
    /// Recherche d'un réglage ; `search_list` : liste des résultats affichée
    /// (sinon la page choisie, correspondances surlignées).
    pub search: String,
    pub search_list: bool,
    pub colors: plugins::colors::State,
    pub login: plugins::login::State,
    /// Empreintes digitales (Verrouillage).
    pub fingerprints: plugins::lock::State,
    pub passwords: plugins::passwords::State,
    pub apps: plugins::apps::State,
    pub plugins: plugins::plugins::State,
    pub assistant: plugins::assistant::State,
    pub dock: plugins::dock::State,
    pub launcher: plugins::launcher::State,
    pub index: plugins::index::State,
    pub share: plugins::share::State,
    pub bar: plugins::bar::State,
    pub theme: plugins::theme::State,
    pub apptheme: plugins::apptheme::State,
    pub about: plugins::about::State,
    /// Page dont la remise par défaut attend confirmation.
    pub defaults_ask: Option<Tab>,
    /// Résultat de la dernière remise par défaut d'une page.
    pub defaults_done: Option<(Tab, Result<String, String>)>,
    /// Valeur d'un curseur en cours de saisie (HIG SEL-08) : un seul à la fois,
    /// le champ se referme en changeant de page.
    pub slider: Option<SliderEdit>,
    /// Le volet tient le clavier : vrai dès son apparition (Hyprland le donne
    /// à une couche « à la demande » qui paraît), faux dès qu'une fenêtre le
    /// reprend (HIG ENT-02).
    pub keyboard: bool,
}

/// Identifiant du volet aux opérations de focus : elles y restent bornées,
/// sans aller aux autres surfaces (HIG ENT-02).
pub(super) const PANEL_ID: &str = "hyprdesk-panel";
/// La navigation des pages et le contenu du tiroir, défilés à part.
const NAV_SCROLL_ID: &str = "hyprdesk-panel-nav";
const CONTENT_SCROLL_ID: &str = "hyprdesk-panel-content";
/// Les groupes du parcours au clavier du tiroir, dans l'ordre de lecture :
/// Ctrl+Tab passe au premier contrôle du suivant (HIG ENT-01).
const PANEL_GROUPS: [&str; 2] = ["hyprdesk-panel-group-nav", "hyprdesk-panel-group-content"];

/// Curseur des Réglages dont la valeur se saisit au clavier (HIG SEL-08) :
/// ce qu'il faut pour poser la valeur tapée comme un glissé puis son
/// relâchement. Un par site d'[`App::slider`].
#[derive(Clone)]
pub struct SliderSpec {
    /// Clé du curseur : le champ ouvert est celui-là seul. Le plus souvent un
    /// littéral ; le nom d'une option pour les curseurs d'options de design.
    pub(in crate::ui) key: Arc<str>,
    /// Domaine du curseur (bornes) et pas de son incrément.
    pub(in crate::ui) range: (f64, f64),
    pub(in crate::ui) step: f64,
    /// Pose la valeur, comme pendant un glissé. Une fermeture : le site dont
    /// le message emporte une donnée du rendu (une forme, une étape, une
    /// clé d'écran) la garde ici, comme son glissé la portait déjà.
    pub(in crate::ui) pose: Arc<dyn Fn(f64) -> Msg + Send + Sync>,
    /// Enregistre, comme au relâchement. Sans la valeur : posée juste avant,
    /// elle ne doit pas être écrasée par celle d'avant la saisie.
    pub(in crate::ui) save: Msg,
}

impl SliderSpec {
    pub(in crate::ui) fn new(
        key: impl Into<Arc<str>>,
        range: (f64, f64),
        step: f64,
        pose: impl Fn(f64) -> Msg + Send + Sync + 'static,
        save: Msg,
    ) -> Self {
        SliderSpec {
            key: key.into(),
            range,
            step,
            pose: Arc::new(pose),
            save,
        }
    }
}

/// La fermeture ne se montre pas : clé, domaine, pas et enregistrement.
/// Écrit sans `debug_struct` : ses `.field("…")` sont pris pour des libellés
/// à traduire par le relevé du catalogue.
impl std::fmt::Debug for SliderSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "SliderSpec({:?}, {:?}, {:?}, {:?})",
            self.key, self.range, self.step, self.save
        )
    }
}

/// Présentation d'un curseur de réglage (HIG SEL-08) : celle que sa rangée
/// avait avant de passer au composant [`App::slider`], pour qu'au repos rien
/// ne bouge.
#[derive(Debug, Clone, Copy)]
pub struct SliderLook {
    /// Taille du texte de la valeur.
    pub(in crate::ui) size: f32,
    /// Largeur du curseur.
    pub(in crate::ui) width: Length,
    /// Largeur de la valeur au repos : `Shrink`, ou fixe quand le site la
    /// fixait.
    pub(in crate::ui) value_width: Length,
    /// Alignement du curseur et de sa valeur l'un à l'autre.
    pub(in crate::ui) align_y: Alignment,
    /// Écart du curseur à sa valeur : 12 px, sauf le site qui en avait un
    /// autre.
    pub(in crate::ui) spacing: u16,
    /// Encre de la valeur quand le site la donnait : une valeur secondaire en
    /// a une propre. Sans elle, l'encre du texte.
    pub(in crate::ui) value_color: Option<Color>,
    /// Plafond de largeur (en px) d'un curseur souple.
    pub(in crate::ui) cap: Option<u16>,
}

impl SliderLook {
    /// Curseur à largeur fixe (en px), la valeur à sa juste largeur : la
    /// rangée se resserre sur les deux.
    pub(in crate::ui) const fn fixed(size: f32, width: u16, align_y: Alignment) -> Self {
        SliderLook {
            size,
            width: Length::Fixed(width as f32),
            value_width: Length::Shrink,
            align_y,
            spacing: 12,
            value_color: None,
            cap: None,
        }
    }

    /// Curseur souple, qui remplit la place que lui laisse la valeur ; celle-ci
    /// garde la largeur fixe (en px) du site.
    pub(in crate::ui) const fn fill(size: f32, value_width: u16, align_y: Alignment) -> Self {
        SliderLook {
            size,
            width: Length::Fill,
            value_width: Length::Fixed(value_width as f32),
            align_y,
            spacing: 12,
            value_color: None,
            cap: None,
        }
    }

    /// Curseur souple plafonné : il remplit la place que lui laisse la valeur,
    /// jusqu'à `cap` px — en colonne étroite, il cède sa place plutôt que de
    /// pousser la valeur. Celle-ci garde sa juste largeur.
    pub(in crate::ui) const fn capped(size: f32, cap: u16, align_y: Alignment) -> Self {
        SliderLook {
            size,
            width: Length::Fill,
            value_width: Length::Shrink,
            align_y,
            spacing: 12,
            value_color: None,
            cap: Some(cap),
        }
    }

    /// L'écart que le site mettait déjà entre le curseur et sa valeur.
    pub(in crate::ui) const fn spacing(mut self, spacing: u16) -> Self {
        self.spacing = spacing;
        self
    }

    /// L'encre que le site donnait à sa valeur.
    pub(in crate::ui) const fn ink(mut self, color: Color) -> Self {
        self.value_color = Some(color);
        self
    }

    /// La largeur exacte que le site donnait à sa valeur, quand elle suit
    /// la croissance du texte.
    pub(in crate::ui) fn value_width(mut self, width: impl Into<Length>) -> Self {
        self.value_width = width.into();
        self
    }
}

/// Champ de la valeur d'un curseur, ouvert (HIG SEL-08) : un seul à la fois,
/// refermé en changeant de page.
#[derive(Debug, Clone)]
pub struct SliderEdit {
    /// Le curseur saisi.
    pub(in crate::ui) spec: SliderSpec,
    /// Saisie en cours.
    draft: String,
}

impl SliderEdit {
    /// Champ ouvert sur la valeur actuelle du curseur : le nombre seul, ramené
    /// au pas du curseur — une valeur calculée en `f32` (un volume en pour
    /// cent) s'ouvre sur « 30 », non sur « 30.000002 ».
    pub(in crate::ui) fn new(spec: SliderSpec, value: f64) -> Self {
        let draft = nombre(au_pas(value, spec.range, spec.step));
        SliderEdit { spec, draft }
    }
}

/// Valeur tapée pour un curseur (HIG SEL-08) : point ou virgule décimaux,
/// l'unité qui suit le nombre est ignorée (`72 px`). La valeur est bornée au
/// domaine, ramenée au pas. Texte illisible : `None`.
fn slider_value(saisie: &str, range: (f64, f64), step: f64) -> Option<f64> {
    // Le nombre en tête de la saisie : signe, chiffres, décimales.
    let mut nombre = String::new();
    let mut digits = 0;
    for (i, c) in saisie.trim().replace(',', ".").char_indices() {
        match c {
            '0'..='9' => {
                digits += 1;
                nombre.push(c);
            }
            '+' | '-' if i == 0 => nombre.push(c),
            '.' if !nombre.contains('.') => nombre.push(c),
            _ => break,
        }
    }
    if digits == 0 {
        return None;
    }
    let v: f64 = nombre.parse().ok()?;
    Some(au_pas(v, range, step))
}

/// La valeur bornée au domaine et ramenée au pas, comptée depuis le minimum.
/// Six décimales au plus : pas de bruit d'addition flottante.
fn au_pas(v: f64, range: (f64, f64), step: f64) -> f64 {
    let (min, max) = (range.0.min(range.1), range.0.max(range.1));
    let step = if step.is_finite() && step > 0.0 {
        step
    } else {
        1.0
    };
    let snapped = min + ((v.clamp(min, max) - min) / step).round() * step;
    (snapped * 1e6).round() / 1e6
}

/// Nombre seul, sans zéro de fin (`48`, `1.5`) : la saisie proposée à
/// l'ouverture du champ de la valeur d'un curseur (HIG SEL-08).
fn nombre(v: f64) -> String {
    let s = format!("{v:.6}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    match s {
        "" | "-0" => "0",
        _ => s,
    }
    .to_string()
}

/// Couleurs proposées (palette Dracula PRO), partagées par les paquets.
pub(in crate::ui) const SWATCHES: [Rgba; 8] = [
    Rgba(0x95, 0x80, 0xff, 0xff),
    Rgba(0xff, 0x80, 0xbf, 0xff),
    Rgba(0x80, 0xff, 0xea, 0xff),
    Rgba(0x8a, 0xff, 0x80, 0xff),
    Rgba(0xff, 0xff, 0x80, 0xff),
    Rgba(0xff, 0x95, 0x80, 0xff),
    Rgba(0x22, 0x21, 0x2c, 0xff),
    Rgba(0x00, 0x00, 0x00, 0xff),
];

/// Espace de noms de la surface du tiroir des réglages.
pub const SETTINGS_NAMESPACE: &str = "hyprdesk-settings";

/// Largeur du tiroir des réglages, marge comprise : 20 % de plus que
/// l'assistant, la navigation et ses onglets s'y déploient.
pub const DRAWER_W: f32 = 936.0;
/// Largeur du tiroir de l'assistant : le fil de discussion se lit mieux
/// sur une colonne étroite.
pub const CHAT_W: f32 = 780.0;
const DRAWER_GAP: f32 = 12.0;
/// Largeur de la navigation du tiroir large, texte au repos.
const NAV_W: f32 = 166.0;
/// Ce que la colonne des réglages garde au minimum : la navigation élargie
/// par le texte ne le prend pas (HIG TYP-08).
const DETAIL_MIN: f32 = 360.0;

/// La navigation et le contenu du tiroir défilent jusqu'à la commande qui a
/// le focus, si Tab l'a posé hors de vue (HIG ENT-01).
fn panel_reveal() -> Task<Msg> {
    super::focus::reveal(CONTENT_SCROLL_ID).chain(super::focus::reveal(NAV_SCROLL_ID))
}

/// Le nom d'un curseur du tiroir aux opérations : la saisie de sa valeur
/// refermée, le focus lui revient (HIG ENT-01).
fn slider_focus_id(key: &str) -> iced::widget::Id {
    iced::widget::Id::from(format!("hyprdesk-slider-{key}"))
}

/// Largeur de la navigation d'un tiroir de `largeur` dont le texte a grandi
/// de `croissance` : icônes seules en tiroir étroit ; sinon la largeur du
/// repos, suivie par le texte — un libellé de paquet et son compteur
/// tiennent côte à côte à toute échelle —, bornée par [`DETAIL_MIN`]
/// (HIG TYP-08).
fn nav_width(largeur: f32, croissance: f32) -> f32 {
    if largeur < 640.0 {
        return 46.0;
    }
    (NAV_W * croissance)
        .min((largeur - 2.0 * DRAWER_GAP - 48.0 - 24.0 - 14.0 - DETAIL_MIN).max(46.0))
}

/// Ligne des actions de l'en-tête à côté du titre : elle y tient tant que le
/// tiroir est large pour le texte (HIG TYP-08).
fn header_one_line(largeur: f32, croissance: f32) -> bool {
    largeur >= 880.0 * croissance
}

impl Panel {
    pub fn new(win: Id) -> Self {
        Panel {
            win,
            surface: Id::unique(),
            tab: plugins::desktop::INFO.id,
            desktop: Default::default(),
            keys: Default::default(),
            icons: Default::default(),
            wallpaper: Default::default(),
            widgets: Default::default(),
            screens: Default::default(),
            workspaces: Default::default(),
            tour: None,
            layouts: Default::default(),
            animations: Default::default(),
            sound: Default::default(),
            index: Default::default(),
            energy: Default::default(),
            panorama_modules: Default::default(),
            performance: Default::default(),
            network: Default::default(),
            bluetooth: Default::default(),
            datetime: Default::default(),
            notifications: Default::default(),
            input: Default::default(),
            autostart: Default::default(),
            hyprconfig: Default::default(),
            search: String::new(),
            search_list: false,
            colors: Default::default(),
            login: Default::default(),
            fingerprints: Default::default(),
            passwords: Default::default(),
            apps: Default::default(),
            plugins: Default::default(),
            assistant: Default::default(),
            dock: Default::default(),
            launcher: Default::default(),
            share: Default::default(),
            bar: Default::default(),
            theme: Default::default(),
            apptheme: Default::default(),
            about: Default::default(),
            defaults_ask: None,
            defaults_done: None,
            slider: None,
            keyboard: true,
        }
    }

    /// Sélectionne un widget et affiche son onglet.
    pub fn select_widget(&mut self, id: &str) {
        self.widgets.widget_sel = Some(id.to_string());
        self.tab = plugins::widgets::INFO.id;
    }
}

impl App {
    /// Affiche un paquet dans le tiroir ouvert ; c'est désormais le dernier
    /// consulté, celui de la réouverture (HIG PAT-09).
    fn panel_show(&mut self, tab: Tab) -> Task<Msg> {
        let Some(panel) = self.panel.as_mut() else {
            return Task::none();
        };
        self.panel_tab = tab;
        panel.tab = tab;
        // Autre page : la valeur d'un curseur en cours de saisie se referme.
        panel.slider = None;
        plugins::on_show(self, tab)
    }

    /// Referme la valeur d'un curseur en cours de saisie (HIG SEL-08) : la
    /// page qui la portait change ou disparaît.
    fn slider_close(&mut self) {
        if let Some(p) = self.panel.as_mut() {
            p.slider = None;
        }
    }

    /// Paquet où le tiroir rouvre : le dernier consulté, s'il est encore
    /// affiché (plugin actif, configuration avancée suffisante) ; sinon le
    /// Bureau.
    fn panel_restore(&self) -> Tab {
        if self.setting_visible(self.panel_tab) {
            self.panel_tab
        } else {
            plugins::desktop::INFO.id
        }
    }

    /// Paquet dont les réglages se montrent et se trouvent : plugin actif,
    /// configuration avancée suffisante. Le tiroir et le lanceur (`:r`) ont
    /// la même liste (HIG PAT-13).
    pub(in crate::ui) fn setting_visible(&self, id: &str) -> bool {
        self.cfg.plugin_on(id) && plugins::visible(id, self.advanced())
    }

    /// Ouvre le tiroir des réglages sur l'écran principal, dans sa propre
    /// surface au-dessus des fenêtres : le mode du bureau ne change pas.
    pub(super) fn open_panel(&mut self) -> Task<Msg> {
        // Sur l'écran du clic (sinon l'écran actif) ; déjà ouvert sur cet
        // écran : rien à refaire (un nouvel appel ne doit pas le dupliquer).
        let Some((win, _)) = self.target_surface() else {
            return Task::none();
        };
        if self.panel_open() && self.panel.as_ref().is_some_and(|p| p.win == win) {
            return Task::none();
        }
        // Un seul tiroir à la fois : l'assistant laisse la place aux réglages.
        let chat = self.close_chat();
        Task::batch([chat, self.open_panel_at(win)])
    }

    /// Bouton Réglages du lanceur : ouvre le tiroir sur l'écran du clic, le
    /// ferme s'il y est déjà affiché, l'y amène s'il est sur un autre écran.
    pub(super) fn toggle_panel(&mut self) -> Task<Msg> {
        let here = self.target_surface().map(|(id, _)| id);
        if self.panel_open() && self.panel.as_ref().map(|p| p.win) == here {
            self.close_panel()
        } else {
            self.open_panel()
        }
    }

    /// Tiroir affiché ou en train de s'ouvrir (pas en train de se fermer).
    pub(super) fn panel_open(&self) -> bool {
        self.panel.is_some() && self.panel_fade.is_none_or(|f| f.to >= 1.0)
    }

    /// Le clavier va au volet des Réglages (raccourci `keys.focus_settings`,
    /// HIG ENT-02) : fermé, il s'ouvre ; ouvert, il reste, et garde le clavier
    /// s'il le tient déjà — le focus reste où il est. Sinon il le reprend : un
    /// volet ouvert qui ne le tenait plus renaît, Hyprland ne le donnant à une
    /// couche « à la demande » qu'à son apparition (comme le dock,
    /// `launcher_grab`). Le focus se pose alors sur son premier contrôle. Le
    /// mode du bureau ne change pas.
    pub(super) fn focus_panel(&mut self) -> Task<Msg> {
        let state = |p: &Panel| (p.surface, p.keyboard);
        let before = self.panel.as_ref().map(state);
        let open = self.open_panel();
        let reborn = self.panel.as_mut().filter(|p| !p.keyboard).map(|p| {
            let monitor = self.surfaces.get(&p.win).cloned().unwrap_or_default();
            let old = std::mem::replace(&mut p.surface, Id::unique());
            p.keyboard = true;
            (old, p.surface, monitor)
        });
        let reborn = match reborn {
            Some((old, fresh, monitor)) => {
                Task::batch([iced::window::close(old), self.panel_surface(fresh, monitor)])
            }
            None => Task::none(),
        };
        // Ouvert et tenant le clavier : rien n'a changé, le focus ne bouge pas.
        // Au tour suivant : la surface nouvelle existe quand le focus se pose.
        let first = (self.panel.as_ref().map(state) != before)
            .then(|| Task::done(Msg::Panel(PanelMsg::FocusFirst)));
        Task::batch([open, reborn, first.unwrap_or_else(Task::none)])
    }

    /// Une commande du tiroir dans le parcours au clavier (HIG ENT-01) : Tab
    /// l'atteint, l'anneau la montre, Entrée ou Espace l'active. Jamais un
    /// champ de saisie, déjà focalisable. Accès clavier complet coupé
    /// (`general.keyboard_access`) : la commande est rendue telle quelle, Tab
    /// ne va plus qu'aux champs.
    /// Une commande qui peut être inactive (`on` faux : sans message) : hors
    /// du parcours tant qu'elle l'est, un arrêt de Tab sans effet n'aide pas.
    pub(in crate::ui) fn focusable_when<'a>(
        &self,
        control: impl Into<Element<'a, Msg>>,
        on: bool,
    ) -> Element<'a, Msg> {
        if on {
            self.focusable(control)
        } else {
            control.into()
        }
    }

    /// Une liste déroulante du tiroir dans le parcours au clavier (HIG
    /// ENT-01) : les flèches changent l'option, liste fermée.
    pub(in crate::ui) fn choosable<'a>(
        &self,
        control: impl Into<Element<'a, Msg>>,
    ) -> Element<'a, Msg> {
        super::focus::choose(control, self.ink().fg, self.cfg.general.keyboard_access)
    }

    pub(in crate::ui) fn focusable<'a>(
        &self,
        control: impl Into<Element<'a, Msg>>,
    ) -> Element<'a, Msg> {
        super::focus::click(control, self.ink().fg, self.cfg.general.keyboard_access)
    }

    /// `hyprdesk keyboard-access` (raccourci `keys.keyboard_access`, HIG
    /// ENT-01) : bascule l'accès clavier complet, l'enregistre et l'annonce à
    /// l'écran. L'annonce dit l'état appliqué : une clé fixée dans
    /// hyprdesk.lua l'emporte sur la bascule.
    pub(super) fn keyboard_access_toggle(&mut self) -> Task<Msg> {
        self.settings.keyboard_access = Some(!self.cfg.general.keyboard_access);
        let saved = self.commit();
        let note = if self.cfg.general.keyboard_access {
            "Accès clavier complet : activé"
        } else {
            "Accès clavier complet : désactivé"
        };
        let shown = self.show_osd_note("input-keyboard-symbolic", crate::i18n::tr(note).into());
        Task::batch([saved, shown])
    }

    /// Le champ de recherche du volet prend le focus, lui seul : les autres
    /// surfaces perdent le leur (HIG ENT-02). Viser le champ, et non le
    /// premier focalisable : la croix de l'en-tête, dans le parcours au
    /// clavier (HIG ENT-01), le précède dans l'arbre.
    pub(super) fn panel_focus_first(&self) -> Task<Msg> {
        iced::widget::operation::focus(SEARCH_ID)
    }

    /// La saisie d'une valeur refermée (Entrée, Échap) : le focus revient au
    /// curseur qui l'avait ouverte, accès clavier complet activé.
    fn slider_refocus(&self, key: &str) -> Task<Msg> {
        if self.cfg.general.keyboard_access {
            iced::widget::operation::focus(slider_focus_id(key))
        } else {
            Task::none()
        }
    }

    /// Le contrôle suivant (précédent) du volet prend le focus ; l'opération
    /// est bornée au volet (HIG ENT-02), elle ne va pas aux autres surfaces.
    pub(super) fn panel_focus_step(&self, previous: bool) -> Task<Msg> {
        super::focus::step(iced::widget::Id::new(PANEL_ID), previous).chain(panel_reveal())
    }

    /// Ctrl+Tab (Ctrl+Maj + Tab) : le focus va au premier contrôle du groupe
    /// suivant (précédent) du tiroir, la navigation ou le contenu (HIG ENT-01).
    pub(super) fn panel_focus_jump(&self, previous: bool) -> Task<Msg> {
        iced::advanced::widget::operate(iced::advanced::widget::operation::scope(
            iced::widget::Id::new(PANEL_ID),
            super::focus::jump(&PANEL_GROUPS, previous),
        ))
        .chain(panel_reveal())
    }

    /// Fond d'écran sous le tiroir, à sa place sur l'écran : ce que son
    /// verre réfracte. Les fenêtres n'y figurent pas, comme sous le dock.
    fn panel_backdrop(&self) -> Option<Element<'_, Msg>> {
        let win = self.panel.as_ref()?.win;
        let name = self.surfaces.get(&win)?;
        let m = self.monitors.iter().find(|m| &m.name == name)?;
        let origin = iced::Point::new(
            m.width as f32 - m.reserved.2 - self.panel_width(),
            m.reserved.1,
        );
        self.wall_backdrop(name, origin)
    }

    /// Crée la surface du tiroir sur `monitor`.
    pub(super) fn panel_surface(&self, id: Id, monitor: String) -> Task<Msg> {
        Task::done(Msg::NewLayerShell {
            settings: iced_layershell::reexport::NewLayerShellSettings {
                size: Some((self.panel_width() as u32, 0)),
                layer: iced_layershell::reexport::Layer::Top,
                anchor: iced_layershell::reexport::Anchor::Top
                    | iced_layershell::reexport::Anchor::Bottom
                    | iced_layershell::reexport::Anchor::Right,
                exclusive_zone: Some(0),
                keyboard_interactivity: iced_layershell::reexport::KeyboardInteractivity::OnDemand,
                output_option: iced_layershell::reexport::OutputOption::OutputName(monitor),
                namespace: Some(SETTINGS_NAMESPACE.into()),
                ..Default::default()
            },
            id,
        })
    }

    fn open_panel_at(&mut self, win: Id) -> Task<Msg> {
        // Tiroir en cours de fermeture : on rouvre le même.
        let reuse = self
            .panel
            .as_ref()
            .filter(|p| p.win == win)
            .map(|p| p.surface);
        if let Some(old) = self
            .panel
            .as_ref()
            .filter(|p| p.win != win)
            .map(|p| p.surface)
        {
            self.panel = None;
            self.panel_fade = None;
            return Task::batch([iced::window::close(old), self.open_panel_at(win)]);
        }
        let mut p = Panel::new(win);
        // Le tiroir rouvre sur le dernier paquet consulté (HIG PAT-09).
        p.tab = self.panel_restore();
        let tab = p.tab;
        let mut create = Task::none();
        p.surface = match reuse {
            Some(id) => id,
            None => {
                let id = Id::unique();
                let monitor = self.surfaces.get(&win).cloned().unwrap_or_default();
                // Pas de capture de l'écran sous le tiroir : figée à
                // l'ouverture, elle gardait le lanceur ou des fenêtres qui
                // n'y étaient plus. Le verre réfracte le fond d'écran.
                create = self.panel_surface(id, monitor);
                id
            }
        };
        let k = &self.cfg.general.keys;
        p.keys.keys = [
            k.toggle.clone(),
            k.settings.clone(),
            k.leave.clone(),
            k.launcher.clone(),
            k.hide.clone(),
            k.lock.clone(),
            k.search.clone(),
            k.switch.clone(),
            k.overview.clone(),
            k.search_desktop.clone(),
            k.assistant.clone(),
            k.focus_bar.clone(),
            k.focus_dock.clone(),
            k.focus_notifications.clone(),
            k.focus_settings.clone(),
            k.force_quit.clone(),
            k.keyboard_access.clone(),
        ];
        self.panel = Some(p);
        let from = self
            .panel_fade
            .map(|f| f.value(std::time::Instant::now()))
            .unwrap_or(0.0);
        self.panel_fade = Some(super::Fade {
            from,
            to: 1.0,
            start: std::time::Instant::now(),
            dur: self.motion_duration(),
        });
        self.selected = None;
        let themes = Task::perform(
            async {
                tokio::task::spawn_blocking(launch::icon_themes)
                    .await
                    .unwrap_or_default()
            },
            |t| plugins::icons::wrap(plugins::icons::Msg::Themes(t)),
        );
        Task::batch([
            create,
            self.sync_window_gaps(),
            themes,
            // Valeurs de hyprland.lua : flou coupé → tiroir opaque.
            plugins::animations::load_current(),
            // Options de Hyprland : valeurs en vigueur, lignes de hyprland.lua.
            self.load_hypr_current(),
            self.load_hypr_source(),
            // Paquet rouvert autre que le Bureau : ses données se chargent,
            // comme à chaque arrivée sur un onglet.
            if tab == plugins::desktop::INFO.id {
                Task::none()
            } else {
                plugins::on_show(self, tab)
            },
            Task::perform(
                async {
                    tokio::task::spawn_blocking(launch::font_families)
                        .await
                        .unwrap_or_default()
                },
                |f| plugins::desktop::wrap(plugins::desktop::Msg::Fonts(f)),
            ),
        ])
    }

    /// Applique les réglages à la config effective, tout de suite, sans
    /// repasser par Lua : le bureau à côté du tiroir change en direct.
    pub(in crate::ui) fn apply(&mut self) -> Task<Msg> {
        let next = self.settings.apply_to(&self.cfg);
        self.install_config(Arc::new(next))
    }

    /// Applique puis enregistre settings.lua (sans rechargement : la mémoire
    /// est déjà à jour ; un rechargement déclenché par la surveillance du
    /// fichier retombe sur la même config et ne change rien).
    /// Petit bouton au logo IA : demande à LYRA de régler un problème
    /// (`origin` : titre de la conversation). Rien sans assistant.
    pub(in crate::ui) fn lyra_button(&self, origin: &str, prompt: String) -> Element<'_, Msg> {
        if !self.cfg.plugin_on("assistant") || !crate::assistant::lyra_installed() {
            return iced::widget::Space::new().into();
        }
        let fg = super::color(self.cfg.general.theme.fg, 1.0);
        let accent = super::color(self.cfg.general.theme.accent, 1.0);
        let glyph: Element<'_, Msg> = super::icons::get("ai-assistant")
            .map(|h| {
                iced::widget::svg(h)
                    .width(16)
                    .height(16)
                    .style(move |_, _| iced::widget::svg::Style { color: Some(fg) })
                    .into()
            })
            .unwrap_or_else(|| {
                iced::widget::text("IA")
                    .size(crate::ui::tokens::ts(12.0))
                    .into()
            });
        // Dans le parcours au clavier (HIG ENT-01) : le clic au centre tombe
        // sur le bouton.
        self.focusable(iced::widget::tooltip(
            iced::widget::button(glyph)
                .padding([4, 8])
                .style(move |_, st| iced::widget::button::Style {
                    background: Some(
                        iced::Color {
                            a: if matches!(st, iced::widget::button::Status::Hovered) {
                                0.3
                            } else {
                                0.15
                            },
                            ..accent
                        }
                        .into(),
                    ),
                    border: iced::Border::default().rounded(8),
                    text_color: fg,
                    ..Default::default()
                })
                .on_press(Msg::AskLyra(origin.to_string(), prompt)),
            crate::i18n::text(crate::i18n::tr("Demander à LYRA de régler le problème"))
                .font(self.font),
            iced::widget::tooltip::Position::Left,
        ))
    }

    pub(in crate::ui) fn commit(&mut self) -> Task<Msg> {
        let applied = self.apply();
        Task::batch([applied, self.save()])
    }

    /// Enregistre puis recharge : activer ou retirer un widget demande de
    /// relancer sa configuration (fonctions Lua des plugins).
    pub(in crate::ui) fn commit_reload(&self) -> Task<Msg> {
        self.save().chain(Task::done(Msg::ConfigChanged))
    }

    pub(in crate::ui) fn save(&self) -> Task<Msg> {
        let (s, path) = (self.settings.clone(), self.config_path.clone());
        let revision = crate::storage::revision();
        self.settings_revision
            .store(revision, std::sync::atomic::Ordering::Release);
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    s.save_versioned(&path, revision).map_err(|e| e.to_string())
                })
                .await
                .map_err(|e| e.to_string())?
            },
            |r| match r {
                Ok(_) => Msg::Noop,
                Err(e) => {
                    tracing::error!("{}", crate::tr!("enregistrement des réglages : {e}", e = e));
                    crate::hypr::notify(
                        4,
                        6000,
                        &crate::tr!("hyprdesk : réglages non enregistrés — {e}", e = e),
                    );
                    Msg::Noop
                }
            },
        )
    }

    pub(in crate::ui) fn panel_ctx(&self) -> Option<(String, String)> {
        let p = self.panel.as_ref()?;
        let m = self.monitor(p.win)?;
        Some((m.name.clone(), m.workspace.clone()))
    }

    pub(super) fn panel_update(&mut self, msg: PanelMsg) -> Task<Msg> {
        match msg {
            PanelMsg::Tab(t) => self.panel_show(t),
            PanelMsg::SubTab(pkg, tab) => {
                self.subtabs.insert(pkg, tab);
                self.slider_close();
                Task::none()
            }
            PanelMsg::Search(q) => {
                if let Some(p) = self.panel.as_mut() {
                    p.search_list = !q.trim().is_empty();
                    p.search = q;
                }
                // La page cède la place aux résultats dès que la requête est écrite.
                self.slider_close();
                Task::none()
            }
            PanelMsg::SearchOpen(pkg, tab) => {
                if let Some(t) = tab {
                    self.subtabs.insert(pkg, t);
                }
                if let Some(p) = self.panel.as_mut() {
                    p.search_list = false;
                }
                self.panel_show(pkg)
            }
            PanelMsg::SearchBack => {
                if let Some(p) = self.panel.as_mut() {
                    p.search_list = !p.search.trim().is_empty();
                }
                self.slider_close();
                Task::none()
            }
            PanelMsg::SearchClear => {
                if let Some(p) = self.panel.as_mut() {
                    p.search.clear();
                    p.search_list = false;
                }
                // Le champ reprend le focus : une nouvelle recherche s'écrit.
                iced::widget::operation::focus(SEARCH_ID)
            }
            PanelMsg::FocusFirst => self.panel_focus_first(),
            PanelMsg::Tour(m) => self.tour_update(m),
            PanelMsg::Advanced(v) => {
                self.settings.advanced_settings = Some(v);
                // Paquet affiché devenu avancé : retour au Bureau, qui reste
                // celui de la réouverture.
                if !v
                    && let Some(p) = self.panel.as_mut()
                    && !plugins::visible(p.tab, false)
                {
                    self.panel_tab = plugins::desktop::INFO.id;
                    p.tab = plugins::desktop::INFO.id;
                }
                self.slider_close();
                self.commit()
            }
            PanelMsg::AdvToggle(key) => {
                // Dépliée : on la replie ; repliée : on la déplie. Le champ
                // d'un curseur qu'elle porte se referme avec elle.
                if !self.adv_open.remove(&key) {
                    self.adv_open.insert(key);
                }
                self.slider_close();
                Task::none()
            }
            PanelMsg::Close => self.close_panel(),
            PanelMsg::Commit => self.commit(),
            PanelMsg::SliderEdit(edit) => {
                // Un seul champ ouvert à la fois.
                self.slider_close();
                let Some(p) = self.panel.as_mut() else {
                    return Task::none();
                };
                p.slider = Some(*edit);
                // Le champ prend le focus, son nombre sélectionné : la
                // saisie le remplace.
                use iced::widget::operation;
                operation::focus(SLIDER_ID).chain(operation::select_all(SLIDER_ID))
            }
            PanelMsg::SliderDraft(s) => {
                if let Some(e) = self.panel.as_mut().and_then(|p| p.slider.as_mut()) {
                    e.draft = s;
                }
                Task::none()
            }
            PanelMsg::SliderSubmit => {
                let Some(e) = self.panel.as_ref().and_then(|p| p.slider.clone()) else {
                    return Task::none();
                };
                // Illisible : rien, le champ garde la saisie.
                let Some(v) = slider_value(&e.draft, e.spec.range, e.spec.step) else {
                    return Task::none();
                };
                self.slider_close();
                // Comme un glissé puis son relâchement : la valeur est posée,
                // puis enregistrée.
                let back = self.slider_refocus(&e.spec.key);
                let save = e.spec.save;
                self.update_inner((e.spec.pose)(v))
                    .chain(Task::done(save))
                    .chain(back)
            }
            PanelMsg::SliderClose => {
                let key = self
                    .panel
                    .as_ref()
                    .and_then(|p| p.slider.as_ref())
                    .map(|e| e.spec.key.clone());
                self.slider_close();
                key.map_or_else(Task::none, |k| self.slider_refocus(&k))
            }
            // Un geste dans le paquet referme la valeur en cours de saisie
            // (HIG SEL-08) : la pose garde la donnée du rendu à l'ouverture
            // du champ (une forme, une étape), qu'un autre réglage bougé
            // depuis pourrait rendre périmée.
            PanelMsg::Plugin(m) => {
                self.slider_close();
                plugins::update(self, m)
            }
            PanelMsg::Defaults(tab) => {
                let Some(p) = self.panel.as_mut() else {
                    return Task::none();
                };
                if p.defaults_ask != Some(tab) {
                    p.defaults_ask = Some(tab);
                    p.defaults_done = None;
                    return Task::none();
                }
                p.defaults_ask = None;
                Task::perform(
                    defaults_job(self.config_path.clone(), Some(tab)),
                    move |r| Msg::Panel(PanelMsg::DefaultsDone(tab, r)),
                )
            }
            PanelMsg::DefaultsCancel => {
                if let Some(p) = self.panel.as_mut() {
                    p.defaults_ask = None;
                }
                Task::none()
            }
            PanelMsg::DefaultsDone(tab, r) => {
                let ok = r.is_ok();
                if let Some(p) = self.panel.as_mut() {
                    p.defaults_done = Some((tab, r));
                }
                if ok {
                    self.after_defaults(tab)
                } else {
                    // Remise faite en partie : ce qui est écrit est relu.
                    self.load_config()
                }
            }
        }
    }

    /// Bouton « Par défaut… » d'une page qui a une configuration par défaut.
    fn defaults_button(&self, p: &Panel) -> Element<'_, Msg> {
        if !crate::defaults::holds_something(p.tab) || p.defaults_ask == Some(p.tab) {
            return Space::new().into();
        }
        tooltip(
            button(
                text("Par défaut…")
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0)),
            )
            .padding([3, 10])
            .style(button::text)
            .on_press(Msg::Panel(PanelMsg::Defaults(p.tab))),
            text("Remettre cette page à sa configuration par défaut").font(self.font),
            iced::widget::tooltip::Position::Left,
        )
        .into()
    }

    /// Sous l'en-tête : confirmation de la remise (ce qu'elle touche), ou
    /// son résultat ; rien sinon.
    fn defaults_status(&self, p: &Panel) -> Option<Element<'_, Msg>> {
        let tab = p.tab;
        if p.defaults_ask == Some(tab) {
            let mut what = vec![crate::i18n::tr("ses réglages").to_string()];
            what.extend(
                crate::defaults::held(tab)
                    .iter()
                    .map(|h| crate::i18n::tr(h.label()).to_string()),
            );
            return Some(
                column![
                    text(crate::tr!(
                        "Remettre « {l} » par défaut : {w}. hyprdesk.lua n'est pas touché ; une copie est gardée.",
                        l = crate::i18n::tr(plugins::find(tab).map_or("", |i| i.label)),
                        w = what.join(", ")
                    ))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(self.ink().dim),
                    row![
                        button(text("Confirmer").font(self.font).size(crate::ui::tokens::ts(12.0)))
                            .padding([4, 12])
                            .style(button::danger)
                            .on_press(Msg::Panel(PanelMsg::Defaults(tab))),
                        button(text("Annuler").font(self.font).size(crate::ui::tokens::ts(12.0)))
                            .padding([4, 12])
                            .style(button::secondary)
                            .on_press(Msg::Panel(PanelMsg::DefaultsCancel)),
                    ]
                    // 12 px entre les deux commandes (HIG ACC-10).
                    .spacing(12),
                ]
                .spacing(8)
                .into(),
            );
        }
        match &p.defaults_done {
            Some((t, r)) if *t == tab => {
                let (msg, c) = match r {
                    Ok(m) => (m.clone(), self.ink().dim),
                    Err(e) => (e.clone(), crate::ui::tone::warning()),
                };
                Some(
                    // Chemin de la copie, erreur : à sélectionner et à copier (HIG LAY-05).
                    super::selectable::selectable(msg)
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                        .color(c)
                        .into(),
                )
            }
            _ => None,
        }
    }

    // -- Vue -----------------------------------------------------------------------

    pub(super) fn panel_width(&self) -> f32 {
        self.drawer_width(self.panel.as_ref().map(|p| p.win), DRAWER_W)
    }

    /// Largeur utile de la colonne des réglages (tiroir moins la navigation,
    /// les marges et la barre de défilement) : les grilles en tirent leur
    /// nombre de colonnes plutôt que de déborder.
    pub(in crate::ui) fn detail_width(&self) -> f32 {
        let w = self.panel_width();
        let nav = self.nav_width();
        (w - 2.0 * DRAWER_GAP - 48.0 - nav - 24.0 - 14.0).max(0.0)
    }

    /// Largeur de la navigation du tiroir, suivie par le texte agrandi
    /// ([`nav_width`]).
    fn nav_width(&self) -> f32 {
        nav_width(self.panel_width(), super::tokens::text_growth())
    }

    /// Largeur du tiroir de l'assistant.
    pub(super) fn chat_width(&self) -> f32 {
        self.drawer_width(
            self.chat
                .as_ref()
                .filter(|c| c.surface.is_some())
                .map(|c| c.win),
            CHAT_W,
        )
    }

    /// Largeur d'un tiroir affiché : celle de son écran s'il est plus étroit
    /// que le tiroir, sinon la largeur du tiroir.
    fn drawer_width(&self, win: Option<Id>, w: f32) -> f32 {
        win.map(|win| {
            let width = self.size_of(win).width;
            if width < 1200.0 { width } else { w }
        })
        .unwrap_or(w)
    }

    /// Onglets du paquet affiché, en pastilles (rien s'il n'en a pas).
    fn subtab_bar(&self, pkg: Tab) -> Element<'_, Msg> {
        // Tiroir large : les onglets sont dans la navigation, sous le paquet.
        if self.panel_width() >= 640.0 {
            return Space::new().into();
        }
        let tabs = plugins::visible_tabs(pkg, self.advanced());
        if tabs.is_empty() {
            return Space::new().into();
        }
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let on = self.subtab(pkg);
        row(tabs.into_iter().map(|(id, label)| {
            let active = id == on;
            let mut content = row![
                text(label)
                    .font(if active { self.title_font() } else { self.font })
                    .size(crate::ui::tokens::ts(13.0))
                    .color(if active { fg } else { self.ink().dim })
            ]
            .spacing(6)
            .align_y(Alignment::Center);
            // Onglet de la configuration avancée : point cyan.
            if plugins::is_advanced(pkg, Some(id)) {
                content = content.push(plugins::adv_icon(13.0));
            }
            self.focusable(
                button(content)
                    .padding([6, 14])
                    .on_press(Msg::Panel(PanelMsg::SubTab(pkg, id)))
                    .style(move |_, status| {
                        let hover = matches!(status, button::Status::Hovered);
                        button::Style {
                            background: Some(
                                if active {
                                    Color { a: 0.28, ..accent }
                                } else if hover {
                                    Color { a: 0.08, ..fg }
                                } else {
                                    Color::TRANSPARENT
                                }
                                .into(),
                            ),
                            text_color: fg,
                            border: Border::default().rounded(999).width(1).color(if active {
                                Color { a: 0.7, ..accent }
                            } else {
                                Color { a: 0.14, ..fg }
                            }),
                            ..Default::default()
                        }
                    }),
            )
        }))
        .spacing(8)
        .wrap()
        .into()
    }

    pub(super) fn view_panel(&self) -> Element<'_, Msg> {
        let Some(p) = &self.panel else {
            return Space::new().into();
        };
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let wide = self.panel_width() >= 640.0;
        let adv = self.advanced();
        let ink = self.ink();
        let dim = ink.dim;
        // Entrée d'un paquet.
        // Recherche en cours : résultats, et leur nombre par paquet.
        let query = p.search.trim().to_string();
        let found = super::settings_search::search(&query, |id| self.setting_visible(id));
        let count = |id: &str| found.iter().filter(|e| e.page == id).count();
        let searching = !query.is_empty();
        let entry = |label: &'static str, icon: &'static str, tab: Tab| -> Element<'_, Msg> {
            let active = p.tab == tab;
            let n = count(tab);
            let glyph: Element<'_, Msg> = super::icons::get(icon)
                .map(|handle| {
                    iced::widget::svg(handle)
                        .width(18)
                        .height(18)
                        .style(move |_, _| iced::widget::svg::Style {
                            color: Some(if active { accent } else { fg }),
                        })
                        .into()
                })
                .unwrap_or_else(|| Space::new().into());
            let mut content = row![glyph].spacing(10).align_y(Alignment::Center);
            // Tiroir étroit (icônes seules) : la fiole suit l'icône d'un
            // paquet avancé, ou d'un paquet qui contient de l'avancé.
            if !wide && (plugins::is_advanced(tab, None) || (adv && plugins::adv_count(tab) > 0)) {
                content = content.push(plugins::adv_icon(12.0));
            }
            if wide {
                content = content.push(
                    text(label)
                        .font(self.font)
                        .size(crate::ui::tokens::ts(13.0))
                        .font(self.ui_font(iced::font::Weight::Medium))
                        .width(Length::Fill),
                );
                // Paquet de la configuration avancée : la fiole ; paquet qui
                // en contient (configuration avancée affichée) : la fiole et
                // le nombre de réglages avancés.
                if plugins::is_advanced(tab, None) {
                    content = content.push(plugins::adv_icon(14.0));
                } else if adv {
                    let n = plugins::adv_count(tab);
                    if n > 0 {
                        content = content.push(
                            row![
                                plugins::adv_icon(13.0),
                                text(n.to_string())
                                    .size(crate::ui::tokens::ts(11.0))
                                    .color(plugins::ADV_COLOR)
                            ]
                            .spacing(3)
                            .align_y(Alignment::Center),
                        );
                    }
                }
                if searching && n > 0 {
                    content = content.push(
                        container(
                            iced::widget::text(n.to_string())
                                .font(self.title_font())
                                .size(crate::ui::tokens::ts(11.0))
                                .color(ink.fg),
                        )
                        .padding([1, 7])
                        .style(move |_| container::Style {
                            background: Some(Color { a: 0.45, ..accent }.into()),
                            border: Border::default().rounded(999),
                            ..Default::default()
                        }),
                    );
                }
            }
            let entry = button(content)
                .width(Length::Fill)
                .padding([9, 12])
                .on_press(Msg::Panel(PanelMsg::Tab(tab)))
                .style(move |_, status| button::Style {
                    background: if active {
                        Some(Color { a: 0.16, ..accent }.into())
                    } else if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                        Some(Color { a: 0.07, ..fg }.into())
                    } else {
                        None
                    },
                    // Choisi : icône en accent, texte plein (l'accent sur son
                    // propre fond teinté se lit mal).
                    text_color: if active { ink.fg } else { fg },
                    border: Border::default().rounded(11),
                    ..Default::default()
                });
            // La navigation des pages, dans le parcours au clavier (HIG ENT-01).
            self.focusable(entry)
        };
        // Navigation par groupes ; le paquet choisi déploie ses onglets en
        // sous-entrées (tiroir large). Paquets coupés : absents (rallumables
        // dans Plugins) ; paquets avancés : seulement en configuration avancée.
        // Recherche : la navigation ne garde que les paquets qui ont une
        // correspondance, avec leur compte (HIG NAV-07).
        let mut nav = column![].spacing(2);
        let mut first = true;
        for g in plugins::GROUPS {
            let items: Vec<&'static plugins::PluginInfo> = g
                .ids
                .iter()
                .filter_map(|id| plugins::BUILTIN.iter().find(|i| i.id == *id))
                .filter(|i| self.setting_visible(i.id))
                .filter(|i| !searching || count(i.id) > 0)
                .collect();
            if items.is_empty() {
                continue;
            }
            if wide {
                nav = nav.push(
                    container(
                        text(crate::i18n::tr(g.label).to_uppercase())
                            .font(self.title_font())
                            .size(crate::ui::tokens::ts(12.0))
                            .color(dim),
                    )
                    .padding(iced::Padding {
                        top: if first { 0.0 } else { 12.0 },
                        bottom: 4.0,
                        left: 12.0,
                        right: 0.0,
                    }),
                );
            } else if !first {
                nav = nav.push(container(rule::horizontal(1)).padding([6, 8]));
            }
            first = false;
            for i in items {
                nav = nav.push(entry(i.label, i.icon, i.id));
                if wide && p.tab == i.id {
                    let on = self.subtab(i.id);
                    for (t, l) in plugins::visible_tabs(i.id, adv) {
                        let active = t == on;
                        let mut tab_label = row![
                            text(crate::i18n::tr(l))
                                .font(if active { self.title_font() } else { self.font })
                                .size(crate::ui::tokens::ts(12.0))
                                .width(Length::Fill)
                        ]
                        .align_y(Alignment::Center);
                        if plugins::is_advanced(i.id, Some(t)) {
                            tab_label = tab_label.push(plugins::adv_icon(13.0));
                        }
                        nav = nav.push(
                            self.focusable(
                                button(tab_label)
                                    .width(Length::Fill)
                                    .padding(iced::Padding {
                                        top: 5.0,
                                        bottom: 5.0,
                                        left: 40.0,
                                        right: 8.0,
                                    })
                                    .on_press(Msg::Panel(PanelMsg::SubTab(i.id, t)))
                                    .style(move |_, status| button::Style {
                                        // Onglet courant : le voile de sélection du paquet
                                        // courant, en permanence (HIG LAY-07).
                                        background: if active {
                                            Some(Color { a: 0.16, ..accent }.into())
                                        } else if matches!(
                                            status,
                                            button::Status::Hovered | button::Status::Pressed
                                        ) {
                                            Some(Color { a: 0.06, ..fg }.into())
                                        } else {
                                            None
                                        },
                                        // L'accent sur son propre fond teinté se lit mal :
                                        // texte plein, comme le paquet choisi.
                                        text_color: if active { ink.fg } else { ink.dim },
                                        border: Border::default().rounded(8),
                                        ..Default::default()
                                    }),
                            ),
                        );
                    }
                }
            }
        }
        let nav = scrollable(nav)
            .id(NAV_SCROLL_ID)
            .height(Length::Fill)
            .width(self.nav_width())
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new()
                    .width(4)
                    .scroller_width(4),
            ));
        let (mon, ws) = self.panel_ctx().unwrap_or_default();
        let close_icon = super::icons::get("window-close").unwrap();
        let close = tooltip(
            self.focusable(
                button(
                    iced::widget::svg(close_icon)
                        .width(16)
                        .height(16)
                        .style(move |_, _| iced::widget::svg::Style { color: Some(fg) }),
                )
                .padding(9)
                .on_press(Msg::Panel(PanelMsg::Close))
                .style(button::text),
            ),
            text("Fermer · Échap").font(self.font),
            iced::widget::tooltip::Position::Left,
        );
        // Tiroir étroit, ou texte agrandi : la ligne des actions ne tient plus
        // à côté du titre (la recherche passait sur « Réglages ») ; elle passe
        // dessous.
        let one_line = header_one_line(self.panel_width(), super::tokens::text_growth());
        // Bouton Clear dans le champ, dès que la requête n'est pas vide.
        let clear = !p.search.is_empty();
        let input = iced::widget::text_input(crate::i18n::tr("Rechercher un réglage…"), &p.search)
            .id(SEARCH_ID)
            .on_input(|q| Msg::Panel(PanelMsg::Search(q)))
            .on_submit(match found.first() {
                Some(e) => Msg::Panel(PanelMsg::SearchOpen(e.page, e.tab)),
                None => Msg::Noop,
            })
            // Le texte s'arrête avant le glyphe.
            .padding(iced::Padding {
                right: if clear { 30.0 } else { 10.0 },
                ..iced::Padding::from([6, 10])
            })
            .size(crate::ui::tokens::ts(13.0))
            .font(self.font)
            .width(if one_line {
                Length::Fixed(230.0)
            } else {
                Length::Fill
            });
        let mut layers: Vec<Element<'_, Msg>> = vec![input.into()];
        if clear {
            let glyph: Element<'_, Msg> = super::icons::get("window-close")
                .map(|h| {
                    iced::widget::svg(h)
                        .width(14)
                        .height(14)
                        .style(move |_, _| iced::widget::svg::Style { color: Some(dim) })
                        .into()
                })
                .unwrap_or_else(|| {
                    text("×")
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                        .color(dim)
                        .into()
                });
            layers.push(
                container(tooltip(
                    self.focusable(
                        button(glyph)
                            .padding(2)
                            .style(button::text)
                            .on_press(Msg::Panel(PanelMsg::SearchClear)),
                    ),
                    text(crate::i18n::tr("Effacer la recherche")).font(self.font),
                    // En-tête du tiroir : la bulle s'ouvre dessous.
                    iced::widget::tooltip::Position::Bottom,
                ))
                // La couche occupe tout le champ : sinon l'empilement pose le
                // bouton à gauche.
                .width(Length::Fill)
                .align_x(Alignment::End)
                .padding(iced::Padding {
                    right: 6.0,
                    ..iced::Padding::ZERO
                })
                .height(Length::Fill)
                .center_y(Length::Fill)
                .into(),
            );
        }
        let actions = row![
            iced::widget::Stack::with_children(layers),
            self.focusable(
                button(
                    text(crate::i18n::tr("Visite guidée"))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                )
                .padding([4, 10])
                .style(button::text)
                .on_press(Msg::Panel(PanelMsg::Tour(super::tour::TourMsg::Start)))
            ),
            self.focusable(
                button(
                    text(crate::i18n::tr("Assistant"))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                )
                .padding([4, 10])
                .style(button::text)
                .on_press(Msg::Control(crate::control::Command::Wizard))
            ),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        let titles = column![
            // Le titre nomme le paquet visible, comme une fenêtre macOS nomme
            // son volet (HIG PAT-16) ; « Réglages » reste en surligne.
            text("Réglages")
                .font(self.font)
                .size(crate::ui::tokens::ts(11.0))
                .color(self.ink().dim)
                .font(self.title_font()),
            // Titre de page : la seule graisse Bold du tiroir (HIG TYP-05).
            text(plugins::find(p.tab).map_or("", |i| i.label))
                .font(self.font)
                .size(crate::ui::tokens::ts(26.0))
                .font(self.page_title_font())
                .color(fg),
            // Aides, états et comptes : la couleur secondaire du thème,
            // garantie à 4,5:1 sur le fond du tiroir (HIG LAY-08).
            text(crate::tr!("{mon}  ·  Bureau {ws}", mon = mon, ws = ws))
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(dim),
        ]
        .spacing(5)
        .width(Length::Fill);
        // La croix tient seule la ligne du haut : sur la ligne des actions,
        // un libellé trop long (traduction, tiroir étroit) débordait dessus.
        let header: Element<'_, Msg> = if one_line {
            row![
                titles,
                column![close, actions].spacing(2).align_x(Alignment::End),
            ]
            .spacing(12)
            .align_y(Alignment::Start)
            .into()
        } else {
            column![
                row![titles, close].spacing(12).align_y(Alignment::Start),
                actions,
            ]
            .spacing(12)
            .into()
        };
        let content: Element<'_, Msg> = if searching && p.search_list {
            self.search_results(&query, &found)
        } else if searching {
            column![
                self.search_banner(&query, count(p.tab)),
                plugins::view(self, p, p.tab)
            ]
            .spacing(14)
            .into()
        } else {
            plugins::view(self, p, p.tab)
        };
        let detail = column![
            row![
                container(Space::new().width(5).height(5)).style(move |_| container::Style {
                    background: Some(accent.into()),
                    border: Border::default().rounded(5),
                    ..Default::default()
                }),
                text("Modifications enregistrées automatiquement")
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(dim),
                Space::new().width(Length::Fill),
                self.defaults_button(p),
            ]
            .spacing(7)
            .align_y(Alignment::Center),
            // Sur sa propre ligne dans le corps des Réglages (HIG SEL-03) :
            // un interrupteur n'a pas sa place dans l'en-tête, qui est la
            // barre d'outils du tiroir.
            self.focusable(
                toggler(adv)
                    .label(crate::i18n::tr("Configuration avancée"))
                    .text_size(12)
                    .font(self.font)
                    .on_toggle(|v| Msg::Panel(PanelMsg::Advanced(v)))
            ),
            self.tour_card(),
            self.defaults_status(p),
            self.subtab_bar(p.tab),
            // Même barre de 4 px que la navigation (HIG PRES-04).
            scrollable(container(content).padding(iced::Padding {
                right: 14.0,
                ..iced::Padding::ZERO
            }))
            .id(CONTENT_SCROLL_ID)
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new()
                    .width(4)
                    .scroller_width(4),
            ))
            .height(Length::Fill),
        ]
        .spacing(18)
        .width(Length::Fill);
        let card = container(
            column![
                header,
                rule::horizontal(1),
                row![
                    // Deux groupes, la navigation et le contenu : Ctrl+Tab
                    // passe de l'un à l'autre (HIG ENT-01).
                    super::focus::group(PANEL_GROUPS[0], nav),
                    super::focus::group(PANEL_GROUPS[1], detail)
                ]
                .spacing(24)
                .height(Length::Fill)
            ]
            .spacing(22),
        )
        .padding(24)
        .width(self.panel_width() - 2.0 * DRAWER_GAP)
        .height(Length::Fill);
        // Sans flou de Hyprland, la transparence rend le tiroir illisible.
        // La surface de lecture du thème (MLG-282) est fondue dans le
        // tiroir entier : un rectangle à part, sous le contenu seul, se
        // voyait dans le verre.
        let opacity = if self.hypr_blur_off() {
            1.0
        } else {
            let th = &self.cfg.general.theme;
            let own = th.shown(crate::config::Place::Settings).opacity;
            let reading = th.shown(crate::config::Place::Content).opacity;
            1.0 - (1.0 - own) * (1.0 - reading)
        };
        // Verre du thème (MLG-268) au rayon du tiroir ; vraie réfraction de
        // l'écran capturé à l'ouverture (MLG-275).
        let th = &self.cfg.general.theme;
        // L'ombre tient dans la marge du tiroir : plus large, la surface la
        // coupait net à son bord et un cadre se voyait autour du tiroir.
        let mut surface = super::design::panel_surface(&self.theme, opacity);
        surface.shadow.offset = iced::Vector::new(0.0, 2.0);
        surface.shadow.blur_radius = DRAWER_GAP - 2.0;
        let card = super::glass::card(
            card,
            crate::config::Place::Settings,
            24.0,
            color(th.fg, 1.0),
            color(
                super::tokens::tinted(th.bg, crate::config::Place::Settings),
                opacity,
            ),
            surface,
            true,
            self.panel_backdrop(),
        );
        let progress = self
            .panel_fade
            .map(|f| f.value(std::time::Instant::now()))
            .unwrap_or(1.0);
        container(
            iced::widget::float(mouse_area(card).on_press(Msg::Noop))
                .translate(move |_, _| iced::Vector::new((1.0 - progress) * 28.0, 0.0)),
        )
        // Aux opérations, le volet se nomme : le focus y est borné (HIG ENT-02).
        .id(iced::widget::Id::new(PANEL_ID))
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::End)
        .padding(DRAWER_GAP)
        .into()
    }

    /// Champ : libellé (et avertissement si hyprdesk.lua surcharge la clé), puis le contrôle.
    /// Libellé vide : le contrôle seul, avec l'avertissement — le libellé est alors
    /// l'intitulé du triangle de divulgation d'une section avancée (HIG LAY-01).
    pub(in crate::ui) fn field<'a>(
        &self,
        label: &'a str,
        key: &str,
        control: impl Into<Element<'a, Msg>>,
    ) -> Element<'a, Msg> {
        let mut head = row![].spacing(10).align_y(Alignment::Center);
        let mut titled = false;
        if !label.is_empty() {
            head = head.push(self.marked(crate::i18n::tr(label), 13, self.ink().dim, self.font));
            titled = true;
        }
        if !key.is_empty() && self.cfg.keys_user.contains(key) {
            head = head.push(
                super::selectable::selectable("surchargé dans hyprdesk.lua — sans effet ici")
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(crate::ui::tone::warning()),
            );
            titled = true;
        }
        let control = control.into();
        if !titled {
            return control;
        }
        // L'avis passe sous le libellé quand le texte agrandi (ou une
        // traduction longue) ne les tient plus côte à côte (HIG TYP-08).
        let head: Element<'_, Msg> = head.wrap().into();
        column![head, control].spacing(6).into()
    }

    /// Curseur d'un réglage et sa valeur, saisissable au clavier (HIG SEL-08).
    /// Au repos : le curseur, puis sa valeur en texte plein ; un clic sur la
    /// valeur ouvre un champ. Entrée pose la valeur (`poser`), puis
    /// l'enregistre (`enregistrer`), comme un glissé puis son relâchement ;
    /// Échap referme le champ sans rien changer. Le composant parle en `f64`
    /// (domaine, pas, valeur) : à chaque site de convertir dans `poser`.
    /// Sa présentation (`look`) est celle que la rangée du site avait déjà :
    /// au repos, rien ne bouge d'une rangée faite à la main.
    pub(in crate::ui) fn slider<'a>(
        &self,
        spec: &SliderSpec,
        value: f64,
        affiche: fn(f64) -> String,
        look: SliderLook,
    ) -> Element<'a, Msg> {
        let (min, max) = spec.range;
        let pose = Arc::clone(&spec.pose);
        let slider = iced::widget::slider(min as f32..=max as f32, value as f32, move |v| {
            pose(f64::from(v))
        })
        .step(spec.step as f32)
        .on_release(spec.save.clone())
        .width(look.width);
        // Accès clavier complet (HIG ENT-01) : le curseur seul entre dans le
        // parcours — les flèches le règlent au pas du réglage et
        // l'enregistrent, Entrée ou Espace ouvrent la saisie de sa valeur. Le
        // champ ouvert n'est jamais enveloppé : il est déjà focalisable.
        let slider: Element<'a, Msg> = if self.cfg.general.keyboard_access {
            let pose = Arc::clone(&spec.pose);
            super::focus::steps(
                slider,
                super::focus::Steps {
                    value: value as f32,
                    range: (min as f32, max as f32),
                    step: spec.step as f32,
                    change: Box::new(move |v| pose(f64::from(v))),
                    commit: Some(spec.save.clone()),
                    enter: Some(Box::new(Msg::Panel(PanelMsg::SliderEdit(Box::new(
                        SliderEdit::new(spec.clone(), value),
                    ))))),
                    id: Some(slider_focus_id(&spec.key)),
                },
                self.ink().fg,
            )
        } else {
            slider.into()
        };
        // L'échelle du curseur (HIG SEL-09) : le minimum à gauche, le maximum
        // à droite, dans le format du réglage, sous le curseur seul. Hauteur de
        // ligne fixée : la valeur, à droite, en tient compte pour rester sur la
        // ligne du curseur.
        let dim = self.ink().dim;
        let echelle_h = scale_line_h();
        let borne = |v: f64| {
            text(affiche(v))
                .font(self.font)
                .size(crate::ui::tokens::ts(SCALE_SIZE))
                .line_height(iced::widget::text::LineHeight::Absolute(echelle_h.into()))
                .color(dim)
                .wrapping(iced::widget::text::Wrapping::None)
        };
        let echelle = row![borne(min), Space::new().width(Length::Fill), borne(max)]
            .spacing(8)
            .width(look.width);
        let mut slider = column![slider].spacing(SCALE_GAP).width(look.width);
        // Dix pas au plus : un trait sous chaque pas, au droit de la poignée.
        if let Some(n) = graduations(spec.range, spec.step) {
            let trait_ = || {
                container(Space::new().width(1).height(TICK_H)).style(move |_| container::Style {
                    background: Some(dim.into()),
                    ..container::Style::default()
                })
            };
            let mut ticks = row![trait_()]
                .width(look.width)
                .padding(iced::Padding::from([0.0, HANDLE_R - 0.5]));
            for _ in 0..n {
                ticks = ticks.push(Space::new().width(Length::Fill)).push(trait_());
            }
            slider = slider.push(ticks);
        }
        let slider = slider.push(echelle);
        // Curseur plafonné : son contenant prend la place libre jusqu'au
        // plafond, la valeur reste à sa suite.
        let slider: Element<'_, Msg> = match look.cap {
            Some(cap) => container(slider)
                .width(Length::Fill)
                .max_width(f32::from(cap))
                .into(),
            None => slider.into(),
        };
        let open = self
            .panel
            .as_ref()
            .and_then(|p| p.slider.as_ref())
            .filter(|e| e.spec.key == spec.key);
        let value: Element<'a, Msg> = match open {
            // La saisie : le nombre seul, aligné à droite, dans le champ de
            // recherche du tiroir.
            Some(e) => iced::widget::text_input("", &e.draft)
                .id(SLIDER_ID)
                .on_input(|s| Msg::Panel(PanelMsg::SliderDraft(s)))
                .on_submit(Msg::Panel(PanelMsg::SliderSubmit))
                .align_x(iced::alignment::Horizontal::Right)
                // Bas, comme un champ de la HIG : la rangée grandit à peine.
                .padding(iced::Padding::from([1, 8]))
                .font(self.font)
                .size(crate::ui::tokens::ts(look.size))
                .width(SLIDER_FIELD_W)
                .into(),
            None => {
                let label = text(affiche(value))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(look.size))
                    // Un nombre tient sur une ligne, même agrandi.
                    .wrapping(iced::widget::text::Wrapping::None)
                    .width(look.value_width);
                // L'encre du site : une valeur secondaire en a une propre.
                let label = match look.value_color {
                    Some(ink) => label.color(ink),
                    None => label,
                };
                tooltip(
                    button(label)
                        // Un texte plein, sans la marge d'un bouton : au repos, la
                        // valeur garde la place et la taille qu'elle avait.
                        .padding(0)
                        .style(button::text)
                        .on_press(Msg::Panel(PanelMsg::SliderEdit(Box::new(SliderEdit::new(
                            spec.clone(),
                            value,
                        ))))),
                    crate::i18n::text(crate::i18n::tr("Saisir une valeur")).font(self.font),
                    // La bulle s'ouvre dessous, sans couvrir le libellé du réglage.
                    iced::widget::tooltip::Position::Bottom,
                )
                .into()
            }
        };
        // La rangée remplit comme son curseur ; un curseur largeur fixe laisse
        // la rangée se resserrer sur ses deux éléments. Un curseur plafonné
        // garde une rangée souple : resserrée, elle ne lui laisserait aucune
        // place, et son contenant s'arrête de lui-même au plafond.
        let rangee = match look.width {
            Length::Shrink | Length::Fixed(_) => Length::Shrink,
            souple => souple,
        };
        // La valeur garde sous elle la place de l'échelle : centrée ou en
        // haut, elle reste sur la ligne du curseur.
        let value = beside_slider(spec, value);
        row![slider, value]
            .spacing(f32::from(look.spacing))
            .width(rangee)
            .align_y(look.align_y)
            .into()
    }

    /// Couleurs de texte lisibles du tiroir (WCAG AA sur son fond) : texte,
    /// texte secondaire (aides, comptes, états) et accent utilisé en texte.
    pub(in crate::ui) fn ink(&self) -> Ink {
        let th = &self.cfg.general.theme;
        let r = crate::theme::readable(th.fg, th.bg, th.accent);
        Ink {
            fg: color(r.fg, 1.0),
            dim: color(r.dim, 1.0),
            accent: color(
                crate::theme::ensure_contrast(th.accent, th.bg, r.fg, 4.5),
                1.0,
            ),
        }
    }

    pub(in crate::ui) fn section<'a>(&self, title: &'a str) -> Element<'a, Msg> {
        // Titre accentué : l'encre garantie à 4,5:1 (HIG FON-05).
        let accent = self.ink().accent;
        column![
            Space::new().height(8),
            self.marked(crate::i18n::tr(title), 15, accent, self.title_font()),
        ]
        .into()
    }

    /// Recherche en cours dans les Réglages (vide sinon).
    fn search_query(&self) -> &str {
        self.panel.as_ref().map(|p| p.search.trim()).unwrap_or("")
    }

    /// Texte dont les mots recherchés sont surlignés.
    pub(in crate::ui) fn marked<'a>(
        &self,
        s: &str,
        size: u32,
        c: Color,
        font: iced::Font,
    ) -> Element<'a, Msg> {
        let q = self.search_query();
        self.marked_with(s, q, size, c, font)
    }

    /// [`App::marked`] pour la recherche `q` donnée : celle du lanceur
    /// (`:r`), qui n'est pas celle du tiroir (HIG PAT-13).
    pub(in crate::ui) fn marked_with<'a>(
        &self,
        s: &str,
        q: &str,
        size: u32,
        c: Color,
        font: iced::Font,
    ) -> Element<'a, Msg> {
        let hits = if q.is_empty() {
            Vec::new()
        } else {
            super::settings_search::ranges(s, q)
        };
        if hits.is_empty() {
            return iced::widget::text(s.to_string())
                .font(font)
                .size(crate::ui::tokens::ts(size as f32))
                .color(c)
                .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                .into();
        }
        let accent = color(self.cfg.general.theme.accent, 1.0);
        let fg = self.ink().fg;
        let mut spans: Vec<iced::widget::text::Span<'a, (), iced::Font>> = Vec::new();
        let mut at = 0;
        for (a, b) in hits {
            if a > at {
                spans.push(
                    iced::widget::span(s[at..a].to_string())
                        .font(font)
                        .size(crate::ui::tokens::ts(size as f32))
                        .color(c),
                );
            }
            spans.push(
                iced::widget::span(s[a..b].to_string())
                    .font(self.title_font())
                    .size(crate::ui::tokens::ts(size as f32))
                    .color(fg)
                    .background(Color { a: 0.45, ..accent })
                    .border(Border::default().rounded(4))
                    .padding([0, 2]),
            );
            at = b;
        }
        if at < s.len() {
            spans.push(
                iced::widget::span(s[at..].to_string())
                    .font(font)
                    .size(crate::ui::tokens::ts(size as f32))
                    .color(c),
            );
        }
        // Libellés et titres de réglage : un mot plus long que la ligne (nom
        // d'option, chemin, variable) se coupe, la rangée grandit en hauteur
        // au lieu d'être rognée (HIG TYP-08).
        iced::widget::rich_text(spans)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
            .into()
    }

    /// Bandeau au-dessus d'une page ouverte depuis la recherche.
    fn search_banner(&self, query: &str, here: usize) -> Element<'_, Msg> {
        let ink = self.ink();
        let accent = color(self.cfg.general.theme.accent, 1.0);
        container(
            row![
                iced::widget::text(crate::tr!(
                    "« {q} » : {n} correspondance(s) sur cette page, surlignée(s)",
                    q = query,
                    n = here
                ))
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(ink.fg)
                .width(Length::Fill),
                button(
                    text("Résultats")
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                )
                .padding([3, 10])
                .style(button::secondary)
                .on_press(Msg::Panel(PanelMsg::SearchBack)),
                button(
                    text("Effacer")
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                )
                .padding([3, 10])
                .style(button::text)
                .on_press(Msg::Panel(PanelMsg::SearchClear)),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        )
        .padding([6, 10])
        .style(move |_| container::Style {
            background: Some(Color { a: 0.12, ..accent }.into()),
            border: Border::default()
                .rounded(10)
                .width(1)
                .color(Color { a: 0.35, ..accent }),
            ..Default::default()
        })
        .into()
    }

    /// Liste des résultats, par paquet : libellé surligné, puis « Paquet › Onglet ».
    fn search_results(
        &self,
        query: &str,
        found: &[&'static super::settings_search::Entry],
    ) -> Element<'_, Msg> {
        let ink = self.ink();
        let fg = ink.fg;
        if found.is_empty() {
            return iced::widget::text(crate::tr!(
                "Aucun réglage ne correspond à « {q} ».",
                q = query
            ))
            .font(self.font)
            .size(crate::ui::tokens::ts(13.0))
            .color(ink.dim)
            .into();
        }
        let mut col = column![
            iced::widget::text(crate::tr!(
                "{n} réglage(s) pour « {q} » — Entrée ouvre le premier.",
                n = found.len(),
                q = query
            ))
            .font(self.font)
            .size(crate::ui::tokens::ts(12.0))
            .color(ink.dim)
        ]
        .spacing(4);
        // Pages et onglets : une ligne. Réglages : le bloc qui les contient
        // (la section de la page, sinon les voisins), le réglage trouvé en
        // surbrillance — pour le voir dans son contexte.
        let accent = color(self.cfg.general.theme.accent, 1.0);
        let mut last = "";
        let mut shown_blocks: Vec<(&str, Option<&str>, usize, Option<&str>)> = Vec::new();
        let mut blocks = 0usize;
        for e in found.iter().take(120) {
            let Some(info) = plugins::BUILTIN.iter().find(|i| i.id == e.page) else {
                continue;
            };
            let in_block = matches!(
                e.kind,
                super::settings_search::Kind::Label | super::settings_search::Kind::Help
            ) && e.at.0 > 0;
            let key = (e.page, e.tab, e.at.0, e.section.as_deref());
            if in_block && e.section.is_some() && shown_blocks.contains(&key) {
                continue;
            }
            if e.page != last {
                col = col.push(self.section(info.label));
                last = e.page;
            }
            let tab_label = e
                .tab
                .and_then(|t| plugins::tabs(e.page).iter().find(|x| x.0 == t))
                .map(|x| crate::i18n::tr(x.1));
            let mut place = match tab_label {
                Some(t) => format!("{} › {t}", crate::i18n::tr(info.label)),
                None => crate::i18n::tr(info.label).to_string(),
            };
            let open = Msg::Panel(PanelMsg::SearchOpen(e.page, e.tab));
            let hover = move |_: &iced::Theme, st: button::Status| button::Style {
                background: Some(
                    Color {
                        a: if matches!(st, button::Status::Hovered | button::Status::Pressed) {
                            0.09
                        } else {
                            0.04
                        },
                        ..fg
                    }
                    .into(),
                ),
                text_color: fg,
                border: Border::default().rounded(10),
                ..Default::default()
            };
            if !in_block {
                col = col.push(
                    button(
                        column![
                            self.marked(crate::i18n::tr(&e.text), 13, fg, self.font),
                            iced::widget::text(place)
                                .font(self.font)
                                .size(crate::ui::tokens::ts(11.0))
                                .color(ink.dim),
                        ]
                        .spacing(2),
                    )
                    .width(Length::Fill)
                    .padding([6, 10])
                    .on_press(open)
                    .style(hover),
                );
                continue;
            }
            if blocks >= 40 {
                continue;
            }
            blocks += 1;
            shown_blocks.push(key);
            if let Some(sec) = &e.section {
                place = format!("{place} › {}", crate::i18n::tr(sec));
            }
            let mut lines = column![
                iced::widget::text(place)
                    .font(self.title_font())
                    .size(crate::ui::tokens::ts(11.0))
                    .color(ink.dim)
            ]
            .spacing(3);
            let members = super::settings_search::block(e);
            let total = members.len();
            for m in members
                .iter()
                .filter(|m| m.section.as_deref() != Some(m.text.as_str()))
                .take(12)
            {
                let tr = crate::i18n::tr(&m.text);
                let hit = super::settings_search::matches(&m.text, query)
                    || super::settings_search::matches(tr, query);
                let help = m.kind == super::settings_search::Kind::Help;
                let line = self.marked(
                    tr,
                    if help { 12 } else { 13 },
                    if hit { fg } else { ink.dim },
                    self.font,
                );
                // Le réglage trouvé : liseré et fond d'accent.
                lines = lines.push(container(line).padding([2, 8]).width(Length::Fill).style(
                    move |_| container::Style {
                        background: hit.then(|| Color { a: 0.12, ..accent }.into()),
                        border: if hit {
                            Border::default()
                                .rounded(6)
                                .width(1)
                                .color(Color { a: 0.55, ..accent })
                        } else {
                            Border::default()
                        },
                        ..Default::default()
                    },
                ));
            }
            if total > 12 {
                lines = lines.push(
                    iced::widget::text(crate::tr!("… {n} de plus dans ce bloc", n = total - 12))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(11.0))
                        .color(ink.dim),
                );
            }
            col = col.push(
                button(lines)
                    .width(Length::Fill)
                    .padding([8, 10])
                    .on_press(open)
                    .style(hover),
            );
        }
        if found.len() > 120 {
            col = col.push(
                iced::widget::text(crate::tr!(
                    "… et {n} autres : précisez la recherche.",
                    n = found.len() - 120
                ))
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(ink.dim),
            );
        }
        col.into()
    }
}

/// Couleurs de texte du tiroir des réglages (voir `App::ink`).
#[derive(Debug, Clone, Copy)]
pub(in crate::ui) struct Ink {
    pub fg: Color,
    /// Texte secondaire, au moins 4,5:1 sur le fond du tiroir.
    pub dim: Color,
    /// Accent lisible en texte (onglet choisi, liens).
    pub accent: Color,
}

/// Remise par défaut de `target` (`None` : tout), hors du fil de
/// l'interface : plan, copie, écriture, fichiers de Hyprland appliqués.
pub(in crate::ui) async fn defaults_job(
    path: std::path::PathBuf,
    target: Option<Tab>,
) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let _reason = crate::history::reason("reglages");
        let target = match target {
            Some(t) => crate::defaults::resolve(t)?,
            None => None,
        };
        let plan = crate::defaults::plan(&path, target)?;
        let live = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some();
        let backup = crate::defaults::apply(&path, &plan, crate::defaults::Apply { live })?;
        let home = std::env::var("HOME").unwrap_or_default();
        let b = backup.display().to_string();
        let b = match b.strip_prefix(&home) {
            Some(rest) if !home.is_empty() => format!("~{rest}"),
            _ => b,
        };
        let mut msg = crate::tr!(
            "Configuration par défaut remise ; copie de l'ancienne : {b}",
            b = b
        );
        if !plan.overridden.is_empty() {
            msg += " — ";
            msg += &crate::tr!(
                "fixé dans hyprdesk.lua, non touché : {k}",
                k = plan.overridden.join(", ")
            );
        }
        Ok(msg)
    })
    .await
    .unwrap_or_else(|e| Err(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le tiroir rouvre sur le dernier paquet consulté (HIG PAT-09) ; un
    /// paquet que la configuration ne montre plus ramène au Bureau.
    #[test]
    fn le_tiroir_rouvre_sur_le_dernier_paquet_consulte() {
        let mut app = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-ui-test/config.lua"),
            true,
        )
        .0;
        app.surfaces
            .insert(iced::window::Id::unique(), "DP-1".into());
        let shown = |app: &App| app.panel.as_ref().map(|p| p.tab);
        let _ = app.open_panel();
        assert_eq!(
            shown(&app),
            Some(plugins::desktop::INFO.id),
            "première ouverture : le Bureau"
        );
        let _ = app.panel_update(PanelMsg::Tab("bar"));
        assert_eq!(shown(&app), Some("bar"));

        // Fermé (fin du fondu), puis rouvert : même paquet.
        app.panel = None;
        app.panel_fade = None;
        let _ = app.open_panel();
        assert_eq!(shown(&app), Some("bar"), "rouvert sur le dernier consulté");

        // « share » n'apparaît qu'en configuration avancée.
        let _ = app.panel_update(PanelMsg::Tab("share"));
        app.panel = None;
        app.panel_fade = None;
        let _ = app.open_panel();
        assert_eq!(
            shown(&app),
            Some(plugins::desktop::INFO.id),
            "paquet devenu invisible : le Bureau"
        );
    }

    /// La valeur tapée pour un curseur (HIG SEL-08) : lue avec une virgule
    /// décimale ou une unité, bornée au domaine, ramenée au pas ; illisible,
    /// refusée.
    #[test]
    fn la_valeur_tapee_est_bornee_au_domaine_et_ramenee_au_pas() {
        let taille = (24.0, 96.0);
        assert_eq!(slider_value("72", taille, 1.0), Some(72.0));
        assert_eq!(
            slider_value(" 72 px ", taille, 1.0),
            Some(72.0),
            "l'unité est ignorée"
        );
        assert_eq!(
            slider_value("1", taille, 1.0),
            Some(24.0),
            "bornée au domaine"
        );
        assert_eq!(
            slider_value("500", taille, 1.0),
            Some(96.0),
            "bornée au domaine"
        );
        assert_eq!(
            slider_value("72,6", taille, 1.0),
            Some(73.0),
            "ramenée au pas"
        );
        assert_eq!(slider_value("+0,50", (-1.0, 1.0), 0.05), Some(0.5));
        assert_eq!(slider_value("1500 Hz", (150.0, 1000.0), 10.0), Some(1000.0));
        assert_eq!(slider_value("0.37", (0.0, 1.0), 0.05), Some(0.35));
        for illisible in ["", "  ", "px", "-", "+.", "abc", "."] {
            assert_eq!(slider_value(illisible, taille, 1.0), None, "{illisible:?}");
        }
    }

    /// Le champ de la valeur d'un curseur (HIG SEL-08) : un seul à la fois ;
    /// Entrée le referme, un texte illisible le garde avec sa saisie ; Échap
    /// et le changement de page le referment.
    #[test]
    fn le_champ_de_la_valeur_dun_curseur_se_referme() {
        let mut app = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-ui-test/config.lua"),
            true,
        )
        .0;
        app.surfaces
            .insert(iced::window::Id::unique(), "DP-1".into());
        let _ = app.open_panel();
        let saisie = |app: &App| {
            app.panel
                .as_ref()
                .and_then(|p| p.slider.as_ref())
                .map(|e| e.draft.clone())
        };
        let ouvrir = |app: &mut App, taille: u16| {
            let _ = app.panel_update(PanelMsg::SliderEdit(Box::new(SliderEdit::new(
                plugins::dock::size_spec(),
                f64::from(taille),
            ))));
        };
        ouvrir(&mut app, 48);
        assert_eq!(saisie(&app).as_deref(), Some("48"), "le nombre seul");
        let _ = app.panel_update(PanelMsg::SliderDraft("72 px".into()));
        assert_eq!(saisie(&app).as_deref(), Some("72 px"));
        // Texte illisible : le champ garde la saisie.
        let _ = app.panel_update(PanelMsg::SliderDraft("px".into()));
        let _ = app.panel_update(PanelMsg::SliderSubmit);
        assert_eq!(saisie(&app).as_deref(), Some("px"));
        // Un second champ ouvert remplace le premier : un seul à la fois.
        ouvrir(&mut app, 60);
        assert_eq!(saisie(&app).as_deref(), Some("60"));
        let _ = app.panel_update(PanelMsg::SliderClose);
        assert_eq!(saisie(&app), None, "Échap referme");
        ouvrir(&mut app, 48);
        let _ = app.panel_update(PanelMsg::Tab("bar"));
        assert_eq!(saisie(&app), None, "changer de paquet referme");
        // Entrée valide : la valeur est posée comme un glissé, l'unité tapée
        // est ignorée.
        ouvrir(&mut app, 48);
        let _ = app.panel_update(PanelMsg::SliderDraft("72 px".into()));
        let _ = app.panel_update(PanelMsg::SliderSubmit);
        assert_eq!(saisie(&app), None, "Entrée valide referme");
        assert_eq!(
            app.settings.launcher_size,
            Some(72),
            "Entrée pose la valeur"
        );
        assert_eq!(app.cfg.general.launcher.size, 72, "le dock la suit");
        // Hors domaine : bornée.
        ouvrir(&mut app, 72);
        let _ = app.panel_update(PanelMsg::SliderDraft("500".into()));
        let _ = app.panel_update(PanelMsg::SliderSubmit);
        assert_eq!(app.settings.launcher_size, Some(96), "bornée au domaine");
    }

    /// Un curseur à pas passé au composant (durée du fondu, `effects.rs`) :
    /// Entrée ramène la valeur au pas, la pose et l'enregistre comme un
    /// relâchement ; hors domaine, elle est bornée (HIG SEL-08).
    #[test]
    fn la_valeur_saisie_dun_curseur_a_pas_est_ramenee_au_pas() {
        let mut app = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-ui-test/config.lua"),
            true,
        )
        .0;
        app.surfaces
            .insert(iced::window::Id::unique(), "DP-1".into());
        let _ = app.open_panel();
        let saisir = |app: &mut App, texte: &str| {
            let _ = app.panel_update(PanelMsg::SliderEdit(Box::new(SliderEdit::new(
                plugins::effects::fade_spec(),
                200.0,
            ))));
            let _ = app.panel_update(PanelMsg::SliderDraft(texte.into()));
            let _ = app.panel_update(PanelMsg::SliderSubmit);
        };
        saisir(&mut app, "254 ms");
        assert_eq!(app.settings.fade_ms, Some(250), "ramenée au pas de 10");
        assert_eq!(app.cfg.general.fade_ms, 250, "posée comme un glissé");
        saisir(&mut app, "5000");
        assert_eq!(app.settings.fade_ms, Some(1000), "bornée au domaine");
    }

    /// Un curseur dont la pose emporte une donnée du rendu — la forme de la
    /// barre, `bar.rs` (HIG SEL-08) : Entrée pose la forme à jour, ses autres
    /// champs gardés ; un geste dans le paquet referme le champ, la forme
    /// qu'il garde pouvant être périmée à Entrée.
    #[test]
    fn la_valeur_saisie_dun_curseur_a_forme_pose_la_forme_a_jour() {
        let mut app = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-ui-test/config.lua"),
            true,
        )
        .0;
        app.surfaces
            .insert(iced::window::Id::unique(), "DP-1".into());
        let _ = app.open_panel();
        let saisie = |app: &App| {
            app.panel
                .as_ref()
                .and_then(|p| p.slider.as_ref())
                .map(|e| e.draft.clone())
        };
        let ouvrir = |app: &mut App, marge: u16| {
            let forme = crate::config::BarShape {
                side_margin: marge,
                radius: Some(7),
                ..Default::default()
            };
            let _ = app.panel_update(PanelMsg::SliderEdit(Box::new(SliderEdit::new(
                plugins::bar::side_margin_spec(forme),
                f64::from(marge),
            ))));
        };
        ouvrir(&mut app, 40);
        assert_eq!(saisie(&app).as_deref(), Some("40"));
        let _ = app.panel_update(PanelMsg::Plugin(PluginMsg::Bar(plugins::bar::Msg::Margin(
            8,
        ))));
        assert_eq!(saisie(&app), None, "un geste du paquet referme le champ");
        assert_eq!(
            app.settings.bar.as_ref().map(|b| b.margin),
            Some(8),
            "le geste du paquet est traité"
        );
        // Entrée : la valeur tapée, le reste de la forme gardé.
        ouvrir(&mut app, 40);
        let _ = app.panel_update(PanelMsg::SliderDraft("150 px".into()));
        let _ = app.panel_update(PanelMsg::SliderSubmit);
        assert_eq!(saisie(&app), None, "Entrée valide referme");
        let barre = app.settings.bar.clone().unwrap_or_default();
        assert_eq!(barre.shape.side_margin, 150, "Entrée pose la valeur");
        assert_eq!(
            barre.shape.radius,
            Some(7),
            "le reste de la forme est gardé"
        );
        assert_eq!(barre.margin, 8, "la marge posée avant n'est pas reprise");
    }

    /// La clé d'un curseur peut être le nom d'une option (options de design
    /// du thème, surcharge d'écran) : deux curseurs de noms différents ne
    /// sont pas le même champ ouvert (HIG SEL-08).
    #[test]
    fn deux_curseurs_de_cle_differente_ne_sont_pas_le_meme_champ() {
        let mut app = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-ui-test/config.lua"),
            true,
        )
        .0;
        app.surfaces
            .insert(iced::window::Id::unique(), "DP-1".into());
        let _ = app.open_panel();
        // Même domaine, même messages : seule la clé les distingue.
        let spec = |nom: &str| {
            SliderSpec::new(
                nom,
                (0.0, 100.0),
                1.0,
                |v| {
                    plugins::theme::wrap(plugins::theme::Msg::Opacity(
                        crate::config::Place::Menus,
                        v as f32 / 100.0,
                    ))
                },
                plugins::theme::wrap(plugins::theme::Msg::Commit),
            )
        };
        let hauteur = spec("screen.DP-1.bar_height");
        let texte = spec("screen.DP-1.text_scale");
        let _ = app.panel_update(PanelMsg::SliderEdit(Box::new(SliderEdit::new(
            hauteur, 40.0,
        ))));
        let ouvert = |app: &App, clef: &str| {
            app.panel
                .as_ref()
                .and_then(|p| p.slider.as_ref())
                .is_some_and(|e| &*e.spec.key == clef)
        };
        assert!(ouvert(&app, "screen.DP-1.bar_height"));
        assert!(!ouvert(&app, "screen.DP-1.text_scale"));
        let _ = app.panel_update(PanelMsg::SliderEdit(Box::new(SliderEdit::new(texte, 1.5))));
        assert!(ouvert(&app, "screen.DP-1.text_scale"));
        assert!(!ouvert(&app, "screen.DP-1.bar_height"));
    }

    /// HIG SEL-09 : un curseur de dix pas au plus est gradué, un pas par
    /// trait ; au-delà, ou sur un domaine vide, seules ses bornes s'affichent.
    /// L'échelle d'un curseur gradué est plus haute de la graduation et de son
    /// écart.
    #[test]
    fn un_curseur_de_dix_pas_au_plus_est_gradue() {
        assert_eq!(graduations((1.0, 6.0), 1.0), Some(5), "passes du flou");
        assert_eq!(graduations((1.0, 4.0), 1.0), Some(3));
        assert_eq!(graduations((0.0, 1.0), 0.1), Some(10), "dix pas : gradué");
        assert_eq!(graduations((0.0, 1.0), 0.05), None, "vingt pas");
        assert_eq!(graduations((24.0, 96.0), 1.0), None);
        assert_eq!(graduations((5.0, 5.0), 1.0), None, "domaine vide");
        assert_eq!(
            graduations((0.0, 1.0), 0.3),
            None,
            "pas qui ne tombe pas juste"
        );
        assert_eq!(graduations((0.0, 4.0), 0.0), None, "pas nul");
        assert_eq!(
            slider_band((1.0, 6.0), 1.0) - slider_band((24.0, 96.0), 1.0),
            TICK_H + SCALE_GAP
        );
        assert_eq!(slider_band((24.0, 96.0), 1.0), scale_line_h() + SCALE_GAP);
    }

    /// Le volume d'un appareil ou d'une application se saisit en pour cent,
    /// comme il s'affiche (HIG SEL-08) : « 50 % » pose 0,5 au seul nœud visé ;
    /// hors domaine, la valeur est bornée au plafond du curseur.
    #[test]
    fn le_volume_saisi_en_pour_cent_est_pose_au_noeud_vise() {
        let mut app = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-ui-test/config.lua"),
            true,
        )
        .0;
        app.surfaces
            .insert(iced::window::Id::unique(), "DP-1".into());
        let _ = app.open_panel();
        let noeud = |id: u32, volume: f32| plugins::sound::Node {
            id,
            name: String::new(),
            default: false,
            volume,
            muted: false,
        };
        if let Some(p) = app.panel.as_mut() {
            p.sound = plugins::sound::State {
                outputs: vec![noeud(55, 0.45)],
                inputs: Vec::new(),
                streams: vec![noeud(78, 0.8)],
                loaded: true,
            };
        }
        let volume = |app: &App, id: u32| {
            app.panel
                .as_ref()
                .and_then(|p| {
                    p.sound
                        .outputs
                        .iter()
                        .chain(p.sound.streams.iter())
                        .find(|n| n.id == id)
                })
                .map(|n| n.volume)
        };
        // Ouvre le champ du curseur du nœud, tape, valide ; rend le brouillon proposé à l'ouverture.
        let saisir = |app: &mut App, id: u32, affiche: f64, texte: &str| {
            let _ = app.panel_update(PanelMsg::SliderEdit(Box::new(SliderEdit::new(
                plugins::sound::volume_spec(id),
                affiche,
            ))));
            let champ = app
                .panel
                .as_ref()
                .and_then(|p| p.slider.as_ref())
                .map(|e| e.draft.clone());
            let _ = app.panel_update(PanelMsg::SliderDraft(texte.into()));
            let _ = app.panel_update(PanelMsg::SliderSubmit);
            champ
        };
        assert_eq!(
            saisir(&mut app, 55, f64::from(0.3_f32 * 100.0), "50 %").as_deref(),
            Some("30"),
            "le champ s'ouvre sur la valeur affichée, en pour cent, sans bruit flottant"
        );
        assert_eq!(volume(&app, 55), Some(0.5), "Entrée pose 50 %");
        assert_eq!(
            volume(&app, 78),
            Some(0.8),
            "les autres nœuds ne bougent pas"
        );
        saisir(&mut app, 55, 50.0, "300");
        assert_eq!(
            volume(&app, 55),
            Some(1.5),
            "hors domaine : borné au plafond, 150 %"
        );
    }

    /// HIG TYP-08 : la navigation suit le texte agrandi sans vider la colonne
    /// des réglages ; en tiroir très étroit, icônes seules. La ligne des
    /// actions de l'en-tête passe sous le titre quand le texte ne la tient
    /// plus à côté.
    #[test]
    fn la_navigation_et_letete_suivent_le_texte_agrandi() {
        const MARGES: f32 = 2.0 * DRAWER_GAP + 48.0 + 24.0 + 14.0;
        assert_eq!(
            nav_width(DRAWER_W, 1.0),
            NAV_W,
            "au repos : largeur du repos"
        );
        assert_eq!(nav_width(DRAWER_W, 2.0), 2.0 * NAV_W, "à 200 % : le double");
        assert!(
            DRAWER_W - MARGES - nav_width(DRAWER_W, 2.0) >= DETAIL_MIN,
            "la colonne des réglages garde sa place"
        );
        assert_eq!(
            nav_width(640.0, 1.0),
            NAV_W,
            "tiroir étroit au repos : inchangé"
        );
        assert!(
            (640.0 - MARGES - nav_width(640.0, 2.0) - DETAIL_MIN).abs() < 1e-3,
            "tiroir étroit et texte agrandi : la colonne garde 360 px"
        );
        assert_eq!(
            nav_width(639.0, 1.0),
            46.0,
            "tiroir très étroit : icônes seules"
        );
        assert!(
            header_one_line(DRAWER_W, 1.0),
            "en-tête sur une ligne au repos"
        );
        assert!(
            !header_one_line(DRAWER_W, 2.0),
            "à 200 % : les actions passent dessous"
        );
    }
}
