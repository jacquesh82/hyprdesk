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

/// Messages du tiroir : ceux de la coquille, et ceux des paquets (enveloppés).
#[derive(Debug, Clone)]
pub enum PanelMsg {
    /// Affiche le paquet d'identifiant donné.
    Tab(Tab),
    /// Affiche un onglet du paquet (paquet, onglet) ; retenu pour la suite.
    SubTab(Tab, &'static str),
    /// Configuration avancée (tous les détails) ou de base (l'essentiel).
    Advanced(bool),
    /// Visite guidée.
    Tour(super::tour::TourMsg),
    Close,
    /// Fin d'un glissé de curseur : on enregistre.
    Commit,
    /// Message d'un paquet de réglages.
    Plugin(PluginMsg),
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
        plugins::on_show(self, tab)
    }

    /// Paquet où le tiroir rouvre : le dernier consulté, s'il est encore
    /// affiché (plugin actif, configuration avancée suffisante) ; sinon le
    /// Bureau.
    fn panel_restore(&self) -> Tab {
        if self.cfg.plugin_on(self.panel_tab) && plugins::visible(self.panel_tab, self.advanced()) {
            self.panel_tab
        } else {
            plugins::desktop::INFO.id
        }
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
        iced::widget::tooltip(
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
        )
        .into()
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
                Task::none()
            }
            PanelMsg::Search(q) => {
                if let Some(p) = self.panel.as_mut() {
                    p.search_list = !q.trim().is_empty();
                    p.search = q;
                }
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
                self.commit()
            }
            PanelMsg::Close => self.close_panel(),
            PanelMsg::Commit => self.commit(),
            PanelMsg::Plugin(m) => plugins::update(self, m),
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
        let fg = color(self.cfg.general.theme.fg, 1.0);
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
                    .color(Color { a: 0.8, ..fg }),
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
                    .spacing(8),
                ]
                .spacing(8)
                .into(),
            );
        }
        match &p.defaults_done {
            Some((t, r)) if *t == tab => {
                let (msg, c) = match r {
                    Ok(m) => (m.clone(), Color { a: 0.75, ..fg }),
                    Err(e) => (e.clone(), crate::ui::tone::warning()),
                };
                Some(
                    text(msg)
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
        let nav = if w >= 640.0 { 166.0 } else { 46.0 };
        (w - 2.0 * DRAWER_GAP - 48.0 - nav - 24.0 - 14.0).max(0.0)
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
                })
                .into()
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
        let found = super::settings_search::search(&query, |id| {
            self.cfg.plugin_on(id) && plugins::visible(id, adv)
        });
        let count = |id: &str| found.iter().filter(|e| e.page == id).count();
        let searching = !query.is_empty();
        let entry = |label: &'static str, icon: &'static str, tab: Tab| -> Element<'_, Msg> {
            let active = p.tab == tab;
            let n = count(tab);
            // Recherche : les paquets sans correspondance s'effacent.
            let fg = if searching && n == 0 {
                Color { a: 0.35, ..fg }
            } else {
                fg
            };
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
            button(content)
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
                })
                .into()
        };
        // Navigation par groupes ; le paquet choisi déploie ses onglets en
        // sous-entrées (tiroir large). Paquets coupés : absents (rallumables
        // dans Plugins) ; paquets avancés : seulement en configuration avancée.
        let mut nav = column![].spacing(2);
        let mut first = true;
        for g in plugins::GROUPS {
            let items: Vec<&'static plugins::PluginInfo> = g
                .ids
                .iter()
                .filter_map(|id| plugins::BUILTIN.iter().find(|i| i.id == *id))
                .filter(|i| self.cfg.plugin_on(i.id) && plugins::visible(i.id, adv))
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
                                    background: if matches!(
                                        status,
                                        button::Status::Hovered | button::Status::Pressed
                                    ) {
                                        Some(Color { a: 0.06, ..fg }.into())
                                    } else {
                                        None
                                    },
                                    text_color: if active { ink.accent } else { ink.dim },
                                    border: Border::default().rounded(8),
                                    ..Default::default()
                                }),
                        );
                    }
                }
            }
        }
        let nav = scrollable(nav)
            .height(Length::Fill)
            .width(if wide { 166 } else { 46 })
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new()
                    .width(4)
                    .scroller_width(4),
            ));
        let (mon, ws) = self.panel_ctx().unwrap_or_default();
        let close_icon = super::icons::get("window-close").unwrap();
        let close = tooltip(
            button(
                iced::widget::svg(close_icon)
                    .width(16)
                    .height(16)
                    .style(move |_, _| iced::widget::svg::Style { color: Some(fg) }),
            )
            .padding(9)
            .on_press(Msg::Panel(PanelMsg::Close))
            .style(button::text),
            text("Fermer · Échap").font(self.font),
            iced::widget::tooltip::Position::Left,
        );
        // Tiroir étroit : la ligne des actions ne tient plus à côté du titre
        // (la recherche passait sur « Réglages ») ; elle passe dessous.
        let one_line = self.panel_width() >= 880.0;
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
                    button(glyph)
                        .padding(2)
                        .style(button::text)
                        .on_press(Msg::Panel(PanelMsg::SearchClear)),
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
            button(
                text(crate::i18n::tr("Visite guidée"))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
            )
            .padding([4, 10])
            .style(button::text)
            .on_press(Msg::Panel(PanelMsg::Tour(super::tour::TourMsg::Start))),
            button(
                text(crate::i18n::tr("Assistant"))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
            )
            .padding([4, 10])
            .style(button::text)
            .on_press(Msg::Control(crate::control::Command::Wizard)),
            toggler(adv)
                .label(crate::i18n::tr("Configuration avancée"))
                .text_size(12)
                .font(self.font)
                .on_toggle(|v| Msg::Panel(PanelMsg::Advanced(v))),
        ]
        .spacing(12)
        .align_y(Alignment::Center);
        let titles = column![
            text("HYPRDESK")
                .font(self.font)
                .size(crate::ui::tokens::ts(11.0))
                .color(self.ink().dim)
                .font(self.title_font()),
            text("Réglages")
                .font(self.font)
                .size(crate::ui::tokens::ts(27.0))
                .font(self.title_font())
                .color(fg),
            text(crate::tr!("{mon}  ·  Bureau {ws}", mon = mon, ws = ws))
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(Color { a: 0.65, ..fg }),
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
                    .color(Color { a: 0.6, ..fg }),
                Space::new().width(Length::Fill),
                self.defaults_button(p),
            ]
            .spacing(7)
            .align_y(Alignment::Center),
            self.tour_card(),
            self.defaults_status(p),
            self.subtab_bar(p.tab),
            scrollable(container(content).padding(iced::Padding {
                right: 14.0,
                ..iced::Padding::ZERO
            }))
            .height(Length::Fill),
        ]
        .spacing(18)
        .width(Length::Fill);
        let card = container(
            column![
                header,
                rule::horizontal(1),
                row![
                    nav,
                    // Réglages sur une surface de lecture stable (MLG-282).
                    super::tokens::content(detail, &self.cfg.general.theme, 16.0)
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
        let opacity = if self.hypr_blur_off() {
            1.0
        } else {
            self.cfg
                .general
                .theme
                .shown(crate::config::Place::Settings)
                .opacity
        };
        // Verre du thème (MLG-268) au rayon du tiroir ; vraie réfraction de
        // l'écran capturé à l'ouverture (MLG-275).
        let th = &self.cfg.general.theme;
        let card = super::glass::card(
            card,
            crate::config::Place::Settings,
            24.0,
            color(th.fg, 1.0),
            color(
                super::tokens::tinted(th.bg, crate::config::Place::Settings),
                opacity,
            ),
            super::design::panel_surface(&self.theme, opacity),
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
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::End)
        .padding(DRAWER_GAP)
        .into()
    }

    /// Champ : libellé (et avertissement si hyprdesk.lua surcharge la clé), puis le contrôle.
    pub(in crate::ui) fn field<'a>(
        &self,
        label: &'a str,
        key: &str,
        control: impl Into<Element<'a, Msg>>,
    ) -> Element<'a, Msg> {
        let fg = color(self.cfg.general.theme.fg, 1.0);
        let mut head = row![self.marked(
            crate::i18n::tr(label),
            14,
            Color { a: 0.85, ..fg },
            self.font
        )]
        .spacing(10)
        .align_y(Alignment::Center);
        if !key.is_empty() && self.cfg.keys_user.contains(key) {
            head = head.push(
                text("surchargé dans hyprdesk.lua — sans effet ici")
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(crate::ui::tone::warning()),
            );
        }
        column![head, control.into()].spacing(6).into()
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
        let accent = color(self.cfg.general.theme.accent, 1.0);
        column![
            Space::new().height(8),
            self.marked(crate::i18n::tr(title), 16, accent, self.title_font()),
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
        iced::widget::rich_text(spans).into()
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
                            self.marked(crate::i18n::tr(&e.text), 14, fg, self.font),
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
}
