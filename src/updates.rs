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

/// Met à jour chaque source, l'une après l'autre ; s'arrête au premier échec.
pub async fn upgrade(sources: &[Source]) -> Vec<Step> {
    let mut steps = Vec::new();
    for s in sources.iter().filter(|s| !s.upgrade.is_empty()) {
        let (prog, args) = s.upgrade.split_first().expect("commande non vide");
        let mut cmd = tokio::process::Command::new(prog);
        cmd.args(args).env("LC_ALL", "C");
        let step = match crate::processes::output(&mut cmd, Duration::from_secs(3600)).await {
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
        "La mise à jour du système a échoué ({os}).\n\nCommande : {cmd}\nCode de sortie : {code}\n\nFin du journal :\n{tail}\n\nQue s'est-il passé, et comment le corriger ?",
        os = os,
        cmd = failed.command,
        code = failed.code.map_or("?".to_string(), |c| c.to_string()),
        tail = failed.tail
    )
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
}
