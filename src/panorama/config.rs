//! Configuration du Panorama : `hd.config({ panorama = { … } })`.
//!
//! ```lua
//! hd.config({ panorama = {
//!   screens  = { "DP-3" },            -- connecteur ou identifiant EDID
//!   pages    = { "media", "system", "clock" },
//!   declined = { "Acme M 1" },        -- écrans refusés (identifiant EDID)
//!   sleep_minutes = 5,                -- 0 : jamais d'horloge de veille
//! } })
//! ```

use mlua::{Table, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct Config {
    /// Écrans au rôle Panorama : connecteur (`DP-3`) ou identifiant EDID.
    pub screens: Vec<String>,
    /// Pages affichées, dans l'ordre.
    pub pages: Vec<String>,
    /// Écrans dont la proposition a été refusée (identifiant EDID).
    pub declined: Vec<String>,
    /// Minutes sans lecture ni toucher avant l'horloge de veille (0 : jamais).
    pub sleep_minutes: u32,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            screens: Vec::new(),
            pages: super::PAGES.iter().map(|(id, _)| id.to_string()).collect(),
            declined: Vec::new(),
            sleep_minutes: 5,
        }
    }
}

const CTX: &str = "hd.config.panorama";

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

/// Lit la table `panorama` ; les clés absentes gardent leur défaut.
pub fn from_lua(t: &Table) -> mlua::Result<Config> {
    let mut c = Config::default();
    for pair in t.pairs::<Value, Value>() {
        let (k, _) = pair?;
        let Value::String(k) = k else { continue };
        let k = k.to_str()?.to_string();
        if !["screens", "pages", "declined", "sleep_minutes"].contains(&k.as_str()) {
            return Err(mlua::Error::RuntimeError(crate::tr!(
                "{CTX} : clé inconnue « {k} » (screens, pages, declined, sleep_minutes)",
                CTX = CTX,
                k = k
            )));
        }
    }
    if let Some(l) = list(t, "screens")? {
        c.screens = l;
    }
    if let Some(l) = list(t, "pages")? {
        let known: Vec<String> = l
            .into_iter()
            .filter(|p| super::PAGES.iter().any(|(id, _)| id == p))
            .collect();
        if !known.is_empty() {
            c.pages = known;
        }
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
    /// Table Lua (relue par [`from_lua`]).
    pub fn to_lua(&self) -> String {
        let mut f = vec![format!("screens = {}", lua_list(&self.screens))];
        if self.pages != Config::default().pages {
            f.push(format!("pages = {}", lua_list(&self.pages)));
        }
        if !self.declined.is_empty() {
            f.push(format!("declined = {}", lua_list(&self.declined)));
        }
        f.push(format!("sleep_minutes = {}", self.sleep_minutes));
        format!("{{ {} }}", f.join(", "))
    }

    /// L'écran `s` a le rôle Panorama.
    pub fn has(&self, s: &super::detect::Screen) -> bool {
        self.screens.iter().any(|e| s.matches(e))
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
            declined: vec!["Acme \"M\" 1".into()],
            sleep_minutes: 0,
        };
        assert_eq!(eval(&c.to_lua()).unwrap(), c);
        assert_eq!(
            eval(&Config::default().to_lua()).unwrap(),
            Config::default()
        );
    }

    #[test]
    fn erreurs_et_pages_inconnues() {
        assert!(eval("{ screen = {} }").is_err());
        assert!(eval("{ sleep_minutes = 'x' }").is_err());
        assert!(eval("{ screens = 3 }").is_err());
        let c = eval("{ pages = { 'radio', 'clock' }, screens = 'DP-1' }").unwrap();
        assert_eq!(c.pages, vec!["clock".to_string()]);
        assert_eq!(c.screens, vec!["DP-1".to_string()]);
        assert_eq!(
            eval("{ pages = { 'x' } }").unwrap().pages,
            Config::default().pages
        );
    }
}
