//! Assistant IA : intégration de LYRA (`hyprpanel-chat`, dépôt
//! mindlog.hyprland_tools), l'assistante du bureau Linux qui fonctionne avec
//! un abonnement Zcode, Claude Code ou Codex.
//!
//! - Le fournisseur est le choix « Assistant IA » des applications par défaut
//!   (`apps.ai` : `claude`, `codex` ou `zcode`).
//! - LYRA s'ouvre en fenêtre, ou dans le terminal (`hyprpanel-chat cli`).
//! - Le contexte de hyprdesk (modes, paquets, applications, écrans,
//!   workspaces, widgets, fichiers, commandes) est écrit dans
//!   `~/.config/hyprpanel/context.d/hyprdesk.md` et donné par `hyprdesk context`.
//!
//! Les options de LYRA dont hyprdesk se sert sont détectées dans son
//! `--help` (`--provider`) : sans elles, LYRA garde son propre choix.

use std::path::PathBuf;

use crate::config::{Config, LauncherGroup, RunningApps};

/// Ouverture de l'assistant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AssistantMode {
    /// Fenêtre de LYRA.
    #[default]
    Window,
    /// `hyprpanel-chat cli` dans le terminal par défaut.
    Terminal,
}

impl AssistantMode {
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "window" => AssistantMode::Window,
            "terminal" => AssistantMode::Terminal,
            _ => return None,
        })
    }
    pub fn as_str(self) -> &'static str {
        match self {
            AssistantMode::Window => "window",
            AssistantMode::Terminal => "terminal",
        }
    }
}

/// Binaire de LYRA, s'il est installé.
pub fn lyra() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|p| std::env::split_paths(&p).collect::<Vec<_>>())
        .chain([home.join(".local/bin")])
        .map(|d| d.join("hyprpanel-chat"))
        .find(|p| p.is_file())
}

/// LYRA installée (vérifié une fois : sert à chaque rendu du lanceur).
pub fn lyra_installed() -> bool {
    lyra_probe().0
}

/// Ce qu'on sait de LYRA : installée, et son `--help` (options détectées).
/// Relu quand son binaire change (mise à jour de LYRA sans relancer le démon),
/// au plus une fois toutes les 5 s : le lanceur appelle ceci à chaque rendu.
fn lyra_probe() -> (bool, String) {
    use std::sync::Mutex;
    use std::time::{Duration, Instant, SystemTime};
    type Probe = (Instant, Option<(PathBuf, SystemTime)>, bool, String);
    static CACHE: Mutex<Option<Probe>> = Mutex::new(None);
    let mut c = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if let Some((at, _, found, help)) = c.as_ref()
        && at.elapsed() < Duration::from_secs(5)
    {
        return (*found, help.clone());
    }
    let bin = lyra();
    let stamp = bin.as_ref().and_then(|b| {
        let m = std::fs::metadata(b).ok()?.modified().ok()?;
        Some((b.clone(), m))
    });
    if let Some((_, old, found, help)) = c.as_mut()
        && *old == stamp
    {
        let r = (*found, help.clone());
        *c = Some((Instant::now(), stamp, r.0, r.1.clone()));
        return r;
    }
    let help = bin
        .as_ref()
        .and_then(|b| std::process::Command::new(b).arg("--help").output().ok())
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    let found = bin.is_some();
    *c = Some((Instant::now(), stamp, found, help.clone()));
    (found, help)
}

fn lyra_help() -> String {
    lyra_probe().1
}

/// LYRA accepte `--provider` (choix du fournisseur à l'ouverture).
pub fn supports_provider() -> bool {
    lyra_help().contains("--provider")
}

/// LYRA accepte `--language <fr|en>` (langue de la conversation).
pub fn supports_language() -> bool {
    lyra_help().contains("--language")
}

/// LYRA lit les contextes de `~/.config/hyprpanel/context.d/`.
pub fn supports_context_dir() -> bool {
    lyra_help().contains("context.d")
}

/// Service d'intégration de LYRA (`hyprpanel.service`) actif.
pub fn service_active() -> bool {
    std::process::Command::new("systemctl")
        .args(["--user", "is-active", "--quiet", "hyprpanel.service"])
        .status()
        .is_ok_and(|s| s.success())
}

/// Fournisseur LYRA d'après le choix « Assistant IA » (`apps.ai`).
pub fn provider(ai: Option<&str>) -> Option<&'static str> {
    match ai?.to_lowercase().as_str() {
        "claude" | "claude-code" | "claude_code" => Some("claude_code"),
        "codex" => Some("codex"),
        "zcode" => Some("zcode"),
        _ => None,
    }
}

/// LYRA propose le mode `serve` (conversation dessinée par hyprdesk).
pub fn supports_serve() -> bool {
    lyra_help().lines().any(|l| {
        let l = l.trim_start();
        l.starts_with("hyprpanel-chat serve") || l.starts_with("serve ")
    })
}

/// Moteur du tiroir de l'assistant (argv) : `$HYPRDESK_CHAT_ENGINE` (essais,
/// ex. `hyprdesk chat-mock`), sinon `hyprpanel-chat serve` si LYRA le propose
/// et que l'assistant s'ouvre « dans sa fenêtre ». `None` : fenêtre ou terminal.
pub fn serve_command(cfg: &Config) -> Option<Vec<String>> {
    if !cfg.plugin_on("assistant") {
        return None;
    }
    if let Ok(e) = std::env::var("HYPRDESK_CHAT_ENGINE") {
        let argv: Vec<String> = e.split_whitespace().map(String::from).collect();
        if !argv.is_empty() {
            return Some(argv);
        }
    }
    if cfg.general.assistant_mode != AssistantMode::Window || !supports_serve() {
        return None;
    }
    let mut argv = vec![lyra()?.display().to_string(), "serve".into()];
    if let Some(p) = provider(cfg.general.apps.get("ai").map(String::as_str)) {
        argv.extend(["--provider".into(), p.into()]);
    }
    Some(argv)
}

/// Commande qui ouvre LYRA, si l'assistant est actif et LYRA installée.
pub fn command(cfg: &Config) -> Option<String> {
    if !cfg.plugin_on("assistant") {
        return None;
    }
    let bin = crate::launch::sh_quote(&lyra()?.display().to_string());
    let mut prov = provider(cfg.general.apps.get("ai").map(String::as_str))
        .filter(|_| supports_provider())
        .map(|p| format!(" --provider {p}"))
        .unwrap_or_default();
    // LYRA parle la langue de l'utilisateur (celle de hyprdesk), si elle sait la recevoir.
    if supports_language() {
        prov += &format!(" --language {}", crate::i18n::code());
    }
    Some(match cfg.general.assistant_mode {
        AssistantMode::Window => format!("{bin}{prov}"),
        AssistantMode::Terminal => {
            crate::launch::in_terminal(&cfg.general.terminal, &format!("{bin} cli{prov}"))
        }
    })
}

/// Contexte de l'environnement hyprdesk, en Markdown, pour l'assistant.
pub fn context(cfg: &Config, config_path: &std::path::Path) -> String {
    let g = &cfg.general;
    let on_off = |b: bool| {
        if b {
            "actif"
        } else {
            crate::i18n::tr("coupé")
        }
    };
    let mut o = String::new();
    o += crate::i18n::tr("# hyprdesk — environnement du bureau\n\n");
    o += &crate::tr!(
        "Langue de l'utilisateur : {lang} (celle de l'interface de hyprdesk).\n\n",
        lang = match crate::i18n::current() {
            crate::i18n::Lang::Fr => "français",
            crate::i18n::Lang::En => "English",
        }
    );
    o += &crate::tr!(
        "hyprdesk {p0} gère le bureau de Hyprland : fond d'écran, icônes, widgets, lanceur (dock), \
         barre, écrans (monitors.lua), workspaces, applications par défaut.\n\n",
        p0 = env!("CARGO_PKG_VERSION")
    );
    o += crate::i18n::tr("## Modes du bureau\n\n");
    o += &crate::tr!(
        "- `no-desktop` : fond seul ; `{p0}`\n- `launcher-only` : fenêtres en place, dock accessible ; `{p1}`\n\
         - `full-desktop` : fenêtres mises de côté (workspace `special:hyprdesk`) ; `{p2}`\n\
         - commandes : `hyprdesk show | launcher-only | hide | cycle`\n\n",
        p0 = g.keys.hide,
        p1 = g.keys.launcher,
        p2 = g.keys.toggle
    );
    o += &crate::tr!(
        "Bureau affiché sur : {p0} ; écran principal : {p1} ; mode : {p2}.\n\n",
        p0 = g.show_on.as_str(),
        p1 = g
            .primary
            .as_deref()
            .unwrap_or(crate::i18n::tr("non défini")),
        p2 = if g.mode == crate::config::Mode::Glass {
            "glass"
        } else {
            "opaque"
        }
    );
    o += "## Paquets\n\n";
    for p in crate::ui::plugins::BUILTIN {
        o += &format!(
            "- {} (`{}`) : {} — {}\n",
            p.label,
            p.id,
            on_off(cfg.plugin_on(p.id)),
            p.summary
        );
    }
    o += crate::i18n::tr("\n## Applications par défaut\n\n");
    for r in crate::apps::ROLES {
        let v = g
            .apps
            .get(r.id)
            .map(String::as_str)
            .unwrap_or(crate::i18n::tr("celle du système"));
        o += &format!("- {} (`{}`) : {}\n", r.label, r.id, v);
    }
    o += &crate::tr!(
        "\nLanceur : fenêtres `{p0}`, regroupement `{p1}`.\n",
        p0 = match g.running_apps {
            RunningApps::Off => "off",
            RunningApps::Workspace => "workspace",
            RunningApps::All => "all",
        },
        p1 = match g.launcher_group {
            LauncherGroup::None => "none",
            LauncherGroup::App => "app",
            LauncherGroup::Workspace => "workspace",
        }
    );
    o += "\n## Workspaces\n\n";
    o += &format!(
        "- permanents : {}\n",
        g.workspaces.map_or(
            crate::i18n::tr("non (Hyprland les crée à la demande)").into(),
            |n| crate::tr!("1 à {n}", n = n)
        )
    );
    for a in &cfg.assigns {
        let app = match &a.app {
            crate::config::AppMatch::App(id) => id.clone(),
            crate::config::AppMatch::Class(re) => format!("classe {re}"),
        };
        o += &format!(
            "- {app} → workspace {}{}\n",
            a.workspace,
            if a.silent {
                crate::i18n::tr(" (sans y aller)")
            } else {
                ""
            }
        );
    }
    if let Some(b) = &cfg.bar {
        o += &crate::tr!(
            "\n## Barre\n\n{pos}, hauteur {h}, écrans : {mon}.\n",
            pos = format!("{:?}", b.position),
            h = b.height,
            mon = b
                .monitors
                .as_ref()
                .map_or(crate::i18n::tr("tous").to_string(), |l| l.join(", "))
        );
    }
    let active: Vec<&str> = cfg
        .widgets
        .iter()
        .filter(|w| !w.in_bar)
        .map(|w| w.id.as_str())
        .collect();
    o += &crate::tr!(
        "\n## Widgets\n\nActifs sur le bureau : {p0}. Disponibles : {p1}.\n",
        p0 = if active.is_empty() {
            crate::i18n::tr("aucun").into()
        } else {
            active.join(", ")
        },
        p1 = cfg
            .catalog
            .iter()
            .map(|t| t.id.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let dir = config_path.parent().unwrap_or(std::path::Path::new("."));
    o += &crate::tr!(
        "\n## Fichiers\n\n- `{p0}` : configuration écrite à la main (garde le dernier mot)\n\
         - `{p1}` : réglages générés par l'écran Réglages et `hyprdesk config` (ne pas éditer)\n\
         - `{p2}` : écrans, généré par hyprdesk (ne pas éditer)\n\
         - `{p3}` : plugins Lua de widgets\n",
        p0 = config_path.display(),
        p1 = crate::config::lua::settings_path(config_path).display(),
        p2 = crate::screens::monitors_path(config_path).display(),
        p3 = dir.join("hyprdesk/widgets").display()
    );
    o += crate::i18n::tr(
        "\n## Agir sur hyprdesk\n\n\
          - `hyprdesk config` : toutes les clés réglables, par paquet ; `hyprdesk config set <clé> <valeur>`\n\
          - `hyprdesk settings <paquet>` : ouvre l'écran Réglages\n\
          - `hyprdesk open <rôle>` : lance l'application par défaut d'un rôle\n\
          - `hyprdesk status`, `hyprdesk check` : état du démon, vérification de la config\n",
    );
    o
}

/// Dossier des contextes partagés avec LYRA (s'il est installé).
pub fn context_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".config")
        });
    let dir = base.join("hyprpanel");
    dir.is_dir().then(|| dir.join("context.d"))
}

/// Écrit le contexte de hyprdesk pour LYRA (sans effet si LYRA est absente
/// ou l'assistant coupé).
pub fn write_context(cfg: &Config, config_path: &std::path::Path) {
    let Some(dir) = context_dir().filter(|_| cfg.plugin_on("assistant")) else {
        return;
    };
    let text = context(cfg, config_path);
    let path = dir.join("hyprdesk.md");
    if std::fs::read_to_string(&path).ok().as_deref() == Some(text.as_str()) {
        return;
    }
    if let Err(e) = std::fs::create_dir_all(&dir)
        .and_then(|()| crate::storage::write(&path, text.as_bytes(), crate::storage::revision()))
    {
        tracing::warn!("{}", crate::tr!("contexte de l'assistant : {e}", e = e));
    }
}

/// Identifiant Wayland de la fenêtre de LYRA (`slint::set_xdg_app_id`).
pub const LYRA_APP_ID: &str = "lyra";

/// Lua de la règle qui présente LYRA comme le tiroir des réglages : flottante,
/// épinglée, contre le bord droit de l'écran principal, sur toute la hauteur
/// utile ; `None` retire la règle. Rectangle : (x, y, largeur, hauteur).
pub fn drawer_rule_lua(rect: Option<(i32, i32, i32, i32)>) -> String {
    let mut lua = String::from(
        "if __hyprdesk_lyra then pcall(function() __hyprdesk_lyra:set_enabled(false) end) end\n\
         __hyprdesk_lyra = nil\n",
    );
    if let Some((x, y, w, h)) = rect {
        lua += &format!(
            "__hyprdesk_lyra = hl.window_rule({{ name = \"hyprdesk-lyra\", match = {{ class = \"^({LYRA_APP_ID})$\" }}, \
             float = true, pin = true, size = \"{w} {h}\", move = \"{x} {y}\", animation = \"slide right\" }})\n"
        );
    }
    lua
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lyra_en_tiroir() {
        let on = drawer_rule_lua(Some((1792, 54, 756, 1362)));
        assert!(on.contains(r#"match = { class = "^(lyra)$" }"#));
        assert!(on.contains(r#"size = "756 1362", move = "1792 54""#));
        assert!(on.contains("pin = true"));
        let off = drawer_rule_lua(None);
        assert!(off.contains("set_enabled(false)") && !off.contains("hl.window_rule"));
    }

    #[test]
    fn fournisseur_et_contexte() {
        assert_eq!(provider(Some("claude")), Some("claude_code"));
        assert_eq!(provider(Some("zcode")), Some("zcode"));
        assert_eq!(provider(Some("aider")), None);
        let mut cfg = Config::default();
        cfg.general.apps.insert("ai".into(), "claude".into());
        cfg.general.workspaces = Some(5);
        let md = context(&cfg, std::path::Path::new("/cfg/hyprdesk.lua"));
        assert!(md.contains("launcher-only") && md.contains("SUPER + ALT + D"));
        assert!(md.contains("Assistant IA (`ai`) : claude"));
        assert!(md.contains("1 à 5"));
        assert!(md.contains("hyprdesk config set"));
        // Assistant coupé : pas de commande LYRA (repli sur l'outil direct).
        cfg.general.disabled_plugins.insert("assistant".into());
        assert!(command(&cfg).is_none());
    }
}
