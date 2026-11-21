//! hyprdesk-i18n : langue de l'interface de hyprdesk, français (langue
//! source) ou anglais. Crate à part, sans iced ni Lua : l'écran de
//! verrouillage, l'écran de connexion et l'agent polkit s'en servent aussi.
//!
//! À la manière de gettext, le texte français sert de clé : `tr("Afficher la
//! barre")` rend « Show bar » en anglais, et le texte tel quel en français ou
//! s'il manque au catalogue (`locales/en.toml`, intégré au binaire). Un texte
//! à trous s'écrit `tr!("Workspace {ws}", ws = n)`.
//!
//! Choix de la langue : `hd.config({ language = "auto" | "fr" | "en" })`.
//! `auto` suit `LC_ALL`, `LC_MESSAGES` puis `LANG` : français si la langue est
//! le français, anglais sinon. Le test `catalogue_complet` vérifie que chaque
//! texte marqué dans le code a sa traduction.

use std::collections::HashMap;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU8, Ordering};

/// Langue de l'interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Fr,
    En,
}

static CURRENT: AtomicU8 = AtomicU8::new(0);
/// Langue choisie explicitement (`fr`, `en`) : les dates la suivent aussi.
static EXPLICIT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Catalogue anglais : texte français → texte anglais.
pub fn catalog() -> &'static HashMap<String, String> {
    static CATALOG: OnceLock<HashMap<String, String>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        toml::from_str(include_str!("../../../locales/en.toml")).unwrap_or_else(|e| {
            tracing::warn!("catalogue anglais illisible : {e}");
            HashMap::new()
        })
    })
}

pub fn current() -> Lang {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Lang::En,
        _ => Lang::Fr,
    }
}

pub fn set(lang: Lang) {
    CURRENT.store(
        match lang {
            Lang::Fr => 0,
            Lang::En => 1,
        },
        Ordering::Relaxed,
    );
}

/// Langue de l'environnement : français s'il le demande, anglais sinon.
pub fn from_env() -> Lang {
    let v = ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|k| std::env::var(k).ok())
        .find(|v| !v.is_empty())
        .unwrap_or_default();
    if v.starts_with("fr") {
        Lang::Fr
    } else {
        Lang::En
    }
}

/// Réglage `language` (`auto`, `fr`, `en`) → langue.
pub fn resolve(setting: &str) -> Lang {
    match setting.trim().to_lowercase().as_str() {
        "fr" | "français" | "francais" | "french" => Lang::Fr,
        "en" | "english" | "anglais" => Lang::En,
        _ => from_env(),
    }
}

/// Applique le réglage `language`.
pub fn apply(setting: &str) {
    // Tests (ici, ou de hyprdesk par la feature `fixed-lang`) : langue
    // source fixe (les tests tournent en parallèle et lisent des textes
    // français ; la langue du poste ne doit pas s'en mêler).
    if cfg!(any(test, feature = "fixed-lang")) {
        return;
    }
    let s = setting.trim().to_lowercase();
    EXPLICIT.store(!s.is_empty() && s != "auto", Ordering::Relaxed);
    set(resolve(setting));
}

/// Texte de plusieurs lignes traduit ligne à ligne (aides de la ligne de
/// commande : chaque ligne est une entrée du catalogue, colonnes comprises).
pub fn tr_lines(s: &str) -> String {
    s.lines().map(tr).collect::<Vec<_>>().join("\n")
}

/// Langue annoncée aux programmes tiers (LYRA) : `fr` ou `en`.
pub fn code() -> &'static str {
    match current() {
        Lang::Fr => "fr",
        Lang::En => "en",
    }
}

/// Langue des processus en ligne de commande, sans exécuter la config :
/// `HYPRDESK_LANG`, sinon la ligne `language = "…"` de settings.lua puis de
/// hyprdesk.lua (celui-ci l'emporte), sinon l'environnement.
pub fn init_cli(config_path: &std::path::Path) {
    if let Ok(v) = std::env::var("HYPRDESK_LANG") {
        return apply(&v);
    }
    apply(&early_setting(config_path, "language").unwrap_or_else(|| "auto".into()));
}

/// Réglage texte lu avant le chargement de la config (langue, moteur de
/// rendu) : settings.lua puis le fichier de config, le dernier trouvé gagne.
pub fn early_setting(config_path: &std::path::Path, key: &str) -> Option<String> {
    let settings = config_path
        .parent()
        .map(|d| d.join("hyprdesk").join("settings.lua"));
    let mut found = None;
    for p in settings
        .iter()
        .chain(std::iter::once(&config_path.to_path_buf()))
    {
        if let Some(v) = std::fs::read_to_string(p)
            .ok()
            .and_then(|s| setting_line(&s, key))
        {
            found = Some(v);
        }
    }
    found
}

/// Valeur de `<key> = "…"` (ou d'un nombre) dans un fichier Lua (hors commentaires) ; la clé
/// est un mot entier (`renderer` ne prend pas `smartui_renderer`).
pub fn setting_line(src: &str, key: &str) -> Option<String> {
    let word = |c: char| c.is_alphanumeric() || c == '_';
    src.lines()
        .map(|l| l.split("--").next().unwrap_or(""))
        .find_map(|l| {
            let at = l.match_indices(key).map(|(i, _)| i).find(|&i| {
                !l[..i].chars().next_back().is_some_and(word)
                    && !l[i + key.len()..].chars().next().is_some_and(word)
            })?;
            let rest = l[at + key.len()..]
                .trim_start()
                .strip_prefix('=')?
                .trim_start();
            let first = rest.chars().next()?;
            if first.is_ascii_digit() {
                // Nombre sans guillemets (`worker_threads = 4`).
                let end = rest
                    .find(|c: char| !c.is_ascii_digit() && c != '.')
                    .unwrap_or(rest.len());
                return Some(rest[..end].to_string());
            }
            let q = Some(first).filter(|c| *c == '"' || *c == '\'')?;
            let rest = &rest[1..];
            Some(rest[..rest.find(q)?].to_string())
        })
}

/// Texte dans la langue de l'interface (le français est la clé).
pub fn tr(s: &str) -> &str {
    tr_in(current(), s)
}

/// Texte dans une langue donnée.
pub fn tr_in(lang: Lang, s: &str) -> &str {
    match lang {
        Lang::Fr => s,
        Lang::En => catalog().get(s).map(String::as_str).unwrap_or(s),
    }
}

/// Remplit les trous `{nom}` d'un texte traduit.
pub fn fill(template: &str, args: &[(&str, String)]) -> String {
    let mut out = template.to_string();
    for (k, v) in args {
        out = out.replace(&format!("{{{k}}}"), v);
    }
    out
}

/// Langue des dates : celle de l'interface si elle est choisie, sinon celle
/// du système (`LC_TIME`, puis `LANG`).
pub fn chrono_locale() -> chrono::Locale {
    if EXPLICIT.load(Ordering::Relaxed) {
        return match current() {
            Lang::Fr => chrono::Locale::fr_FR,
            Lang::En => chrono::Locale::en_US,
        };
    }
    let lang = std::env::var("LC_TIME")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_default();
    let base = lang.split('.').next().unwrap_or("");
    chrono::Locale::try_from(base).unwrap_or(chrono::Locale::POSIX)
}

/// `tr!("texte")` ou `tr!("texte {a}", a = x)` : traduit, puis remplit.
#[macro_export]
macro_rules! tr {
    ($s:literal) => {
        $crate::tr($s)
    };
    ($s:literal, $($k:ident = $v:expr),+ $(,)?) => {
        $crate::fill($crate::tr($s), &[$((stringify!($k), ($v).to_string())),+])
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trous_remplis() {
        assert_eq!(fill("a {x} b {x}", &[("x", "3".into())]), "a 3 b 3");
        assert_eq!(tr_in(Lang::Fr, "Afficher la barre"), "Afficher la barre");
    }
}
