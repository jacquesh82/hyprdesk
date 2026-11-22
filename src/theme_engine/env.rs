//! Environnement de lancement (§9.2 de la spécification) : les variables
//! `HYPRDESK_*` exposées aux applications lancées par le démon (autostart,
//! lanceur, dock). Lecture au démarrage des processus : la palette publiée
//! au moment de la session fait foi — les applications qui veulent suivre le
//! thème à chaud lisent `palette.json` (§9.1) ou la crate cliente.
//!
//! Injection au démarrage du démon, avant tout fil : mutation d'environnement
//! après coup n'y est pas sûre.

use super::color;
use super::paths::Roots;
use super::resolve::Resolved;
use super::state;

/// Paires `HYPRDESK_*` d'une palette résolue (fonction pure, testable).
pub fn from_resolved(resolved: &Resolved) -> Vec<(String, String)> {
    let slot = |name: &str| {
        resolved
            .palette
            .get(name)
            .map(|c| color::fmt(*c, color::Fmt::Hex))
            .unwrap_or_default()
    };
    vec![
        ("HYPRDESK_THEME_ID".to_string(), resolved.id.clone()),
        (
            "HYPRDESK_THEME_VARIANT".to_string(),
            resolved.variant.clone(),
        ),
        ("HYPRDESK_BG".to_string(), slot("bg")),
        ("HYPRDESK_FG".to_string(), slot("fg")),
        ("HYPRDESK_ACCENT".to_string(), slot("accent")),
        ("HYPRDESK_SURFACE".to_string(), slot("surface")),
        ("HYPRDESK_BORDER".to_string(), slot("border")),
    ]
}

/// Injecte la palette publiée dans l'environnement du processus, avant la
/// mise en route des fils. Appelé une fois au démarrage du démon ; la
/// palette publiée par la première application du thème fera foi pour la
/// session suivante.
pub fn inject_session() {
    let roots = Roots::from_env();
    let Some(doc) = state::read_palette(&roots) else {
        return;
    };
    for (key, value) in doc_colors(&doc) {
        // SAFETY : appel unique au démarrage du démon, avant la création de
        // tout fil — mutation d'environnement sûre à ce stade.
        unsafe { std::env::set_var(key, value) };
    }
}

/// Les paires de la palette publiée (doc sérialisé).
/// Préfixe de lancement (MLG-235) : les applications sont lancées par
/// Hyprland (`hl.dsp.exec_cmd`), pas par le démon — l'environnement du démon
/// ne leur arrive pas. Chaque lancement exporte donc les variables de la
/// palette publiée à cet instant (`export …; commande`, valable aussi pour
/// une commande composée). Vide sans palette publiée.
pub fn launch_prefix() -> String {
    let Some(doc) = state::read_palette(&Roots::from_env()) else {
        return String::new();
    };
    prefix_of(&doc)
}

fn prefix_of(doc: &state::PaletteDoc) -> String {
    let vars: Vec<String> = doc_colors(doc)
        .into_iter()
        // Valeurs bornées (identifiant, variante, #hex) : rien à échapper,
        // tout autre caractère écarte la variable.
        .filter(|(_, v)| {
            !v.is_empty()
                && v.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '#' | '-' | '_'))
        })
        .map(|(k, v)| format!("{k}={v}"))
        .collect();
    if vars.is_empty() {
        return String::new();
    }
    format!("export {}; ", vars.join(" "))
}

fn doc_colors(doc: &state::PaletteDoc) -> Vec<(String, String)> {
    let mut out = vec![
        ("HYPRDESK_THEME_ID".to_string(), doc.pack.clone()),
        ("HYPRDESK_THEME_VARIANT".to_string(), doc.variant.clone()),
    ];
    for (slot, var) in [
        ("bg", "HYPRDESK_BG"),
        ("fg", "HYPRDESK_FG"),
        ("accent", "HYPRDESK_ACCENT"),
        ("surface", "HYPRDESK_SURFACE"),
        ("border", "HYPRDESK_BORDER"),
    ] {
        if let Some(v) = doc.colors.get(slot) {
            out.push((var.to_string(), v.clone()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::resolve;
    use super::*;
    use crate::config::Theme;

    #[test]
    fn paires_complètes_depuis_la_palette_résolue() {
        let r = resolve::from_theme(&Theme::default());
        let vars = from_resolved(&r);
        for attendu in [
            "HYPRDESK_THEME_ID",
            "HYPRDESK_THEME_VARIANT",
            "HYPRDESK_BG",
            "HYPRDESK_FG",
            "HYPRDESK_ACCENT",
            "HYPRDESK_SURFACE",
            "HYPRDESK_BORDER",
        ] {
            assert!(vars.iter().any(|(k, _)| k == attendu), "{vars:?}");
        }
        let (id, pack) = vars.iter().find(|(k, _)| k == "HYPRDESK_THEME_ID").unwrap();
        assert_eq!(pack, "regle", "{id}");
        // Les couleurs sont en hexadécimal.
        let (_, bg) = vars.iter().find(|(k, _)| k == "HYPRDESK_BG").unwrap();
        assert!(bg.starts_with('#'), "{bg}");
    }

    #[test]
    fn préfixe_de_lancement_borné() {
        let mut doc = state::PaletteDoc {
            schema: state::SCHEMA,
            pack: "ardoise".to_string(),
            variant: "dark".to_string(),
            revision: 1,
            colors: [("bg".to_string(), "#171b27".to_string())].into(),
            effects: Default::default(),
            fonts: None,
            updated: String::new(),
        };
        assert_eq!(
            prefix_of(&doc),
            "export HYPRDESK_THEME_ID=ardoise HYPRDESK_THEME_VARIANT=dark HYPRDESK_BG=#171b27; "
        );
        // Une valeur hors bornes n'est jamais passée au shell.
        doc.pack = "x; rm -rf ~".to_string();
        assert!(!prefix_of(&doc).contains("rm"));
    }
}
