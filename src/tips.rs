//! Astuces du shell (HIG DAT-07) : une ou deux phrases sur une fonction,
//! montrées dans sa surface, une fois par fonction, au plus une par jour, et
//! seulement si la fonction n'a pas encore servi.
//!
//! L'état tient dans `$XDG_STATE_HOME/hyprdesk/tips`, une ligne par fait :
//! `shown <id> <époque>` (astuce montrée), `used <id>` (fonction utilisée).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// Surface où une astuce s'affiche.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Launcher,
    Overview,
    /// Bulle du dock, au survol d'une application.
    Dock,
}

/// Une astuce : identifiant stable, surface, symbole de la fonction, texte.
#[derive(Debug, PartialEq, Eq)]
pub struct Tip {
    pub id: &'static str,
    pub surface: Surface,
    pub icon: &'static str,
    pub text: &'static str,
}

/// Catalogue, dans l'ordre où les astuces d'une surface sont proposées.
pub const TIPS: &[Tip] = &[
    Tip {
        id: "lanceur-fichiers",
        surface: Surface::Launcher,
        icon: "system-search",
        text: "Tapez :f puis un nom pour trouver un fichier, sur tous vos disques.",
    },
    Tip {
        id: "lanceur-reglages",
        surface: Surface::Launcher,
        icon: "preferences-system",
        text: "Tapez :r puis un mot pour aller droit à un réglage.",
    },
    Tip {
        id: "lanceur-presse-papiers",
        surface: Surface::Launcher,
        icon: "edit-paste-symbolic",
        text: "Tapez :clip pour retrouver ce que vous avez copié.",
    },
    Tip {
        id: "apercu-glisser",
        surface: Surface::Overview,
        icon: "view-grid",
        text: "Glissez une fenêtre sur un autre espace pour l'y déplacer.",
    },
    Tip {
        id: "dock-clic-droit",
        surface: Surface::Dock,
        icon: "view-more",
        text: "Faites un clic droit sur une icône pour ses actions : nouvelle fenêtre, épingler au dock ou l'en retirer.",
    },
];

/// Écart minimal entre deux astuces, toutes surfaces confondues.
pub const CADENCE: i64 = 24 * 3600;

/// Ce qui a été montré et ce qui a servi.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct State {
    pub shown: BTreeMap<String, i64>,
    pub used: BTreeSet<String>,
}

pub fn state_path() -> PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state")
        })
        .join("hyprdesk")
        .join("tips")
}

/// Secondes depuis l'époque.
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

impl State {
    /// Lit l'état ; fichier absent ou illisible : vide. Lignes inconnues ignorées.
    pub fn parse(s: &str) -> Self {
        let mut st = State::default();
        for l in s.lines() {
            let mut w = l.split_whitespace();
            match (w.next(), w.next(), w.next()) {
                (Some("shown"), Some(id), Some(t)) => {
                    if let Ok(t) = t.parse() {
                        st.shown.insert(id.to_string(), t);
                    }
                }
                (Some("used"), Some(id), None) => {
                    st.used.insert(id.to_string());
                }
                _ => {}
            }
        }
        st
    }

    pub fn render(&self) -> String {
        let mut out = String::new();
        for (id, t) in &self.shown {
            out.push_str(&format!("shown {id} {t}\n"));
        }
        for id in &self.used {
            out.push_str(&format!("used {id}\n"));
        }
        out
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .map(|s| Self::parse(&s))
            .unwrap_or_default()
    }

    /// Réunit un autre état à celui-ci : la date la plus récente d'une
    /// astuce, toutes les fonctions servies. L'ordre des écritures et une
    /// autre instance ne font rien perdre : l'union ne recule jamais.
    pub fn merge(&mut self, other: &State) {
        for (id, &t) in &other.shown {
            let e = self.shown.entry(id.clone()).or_insert(t);
            *e = (*e).max(t);
        }
        self.used.extend(other.used.iter().cloned());
    }

    /// Écrit l'union de cet état et du fichier (temporaire puis renommage).
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        let mut all = Self::load(path);
        all.merge(self);
        // Un temporaire par écriture : deux sauvegardes proches ne se gênent pas.
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let tmp = path.with_extension(format!("tmp-{}-{n}", std::process::id()));
        std::fs::write(&tmp, all.render())?;
        std::fs::rename(tmp, path)
    }

    /// Astuce à montrer sur `surface` à l'instant `now` : la première ni
    /// montrée ni déjà servie, si aucune astuce n'est apparue depuis
    /// [`CADENCE`].
    pub fn eligible(&self, surface: Surface, now: i64) -> Option<&'static Tip> {
        if self.shown.values().any(|&t| now - t < CADENCE) {
            return None;
        }
        TIPS.iter().find(|t| {
            t.surface == surface && !self.shown.contains_key(t.id) && !self.used.contains(t.id)
        })
    }

    pub fn mark_shown(&mut self, id: &str, now: i64) {
        self.shown.insert(id.to_string(), now);
    }

    /// La fonction a servi : son astuce ne sera plus proposée. `true` si
    /// c'est nouveau (l'état est à écrire).
    pub fn mark_used(&mut self, id: &str) -> bool {
        self.used.insert(id.to_string())
    }
}

/// Astuce du lanceur qu'une requête met en œuvre (`:f …`, `:clip …` ; `:r`
/// est reconnu par le lanceur, qui tient l'index des réglages).
pub fn launcher_use(query: &str) -> Option<&'static str> {
    if crate::fileindex::proto::query(query).is_some() {
        Some("lanceur-fichiers")
    } else if crate::clipboard::query(query).is_some() {
        Some("lanceur-presse-papiers")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_par_jour_une_fois_par_fonction() {
        let mut st = State::default();
        let t0 = 1_700_000_000;
        let a = st.eligible(Surface::Launcher, t0).unwrap();
        assert_eq!(a.id, "lanceur-fichiers");
        st.mark_shown(a.id, t0);
        // Le même jour : plus rien, même sur une autre surface.
        assert!(st.eligible(Surface::Launcher, t0 + 3600).is_none());
        assert!(st.eligible(Surface::Overview, t0 + 3600).is_none());
        // Le lendemain : la suivante, jamais la même.
        let b = st.eligible(Surface::Launcher, t0 + CADENCE).unwrap();
        assert_eq!(b.id, "lanceur-reglages");
        // Une fonction déjà servie n'a pas d'astuce.
        assert!(st.mark_used("lanceur-reglages"));
        assert!(!st.mark_used("lanceur-reglages"));
        let c = st.eligible(Surface::Launcher, t0 + CADENCE).unwrap();
        assert_eq!(c.id, "lanceur-presse-papiers");
        st.mark_used("lanceur-presse-papiers");
        assert!(st.eligible(Surface::Launcher, t0 + CADENCE).is_none());
        assert_eq!(
            st.eligible(Surface::Overview, t0 + CADENCE).map(|t| t.id),
            Some("apercu-glisser")
        );
        assert_eq!(
            st.eligible(Surface::Dock, t0 + CADENCE).map(|t| t.id),
            Some("dock-clic-droit")
        );
        st.mark_used("dock-clic-droit");
        assert!(st.eligible(Surface::Dock, t0 + CADENCE).is_none());
    }

    #[test]
    fn etat_lu_et_ecrit() {
        let mut st = State::default();
        st.mark_shown("lanceur-fichiers", 42);
        st.mark_used("apercu-glisser");
        let txt = st.render();
        assert_eq!(State::parse(&txt), st);
        assert_eq!(
            State::parse("bruit\nshown x pas-un-nombre\nused y\n")
                .used
                .len(),
            1
        );
        let dir = std::env::temp_dir().join(format!("hyprdesk-tips-{}", std::process::id()));
        let p = dir.join("hyprdesk/tips");
        st.save(&p).unwrap();
        assert_eq!(State::load(&p), st);
        // Une écriture plus ancienne ne fait rien reculer.
        let mut vieux = State::default();
        vieux.mark_shown("lanceur-fichiers", 7);
        vieux.save(&p).unwrap();
        assert_eq!(State::load(&p), st);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn requetes_du_lanceur() {
        assert_eq!(launcher_use(":f rapport"), Some("lanceur-fichiers"));
        assert_eq!(launcher_use(":clip"), Some("lanceur-presse-papiers"));
        assert_eq!(launcher_use("firefox"), None);
    }

    /// Textes du catalogue traduits.
    #[test]
    fn textes_traduits() {
        let missing = crate::i18n::untranslated("astuces", TIPS.iter().map(|t| t.text));
        assert!(missing.is_empty(), "sans traduction : {missing:#?}");
    }
}
