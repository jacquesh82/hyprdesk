//! Réglages des modules, déclarés une fois et montrés par la zone de
//! configuration du Panorama (appui long sur un module). Sans interface.
//!
//! Un réglage est une option de module (`plugin_options.panorama`, clé
//! `<module>.<clé>` : voir `option` et `with_option`). Les clés `side.*` du
//! même tableau retiennent le contenu de la zone d'accueil : ce ne sont pas
//! des réglages, elles ne sont pas déclarées ici.
//!
//! Un module peut vérifier un réglage une fois enregistré (la météo lit le
//! bulletin de la ville) : la zone montre sa réponse.

use crate::config::Config;

/// Nature d'un réglage, et ce qu'il propose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Un texte à saisir ; son texte d'exemple.
    Text { placeholder: &'static str },
    /// Un écran branché ; la valeur vide : l'écran actif.
    Screens,
}

/// Libellé de l'écran actif (réglage `Screens` à la valeur vide).
pub const ACTIVE_SCREEN: &str = "Écran actif";

/// Réponse du module à un réglage enregistré : une ligne à montrer, ou la
/// raison pour laquelle le réglage ne convient pas. Bloquante (le temps
/// d'une lecture).
pub type Verify = fn(&str) -> Result<String, String>;

/// Un réglage d'un module. (Pas de comparaison : une vérification est un
/// pointeur de fonction, qui ne se compare pas.)
#[derive(Debug, Clone, Copy)]
pub struct Setting {
    /// Clé de l'option, sans le nom du module (`city` pour `meteo.city`).
    pub key: &'static str,
    /// Libellé affiché (traduit à l'écran).
    pub label: &'static str,
    pub kind: Kind,
    /// Vérification du réglage enregistré ; `None` : le module n'a rien à
    /// répondre.
    pub verify: Option<Verify>,
}

impl Setting {
    /// Le réglage demande-t-il une saisie au clavier ?
    pub fn is_text(&self) -> bool {
        matches!(self.kind, Kind::Text { .. })
    }

    /// Valeur réglée de `module` ; vide : celle par défaut du module.
    pub fn value(&self, cfg: &Config, module: &str) -> String {
        super::option(cfg, module, self.key)
    }
}

/// Réglages de `module`, dans l'ordre où la zone les montre ; vide : ce
/// module n'en a pas (ou se règle ailleurs, comme le pont Hue).
pub fn of(module: &str) -> &'static [Setting] {
    match module {
        "meteo" => &[Setting {
            key: "city",
            label: "Ville",
            kind: Kind::Text {
                placeholder: "Paris",
            },
            verify: Some(super::meteo::check_city),
        }],
        "notes" => &[
            Setting {
                key: "command",
                label: "Commande",
                kind: Kind::Text {
                    placeholder: "gnome-text-editor",
                },
                verify: None,
            },
            Setting {
                key: "monitor",
                label: "Écran",
                kind: Kind::Screens,
                verify: None,
            },
        ],
        _ => &[],
    }
}

/// Le premier réglage de `module` à saisir au clavier : celui qui prend le
/// focus quand la zone s'ouvre ; `None` : rien à saisir.
pub fn text_setting(module: &str) -> Option<&'static Setting> {
    of(module).iter().find(|s| s.is_text())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::panorama::modules::{CATALOGUE, with_option};

    /// Chaque réglage déclaré est sous une clé unique de son module, et ce
    /// sont bien celles que les modules lisent.
    #[test]
    fn registre_coherent() {
        for (module, _, _) in CATALOGUE {
            let mut keys: Vec<&str> = Vec::new();
            for s in of(module) {
                assert!(!keys.contains(&s.key), "{module}.{}", s.key);
                keys.push(s.key);
            }
        }
        assert_eq!(
            of("meteo").iter().map(|s| s.key).collect::<Vec<_>>(),
            ["city"]
        );
        assert_eq!(
            of("notes").iter().map(|s| s.key).collect::<Vec<_>>(),
            ["command", "monitor"]
        );
        // Sans réglage : les autres, le pont Hue se règle dans les Réglages,
        // et l'inconnu.
        for m in [
            "hue", "voice", "media", "playlist", "clock", "calendar", "system", "inconnu",
        ] {
            assert!(of(m).is_empty(), "{m}");
        }
    }

    #[test]
    fn premier_champ() {
        assert_eq!(text_setting("meteo").map(|s| s.key), Some("city"));
        assert_eq!(text_setting("notes").map(|s| s.key), Some("command"));
        assert!(text_setting("hue").is_none());
        assert_eq!(of("notes")[1].kind, Kind::Screens);
    }

    /// Libellés et textes d'exemple : traduits.
    #[test]
    fn libelles_traduits() {
        let mut items: Vec<&str> = vec![ACTIVE_SCREEN];
        for (module, _, _) in CATALOGUE {
            for s in of(module) {
                items.push(s.label);
                if let Kind::Text { placeholder } = s.kind {
                    items.push(placeholder);
                }
            }
        }
        let missing = crate::i18n::untranslated("modules", items);
        assert!(missing.is_empty(), "sans traduction : {missing:?}");
    }

    /// Valeur réglée, vide par défaut ; elle s'écrit par `with_option`.
    #[test]
    fn valeur_d_un_reglage() {
        let mut cfg = Config::default();
        let city = &of("meteo")[0];
        assert_eq!(city.value(&cfg, "meteo"), "");
        cfg.general.plugin_options = with_option(Default::default(), "meteo", "city", "Lyon");
        assert_eq!(city.value(&cfg, "meteo"), "Lyon");
    }
}
