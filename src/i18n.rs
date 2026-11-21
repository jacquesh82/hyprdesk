//! Langue de l'interface : le cœur est le crate `hyprdesk-i18n` (catalogue,
//! `tr`, `tr!`, choix de la langue), partagé avec les binaires dédiés. Ici :
//! le widget texte traduit d'iced, et les tests qui vérifient que chaque texte
//! marqué dans le code de hyprdesk a sa traduction.

pub use hyprdesk_i18n::{
    Lang, apply, catalog, chrono_locale, code, current, early_setting, fill, from_env, init_cli,
    resolve, set, setting_line, tr, tr_in, tr_lines,
};

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
            Lang::En => match catalog().get(&o) {
                Some(t) => Cow::Borrowed(t.as_str()),
                None => Cow::Owned(o),
            },
        },
    };
    iced::widget::text(f)
}

/// Catalogue anglais (tests).
#[cfg(test)]
fn english() -> &'static std::collections::HashMap<String, String> {
    catalog()
}

/// Valeur de `language = "…"` dans un fichier Lua (hors commentaires).
#[cfg(test)]
fn language_line(src: &str) -> Option<String> {
    setting_line(src, "language")
}

/// Le texte a sa traduction anglaise (tests des registres).
#[cfg(test)]
pub fn has_english(s: &str) -> bool {
    catalog().contains_key(s)
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
        // Crates du dépôt (authentification, langue) : mêmes textes affichés.
        walk(&root.join("crates"), "rs", &mut files);
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
        assert_eq!(
            setting_line(
                "  smartui_renderer = 'x', renderer = 'software'",
                "renderer"
            ),
            Some("software".into())
        );
        assert_eq!(setting_line("  renderers = 'x'", "renderer"), None);
        assert_eq!(
            setting_line("  worker_threads = 8,", "worker_threads"),
            Some("8".into())
        );
    }
}
