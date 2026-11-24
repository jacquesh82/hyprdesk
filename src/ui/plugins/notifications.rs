//! Paquet intégré « Notifications » (HIG PAT-02, PAT-07, SYS-03) : par
//! application, l'autorisation d'afficher et le style de remise ; par-dessus,
//! l'effacement de l'historique. L'état gardé est celui de la réception
//! (`notifications::Authorizations`, fichier `notifications-apps`) : une
//! décision prise ici vaut pour la notification suivante de l'application.
//!
//! Contrat d'un paquet de réglages : voir `docs/PLUGINS.md`.

use crate::i18n::text;
use crate::notifications::{Authorization, Authorizations, Delivery, Notification, Passage};
use iced::widget::{Space, button, column, pick_list, row, toggler};
use iced::{Alignment, Element, Length, Task};

use super::super::panel::PanelMsg;
use super::super::toasts::{self, card_app, passage_key};
use super::super::{App, Msg as UiMsg};
use super::{PluginInfo, PluginMsg};

pub const INFO: PluginInfo = PluginInfo {
    id: "notifications",
    label: "Notifications",
    icon: "preferences-system-notifications",
    summary: "Autorisation d'afficher et style de remise, par application.",
    builtin: true,
};

pub fn wrap(m: Msg) -> UiMsg {
    UiMsg::Panel(PanelMsg::Plugin(PluginMsg::Notifications(m)))
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    /// Applications connues et leur décision, dans l'ordre de leur inscription.
    pub apps: Vec<(String, Authorization)>,
    pub loaded: bool,
    /// « Effacer l’historique » armé : le clic suivant efface.
    pub confirm_clear: bool,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Loaded(State),
    /// Style de remise d'une application.
    Style(String, Delivery),
    /// Autorisation d'une application, accordée ou révoquée.
    Allow(String, bool),
    ClearAsk,
    Clear,
    ClearCancel,
}

/// Style de remise au menu : le style et son libellé affiché.
#[derive(Debug, Clone, PartialEq)]
pub struct StyleChoice(pub Delivery, pub String);

impl std::fmt::Display for StyleChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.1)
    }
}

/// Les trois styles de remise (HIG SYS-03), dans l'ordre du menu.
pub fn styles() -> Vec<StyleChoice> {
    [
        (Delivery::Banner, "Bannières"),
        (Delivery::Alert, "Alertes"),
        (Delivery::Silent, "Aucune"),
    ]
    .into_iter()
    .map(|(style, label)| StyleChoice(style, crate::i18n::tr(label).to_string()))
    .collect()
}

/// Décision d'une application qui n'a pas l'autorisation.
fn refused() -> Authorization {
    Authorization {
        granted: false,
        delivery: Delivery::default(),
    }
}

/// Les applications connues (HIG PAT-07) : celles qui ont une décision, puis
/// celles de l'historique et celles du passage du Silence. Sans décision, une
/// application figure comme non autorisée : sa demande n'a pas encore de
/// réponse, et l'autoriser ici la lui donne.
pub fn known(
    auths: &Authorizations,
    history: &[String],
    passage: &Passage,
) -> Vec<(String, Authorization)> {
    let mut apps = auths.apps.clone();
    for key in history
        .iter()
        .chain(passage.apps.iter().map(|(key, _)| key))
    {
        if !apps.iter().any(|(k, _)| k == key) {
            apps.push((key.clone(), refused()));
        }
    }
    apps
}

fn read(history: &[String]) -> State {
    State {
        apps: known(&Authorizations::load(), history, &Passage::load()),
        loaded: true,
        ..Default::default()
    }
}

/// Une décision prise sur la page (HIG PAT-02 : garder, révoquer ; HIG
/// SYS-03 : le style), appliquée à un état gardé et à la page : `set` inscrit
/// ou remplace la décision et oublie le style d'une application refusée ; la
/// rangée prend la décision inscrite, à sa place. L'état à enregistrer est
/// rendu. Une application sans décision (connue par l'historique) part de
/// « non autorisée » : l'autoriser ici la lui donne.
fn decide(
    mut auths: Authorizations,
    page: &mut [(String, Authorization)],
    key: &str,
    change: impl FnOnce(&mut Authorization),
) -> Authorizations {
    let mut decision = page
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, a)| *a)
        .unwrap_or_else(refused);
    change(&mut decision);
    auths = auths.set(key, decision);
    let written = auths.get(key).unwrap_or(decision);
    if let Some((_, a)) = page.iter_mut().find(|(k, _)| k == key) {
        *a = written;
    }
    auths
}

/// De quoi remplir la page, lu hors du fil de l'interface. Les clés de
/// l'historique sont prises ici : l'historique appartient à l'interface.
pub fn load(app: &App) -> Task<UiMsg> {
    let history: Vec<String> = app.toasts.history.iter().map(passage_key).collect();
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || read(&history))
                .await
                .unwrap_or_default()
        },
        |s| wrap(Msg::Loaded(s)),
    )
}

impl App {
    /// La notification la plus récente de l'application `key` dans
    /// l'historique : son nom et son icône viennent de là.
    fn notifications_app(&self, key: &str) -> Option<&Notification> {
        self.toasts.history.iter().find(|n| passage_key(n) == key)
    }

    pub(in crate::ui) fn notifications_page(&self) -> Element<'_, UiMsg> {
        let ink = self.ink();
        let Some(s) = self
            .panel
            .as_ref()
            .map(|p| &p.notifications)
            .filter(|s| s.loaded)
        else {
            return text("Lecture…")
                .font(self.font)
                .size(crate::ui::tokens::ts(13.0))
                .color(ink.dim)
                .into();
        };
        let small = |t: String| {
            text(t)
                .font(self.font)
                .size(crate::ui::tokens::ts(12.0))
                .color(ink.dim)
        };
        let mut col = column![
            small(
                "Une application demande l'autorisation à sa première notification. Refusée, ses notifications ne s'affichent plus et n'entrent pas dans l'historique."
                    .to_string(),
            ),
            self.section("Applications"),
        ]
        .spacing(10);
        if s.apps.is_empty() {
            col = col.push(small(
                "Aucune application n'a encore demandé à envoyer de notifications.".to_string(),
            ));
        }
        for (key, a) in &s.apps {
            col = col.push(self.notifications_row(key, *a));
        }
        col = col.push(self.section("Historique"));
        if self.toasts.history.is_empty() {
            col = col.push(small("L’historique est vide.".to_string()));
        }
        col = col.push(self.notifications_clear(s.confirm_clear));
        col.into()
    }

    /// Une rangée : icône et nom de l'application, puis son style de remise
    /// (accordée seule) et l'interrupteur « Autoriser », coupé = révoquée.
    /// Sans historique (refusée, jamais remise), l'application garde sa clé
    /// pour nom et la cloche pour icône.
    fn notifications_row<'a>(&'a self, key: &'a str, a: Authorization) -> Element<'a, UiMsg> {
        let ink = self.ink();
        let n = self.notifications_app(key);
        let name = n.map(card_app).unwrap_or_else(|| key.to_string());
        let name = crate::ui::plugins::bar::surface::truncate(&name, 40);
        let icon = n.and_then(|n| self.class_icon_name(n)).unwrap_or_default();
        let mut line = row![].spacing(10).align_y(Alignment::Center);
        line = line.push(self.icon_or_bell(&icon, 24.0, ink.dim));
        // Le nom prend la place qui reste et s'y replie : les commandes ne
        // sortent pas du tiroir sur un petit écran.
        line = line.push(
            column![
                text(name)
                    .font(self.font)
                    .size(crate::ui::tokens::ts(13.0))
                    .color(if a.granted { ink.fg } else { ink.dim })
                    .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
            ]
            .width(Length::Fill),
        );
        if a.granted {
            let choices = styles();
            let current = choices.iter().find(|c| c.0 == a.delivery).cloned();
            line = line.push(
                pick_list(choices, current, move |c| {
                    wrap(Msg::Style(key.to_string(), c.0))
                })
                .font(self.font)
                .width(Length::Fixed(120.0)),
            );
        } else {
            // Sans style tant qu'elle n'est pas autorisée : la place reste, les
            // interrupteurs s'alignent.
            line = line.push(Space::new().width(120));
        }
        line = line.push(
            toggler(a.granted)
                .label(crate::i18n::tr("Autoriser"))
                .text_size(13)
                .font(self.font)
                .on_toggle(move |on| wrap(Msg::Allow(key.to_string(), on))),
        );
        line.into()
    }

    /// « Effacer l’historique » : commande destructive, confirmée en deux
    /// temps comme la remise à zéro ; sans effet, donc sans clic, historique
    /// vide. Efface par le chemin du centre Continuité et de la ligne de
    /// commande.
    fn notifications_clear(&self, armed: bool) -> Element<'_, UiMsg> {
        let label = |t: &'static str| text(t).font(self.font).size(crate::ui::tokens::ts(13.0));
        if armed {
            Element::from(
                row![
                    button(label("Confirmer : effacer l’historique"))
                        .padding([6, 14])
                        .style(button::danger)
                        .on_press(wrap(Msg::Clear)),
                    button(label("Annuler"))
                        .padding([6, 14])
                        .style(button::secondary)
                        .on_press(wrap(Msg::ClearCancel)),
                ]
                .spacing(10),
            )
        } else {
            button(label("Effacer l’historique"))
                .padding([6, 14])
                .style(button::secondary)
                .on_press_maybe((!self.toasts.history.is_empty()).then(|| wrap(Msg::ClearAsk)))
                .into()
        }
    }

    /// Une décision prise sur la page : le fichier gardé d'abord, la rangée
    /// ensuite, à sa place.
    fn notifications_decide(
        &mut self,
        key: &str,
        change: impl FnOnce(&mut Authorization),
    ) -> Task<UiMsg> {
        let Some(p) = self.panel.as_mut() else {
            return Task::none();
        };
        decide(
            Authorizations::load(),
            &mut p.notifications.apps,
            key,
            change,
        )
        .save();
        Task::none()
    }

    pub(in crate::ui) fn notifications_update(&mut self, m: Msg) -> Task<UiMsg> {
        match m {
            Msg::Loaded(s) => {
                // L'icône de l'application : demandée à l'arrivée de ses
                // notifications, sauf quand chacune apportait la sienne. La
                // cloche de repli aussi : une application refusée n'a pas
                // d'historique où la prendre.
                let names: Vec<String> = s
                    .apps
                    .iter()
                    .filter_map(|(key, _)| self.notifications_app(key))
                    .filter_map(|n| self.class_icon_name(n))
                    .collect();
                let mut wanted: Vec<Task<UiMsg>> = vec![self.want_icon(toasts::ICON)];
                wanted.extend(names.iter().map(|n| self.want_icon(n)));
                if let Some(p) = self.panel.as_mut() {
                    p.notifications = s;
                }
                Task::batch(wanted)
            }
            Msg::Style(key, delivery) => {
                self.notifications_decide(&key, move |a| a.delivery = delivery)
            }
            Msg::Allow(key, granted) => {
                self.notifications_decide(&key, move |a| a.granted = granted)
            }
            Msg::ClearAsk => {
                if let Some(p) = self.panel.as_mut() {
                    p.notifications.confirm_clear = true;
                }
                Task::none()
            }
            Msg::ClearCancel => {
                if let Some(p) = self.panel.as_mut() {
                    p.notifications.confirm_clear = false;
                }
                Task::none()
            }
            Msg::Clear => {
                if let Some(p) = self.panel.as_mut() {
                    p.notifications.confirm_clear = false;
                }
                Task::done(UiMsg::Control(crate::control::Command::Notifications(
                    "clear".into(),
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn granted(delivery: Delivery) -> Authorization {
        Authorization {
            granted: true,
            delivery,
        }
    }

    #[test]
    fn applications_connues() {
        let auths = Authorizations::default()
            .set("a", granted(Delivery::Alert))
            .set("b", refused());
        let apps = known(
            &auths,
            &["c".to_string(), "a".to_string()],
            &Passage::default().set("d", true).set("a", false),
        );
        let keys: Vec<&str> = apps.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(
            keys,
            ["a", "b", "c", "d"],
            "décisions, historique, passage, sans doublon"
        );
        assert_eq!(apps[0].1, granted(Delivery::Alert));
        assert_eq!(apps[1].1, refused());
        assert_eq!(apps[2].1, refused(), "sans décision : pas encore autorisée");
    }

    /// HIG PAT-02, SYS-03 : couper « Autoriser » révoque et oublie le style ;
    /// autoriser depuis la page une application sans décision l'inscrit.
    #[test]
    fn decision_de_la_page_appliquee() {
        let auths = Authorizations::default()
            .set("a", granted(Delivery::Alert))
            .set("b", granted(Delivery::Silent));
        let mut page = known(&auths, &["c".to_string()], &Passage::default());

        let auths = decide(auths.clone(), &mut page, "a", |a| a.granted = false);
        assert_eq!(auths.get("a"), Some(refused()), "révoquée : sans style");
        assert_eq!(page[0].1, refused(), "la rangée suit le fichier");
        assert_eq!(
            auths.get("b"),
            Some(granted(Delivery::Silent)),
            "les autres restent"
        );

        let auths = decide(auths.clone(), &mut page, "c", |a| a.granted = true);
        assert_eq!(
            auths.get("c"),
            Some(granted(Delivery::default())),
            "autorisée : inscrite, style par défaut"
        );
        assert_eq!(page[2].1, granted(Delivery::default()));

        let auths = decide(auths, &mut page, "b", |a| a.delivery = Delivery::Banner);
        assert_eq!(auths.get("b"), Some(granted(Delivery::default())));
    }

    #[test]
    fn les_trois_styles_au_menu() {
        let styles: Vec<Delivery> = styles().into_iter().map(|c| c.0).collect();
        assert_eq!(
            styles,
            [Delivery::Banner, Delivery::Alert, Delivery::Silent]
        );
    }
}
