//! Visite guidée de l'écran Réglages : une suite d'étapes qui ouvrent chacune
//! un paquet (et un onglet) et le présentent dans une carte au-dessus de la
//! page. Proposée à la première ouverture (clé `tour_seen`), relançable par
//! le bouton « Visite guidée » de l'en-tête ou `hyprdesk tour`.

use iced::widget::{Space, button, column, container, row};
use iced::{Alignment, Border, Color, Element, Length, Task};

use super::panel::PanelMsg;
use super::{App, Msg, color, plugins};
use crate::i18n::text;

/// Étape : paquet, onglet (facultatif), titre, explication.
pub struct Step {
    pub pkg: &'static str,
    pub tab: Option<&'static str>,
    pub title: &'static str,
    pub body: &'static str,
}

const fn step(
    pkg: &'static str,
    tab: Option<&'static str>,
    title: &'static str,
    body: &'static str,
) -> Step {
    Step {
        pkg,
        tab,
        title,
        body,
    }
}

pub const STEPS: &[Step] = &[
    step(
        "desktop",
        Some("affichage"),
        "Bienvenue dans hyprdesk",
        "hyprdesk rend le bureau visible d'une touche, sous des fenêtres en tuiles. Trois modes : \
         no-desktop (fenêtres seules), launcher-only (le lanceur par-dessus) et full-desktop (le \
         bureau entier). Ici : la langue, les écrans où s'affiche le bureau, le mode opaque ou verre.",
    ),
    step(
        "keys",
        Some("ensemble"),
        "Les raccourcis",
        "SUPER + D passe d'un mode à l'autre (masqué, lanceur seul, bureau complet), SUPER + ALT + D \
         mène au lanceur seul, SUPER + CTRL + D masque tout ; SUPER + SHIFT + D ouvre ces réglages, \
         SUPER + L verrouille. Cette page montre tous les raccourcis actifs, hors plein bureau et \
         bureau affiché ; hyprdesk ajoute ceux de fenêtres et de workspaces qui manquent. Échap \
         ferme le tiroir.",
    ),
    step(
        "wallpaper",
        None,
        "Le fond d'écran",
        "Une image, un diaporama ou une couleur, pour tous les bureaux, un workspace ou un écran. \
         Une image peut s'étaler sur toute la disposition des écrans. La liste se relit à chaque \
         visite de cet onglet.",
    ),
    step(
        "icons",
        None,
        "Les icônes du bureau",
        "Des raccourcis d'applications posés sur le bureau, sur tous les workspaces ou un seul. \
         Réglages ouverts, ils se déplacent en les glissant sur le bureau.",
    ),
    step(
        "widgets",
        Some("bureau"),
        "Les widgets",
        "Horloge, calendrier, système, lecteur média… Choisissez-en un pour régler sa place et sa \
         taille, ou glissez-le directement sur le bureau. D'autres s'ajoutent en plugins Lua.",
    ),
    step(
        "theme",
        Some("themes"),
        "Le thème",
        "Des thèmes livrés, les vôtres, ou un thème créé depuis une image : couleurs, fonds et \
         effets d'un coup. Les onglets Couleurs et Texte règlent chaque teinte, la police et sa taille.",
    ),
    step(
        "bar",
        Some("general"),
        "La barre",
        "En haut ou en bas, sur tous les écrans ou le principal, compacte sur les écrans étroits. \
         L'onglet Modules choisit ce qu'elle affiche à gauche, au centre et à droite.",
    ),
    step(
        "launcher",
        Some("contenu"),
        "Le lanceur",
        "Le dock : les fenêtres ouvertes, regroupées par application ou par workspace, et vos \
         applications épinglées. Au survol d'un groupe, choisissez la fenêtre.",
    ),
    step(
        "screens",
        Some("disposition"),
        "Les écrans",
        "Placez vos écrans, choisissez le principal et l'orientation. Le plan donne les cotes de \
         chaque écran et de l'ensemble : utile pour tailler un fond d'écran. Identifier affiche \
         le numéro de chaque écran.",
    ),
    step(
        "workspaces",
        Some("ecrans"),
        "Les workspaces",
        "Combien de workspaces existent toujours, sur quel écran, et quels écrans leur sont \
         dédiés. L'onglet Applications envoie une application sur son workspace.",
    ),
    step(
        "layouts",
        Some("general"),
        "Les layouts",
        "Comment Hyprland range les fenêtres : Dwindle, Master, Scrolling, Monocle ou un layout \
         personnalisé, par défaut ou par workspace. Chaque onglet de layout montre ses raccourcis.",
    ),
    step(
        "apps",
        None,
        "Les applications par défaut",
        "Navigateur, terminal, visionneuses, assistant IA… « hyprdesk open <rôle> » lance celle \
         choisie, par exemple depuis un raccourci.",
    ),
    step(
        "assistant",
        None,
        "L'assistant LYRA",
        "LYRA connaît votre bureau et s'ouvre dans son tiroir, avec Claude Code, Codex ou Zcode. \
         Elle demande toujours avant d'agir.",
    ),
    step(
        "lock",
        None,
        "Le verrouillage",
        "L'écran de verrouillage de hyprdesk : fond, contenus affichés, et « Verrouiller \
         maintenant ».",
    ),
    step(
        "desktop",
        Some("affichage"),
        "Pour aller plus loin",
        "« Configuration avancée » (en haut) affiche tous les détails. Tout se règle aussi en ligne \
         de commande (hyprdesk config list) ou dans hyprdesk.lua. Cette visite se relance avec le \
         bouton « Visite guidée ».",
    ),
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
    /// Étapes possibles : celles dont le paquet est actif.
    fn tour_steps(&self) -> Vec<&'static Step> {
        STEPS
            .iter()
            .filter(|s| self.cfg.plugin_on(s.pkg) && plugins::visible(s.pkg, true))
            .collect()
    }

    /// Va à l'étape `i` : son paquet, son onglet.
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
        Task::done(Msg::Panel(PanelMsg::Tab(
            plugins::find(pkg).map_or("desktop", |p| p.id),
        )))
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
                if self.cfg.general.tour_seen {
                    return Task::none();
                }
                self.settings.tour_seen = Some(true);
                self.commit()
            }
        }
    }

    /// Carte de la visite (étape en cours), ou proposition à la première
    /// ouverture ; rien sinon.
    pub(super) fn tour_card(&self) -> Element<'_, Msg> {
        let Some(p) = &self.panel else {
            return Space::new().width(0).height(0).into();
        };
        let th = &self.cfg.general.theme;
        let (fg, accent) = (color(th.fg, 1.0), color(th.accent, 1.0));
        let dim = Color { a: 0.65, ..fg };
        let card = |content| card(content, accent);
        let go = |m: TourMsg| Msg::Panel(PanelMsg::Tour(m));
        let Some(i) = p.tour else {
            if self.cfg.general.tour_seen {
                return Space::new().width(0).height(0).into();
            }
            return card(
                row![
                    column![
                        text(crate::i18n::tr("Première visite ?"))
                            .font(self.title_font())
                            .size(14)
                            .color(fg),
                        text(crate::i18n::tr(
                            "Une visite guidée présente chaque réglage de hyprdesk, en quelques étapes."
                        ))
                        .font(self.font)
                        .size(12)
                        .color(dim),
                    ]
                    .spacing(4)
                    .width(Length::Fill),
                    button(text(crate::i18n::tr("Commencer")).font(self.font).size(12))
                        .padding([5, 12])
                        .on_press(go(TourMsg::Start)),
                    button(text(crate::i18n::tr("Plus tard")).font(self.font).size(12))
                        .padding([5, 12])
                        .style(button::text)
                        .on_press(go(TourMsg::End)),
                ]
                .spacing(10)
                .align_y(Alignment::Center)
                .into(),
            );
        };
        let steps = self.tour_steps();
        let Some(s) = steps.get(i) else {
            return Space::new().width(0).height(0).into();
        };
        let last = i + 1 == steps.len();
        let mut buttons = row![].spacing(8).align_y(Alignment::Center);
        if i > 0 {
            buttons = buttons.push(
                button(text(crate::i18n::tr("Précédent")).font(self.font).size(12))
                    .padding([5, 12])
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
                .size(12),
            )
            .padding([5, 12])
            .on_press(go(if last { TourMsg::End } else { TourMsg::Next })),
        );
        buttons = buttons.push(Space::new().width(Length::Fill));
        if !last {
            buttons = buttons.push(
                button(
                    text(crate::i18n::tr("Quitter la visite"))
                        .font(self.font)
                        .size(11),
                )
                .padding([4, 8])
                .style(button::text)
                .on_press(go(TourMsg::End)),
            );
        }
        card(
            column![
                text(crate::tr!(
                    "VISITE GUIDÉE · {n} / {total}",
                    n = i + 1,
                    total = steps.len()
                ))
                .font(self.title_font())
                .size(10)
                .color(accent),
                text(crate::i18n::tr(s.title))
                    .font(self.title_font())
                    .size(16)
                    .color(fg),
                text(crate::i18n::tr(s.body))
                    .font(self.font)
                    .size(13)
                    .color(fg),
                buttons,
            ]
            .spacing(8)
            .into(),
        )
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
        run(&mut app, TourMsg::Next);
        assert_eq!(app.panel.as_ref().unwrap().tab, "keys");
        assert_eq!(app.subtab("keys"), "ensemble");
        run(&mut app, TourMsg::Next);
        assert_eq!(app.panel.as_ref().unwrap().tab, "wallpaper");
        run(&mut app, TourMsg::Prev);
        assert_eq!(app.panel.as_ref().unwrap().tour, Some(1));
        run(&mut app, TourMsg::End);
        assert_eq!(app.panel.as_ref().unwrap().tour, None);
        assert_eq!(app.settings.tour_seen, Some(true));
    }

    /// Chaque étape désigne un paquet et un onglet qui existent.
    #[test]
    fn etapes() {
        assert!(STEPS.len() >= 10);
        for s in STEPS {
            assert!(plugins::find(s.pkg).is_some(), "{}", s.pkg);
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
