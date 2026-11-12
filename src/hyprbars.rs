//! Hyprbars (plugin de Hyprland, dépôt hyprwm/hyprland-plugins) : barres de
//! titre des fenêtres, seulement en plein bureau.
//!
//! Option `hyprbars` (coupée par défaut) : en entrant en full-desktop,
//! hyprdesk charge le plugin (`hyprctl plugin load`) ; en sortant, il le
//! décharge. Le plugin est cherché là où `hyprpm` le construit et dans les
//! dossiers de plugins des distributions, ou au chemin `hyprbars_path`.
//!
//! Boutons de la barre de titre (`hl.plugin.hyprbars.add_button`, de droite
//! à gauche) : fermer, agrandir, réduire. Le plugin les oublie quand il est
//! déchargé et à chaque relecture de la config : hyprdesk les repose après
//! le chargement et après une relecture.

use std::path::{Path, PathBuf};

/// Emplacements connus de `hyprbars.so`.
fn candidates() -> Vec<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let mut v = Vec::new();
    // hyprpm : /var/cache/hyprpm/<utilisateur>/hyprland-plugins/hyprbars.so
    if let Ok(rd) = std::fs::read_dir("/var/cache/hyprpm") {
        for e in rd.flatten() {
            v.push(e.path().join("hyprland-plugins").join("hyprbars.so"));
        }
    }
    for d in [
        "/usr/lib/hyprland/plugins",
        "/usr/lib/hyprland-plugins",
        "/usr/lib64/hyprland/plugins",
        "/usr/local/lib/hyprland/plugins",
    ] {
        v.push(Path::new(d).join("hyprbars.so"));
        v.push(Path::new(d).join("libhyprbars.so"));
    }
    v.push(home.join(".local/share/hyprland/plugins/hyprbars.so"));
    v.push(home.join(".local/lib/hyprbars.so"));
    v
}

/// Chemin du plugin : celui réglé s'il existe, sinon le premier trouvé.
pub fn find(custom: &str) -> Option<PathBuf> {
    let custom = custom.trim();
    if !custom.is_empty() {
        let p = PathBuf::from(crate::apptheme::expand_home(custom));
        return p.is_file().then_some(p);
    }
    candidates().into_iter().find(|p| p.is_file())
}

fn hyprctl(args: &[&str]) -> Result<String, String> {
    let out = std::process::Command::new("hyprctl")
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// Le plugin est chargé dans Hyprland.
pub fn loaded() -> bool {
    hyprctl(&["plugin", "list"]).is_ok_and(|s| s.to_lowercase().contains("hyprbars"))
}

/// Charge le plugin (rien s'il l'est déjà).
pub fn load(path: &Path) -> Result<(), String> {
    if loaded() {
        return Ok(());
    }
    let r = hyprctl(&["plugin", "load", &path.display().to_string()])?;
    if r == "ok" || loaded() {
        Ok(())
    } else {
        Err(crate::tr!("Hyprbars : {r}", r = r))
    }
}

/// Décharge le plugin (rien s'il ne l'est pas).
pub fn unload(path: &Path) -> Result<(), String> {
    if !loaded() {
        return Ok(());
    }
    let r = hyprctl(&["plugin", "unload", &path.display().to_string()])?;
    if r == "ok" || !loaded() {
        // Boutons partis avec le plugin : la prochaine pose doit avoir lieu.
        let _ = hyprpanel_core::ipc::eval(&format!("{MARK} = nil"));
        Ok(())
    } else {
        Err(crate::tr!("Hyprbars : {r}", r = r))
    }
}

/// Lua de la barre et de ses boutons : fermer (le plus à droite), agrandir,
/// réduire, à la manière des fenêtres de bureau (symboles seuls, fond
/// transparent), aux couleurs du thème (`fg` texte, `bg` barre). Le clic
/// donne d'abord le focus à la fenêtre : les commandes visent l'active.
pub fn buttons_lua(
    exe: &str,
    fg: crate::config::Rgba,
    bg: crate::config::Rgba,
    [close, maximize]: [&str; 2],
) -> String {
    use hyprpanel_core::ipc::lua_escape;
    let hex = |c: crate::config::Rgba| format!("{:02x}{:02x}{:02x}", c.0, c.1, c.2);
    let (fg, bg) = (hex(fg), hex(bg));
    let minimize_cmd = format!("{} window minimize", crate::launch::sh_quote(exe));
    // Réduire : « ⎽ » (trait bas d'une police de texte) un peu plus grand, ce
    // qui le pose au bas du carré et de la croix (mesuré sur l'écran : le
    // plugin centre chaque symbole, taille = 62 % du bouton).
    let add = [
        (close, BUTTON, "hyprctl dispatch 'hl.dsp.window.close()'".to_string()),
        (
            maximize,
            BUTTON,
            r#"hyprctl dispatch 'hl.dsp.window.fullscreen({ mode = "maximized", action = "toggle" })'"#
                .to_string(),
        ),
        ("⎽", MINIMIZE, minimize_cmd),
    ]
    .iter()
    .map(|(icon, size, action)| {
        format!(
            "hl.plugin.hyprbars.add_button({{ bg_color = \"rgba(00000000)\", fg_color = \"rgb({fg})\", size = {size}, icon = \"{icon}\", action = \"{}\" }})",
            lua_escape(action)
        )
    })
    .collect::<Vec<_>>()
    .join("\n");
    // Barre assez haute pour des boutons lisibles (symbole : 62 % du bouton).
    // Réglage refusé (option inconnue d'une autre version) : les boutons
    // restent posés.
    let style = format!(
        "pcall(hl.config, {{ plugin = {{ hyprbars = {{ bar_height = {HEIGHT}, bar_text_size = 11, bar_padding = 8, bar_button_padding = 2, bar_color = \"rgb({bg})\", [\"col.text\"] = \"rgb({fg})\" }} }} }})"
    );
    // Le plugin n'a pas de retrait : une seule pose par cycle. Une relecture
    // de la config efface ses boutons et l'état Lua (donc la marque) ; le
    // déchargement, lui, efface la marque explicitement (`unload`).
    // Fenêtres étiquetées par hyprdesk (écrans hors plein bureau) : sans barre.
    let rule = format!(
        "pcall(function() return hl.window_rule({{ name = \"hyprdesk-nobar\", match = {{ tag = \"{NOBAR}\" }}, [\"hyprbars:no_bar\"] = true }}) end)"
    );
    format!(
        "if hl.plugin and hl.plugin.hyprbars and not {MARK} then\n{style}\n{rule}\n{add}\n{MARK} = true\nend"
    )
}

/// Étiquette des fenêtres sans barre : le plugin est global, mais seuls les
/// écrans en plein bureau en veulent une.
pub const NOBAR: &str = "hyprdesk-nobar";

/// Lua qui pose (`add`) ou retire (`remove`) l'étiquette. Changer une
/// étiquette fait réévaluer les règles de la fenêtre, donc sa barre ;
/// `force` la retire d'abord, pour une règle qui vient d'être (re)créée.
pub fn tags_lua(add: &[String], remove: &[String], force: bool) -> String {
    use hyprpanel_core::ipc::lua_escape;
    let one = |sign: char, a: &str| {
        format!(
            "pcall(function() hl.dispatch(hl.dsp.window.tag({{ tag = \"{sign}{NOBAR}\", window = \"address:{}\" }})) end)\n",
            lua_escape(a)
        )
    };
    let mut lua = String::new();
    for a in remove {
        lua += &one('-', a);
    }
    for a in add {
        if force {
            lua += &one('-', a);
        }
        lua += &one('+', a);
    }
    lua
}

/// Pose ou retire l'étiquette (rien à faire : aucun appel).
pub fn set_tags(add: &[String], remove: &[String], force: bool) {
    if add.is_empty() && remove.is_empty() {
        return;
    }
    if let Err(e) = hyprpanel_core::ipc::eval(&tags_lua(add, remove, force)) {
        tracing::warn!("{}", crate::tr!("Hyprbars, étiquettes : {e}", e = e));
    }
}

/// Symboles de fermer et d'agrandir, selon les polices installées (le texte
/// de la barre se replie sur elles) : icônes Material Design d'une Nerd Font
/// (même case, même bas), sinon celles d'Unicode (Noto Sans Symbols 2),
/// sinon des caractères courants.
pub fn glyphs() -> [&'static str; 2] {
    let has = |chars: &str| {
        std::process::Command::new("fc-list")
            .arg(format!(":charset={chars}"))
            .arg("family")
            .output()
            .is_ok_and(|o| !o.stdout.is_empty())
    };
    if has("f05ad f05af") {
        ["\u{f05ad}", "\u{f05af}"]
    } else if has("1f5d6 1f5d9") {
        ["\u{1f5d9}", "\u{1f5d6}"]
    } else {
        ["✕", "☐"]
    }
}

/// Hauteur de la barre et taille d'un bouton (pixels logiques).
const HEIGHT: u32 = 34;
const BUTTON: u32 = 32;
/// Bouton « réduire » : son trait arrive au bas des deux autres.
const MINIMIZE: u32 = 38;

/// Marque Lua : boutons déjà posés depuis le dernier chargement ou relecture.
const MARK: &str = "__hyprdesk_bar_buttons";

/// Pose la barre et ses boutons (le plugin chargé), aux couleurs du thème.
pub fn add_buttons(fg: crate::config::Rgba, bg: crate::config::Rgba) -> Result<(), String> {
    if !loaded() {
        return Ok(());
    }
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "hyprdesk".into());
    let r = hyprpanel_core::ipc::eval(&buttons_lua(&exe, fg, bg, glyphs())).map_err(|e| e.to_string())?;
    if r.trim() == "ok" {
        Ok(())
    } else {
        Err(crate::tr!("Hyprbars, boutons : {r}", r = r.trim()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boutons_de_la_barre() {
        let fg = crate::config::Rgba(0xf6, 0xf7, 0xff, 255);
        let bg = crate::config::Rgba(0x22, 0x21, 0x2c, 255);
        let lua = buttons_lua("/opt/hyprdesk", fg, bg, ["✕", "☐"]);
        let lines: Vec<&str> = lua.lines().filter(|l| l.contains("add_button")).collect();
        assert_eq!(lines.len(), 3);
        // De droite à gauche : fermer, agrandir, réduire.
        assert!(lines[0].contains("hl.dsp.window.close()"));
        assert!(lines[1].contains(r#"mode = \"maximized\""#));
        assert!(lines[2].contains("'/opt/hyprdesk' window minimize"));
        // Symboles seuls, lisibles, aux couleurs du thème.
        assert!(lines.iter().all(|l| l.contains("rgba(00000000)") && l.contains("rgb(f6f7ff)")));
        assert!(lines[0].contains("size = 32") && lines[2].contains("size = 38") && lines[2].contains("⎽"));
        assert!(lua.contains("bar_height = 34") && lua.contains("bar_color = \"rgb(22212c)\""));
        // Règle « pas de barre » des fenêtres étiquetées, dans le bloc gardé.
        assert!(lua.contains(r#"match = { tag = "hyprdesk-nobar" }, ["hyprbars:no_bar"] = true"#));
        // Une seule pose par cycle (pas de retrait dans le plugin).
        assert!(lua.starts_with("if hl.plugin and hl.plugin.hyprbars and not __hyprdesk_bar_buttons then"));
        assert!(lua.trim_end().ends_with("__hyprdesk_bar_buttons = true\nend"));
    }

    #[test]
    fn etiquettes_sans_barre() {
        let lua = tags_lua(&["0xa".into()], &["0xb".into()], false);
        assert!(lua.contains(r#"tag = "-hyprdesk-nobar", window = "address:0xb""#));
        assert!(lua.contains(r#"tag = "+hyprdesk-nobar", window = "address:0xa""#));
        assert!(!lua.contains(r#""-hyprdesk-nobar", window = "address:0xa""#));
        // Règle recréée : retirée puis reposée, pour la réévaluation.
        let forced = tags_lua(&["0xa".into()], &[], true);
        assert!(forced.find("-hyprdesk-nobar").unwrap() < forced.find("+hyprdesk-nobar").unwrap());
        assert!(tags_lua(&[], &[], true).is_empty());
    }
}
