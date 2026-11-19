//! Configuration du Panorama : `hd.config({ panorama = { … } })`.
//!
//! ```lua
//! hd.config({ panorama = {
//!   screens  = { "DP-3" },            -- connecteur ou identifiant EDID
//!   pages    = { "media", "system", "clock", "video" },
//!   layout   = "split2",              -- single | split2 | split3
//!   panes    = { "media", "video", "system" }, -- widget de chaque zone
//!   sources  = { { name = "Free TV", url = "https://tv.free.fr" } },
//!   browser  = "brave",               -- vide : le premier navigateur Chromium trouvé
//!   declined = { "Acme M 1" },        -- écrans refusés (identifiant EDID)
//!   sleep_minutes = 5,                -- 0 : jamais d'horloge de veille
//! } })
//! ```

use mlua::{Table, Value};

/// Disposition de l'écran : une page à la fois, ou 2 ou 3 zones côte à côte.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Layout {
    #[default]
    Single,
    Split2,
    Split3,
    Dashboard,
}

impl Layout {
    pub const ALL: [Layout; 4] = [
        Layout::Single,
        Layout::Split2,
        Layout::Split3,
        Layout::Dashboard,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Layout::Single => "single",
            Layout::Split2 => "split2",
            Layout::Split3 => "split3",
            Layout::Dashboard => "dashboard",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s.trim() {
            "single" | "1" => Layout::Single,
            "split2" | "2" => Layout::Split2,
            "split3" | "3" => Layout::Split3,
            "dashboard" | "4" => Layout::Dashboard,
            _ => return None,
        })
    }

    /// Nombre de zones.
    pub fn panes(self) -> usize {
        match self {
            Layout::Single => 1,
            Layout::Split2 => 2,
            Layout::Split3 => 3,
            Layout::Dashboard => 4,
        }
    }
}

/// Source vidéo : un service ouvert en fenêtre d'application du navigateur.
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    pub name: String,
    pub url: String,
}

/// Sources par défaut (modifiables dans Réglages → Panorama).
pub fn default_sources() -> Vec<Source> {
    [
        ("Free TV", "https://tv.free.fr"),
        ("Netflix", "https://www.netflix.com"),
        ("Disney+", "https://www.disneyplus.com"),
        ("HBO Max", "https://play.hbomax.com"),
        ("Canal+", "https://www.canalplus.com"),
        ("Prime Video", "https://www.primevideo.com"),
        ("YouTube", "https://www.youtube.com"),
    ]
    .iter()
    .map(|(n, u)| Source {
        name: n.to_string(),
        url: u.to_string(),
    })
    .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// Écrans au rôle Panorama : connecteur (`DP-3`) ou identifiant EDID.
    pub screens: Vec<String>,
    /// Pages affichées une à une (disposition `single`), dans l'ordre.
    pub pages: Vec<String>,
    pub layout: Layout,
    /// Widget de chaque zone (dispositions `split2` et `split3`).
    pub panes: Vec<String>,
    /// Sources du widget Vidéo.
    pub sources: Vec<Source>,
    /// Navigateur des sources vidéo (vide : détection automatique).
    pub browser: String,
    /// Écrans dont la proposition a été refusée (identifiant EDID).
    pub declined: Vec<String>,
    /// Minutes sans lecture ni toucher avant l'horloge de veille (0 : jamais).
    pub sleep_minutes: u32,
    pub design: super::design::Design,
}

/// Widgets des zones par défaut.
pub fn default_panes() -> Vec<String> {
    ["media", "video", "system"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            screens: Vec::new(),
            pages: super::PAGES.iter().map(|(id, _)| id.to_string()).collect(),
            layout: Layout::Single,
            panes: default_panes(),
            sources: default_sources(),
            browser: String::new(),
            declined: Vec::new(),
            sleep_minutes: 5,
            design: Default::default(),
        }
    }
}

const CTX: &str = "hd.config.panorama";
const KEYS: &[&str] = &[
    "screens",
    "pages",
    "layout",
    "panes",
    "sources",
    "browser",
    "declined",
    "sleep_minutes",
    "design",
];

fn list(t: &Table, key: &str) -> mlua::Result<Option<Vec<String>>> {
    match t.get::<Value>(key)? {
        Value::Nil => Ok(None),
        Value::Table(l) => {
            let mut out = Vec::new();
            for v in l.sequence_values::<String>() {
                let v = v?;
                if !v.trim().is_empty() {
                    out.push(v.trim().to_string());
                }
            }
            Ok(Some(out))
        }
        Value::String(s) => Ok(Some(vec![s.to_str()?.trim().to_string()])),
        v => Err(mlua::Error::RuntimeError(crate::tr!(
            "{CTX} : « {key} » attend une liste de chaînes, pas {p0}",
            CTX = CTX,
            key = key.to_string(),
            p0 = v.type_name()
        ))),
    }
}

fn known_widget(id: &str) -> bool {
    super::PAGES.iter().any(|(p, _)| *p == id)
}

/// Lit la table `panorama` ; les clés absentes gardent leur défaut.
pub fn from_lua(t: &Table) -> mlua::Result<Config> {
    let mut c = Config::default();
    for pair in t.pairs::<Value, Value>() {
        let (k, _) = pair?;
        let Value::String(k) = k else { continue };
        let k = k.to_str()?.to_string();
        if !KEYS.contains(&k.as_str()) {
            return Err(mlua::Error::RuntimeError(crate::tr!(
                "{CTX} : clé inconnue « {k} » ({p0})",
                CTX = CTX,
                k = k,
                p0 = KEYS.join(", ")
            )));
        }
    }
    if let Some(l) = list(t, "screens")? {
        c.screens = l;
    }
    if let Some(l) = list(t, "pages")? {
        let known: Vec<String> = l.into_iter().filter(|p| known_widget(p)).collect();
        if !known.is_empty() {
            c.pages = known;
        }
    }
    if let Some(s) = t.get::<Option<String>>("layout")? {
        c.layout = Layout::parse(&s).ok_or_else(|| {
            mlua::Error::RuntimeError(crate::tr!(
                "{CTX} : « layout » attend single, split2 ou split3",
                CTX = CTX
            ))
        })?;
    }
    if let Some(l) = list(t, "panes")? {
        let mut panes = default_panes();
        for (i, p) in l.into_iter().take(3).enumerate() {
            if known_widget(&p) {
                panes[i] = p;
            }
        }
        c.panes = panes;
    }
    if let Some(l) = t.get::<Option<Table>>("sources")? {
        let mut out = Vec::new();
        for v in l.sequence_values::<Table>() {
            let v = v?;
            let name: String = v.get::<Option<String>>("name")?.unwrap_or_default();
            let url: String = v.get::<Option<String>>("url")?.unwrap_or_default();
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(mlua::Error::RuntimeError(crate::tr!(
                    "{CTX} : source « {name} » : l'adresse doit commencer par https://",
                    CTX = CTX,
                    name = name
                )));
            }
            out.push(Source {
                name: if name.trim().is_empty() {
                    url.clone()
                } else {
                    name.trim().to_string()
                },
                url: url.trim().to_string(),
            });
        }
        c.sources = out;
    }
    if let Some(b) = t.get::<Option<String>>("browser")? {
        c.browser = b.trim().to_string();
    }
    if let Some(l) = list(t, "declined")? {
        c.declined = l;
    }
    match t.get::<Value>("sleep_minutes")? {
        Value::Nil => {}
        Value::Integer(n) if n >= 0 => c.sleep_minutes = n as u32,
        Value::Number(n) if n >= 0.0 => c.sleep_minutes = n as u32,
        _ => {
            return Err(mlua::Error::RuntimeError(crate::tr!(
                "{CTX} : « sleep_minutes » attend un nombre de minutes (0 : jamais)",
                CTX = CTX
            )));
        }
    }
    if let Some(t) = t.get::<Option<Table>>("design")? {
        c.design = super::design::from_lua(&t)?;
    }
    if t.get::<Value>("layout")?.is_nil() && c.design.preset != "cinematic" {
        c.layout = Layout::Dashboard;
    }
    Ok(c)
}

fn q(s: &str) -> String {
    format!("\"{}\"", hyprpanel_core::ipc::lua_escape(s))
}

fn lua_list(l: &[String]) -> String {
    format!(
        "{{ {} }}",
        l.iter().map(|s| q(s)).collect::<Vec<_>>().join(", ")
    )
}

impl Config {
    /// Table Lua (relue par [`from_lua`]) ; les valeurs par défaut sont omises.
    pub fn to_lua(&self) -> String {
        let d = Config::default();
        let mut f = vec![format!("screens = {}", lua_list(&self.screens))];
        if self.pages != d.pages {
            f.push(format!("pages = {}", lua_list(&self.pages)));
        }
        if self.layout != d.layout {
            f.push(format!("layout = {}", q(self.layout.id())));
        }
        if self.panes != d.panes {
            f.push(format!("panes = {}", lua_list(&self.panes)));
        }
        if self.sources != d.sources {
            let s: Vec<String> = self
                .sources
                .iter()
                .map(|s| format!("{{ name = {}, url = {} }}", q(&s.name), q(&s.url)))
                .collect();
            f.push(format!("sources = {{ {} }}", s.join(", ")));
        }
        if !self.browser.is_empty() {
            f.push(format!("browser = {}", q(&self.browser)));
        }
        if !self.declined.is_empty() {
            f.push(format!("declined = {}", lua_list(&self.declined)));
        }
        f.push(format!("sleep_minutes = {}", self.sleep_minutes));
        if self.design != d.design {
            f.push(format!("design = {}", self.design.to_lua()));
        }
        format!("{{ {} }}", f.join(", "))
    }

    /// L'écran `s` a le rôle Panorama.
    pub fn has(&self, s: &super::detect::Screen) -> bool {
        self.screens.iter().any(|e| s.matches(e))
    }

    /// Widget de la zone `i` (le premier connu par défaut).
    pub fn pane(&self, i: usize) -> &str {
        self.panes
            .get(i)
            .map(String::as_str)
            .unwrap_or(super::PAGES[0].0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval(src: &str) -> mlua::Result<Config> {
        let lua = mlua::Lua::new();
        let t: Table = lua.load(src).eval()?;
        from_lua(&t)
    }

    #[test]
    fn aller_retour() {
        let c = Config {
            screens: vec!["DP-3".into(), "Corsair XENEON EDGE S1".into()],
            pages: vec!["clock".into(), "media".into()],
            layout: Layout::Split3,
            panes: vec!["video".into(), "clock".into(), "media".into()],
            sources: vec![Source {
                name: "Free \"TV\"".into(),
                url: "https://tv.free.fr".into(),
            }],
            browser: "chromium".into(),
            declined: vec!["Acme \"M\" 1".into()],
            sleep_minutes: 0,
            design: Default::default(),
        };
        assert_eq!(eval(&c.to_lua()).unwrap(), c);
        assert_eq!(
            eval(&Config::default().to_lua()).unwrap(),
            Config::default()
        );
        // Les défauts ne sont pas écrits.
        assert!(!Config::default().to_lua().contains("sources"));
    }

    #[test]
    fn erreurs_et_pages_inconnues() {
        assert!(eval("{ screen = {} }").is_err());
        assert!(eval("{ sleep_minutes = 'x' }").is_err());
        assert!(eval("{ screens = 3 }").is_err());
        assert!(eval("{ layout = 'split9' }").is_err());
        assert!(eval("{ sources = { { name = 'x', url = 'file:///etc' } } }").is_err());
        let c = eval("{ pages = { 'radio', 'clock' }, screens = 'DP-1' }").unwrap();
        assert_eq!(c.pages, vec!["clock".to_string()]);
        assert_eq!(c.screens, vec!["DP-1".to_string()]);
        assert_eq!(
            eval("{ pages = { 'x' } }").unwrap().pages,
            Config::default().pages
        );
        // Zones : inconnues remplacées par le défaut, trois au plus.
        let c = eval("{ panes = { 'clock', 'radio' } }").unwrap();
        assert_eq!(c.panes, vec!["clock", "video", "system"]);
        assert_eq!(c.pane(0), "clock");
    }
}
