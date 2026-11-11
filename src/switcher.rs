//! Sélecteur de fenêtres (MLG-22) : Alt+Tab natif. Les fenêtres viennent de
//! `hyprctl clients -j`, dans l'ordre du dernier focus (`focusHistoryID`) ;
//! celles que hyprdesk a mises de côté (workspace spécial du bureau) et les
//! fenêtres masquées sont écartées.

use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct Win {
    pub address: String,
    pub class: String,
    pub title: String,
    pub workspace: String,
    pub focus: i64,
    /// Mise de côté par le plein bureau (`special:hyprdesk`).
    pub stashed: bool,
}

/// Fenêtres dans l'ordre du focus ; `stashed` : aussi celles que le plein
/// bureau a mises de côté (`special:hyprdesk`), marquées.
pub fn parse_with(v: &Value, stashed: bool) -> Vec<Win> {
    let mut out: Vec<Win> = v
        .as_array()
        .map(|a| a.as_slice())
        .unwrap_or_default()
        .iter()
        .filter(|c| {
            c["mapped"].as_bool().unwrap_or(true) && !c["hidden"].as_bool().unwrap_or(false)
        })
        .filter_map(|c| {
            let ws = c["workspace"]["name"]
                .as_str()
                .unwrap_or_default()
                .to_string();
            let is_stash = ws == crate::hypr::STASH;
            if ws.starts_with("special:hyprdesk") && !(stashed && is_stash) {
                return None;
            }
            Some(Win {
                stashed: is_stash,
                address: c["address"].as_str()?.to_string(),
                class: c["class"].as_str().unwrap_or_default().to_string(),
                title: c["title"].as_str().unwrap_or_default().to_string(),
                workspace: ws,
                focus: c["focusHistoryID"].as_i64().unwrap_or(i64::MAX),
            })
        })
        .collect();
    out.sort_by_key(|w| w.focus);
    out
}

/// Fenêtres, relevé bloquant.
/// Comme `read`, avec les fenêtres mises de côté par le plein bureau.
pub fn read_with(stashed: bool) -> Vec<Win> {
    hyprpanel_core::ipc::query_json("clients")
        .map(|v| parse_with(&v, stashed))
        .unwrap_or_default()
}

/// Sélection de départ : la fenêtre d'avant (Alt+Tab), ou la plus ancienne
/// (Alt+Maj+Tab).
pub fn initial(n: usize, back: bool) -> usize {
    match (n, back) {
        (0, _) => 0,
        (1, _) => 0,
        (_, false) => 1,
        (n, true) => n - 1,
    }
}

/// Le même raccourci avec Maj (« ALT + Tab » → « ALT + SHIFT + Tab »).
pub fn with_shift(combo: &str) -> Option<String> {
    let parts: Vec<&str> = combo
        .split('+')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let (key, mods) = parts.split_last()?;
    if mods.iter().any(|m| m.eq_ignore_ascii_case("SHIFT")) {
        return None;
    }
    let mut v: Vec<&str> = mods.to_vec();
    v.push("SHIFT");
    v.push(key);
    Some(v.join(" + "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordre_du_focus() {
        let v: Value = serde_json::from_str(
            r#"[
              {"address":"0x2","class":"kitty","title":"t","workspace":{"name":"1"},"focusHistoryID":1,"mapped":true,"hidden":false},
              {"address":"0x1","class":"firefox","title":"f","workspace":{"name":"2"},"focusHistoryID":0,"mapped":true,"hidden":false},
              {"address":"0x3","class":"x","title":"cachée","workspace":{"name":"special:hyprdesk-bureau"},"focusHistoryID":2},
              {"address":"0x4","class":"y","title":"masquée","workspace":{"name":"3"},"focusHistoryID":3,"hidden":true}
            ]"#,
        )
        .unwrap();
        let w = parse_with(&v, false);
        let a: Vec<&str> = w.iter().map(|w| w.address.as_str()).collect();
        assert_eq!(a, ["0x1", "0x2"]);
        // Plein bureau : les fenêtres mises de côté sont proposées, marquées ;
        // celles parties avec le bureau (hyprdesk-bureau) jamais.
        let v2: Value = serde_json::from_str(
            r#"[
              {"address":"0x5","class":"kitty","title":"cachée","workspace":{"name":"special:hyprdesk"},"focusHistoryID":0},
              {"address":"0x3","class":"x","title":"bureau","workspace":{"name":"special:hyprdesk-bureau"},"focusHistoryID":1}
            ]"#,
        )
        .unwrap();
        assert!(parse_with(&v2, false).is_empty());
        let w = parse_with(&v2, true);
        assert_eq!(w.len(), 1);
        assert!(w[0].stashed && w[0].address == "0x5");
    }

    #[test]
    fn depart() {
        assert_eq!(initial(0, false), 0);
        assert_eq!(initial(1, false), 0);
        assert_eq!(initial(4, false), 1);
        assert_eq!(initial(4, true), 3);
    }

    #[test]
    fn avec_maj() {
        assert_eq!(
            with_shift("ALT + Tab").as_deref(),
            Some("ALT + SHIFT + Tab")
        );
        assert_eq!(
            with_shift("SUPER+Tab").as_deref(),
            Some("SUPER + SHIFT + Tab")
        );
        assert_eq!(with_shift("ALT + SHIFT + Tab"), None);
    }
}
