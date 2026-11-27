//! État de `pam_faillock` pour un compte, lu sans privilège.
//!
//! La pile `system-auth` d'Arch et de Fedora compte les échecs de mot de
//! passe (`pam_faillock`) : après `deny` échecs (3 par défaut) en
//! `fail_interval`, le compte est refusé partout (verrou, sudo, console)
//! pendant `unlock_time`, même avec le bon mot de passe, et PAM ne répond
//! que « Authentication failure ». Sur un écran de verrouillage, on tape
//! alors le bon mot de passe en boucle sans comprendre.
//!
//! Le fichier de décompte (`<dir>/<user>`) appartient à l'utilisateur et se
//! lit sans droit : l'écran prévient avant le dernier essai et affiche le
//! temps restant pendant le blocage. Lecture seule : rien ici ne lève le
//! blocage, qui reste l'affaire de `faillock --reset` (ou du délai).

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Réglages de `faillock.conf` utiles au décompte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub dir: PathBuf,
    /// Échecs qui bloquent le compte (0 : jamais).
    pub deny: u32,
    /// Fenêtre de comptage des échecs, en secondes.
    pub fail_interval: u64,
    /// Durée du blocage, en secondes (0 : jusqu'à `faillock --reset`).
    pub unlock_time: u64,
}

impl Default for Config {
    /// Valeurs par défaut de pam_faillock.
    fn default() -> Self {
        Config {
            dir: PathBuf::from("/var/run/faillock"),
            deny: 3,
            fail_interval: 900,
            unlock_time: 600,
        }
    }
}

/// Lit `faillock.conf` (format `clé = valeur`, `#` en commentaire) ; une
/// clé absente ou illisible garde sa valeur par défaut.
pub fn parse_config(text: &str) -> Config {
    let mut c = Config::default();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let v = v.trim();
        match k.trim() {
            "dir" if !v.is_empty() => c.dir = PathBuf::from(v),
            "deny" => c.deny = v.parse().unwrap_or(c.deny),
            "fail_interval" => c.fail_interval = v.parse().unwrap_or(c.fail_interval),
            "unlock_time" => {
                c.unlock_time = if v == "never" {
                    0
                } else {
                    v.parse().unwrap_or(c.unlock_time)
                }
            }
            _ => {}
        }
    }
    c
}

/// Taille d'un enregistrement `struct tally` : source[52], réservé u16,
/// état u16, heure u64.
const RECORD: usize = 64;
const TALLY_STATUS_VALID: u16 = 0x1;

/// Heures (secondes depuis l'époque) des échecs valides du fichier.
pub fn parse_tally(bytes: &[u8]) -> Vec<u64> {
    bytes
        .chunks_exact(RECORD)
        .filter_map(|r| {
            let status = u16::from_ne_bytes([r[54], r[55]]);
            let time = u64::from_ne_bytes(r[56..64].try_into().ok()?);
            (status & TALLY_STATUS_VALID != 0).then_some(time)
        })
        .collect()
}

/// État du compte à l'instant `now`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Pas de blocage ; `left` essais avant qu'il tombe (None : faillock
    /// ne bloque pas).
    Open { left: Option<u32> },
    /// Compte bloqué : `secs` secondes restantes (None : jusqu'à
    /// `faillock --reset`).
    Locked { secs: Option<u64> },
}

/// Même règle que pam_faillock. Blocage : tous les échecs valides comptent
/// (la fenêtre `fail_interval` n'invalide les anciens qu'à l'écriture d'un
/// nouvel échec), levé `unlock_time` après le dernier. Essais restants :
/// ceux de la fenêtre, comme les comptera le prochain échec.
pub fn state(c: &Config, failures: &[u64], now: u64) -> State {
    if c.deny == 0 {
        return State::Open { left: None };
    }
    if failures.len() as u32 >= c.deny {
        let last = failures.iter().copied().max().unwrap_or(now);
        if c.unlock_time == 0 {
            return State::Locked { secs: None };
        }
        let until = last + c.unlock_time;
        if now < until {
            return State::Locked {
                secs: Some(until - now),
            };
        }
        // Délai écoulé : le prochain échec efface les anciens.
        return State::Open { left: Some(c.deny) };
    }
    let recent = failures
        .iter()
        .filter(|t| now.saturating_sub(**t) <= c.fail_interval)
        .count() as u32;
    State::Open {
        left: Some(c.deny.saturating_sub(recent)),
    }
}

fn config_path() -> &'static Path {
    Path::new("/etc/security/faillock.conf")
}

/// État de faillock pour `user`, d'après le système. None : pas de
/// faillock lisible (module absent, dossier inaccessible) ; rien à dire.
pub fn current(user: &str) -> Option<State> {
    if user.is_empty() || user.contains('/') {
        return None;
    }
    let c = parse_config(&std::fs::read_to_string(config_path()).unwrap_or_default());
    if !c.dir.is_dir() {
        return None;
    }
    let bytes = match std::fs::read(c.dir.join(user)) {
        Ok(b) => b,
        // Pas encore d'échec : pas de fichier.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(_) => return None,
    };
    let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
    Some(state(&c, &parse_tally(&bytes), now))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(time: u64, status: u16) -> Vec<u8> {
        let mut r = vec![0u8; RECORD];
        r[..13].copy_from_slice(b"hyprdesk-lock");
        r[54..56].copy_from_slice(&status.to_ne_bytes());
        r[56..64].copy_from_slice(&time.to_ne_bytes());
        r
    }

    #[test]
    fn configuration() {
        let c = parse_config(
            "# dir = /ailleurs\ndeny = 5\n  unlock_time = never # jamais\nfail_interval=60\n",
        );
        assert_eq!(c.dir, PathBuf::from("/var/run/faillock"));
        assert_eq!((c.deny, c.unlock_time, c.fail_interval), (5, 0, 60));
        assert_eq!(parse_config(""), Config::default());
        assert_eq!(parse_config("deny = x").deny, 3);
    }

    #[test]
    fn decompte() {
        let mut b = record(100, 0x9);
        b.extend(record(110, 0x8)); // invalidé : ne compte pas
        b.extend(record(120, 0x1));
        b.extend([0u8; 10]); // reste tronqué ignoré
        assert_eq!(parse_tally(&b), vec![100, 120]);
    }

    #[test]
    fn regle_de_pam_faillock() {
        let c = Config::default();
        assert_eq!(state(&c, &[], 1000), State::Open { left: Some(3) });
        assert_eq!(state(&c, &[990, 995], 1000), State::Open { left: Some(1) });
        // Hors de la fenêtre de 900 s : oublié.
        assert_eq!(state(&c, &[10, 995], 1000), State::Open { left: Some(2) });
        // Trois échecs : bloqué 600 s après le dernier.
        assert_eq!(
            state(&c, &[990, 995, 1000], 1100),
            State::Locked { secs: Some(500) }
        );
        assert_eq!(
            state(&c, &[990, 995, 1000], 1600),
            State::Open { left: Some(3) }
        );
        let never = Config {
            unlock_time: 0,
            ..Config::default()
        };
        assert_eq!(
            state(&never, &[990, 995, 1000], 99_999),
            State::Locked { secs: None },
            "sans délai de levée : bloqué jusqu'à faillock --reset"
        );
        let off = Config {
            deny: 0,
            ..Config::default()
        };
        assert_eq!(state(&off, &[1, 2, 3], 4), State::Open { left: None });
    }
}
