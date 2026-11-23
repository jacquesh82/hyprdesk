//! Visite guidée (« Première visite ») : une fenêtre au-dessus de tout, qui
//! présente les réglages bloc par bloc — un bloc par grand thème des
//! Réglages, quelques paragraphes chacun — pendant que le tiroir derrière
//! s'ouvre sur le thème présenté. Proposée à la première ouverture des
//! Réglages (clé `tour_seen`), relançable par le bouton « Visite guidée » de
//! l'en-tête ou `hyprdesk tour`.

use crate::ui::tokens::Shown as _;
use iced::widget::{Space, button, column, container, row, scrollable, text as raw_text};
use iced::window::Id;
use iced::{Alignment, Border, Color, Element, Length, Shadow, Task, Vector};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use super::panel::PanelMsg;
use super::{App, Msg, color, plugins};
use crate::i18n::text;

pub const NAMESPACE: &str = "hyprdesk-tour";
const WIDTH: f32 = 780.0;
const HEIGHT: f32 = 520.0;
const SHADOW: f32 = 18.0;

/// Un réglage présenté dans un bloc : son paquet (le paragraphe n'apparaît
/// que s'il est actif), un titre, un paragraphe.
pub struct Item {
    pub pkg: &'static str,
    pub title: &'static str,
    pub body: &'static str,
}

/// Un bloc : le paquet (et l'onglet) ouvert derrière, un titre, une phrase
/// d'introduction, ses réglages.
pub struct Step {
    pub pkg: &'static str,
    pub tab: Option<&'static str>,
    pub title: &'static str,
    pub intro: &'static str,
    pub items: &'static [Item],
}

const fn it(pkg: &'static str, title: &'static str, body: &'static str) -> Item {
    Item { pkg, title, body }
}

pub const STEPS: &[Step] = &[
    Step {
        pkg: "desktop",
        tab: Some("affichage"),
        title: "Bienvenue dans hyprdesk",
        intro: "hyprdesk ajoute un vrai bureau à Hyprland sans rien retirer à vos fenêtres en \
                tuiles. Cette visite présente les réglages par grands thèmes ; derrière, les \
                Réglages s'ouvrent sur chacun.",
        items: &[
            it(
                "desktop",
                "Trois modes, une touche",
                "SUPER + D passe de no-desktop (vos fenêtres seules) à launcher-only (le dock \
                 par-dessus), puis à full-desktop : le bureau complet, avec son fond, ses icônes \
                 et ses widgets, les fenêtres flottant au-dessus. Un appui de plus et tout se masque.",
            ),
            it(
                "keys",
                "Les autres touches à connaître",
                "SUPER + SHIFT + D ouvre ces Réglages, SUPER + R le lanceur d'applications, \
                 SUPER + L verrouille. En cas de souci, SUPER + Échap masque le bureau et rend \
                 tous ses raccourcis à Hyprland.",
            ),
            it(
                "history",
                "Rien n'est perdu",
                "Chaque changement s'enregistre aussitôt, après une copie de l'état précédent \
                 (Configuration → Historique). Et « ./scripts/uninstall.sh --restore » remet \
                 toute la machine comme avant hyprdesk.",
            ),
        ],
    },
    Step {
        pkg: "wallpaper",
        tab: None,
        title: "Le bureau",
        intro: "Ce qui apparaît quand le bureau est affiché : le fond, les icônes, et la façon \
                dont il se montre.",
        items: &[
            it(
                "wallpaper",
                "Fond d'écran",
                "Une image, une vidéo, un diaporama ou une couleur, pour tous les bureaux, un \
                 workspace ou un seul écran. Une même image peut s'étaler sur toute la \
                 disposition des écrans.",
            ),
            it(
                "icons",
                "Icônes",
                "Des raccourcis vers des applications, des fichiers ou des dossiers, sur tous les \
                 workspaces ou un seul. Réglages ouverts, ils se rangent en les glissant sur le bureau.",
            ),
            it(
                "desktop",
                "Affichage",
                "Opaque (votre fond) ou verre (les fenêtres floutées derrière), les écrans où le \
                 bureau apparaît, et ce que deviennent les applications ouvertes depuis le bureau.",
            ),
        ],
    },
    Step {
        pkg: "bar",
        tab: Some("general"),
        title: "Barre, dock et Panorama",
        intro: "Les éléments qui restent à portée de main, et ce qui s'y ajoute.",
        items: &[
            it(
                "bar",
                "La barre",
                "En haut ou en bas, sur tous les écrans ou le principal. On y choisit les modules \
                 de gauche, du centre et de droite : workspaces, horloge, son, réseau, \
                 notifications, session…",
            ),
            it(
                "dock",
                "Le dock",
                "Les fenêtres ouvertes, regroupées par application ou par workspace, et vos \
                 applications épinglées. Au survol d'un groupe, on choisit la fenêtre voulue.",
            ),
            it(
                "panorama",
                "Le Panorama",
                "Pour un écran bandeau ou une bande d'écran : des pages faites de modules \
                 (lecteur, système, raccourcis, radios, Hue, notes…) qu'on fait défiler au doigt \
                 ou à la souris.",
            ),
            it(
                "widgets",
                "Widgets et plugins",
                "Horloge, calendrier, système, média… à poser sur le bureau en les glissant. \
                 D'autres s'installent depuis Plugins, ou s'écrivent en quelques lignes de Lua.",
            ),
        ],
    },
    Step {
        pkg: "theme",
        tab: Some("themes"),
        title: "L'apparence",
        intro: "Un style cohérent partout, du bureau jusqu'à vos applications.",
        items: &[
            it(
                "theme",
                "Thème",
                "Des thèmes livrés, les vôtres, ou un thème tiré d'une image : couleurs, police et \
                 effets d'un coup. Les onglets Couleurs et Texte règlent ensuite chaque détail.",
            ),
            it(
                "apptheme",
                "Modules du thème",
                "Les modules du moteur de thème portent le thème jusqu'à kitty et aux \
                 applications GTK, Qt et iced, pour que tout se ressemble.",
            ),
            it(
                "animations",
                "Animations et effets",
                "Ouverture des fenêtres, changement de workspace, flou et ombres : du plus sobre \
                 au plus vivant, ou coupés pour ménager la carte graphique.",
            ),
        ],
    },
    Step {
        pkg: "layouts",
        tab: Some("general"),
        title: "Les fenêtres",
        intro: "Comment Hyprland range, montre et regroupe vos fenêtres.",
        items: &[
            it(
                "layouts",
                "Layouts",
                "Dwindle, Master, Scrolling, Monocle ou un layout à vous, par défaut ou pour un \
                 workspace. Chaque layout présente ses propres raccourcis.",
            ),
            it(
                "workspaces",
                "Workspaces",
                "Combien en garder toujours ouverts, sur quel écran, et quelles applications \
                 s'ouvrent d'elles-mêmes sur le leur.",
            ),
            it(
                "windows",
                "Apparence des fenêtres",
                "Marges, bordures, coins, barres de titre et groupes de fenêtres en onglets.",
            ),
        ],
    },
    Step {
        pkg: "screens",
        tab: Some("disposition"),
        title: "Les écrans",
        intro: "La disposition de vos écrans, et le confort de lecture.",
        items: &[
            it(
                "screens",
                "Disposition",
                "Glissez les écrans pour les placer, choisissez le principal, la résolution et \
                 l'orientation. Un changement ne reste que si vous le confirmez : sinon \
                 l'affichage d'avant revient de lui-même.",
            ),
            it(
                "colors",
                "Couleurs",
                "Une lumière plus chaude le soir, la luminosité, des profils de couleur : pour \
                 reposer les yeux.",
            ),
        ],
    },
    Step {
        pkg: "keys",
        tab: Some("ensemble"),
        title: "Clavier, souris et raccourcis",
        intro: "Tous vos raccourcis au même endroit, et vos périphériques à votre main.",
        items: &[
            it(
                "keys",
                "Raccourcis",
                "Un seul tableau pour tous les raccourcis actifs : ceux de hyprdesk, ceux de votre \
                 configuration Hyprland, et les essentiels de fenêtres et de workspaces que \
                 hyprdesk ajoute s'ils manquent. Une combinaison se change d'un appui sur « Capturer ».",
            ),
            it(
                "keys",
                "Conflits",
                "Si une combinaison est déjà prise ailleurs, hyprdesk le signale et propose de la \
                 prendre, de garder l'existante ou d'en choisir une autre.",
            ),
            it(
                "input",
                "Clavier, souris, pavé tactile",
                "Disposition du clavier, vitesse du pointeur, défilement naturel et gestes du pavé \
                 tactile.",
            ),
        ],
    },
    Step {
        pkg: "lock",
        tab: Some("ecran"),
        title: "Le système",
        intro: "Ce qu'on règle d'habitude dans un panneau système, sans quitter le bureau.",
        items: &[
            it(
                "sound",
                "Son, Wi-Fi, Bluetooth",
                "Sortie et entrée audio, réseaux Wi-Fi et appareils Bluetooth.",
            ),
            it(
                "performance",
                "Énergie et performance",
                "Profil d'énergie, veille, et carte graphique utilisée ; un bouton applique le \
                 réglage comme il le faut (relance de hyprdesk ou nouvelle session).",
            ),
            it(
                "lock",
                "Verrouillage et connexion",
                "L'écran de verrouillage de hyprdesk, l'empreinte digitale si un lecteur est \
                 présent, et un écran de connexion à votre image.",
            ),
            it(
                "passwords",
                "Mots de passe",
                "myMFA garde vos mots de passe et codes de double authentification, et les \
                 remplit pour vous.",
            ),
        ],
    },
    Step {
        pkg: "apps",
        tab: None,
        title: "Les applications",
        intro: "Ce qui s'ouvre, ce qui démarre, et comment tout retrouver.",
        items: &[
            it(
                "apps",
                "Applications par défaut",
                "Navigateur, terminal, éditeur, visionneuses… « hyprdesk open <rôle> » lance \
                 celle choisie, par exemple depuis un raccourci.",
            ),
            it(
                "autostart",
                "Au démarrage",
                "Ce qui se lance à la connexion ; hyprdesk le surveille et le relance s'il s'arrête.",
            ),
            it(
                "index",
                "Recherche de fichiers",
                "Le lanceur trouve aussi vos fichiers : tapez « :f » suivi d'un nom.",
            ),
            it(
                "assistant",
                "LYRA",
                "L'assistante IA connaît votre bureau, s'ouvre dans son tiroir, et demande \
                 toujours avant d'agir.",
            ),
        ],
    },
    Step {
        pkg: "history",
        tab: None,
        title: "Pour aller plus loin",
        intro: "Tout est réglable, et tout se défait.",
        items: &[
            it(
                "desktop",
                "Configuration avancée",
                "L'interrupteur en haut des Réglages affiche tous les détails, marqués en cyan : \
                 options de Hyprland, durées, commandes, carte graphique…",
            ),
            it(
                "history",
                "Historique",
                "Chaque version de la configuration est gardée : on revient à n'importe laquelle \
                 d'un clic.",
            ),
            it(
                "desktop",
                "Lua et ligne de commande",
                "Tout se règle aussi par « hyprdesk config », ou dans hyprdesk.lua pour aller \
                 plus loin. Cette visite se relance avec le bouton « Visite guidée ».",
            ),
        ],
    },
];

/// Commandes de la visite.
#[derive(Debug, Clone)]
pub enum TourMsg {
    Start,
    Next,
    Prev,
    /// Fin ou abandon : la visite ne sera plus proposée.
    End,
}

impl App {
    /// Blocs possibles : ceux dont le paquet ouvert derrière est actif.
    fn tour_steps(&self) -> Vec<&'static Step> {
        STEPS
            .iter()
            .filter(|s| self.cfg.plugin_on(s.pkg) && plugins::visible(s.pkg, true))
            .collect()
    }

    /// Va au bloc `i` : la fenêtre de visite (ouverte au premier bloc), et
    /// derrière, son paquet et son onglet.
    fn tour_go(&mut self, i: usize) -> Task<Msg> {
        let steps = self.tour_steps();
        let Some(s) = steps.get(i) else {
            return Task::none();
        };
        let (pkg, tab) = (s.pkg, s.tab);
        if let Some(p) = self.panel.as_mut() {
            p.tour = Some(i);
        }
        // Un paquet avancé n'apparaît qu'en configuration avancée : la visite le montre quand même.
        if let Some(t) = tab {
            self.subtabs.insert(pkg, t);
        }
        let open = if self.tour_win.is_none() {
            let win = Id::unique();
            self.tour_win = Some(win);
            Task::done(Msg::NewLayerShell {
                settings: NewLayerShellSettings {
                    size: Some((
                        (WIDTH + 2.0 * SHADOW) as u32,
                        (HEIGHT + 2.0 * SHADOW) as u32,
                    )),
                    // Au-dessus de tout, Réglages compris.
                    layer: Layer::Overlay,
                    anchor: LsAnchor::empty(),
                    exclusive_zone: Some(-1),
                    keyboard_interactivity: KeyboardInteractivity::OnDemand,
                    output_option: OutputOption::Active,
                    namespace: Some(NAMESPACE.into()),
                    ..Default::default()
                },
                id: win,
            })
        } else {
            Task::none()
        };
        Task::batch([
            open,
            Task::done(Msg::Panel(PanelMsg::Tab(
                plugins::find(pkg).map_or("desktop", |p| p.id),
            ))),
        ])
    }

    pub(super) fn tour_update(&mut self, m: TourMsg) -> Task<Msg> {
        let at = self.panel.as_ref().and_then(|p| p.tour);
        let n = self.tour_steps().len();
        match m {
            TourMsg::Start => self.tour_go(0),
            TourMsg::Next => match at {
                Some(i) if i + 1 < n => self.tour_go(i + 1),
                _ => self.tour_update(TourMsg::End),
            },
            TourMsg::Prev => self.tour_go(at.unwrap_or(0).saturating_sub(1)),
            TourMsg::End => {
                if let Some(p) = self.panel.as_mut() {
                    p.tour = None;
                }
                let close = match self.tour_win.take() {
                    Some(w) => iced::window::close(w),
                    None => Task::none(),
                };
                if self.cfg.general.tour_seen {
                    return close;
                }
                self.settings.tour_seen = Some(true);
                Task::batch([close, self.commit()])
            }
        }
    }

    /// Dans le tiroir : la proposition de visite à la première ouverture ;
    /// rien pendant la visite (elle a sa fenêtre) ni ensuite.
    pub(super) fn tour_card(&self) -> Element<'_, Msg> {
        let Some(p) = &self.panel else {
            return Space::new().width(0).height(0).into();
        };
        if p.tour.is_some() || self.cfg.general.tour_seen {
            return Space::new().width(0).height(0).into();
        }
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let dim = Color { a: 0.65, ..fg };
        let go = |m: TourMsg| Msg::Panel(PanelMsg::Tour(m));
        card(
            row![
                column![
                    text(crate::i18n::tr("Première visite ?"))
                        .font(self.title_font())
                        .size(crate::ui::tokens::ts(14.0))
                        .color(fg),
                    text(crate::i18n::tr(
                        "Une visite guidée présente les réglages de hyprdesk, thème par thème."
                    ))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(dim),
                ]
                .spacing(4)
                .width(Length::Fill),
                button(
                    text(crate::i18n::tr("Commencer"))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                )
                .padding([5, 12])
                .on_press(go(TourMsg::Start)),
                button(
                    text(crate::i18n::tr("Plus tard"))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                )
                .padding([5, 12])
                .style(button::text)
                .on_press(go(TourMsg::End)),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .into(),
            accent,
        )
    }

    /// Fenêtre de la visite : le bloc en cours, ses réglages en tuiles.
    pub(super) fn view_tour(&self) -> Element<'_, Msg> {
        let i = self.panel.as_ref().and_then(|p| p.tour).unwrap_or(0);
        let steps = self.tour_steps();
        let Some(s) = steps.get(i) else {
            return Space::new().into();
        };
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let dim = Color { a: 0.72, ..fg };
        let bg = color(
            th.bg,
            th.shown(crate::config::Place::Menus).opacity.max(0.97),
        );
        let go = |m: TourMsg| Msg::Panel(PanelMsg::Tour(m));
        let last = i + 1 == steps.len();
        // Les réglages du bloc, en tuiles sur deux colonnes.
        let items: Vec<&Item> = s
            .items
            .iter()
            .filter(|it| self.cfg.plugin_on(it.pkg))
            .collect();
        let tile = |it: &Item| -> Element<'_, Msg> {
            container(
                column![
                    text(crate::i18n::tr(it.title))
                        .font(self.title_font())
                        .size(crate::ui::tokens::ts(15.0))
                        .color(fg),
                    raw_text(crate::i18n::tr(it.body).to_string())
                        .font(self.font)
                        .size(crate::ui::tokens::ts(13.0))
                        .line_height(1.35)
                        .color(dim),
                ]
                .spacing(6),
            )
            .padding(14)
            .width(Length::Fill)
            .style(move |_| container::Style {
                background: Some(Color { a: 0.06, ..fg }.into()),
                border: Border::default().rounded(12),
                ..Default::default()
            })
            .into()
        };
        let mut grid = column![].spacing(12);
        for pair in items.chunks(2) {
            let mut r = row![].spacing(12);
            for it in pair {
                r = r.push(tile(it));
            }
            if pair.len() == 1 {
                r = r.push(Space::new().width(Length::Fill));
            }
            grid = grid.push(r);
        }
        // Points d'avancement.
        let mut dots = row![].spacing(6).align_y(Alignment::Center);
        for k in 0..steps.len() {
            let on = k == i;
            dots = dots.push(
                container(Space::new().width(if on { 18 } else { 7 }).height(7)).style(move |_| {
                    container::Style {
                        background: Some(if on { accent } else { Color { a: 0.3, ..fg } }.into()),
                        border: Border::default().rounded(4),
                        ..Default::default()
                    }
                }),
            );
        }
        let mut buttons = row![dots, Space::new().width(Length::Fill)]
            .spacing(8)
            .align_y(Alignment::Center);
        if !last {
            buttons = buttons.push(
                button(
                    text(crate::i18n::tr("Quitter la visite"))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0)),
                )
                .padding([6, 10])
                .style(button::text)
                .on_press(go(TourMsg::End)),
            );
        }
        if i > 0 {
            buttons = buttons.push(
                button(
                    text(crate::i18n::tr("Précédent"))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(13.0)),
                )
                .padding([7, 14])
                .style(button::secondary)
                .on_press(go(TourMsg::Prev)),
            );
        }
        buttons = buttons.push(
            button(
                text(if last {
                    crate::i18n::tr("Terminer")
                } else {
                    crate::i18n::tr("Suivant")
                })
                .font(self.font)
                .size(crate::ui::tokens::ts(13.0)),
            )
            .padding([7, 16])
            .on_press(go(if last { TourMsg::End } else { TourMsg::Next })),
        );
        let col = column![
            text(crate::tr!(
                "VISITE GUIDÉE · {n} / {total}",
                n = i + 1,
                total = steps.len()
            ))
            .font(self.title_font())
            .size(crate::ui::tokens::ts(11.0))
            .color(accent),
            text(crate::i18n::tr(s.title))
                .font(self.title_font())
                .size(crate::ui::tokens::ts(22.0))
                .color(fg),
            raw_text(crate::i18n::tr(s.intro).to_string())
                .font(self.font)
                .size(crate::ui::tokens::ts(14.0))
                .line_height(1.35)
                .color(fg),
            scrollable(grid).height(Length::Fill),
            buttons,
        ]
        .spacing(12);
        let panel = container(col)
            .padding(24)
            .width(WIDTH)
            .height(HEIGHT)
            .style(move |_| container::Style {
                background: Some(bg.into()),
                border: Border::default()
                    .rounded(super::design::RADIUS)
                    .width(1)
                    .color(Color { a: 0.5, ..accent }),
                shadow: Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.45),
                    offset: Vector::new(0.0, 8.0),
                    blur_radius: 22.0,
                },
                text_color: Some(fg),
                ..Default::default()
            });
        container(panel).padding(SHADOW).into()
    }
}

/// Carte de la visite, bordée de la couleur d'accent.
fn card(content: Element<'_, Msg>, accent: Color) -> Element<'_, Msg> {
    container(content)
        .padding(14)
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Color { a: 0.14, ..accent }.into()),
            border: Border::default().rounded(12).width(1).color(accent),
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Démarrer ouvre la première étape, Suivant avance d'un paquet à
    /// l'autre (avec son onglet), Quitter retient que la visite a été vue.
    #[test]
    fn parcours() {
        let mut app = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-ui-test/config.lua"),
            true,
        )
        .0;
        app.surfaces
            .insert(iced::window::Id::unique(), "DP-1".into());
        let _ = app.open_panel();
        let run = |app: &mut App, m: TourMsg| {
            let _ = app.tour_update(m);
            // L'onglet suit (message rendu par tour_go).
            if let Some(p) = app.panel.as_ref().and_then(|p| p.tour) {
                let pkg = app.tour_steps()[p].pkg;
                let _ = app.update_inner(Msg::Panel(PanelMsg::Tab(pkg)));
            }
        };
        run(&mut app, TourMsg::Start);
        assert_eq!(app.panel.as_ref().unwrap().tour, Some(0));
        assert_eq!(app.panel.as_ref().unwrap().tab, "desktop");
        assert!(app.tour_win.is_some(), "fenêtre de visite ouverte");
        run(&mut app, TourMsg::Next);
        assert_eq!(app.panel.as_ref().unwrap().tab, "wallpaper");
        run(&mut app, TourMsg::Next);
        assert_eq!(app.panel.as_ref().unwrap().tab, "bar");
        assert_eq!(app.subtab("bar"), "general");
        run(&mut app, TourMsg::Prev);
        assert_eq!(app.panel.as_ref().unwrap().tour, Some(1));
        run(&mut app, TourMsg::End);
        assert_eq!(app.panel.as_ref().unwrap().tour, None);
        assert!(app.tour_win.is_none(), "fenêtre de visite fermée");
        assert_eq!(app.settings.tour_seen, Some(true));
    }

    /// Chaque étape désigne un paquet et un onglet qui existent.
    #[test]
    fn etapes() {
        assert!(STEPS.len() >= 8);
        for s in STEPS {
            assert!(plugins::find(s.pkg).is_some(), "{}", s.pkg);
            assert!(!s.items.is_empty(), "{}", s.title);
            for it in s.items {
                assert!(plugins::find(it.pkg).is_some(), "{}", it.pkg);
            }
            if let Some(t) = s.tab {
                assert!(
                    plugins::tabs(s.pkg).iter().any(|(id, _)| *id == t),
                    "{}/{t}",
                    s.pkg
                );
            }
        }
    }
}
