//! Démarrage automatique de la session Hyprland (Réglages → Démarrage
//! automatique → Hyprland) : les commandes lancées à l'ouverture de session.
//!
//! - Celles que hyprdesk tient sont dans `autostart.lua`, à côté de
//!   hyprland.lua, dans leur propre `hl.on("hyprland.start", …)` (Hyprland
//!   accepte plusieurs abonnements au même évènement). hyprland.lua le charge
//!   par `require("autostart")`, ajouté au premier enregistrement après une
//!   copie de sauvegarde. Elles s'ajoutent, se retirent et se coupent ici.
//! - Celles de hyprland.lua (lignes `hl.exec_cmd(…)` des blocs
//!   `hl.on("hyprland.start", …)`) se coupent ou se rétablissent en
//!   commentant la ligne, sans rien toucher d'autre. Les lignes qui préparent
//!   la session (environnement transmis à systemd, démarrage de hyprdesk)
//!   restent verrouillées.
//!
//! Les démarrages XDG (`~/.config/autostart`) relèvent de `crate::autostart`.

use std::path::{Path, PathBuf};

/// Une commande lancée à l'ouverture de session.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    /// Commande affichée : le texte de la chaîne, ou l'expression Lua telle
    /// quelle (`scriptsDir .. "/x"`).
    pub command: String,
    pub enabled: bool,
    /// Commentaire en fin de ligne (`-- géré par hyprdesk`…).
    pub note: Option<String>,
    /// Ligne dans le fichier (0 = première).
    pub line: usize,
    /// Préparation de la session : ni coupée ni retirée.
    pub locked: bool,
}

/// `autostart.lua`, à côté de `hyprdesk.lua` (requis par `hyprland.lua`).
pub fn lua_path(config_path: &Path) -> PathBuf {
    dir(config_path).join("autostart.lua")
}

/// `hyprland.lua`, à côté de `hyprdesk.lua`.
pub fn hyprland_path(config_path: &Path) -> PathBuf {
    dir(config_path).join("hyprland.lua")
}

fn dir(config_path: &Path) -> &Path {
    config_path.parent().unwrap_or_else(|| Path::new("."))
}

/// Chaîne Lua entre guillemets doubles.
pub fn lua_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Texte d'une chaîne Lua simple (`"…"` ou `'…'`), sinon rien.
fn lua_literal(expr: &str) -> Option<String> {
    let e = expr.trim();
    let q = e.chars().next().filter(|c| *c == '"' || *c == '\'')?;
    let body = e.strip_prefix(q)?.strip_suffix(q)?;
    let mut out = String::new();
    let mut chars = body.chars();
    while let Some(c) = chars.next() {
        if c == q {
            // Guillemet non échappé au milieu : concaténation, pas une chaîne simple.
            return None;
        }
        if c == '\\' {
            match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                other => out.push(other),
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

/// Ligne `hl.exec_cmd(…)`, commentée ou non : (active, commande, note).
fn exec_line(line: &str) -> Option<(bool, String, Option<String>)> {
    let t = line.trim_start();
    let (enabled, body) = match t.strip_prefix("--") {
        Some(rest) => (false, rest.trim_start()),
        None => (true, t),
    };
    let args = body.strip_prefix("hl.exec_cmd(")?;
    // Parenthèse fermante, hors des chaînes.
    let (mut depth, mut quote, mut escaped, mut end) = (1usize, None::<char>, false, None);
    for (i, c) in args.char_indices() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let end = end?;
    let inner = &args[..end];
    let command = lua_literal(inner).unwrap_or_else(|| inner.trim().to_string());
    let note = args[end + 1..]
        .trim()
        .strip_prefix("--")
        .map(|n| n.trim().to_string())
        .filter(|n| !n.is_empty());
    Some((enabled, command, note))
}

/// Préparation de la session : à laisser en place.
pub fn locked(command: &str) -> bool {
    command.contains("import-environment")
        || command.contains("update-activation-environment")
        || command.contains("hyprdesk.service")
        || command.contains("hyprdesk daemon")
}

/// Commandes des blocs `hl.on("hyprland.start", …)` de hyprland.lua.
pub fn scan_hyprland(src: &str) -> Vec<Item> {
    let mut out = Vec::new();
    let mut block: Option<usize> = None;
    for (i, l) in src.lines().enumerate() {
        let t = l.trim_start();
        let indent = l.len() - t.len();
        match block {
            None => {
                if t.starts_with("hl.on(\"hyprland.start\"") || t.starts_with("hl.on('hyprland.start'") {
                    block = Some(indent);
                }
            }
            Some(b) => {
                if indent == b && t.starts_with("end") {
                    block = None;
                } else if let Some((enabled, command, note)) = exec_line(l) {
                    out.push(Item {
                        locked: locked(&command),
                        command,
                        enabled,
                        note,
                        line: i,
                    });
                }
            }
        }
    }
    out
}

/// Commandes de `autostart.lua` (toutes ses lignes `hl.exec_cmd`).
pub fn scan_ours(src: &str) -> Vec<Item> {
    src.lines()
        .enumerate()
        .filter_map(|(i, l)| {
            let (enabled, command, note) = exec_line(l)?;
            Some(Item {
                command,
                enabled,
                note,
                line: i,
                locked: false,
            })
        })
        .collect()
}

/// Contenu de `autostart.lua` : (active, commande).
pub fn render(items: &[(bool, String)]) -> String {
    let head = crate::i18n::tr_lines(
        "-- Généré par hyprdesk (Réglages → Démarrage automatique) — NE PAS ÉDITER : réécrit à chaque enregistrement.\n\
         -- Requis par hyprland.lua : require(\"autostart\"). Une ligne commentée est coupée.\n",
    );
    let mut s = format!("{head}\nhl.on(\"hyprland.start\", function()\n");
    for (enabled, cmd) in items {
        let off = if *enabled { "" } else { "-- " };
        s.push_str(&format!("    {off}hl.exec_cmd({})\n", lua_string(cmd)));
    }
    s.push_str("end)\n");
    s
}

/// Ligne qui charge `autostart.lua`.
const REQUIRE: &str = "require(\"autostart\")";

/// hyprland.lua avec `require("autostart")`, s'il n'y est pas encore : après
/// le dernier `require(…)`, sinon à la fin.
pub fn with_require(src: &str) -> Option<String> {
    let active = |l: &str| l.trim_start().starts_with(REQUIRE);
    if src.lines().any(active) {
        return None;
    }
    let line = format!(
        "{REQUIRE} {}",
        crate::i18n::tr("-- hyprdesk : démarrage automatique (Réglages → Démarrage automatique)")
    );
    let lines: Vec<&str> = src.lines().collect();
    let last_require = lines
        .iter()
        .rposition(|l| l.starts_with("require(") || l.starts_with("require "));
    let mut out: Vec<String> = lines.iter().map(|l| l.to_string()).collect();
    match last_require {
        Some(i) => out.insert(i + 1, line),
        None => {
            out.push(String::new());
            out.push(line);
        }
    }
    let mut s = out.join("\n");
    s.push('\n');
    Some(s)
}

/// hyprland.lua avec la ligne `line` coupée ou rétablie (commentaire en tête),
/// si c'est bien une ligne `hl.exec_cmd` de `command`.
pub fn with_line(src: &str, line: usize, command: &str, enabled: bool) -> Option<String> {
    let mut lines: Vec<String> = src.lines().map(str::to_string).collect();
    let l = lines.get(line)?;
    let (now, cmd, _) = exec_line(l)?;
    if cmd != command || locked(&cmd) {
        return None;
    }
    if now == enabled {
        return Some(src.to_string());
    }
    let t = l.trim_start();
    let indent = &l[..l.len() - t.len()];
    let new = if enabled {
        let rest = t.strip_prefix("--")?;
        format!("{indent}{}", rest.strip_prefix(' ').unwrap_or(rest))
    } else {
        format!("{indent}-- {t}")
    };
    lines[line] = new;
    let mut s = lines.join("\n");
    if src.ends_with('\n') {
        s.push('\n');
    }
    Some(s)
}

/// hyprland.lua sans la ligne `line`, si c'est bien une ligne `hl.exec_cmd`
/// de `command` (et pas une ligne qui prépare la session).
pub fn without_line(src: &str, line: usize, command: &str) -> Option<String> {
    let mut lines: Vec<&str> = src.lines().collect();
    let (_, cmd, _) = exec_line(lines.get(line)?)?;
    if cmd != command || locked(&cmd) {
        return None;
    }
    lines.remove(line);
    let mut s = lines.join("\n");
    if src.ends_with('\n') {
        s.push('\n');
    }
    Some(s)
}

/// Les deux listes : (hyprdesk, hyprland.lua). hyprland.lua absent : vide.
pub fn read(config_path: &Path) -> (Vec<Item>, Vec<Item>) {
    let ours = std::fs::read_to_string(lua_path(config_path))
        .map(|s| scan_ours(&s))
        .unwrap_or_default();
    let theirs = std::fs::read_to_string(hyprland_path(config_path))
        .map(|s| scan_hyprland(&s))
        .unwrap_or_default();
    (ours, theirs)
}

/// Copie de hyprland.lua avant la première modification par cette page.
fn backup(path: &Path, src: &str) -> std::io::Result<()> {
    let bak = path.with_extension("lua.avant-hyprdesk-autostart.bak");
    if !bak.exists() {
        std::fs::write(bak, src)?;
    }
    Ok(())
}

/// Enregistre `autostart.lua`, puis s'assure que hyprland.lua le charge.
pub fn save_ours(config_path: &Path, items: &[(bool, String)]) -> Result<(), String> {
    let hypr = hyprland_path(config_path);
    let src = std::fs::read_to_string(&hypr).map_err(|e| {
        crate::tr!("{p} illisible : {e}", p = hypr.display(), e = e)
    })?;
    crate::storage::write(
        &lua_path(config_path),
        render(items).as_bytes(),
        crate::storage::revision(),
    )
    .map_err(|e| e.to_string())?;
    if let Some(new) = with_require(&src) {
        backup(&hypr, &src).map_err(|e| e.to_string())?;
        std::fs::write(&hypr, new).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Coupe ou rétablit une ligne de hyprland.lua.
pub fn set_hyprland(config_path: &Path, it: &Item, enabled: bool) -> Result<(), String> {
    let hypr = hyprland_path(config_path);
    let src = std::fs::read_to_string(&hypr).map_err(|e| e.to_string())?;
    let new = with_line(&src, it.line, &it.command, enabled).ok_or_else(|| {
        crate::i18n::tr("hyprland.lua a changé entre-temps : la liste est relue.").to_string()
    })?;
    if new != src {
        backup(&hypr, &src).map_err(|e| e.to_string())?;
        std::fs::write(&hypr, new).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Supprime une ligne de hyprland.lua (copie de sauvegarde d'abord).
pub fn remove_hyprland(config_path: &Path, it: &Item) -> Result<(), String> {
    let hypr = hyprland_path(config_path);
    let src = std::fs::read_to_string(&hypr).map_err(|e| e.to_string())?;
    let new = without_line(&src, it.line, &it.command).ok_or_else(|| {
        crate::i18n::tr("hyprland.lua a changé entre-temps : la liste est relue.").to_string()
    })?;
    backup(&hypr, &src).map_err(|e| e.to_string())?;
    std::fs::write(&hypr, new).map_err(|e| e.to_string())
}

/// Lance la commande tout de suite, par Hyprland.
pub fn run_now(command: &str) -> Result<(), String> {
    crate::hypr::exec(command).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const HYPR: &str = r#"require("monitors")
require("inputs")

local x = 1
hl.on("hyprland.start", function()
    hl.exec_cmd("sh -c 'systemctl --user import-environment WAYLAND_DISPLAY; systemctl --user start hyprdesk.service'")
    hl.exec_cmd("nm-applet --indicator")
    -- hl.exec_cmd("wayle shell") -- notifications gérées par hyprdesk
    hl.exec_cmd(scriptsDir .. "/battery")
end)
hl.exec_cmd("hors-bloc")
"#;

    #[test]
    fn lignes_du_bloc_de_demarrage() {
        let items = scan_hyprland(HYPR);
        let cmds: Vec<&str> = items.iter().map(|i| i.command.as_str()).collect();
        assert_eq!(cmds.len(), 4, "{cmds:?}");
        assert!(items[0].locked);
        assert_eq!(cmds[1], "nm-applet --indicator");
        assert!(items[1].enabled && !items[1].locked);
        assert_eq!(cmds[2], "wayle shell");
        assert!(!items[2].enabled);
        assert_eq!(items[2].note.as_deref(), Some("notifications gérées par hyprdesk"));
        assert_eq!(cmds[3], "scriptsDir .. \"/battery\"");
        assert!(!cmds.contains(&"hors-bloc"));
    }

    #[test]
    fn couper_et_retablir_une_ligne() {
        let it = scan_hyprland(HYPR)[1].clone();
        let off = with_line(HYPR, it.line, &it.command, false).unwrap();
        assert!(off.contains("    -- hl.exec_cmd(\"nm-applet --indicator\")\n"));
        let back = with_line(&off, it.line, &it.command, true).unwrap();
        assert_eq!(back, HYPR);
        // Ligne verrouillée, ou commande qui ne correspond plus : rien.
        let lock = scan_hyprland(HYPR)[0].clone();
        assert!(with_line(HYPR, lock.line, &lock.command, false).is_none());
        assert!(with_line(HYPR, it.line, "autre", false).is_none());
        // Rétablir une ligne commentée par hyprdesk garde sa note.
        let w = scan_hyprland(HYPR)[2].clone();
        let on = with_line(HYPR, w.line, &w.command, true).unwrap();
        assert!(on.contains("    hl.exec_cmd(\"wayle shell\") -- notifications"));
    }

    #[test]
    fn supprimer_une_ligne() {
        let it = scan_hyprland(HYPR)[1].clone();
        let out = without_line(HYPR, it.line, &it.command).unwrap();
        assert!(!out.contains("nm-applet"));
        assert_eq!(out.lines().count(), HYPR.lines().count() - 1);
        let lock = scan_hyprland(HYPR)[0].clone();
        assert!(without_line(HYPR, lock.line, &lock.command).is_none());
    }

    #[test]
    fn fichier_de_hyprdesk_aller_retour() {
        let items = vec![
            (true, "discord --start-minimized".to_string()),
            (false, "sh -c 'echo \"x\" > /tmp/a'".to_string()),
        ];
        let src = render(&items);
        assert!(src.contains("hl.on(\"hyprland.start\", function()"));
        let back: Vec<(bool, String)> = scan_ours(&src)
            .into_iter()
            .map(|i| (i.enabled, i.command))
            .collect();
        assert_eq!(back, items);
    }

    #[test]
    fn require_ajoute_une_fois() {
        let s = with_require(HYPR).unwrap();
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines[1], "require(\"inputs\")");
        assert!(lines[2].starts_with("require(\"autostart\")"));
        assert!(with_require(&s).is_none());
        let bare = with_require("local a = 1\n").unwrap();
        assert!(bare.ends_with(&format!("{}\n", bare.lines().last().unwrap())));
        assert!(bare.lines().last().unwrap().starts_with("require(\"autostart\")"));
    }
}
