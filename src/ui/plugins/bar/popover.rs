//! Menus déroulants de la barre, à la manière de macOS : une carte arrondie,
//! translucide et floutée, ombre portée, qui descend sous le module cliqué.
//!
//! Deux surfaces : la carte (calque `overlay`, clavier exclusif pour Échap)
//! et un « capteur » transparent plein écran juste dessous, qui ferme le menu
//! au premier clic ailleurs.

use crate::i18n::text;
use crate::ui::tokens::Shown as _;
use iced::widget::{Space, button, column, container, mouse_area, row, slider, toggler};
use iced::window::Id;
use iced::{Alignment, Background, Border, Color, Element, Length, Padding, Size, Task, Vector};
use iced_layershell::reexport::{
    Anchor as LsAnchor, KeyboardInteractivity, Layer, NewLayerShellSettings, OutputOption,
};

use crate::config::{BarPosition, MenuEntry};
use crate::ui::plugins::bar::style::convexe::Edge;
use crate::ui::{App, Fade, Msg, color, format_now};
mod bar_menus;
pub(in crate::ui) mod continuity;

pub const POPOVER_NAMESPACE: &str = "hyprdesk-popover";
/// Marge transparente autour de la carte, pour son ombre.
pub const SHADOW: f32 = 18.0;
const WIDTH: f32 = 328.0;
/// Côté d'une image de menu (pochette).
const IMAGE: f32 = 120.0;
/// Colonne d'un sous-menu d'un niveau (HIG MENU-03) : largeur, écart à la
/// liste parente, délai d'ouverture au survol et de fermeture.
const SUB_W: f32 = 216.0;
const SUB_GAP: f32 = 8.0;
const SUB_OPEN: std::time::Duration = std::time::Duration::from_millis(180);
const SUB_CLOSE: std::time::Duration = std::time::Duration::from_millis(300);
/// Écart entre la barre et le haut de la carte.
pub(in crate::ui) const GAP: f32 = 10.0;

/// Ce qu'on demande d'ouvrir (le contenu peut devoir être calculé avant).
#[derive(Debug, Clone)]
pub enum Request {
    Volume,
    Brightness,
    Calendar,
    Widget(String),
    Workspaces,
    Overflow,
    /// Menu d'un widget Lua : sa fonction `menu`, et l'id du widget à rafraîchir après une action.
    LuaMenu {
        widget: String,
        func: usize,
    },
    /// Menu dbusmenu d'une icône de la zone de notification.
    Tray {
        bus: String,
        menu_path: String,
    },
    /// Choix de la fin du Silence, au centre de contrôle (HIG PAT-04).
    Silence,
    /// Menu de la fenêtre active, sur son titre (MLG-311).
    Window,
    /// Menu « Fenêtres » : toutes les fenêtres ouvertes, un clic y mène.
    Windows,
    /// Menu du shell « hyprdesk », toujours en tête de la barre (HIG MENU-01).
    Shell,
    /// Menu « Aide », en dernier des titres de la barre (HIG MENU-05).
    Help,
    /// Vue étendue de la bulle qui occupe la pilule Island : son corps, ses
    /// actions, l'accès à la carte des notifications (HIG SYS-13).
    Activity {
        id: u32,
    },
    /// Menu contextuel d'un module de barre : la clé du module (HIG ENT-07).
    /// La carte porte la clé `menu_key(module)`.
    ModuleMenu(String),
}

/// Suffixe de la clé de la carte d'un menu contextuel de module (HIG ENT-07).
/// Une clé distincte de celle du clic gauche : un clic droit ne bascule pas la
/// carte du clic gauche. La carte s'ancre au module, sous sa clé à lui.
pub(in crate::ui) const MENU_KEY: &str = "!menu";

/// Clé de la carte du menu contextuel du module `key`.
pub(in crate::ui) fn menu_key(key: &str) -> String {
    format!("{key}{MENU_KEY}")
}

/// Clé du module d'une clé de carte : celle du menu contextuel compris.
pub(in crate::ui) fn module_key(key: &str) -> &str {
    key.strip_suffix(MENU_KEY).unwrap_or(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use iced::{Point, Rectangle};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    fn app() -> (App, Id) {
        let (mut app, _) = App::boot_with(
            std::path::PathBuf::from("/tmp/hyprdesk-bar-tests/config.lua"),
            true,
        );
        let bar = Id::unique();
        app.monitors = vec![crate::hypr::Monitor {
            dpms_off: false,
            name: "A".into(),
            width: 900,
            height: 640,
            x: 0.0,
            y: 0.0,
            scale: 1.0,
            workspace: "1".into(),
            base: String::new(),
            focused: true,
            reserved: (0.0, 44.0, 0.0, 0.0),
        }];
        app.bar_surfaces.insert(
            bar,
            crate::ui::plugins::bar::surface::BarSurface {
                monitor: "A".into(),
                position: BarPosition::Top,
                height: 44,
                margin: 0,
                ..Default::default()
            },
        );
        Arc::make_mut(&mut app.cfg).general.transition = crate::config::Transition::Fade;
        (app, bar)
    }

    #[test]
    fn convex_drawers_attach_to_each_edge_and_close_without_idle_animation() {
        for position in [BarPosition::Top, BarPosition::Bottom] {
            for (x, expected) in [
                (20.0, Edge::Left),
                (
                    440.0,
                    if position == BarPosition::Top {
                        Edge::Top
                    } else {
                        Edge::Bottom
                    },
                ),
                (850.0, Edge::Right),
            ] {
                let (mut app, bar) = app();
                let loaded = crate::config::lua::load_source(
                    "hd.bar({style='convex',convex={duration=0}})",
                    "test.lua",
                    std::path::Path::new("/tmp"),
                )
                .unwrap();
                app.cfg = Arc::new(loaded.config);
                app.bar_surfaces.get_mut(&bar).unwrap().position = position;
                app.bar_anchors.insert(
                    (bar, "volume".into()),
                    Rectangle::new(Point::new(x, 0.0), Size::new(30.0, 44.0)),
                );
                let _ = app.open_popover(bar, "volume".into(), Kind::Volume);
                let p = app.popover.as_ref().unwrap();
                assert_eq!(p.attached.unwrap().edge, expected);
                assert!(p.left >= 0.0 && p.left + p.size.width <= 900.0);
                assert!(p.attached.unwrap().top + p.size.height <= 640.0);
                let _ = app.popover_frame();
                assert!(!app.popover.as_ref().unwrap().animating(Instant::now()));
                let _ = app.close_popover();
                assert!(app.popover.is_none());
            }
        }
    }

    /// HIG PRES-09 : le bouton de confirmation porte le verbe de l'entrée.
    #[test]
    fn la_confirmation_porte_le_verbe_de_l_entree() {
        assert_eq!(
            confirm_label("Éteindre…"),
            crate::i18n::tr("Éteindre").trim_end_matches(['…', '.'])
        );
        assert_eq!(confirm_label("Redémarrer"), crate::i18n::tr("Redémarrer"));
        assert_eq!(confirm_label(" … "), crate::i18n::tr("Confirmer"));
    }

    /// HIG PRES-02 : une carte qui change de taille interpole sa hauteur ;
    /// la surface garde la plus grande des deux, puis se resserre.
    #[test]
    fn une_carte_qui_change_de_taille_interpole_sa_hauteur() {
        let (mut app, bar) = app();
        let _ = app.open_popover(bar, "volume".into(), Kind::Volume);
        let win = app.popover.as_ref().unwrap().win;
        let settle = |app: &mut App| {
            let past = Instant::now() - Duration::from_secs(1);
            let p = app.popover.as_mut().unwrap();
            p.visibility.start = past;
            if let Some(g) = p.grow.as_mut() {
                g.fade.start = past;
            }
            let _ = app.popover_frame();
            assert!(app.popover.as_ref().unwrap().grow.is_none());
        };
        let _ = app.fit_popover(win, 200.0);
        settle(&mut app);
        let low = app.popover.as_ref().unwrap().size.height;

        // Plus haute : la surface s'agrandit d'emblée, le cadre part de
        // l'ancienne hauteur.
        let _ = app.fit_popover(win, 300.0);
        let p = app.popover.as_ref().unwrap();
        assert_eq!(p.size.height, low + 100.0);
        assert!((p.drawn_height(Instant::now()) - low).abs() < 5.0);
        settle(&mut app);
        assert_eq!(app.popover.as_ref().unwrap().size.height, low + 100.0);

        // Plus basse : la surface attend la fin ; une même mesure ne relance rien.
        let _ = app.fit_popover(win, 200.0);
        let start = app.popover.as_ref().unwrap().grow.unwrap().fade.start;
        assert_eq!(app.popover.as_ref().unwrap().size.height, low + 100.0);
        let _ = app.fit_popover(win, 200.0);
        assert_eq!(
            app.popover.as_ref().unwrap().grow.unwrap().fade.start,
            start
        );
        settle(&mut app);
        assert_eq!(app.popover.as_ref().unwrap().size.height, low);
        let _ = app.fit_popover(win, 300.0);
        settle(&mut app);
        let _ = app.fit_popover(win, 200.0);

        // Un changement en cours repart de la hauteur dessinée.
        {
            let p = app.popover.as_mut().unwrap();
            let g = p.grow.as_mut().unwrap();
            g.fade.start = Instant::now() - g.fade.dur / 2;
        }
        let mid = app.popover.as_ref().unwrap().drawn_height(Instant::now());
        assert!(mid < low + 95.0 && mid > low + 5.0);
        let _ = app.fit_popover(win, 300.0);
        let g = app.popover.as_ref().unwrap().grow.unwrap();
        assert!((g.from - mid).abs() < 5.0);
        assert_eq!(g.to, low + 100.0);
        settle(&mut app);

        // À la fermeture : immédiat.
        let _ = app.close_popover();
        let _ = app.fit_popover(win, 250.0);
        let p = app.popover.as_ref().unwrap();
        assert!(p.grow.is_none());
        assert_eq!(p.size.height, low + 50.0);
        let _ = app.open_popover(bar, "volume".into(), Kind::Volume);
        let _ = app.fit_popover(win, 300.0);
        settle(&mut app);

        // Sans transition : immédiat.
        Arc::make_mut(&mut app.cfg).general.transition = crate::config::Transition::None;
        let _ = app.fit_popover(win, 200.0);
        let p = app.popover.as_ref().unwrap();
        assert!(p.grow.is_none());
        assert_eq!(p.size.height, low);
    }

    /// HIG PRES-02 : une carte remplacée va vers la hauteur de la nouvelle.
    #[test]
    fn une_carte_remplacee_interpole_sa_hauteur() {
        let (mut app, bar) = app();
        let _ = app.open_popover(bar, "volume".into(), Kind::Volume);
        let before = app.popover.as_ref().unwrap().size.height;
        let _ = app.open_popover(bar, "calendar".into(), Kind::Calendar);
        let p = app.popover.as_ref().unwrap();
        let g = p
            .grow
            .expect("Son et Calendrier n'ont pas la même hauteur estimée");
        assert!((g.from - before).abs() < 0.5);
        assert_eq!(p.size.height, g.from.max(g.to));
    }

    #[test]
    fn late_first_present_keeps_the_entire_opening_transition() {
        let (mut app, bar) = app();
        let _ = app.open_popover(bar, "volume".into(), Kind::Volume);
        let win = app.popover.as_ref().unwrap().win;
        app.popover.as_mut().unwrap().shown = false;
        app.popover.as_mut().unwrap().visibility.start = Instant::now() - Duration::from_secs(1);
        let _ = app.popover_frame();
        assert!(!app.popover.as_ref().unwrap().animating(Instant::now()));
        let _ = app.popover_shown(win);
        assert!(app.popover.as_ref().unwrap().animating(Instant::now()));
        assert!(
            app.popover
                .as_ref()
                .unwrap()
                .visibility
                .value(Instant::now())
                < 0.1
        );
    }

    #[test]
    fn cards_are_bounded_and_anchored_without_a_pointer() {
        for width in [320.0, 640.0, 900.0, 1440.0] {
            for x in [0.0, width / 2.0, width - 1.0] {
                let (left, w, origin) = placement(
                    width,
                    Rectangle::new(Point::new(x, 0.0), Size::new(1.0, 44.0)),
                );
                assert!(left + SHADOW >= 12.0 - 0.01);
                assert!(left + w - SHADOW <= width - 12.0 + 0.01);
                assert!((0.05..=0.95).contains(&origin));
            }
        }
        let (mut app, bar) = app();
        app.bar_anchors.insert(
            (bar, "volume".into()),
            Rectangle::new(Point::new(820.0, 0.0), Size::new(30.0, 44.0)),
        );
        let _ = app.open_popover(bar, "volume".into(), Kind::Volume);
        assert!(app.popover.as_ref().unwrap().left > 500.0);
        assert!(app.cursor.is_empty());
        app.bar_anchors.insert(
            (bar, "controls".into()),
            Rectangle::new(Point::new(850.0, 0.0), Size::new(30.0, 44.0)),
        );
        app.bar_anchors.insert(
            (bar, "overflow".into()),
            Rectangle::new(Point::new(20.0, 0.0), Size::new(30.0, 44.0)),
        );
        let _ = app.open_popover(bar, "controls".into(), Kind::Widget("controls".into()));
        let _ = app.open_popover(
            bar,
            "bar:controls:reseau".into(),
            Kind::Widget("controls".into()),
        );
        assert!(
            app.popover.as_ref().unwrap().left > 500.0,
            "une ancienne ancre de débordement ne déplace pas les sous-menus du centre de contrôle"
        );
    }

    #[test]
    fn close_can_reverse_and_switches_reuse_both_surfaces() {
        let (mut app, bar) = app();
        let _ = app.open_popover(bar, "volume".into(), Kind::Volume);
        let (win, catcher) = {
            let p = app.popover.as_mut().unwrap();
            p.visibility.from = 1.0;
            p.visibility.to = 1.0;
            (p.win, p.catcher)
        };
        let _ = app.close_popover();
        assert!(app.popover.as_ref().unwrap().closing());
        let _ = app.open_popover(bar, "volume".into(), Kind::Volume);
        assert!(!app.popover.as_ref().unwrap().closing());
        assert_eq!(app.popover.as_ref().unwrap().win, win);
        let _ = app.open_popover(bar, "controls".into(), Kind::Widget("controls".into()));
        let p = app.popover.as_ref().unwrap();
        assert_eq!((p.win, p.catcher), (win, catcher));
        assert!(matches!(p.previous, Some(Kind::Volume)));
        app.popover.as_mut().unwrap().change.as_mut().unwrap().start =
            Instant::now() - Duration::from_secs(1);
        app.popover.as_mut().unwrap().visibility.start = Instant::now() - Duration::from_secs(1);
        // Changement de hauteur de la carte remplacée (HIG PRES-02).
        if let Some(g) = app.popover.as_mut().unwrap().grow.as_mut() {
            g.fade.start = Instant::now() - Duration::from_secs(1);
        }
        let _ = app.popover_frame();
        assert!(app.popover.as_ref().unwrap().previous.is_none());
        assert!(!app.popover.as_ref().unwrap().animating(Instant::now()));
        let _ = app.close_popover();
        app.popover.as_mut().unwrap().visibility.start = Instant::now() - Duration::from_secs(1);
        let _ = app.popover_frame();
        assert!(app.popover.is_none());
    }

    #[test]
    fn final_frame_is_committed_even_after_a_slow_first_present() {
        let (mut app, bar) = app();
        let _ = app.open_popover(bar, "controls".into(), Kind::Widget("controls".into()));
        app.popover.as_mut().unwrap().visibility.start = Instant::now() - Duration::from_secs(1);
        assert!(app.popover.as_ref().unwrap().animating(Instant::now()));
        let _ = app.popover_frame();
        assert!(!app.popover.as_ref().unwrap().animating(Instant::now()));
        Arc::make_mut(&mut app.cfg).general.transition = crate::config::Transition::None;
        let _ = app.close_popover();
        assert!(app.popover.is_none());
    }

    #[test]
    fn opening_before_the_first_layout_follows_the_late_anchor() {
        let (mut app, bar) = app();
        let _ = app.open_popover(bar, "controls".into(), Kind::Widget("controls".into()));
        let win = app.popover.as_ref().unwrap().win;
        let _ = app.update_inner(Msg::BarAnchor(
            bar,
            "controls".into(),
            Rectangle::new(Point::new(850.0, 0.0), Size::new(30.0, 44.0)),
        ));
        assert_eq!(app.popover.as_ref().unwrap().win, win);
        assert!(app.popover.as_ref().unwrap().left > 500.0);
    }

    /// Le centre de Continuité est une carte étroite sous son module, comme
    /// celui de macOS, à la hauteur de ses tuiles ; les autres menus gardent
    /// leur taille.
    #[test]
    fn continuity_controls_are_a_narrow_card_and_other_menus_keep_their_size() {
        let (mut app, bar) = app();
        let loaded = crate::config::lua::load_source(
            "hd.widget('controls', {id='controls'}); hd.bar({style='continuity', right={'controls'}})",
            "test.lua",
            std::path::Path::new("/tmp"),
        )
        .unwrap();
        app.cfg = Arc::new(loaded.config);
        let widget = app
            .cfg
            .widgets
            .iter()
            .find(|w| matches!(w.kind, crate::config::WidgetKind::Controls))
            .unwrap()
            .id
            .clone();
        let _ = app.open_popover(bar, widget.clone(), Kind::Widget(widget));
        let p = app.popover.as_ref().unwrap();
        assert!(p.dashboard.is_some());
        assert_eq!(p.size.width, continuity::WIDTH + 2.0 * SHADOW);
        // Centrée sur son module, à 12 px au moins des bords de l'écran.
        assert!(p.left + SHADOW >= 12.0 && p.left + p.size.width - SHADOW <= 900.0 - 12.0);
        let win = p.win;
        // Sa hauteur est celle de ses tuiles, pas celle de son contenu
        // mesuré : connexions, Silence et carrés, Son (sans luminosité
        // réglable), lecture.
        let _ = app.fit_popover(win, 80.0);
        let square = ((continuity::WIDTH - 24.0 - 10.0) / 2.0 - 10.0) / 2.0;
        assert_eq!(
            app.popover.as_ref().unwrap().size.height,
            24.0 + 56.0 + 10.0 + square + 10.0 + 66.0 + 10.0 + 64.0 + 2.0 * SHADOW
        );
        let _ = app.open_popover(bar, "volume".into(), Kind::Volume);
        let p = app.popover.as_ref().unwrap();
        assert_eq!(p.win, win);
        assert!(p.dashboard.is_none());
        assert_eq!(p.size.width, WIDTH + 2.0 * SHADOW);
    }

    /// HIG SYS-02 : une liste de l'historique qui s'ouvre lit tout — la carte
    /// Notifications ; un autre menu ne lit rien, le centre Continuité non
    /// plus (il ne liste pas l'historique).
    #[test]
    fn une_liste_de_l_historique_qui_s_ouvre_lit_tout() {
        let (mut app, bar) = app();
        let non_lue = |id: u32| {
            crate::notifications::build(id, "a", "", "s", "", &[], &Default::default(), -1)
        };
        app.toasts.history = vec![non_lue(1), non_lue(2)];
        let _ = app.open_popover(bar, "volume".into(), Kind::Volume);
        assert_eq!(
            app.toasts.unread(),
            2,
            "un menu sans historique ne lit rien"
        );
        let _ = app.open_popover(bar, "notifications".into(), Kind::Notifications);
        assert_eq!(app.toasts.unread(), 0, "la carte Notifications lit tout");
        // Une nouvelle arrive : la carte, déjà ouverte, se rouvre sur elle.
        app.toasts.history = vec![non_lue(3)];
        let _ = app.open_popover(bar, "notifications".into(), Kind::Notifications);
        assert_eq!(app.toasts.unread(), 0, "la carte rouverte lit tout");
        // Une notification arrive pendant que la carte est ouverte : la liste
        // la montre, elle est lue ; sous un autre menu, non.
        let arrivee = |id: u32| {
            crate::ui::toasts::TMsg::Event(crate::notifications::Event::Shown(non_lue(id)))
        };
        let _ = app.toasts_update(arrivee(5));
        assert_eq!(
            app.toasts.unread(),
            0,
            "une liste ouverte montre ce qui arrive"
        );
        let _ = app.open_popover(bar, "volume".into(), Kind::Volume);
        let _ = app.toasts_update(arrivee(6));
        assert_eq!(app.toasts.unread(), 1, "sans liste ouverte : non lue");
        // Le centre Continuité ne liste pas l'historique : il ne lit rien.
        let loaded = crate::config::lua::load_source(
            "hd.widget('controls', {id='controls'}); hd.bar({style='continuity', right={'controls'}})",
            "test.lua",
            std::path::Path::new("/tmp"),
        )
        .unwrap();
        app.cfg = Arc::new(loaded.config);
        app.toasts.history = vec![non_lue(4)];
        let _ = app.open_popover(bar, "controls".into(), Kind::Widget("controls".into()));
        assert!(app.popover.as_ref().unwrap().dashboard.is_some());
        assert_eq!(app.toasts.unread(), 1, "le centre Continuité ne lit rien");
    }

    /// HIG ENT-02 : le raccourci du centre ouvre la carte Notifications sur la
    /// barre de l'écran actif ; pressé deux fois, il reste ouvert. Sans barre
    /// (bureau masqué), rien : la carte est ancrée à la barre.
    #[test]
    fn le_raccourci_du_centre_l_ouvre_et_le_garde() {
        fn focus(app: &mut App) {
            let _ = app.update_inner(crate::ui::Msg::Control(crate::control::Command::Focus(
                "notifications".into(),
            )));
        }
        {
            let (mut app, bar) = app();
            focus(&mut app);
            let p = app.popover.as_ref().expect("le raccourci ouvre le centre");
            assert!(p.bar == bar && matches!(p.kind, Kind::Notifications));
            assert!(!p.closing());
            let win = p.win;
            focus(&mut app);
            let p = app
                .popover
                .as_ref()
                .expect("pressé deux fois, il reste ouvert");
            assert_eq!(p.win, win);
            assert!(!p.closing());
            // Une autre barre que celle de l'écran actif : celle de l'écran actif.
            let other = Id::unique();
            app.bar_surfaces.insert(
                other,
                crate::ui::plugins::bar::surface::BarSurface {
                    monitor: "B".into(),
                    ..Default::default()
                },
            );
            let _ = app.open_popover(other, "volume".into(), Kind::Volume);
            focus(&mut app);
            assert_eq!(app.popover.as_ref().unwrap().bar, bar);
        }
        // Sans barre : rien à ouvrir.
        let (mut app, _) = app();
        app.bar_surfaces.clear();
        focus(&mut app);
        assert!(app.popover.is_none());
    }

    /// Une entrée, un trait, le sous-menu « Déplacer vers l'espace » (quatre
    /// espaces), un trait : de quoi ouvrir une colonne.
    fn submenu_menu() -> Kind {
        let item = |label: &str, action: i64| MenuEntry::Item {
            label: label.into(),
            detail: None,
            icon: None,
            checked: None,
            enabled: true,
            action: Some(action),
            confirm: None,
        };
        Kind::Menu {
            entries: vec![
                item("Plein écran", 1),
                MenuEntry::Separator,
                MenuEntry::Submenu {
                    label: "Déplacer vers l'espace".into(),
                    icon: None,
                    enabled: true,
                    children: vec![
                        item("Espace 1", 101),
                        item("Espace 3", 103),
                        item("Espace 4", 104),
                        item("Espace 5", 105),
                    ],
                },
                MenuEntry::Separator,
            ],
            source: MenuSource::Window {
                address: "0xabc".into(),
            },
        }
    }

    fn open_submenu_menu(x: f32) -> App {
        let (mut app, bar) = app();
        app.bar_anchors.insert(
            (bar, "menus:window".into()),
            Rectangle::new(Point::new(x, 0.0), Size::new(64.0, 44.0)),
        );
        let _ = app.open_popover(bar, "menus:window".into(), submenu_menu());
        app
    }

    /// HIG ACC-01 : un menu de la barre se parcourt au clavier, l'`App` tient
    /// l'entrée choisie (traits sautés, en boucle), flèche droite descend dans
    /// la colonne d'un sous-menu, flèche gauche en remonte, et le lecteur
    /// d'écran voit l'entrée choisie au focus.
    #[test]
    fn menu_de_la_barre_au_clavier() {
        use crate::a11y::model::State;
        use iced::keyboard::{Key, key::Named};
        let mut app = open_submenu_menu(12.0);
        let win = app.popover.as_ref().unwrap().win;
        let press = |app: &mut App, k: Named| {
            app.popover_menu_key(win, &Key::Named(k), false)
                .expect("le menu garde ses flèches")
        };
        let p = |app: &App| {
            let p = app.popover.as_ref().unwrap();
            (p.keyed, p.keyed_sub, p.sub)
        };
        assert_eq!(
            p(&app),
            (None, None, None),
            "ouvert à la souris : rien de choisi"
        );
        let _ = press(&mut app, Named::ArrowDown);
        assert_eq!(p(&app), (Some(0), None, None));
        let _ = press(&mut app, Named::ArrowDown);
        assert_eq!(p(&app), (Some(2), None, None), "le trait est sauté");
        let _ = press(&mut app, Named::ArrowDown);
        assert_eq!(p(&app), (Some(0), None, None), "en boucle");
        let _ = press(&mut app, Named::End);
        assert_eq!(p(&app), (Some(2), None, None));
        // Flèche droite : la colonne s'ouvre, sa première entrée choisie.
        let _ = press(&mut app, Named::ArrowRight);
        assert_eq!(p(&app), (Some(2), Some((2, 0)), Some(2)));
        let _ = press(&mut app, Named::ArrowUp);
        assert_eq!(p(&app), (Some(2), Some((2, 3)), Some(2)));
        let menu = app.menu_a11y().expect("menu décrit");
        assert!(!menu.children[1].states.contains(State::Focused));
        assert!(menu.children[1].children[3].states.contains(State::Focused));
        // Flèche gauche : la colonne se referme, sa ligne reprend le focus.
        let _ = press(&mut app, Named::ArrowLeft);
        assert_eq!(p(&app), (Some(2), None, None));
        let menu = app.menu_a11y().expect("menu décrit");
        assert!(menu.children[1].states.contains(State::Focused));
        let _ = press(&mut app, Named::Enter);
        assert_eq!(
            p(&app),
            (Some(2), Some((2, 0)), Some(2)),
            "Entrée ouvre le sous-menu"
        );
        let _ = press(&mut app, Named::ArrowLeft);
        // Ouvert au survol, sans y descendre : flèche gauche le referme aussi.
        let _ = app.popover_update(PMsg::Activate(2));
        assert_eq!(p(&app), (Some(2), None, Some(2)));
        let _ = press(&mut app, Named::ArrowLeft);
        assert_eq!(p(&app), (Some(2), None, None));
        // Une autre ligne de la liste : la colonne ouverte se referme.
        let _ = app.popover_update(PMsg::Activate(2));
        let _ = press(&mut app, Named::Home);
        assert_eq!(p(&app), (Some(0), None, None));
        // Échap et les lettres gardent leur chemin.
        assert!(
            app.popover_menu_key(win, &Key::Named(Named::Escape), false)
                .is_none()
        );
        assert!(
            app.popover_menu_key(win, &Key::Character("a".into()), false)
                .is_none()
        );
        // Une autre surface : rien.
        assert!(
            app.popover_menu_key(Id::unique(), &Key::Named(Named::ArrowDown), false)
                .is_none()
        );
        // Entrée sur une commande : son action, la carte se referme.
        let _ = press(&mut app, Named::Enter);
        assert!(app.popover.as_ref().is_none_or(|p| p.closing()));
    }

    /// Menu à champ de saisie (Aide) : le champ garde ses touches ; flèches
    /// haut et bas passent aux entrées, Tab ramène au champ, que le lecteur
    /// d'écran voit alors au focus.
    #[test]
    fn menu_a_champ_de_saisie_au_clavier() {
        use crate::a11y::model::State;
        use iced::keyboard::{Key, key::Named};
        let (mut app, bar) = app();
        let item = |label: &str, action: i64| MenuEntry::Item {
            label: label.into(),
            detail: None,
            icon: None,
            checked: None,
            enabled: true,
            action: Some(action),
            confirm: None,
        };
        let kind = Kind::Menu {
            entries: vec![
                MenuEntry::Input {
                    label: None,
                    placeholder: "Rechercher".into(),
                    secret: false,
                    action: Some(1),
                },
                MenuEntry::Separator,
                item("Visite guidée", 2),
                item("À propos", 3),
            ],
            source: MenuSource::Shell,
        };
        app.bar_keyboard = Some(bar);
        let _ = app.open_popover(bar, "menus:help".into(), kind);
        let win = app.popover.as_ref().unwrap().win;
        let key = |app: &mut App, k: Key| app.popover_menu_key(win, &k, false);
        assert_eq!(
            app.popover.as_ref().unwrap().keyed,
            None,
            "le champ d'abord"
        );
        assert!(
            app.menu_a11y().unwrap().children[0]
                .states
                .contains(State::Focused)
        );
        for k in [
            Named::Enter,
            Named::Space,
            Named::Home,
            Named::ArrowLeft,
            Named::Tab,
        ] {
            assert!(
                key(&mut app, Key::Named(k)).is_none(),
                "{k:?} reste au champ"
            );
        }
        assert!(key(&mut app, Key::Character("a".into())).is_none());
        assert!(key(&mut app, Key::Named(Named::ArrowUp)).is_some());
        assert_eq!(app.popover.as_ref().unwrap().keyed, Some(3));
        let _ = key(&mut app, Key::Named(Named::ArrowDown));
        assert_eq!(
            app.popover.as_ref().unwrap().keyed,
            Some(2),
            "en boucle, le trait sauté"
        );
        let menu = app.menu_a11y().unwrap();
        assert!(!menu.children[0].states.contains(State::Focused));
        assert!(menu.children[1].states.contains(State::Focused));
        let _ = key(&mut app, Key::Named(Named::Tab));
        assert_eq!(
            app.popover.as_ref().unwrap().keyed,
            None,
            "Tab ramène au champ"
        );
    }

    /// La barre d'un autre écran tient le clavier : un menu ouvert à la
    /// souris ne choisit rien.
    #[test]
    fn menu_ouvert_a_la_souris_ailleurs_ne_choisit_rien() {
        let (mut app, bar) = app();
        app.bar_keyboard = Some(Id::unique());
        let _ = app.open_popover(bar, "menus:window".into(), submenu_menu());
        assert_eq!(app.popover.as_ref().unwrap().keyed, None);
    }

    /// HIG ACC-01 : `DoAction` d'un lecteur d'écran sur une entrée du menu,
    /// désignée par la clé que l'arbre lui donne : comme un clic.
    #[test]
    fn menu_de_la_barre_par_le_lecteur_d_ecran() {
        let mut app = open_submenu_menu(12.0);
        // Ouvert, la ligne du sous-menu déplie sa colonne.
        let _ = app.a11y_action("/menus:window/e2");
        assert_eq!(app.popover.as_ref().unwrap().sub, Some(2));
        let tree = crate::a11y::model::Tree::flatten(&crate::a11y::root(app.a11y_surfaces()));
        let key = |name: &str| {
            tree.nodes
                .iter()
                .find(|n| n.name == name)
                .map(|n| (n.key.clone(), n.action))
        };
        assert_eq!(key("Espace 3"), Some(("/menus:window/e2/e1".into(), true)));
        // Une colonne qui n'est pas l'ouverte : rien.
        let _ = app.a11y_action("/menus:window/e0/e1");
        assert!(!app.popover.as_ref().unwrap().closing());
        let _ = app.a11y_action("/menus:window/e2/e1");
        assert!(
            app.popover.as_ref().is_none_or(|p| p.closing()),
            "action lancée, carte refermée"
        );
    }

    /// HIG ACC-01 : la place d'un menu à l'écran, pour `Component.GetExtents`
    /// ; à côté d'une barre verticale, aucune.
    #[test]
    fn place_du_menu_a_l_ecran() {
        let mut app = open_submenu_menu(12.0);
        let m = app.monitors[0].clone();
        let p = app.popover.as_mut().unwrap();
        let (x, y, w, h) = p.screen_rect(&m).expect("sous une barre horizontale");
        // La surface porte l'ombre de la carte : elle déborde d'autant.
        assert!(
            x >= m.x - SHADOW && y >= m.y - SHADOW && w > 0.0 && h > 0.0,
            "{x} {y}"
        );
        assert!(x + w <= m.x + m.width as f32 + 2.0 * SHADOW);
        p.attached = None;
        p.position = BarPosition::Left;
        assert!(p.screen_rect(&m).is_none());
    }

    /// Ouvert depuis la barre au clavier : la première entrée est choisie.
    #[test]
    fn menu_ouvert_au_clavier_choisit_sa_premiere_entree() {
        let (mut app, bar) = app();
        app.bar_keyboard = Some(bar);
        let _ = app.open_popover(bar, "menus:window".into(), submenu_menu());
        assert_eq!(app.popover.as_ref().unwrap().keyed, Some(0));
    }

    #[test]
    fn sous_menu_la_colonne_passe_a_gauche_pres_du_bord_droit() {
        let surface = WIDTH + 2.0 * SHADOW;
        let extra = SUB_GAP + SUB_W;
        // (gauche de la surface, largeur, colonne à gauche, largeur de la colonne).
        assert_eq!(
            widened(900.0, 12.0, surface, None),
            (12.0, surface + extra, false, SUB_W)
        );
        assert_eq!(
            widened(900.0, 560.0, surface, None),
            (560.0 - extra, surface + extra, true, SUB_W)
        );
        // Écran de 500 px : la colonne se comprime à la place disponible.
        assert_eq!(
            widened(500.0, 12.0, surface, None),
            (12.0, surface + 130.0, false, 122.0)
        );
        // Côté imposé (barre verticale, tiroir) : sans égard à la place.
        assert_eq!(
            widened(900.0, 300.0, surface, Some(true)),
            (300.0 - extra, surface + extra, true, SUB_W)
        );
        // La colonne part à la hauteur de sa ligne : une entrée, un trait.
        let Kind::Menu { entries, .. } = submenu_menu() else {
            panic!("menu attendu");
        };
        assert_eq!(entry_top(&entries, 2, None), 30.0 + 11.0);
    }

    #[test]
    fn sous_menu_le_survol_ouvre_apres_un_delai_et_la_colonne_annule_la_fermeture() {
        let surface = WIDTH + 2.0 * SHADOW;
        let extra = SUB_GAP + SUB_W;
        let mut app = open_submenu_menu(300.0);
        let geometry = |app: &App| {
            let p = app.popover.as_ref().unwrap();
            (p.left, p.size.width, p.sub_left, p.sub_width)
        };
        // Fermée : la carte ordinaire.
        assert_eq!(geometry(&app), (150.0, surface, false, 0.0));
        // Ligne ordinaire survolée avant la fin du délai : l'ouverture en
        // attente est annulée, le jeton armé n'ouvre rien.
        let _ = app.popover_update(PMsg::MenuHover(Some(2)));
        let stale = app.popover.as_ref().unwrap().sub_gen;
        let _ = app.popover_update(PMsg::MenuHover(None));
        assert!(app.popover.as_ref().unwrap().sub_timer.is_none());
        let _ = app.popover_update(PMsg::MenuTick(stale));
        assert_eq!(app.popover.as_ref().unwrap().sub, None);
        // Survol de la ligne : rien tout de suite, ouverte au jeton armé.
        let _ = app.popover_update(PMsg::MenuHover(Some(2)));
        assert_eq!(geometry(&app), (150.0, surface, false, 0.0));
        let token = app.popover.as_ref().unwrap().sub_gen;
        let _ = app.popover_update(PMsg::MenuTick(token - 1));
        assert_eq!(geometry(&app), (150.0, surface, false, 0.0));
        let _ = app.popover_update(PMsg::MenuTick(token));
        assert_eq!(geometry(&app), (150.0, surface + extra, false, SUB_W));
        assert_eq!(app.popover.as_ref().unwrap().sub, Some(2));
        // Autre ligne survolée : fermeture en attente ; entrer dans la
        // colonne l'annule, et le jeton armé ne referme rien.
        let _ = app.popover_update(PMsg::MenuHover(None));
        assert_eq!(
            app.popover.as_ref().unwrap().sub_timer,
            Some((token + 1, None))
        );
        let _ = app.popover_update(PMsg::MenuEnter);
        assert!(app.popover.as_ref().unwrap().sub_timer.is_none());
        let _ = app.popover_update(PMsg::MenuTick(token + 1));
        assert_eq!(app.popover.as_ref().unwrap().sub, Some(2));
        // Sortie de la carte : fermée au jeton armé, la carte redevient ordinaire.
        let _ = app.popover_update(PMsg::MenuLeave);
        let token = app.popover.as_ref().unwrap().sub_gen;
        let _ = app.popover_update(PMsg::MenuTick(token));
        assert_eq!(app.popover.as_ref().unwrap().sub, None);
        assert_eq!(geometry(&app), (150.0, surface, false, SUB_W));
        // Un clic sur la ligne ouvre aussitôt.
        let _ = app.popover_update(PMsg::Activate(2));
        assert_eq!(geometry(&app), (150.0, surface + extra, false, SUB_W));
        // Près du bord droit : la colonne passe à gauche, la liste ne bouge pas.
        let mut app = open_submenu_menu(820.0);
        let _ = app.popover_update(PMsg::Activate(2));
        assert_eq!(
            geometry(&app),
            (542.0 - extra, surface + extra, true, SUB_W)
        );
    }

    #[test]
    fn champ_du_menu_aide_valide_referme_la_carte() {
        let (mut app, bar) = app();
        let _ = app.open_popover(
            bar,
            "menus:help".into(),
            Kind::Menu {
                entries: super::super::menus::help_entries(),
                source: MenuSource::Shell,
            },
        );
        let _ = app.popover_update(PMsg::Input(0, "volume".into()));
        assert_eq!(
            app.popover
                .as_ref()
                .and_then(|p| p.inputs.get(&0))
                .map(String::as_str),
            Some("volume")
        );
        // Validé : le texte part avec la commande, il n'en reste rien dans la
        // carte, qui se referme (HIG MENU-05).
        let _ = app.popover_update(PMsg::Submit(0));
        let closed = match app.popover.as_ref() {
            None => true,
            Some(p) => p.closing() && !p.inputs.contains_key(&0),
        };
        assert!(closed, "la carte du menu « Aide » se referme");
    }

    #[test]
    fn seul_un_menu_qui_commence_par_un_champ_prend_le_clavier() {
        // La recherche ouvre le menu « Aide » : le champ reçoit le clavier.
        assert!(leads_with_input(&Kind::Menu {
            entries: super::super::menus::help_entries(),
            source: MenuSource::Shell,
        }));
        // Les menus qui commencent par une ligne, les cartes simples : non.
        assert!(!leads_with_input(&Kind::Menu {
            entries: super::super::menus::entries(None),
            source: MenuSource::Shell,
        }));
        assert!(!leads_with_input(&Kind::Menu {
            entries: vec![
                crate::config::MenuEntry::Item {
                    label: "Ligne".into(),
                    detail: None,
                    icon: None,
                    checked: None,
                    enabled: true,
                    action: Some(1),
                    confirm: None,
                },
                crate::config::MenuEntry::Input {
                    label: None,
                    placeholder: String::new(),
                    secret: true,
                    action: Some(2),
                },
            ],
            source: MenuSource::Lua { widget: "w".into() },
        }));
        assert!(!leads_with_input(&Kind::Volume));
        // Un mot de passe en tête d'un menu Lua ne prend pas le clavier tout
        // seul.
        assert!(!leads_with_input(&Kind::Menu {
            entries: vec![crate::config::MenuEntry::Input {
                label: None,
                placeholder: String::new(),
                secret: true,
                action: Some(2),
            }],
            source: MenuSource::Lua { widget: "w".into() },
        }));
    }

    #[test]
    fn sous_menu_une_entree_de_la_colonne_rend_son_action() {
        let mut app = open_submenu_menu(300.0);
        // Colonne fermée : rien.
        assert!(app.sub_action(0).is_none());
        let _ = app.popover_update(PMsg::Activate(2));
        assert!(matches!(
            app.sub_action(0),
            Some((MenuSource::Window { address }, 101)) if address == "0xabc"
        ));
        assert!(matches!(app.sub_action(3), Some((_, 105))));
        // Hors de la colonne : rien.
        assert!(app.sub_action(4).is_none());
        // Un menu remplacé referme sa colonne.
        let _ = app.open_popover(
            app.popover.as_ref().unwrap().bar,
            "menus:window".into(),
            submenu_menu(),
        );
        assert_eq!(app.popover.as_ref().unwrap().sub, None);
        assert!(app.sub_action(0).is_none());
        // Les icônes du menu : chevron de la ligne de sous-menu comprise.
        let Kind::Menu { entries, .. } = submenu_menu() else {
            panic!("menu attendu");
        };
        assert_eq!(menu_icons(&entries), [crate::ui::menu::CHEVRON]);
    }
}

/// Nom du premier champ de saisie d'une carte : lui seul en porte un, c'est
/// lui qui reçoit le clavier.
const FIRST_INPUT: &str = "hyprdesk-popover-input-0";

/// Un menu qui commence par un champ en clair — la recherche du menu « Aide »
/// (HIG MENU-05) — reçoit le clavier sur ce champ dès son ouverture : on y
/// tape sans y cliquer. Échap le quitte d'abord, puis referme la carte,
/// comme le renommage d'un espace. Un champ masqué (mot de passe) d'un menu
/// Lua garde la main : il ne prend le clavier qu'au clic.
fn leads_with_input(kind: &Kind) -> bool {
    matches!(
        kind,
        Kind::Menu { entries, .. }
            if matches!(entries.first(), Some(MenuEntry::Input { secret: false, .. }))
    )
}

/// Entrée qu'on choisit au clavier : commande active, sous-menu non vide ;
/// dans la colonne d'un sous-menu (`column`), une commande sans
/// confirmation, la seule que son clic lance.
pub(in crate::ui) fn keyable(e: &MenuEntry, column: bool) -> bool {
    match e {
        MenuEntry::Item {
            enabled,
            action,
            confirm,
            ..
        } => *enabled && action.is_some() && (!column || confirm.is_none()),
        MenuEntry::Submenu {
            enabled, children, ..
        } => !column && *enabled && !children.is_empty(),
        _ => false,
    }
}

/// Menu qu'on parcourt au clavier : sans champ de saisie, qui garde ses
/// flèches.
fn keyboard_menu(entries: &[MenuEntry]) -> bool {
    !entries.iter().any(|e| matches!(e, MenuEntry::Input { .. }))
}

/// Entrée choisie à l'ouverture : la première, quand le menu s'ouvre depuis
/// la barre au clavier ; sinon aucune.
fn keyed_first(kind: &Kind, by_keyboard: bool) -> Option<usize> {
    let Kind::Menu { entries, .. } = kind else {
        return None;
    };
    (by_keyboard && keyboard_menu(entries))
        .then(|| {
            crate::ui::menu::step_where(entries.len(), None, true, |i| keyable(&entries[i], false))
        })
        .flatten()
}

fn focus_first_input(kind: &Kind) -> Task<Msg> {
    if leads_with_input(kind) {
        iced::widget::operation::focus(FIRST_INPUT)
    } else {
        Task::none()
    }
}

/// Icônes à résoudre pour ouvrir un menu : celles de ses entrées, colonne de
/// sous-menu comprise, et le chevron d'une ligne de sous-menu.
fn menu_icons(entries: &[MenuEntry]) -> Vec<&str> {
    fn walk<'a>(entries: &'a [MenuEntry], out: &mut Vec<&'a str>) {
        for e in entries {
            match e {
                MenuEntry::Item { icon: Some(i), .. } => out.push(i),
                MenuEntry::Submenu { icon, children, .. } => {
                    if let Some(i) = icon {
                        out.push(i);
                    }
                    out.push(crate::ui::menu::CHEVRON);
                    walk(children, out);
                }
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(entries, &mut out);
    out
}

#[derive(Debug, Clone)]
pub enum MenuSource {
    Lua {
        widget: String,
    },
    Tray {
        bus: String,
        menu_path: String,
    },
    /// Fenêtre active (adresse lue à l'ouverture du menu).
    Window {
        address: String,
    },
    /// Menu « Fenêtres » : l'action d'une entrée est le rang de l'adresse de
    /// sa fenêtre (lues à l'ouverture du menu).
    Windows {
        addresses: Vec<String>,
    },
    /// Menu « hyprdesk » : les commandes du shell (`bar::menus`).
    Shell,
    /// Fin du Silence : l'action de l'entrée est son rang (`toasts::silence_command`).
    Silence,
    /// Menu contextuel d'un module de barre : les commandes du module
    /// (`bar::menus`, HIG ENT-07).
    Module,
}

#[derive(Debug, Clone)]
pub enum Kind {
    Notifications,
    /// Vue étendue de la notification `id`, bulle vivante ou entrée de
    /// l'historique depuis (HIG SYS-13).
    Activity(u32),
    Volume,
    Brightness,
    Calendar,
    Widget(String),
    Workspaces,
    Overflow(Vec<crate::config::BarItem>),
    Menu {
        entries: Vec<MenuEntry>,
        source: MenuSource,
    },
}

#[derive(Debug, Clone)]
pub enum PMsg {
    /// Clic sur l'entrée n du menu.
    Activate(usize),
    /// Confirmation d'une entrée « confirm ».
    Confirm(usize),
    CancelConfirm,
    Spawn(String),
    /// Saisie dans l'entrée n (champ `input`).
    Input(usize, String),
    /// Entrée validée : l'action reçoit le texte.
    Submit(usize),
    /// Survol de l'entrée n du menu : une ligne de sous-menu, sinon une ligne
    /// ordinaire.
    MenuHover(Option<usize>),
    /// Le pointeur entre dans la carte d'un menu, ou la quitte.
    MenuEnter,
    MenuLeave,
    /// Échéance d'un changement de sous-menu en attente (jeton armé).
    MenuTick(u64),
    /// Clic sur l'entrée n de la colonne du sous-menu ouvert.
    SubActivate(usize),
    /// « Toutes les notifications », au pied de la vue étendue (HIG SYS-13) :
    /// la carte Notifications prend sa place, sous la même ancre.
    ActivityHistory,
    /// « Calendrier », au pied de la vue étendue : l'horloge que la bulle
    /// remplace garde son calendrier.
    ActivityCalendar,
}

#[derive(Debug, Clone, Copy)]
pub(in crate::ui) struct Attached {
    pub edge: Edge,
    pub top: f32,
    pub bar_height: f32,
    height: f32,
    from_height: f32,
}

/// Changement de hauteur d'une carte non rattachée (HIG PRES-02) : le cadre
/// dessiné va de `from` à `to` ; la surface garde la plus grande des deux
/// jusqu'à la fin.
#[derive(Debug, Clone, Copy)]
pub(in crate::ui) struct Grow {
    from: f32,
    to: f32,
    fade: Fade,
}

pub struct Popover {
    pub win: Id,
    pub catcher: Id,
    pub bar: Id,
    pub position: BarPosition,
    pub left: f32,
    pub origin: f32,
    pub(in crate::ui) attached: Option<Attached>,
    pub(in crate::ui) grow: Option<Grow>,
    pub visibility: Fade,
    shown: bool,
    pub change: Option<Fade>,
    pub previous: Option<Kind>,
    pub slide_from: f32,
    /// Module d'origine : recliquer dessus referme.
    pub key: String,
    anchor_key: String,
    pub kind: Kind,
    pub size: Size,
    pub(in crate::ui) dashboard: Option<continuity::Dashboard>,
    /// Dernière relecture du menu (menus du tray : suivis ouverts).
    pub refreshed: std::time::Instant,
    max_height: f32,
    /// Entrée en attente de confirmation.
    pub confirm: Option<usize>,
    /// Texte des champs de saisie, par entrée.
    pub inputs: std::collections::HashMap<usize, String>,
    /// Sous-menu ouvert (HIG MENU-03) : rang de sa ligne dans la liste ;
    /// colonne à gauche de la liste, sa largeur ; changement en attente
    /// (jeton armé, cible).
    pub(in crate::ui) sub: Option<usize>,
    sub_left: bool,
    sub_width: f32,
    pub(in crate::ui) sub_timer: Option<(u64, Option<usize>)>,
    sub_gen: u64,
    /// Entrée choisie au clavier dans la liste (HIG ACC-01), et dans la
    /// colonne du sous-menu : (ligne parente, rang dans la colonne), valable
    /// tant que ce sous-menu est l'ouvert. L'`App` la tient, la ligne se
    /// surligne et le lecteur d'écran l'annonce au focus.
    pub(in crate::ui) keyed: Option<usize>,
    pub(in crate::ui) keyed_sub: Option<(usize, usize)>,
}

impl Popover {
    fn margin(&self) -> (i32, i32, i32, i32) {
        if let Some(a) = self.attached {
            (a.top as i32, 0, 0, self.left as i32)
        } else {
            let v = (GAP - SHADOW) as i32;
            if self.position == BarPosition::Top {
                (v, 0, 0, self.left as i32)
            } else {
                (0, 0, v, self.left as i32)
            }
        }
    }

    /// Place de la surface sur l'écran `m` (x, y, largeur, hauteur), en
    /// pixels logiques : ses marges, comptées depuis le bord de l'écran
    /// rattachée, depuis la zone que les barres réservent sinon. À côté
    /// d'une barre verticale, la hauteur du module n'est pas retenue : aucune.
    pub(in crate::ui) fn screen_rect(
        &self,
        m: &crate::hypr::Monitor,
    ) -> Option<(f32, f32, f32, f32)> {
        let (top, _, bottom, left) = self.margin();
        let (rl, rt, _, rb) = m.reserved;
        let (w, h) = (self.size.width, self.size.height);
        let (x, y) = match (self.attached, self.position) {
            // Ancrée à droite : la marge de gauche ne compte pas.
            (Some(_), BarPosition::Right) => (m.x + m.width as f32 - w, m.y + top as f32),
            (Some(_), _) => (m.x + left as f32, m.y + top as f32),
            (None, BarPosition::Top) => (m.x + rl + left as f32, m.y + rt + top as f32),
            (None, BarPosition::Bottom) => (
                m.x + rl + left as f32,
                m.y + m.height as f32 - rb - bottom as f32 - h,
            ),
            (None, _) => return None,
        };
        Some((x, y, w, h))
    }

    pub(in crate::ui) fn closing(&self) -> bool {
        self.visibility.to == 0.0
    }

    /// Hauteur visée de la carte : la fin du changement en cours.
    fn target_height(&self) -> f32 {
        self.attached
            .map(|a| a.height)
            .or(self.grow.map(|g| g.to))
            .unwrap_or(self.size.height)
    }

    /// Hauteur de la surface que le cadre occupe à l'instant `now`.
    pub(in crate::ui) fn drawn_height(&self, now: std::time::Instant) -> f32 {
        self.grow.map_or(self.size.height, |g| {
            g.from + (g.to - g.from) * g.fade.value(now)
        })
    }

    /// Nouvelle hauteur d'une carte non rattachée (HIG PRES-02) : le cadre y
    /// va depuis sa hauteur dessinée, la surface prend la plus grande des
    /// deux. Sans durée, avant le premier rendu, à la fermeture, ou d'un
    /// tiroir et du tableau Continuité : immédiate.
    fn retarget(&mut self, target: f32, now: std::time::Instant, dur: std::time::Duration) {
        if dur.is_zero()
            || !self.shown
            || self.closing()
            || self.attached.is_some()
            || self.dashboard.is_some()
        {
            self.size.height = target;
            self.grow = None;
            return;
        }
        let from = self.drawn_height(now);
        if (from - target).abs() < 0.5 {
            self.size.height = target;
            self.grow = None;
            return;
        }
        self.grow = Some(Grow {
            from,
            to: target,
            fade: Fade {
                from: 0.0,
                to: 1.0,
                start: now,
                dur,
            },
        });
        self.size.height = from.max(target);
    }
    pub(in crate::ui) fn animating(&self, now: std::time::Instant) -> bool {
        self.shown
            && (!self.visibility.done(now)
                || self.visibility.from != self.visibility.to
                || self.change.is_some()
                || self.grow.is_some())
    }
}

impl Drop for Popover {
    fn drop(&mut self) {
        for input in self.inputs.values_mut() {
            crate::lock::pam::wipe(input);
        }
    }
}

/// Carte ancrée à la boîte rendue du module, même à l'ouverture au clavier.
pub(in crate::ui) fn placement(screen_width: f32, anchor: iced::Rectangle) -> (f32, f32, f32) {
    let width = WIDTH.min((screen_width - 24.0).max(80.0));
    let card_left =
        (anchor.center_x() - width / 2.0).clamp(12.0, (screen_width - width - 12.0).max(12.0));
    let left = card_left - SHADOW;
    let origin = ((anchor.center_x() - left) / (width + 2.0 * SHADOW)).clamp(0.05, 0.95);
    (left, width + 2.0 * SHADOW, origin)
}

/// Carte d'un menu élargie de sa colonne de sous-menu (HIG MENU-03). À droite
/// de la liste tant qu'il reste la place jusqu'au bord de l'écran (12 px),
/// sinon à gauche ; écran trop étroit : la colonne se comprime du côté le plus
/// large. `forced` : côté imposé (vrai : à gauche). Renvoie la position et la
/// largeur de la surface, le côté de la colonne, sa largeur. La liste parente
/// ne bouge pas.
fn widened(screen: f32, left: f32, width: f32, forced: Option<bool>) -> (f32, f32, bool, f32) {
    let card_left = left + SHADOW;
    let room = |sub_left: bool| {
        if sub_left {
            card_left - 12.0
        } else {
            screen - 12.0 - (left + width - SHADOW)
        }
    };
    let want = SUB_GAP + SUB_W;
    let sub_left = forced.unwrap_or_else(|| !(room(false) >= want || room(false) >= room(true)));
    let sub_width = SUB_W.min((room(sub_left) - SUB_GAP).max(120.0));
    let extra = SUB_GAP + sub_width;
    if sub_left {
        (left - extra, width + extra, true, sub_width)
    } else {
        (left, width + extra, false, sub_width)
    }
}

/// Centre de contrôle de Continuité : une carte étroite sous son module,
/// comme celle de macOS, gardée à 12 px des bords de l'écran.
fn dashboard_placement(screen_width: f32, anchor: iced::Rectangle) -> (f32, f32, f32) {
    let width = continuity::WIDTH.min((screen_width - 24.0).max(80.0));
    let card_left =
        (anchor.center_x() - width / 2.0).clamp(12.0, (screen_width - width - 12.0).max(12.0));
    let left = card_left - SHADOW;
    let origin = ((anchor.center_x() - left) / (width + 2.0 * SHADOW)).clamp(0.05, 0.95);
    (left, width + 2.0 * SHADOW, origin)
}

/// Les commandes latérales ouvrent un tiroir depuis leur bord ; le groupe
/// central agrandit sa niche. Les coordonnées ignorent les zones exclusives.
fn attached_placement(
    screen: Size,
    anchor: iced::Rectangle,
    bar: &crate::ui::plugins::bar::surface::BarSurface,
    height: f32,
    // Élargissement de la colonne d'un sous-menu, vers l'intérieur de l'écran.
    extra: f32,
) -> (f32, Size, Attached) {
    let edge = if anchor.center_x() < screen.width * 0.28 {
        Edge::Left
    } else if anchor.center_x() > screen.width * 0.72 {
        Edge::Right
    } else if bar.position == BarPosition::Top {
        Edge::Top
    } else {
        Edge::Bottom
    };
    let width = (WIDTH + 48.0 + extra).min(screen.width);
    let height = height.min((screen.height - bar.height as f32 - 12.0).max(40.0));
    let left = match edge {
        Edge::Left => 0.0,
        Edge::Right => screen.width - width,
        _ => (screen.width - width) * 0.5,
    };
    let top = match edge {
        Edge::Top => 0.0,
        Edge::Bottom => screen.height - height,
        _ => ((screen.height - height) * 0.5).max(bar.height as f32 + 12.0),
    };
    (
        left,
        Size::new(width, height),
        Attached {
            edge,
            top,
            bar_height: bar.height as f32,
            height,
            from_height: height,
        },
    )
}

/// Bouton qui valide une entrée à confirmer : le verbe de l'entrée
/// (« Éteindre », « Redémarrer »), pas « Confirmer » (HIG PRES-09). Libellé
/// traduit, sans ses points de suspension.
fn confirm_label(label: &str) -> String {
    let verb = crate::i18n::tr(label)
        .trim()
        .trim_end_matches(['…', '.'])
        .trim_end()
        .to_string();
    if verb.is_empty() {
        crate::i18n::tr("Confirmer").to_string()
    } else {
        verb
    }
}

/// Détail sous le libellé (deux lignes) plutôt qu'aligné à droite, quand
/// l'ensemble ne tiendrait pas sur une ligne.
use crate::ui::menu::stacked;

/// Hauteur de la carte pour un contenu donné (les surfaces layer-shell ont une taille fixe).
/// Hauteur d'une section du menu Son : titre, curseur, muet.
const AUDIO_SECTION: f32 = 26.0 + 32.0 + 30.0;

/// Périphériques listés : seulement s'il y a un choix à faire.
fn device_rows(list: &[crate::ui::plugins::bar::surface::AudioDevice]) -> f32 {
    if list.len() > 1 {
        list.len() as f32 * 30.0
    } else {
        0.0
    }
}

fn card_height(
    kind: &Kind,
    confirm: Option<usize>,
    audio: &crate::ui::plugins::bar::surface::Audio,
    light: &crate::ui::plugins::bar::surface::Light,
) -> f32 {
    let inner = match kind {
        Kind::Notifications => 420.0,
        // Estimation d'ouverture : le capteur de contenu règle la carte sur
        // son titre, son corps, ses actions et son pied.
        Kind::Activity(_) => 176.0,
        Kind::Widget(_) => 420.0,
        Kind::Workspaces => 300.0,
        Kind::Overflow(items) => 44.0 + items.len() as f32 * 48.0,
        // Sortie, séparateur, micro (ou « aucun »), séparateur, réglages.
        Kind::Volume => {
            let mic = if audio.mic.is_some() {
                AUDIO_SECTION + device_rows(&audio.inputs)
            } else {
                26.0 + 30.0
            };
            AUDIO_SECTION + device_rows(&audio.outputs) + 11.0 + mic + 11.0 + 30.0 + 16.0
        }
        // Un curseur (titre + ligne) par rétroéclairage ou écran.
        Kind::Brightness => {
            let n = light.screen.is_some() as usize
                + light.kbd.is_some() as usize
                + light.displays.len();
            n.max(1) as f32 * (26.0 + 32.0) + 16.0
        }
        // Six lignes au plus : on change de mois sans changer la taille de la carte.
        Kind::Calendar => 112.0 + 28.0 + 6.0 * 23.0,
        Kind::Menu { entries, .. } => {
            let rows: f32 = entries
                .iter()
                .enumerate()
                .map(|(i, e)| entry_height(i, e, confirm))
                .sum();
            rows.max(30.0)
        }
    };
    inner + 12.0
}

/// Hauteur d'une entrée de menu (`confirm` : entrée en attente de confirmation).
fn entry_height(i: usize, e: &MenuEntry, confirm: Option<usize>) -> f32 {
    match e {
        MenuEntry::Header(_) => 26.0,
        MenuEntry::Image(_) => IMAGE + 12.0,
        MenuEntry::Separator => 11.0,
        MenuEntry::Input { label, .. } => 44.0 + if label.is_some() { 20.0 } else { 0.0 },
        MenuEntry::Submenu { .. } => 30.0,
        MenuEntry::Item { label, detail, .. } => {
            let base = if stacked(label, detail.as_deref()) {
                44.0
            } else {
                30.0
            };
            base + if confirm == Some(i) { 40.0 } else { 0.0 }
        }
    }
}

/// Haut de la colonne d'un sous-menu : celui de sa ligne dans la liste.
fn entry_top(entries: &[MenuEntry], sub: usize, confirm: Option<usize>) -> f32 {
    entries
        .iter()
        .enumerate()
        .take(sub)
        .map(|(i, e)| entry_height(i, e, confirm))
        .sum()
}

impl App {
    /// L'ouverture peut précéder le premier relevé des limites (démarrage,
    /// accès clavier, aperçu isolé). Recaler la carte dès que son ancre existe.
    pub(in crate::ui) fn refresh_popover_anchor(&mut self) -> Task<Msg> {
        let Some(p) = self.popover.as_ref().filter(|p| !p.closing()) else {
            return Task::none();
        };
        let Some(bar) = self.bar_surfaces.get(&p.bar) else {
            return Task::none();
        };
        let Some(m) = self.monitors.iter().find(|m| m.name == bar.monitor) else {
            return Task::none();
        };
        let hidden = self.bar_overflow_items(p.bar);
        let module = module_key(&p.key);
        let overflowed = hidden.iter().any(|item| {
            crate::ui::plugins::bar::surface::bar_item_key(item) == module
                || (matches!(item, crate::config::BarItem::Tray) && module.starts_with("tray:"))
        });
        let key = if overflowed {
            "overflow"
        } else {
            p.anchor_key.as_str()
        };
        if matches!(p.kind, Kind::Notifications) {
            return Task::none();
        }
        let Some(anchor) = self.bar_anchors.get(&(p.bar, key.into())).copied() else {
            return Task::none();
        };
        if p.attached.is_some() {
            // Colonne d'un sous-menu ouverte (HIG MENU-03) : le tiroir
            // s'élargit vers l'intérieur de l'écran.
            let extra = if p.sub.is_some() {
                SUB_GAP + SUB_W
            } else {
                0.0
            };
            let (left, size, attached) = attached_placement(
                Size::new(m.width as f32, m.height as f32),
                anchor,
                bar,
                p.size.height,
                extra,
            );
            let p = self.popover.as_mut().unwrap();
            let old = p.attached.unwrap();
            let moved = (p.left - left).abs() > 0.5 || (old.top - attached.top).abs() > 0.5;
            let resized = (p.size.width - size.width).abs() > 0.5;
            p.left = left;
            p.size.width = size.width;
            p.sub_left = attached.edge == Edge::Right;
            p.sub_width = if p.sub.is_some() { SUB_W } else { 0.0 };
            p.attached = Some(Attached {
                height: old.height,
                from_height: old.from_height,
                ..attached
            });
            let mut tasks = Vec::new();
            if moved {
                tasks.push(Task::done(Msg::MarginChange {
                    id: p.win,
                    margin: p.margin(),
                }));
            }
            if resized {
                tasks.push(Task::done(Msg::SizeChange {
                    id: p.win,
                    size: (p.size.width as u32, p.size.height as u32),
                }));
            }
            return Task::batch(tasks);
        }
        let (left, width, origin) = if p.dashboard.is_some() {
            dashboard_placement(m.width as f32, anchor)
        } else {
            placement(m.width as f32, anchor)
        };
        // Colonne d'un sous-menu ouverte (HIG MENU-03) : la carte s'élargit
        // sans que la liste bouge. Barre verticale : du côté opposé à la barre.
        let (left, width, column) = if p.dashboard.is_some() || p.sub.is_none() {
            (left, width, None)
        } else {
            let forced = match bar.position {
                BarPosition::Left => Some(false),
                BarPosition::Right => Some(true),
                _ => None,
            };
            let (left, width, sub_left, sub_width) = widened(m.width as f32, left, width, forced);
            (left, width, Some((sub_left, sub_width)))
        };
        let height = if p.dashboard.is_some() {
            let max_height = (m.height as f32 - bar.height as f32 - 2.0 * GAP).max(80.0);
            (self.dashboard_height(width - 2.0 * SHADOW, &bar.monitor) + 2.0 * SHADOW)
                .min(max_height)
        } else {
            p.size.height
        };
        let p = self.popover.as_mut().unwrap();
        if let Some((sub_left, sub_width)) = column {
            p.sub_left = sub_left;
            p.sub_width = sub_width;
        }
        p.origin = origin;
        let mut tasks = Vec::new();
        if (p.left - left).abs() > 0.5 {
            if p.change.is_some() {
                p.slide_from = (p.slide_from + p.left - left).clamp(-90.0, 90.0);
            }
            p.left = left;
            let v = (GAP - SHADOW) as i32;
            tasks.push(Task::done(Msg::MarginChange {
                id: p.win,
                margin: if p.position == BarPosition::Top {
                    (v, 0, 0, left as i32)
                } else {
                    (0, 0, v, left as i32)
                },
            }));
        }
        if (p.size.width - width).abs() > 0.5 || (p.size.height - height).abs() > 0.5 {
            p.size = Size::new(width, height);
            tasks.push(Task::done(Msg::SizeChange {
                id: p.win,
                size: (width as u32, height as u32),
            }));
        }
        Task::batch(tasks)
    }

    /// Ouvre un menu sous le module cliqué de la barre `bar` (ou le referme).
    pub(in crate::ui) fn open_popover(&mut self, bar: Id, key: String, kind: Kind) -> Task<Msg> {
        let now = std::time::Instant::now();
        // Ouvert depuis la barre au clavier : sa première entrée est choisie.
        let by_keyboard = self.bar_keyboard == Some(bar);
        let Some(b) = self.bar_surfaces.get(&bar).cloned() else {
            return Task::none();
        };
        let Some(m) = self.monitors.iter().find(|m| m.name == b.monitor).cloned() else {
            return Task::none();
        };
        let hidden = self.bar_overflow_items(bar);
        // Menu contextuel d'un module (HIG ENT-07) : sa carte s'ancre au
        // module, comme celle de son clic gauche.
        let module = module_key(&key);
        let overflowed = hidden.iter().any(|item| {
            crate::ui::plugins::bar::surface::bar_item_key(item) == module
                || (matches!(item, crate::config::BarItem::Tray) && module.starts_with("tray:"))
        });
        let anchor_key = if overflowed {
            "overflow".to_string()
        } else if self.bar_anchors.contains_key(&(bar, module.to_string())) || module != key {
            module.to_string()
        } else {
            self.popover
                .as_ref()
                .filter(|p| p.bar == bar)
                .map_or_else(|| key.clone(), |p| p.anchor_key.clone())
        };
        let mut anchor = self
            .bar_anchors
            .get(&(bar, anchor_key.clone()))
            .copied()
            .unwrap_or_else(|| {
                iced::Rectangle::new(
                    iced::Point::new(m.width as f32 / 2.0, 0.0),
                    Size::new(1.0, b.height as f32),
                )
            });
        if matches!(kind, Kind::Notifications) {
            anchor.x = m.width as f32 * 0.5 - anchor.width * 0.5;
        }
        let wide = self.cfg.bar.as_ref().is_some_and(|bar| {
            bar.style == crate::config::BarStyle::Continuite
                && matches!(&kind, Kind::Widget(id) if self.cfg.widgets.iter().any(|w| w.id == *id
                    && matches!(w.kind, crate::config::WidgetKind::Controls)))
        });
        // Une liste de l'historique qui s'ouvre lit tout (HIG SYS-02) : la
        // carte Notifications. Le centre Continuité, composé comme celui de
        // macOS, ne la montre pas.
        let mut tasks = Vec::new();
        if matches!(kind, Kind::Notifications) {
            tasks.push(self.toasts_update(crate::ui::toasts::TMsg::Read));
        }
        let (mut left, width, origin) = if wide {
            dashboard_placement(m.width as f32, anchor)
        } else {
            placement(m.width as f32, anchor)
        };
        let options = self.convex_options().filter(|o| o.attached_panels);
        let max_height = (m.height as f32 - b.height as f32 - 2.0 * GAP).max(80.0);
        let h = if wide {
            (self.dashboard_height(width - 2.0 * SHADOW, &b.monitor) + 2.0 * SHADOW).min(max_height)
        } else {
            (card_height(&kind, None, &self.audio, &self.light)
                + 2.0 * SHADOW
                + if options.is_some() {
                    b.height as f32
                } else {
                    0.0
                })
            .min(max_height)
        };
        let mut size = Size::new(width, h);
        let attached = options.map(|_| {
            let (x, s, a) = attached_placement(
                Size::new(m.width as f32, m.height as f32),
                anchor,
                &b,
                h,
                0.0,
            );
            left = x;
            size = s;
            a
        });
        let mut attached = attached;
        if let Some(a) = attached.as_mut()
            && let Some(old) = self
                .popover
                .as_ref()
                .filter(|p| p.bar == bar)
                .and_then(|p| p.attached)
                .filter(|old| old.edge == a.edge)
        {
            a.from_height = old.height;
            size.height = size.height.max(old.height);
            a.top = match a.edge {
                Edge::Top => 0.0,
                Edge::Bottom => m.height as f32 - size.height,
                _ => (m.height as f32 - size.height) * 0.5,
            };
        }
        let duration = self.bar_duration(options.map_or(180, |o| o.duration as u64));
        let change_duration = self.bar_duration(160);
        let v = (GAP - SHADOW) as i32;
        // Barre verticale (MLG-277) : le menu s'ouvre à côté d'elle, à la
        // hauteur du module.
        let side_top = if b.position.vertical() {
            let roi = self.bar_roi(&b);
            let top = (anchor.y + roi.y - m.reserved.1).max(0.0);
            top.min((m.height as f32 - m.reserved.1 - m.reserved.3 - size.height).max(0.0)) as i32
        } else {
            0
        };
        let margin = if let Some(a) = attached {
            (a.top as i32, 0, 0, left as i32)
        } else {
            match b.position {
                BarPosition::Top => (v, 0, 0, left as i32),
                BarPosition::Bottom => (0, 0, v, left as i32),
                BarPosition::Left => (side_top, 0, 0, v),
                BarPosition::Right => (side_top, v, 0, 0),
            }
        };
        if wide {
            tasks.extend(continuity::ICONS.iter().map(|name| self.want_icon(name)));
        }
        if let Kind::Menu { entries, .. } = &kind {
            tasks.extend(menu_icons(entries).iter().map(|i| self.want_icon(i)));
        }
        if let Some(p) = self
            .popover
            .as_mut()
            .filter(|p| p.bar == bar && p.position == b.position)
        {
            let reversing = p.closing() && p.key == key;
            let edge_changed = p.attached.map(|a| a.edge) != attached.map(|a| a.edge);
            // Carte simple remplacée (HIG PRES-02) : le cadre va vers sa
            // nouvelle hauteur au rythme du fondu de contenu.
            let was_plain = p.attached.is_none() && p.dashboard.is_none();
            p.visibility = Fade {
                from: if edge_changed {
                    0.0
                } else {
                    p.visibility.value(now)
                },
                to: 1.0,
                start: now,
                dur: duration,
            };
            if !reversing {
                let previous = std::mem::replace(&mut p.kind, kind);
                p.previous = (!wide && p.dashboard.is_none()).then_some(previous);
                p.dashboard = wide.then(continuity::Dashboard::new);
                p.change = Some(Fade {
                    from: 0.0,
                    to: 1.0,
                    start: now,
                    dur: change_duration,
                });
                p.slide_from = (p.left - left).clamp(-90.0, 90.0);
            }
            p.key = key;
            p.anchor_key = anchor_key;
            p.left = left;
            p.origin = origin;
            p.attached = attached;
            if was_plain {
                p.size.width = size.width;
                p.retarget(size.height, now, change_duration);
            } else {
                p.size = size;
                p.grow = None;
            }
            p.max_height = max_height;
            p.confirm = None;
            // Un menu remplacé referme le sous-menu du précédent.
            p.sub = None;
            p.sub_timer = None;
            p.sub_left = false;
            p.sub_width = 0.0;
            p.keyed = keyed_first(&p.kind, by_keyboard);
            p.keyed_sub = None;
            for input in p.inputs.values_mut() {
                crate::lock::pam::wipe(input);
            }
            p.inputs.clear();
            // Même carte, nouveau contenu : le champ existe déjà, il prend le
            // clavier tout de suite.
            tasks.push(focus_first_input(&p.kind));
            tasks.push(Task::done(Msg::MarginChange { id: p.win, margin }));
            tasks.push(Task::done(Msg::SizeChange {
                id: p.win,
                size: (p.size.width as u32, p.size.height as u32),
            }));
            // Le contenu remplacé emporte le menu d'une carte de notification
            // ouvert sur lui (HIG ENT-07).
            let win = p.win;
            if !reversing {
                tasks.push(self.menu_on_card(win));
            }
            return Task::batch(tasks);
        }
        tasks.push(self.destroy_popover());
        let (catcher, win) = (Id::unique(), Id::unique());
        // Vraie réfraction (MLG-275) : l'écran est capturé avant que le menu
        // n'apparaisse ; son fondu attend la capture.
        self.behind = None;
        let capture = crate::ui::behind::wanted(crate::config::Place::Menus);
        if capture {
            tasks.push(Task::perform(
                crate::ui::behind::capture(
                    m.name.clone(),
                    POPOVER_NAMESPACE,
                    iced::Point::new(m.x, m.y),
                    Size::new(m.width as f32, m.height as f32),
                ),
                move |b| Msg::Behind(win, b),
            ));
        }
        // Le capteur d'abord : la carte, créée ensuite et sur le calque overlay, passe au-dessus.
        tasks.push(Task::done(Msg::NewLayerShell {
            settings: NewLayerShellSettings {
                size: Some((0, 0)),
                // Barre par-dessus un plein écran : le capteur aussi, sinon un
                // clic hors du menu irait à la fenêtre (HIG PAT-03).
                layer: if self.bar_over_fullscreen.contains(&bar) {
                    Layer::Overlay
                } else {
                    Layer::Top
                },
                anchor: LsAnchor::Top | LsAnchor::Bottom | LsAnchor::Left | LsAnchor::Right,
                exclusive_zone: Some(-1),
                keyboard_interactivity: KeyboardInteractivity::None,
                // La barre garde ses événements pour passer directement d'un
                // menu à l'autre, sans traverser le capteur plein écran.
                margin: Some(match b.position {
                    BarPosition::Top => (b.height as i32, 0, 0, 0),
                    BarPosition::Bottom => (0, 0, b.height as i32, 0),
                    BarPosition::Left => (0, 0, 0, b.height as i32),
                    BarPosition::Right => (0, b.height as i32, 0, 0),
                }),
                output_option: OutputOption::OutputName(m.name.clone()),
                namespace: Some("hyprdesk-catcher".into()),
                ..Default::default()
            },
            id: catcher,
        }));
        let edge = if attached.is_some() || b.position != BarPosition::Bottom {
            LsAnchor::Top
        } else {
            LsAnchor::Bottom
        };
        let side = if b.position == BarPosition::Right {
            LsAnchor::Right
        } else {
            LsAnchor::Left
        };
        tasks.push(Task::done(Msg::NewLayerShell {
            settings: NewLayerShellSettings {
                size: Some((size.width as u32, size.height as u32)),
                layer: Layer::Overlay,
                anchor: edge | side,
                exclusive_zone: Some(if attached.is_some() { -1 } else { 0 }),
                margin: Some(margin),
                keyboard_interactivity: KeyboardInteractivity::Exclusive,
                output_option: OutputOption::OutputName(m.name.clone()),
                namespace: Some(POPOVER_NAMESPACE.into()),
                ..Default::default()
            },
            id: win,
        }));
        if let Kind::Menu { entries, .. } = &kind {
            tasks.extend(menu_icons(entries).iter().map(|i| self.want_icon(i)));
        }
        tasks.push(self.want_icon("object-select-symbolic"));
        let keyed = keyed_first(&kind, by_keyboard);
        self.popover = Some(Popover {
            win,
            catcher,
            bar,
            position: b.position,
            left,
            origin,
            attached,
            grow: None,
            shown: cfg!(test),
            visibility: Fade {
                from: 0.0,
                to: 1.0,
                start: if capture {
                    now + crate::ui::behind::DELAY
                } else {
                    now
                },
                dur: duration,
            },
            change: None,
            previous: None,
            slide_from: 0.0,
            key,
            anchor_key,
            kind,
            size,
            dashboard: wide.then(continuity::Dashboard::new),
            refreshed: std::time::Instant::now(),
            max_height,
            confirm: None,
            inputs: Default::default(),
            sub: None,
            sub_left: false,
            sub_width: 0.0,
            sub_timer: None,
            sub_gen: 0,
            keyed,
            keyed_sub: None,
        });
        Task::batch(tasks)
    }

    /// Champ de saisie d'un menu (`{ input = "…", secret = true, action = … }`).
    fn pop_input<'a>(
        &'a self,
        alpha: f32,
        i: usize,
        label: Option<&'a str>,
        placeholder: &'a str,
        secret: bool,
    ) -> Element<'a, Msg> {
        let th = &self.cfg.general.theme;
        let fg = color(
            if self.convex_options().is_some() {
                self.bar_palette().fg
            } else {
                th.fg
            },
            alpha,
        );
        let value = self
            .popover
            .as_ref()
            .and_then(|p| p.inputs.get(&i))
            .map(String::as_str)
            .unwrap_or("");
        let mut col = column![].spacing(4).padding([4, 10]);
        if let Some(l) = label {
            col = col.push(
                text(l.to_string())
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .color(Color {
                        a: 0.7 * alpha,
                        ..fg
                    }),
            );
        }
        // Seul le premier champ porte un nom : c'est lui qui reçoit le clavier
        // d'un menu qui commence par un champ.
        let input = iced::widget::text_input(placeholder, value);
        let input = if i == 0 { input.id(FIRST_INPUT) } else { input };
        col.push(
            input
                .secure(secret)
                .on_input(move |s| Msg::Popover(PMsg::Input(i, s)))
                .on_submit(Msg::Popover(PMsg::Submit(i)))
                .font(self.font)
                .size(crate::ui::tokens::ts(13.0))
                .padding(8)
                .style(move |theme, status| {
                    let mut style = iced::widget::text_input::default(theme, status);
                    style.background = style.background.scale_alpha(alpha);
                    style.border.color.a *= alpha;
                    style.icon.a *= alpha;
                    style.placeholder.a *= alpha;
                    style.value.a *= alpha;
                    style.selection.a *= alpha;
                    style
                }),
        )
        .into()
    }

    /// Menu d'une carte de notification (HIG ENT-07) ouvert sur la surface
    /// `win` de la carte de la barre : il part avec elle.
    fn menu_on_card(&mut self, win: Id) -> Task<Msg> {
        if self.icon_menu.as_ref().is_some_and(|m| m.desk == win) {
            self.close_icon_menu()
        } else {
            Task::none()
        }
    }

    pub(in crate::ui) fn destroy_popover(&mut self) -> Task<Msg> {
        self.cal_months = 0;
        match self.popover.take() {
            Some(p) => Task::batch([
                self.menu_on_card(p.win),
                iced::window::close(p.win),
                iced::window::close(p.catcher),
            ]),
            None => Task::none(),
        }
    }

    pub(in crate::ui) fn close_popover(&mut self) -> Task<Msg> {
        let duration = self.bar_duration(
            self.convex_options()
                .filter(|o| o.attached_panels)
                .map_or(120, |o| o.duration as u64),
        );
        if duration.is_zero() {
            return self.destroy_popover();
        }
        if let Some(p) = self.popover.as_mut()
            && !p.closing()
        {
            let now = std::time::Instant::now();
            p.visibility = Fade {
                from: p.visibility.value(now),
                to: 0.0,
                start: now,
                dur: duration,
            };
        }
        Task::none()
    }

    pub(in crate::ui) fn popover_frame(&mut self) -> Task<Msg> {
        if self
            .popover
            .as_ref()
            .is_some_and(|p| !p.shown && !p.closing())
        {
            return Task::none();
        }
        let mut resize = Task::none();
        let now = std::time::Instant::now();
        let still = self.bar_duration(180).is_zero();
        if self
            .popover
            .as_ref()
            .is_some_and(|p| p.closing() && (still || p.visibility.done(now)))
        {
            return self.destroy_popover();
        }
        if let Some(p) = self.popover.as_mut() {
            if p.change.is_some_and(|f| still || f.done(now)) {
                p.previous = None;
                p.change = None;
            }
            if let Some(g) = p.grow
                && (still || g.fade.done(now))
            {
                p.grow = None;
                if (p.size.height - g.to).abs() > 0.5 {
                    p.size.height = g.to;
                    resize = Task::done(Msg::SizeChange {
                        id: p.win,
                        size: (p.size.width as u32, p.size.height as u32),
                    });
                }
            }
            if p.change.is_none()
                && let Some(a) = p.attached
                && (p.size.height - a.height).abs() > 0.5
            {
                p.size.height = a.height;
                resize = Task::done(Msg::SizeChange {
                    id: p.win,
                    size: (p.size.width as u32, p.size.height as u32),
                });
            }
            if still || p.visibility.done(now) {
                p.visibility.from = p.visibility.to;
                p.visibility.dur = std::time::Duration::ZERO;
            }
        }
        Task::batch([resize, self.refresh_popover_anchor()])
    }

    pub(in crate::ui) fn popover_update(&mut self, m: PMsg) -> Task<Msg> {
        // Pied de la vue étendue (HIG SYS-13) : la carte demandée prend la
        // place de la vue, sous la même ancre — le contenu se fond, et
        // recliquer sur la pilule referme.
        if matches!(m, PMsg::ActivityHistory | PMsg::ActivityCalendar) {
            let open = self
                .popover
                .as_ref()
                .filter(|p| !p.closing())
                .map(|p| (p.bar, p.key.clone()));
            return match (open, m) {
                (Some((bar, key)), PMsg::ActivityHistory) => {
                    self.open_popover(bar, key, Kind::Notifications)
                }
                (Some((bar, key)), PMsg::ActivityCalendar) => {
                    self.open_popover(bar, key, Kind::Calendar)
                }
                _ => Task::none(),
            };
        }
        let Some(p) = self.popover.as_mut() else {
            return Task::none();
        };
        match m {
            // Traités avant la garde : la carte y est forcément ouverte.
            PMsg::ActivityHistory | PMsg::ActivityCalendar => Task::none(),
            PMsg::Spawn(cmd) => {
                let t = self.run_from_desktop(cmd, false);
                Task::batch([t, self.close_popover()])
            }
            PMsg::Input(i, s) => {
                let old = p.inputs.insert(i, s);
                if let Some(mut old) = old {
                    crate::lock::pam::wipe(&mut old);
                }
                Task::none()
            }
            PMsg::Submit(i) => {
                let Kind::Menu { entries, source } = &p.kind else {
                    return Task::none();
                };
                let Some(MenuEntry::Input {
                    action: Some(action),
                    ..
                }) = entries.get(i)
                else {
                    return Task::none();
                };
                let (action, source) = (*action, source.clone());
                // Le texte part avec sa commande ; il n'en reste rien ici.
                let text = p.inputs.remove(&i).unwrap_or_default();
                let t = match source {
                    MenuSource::Lua { widget } => {
                        let lua = self.lua.clone();
                        Task::perform(
                            async move {
                                if let Err(e) = lua.menu_action(action as usize, Some(text)).await {
                                    tracing::warn!("{}", crate::tr!("menu : {e}", e = e));
                                }
                                widget
                            },
                            Msg::Repoll,
                        )
                    }
                    // Menu « Aide » : le champ de recherche ouvre celui des
                    // Réglages sur les mots saisis (HIG MENU-05).
                    MenuSource::Shell => super::menus::submit(action, &text),
                    MenuSource::Tray { .. }
                    | MenuSource::Window { .. }
                    | MenuSource::Windows { .. }
                    | MenuSource::Silence
                    | MenuSource::Module => Task::none(),
                };
                Task::batch([t, self.close_popover()])
            }
            PMsg::CancelConfirm => {
                p.confirm = None;
                self.resize_popover()
            }
            PMsg::MenuHover(target) => {
                // Survol d'une ligne de sous-menu : ouverte après `SUB_OPEN`.
                // Autre ligne, ou sortie de la carte : fermée (ou changée)
                // après `SUB_CLOSE`, le temps d'atteindre la colonne.
                if !matches!(p.kind, Kind::Menu { .. }) || p.sub == target {
                    p.sub_timer = None;
                    return Task::none();
                }
                let (delay, target) = match (p.sub, target) {
                    (None, Some(_)) => (SUB_OPEN, target),
                    (Some(_), _) => (SUB_CLOSE, target),
                    // Rien d'ouvert : une ouverture en attente est annulée.
                    (None, None) => {
                        p.sub_timer = None;
                        return Task::none();
                    }
                };
                p.sub_gen += 1;
                let token = p.sub_gen;
                p.sub_timer = Some((token, target));
                Task::perform(
                    // La pause naît dans la tâche : la mise à jour reste hors réacteur.
                    async move {
                        tokio::time::sleep(delay).await;
                    },
                    move |_| Msg::Popover(PMsg::MenuTick(token)),
                )
            }
            PMsg::MenuLeave => self.popover_update(PMsg::MenuHover(None)),
            PMsg::MenuEnter => {
                p.sub_timer = None;
                Task::none()
            }
            PMsg::MenuTick(token) => {
                if p.sub_timer.is_some_and(|(armed, _)| armed == token)
                    && let Some((_, target)) = p.sub_timer.take()
                    && p.sub != target
                {
                    p.sub = target;
                    return self.refresh_popover_anchor();
                }
                Task::none()
            }
            PMsg::Activate(i) => {
                let Kind::Menu { entries, .. } = &p.kind else {
                    return Task::none();
                };
                // Ligne de sous-menu : un clic l'ouvre aussitôt (écran
                // tactile, clavier).
                if let Some(MenuEntry::Submenu {
                    enabled: true,
                    children,
                    ..
                }) = entries.get(i)
                {
                    p.sub_timer = None;
                    if p.sub != Some(i) && !children.is_empty() {
                        p.sub = Some(i);
                        return self.refresh_popover_anchor();
                    }
                    return Task::none();
                }
                let Some(MenuEntry::Item {
                    enabled: true,
                    confirm,
                    ..
                }) = entries.get(i)
                else {
                    return Task::none();
                };
                if confirm.is_some() && p.confirm != Some(i) {
                    p.confirm = Some(i);
                    return self.resize_popover();
                }
                self.popover_update(PMsg::Confirm(i))
            }
            PMsg::Confirm(i) => {
                let Kind::Menu { entries, source } = &p.kind else {
                    return Task::none();
                };
                let Some(MenuEntry::Item {
                    action: Some(action),
                    ..
                }) = entries.get(i)
                else {
                    return self.close_popover();
                };
                let action = *action;
                let source = source.clone();
                self.menu_action(source, action)
            }
            PMsg::SubActivate(j) => {
                let Some((source, action)) = self.sub_action(j) else {
                    return Task::none();
                };
                self.menu_action(source, action)
            }
        }
    }

    /// Touche reçue par un menu de la barre (HIG ACC-01, ENT-01) : flèches
    /// haut et bas, Tab et Maj + Tab, Début et Fin choisissent une entrée ;
    /// flèche droite ouvre un sous-menu et descend dans sa colonne, flèche
    /// gauche en remonte (ou referme le sous-menu ouvert) ; Entrée ou Espace
    /// l'activent, comme un clic. Menu à champ de saisie (Aide) : le champ
    /// garde ses touches ; flèches haut et bas passent aux entrées, le champ
    /// rend son focus, Tab y revient. Rien (`None`) pour une autre surface ou
    /// une autre touche : Échap garde son chemin.
    pub(in crate::ui) fn popover_menu_key(
        &mut self,
        id: Id,
        key: &iced::keyboard::Key,
        shift: bool,
    ) -> Option<Task<Msg>> {
        use iced::keyboard::{Key, key::Named};
        let p = self
            .popover
            .as_ref()
            .filter(|p| p.win == id && !p.closing())?;
        let Kind::Menu { entries, .. } = &p.kind else {
            return None;
        };
        let field = !keyboard_menu(entries);
        let Key::Named(name) = key else {
            return None;
        };
        // Le champ tient le clavier : seules les flèches haut et bas en
        // sortent, vers les entrées.
        if field && p.keyed.is_none() {
            let forward = match name {
                Named::ArrowDown => true,
                Named::ArrowUp => false,
                _ => return None,
            };
            let next = crate::ui::menu::step_where(entries.len(), None, forward, |i| {
                keyable(&entries[i], false)
            })?;
            self.popover.as_mut()?.keyed = Some(next);
            return Some(iced::advanced::widget::operate(
                iced::advanced::widget::operation::focusable::unfocus(),
            ));
        }
        // Tab ramène au champ.
        if field && *name == Named::Tab {
            let p = self.popover.as_mut()?;
            p.keyed = None;
            p.keyed_sub = None;
            let close = p.sub.take().is_some();
            p.sub_timer = None;
            let back = focus_first_input(&p.kind);
            return Some(if close {
                Task::batch([back, self.refresh_popover_anchor()])
            } else {
                back
            });
        }
        // La colonne du sous-menu ouvert, si le clavier y est descendu.
        let column = p
            .keyed_sub
            .filter(|(parent, _)| p.sub == Some(*parent))
            .and_then(|(parent, j)| match entries.get(parent) {
                Some(MenuEntry::Submenu { children, .. }) => Some((parent, j, children)),
                _ => None,
            });
        let (list, from): (&[MenuEntry], Option<usize>) = match column {
            Some((_, j, children)) => (children, Some(j)),
            None => (entries, p.keyed),
        };
        let in_column = column.is_some();
        let step = |from, forward| {
            crate::ui::menu::step_where(list.len(), from, forward, |i| keyable(&list[i], in_column))
        };
        let submenu_at = |i: usize| matches!(list.get(i), Some(e @ MenuEntry::Submenu { .. }) if keyable(e, false));
        let next = match name {
            Named::Tab => step(from, !shift),
            Named::ArrowDown => step(from, true),
            Named::ArrowUp => step(from, false),
            Named::Home | Named::PageUp => step(None, true),
            Named::End | Named::PageDown => step(None, false),
            // Dans la colonne, ou sous-menu ouvert au survol : il se referme,
            // sa ligne reprend le choix.
            Named::ArrowLeft if in_column || p.sub.is_some() => {
                let parent = column.map(|(parent, ..)| parent).or(p.sub);
                let p = self.popover.as_mut()?;
                p.keyed_sub = None;
                p.keyed = parent;
                p.sub = None;
                p.sub_timer = None;
                return Some(self.refresh_popover_anchor());
            }
            Named::ArrowRight if !in_column => {
                return Some(match from.filter(|&i| submenu_at(i)) {
                    Some(i) => self.popover_open_column(i),
                    None => Task::none(),
                });
            }
            Named::Enter | Named::Space => {
                return Some(match (column, from) {
                    (Some((_, j, _)), _) => self.popover_update(PMsg::SubActivate(j)),
                    (None, Some(i)) if submenu_at(i) => self.popover_open_column(i),
                    (None, Some(i)) => self.popover_update(PMsg::Activate(i)),
                    (None, None) => Task::none(),
                });
            }
            _ => return None,
        };
        let parent = column.map(|(parent, ..)| parent);
        let p = self.popover.as_mut()?;
        if let Some(parent) = parent {
            p.keyed_sub = next.map(|j| (parent, j));
            return Some(Task::none());
        }
        p.keyed = next;
        // Une autre ligne de la liste : le sous-menu ouvert se referme.
        if p.sub.is_some() && p.sub != next {
            p.sub = None;
            p.sub_timer = None;
            p.keyed_sub = None;
            return Some(self.refresh_popover_anchor());
        }
        Some(Task::none())
    }

    /// Le sous-menu de la ligne `i` s'ouvre, et le clavier descend à la
    /// première entrée de sa colonne.
    fn popover_open_column(&mut self, i: usize) -> Task<Msg> {
        let Some(p) = self.popover.as_mut() else {
            return Task::none();
        };
        let Kind::Menu { entries, .. } = &p.kind else {
            return Task::none();
        };
        let Some(MenuEntry::Submenu { children, .. }) = entries.get(i) else {
            return Task::none();
        };
        let first = crate::ui::menu::step_where(children.len(), None, true, |j| {
            keyable(&children[j], true)
        });
        p.keyed = Some(i);
        p.keyed_sub = first.map(|j| (i, j));
        p.sub_timer = None;
        if p.sub == Some(i) {
            return Task::none();
        }
        p.sub = Some(i);
        self.refresh_popover_anchor()
    }

    /// Action de l'entrée n de la colonne ouverte. Rien si la ligne est
    /// inactive ou sans action ; une confirmation ne s'ouvre que dans la
    /// liste parente : une telle entrée reste une ligne d'information.
    fn sub_action(&self, j: usize) -> Option<(MenuSource, i64)> {
        let p = self.popover.as_ref()?;
        let Kind::Menu { entries, source } = &p.kind else {
            return None;
        };
        let MenuEntry::Submenu { children, .. } = entries.get(p.sub?)? else {
            return None;
        };
        match children.get(j)? {
            MenuEntry::Item {
                enabled: true,
                action: Some(action),
                confirm: None,
                ..
            } => Some((source.clone(), *action)),
            _ => None,
        }
    }

    /// Lance l'action `action` d'un menu (`source`), puis referme la carte.
    fn menu_action(&mut self, source: MenuSource, action: i64) -> Task<Msg> {
        let t = match source {
            MenuSource::Lua { widget } => {
                let lua = self.lua.clone();
                Task::perform(
                    async move {
                        if let Err(e) = lua.menu_action(action as usize, None).await {
                            tracing::warn!("{}", crate::tr!("menu : {e}", e = e));
                        }
                        widget
                    },
                    Msg::Repoll,
                )
            }
            MenuSource::Tray { bus, menu_path } => Task::perform(
                async move { crate::tray::menu_event(&bus, &menu_path, action as i32).await },
                |_| Msg::Noop,
            ),
            MenuSource::Window { address } => Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        if let Err(e) = super::window::act(&address, action) {
                            tracing::warn!("{}", crate::tr!("fenêtre : {e}", e = e));
                        }
                    })
                    .await
                },
                |_| Msg::Noop,
            ),
            // Menu « Fenêtres » : le focus va à la fenêtre, son espace suit ;
            // réduite, elle revient d'abord sur l'espace affiché.
            MenuSource::Windows { addresses } => {
                let Some(address) = usize::try_from(action)
                    .ok()
                    .and_then(|i| addresses.get(i).cloned())
                else {
                    return self.close_popover();
                };
                Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || {
                            if let Err(e) = crate::hypr::focus_window(&address) {
                                tracing::warn!("{}", crate::tr!("fenêtre : {e}", e = e));
                            }
                        })
                        .await
                    },
                    |_| Msg::Noop,
                )
            }
            // Menu « hyprdesk » (HIG MENU-01).
            MenuSource::Shell => super::menus::act(self, action),
            MenuSource::Silence => {
                // Fin choisie du Silence (HIG PAT-04) : le rang de
                // l'entrée dit la fin.
                self.toasts
                    .command(&crate::ui::toasts::silence_command(action));
                Task::none()
            }
            // Menu contextuel d'un module de barre (HIG ENT-07).
            MenuSource::Module => super::menus::module_act(self, action),
        };
        Task::batch([t, self.close_popover()])
    }

    fn resize_popover(&mut self) -> Task<Msg> {
        // Le prochain rendu mesure le contenu, y compris les confirmations et les retours à la ligne.
        Task::none()
    }

    pub(in crate::ui) fn fit_popover(&mut self, win: Id, content_height: f32) -> Task<Msg> {
        if !content_height.is_finite() || content_height <= 0.0 {
            return Task::none();
        }
        let dur = self.bar_duration(160);
        let Some(p) = self
            .popover
            .as_mut()
            .filter(|p| p.win == win && p.dashboard.is_none())
        else {
            return Task::none();
        };
        let header = p
            .attached
            .filter(|a| matches!(a.edge, Edge::Top | Edge::Bottom))
            .map_or(0.0, |a| a.bar_height);
        let height = (content_height + 24.0 + 2.0 * SHADOW + header)
            .ceil()
            .min(p.max_height);
        if (height - p.target_height()).abs() < 1.0 {
            return Task::none();
        }
        if let Some(a) = p.attached.as_mut() {
            let mix = p.change.map_or(1.0, |f| f.value(std::time::Instant::now()));
            a.from_height = a.from_height + (a.height - a.from_height) * mix;
            a.height = height;
            p.size.height = if p.change.is_some() {
                p.size.height.max(height)
            } else {
                height
            };
        } else {
            p.retarget(height, std::time::Instant::now(), dur);
        }
        let resize = Task::done(Msg::SizeChange {
            id: win,
            size: (p.size.width as u32, p.size.height as u32),
        });
        Task::batch([resize, self.refresh_popover_anchor()])
    }

    pub(in crate::ui) fn view_catcher(&self) -> Element<'_, Msg> {
        mouse_area(
            container(Space::new())
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(Msg::ClosePopover)
        .on_right_press(Msg::ClosePopover)
        .into()
    }

    fn pop_content<'a>(
        &'a self,
        kind: &'a Kind,
        confirm: Option<usize>,
        alpha: f32,
    ) -> Element<'a, Msg> {
        let content = match kind {
            Kind::Notifications => self.view_notification_history(alpha),
            Kind::Activity(id) => self.view_activity(alpha, *id),
            Kind::Volume => self.pop_volume(alpha),
            Kind::Brightness => self.pop_brightness(alpha),
            Kind::Calendar => self.pop_calendar(alpha),
            Kind::Widget(id) => match self
                .cfg
                .widgets
                .iter()
                .find(|w| &w.id == id)
                .map(|w| &w.kind)
            {
                Some(crate::config::WidgetKind::Controls) => {
                    let bar = self.popover.as_ref().map(|p| p.bar);
                    let monitor = bar
                        .and_then(|id| self.bar_surfaces.get(&id))
                        .map_or("", |b| b.monitor.as_str());
                    self.controls_view(alpha, monitor, bar)
                }
                _ => Space::new().into(),
            },
            Kind::Workspaces => self.pop_workspaces(alpha),
            Kind::Overflow(items) => self.pop_overflow(alpha, items),
            Kind::Menu { entries, .. } => {
                let sub = self.popover.as_ref().and_then(|p| p.sub);
                self.pop_menu_card(alpha, entries, confirm, sub)
            }
        };
        let fg = color(
            if self.convex_options().is_some() {
                self.bar_palette().fg
            } else {
                self.cfg.general.theme.fg
            },
            alpha,
        );
        container(content)
            .style(move |_| container::Style {
                text_color: Some(fg),
                ..Default::default()
            })
            .into()
    }

    pub(in crate::ui) fn popover_shown(&mut self, id: Id) -> Task<Msg> {
        // Carte nouvelle : son champ n'existait pas à l'ouverture ; il prend
        // le clavier une fois la carte affichée.
        match self.popover.as_mut().filter(|p| p.win == id && !p.shown) {
            Some(p) => {
                p.shown = true;
                if p.visibility.to == 1.0 {
                    p.visibility.from = 0.0;
                    p.visibility.start = std::time::Instant::now();
                }
                focus_first_input(&p.kind)
            }
            None => Task::none(),
        }
    }

    pub(in crate::ui) fn view_popover(&self) -> Element<'_, Msg> {
        let Some(p) = &self.popover else {
            return Space::new().into();
        };
        let now = std::time::Instant::now();
        let still = self.bar_duration(180).is_zero();
        let alpha = if !p.shown {
            0.0
        } else if still {
            p.visibility.to
        } else {
            p.visibility.value(now)
        };
        let mix = if still {
            1.0
        } else {
            p.change.map_or(1.0, |f| f.value(now))
        };
        let th = &self.cfg.general.theme;
        let fg = color(
            if self.convex_options().is_some() {
                self.bar_palette().fg
            } else {
                th.fg
            },
            alpha,
        );
        let bg = color(
            crate::ui::tokens::tinted(th.bg, crate::config::Place::Menus),
            th.shown(crate::config::Place::Menus).opacity * alpha,
        );
        let win = p.win;
        let content = || {
            let content_alpha = if p.attached.is_some() { 1.0 } else { alpha };
            let current =
                iced::widget::sensor(self.pop_content(&p.kind, p.confirm, content_alpha * mix))
                    .key(win)
                    .on_resize(move |size| Msg::PopoverMeasured(win, size.height));
            let mut stack = iced::widget::stack![];
            if let Some(previous) = &p.previous {
                stack = stack.push(crate::ui::plugins::bar::motion::inert(self.pop_content(
                    previous,
                    None,
                    content_alpha * (1.0 - mix),
                )));
            }
            stack.push(current)
        };
        if let Some(a) = p.attached {
            let header = if matches!(a.edge, Edge::Top | Edge::Bottom) {
                a.bar_height
            } else {
                0.0
            };
            let shape_height = a.from_height + (a.height - a.from_height) * mix;
            let body = container(iced::widget::scrollable(content()).height(Length::Fill))
                .padding(Padding {
                    top: 24.0 + header,
                    bottom: 24.0,
                    left: 36.0,
                    right: 36.0,
                })
                .width(Length::Fill)
                .height(a.height);
            let body: Element<'_, Msg> = if p.closing() {
                crate::ui::plugins::bar::motion::inert(body)
            } else {
                body.into()
            };
            let pal = self.bar_palette();
            let track = if p.shown {
                p.visibility
            } else {
                crate::ui::Fade {
                    from: 0.0,
                    to: 0.0,
                    start: p.visibility.start,
                    dur: std::time::Duration::ZERO,
                }
            };
            let shape = container(crate::ui::kinetic::reveal(
                body,
                crate::ui::kinetic::Track::Ease(track),
                a.edge,
                color(pal.bg, pal.opacity),
                p.shown.then_some(Msg::MotionSettled),
            ))
            .width(Length::Fill)
            .height(shape_height)
            .clip(true);
            return iced::widget::sensor(
                container(shape)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .align_y(match a.edge {
                        Edge::Top => Alignment::Start,
                        Edge::Bottom => Alignment::End,
                        _ => Alignment::Center,
                    })
                    .clip(true),
            )
            .key(win)
            .on_show(move |_| Msg::PopoverShown(win))
            .into();
        }
        // Verre du thème (MLG-268) au rayon de la carte ; vraie réfraction de
        // l'écran capturé à l'ouverture (MLG-275).
        let mut style = crate::ui::design::bar_card(bg, fg);
        style.border.color.a *= alpha;
        style.shadow.color.a *= alpha;
        let card: Element<'_, Msg> = if let Some(dashboard) = &p.dashboard {
            // Centre de Continuité : ses tuiles sur le verre des menus.
            crate::ui::glass::card(
                self.continuity_dashboard(p, dashboard, alpha),
                crate::config::Place::Menus,
                crate::ui::design::RADIUS,
                fg,
                bg,
                style,
                true,
                self.behind_backdrop(win),
            )
        } else {
            // Pendant un changement de taille (HIG PRES-02), le contenu qui
            // dépasse brièvement le cadre ne fait pas clignoter la barre de
            // défilement ; le widget reste le même, son état aussi.
            let bar = if p.grow.is_some() {
                iced::widget::scrollable::Scrollbar::hidden()
            } else {
                iced::widget::scrollable::Scrollbar::new()
            };
            crate::ui::glass::card(
                container(
                    iced::widget::scrollable(content())
                        .direction(iced::widget::scrollable::Direction::Vertical(bar))
                        .height(Length::Fill),
                )
                .padding(12)
                .width(Length::Fill)
                .height(Length::Fill),
                crate::config::Place::Menus,
                crate::ui::design::RADIUS,
                fg,
                bg,
                style,
                true,
                self.behind_backdrop(win),
            )
        };
        let card = if p.closing() {
            crate::ui::plugins::bar::motion::inert(card)
        } else {
            card
        };
        let direction = if p.position == BarPosition::Top {
            -1.0
        } else {
            1.0
        };
        let origin = if p.position == BarPosition::Top {
            iced::Point::new(p.origin, 0.0)
        } else {
            iced::Point::new(p.origin, 1.0)
        };
        // Mouvement du thème (MLG-271) : échelle et déplacement de départ.
        let mo = crate::ui::tokens::motion();
        let scale = mo.scale + (1.0 - mo.scale) * alpha;
        // Changement de taille (HIG PRES-02) : le cadre a sa hauteur
        // interpolée, collé au bord où la surface est ancrée.
        let framed = container(crate::ui::plugins::bar::motion::surface(
            card,
            scale,
            Vector::new(
                p.slide_from * (1.0 - mix),
                direction * mo.offset * (1.0 - alpha),
            ),
            origin,
        ))
        .padding(SHADOW)
        .width(Length::Fill)
        .height(if p.grow.is_some() {
            Length::Fixed(p.drawn_height(now))
        } else {
            Length::Fill
        });
        iced::widget::sensor(
            container(framed)
                .width(Length::Fill)
                .height(Length::Fill)
                .align_y(if p.position == BarPosition::Bottom {
                    Alignment::End
                } else {
                    Alignment::Start
                }),
        )
        .key(win)
        .on_show(move |_| Msg::PopoverShown(win))
        .into()
    }

    /// Encre des menus de la barre : la palette de la barre convexe, sinon le thème.
    pub(in crate::ui) fn pop_ink(&self, alpha: f32) -> crate::ui::menu::Ink {
        let mut ink = self.menu_ink(alpha);
        if self.convex_options().is_some() {
            ink.fg = color(self.bar_palette().fg, 1.0);
        }
        ink
    }

    pub(in crate::ui) fn pop_header(&self, alpha: f32, label: String) -> Element<'_, Msg> {
        self.menu_header(self.pop_ink(alpha), label)
    }

    pub(in crate::ui) fn pop_separator(&self, alpha: f32) -> Element<'_, Msg> {
        self.menu_separator(self.pop_ink(alpha))
    }

    /// Ligne de menu (composant commun, `crate::ui::menu`). `enabled` à faux :
    /// entrée grisée ; sans action (`clickable` faux) : ligne d'information,
    /// lisible mais sans survol.
    #[allow(clippy::too_many_arguments)]
    fn pop_row<'a>(
        &'a self,
        alpha: f32,
        lead: Element<'a, Msg>,
        label: String,
        detail: Option<String>,
        enabled: bool,
        clickable: bool,
        msg: Option<Msg>,
    ) -> Element<'a, Msg> {
        let st = crate::ui::menu::RowState {
            enabled,
            clickable,
            ..Default::default()
        };
        self.menu_row(self.pop_ink(alpha), lead, label, detail, st, msg)
    }

    /// Menu et sa colonne de sous-menu ouverte (HIG MENU-03). La liste garde
    /// sa largeur et sa place ; la colonne part à la hauteur de sa ligne, à
    /// droite, ou à gauche quand le bord droit de l'écran était trop proche.
    /// Sans sous-menu dans la liste : la liste seule, inchangée.
    fn pop_menu_card<'a>(
        &'a self,
        alpha: f32,
        entries: &'a [MenuEntry],
        confirm: Option<usize>,
        sub: Option<usize>,
    ) -> Element<'a, Msg> {
        let open = sub.is_some_and(|i| {
            matches!(
                entries.get(i),
                Some(MenuEntry::Submenu { children, .. }) if !children.is_empty()
            )
        });
        let sub = sub.filter(|_| open);
        // Entrées choisies au clavier : dans la liste, et dans la colonne du
        // sous-menu ouvert.
        let (keyed, keyed_sub) = self.popover.as_ref().map_or((None, None), |p| {
            (
                p.keyed,
                p.keyed_sub
                    .filter(|(parent, _)| sub == Some(*parent))
                    .map(|(_, j)| j),
            )
        });
        let list = self.pop_menu(alpha, entries, confirm, sub, keyed, false);
        if !entries
            .iter()
            .any(|e| matches!(e, MenuEntry::Submenu { .. }))
        {
            return list;
        }
        let enter = Msg::Popover(PMsg::MenuEnter);
        let leave = Msg::Popover(PMsg::MenuLeave);
        let Some((i, MenuEntry::Submenu { children, .. })) = sub.map(|i| (i, &entries[i])) else {
            return mouse_area(list).on_enter(enter).on_exit(leave).into();
        };
        let p = self.popover.as_ref().expect("carte d'un menu ouvert");
        let inner = p.size.width
            - if p.attached.is_some() {
                72.0
            } else {
                24.0 + 2.0 * SHADOW
            };
        let column = container(self.pop_menu(alpha, children, None, None, keyed_sub, true))
            .width(p.sub_width)
            .padding(Padding {
                top: entry_top(entries, i, confirm),
                right: 0.0,
                bottom: 0.0,
                left: 0.0,
            });
        // Écart de la colonne à la liste, du côté où elles se joignent.
        let gap = Padding {
            top: 0.0,
            right: if p.sub_left { SUB_GAP } else { 0.0 },
            bottom: 0.0,
            left: if p.sub_left { 0.0 } else { SUB_GAP },
        };
        // La liste garde la largeur qu'elle avait sans colonne.
        let list_w = inner - SUB_GAP - p.sub_width;
        let column = container(column).padding(gap);
        let list = container(list).width(Length::Fixed(list_w));
        let content: Element<'_, Msg> = if p.sub_left {
            row![column, list].into()
        } else {
            row![list, column].into()
        };
        mouse_area(content).on_enter(enter).on_exit(leave).into()
    }

    /// Liste d'entrées d'un menu. `sub` : ligne de sous-menu ouverte (surlignée
    /// tant que sa colonne l'est) ; les lignes signalent leur survol. `column`
    /// : cette liste est la colonne d'un sous-menu (ses entrées s'activent
    /// par `PMsg::SubActivate`). `keyed` : l'entrée choisie au clavier,
    /// surlignée comme au survol.
    #[allow(clippy::too_many_arguments)]
    fn pop_menu<'a>(
        &'a self,
        alpha: f32,
        entries: &'a [MenuEntry],
        confirm: Option<usize>,
        sub: Option<usize>,
        keyed: Option<usize>,
        column: bool,
    ) -> Element<'a, Msg> {
        let th = &self.cfg.general.theme;
        let fg = color(
            if self.convex_options().is_some() {
                self.bar_palette().fg
            } else {
                th.fg
            },
            1.0 * alpha,
        );
        let mut col = column![].spacing(0);
        if entries.is_empty() {
            col = col.push(self.pop_row(
                alpha,
                Space::new().into(),
                crate::i18n::tr("Rien à afficher").into(),
                None,
                false,
                false,
                None,
            ));
        }
        let hover = !column
            && entries
                .iter()
                .any(|e| matches!(e, MenuEntry::Submenu { .. }));
        for (i, e) in entries.iter().enumerate() {
            let row: Element<'_, Msg> = match e {
                MenuEntry::Header(h) => self.pop_header(alpha, h.clone()),
                MenuEntry::Image(p) => container(
                    iced::widget::image(iced::widget::image::Handle::from_path(p))
                        .width(IMAGE)
                        .height(IMAGE)
                        .opacity(alpha)
                        .content_fit(iced::ContentFit::Cover)
                        .border_radius(10),
                )
                .center_x(Length::Fill)
                .padding([6, 0])
                .into(),
                MenuEntry::Submenu {
                    label,
                    icon,
                    enabled,
                    children,
                } => {
                    let lead: Element<'_, Msg> = icon
                        .as_deref()
                        .and_then(|name| self.icon_named(name, crate::ui::tokens::glyph(16.0), fg))
                        .unwrap_or_else(|| Space::new().into());
                    let live = *enabled && !children.is_empty();
                    let st = crate::ui::menu::RowState {
                        enabled: *enabled,
                        clickable: live,
                        submenu: true,
                        // Surlignée tant que sa colonne est ouverte, ou
                        // choisie au clavier.
                        selected: sub == Some(i) || keyed == Some(i),
                        ..Default::default()
                    };
                    self.menu_row(
                        self.pop_ink(alpha),
                        lead,
                        label.clone(),
                        None,
                        st,
                        live.then_some(Msg::Popover(PMsg::Activate(i))),
                    )
                }
                MenuEntry::Separator => self.pop_separator(alpha),
                MenuEntry::Input {
                    label,
                    placeholder,
                    secret,
                    ..
                } => self.pop_input(alpha, i, label.as_deref(), placeholder, *secret),
                MenuEntry::Item {
                    label,
                    detail,
                    icon,
                    checked,
                    enabled,
                    action,
                    confirm: ask,
                } => {
                    let lead: Element<'_, Msg> = match (checked, icon) {
                        (Some(true), _) => self
                            .icon_named(
                                "object-select-symbolic",
                                crate::ui::tokens::glyph(14.0),
                                fg,
                            )
                            .unwrap_or_else(|| {
                                text("•")
                                    .font(self.font)
                                    .size(crate::ui::tokens::ts(13.0))
                                    .into()
                            }),
                        (_, Some(name)) => self
                            .icon_named(name, crate::ui::tokens::glyph(16.0), fg)
                            .unwrap_or_else(|| Space::new().into()),
                        _ => Space::new().into(),
                    };
                    let st = crate::ui::menu::RowState {
                        enabled: *enabled,
                        clickable: action.is_some() && (!column || ask.is_none()),
                        selected: keyed == Some(i),
                        ..Default::default()
                    };
                    let row_el = self.menu_row(
                        self.pop_ink(alpha),
                        lead,
                        label.clone(),
                        detail.clone(),
                        st,
                        Some(Msg::Popover(if column {
                            PMsg::SubActivate(i)
                        } else {
                            PMsg::Activate(i)
                        })),
                    );
                    if confirm == Some(i) {
                        let question = ask.clone().unwrap_or_else(|| "Confirmer ?".into());
                        let danger = crate::ui::tone::danger();
                        let font = self.ui_font(iced::font::Weight::Medium);
                        column![
                            row_el,
                            row![
                                text(question)
                                    .font(self.font)
                                    .size(crate::ui::tokens::ts(12.0))
                                    .font(font)
                                    .width(Length::Fill),
                                button(
                                    text("Annuler")
                                        .font(self.font)
                                        .size(crate::ui::tokens::ts(12.0))
                                        .font(font)
                                )
                                .padding([4, 10])
                                .on_press(Msg::Popover(PMsg::CancelConfirm))
                                .style(move |_, st| {
                                    button::Style {
                                        background: Some(Background::Color(Color {
                                            a: if matches!(st, button::Status::Hovered) {
                                                0.18
                                            } else {
                                                0.10
                                            },
                                            ..fg
                                        })),
                                        text_color: fg,
                                        border: Border::default()
                                            .rounded(crate::ui::design::SMALL_RADIUS),
                                        ..Default::default()
                                    }
                                }),
                                button(
                                    text(confirm_label(label))
                                        .font(self.font)
                                        .size(crate::ui::tokens::ts(12.0))
                                        .font(font)
                                )
                                .padding([4, 10])
                                .on_press(Msg::Popover(PMsg::Confirm(i)))
                                .style(move |_, st| {
                                    button::Style {
                                        background: Some(Background::Color(Color {
                                            a: if matches!(st, button::Status::Hovered) {
                                                1.0
                                            } else {
                                                0.85
                                            },
                                            ..danger
                                        })),
                                        text_color: Color::WHITE,
                                        border: Border::default()
                                            .rounded(crate::ui::design::SMALL_RADIUS),
                                        ..Default::default()
                                    }
                                }),
                            ]
                            .spacing(6)
                            .align_y(Alignment::Center)
                            .padding([4, 10])
                            .height(40),
                        ]
                        .into()
                    } else {
                        row_el
                    }
                }
            };
            col = col.push(if hover {
                mouse_area(row)
                    .on_enter(Msg::Popover(PMsg::MenuHover(
                        matches!(e, MenuEntry::Submenu { .. }).then_some(i),
                    )))
                    .into()
            } else {
                row
            });
        }
        col.into()
    }

    fn pop_volume(&self, alpha: f32) -> Element<'_, Msg> {
        let a = &self.audio;
        let mut col = column![self.pop_header(alpha, crate::i18n::tr("Sortie").into())];
        col = col.push(self.audio_controls(
            alpha,
            self.volume,
            crate::ui::plugins::bar::surface::volume_icon(self.volume),
            Msg::VolumeSet,
            Msg::VolumeMute,
        ));
        col = col.push(self.device_list(alpha, &a.outputs));
        col = col.push(self.pop_separator(alpha));
        col = col.push(self.pop_header(alpha, crate::i18n::tr("Micro").into()));
        if a.mic.is_some() {
            col = col.push(self.audio_controls(
                alpha,
                a.mic,
                crate::ui::plugins::bar::surface::mic_icon(a.mic),
                Msg::MicSet,
                Msg::MicMute,
            ));
            col = col.push(self.device_list(alpha, &a.inputs));
        } else {
            col = col.push(self.pop_row(
                alpha,
                Space::new().into(),
                crate::i18n::tr("Aucun micro").into(),
                None,
                false,
                false,
                None,
            ));
        }
        col.push(self.pop_separator(alpha))
            .push(self.pop_row(
                alpha,
                Space::new().into(),
                crate::i18n::tr("Réglages audio…").into(),
                None,
                true,
                true,
                Some(Msg::Popover(PMsg::Spawn("pavucontrol".into()))),
            ))
            .into()
    }

    /// Menu Luminosité : un curseur par rétroéclairage et par écran DDC/CI.
    fn pop_brightness(&self, alpha: f32) -> Element<'_, Msg> {
        use crate::ui::plugins::bar::surface::LightTarget;
        let l = &self.light;
        let mut rows: Vec<(String, LightTarget)> = Vec::new();
        if l.screen.is_some() {
            rows.push((crate::i18n::tr("Écran").to_string(), LightTarget::Screen));
        }
        for d in &l.displays {
            rows.push((d.connector.clone(), LightTarget::Ddc(d.bus)));
        }
        if l.kbd.is_some() {
            rows.push((crate::i18n::tr("Clavier").to_string(), LightTarget::Kbd));
        }
        let mut col = column![];
        if rows.is_empty() {
            col = col
                .push(self.pop_header(alpha, crate::i18n::tr("Aucune luminosité réglable").into()));
        }
        for (title, t) in rows {
            col = col.push(self.pop_header(alpha, title));
            col = col.push(self.light_slider(alpha, t));
        }
        col.into()
    }

    pub(in crate::ui) fn light_slider(
        &self,
        alpha: f32,
        t: crate::ui::plugins::bar::surface::LightTarget,
    ) -> Element<'_, Msg> {
        let v = self.light.level(t).unwrap_or(0.0);
        row![
            slider(0.0..=1.0f32, v, move |x| Msg::LightSet(t, x))
                .on_release(Msg::LightApply(t))
                .step(0.01f32)
                .style(move |theme, status| crate::ui::design::bar_slider_alpha(
                    theme, status, alpha
                ))
                .width(Length::Fill),
            text(format!("{:.0} %", v * 100.0))
                .size(crate::ui::tokens::ts(12.0))
                .font(self.ui_font(iced::font::Weight::Medium))
                .width(40)
                .align_x(Alignment::End),
        ]
        .spacing(10)
        .align_y(Alignment::Center)
        .padding([6, 10])
        .height(32)
        .into()
    }

    /// Curseur de volume et interrupteur « Muet » d'un périphérique.
    fn audio_controls(
        &self,
        alpha: f32,
        value: Option<(f32, bool)>,
        icon: &str,
        set: fn(f32) -> Msg,
        mute: Msg,
    ) -> Element<'_, Msg> {
        let fg = color(
            if self.convex_options().is_some() {
                self.bar_palette().fg
            } else {
                self.cfg.general.theme.fg
            },
            1.0 * alpha,
        );
        let (v, muted) = value.unwrap_or((0.0, false));
        let font = self.ui_font(iced::font::Weight::Medium);
        column![
            row![
                // Trait de la valeur en Medium (HIG ICO-12).
                self.icon_named_weighted(icon, crate::ui::tokens::glyph(16.0), fg, font.weight)
                    .unwrap_or_else(|| Space::new().into()),
                slider(0.0..=1.5f32, v.min(1.5), set)
                    .step(0.01f32)
                    .style(move |theme, status| crate::ui::design::bar_slider_alpha(
                        theme, status, alpha
                    ))
                    .width(Length::Fill),
                text(format!("{:.0} %", v * 100.0))
                    .font(self.font)
                    .size(crate::ui::tokens::ts(12.0))
                    .font(font)
                    .width(40)
                    .align_x(Alignment::End),
            ]
            .spacing(10)
            .align_y(Alignment::Center)
            .padding([6, 10])
            .height(32),
            row![
                text("Muet")
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
                    .font(self.ui_font(iced::font::Weight::Normal))
                    .width(Length::Fill),
                toggler(muted)
                    .on_toggle(move |_| mute.clone())
                    .style(move |theme, status| {
                        let mut style = toggler::default(theme, status);
                        style.background = style.background.scale_alpha(alpha);
                        style.foreground = style.foreground.scale_alpha(alpha);
                        style.background_border_color.a *= alpha;
                        style.foreground_border_color.a *= alpha;
                        style.text_color = style.text_color.map(|c| c.scale_alpha(alpha));
                        style
                    })
            ]
            .align_y(Alignment::Center)
            .padding([4, 10])
            .height(30),
        ]
        .into()
    }

    /// Choix du périphérique par défaut (coché), s'il y en a plusieurs.
    fn device_list<'a>(
        &'a self,
        alpha: f32,
        list: &'a [crate::ui::plugins::bar::surface::AudioDevice],
    ) -> Element<'a, Msg> {
        if list.len() < 2 {
            return Space::new().into();
        }
        let fg = color(
            if self.convex_options().is_some() {
                self.bar_palette().fg
            } else {
                self.cfg.general.theme.fg
            },
            1.0 * alpha,
        );
        column(list.iter().map(|d| {
            let lead = if d.default {
                self.icon_named("object-select-symbolic", crate::ui::tokens::glyph(14.0), fg)
                    .unwrap_or_else(|| Space::new().into())
            } else {
                Space::new().into()
            };
            self.pop_row(
                alpha,
                lead,
                crate::ui::plugins::bar::surface::truncate(&d.name, 28),
                None,
                true,
                !d.default,
                Some(Msg::AudioDefault(d.id)),
            )
        }))
        .into()
    }

    fn pop_calendar(&self, alpha: f32) -> Element<'_, Msg> {
        let th = &self.cfg.general.theme;
        let (fg, dim, accent) = (
            color(
                if self.convex_options().is_some() {
                    self.bar_palette().fg
                } else {
                    th.fg
                },
                1.0 * alpha,
            ),
            color(
                if self.convex_options().is_some() {
                    self.bar_palette().fg
                } else {
                    th.fg
                },
                0.55 * alpha,
            ),
            color(th.accent, 1.0 * alpha),
        );
        let title = format_now("%A %-d %B %Y");
        let mut t = title.chars();
        let title = t
            .next()
            .map(|c| c.to_uppercase().collect::<String>() + t.as_str())
            .unwrap_or_default();
        column![
            container(
                column![
                    text(title)
                        .font(self.font)
                        .size(crate::ui::tokens::ts(15.0))
                        .font(self.ui_font(iced::font::Weight::Semibold))
                        .color(fg),
                    text(format_now("%H:%M"))
                        .font(self.font)
                        .size(crate::ui::tokens::ts(12.0))
                        .color(dim)
                        .font(self.ui_font(iced::font::Weight::Normal)),
                ]
                .spacing(2)
            )
            .padding([6, 10]),
            container(self.calendar_nav(alpha, fg, accent)).padding([0, 10]),
            container(
                self.view_calendar(
                    12.5,
                    fg,
                    dim,
                    accent,
                    Some(
                        self.popover
                            .as_ref()
                            .map_or(WIDTH - 44.0, |p| p.size.width - 2.0 * SHADOW - 44.0)
                    ),
                    Some(self.cal_months)
                )
            )
            .padding([4, 10]),
        ]
        .into()
    }

    /// Mois précédent, aujourd'hui, mois suivant.
    fn calendar_nav(&self, alpha: f32, fg: Color, accent: Color) -> Element<'_, Msg> {
        let nav = |label: &str, delta: i32| {
            button(
                text(label.to_string())
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
                    .color(fg),
            )
            .padding([2, 10])
            .style(move |_, st| button::Style {
                background: matches!(st, button::Status::Hovered).then(|| {
                    Color {
                        a: 0.10 * alpha,
                        ..accent
                    }
                    .into()
                }),
                border: Border::default().rounded(8),
                text_color: fg,
                ..Default::default()
            })
            .on_press(Msg::CalendarMonth(delta))
        };
        let first =
            crate::ui::widget_view::month_start(chrono::Local::now().date_naive(), self.cal_months);
        let month = first
            .format_localized("%B %Y", crate::i18n::chrono_locale())
            .to_string();
        let mut r = row![
            nav("‹", -1),
            text(month)
                .font(self.ui_font(iced::font::Weight::Semibold))
                .size(crate::ui::tokens::ts(13.0))
                .color(fg)
                .width(Length::Fill)
                .align_x(Alignment::Center),
            nav("›", 1)
        ]
        .spacing(4);
        if self.cal_months != 0 {
            r = r.push(nav(crate::i18n::tr("Aujourd'hui"), 0));
        }
        r.align_y(Alignment::Center).into()
    }
}
