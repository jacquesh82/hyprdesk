//! Langue de l'interface : français (langue source) ou anglais.
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
fn english() -> &'static HashMap<String, String> {
    static CATALOG: OnceLock<HashMap<String, String>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        toml::from_str(include_str!("../locales/en.toml")).unwrap_or_else(|e| {
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
    // Tests : langue source fixe (les tests tournent en parallèle et lisent
    // des textes français ; la langue du poste ne doit pas s'en mêler).
    if cfg!(test) {
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
    let mut setting = String::from("auto");
    let settings = config_path
        .parent()
        .map(|d| d.join("hyprdesk").join("settings.lua"));
    for p in settings
        .iter()
        .chain(std::iter::once(&config_path.to_path_buf()))
    {
        if let Some(v) = std::fs::read_to_string(p)
            .ok()
            .and_then(|s| language_line(&s))
        {
            setting = v;
        }
    }
    apply(&setting);
}

/// Valeur de `language = "…"` dans un fichier Lua (hors commentaires).
fn language_line(src: &str) -> Option<String> {
    src.lines()
        .map(|l| l.split("--").next().unwrap_or(""))
        .find_map(|l| {
            let (_, rest) = l.split_once("language")?;
            let rest = rest.trim_start().strip_prefix('=')?.trim_start();
            let q = rest.chars().next().filter(|c| *c == '"' || *c == '\'')?;
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
        Lang::En => english().get(s).map(String::as_str).unwrap_or(s),
    }
}

/// Le texte a sa traduction anglaise (tests des registres).
#[cfg(test)]
pub fn has_english(s: &str) -> bool {
    english().contains_key(s)
}

/// Tests : textes sans traduction anglaise parmi `items` (ceux sans lettre
/// ignorés). Avec `HYPRDESK_I18N_TODO_DIR`, les écrit dans `<dossier>/<nom>.toml`.
#[cfg(test)]
pub fn untranslated<'a>(name: &str, items: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut missing: Vec<String> = items
        .into_iter()
        .filter(|s| s.chars().any(char::is_alphabetic) && !has_english(s))
        .map(str::to_string)
        .collect();
    missing.sort();
    missing.dedup();
    if let Ok(d) = std::env::var("HYPRDESK_I18N_TODO_DIR") {
        let toml: String = missing
            .iter()
            .map(|s| format!("{} = \"\"\n", serde_json::to_string(s).unwrap()))
            .collect();
        std::fs::write(std::path::Path::new(&d).join(format!("{name}.toml")), toml).unwrap();
    }
    missing
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

/// Widget texte d'iced, traduit : remplace `iced::widget::text` dans
/// l'interface. Un texte présent au catalogue (littéral ou calculé) s'affiche
/// dans la langue choisie ; les autres (titres de fenêtres, valeurs) tels quels.
pub fn text<'a, Theme, Renderer>(
    s: impl iced::widget::text::IntoFragment<'a>,
) -> iced::widget::Text<'a, Theme, Renderer>
where
    Theme: iced::widget::text::Catalog + 'a,
    Renderer: iced::advanced::text::Renderer,
{
    use std::borrow::Cow;
    let f = match s.into_fragment() {
        Cow::Borrowed(b) => Cow::Borrowed(tr(b)),
        Cow::Owned(o) => match current() {
            Lang::Fr => Cow::Owned(o),
            Lang::En => match english().get(&o) {
                Some(t) => Cow::Borrowed(t.as_str()),
                None => Cow::Owned(o),
            },
        },
    };
    iced::widget::text(f)
}

/// `tr!("texte")` ou `tr!("texte {a}", a = x)` : traduit, puis remplit.
#[macro_export]
macro_rules! tr {
    ($s:literal) => {
        $crate::i18n::tr($s)
    };
    ($s:literal, $($k:ident = $v:expr),+ $(,)?) => {
        $crate::i18n::fill($crate::i18n::tr($s), &[$((stringify!($k), ($v).to_string())),+])
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Appels dont le premier argument littéral est un texte affiché.
    const CALLS: &[&str] = &[
        "tr(",
        "tr!(",
        "text(",
        ".section(",
        ".field(",
        "radio(",
        ".label(",
        "text_input(",
        ".placeholder(",
        "small(",
        "label: ",
        "summary: ",
        "hd.t(",
    ];

    /// Littéral Rust ou Lua qui commence à `src` (après le guillemet ouvrant),
    /// tel que le compilateur le lit (`\` en fin de ligne : saut et retrait ignorés).
    fn literal(src: &str) -> String {
        let mut out = String::new();
        let mut it = src.chars().peekable();
        while let Some(c) = it.next() {
            match c {
                '"' => break,
                '\\' => match it.next() {
                    Some('n') => out.push('\n'),
                    Some('t') => out.push('\t'),
                    Some('\n') => {
                        while it.peek().is_some_and(|c| c.is_whitespace()) {
                            it.next();
                        }
                    }
                    Some(o) => out.push(o),
                    None => break,
                },
                o => out.push(o),
            }
        }
        out
    }

    /// Textes marqués dans le code (hors tests et commentaires).
    pub(crate) fn marked() -> Vec<(String, String)> {
        fn walk(dir: &std::path::Path, ext: &str, files: &mut Vec<std::path::PathBuf>) {
            for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, ext, files);
                } else if p.extension().is_some_and(|x| x == ext) {
                    files.push(p);
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        walk(&root.join("src"), "rs", &mut files);
        walk(&root.join("assets"), "lua", &mut files);
        let mut out = Vec::new();
        for f in files {
            if f.ends_with("i18n.rs") {
                continue;
            }
            let full = std::fs::read_to_string(&f).unwrap();
            let src = full.split("#[cfg(test)]").next().unwrap_or("");
            // Commentaires retirés (lignes entières), longueurs préservées.
            let src: String = src
                .lines()
                .map(|l| {
                    let t = l.trim_start();
                    if t.starts_with("//") || t.starts_with("--") {
                        " ".repeat(l.len())
                    } else {
                        l.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            let name = f.strip_prefix(root).unwrap().display().to_string();
            for call in CALLS {
                for (i, _) in src.match_indices(call) {
                    // `tr(` ne doit pas être la fin d'un autre nom (`attr(`).
                    let before = src[..i].chars().next_back();
                    if !call.starts_with(['.', 'l', 's', 'h'])
                        && before.is_some_and(|c| c.is_alphanumeric() || c == '_')
                    {
                        continue;
                    }
                    let rest = src[i + call.len()..].trim_start();
                    if let Some(lit) = rest.strip_prefix('"') {
                        let lit = literal(lit);
                        if lit.chars().any(char::is_alphabetic) {
                            out.push((name.clone(), lit));
                        }
                    }
                }
            }
        }
        out
    }

    #[test]
    fn catalogue_complet() {
        let en = english();
        assert!(!en.is_empty(), "catalogue anglais vide ou illisible");
        let mut missing: Vec<String> = marked()
            .into_iter()
            .filter(|(_, s)| !en.contains_key(s))
            .map(|(f, s)| format!("{f} : {s:?}"))
            .collect();
        missing.sort();
        missing.dedup();
        // Pour compléter : HYPRDESK_I18N_TODO=fichier écrit les entrées manquantes.
        if let Ok(p) = std::env::var("HYPRDESK_I18N_TODO") {
            let mut todo: Vec<String> = marked()
                .into_iter()
                .filter(|(_, s)| !en.contains_key(s))
                .map(|(_, s)| s)
                .collect();
            todo.sort();
            todo.dedup();
            let toml: String = todo
                .iter()
                .map(|s| format!("{} = \"\"\n", serde_json::to_string(s).unwrap()))
                .collect();
            std::fs::write(p, toml).unwrap();
        }
        assert!(
            missing.is_empty(),
            "{} texte(s) sans traduction anglaise :\n{}",
            missing.len(),
            missing.join("\n")
        );
    }

    /// Aide de la ligne de commande : chaque ligne a sa traduction.
    #[test]
    fn aide_traduite() {
        let missing = untranslated("usage", crate::USAGE.lines());
        assert!(missing.is_empty(), "sans traduction : {missing:#?}");
    }

    /// Commentaires des textes Lua produits par hyprdesk (extrait pour
    /// hyprland.lua, en-tête de monitors.lua) : chacun a sa traduction.
    #[test]
    fn commentaires_generes_traduits() {
        let monitors = crate::screens::render(&[], "");
        let comments: Vec<&str> = crate::SNIPPET
            .lines()
            .chain(monitors.lines())
            .filter(|l| l.trim_start().starts_with("--"))
            .collect();
        let missing = untranslated("lua", comments);
        assert!(missing.is_empty(), "sans traduction : {missing:#?}");
    }

    /// Exemples livrés en deux langues : même code, seuls les commentaires et
    /// les textes affichés diffèrent.
    #[test]
    fn exemples_paralleles() {
        fn code(src: &str) -> Vec<String> {
            src.lines()
                .map(|l| {
                    let mut out = String::new();
                    let mut in_str = false;
                    let mut chars = l.chars().peekable();
                    while let Some(c) = chars.next() {
                        if in_str {
                            if c == '\\' {
                                chars.next();
                            } else if c == '"' {
                                in_str = false;
                                out.push('"');
                            }
                        } else if c == '"' {
                            in_str = true;
                            out.push('"');
                        } else if c == '-' && chars.peek() == Some(&'-') {
                            break;
                        } else {
                            out.push(c);
                        }
                    }
                    out.trim_end().to_string()
                })
                .collect()
        }
        let pairs = [
            (
                include_str!("../assets/hyprdesk.lua"),
                include_str!("../assets/hyprdesk.en.lua"),
            ),
            (
                include_str!("../assets/hyprdesk.meta.lua"),
                include_str!("../assets/hyprdesk.meta.en.lua"),
            ),
        ];
        for (fr, en) in pairs {
            assert_eq!(code(fr), code(en));
        }
    }

    #[test]
    fn langue() {
        // Sans toucher à la langue globale (les tests tournent en parallèle).
        assert_eq!(tr_in(Lang::En, "Afficher la barre"), "Show bar");
        assert_eq!(tr_in(Lang::En, "texte absent"), "texte absent");
        assert_eq!(tr_in(Lang::Fr, "Afficher la barre"), "Afficher la barre");
        assert_eq!(
            fill(tr_in(Lang::En, "Workspace {ws}"), &[("ws", "3".into())]),
            "Workspace 3"
        );
        assert_eq!(resolve("en"), Lang::En);
        assert_eq!(resolve("FR"), Lang::Fr);
        assert_eq!(
            language_line("hd.config({\n  language = \"en\", -- anglais\n})"),
            Some("en".into())
        );
        assert_eq!(language_line("-- language = \"en\""), None);
    }
}
