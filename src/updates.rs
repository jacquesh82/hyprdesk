//! Mises à jour en attente (MLG-31) : sources de paquets détectées sur la
//! machine, nombre de paquets à mettre à jour, mise à jour non interactive.
//!
//! - Arch : `checkupdates` (pacman-contrib, sans droits, base temporaire) ;
//!   AUR par `yay` ou `paru` (`-Qua`), qui fait alors toute la mise à jour,
//!   dépôts compris, en passant par `pkexec` (agent polkit, pas de terminal).
//! - Debian / Ubuntu : `apt list --upgradable` ; Fedora : `dnf check-update` ;
//!   openSUSE : `zypper list-updates` ; Flatpak : `flatpak remote-ls --updates`.
//!
//! Les droits d'administration passent par `pkexec` : la mise à jour se fait
//! en un clic, le mot de passe est demandé par l'agent polkit de la session.

use std::time::Duration;

/// Une source de paquets.
#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    pub id: &'static str,
    pub label: &'static str,
    /// Commande de comptage (sans droits).
    pub check: Vec<String>,
    /// Commande de mise à jour non interactive.
    pub upgrade: Vec<String>,
}

fn argv(s: &[&str]) -> Vec<String> {
    s.iter().map(|a| a.to_string()).collect()
}

/// Sources disponibles, d'après les programmes présents (`has`).
pub fn detect(has: impl Fn(&str) -> bool) -> Vec<Source> {
    let mut out = Vec::new();
    if has("pacman") {
        let helper = ["yay", "paru"].into_iter().find(|h| has(h));
        if has("checkupdates") {
            out.push(Source {
                id: "pacman",
                label: "Dépôts Arch",
                check: argv(&["checkupdates"]),
                upgrade: match helper {
                    // L'assistant AUR met aussi à jour les dépôts : une seule étape.
                    Some(_) => Vec::new(),
                    None => argv(&["pkexec", "pacman", "-Syu", "--noconfirm"]),
                },
            });
        }
        if let Some(h) = helper {
            out.push(Source {
                id: "aur",
                label: "AUR",
                check: argv(&[h, "-Qua"]),
                upgrade: argv(&[
                    h,
                    "-Syu",
                    "--noconfirm",
                    "--sudo",
                    "pkexec",
                    "--answerclean",
                    "None",
                    "--answerdiff",
                    "None",
                ]),
            });
        }
    } else if has("apt-get") {
        out.push(Source {
            id: "apt",
            label: "APT",
            check: argv(&["apt", "list", "--upgradable"]),
            upgrade: argv(&[
                "pkexec",
                "sh",
                "-c",
                "apt-get update && DEBIAN_FRONTEND=noninteractive apt-get -y upgrade",
            ]),
        });
    } else if has("dnf") {
        out.push(Source {
            id: "dnf",
            label: "DNF",
            check: argv(&["dnf", "-q", "check-update"]),
            upgrade: argv(&["pkexec", "dnf", "-y", "upgrade", "--refresh"]),
        });
    } else if has("zypper") {
        out.push(Source {
            id: "zypper",
            label: "Zypper",
            check: argv(&["zypper", "-q", "--non-interactive", "list-updates"]),
            upgrade: argv(&["pkexec", "zypper", "--non-interactive", "update"]),
        });
    }
    if has("flatpak") {
        out.push(Source {
            id: "flatpak",
            label: "Flatpak",
            check: argv(&["flatpak", "remote-ls", "--updates", "--columns=application"]),
            upgrade: argv(&["flatpak", "update", "-y", "--noninteractive"]),
        });
    }
    out
}

/// Sources de cette machine.
pub fn sources() -> Vec<Source> {
    detect(crate::launch::executable)
}

/// Commandes remplacées (Réglages → Plugins → Mises à jour → Configurer) :
/// source → (relevé, mise à jour), en shell ; vide : celle de hyprdesk.
pub fn with_overrides(
    mut list: Vec<Source>,
    over: &std::collections::BTreeMap<String, (String, String)>,
) -> Vec<Source> {
    let sh = |c: &str| vec!["sh".to_string(), "-c".to_string(), c.to_string()];
    for s in list.iter_mut() {
        if let Some((check, upgrade)) = over.get(s.id) {
            if !check.trim().is_empty() {
                s.check = sh(check.trim());
            }
            if !upgrade.trim().is_empty() {
                s.upgrade = sh(upgrade.trim());
            }
        }
    }
    list
}

/// Sources de cette machine, avec les commandes réglées dans settings.lua.
pub fn configured_sources(config_path: &std::path::Path) -> Vec<Source> {
    let over = crate::config::lua::load_settings_only(config_path)
        .map(|c| c.general.update_commands)
        .unwrap_or_default();
    with_overrides(sources(), &over)
}

/// Commande affichée (argv rejoint, ou le texte shell d'une commande remplacée).
pub fn shown(argv: &[String]) -> String {
    match argv {
        [sh, c, cmd] if sh == "sh" && c == "-c" => cmd.clone(),
        _ => argv.join(" "),
    }
}

/// Paquets à mettre à jour, d'après la sortie de la commande de comptage.
pub fn parse(source: &str, out: &str) -> Vec<String> {
    let lines = out.lines().map(str::trim).filter(|l| !l.is_empty());
    match source {
        // `nom ancienne -> nouvelle`
        "pacman" | "aur" => lines
            .filter(|l| l.contains("->"))
            .filter_map(|l| l.split_whitespace().next().map(String::from))
            .collect(),
        // `nom/dépôt version arch [upgradable from: …]`, après « Listing… ».
        "apt" => lines
            .filter(|l| l.contains('/') && l.contains("upgradable"))
            .filter_map(|l| l.split('/').next().map(String::from))
            .collect(),
        // `nom.arch version dépôt` ; « Obsoleting Packages » clôt la liste.
        "dnf" => lines
            .take_while(|l| !l.starts_with("Obsoleting"))
            .filter(|l| l.split_whitespace().count() == 3)
            .filter_map(|l| l.split_whitespace().next())
            .map(|n| n.rsplit_once('.').map_or(n, |(name, _)| name).to_string())
            .collect(),
        // Tableau : `v | dépôt | nom | actuelle | disponible | arch`.
        "zypper" => lines
            .filter(|l| l.starts_with("v "))
            .filter_map(|l| l.split('|').nth(2).map(|n| n.trim().to_string()))
            .collect(),
        "flatpak" => lines
            .filter(|l| l.contains('.') && !l.contains(' '))
            .map(String::from)
            .collect(),
        _ => Vec::new(),
    }
}

/// Paquets en attente d'une source.
#[derive(Debug, Clone, PartialEq)]
pub struct Pending {
    pub source: &'static str,
    pub label: &'static str,
    pub packages: Vec<String>,
}

/// Relevé de toutes les sources (les échecs de comptage sont ignorés : une
/// source injoignable ne doit pas cacher les autres).
pub async fn check(sources: &[Source]) -> Vec<Pending> {
    let mut out = Vec::new();
    for s in sources {
        let Some((prog, args)) = s.check.split_first() else {
            continue;
        };
        let mut cmd = tokio::process::Command::new(prog);
        cmd.args(args).env("LC_ALL", "C");
        let Ok(o) = crate::processes::output(&mut cmd, Duration::from_secs(180)).await else {
            continue;
        };
        out.push(Pending {
            source: s.id,
            label: s.label,
            packages: parse(s.id, &String::from_utf8_lossy(&o.stdout)),
        });
    }
    out
}

/// Total des paquets en attente.
pub fn total(p: &[Pending]) -> usize {
    p.iter().map(|p| p.packages.len()).sum()
}

/// Une étape de mise à jour.
#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    pub source: &'static str,
    pub command: String,
    pub ok: bool,
    pub code: Option<i32>,
    /// Fin du journal (sorties standard et d'erreur).
    pub tail: String,
}

/// Lignes gardées du journal d'une étape.
pub const TAIL_LINES: usize = 60;

pub fn tail(out: &str, n: usize) -> String {
    let lines: Vec<&str> = out.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

/// Journal complet des mises à jour (tournant : 1 Mio, 3 anciens gardés).
pub fn log_path() -> std::path::PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join(".local/state")
        })
        .join("hyprdesk")
        .join("updates.log")
}

fn open_log() -> crate::processes::RotatingLog {
    crate::processes::RotatingLog::open(&log_path(), 1024 * 1024, 3)
}

/// Met à jour chaque source, l'une après l'autre ; s'arrête au premier échec.
pub async fn upgrade(sources: &[Source]) -> Vec<Step> {
    let mut steps = Vec::new();
    for s in sources.iter().filter(|s| !s.upgrade.is_empty()) {
        let (prog, args) = s.upgrade.split_first().expect("commande non vide");
        let mut cmd = tokio::process::Command::new(prog);
        cmd.args(args).env("LC_ALL", "C");
        let mut log = open_log();
        log.write(
            format!(
                "\n== {} — {}\n",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                s.upgrade.join(" ")
            )
            .as_bytes(),
        );
        let step =
            match crate::processes::output_logged(&mut cmd, Duration::from_secs(3600), log).await {
                Ok(o) => {
                    let mut log = String::from_utf8_lossy(&o.stdout).into_owned();
                    log.push_str(&String::from_utf8_lossy(&o.stderr));
                    Step {
                        source: s.id,
                        command: s.upgrade.join(" "),
                        ok: o.status.success(),
                        code: o.status.code(),
                        tail: tail(&log, TAIL_LINES),
                    }
                }
                Err(e) => Step {
                    source: s.id,
                    command: s.upgrade.join(" "),
                    ok: false,
                    code: None,
                    tail: e.to_string(),
                },
            };
        let failed = !step.ok;
        steps.push(step);
        if failed {
            break;
        }
    }
    steps
}

/// Description d'un échec, à copier ou à soumettre à l'assistant.
pub fn report(failed: &Step) -> String {
    let os = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| {
            s.lines().find_map(|l| {
                l.strip_prefix("PRETTY_NAME=")
                    .map(|v| v.trim_matches('"').to_string())
            })
        })
        .unwrap_or_else(|| "Linux".into());
    crate::tr!(
        "La mise à jour du système a échoué ({os}).\n\nCommande : {cmd}\nCode de sortie : {code}\nJournal complet : {log}\n\nFin du journal :\n{tail}\n\nQue s'est-il passé, et comment le corriger ?",
        os = os,
        log = log_path().display(),
        cmd = failed.command,
        code = failed.code.map_or("?".to_string(), |c| c.to_string()),
        tail = failed.tail
    )
}

/// État partagé avec le plugin de barre (`majs.lua`), en lignes simples
/// faciles à lire en Lua :
/// `state=idle|checking|running|failed`, `checked=<unix>`, `total=<n>`,
/// puis une ligne `source=<libellé>|<n>|<paquets séparés par des espaces>`
/// par source.
pub fn state_path() -> std::path::PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("hyprdesk-updates.txt")
}

/// Rapport du dernier échec (`hyprdesk updates --report` le renvoie au démon).
pub fn report_path() -> std::path::PathBuf {
    state_path().with_file_name("hyprdesk-updates-report.txt")
}

pub fn render_state(state: &str, checked: u64, pending: &[Pending]) -> String {
    let mut out = format!(
        "state={state}\nchecked={checked}\ntotal={}\n",
        total(pending)
    );
    for p in pending {
        out.push_str(&format!(
            "source={}|{}|{}\n",
            crate::i18n::tr(p.label),
            p.packages.len(),
            p.packages.join(" ")
        ));
    }
    out
}

/// État courant (`state=…`), s'il existe.
pub fn current_state() -> Option<String> {
    let s = std::fs::read_to_string(state_path()).ok()?;
    effective_state(&s, |pid| {
        std::path::Path::new(&format!("/proc/{pid}")).exists()
    })
}

/// État lu, une tâche « checking » ou « running » dont le processus
/// (`pid=`) n'existe plus comptant pour « idle » : une mise à jour
/// interrompue ne bloque pas les suivantes.
pub fn effective_state(s: &str, alive: impl Fn(u32) -> bool) -> Option<String> {
    let state = s
        .lines()
        .find_map(|l| l.strip_prefix("state=").map(String::from))?;
    let pid = s.lines().find_map(|l| {
        l.strip_prefix("pid=")
            .and_then(|p| p.trim().parse::<u32>().ok())
    });
    match (state.as_str(), pid) {
        ("checking" | "running", Some(p)) if !alive(p) => Some("idle".into()),
        _ => Some(state),
    }
}

/// Remplace la ligne `state=` en gardant le reste (comptes affichés).
pub fn set_state(state: &str) {
    let path = state_path();
    let old = std::fs::read_to_string(&path).unwrap_or_default();
    let mut lines: Vec<String> = old
        .lines()
        .filter(|l| !l.starts_with("state=") && !l.starts_with("pid="))
        .map(String::from)
        .collect();
    lines.insert(0, format!("state={state}"));
    // Tâche en cours : son processus, pour reconnaître une tâche interrompue.
    if matches!(state, "checking" | "running") {
        lines.insert(1, format!("pid={}", std::process::id()));
    }
    let _ = std::fs::write(&path, lines.join("\n") + "\n");
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Relevé, écrit pour le plugin ; un relevé ou une mise à jour en cours
/// n'est pas doublé.
pub async fn refresh(sources: &[Source]) -> Vec<Pending> {
    if matches!(
        current_state().as_deref(),
        Some("checking") | Some("running")
    ) {
        return Vec::new();
    }
    set_state("checking");
    let pending = check(sources).await;
    let _ = std::fs::write(state_path(), render_state("idle", now(), &pending));
    pending
}

/// Mise à jour complète pour le plugin : état « running », puis relevé ;
/// en cas d'échec, état « failed », rapport gardé et confié au démon
/// (assistant LYRA, ou fenêtre de détail copiable).
pub async fn upgrade_and_report(sources: &[Source]) -> Option<String> {
    set_state("running");
    let steps = upgrade(sources).await;
    match steps.iter().find(|s| !s.ok) {
        None => {
            let pending = check(sources).await;
            let _ = std::fs::write(state_path(), render_state("idle", now(), &pending));
            crate::hypr::notify(1, 5000, crate::i18n::tr("hyprdesk : système à jour"));
            None
        }
        Some(f) => {
            let r = report(f);
            let _ = std::fs::write(report_path(), &r);
            set_state("failed");
            send_problem(&r, false);
            Some(r)
        }
    }
}

/// Confie un problème au démon (`show` : demande explicite, la conversation
/// s'affiche) ; sans démon, une notification Hyprland.
pub fn send_problem(text: &str, show: bool) {
    let cmd = if show {
        crate::control::Command::ProblemShow(text.to_string())
    } else {
        crate::control::Command::Problem(text.to_string())
    };
    // Le démon répond « ok » ; toute autre réponse (démon absent, commande
    // refusée) ne doit pas passer inaperçue.
    match crate::control::send(cmd) {
        Ok(r) if r == "ok" => {}
        Ok(r) => {
            tracing::warn!(
                "{}",
                crate::tr!("problème non transmis au démon : {r}", r = r)
            );
            crate::hypr::notify(
                3,
                8000,
                &crate::tr!(
                    "hyprdesk : la mise à jour a échoué — détail dans {p}",
                    p = report_path().display()
                ),
            );
        }
        Err(_) => crate::hypr::notify(
            3,
            8000,
            crate::i18n::tr("hyprdesk : la mise à jour a échoué — hyprdesk updates --report"),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sources_selon_la_machine() {
        let arch = detect(|p| ["pacman", "checkupdates", "yay", "flatpak"].contains(&p));
        let ids: Vec<&str> = arch.iter().map(|s| s.id).collect();
        assert_eq!(ids, ["pacman", "aur", "flatpak"]);
        // L'assistant AUR fait toute la mise à jour (dépôts compris), par pkexec.
        assert!(arch[0].upgrade.is_empty());
        assert!(
            arch[1]
                .upgrade
                .join(" ")
                .contains("yay -Syu --noconfirm --sudo pkexec")
        );
        let pacman_seul = detect(|p| ["pacman", "checkupdates"].contains(&p));
        assert_eq!(
            pacman_seul[0].upgrade.join(" "),
            "pkexec pacman -Syu --noconfirm"
        );
        let debian = detect(|p| p == "apt-get");
        assert_eq!(debian[0].id, "apt");
        assert!(debian[0].upgrade[0] == "pkexec");
        assert!(detect(|_| false).is_empty());
    }

    #[test]
    fn analyse_des_sorties() {
        assert_eq!(
            parse(
                "pacman",
                "linux 6.17.1-1 -> 6.17.2-1\nfirefox 144.0-1 -> 144.0.1-1\n"
            ),
            ["linux", "firefox"]
        );
        assert_eq!(
            parse(
                "apt",
                "Listing...\nfirefox/stable 144.0 amd64 [upgradable from: 143.0]\nlibc6/stable 2.41 amd64 [upgradable from: 2.40]\n"
            ),
            ["firefox", "libc6"]
        );
        assert_eq!(
            parse(
                "dnf",
                "\nkernel.x86_64  6.17.2-200.fc43  updates\nvim-minimal.x86_64 2:9.1-1.fc43 updates\nObsoleting Packages\ngrub2.x86_64 1:2.12 updates\n"
            ),
            ["kernel", "vim-minimal"]
        );
        assert_eq!(
            parse(
                "zypper",
                "S | Repository | Name | Current Version | Available Version | Arch\n--+---\nv | repo-oss | bash | 5.2-1 | 5.2-2 | x86_64\n"
            ),
            ["bash"]
        );
        assert_eq!(
            parse("flatpak", "org.mozilla.firefox\norg.gnome.Platform\n"),
            ["org.mozilla.firefox", "org.gnome.Platform"]
        );
        assert!(parse("pacman", "").is_empty());
    }

    #[test]
    fn etat_pour_le_plugin() {
        let p = vec![
            Pending {
                source: "pacman",
                label: "Dépôts Arch",
                packages: vec!["linux".into(), "firefox".into()],
            },
            Pending {
                source: "flatpak",
                label: "Flatpak",
                packages: vec![],
            },
        ];
        let s = render_state("idle", 42, &p);
        assert!(s.starts_with("state=idle\nchecked=42\ntotal=2\n"), "{s}");
        assert!(s.contains("|2|linux firefox\n") && s.contains("Flatpak|0|\n"));
    }

    #[test]
    fn journal_et_rapport() {
        let log: String = (1..=100).map(|i| format!("ligne {i}\n")).collect();
        let t = tail(&log, 3);
        assert_eq!(t, "ligne 98\nligne 99\nligne 100");
        let r = report(&Step {
            source: "aur",
            command: "yay -Syu".into(),
            ok: false,
            code: Some(1),
            tail: "error: failed to commit transaction (conflicting files)".into(),
        });
        assert!(r.contains("yay -Syu") && r.contains("conflicting files") && r.contains(": 1"));
    }

    #[test]
    fn commandes_remplacees() {
        let has = |p: &str| matches!(p, "pacman" | "checkupdates" | "yay" | "flatpak");
        let mut over = std::collections::BTreeMap::new();
        over.insert(
            "aur".to_string(),
            (String::new(), "yay -Syu --noconfirm".to_string()),
        );
        let list = with_overrides(detect(has), &over);
        let aur = list.iter().find(|s| s.id == "aur").unwrap();
        assert_eq!(aur.check, vec!["yay".to_string(), "-Qua".to_string()]);
        assert_eq!(shown(&aur.upgrade), "yay -Syu --noconfirm");
        let flat = list.iter().find(|s| s.id == "flatpak").unwrap();
        assert_eq!(
            shown(&flat.check),
            "flatpak remote-ls --updates --columns=application"
        );
    }

    #[test]
    fn tache_interrompue() {
        let s = "state=running\npid=4242\nchecked=1\ntotal=3\n";
        assert_eq!(effective_state(s, |_| true).as_deref(), Some("running"));
        assert_eq!(effective_state(s, |_| false).as_deref(), Some("idle"));
        assert_eq!(
            effective_state("state=failed\n", |_| false).as_deref(),
            Some("failed")
        );
        // Ancien fichier sans pid : tel quel.
        assert_eq!(
            effective_state("state=checking\n", |_| false).as_deref(),
            Some("checking")
        );
    }
}
