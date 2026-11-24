//! Recherche dans les Réglages : l'index des libellés de chaque page (titres
//! de section, champs, textes, onglets), relu dans le source des paquets
//! embarqué à la compilation — il suit donc les pages sans liste à tenir.
//! Un libellé est rattaché à l'onglet dont la fonction l'affiche
//! (`if self.on_subtab(INFO.id, "x") { return self.f(); }`).

use std::sync::LazyLock;

use super::plugins;

/// Source de chaque paquet intégré.
const SOURCES: &[(&str, &str)] = &[
    ("about", include_str!("plugins/about.rs")),
    ("animations", include_str!("plugins/animations.rs")),
    ("apps", include_str!("plugins/apps.rs")),
    ("apptheme", include_str!("plugins/apptheme.rs")),
    ("assistant", include_str!("plugins/assistant.rs")),
    ("bt", include_str!("plugins/bluetooth.rs")),
    ("autostart", include_str!("plugins/autostart.rs")),
    ("bar", include_str!("plugins/bar.rs")),
    ("colors", include_str!("plugins/colors.rs")),
    // Libellés des curseurs (`crate::colors::FIELDS`).
    ("colors", include_str!("../colors.rs")),
    ("datetime", include_str!("plugins/datetime.rs")),
    ("desktop", include_str!("plugins/desktop.rs")),
    ("animations", include_str!("plugins/effects.rs")),
    ("energy", include_str!("plugins/energy.rs")),
    ("hyprconfig", include_str!("plugins/hyprconfig.rs")),
    ("history", include_str!("plugins/history.rs")),
    ("icons", include_str!("plugins/icons.rs")),
    ("index", include_str!("plugins/index.rs")),
    ("input", include_str!("plugins/input.rs")),
    ("keys", include_str!("plugins/keys.rs")),
    ("dock", include_str!("plugins/dock.rs")),
    ("launcher", include_str!("plugins/launcher.rs")),
    ("launcher", include_str!("plugins/launcher/grid.rs")),
    ("layouts", include_str!("plugins/layouts.rs")),
    ("lock", include_str!("plugins/lock.rs")),
    ("login", include_str!("plugins/login.rs")),
    ("modules", include_str!("plugins/modules.rs")),
    ("modules", include_str!("plugins/panorama/modules.rs")),
    ("network", include_str!("plugins/network.rs")),
    ("panorama", include_str!("plugins/panorama.rs")),
    ("panorama", include_str!("plugins/panorama/design.rs")),
    ("passwords", include_str!("plugins/passwords.rs")),
    ("performance", include_str!("plugins/performance.rs")),
    ("plugins", include_str!("plugins/plugins.rs")),
    ("screens", include_str!("plugins/screens.rs")),
    ("share", include_str!("plugins/share.rs")),
    ("sound", include_str!("plugins/sound.rs")),
    ("theme", include_str!("plugins/theme.rs")),
    ("wallpaper", include_str!("plugins/wallpaper.rs")),
    ("widgets", include_str!("plugins/widgets.rs")),
    ("workspaces", include_str!("plugins/workspaces.rs")),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Page,
    Tab,
    Label,
    /// Texte d'aide (long).
    Help,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub page: &'static str,
    pub tab: Option<&'static str>,
    /// Libellé tel qu'écrit (français) ; traduit à l'affichage.
    pub text: String,
    pub kind: Kind,
    /// Section de la page qui le montre (`self.section("…")` qui le
    /// précède dans la même fonction) : le bloc affiché en résultat.
    pub section: Option<String>,
    /// Place dans le source (fichier, position) : l'ordre du bloc.
    pub at: (usize, usize),
}

/// Lit une chaîne Rust à partir du guillemet ouvrant ; rend (texte, fin).
fn literal(s: &str, start: usize) -> Option<(String, usize)> {
    let b = s.as_bytes();
    if b.get(start) != Some(&b'"') {
        return None;
    }
    let mut out = String::new();
    let mut it = s[start + 1..].char_indices().peekable();
    while let Some((i, c)) = it.next() {
        match c {
            '"' => return Some((out, start + 1 + i + 1)),
            '\\' => match it.next().map(|x| x.1) {
                Some('n') => out.push(' '),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('\'') => out.push('\''),
                // Continuation de ligne : les blancs qui suivent disparaissent.
                Some('\n') => {
                    while it.peek().is_some_and(|x| x.1.is_whitespace()) {
                        it.next();
                    }
                }
                Some(o) => out.push(o),
                None => return None,
            },
            _ => out.push(c),
        }
    }
    None
}

/// Appels qui affichent un libellé.
const PATTERNS: &[&str] = &[
    "text(\"",
    "self.field(\"",
    ".adv(\"",
    "self.section(\"",
    "section(crate::i18n::tr(\"",
    "field(crate::i18n::tr(\"",
    ".label(crate::i18n::tr(\"",
    ".label(\"",
];

/// Onglet de chaque fonction de page : `on_subtab(INFO.id, "x")` puis `self.f(`.
fn tab_functions(src: &str, tabs: &[plugins::SubTab]) -> Vec<(String, &'static str)> {
    let mut out = Vec::new();
    let mut rest = src;
    while let Some(i) = rest.find("on_subtab(INFO.id, \"") {
        let after = &rest[i + 20..];
        let Some(end) = after.find('"') else { break };
        let tab = &after[..end];
        if let Some(&(t, _)) = tabs.iter().find(|(t, _)| *t == tab) {
            let window: String = after[end..].chars().take(160).collect();
            if let Some(j) = window.find("self.") {
                let name: String = window[j + 5..]
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() {
                    out.push((name, t));
                }
            }
        }
        rest = after;
    }
    out
}

fn build() -> Vec<Entry> {
    let mut v = Vec::new();
    for info in plugins::BUILTIN {
        v.push(Entry {
            page: info.id,
            tab: None,
            text: info.label.to_string(),
            kind: Kind::Page,
            section: None,
            at: (0, 0),
        });
        v.push(Entry {
            page: info.id,
            tab: None,
            text: info.summary.to_string(),
            kind: Kind::Help,
            section: None,
            at: (0, 0),
        });
        for (t, l) in plugins::tabs(info.id) {
            v.push(Entry {
                page: info.id,
                tab: Some(t),
                text: l.to_string(),
                kind: Kind::Tab,
                section: None,
                at: (0, 0),
            });
        }
    }
    for (file, (page, src)) in SOURCES.iter().enumerate() {
        let tabs = plugins::tabs(page);
        let funcs = tab_functions(src, tabs);
        // Début de chaque fonction : (position, onglet).
        let mut starts: Vec<(usize, Option<&'static str>)> = Vec::new();
        for (pos, _) in src.match_indices("fn ") {
            let name: String = src[pos + 3..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            starts.push((pos, funcs.iter().find(|(f, _)| *f == name).map(|x| x.1)));
        }
        // Les essais ne sont pas des pages.
        let end = src.find("#[cfg(test)]").unwrap_or(src.len());
        // Titres de section : (position, titre).
        let mut sections: Vec<(usize, String)> = Vec::new();
        for pat in ["self.section(\"", "section(crate::i18n::tr(\""] {
            for (pos, _) in src[..end].match_indices(pat) {
                if let Some((t, _)) = literal(src, pos + pat.len() - 1) {
                    sections.push((pos, t.trim().to_string()));
                }
            }
        }
        sections.sort_by_key(|x| x.0);
        // Section d'un libellé : la dernière ouverte avant lui, dans sa fonction.
        let section_at = |pos: usize| -> Option<String> {
            let f = starts
                .iter()
                .rev()
                .find(|(p, _)| *p < pos)
                .map_or(0, |x| x.0);
            sections
                .iter()
                .rev()
                .find(|(p, _)| *p <= pos && *p >= f)
                .map(|x| x.1.clone())
        };
        // Tables de choix et de curseurs : `("id", "Libellé"…)`.
        for (pos, _) in src[..end].match_indices("(\"") {
            let Some((id, after)) = literal(src, pos + 1) else {
                continue;
            };
            let is_id = !id.is_empty()
                && id
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || "_-.:".contains(c));
            if !is_id || !src[after..].starts_with(", \"") {
                continue;
            }
            let Some((s, _)) = literal(src, after + 2) else {
                continue;
            };
            let s = s.trim().to_string();
            let first_upper = s.chars().next().is_some_and(|c| c.is_uppercase());
            if !first_upper || s.contains('{') || s.chars().count() > 70 {
                continue;
            }
            let tab = starts
                .iter()
                .rev()
                .find(|(p, _)| *p < pos)
                .and_then(|x| x.1);
            if !v.iter().any(|e: &Entry| e.page == *page && e.text == s) {
                v.push(Entry {
                    page,
                    tab,
                    text: s,
                    kind: Kind::Label,
                    section: section_at(pos),
                    at: (file + 1, pos),
                });
            }
        }
        for pat in PATTERNS {
            for (pos, _) in src[..end].match_indices(pat) {
                let Some((s, _)) = literal(src, pos + pat.len() - 1) else {
                    continue;
                };
                let s = s.trim().to_string();
                let letters = s.chars().filter(|c| c.is_alphabetic()).count();
                if letters < 3 || s.contains('{') || s.chars().count() > 260 {
                    continue;
                }
                let tab = starts
                    .iter()
                    .rev()
                    .find(|(p, _)| *p < pos)
                    .and_then(|x| x.1);
                let kind = if s.chars().count() > 70 {
                    Kind::Help
                } else {
                    Kind::Label
                };
                if !v.iter().any(|e: &Entry| e.page == *page && e.text == s) {
                    v.push(Entry {
                        page,
                        tab,
                        text: s,
                        kind,
                        section: section_at(pos),
                        at: (file + 1, pos),
                    });
                }
            }
        }
    }
    v
}

static INDEX: LazyLock<Vec<Entry>> = LazyLock::new(build);

/// Minuscules sans accents, caractère pour caractère (les positions restent alignées).
fn fold(c: char) -> char {
    let c = c.to_lowercase().next().unwrap_or(c);
    match c {
        'à' | 'â' | 'ä' | 'á' | 'ã' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'î' | 'ï' | 'í' => 'i',
        'ô' | 'ö' | 'ó' | 'õ' => 'o',
        'ù' | 'û' | 'ü' | 'ú' => 'u',
        'ç' => 'c',
        'ÿ' => 'y',
        'œ' => 'o',
        'æ' => 'a',
        '’' => '\'',
        o => o,
    }
}

fn terms(query: &str) -> Vec<Vec<char>> {
    query
        .split_whitespace()
        .map(|t| t.chars().map(fold).collect::<Vec<char>>())
        .filter(|t| !t.is_empty())
        .collect()
}

fn contains(hay: &[char], needle: &[char]) -> bool {
    needle.len() <= hay.len() && hay.windows(needle.len()).any(|w| w == needle)
}

/// Le texte contient tous les mots de la recherche (sans accents ni casse).
pub fn matches(text: &str, query: &str) -> bool {
    let t = terms(query);
    if t.is_empty() {
        return false;
    }
    let hay: Vec<char> = text.chars().map(fold).collect();
    t.iter().all(|n| contains(&hay, n))
}

/// Plages (octets) du texte qui correspondent à un mot de la recherche.
pub fn ranges(text: &str, query: &str) -> Vec<(usize, usize)> {
    let t = terms(query);
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let hay: Vec<char> = chars.iter().map(|x| fold(x.1)).collect();
    let mut marked = vec![false; hay.len()];
    for n in &t {
        if n.len() > hay.len() {
            continue;
        }
        for i in 0..=hay.len() - n.len() {
            if hay[i..i + n.len()] == n[..] {
                marked[i..i + n.len()].iter_mut().for_each(|m| *m = true);
            }
        }
    }
    let mut out = Vec::new();
    let mut i = 0;
    while i < marked.len() {
        if marked[i] {
            let s = i;
            while i < marked.len() && marked[i] {
                i += 1;
            }
            let end = chars.get(i).map(|x| x.0).unwrap_or(text.len());
            out.push((chars[s].0, end));
        } else {
            i += 1;
        }
    }
    out
}

/// Résultats d'une recherche, paquets visibles seulement (`visible(id)`),
/// les plus parlants d'abord : pages, onglets, libellés, aides.
pub fn search(query: &str, visible: impl Fn(&str) -> bool) -> Vec<&'static Entry> {
    if terms(query).is_empty() {
        return Vec::new();
    }
    let mut r: Vec<&'static Entry> = INDEX
        .iter()
        .filter(|e| visible(e.page))
        .filter(|e| matches(&e.text, query) || matches(crate::i18n::tr(&e.text), query))
        .collect();
    let order = |p: &str| {
        plugins::BUILTIN
            .iter()
            .position(|i| i.id == p)
            .unwrap_or(usize::MAX)
    };
    r.sort_by_key(|e| {
        (
            order(e.page),
            e.kind,
            e.tab.is_some(),
            e.text.chars().count(),
        )
    });
    r
}

/// Bloc d'un résultat : les entrées de la même section (même paquet, même
/// onglet), dans l'ordre de la page. Sans section : les voisines proches.
pub fn block(e: &Entry) -> Vec<&'static Entry> {
    let mut v: Vec<&'static Entry> = INDEX
        .iter()
        .filter(|x| {
            x.page == e.page
                && x.tab == e.tab
                && x.at.0 == e.at.0
                && matches!(x.kind, Kind::Label | Kind::Help)
                && x.section == e.section
        })
        .collect();
    v.sort_by_key(|x| x.at);
    if e.section.is_none() || e.at.0 == 0 {
        // Pas de section : trois voisines de part et d'autre.
        let i = v.iter().position(|x| std::ptr::eq(*x, e)).unwrap_or(0);
        let lo = i.saturating_sub(3);
        let hi = (i + 4).min(v.len());
        v = v[lo..hi].to_vec();
    }
    v
}

/// Résultats pour le lanceur (`:r`) : [`search`], classés pour une liste
/// courte — les entrées les plus parlantes d'abord (page, onglet, libellé,
/// aide ; le libellé le plus court) — et non groupés par paquet comme au
/// tiroir, qui les montre sous leur en-tête.
pub fn launcher(query: &str, visible: impl Fn(&str) -> bool) -> Vec<&'static Entry> {
    let mut r = search(query, visible);
    r.sort_by_key(|e| (e.kind, e.text.chars().count()));
    r
}

/// Préfixe du lanceur : « :r fond d'écran ».
pub const PREFIX: &str = ":r";
/// Sa forme longue : « :reglages fond d'écran ».
pub const PREFIX_LONG: &str = ":reglages";

/// Mode réglages du lanceur (`:r`, `:reglages`) : la requête sans son
/// préfixe. Il tient seul ou suivi d'une espace — « :rouge » n'est pas ce
/// mode (HIG PAT-13).
pub fn query(q: &str) -> Option<&str> {
    let rest = q.trim_start();
    let rest = rest
        .strip_prefix(PREFIX_LONG)
        .or_else(|| rest.strip_prefix(PREFIX))?;
    (rest.is_empty() || rest.starts_with(' ')).then(|| rest.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recherche_dans_les_reglages() {
        // Libellés relus dans les pages, rattachés à leur onglet.
        let r = search("gamma", |_| true);
        assert!(
            r.iter()
                .any(|e| e.page == "colors" && e.text == "Gamma rouge"),
            "{r:?}"
        );
        let r = search("raccourcis", |_| true);
        assert!(
            r.iter()
                .any(|e| e.page == "keys" && e.tab == Some("hyprdesk"))
        );
        // Sans accents ni casse ; tous les mots.
        assert!(matches("Écran de connexion", "ecran CONNEX"));
        assert!(!matches("Écran de connexion", "ecran wifi"));
        assert_eq!(ranges("Fond d'écran", "ecran"), vec![(7, 13)]);
        assert!(search("   ", |_| true).is_empty());
        // Paquet masqué : absent.
        assert!(
            search("gamma", |p| p != "colors")
                .iter()
                .all(|e| e.page != "colors")
        );
        assert!(INDEX.len() > 300, "{}", INDEX.len());
    }

    /// Le lanceur (`:r`) trouve dans le même index, avec le filtre du tiroir.
    #[test]
    fn prefixe_du_mode_reglages_du_lanceur() {
        assert_eq!(query(":r fond"), Some("fond"));
        assert_eq!(query(":reglages fond d'écran"), Some("fond d'écran"));
        assert_eq!(query(":r"), Some(""));
        assert_eq!(query("  :r   "), Some(""));
        assert_eq!(query(":rouge"), None); // « :rouge » ≠ « :r » seul
        assert_eq!(query("fond"), None);
        assert_eq!(query(":clip x"), None);
        // Le lanceur trouve dans le même index que le tiroir ; sa liste
        // courte met les pages devant : pour « fond d'écran », la page
        // « Fond d'écran » est la première ligne.
        let r = launcher("fond d'écran", |_| true);
        assert_eq!(r[0].kind, Kind::Page, "{r:?}");
        assert!(
            r.iter()
                .any(|e| e.page == "wallpaper" && e.kind == Kind::Page),
            "{r:?}"
        );
        // Sans préfixe ni mots : rien à ouvrir.
        assert!(launcher("   ", |_| true).is_empty());
    }

    /// Un réglage trouvé vient avec son bloc : la section qui le montre.
    #[test]
    fn bloc_de_contexte() {
        let with = INDEX.iter().filter(|e| e.section.is_some()).count();
        assert!(with > 100, "{with} libellés rattachés à une section");
        let r = search("hauteur", |_| true);
        let e = r
            .iter()
            .find(|e| e.kind == Kind::Label && e.section.is_some())
            .expect("un libellé de section");
        let b = block(e);
        assert!(b.iter().any(|x| std::ptr::eq(*x, *e)));
        assert!(b.iter().all(|x| x.page == e.page && x.section == e.section));
    }
}
