//! Recherche floue du lanceur (MLG-7) : les lettres tapées doivent
//! apparaître dans l'ordre (« ffx » trouve Firefox), sans tenir compte de la
//! casse ni des accents ; un début de mot, des lettres consécutives et un
//! nom court passent devant.

/// Minuscule sans accent (é → e, ç → c…).
pub fn fold(c: char) -> char {
    let c = c.to_lowercase().next().unwrap_or(c);
    match c {
        'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
        'ç' => 'c',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ñ' => 'n',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ý' | 'ÿ' => 'y',
        c => c,
    }
}

/// Score de `query` dans `text` (plus haut : meilleur), `None` si les
/// lettres n'y sont pas dans l'ordre. Les espaces de la requête sont ignorés.
pub fn score(query: &str, text: &str) -> Option<i32> {
    let q: Vec<char> = query
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(fold)
        .collect();
    if q.is_empty() {
        return Some(0);
    }
    let t: Vec<char> = text.chars().map(fold).collect();
    let mut s = 0;
    let mut qi = 0;
    let mut prev: Option<usize> = None;
    for (i, &c) in t.iter().enumerate() {
        if qi == q.len() {
            break;
        }
        if c != q[qi] {
            continue;
        }
        let word_start = i == 0 || !t[i - 1].is_alphanumeric();
        s += 10;
        if word_start {
            s += 15;
        }
        if i == 0 {
            s += 10;
        }
        match prev {
            Some(p) if p + 1 == i => s += 12,
            Some(p) => s -= ((i - p - 1) as i32).min(8),
            None => s -= (i as i32).min(10),
        }
        prev = Some(i);
        qi += 1;
    }
    if qi < q.len() {
        return None;
    }
    // À score égal, le nom le plus court (le plus proche de la requête).
    Some(s - (t.len() as i32 / 4))
}

/// Indices de `names` correspondant à `query`, du meilleur au moins bon
/// (ordre alphabétique sans requête), au plus `limit`.
#[cfg_attr(not(test), allow(dead_code))]
pub fn rank(query: &str, names: &[&str], limit: usize) -> Vec<usize> {
    rank_with(query, names, &[], limit)
}

/// Comme `rank`, avec un bonus d'usage par nom (`bonus[i]`, 0 si absent) :
/// sans requête, les plus lancées d'abord, puis l'ordre alphabétique.
pub fn rank_with(query: &str, names: &[&str], bonus: &[i32], limit: usize) -> Vec<usize> {
    let b = |i: usize| bonus.get(i).copied().unwrap_or(0);
    let mut hits: Vec<(i32, usize)> = names
        .iter()
        .enumerate()
        .filter_map(|(i, n)| score(query, n).map(|s| (s + b(i), i)))
        .collect();
    if query.trim().is_empty() {
        hits.sort_by(|a, x| {
            let fa: String = names[a.1].chars().map(fold).collect();
            let fb: String = names[x.1].chars().map(fold).collect();
            b(x.1).cmp(&b(a.1)).then(fa.cmp(&fb))
        });
    } else {
        hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    }
    hits.into_iter().take(limit).map(|(_, i)| i).collect()
}

/// Lancements depuis la recherche : `id nombre dernier(unix)` par ligne,
/// dans `$XDG_STATE_HOME/hyprdesk/launches`.
pub fn usage_path() -> std::path::PathBuf {
    std::env::var_os("XDG_STATE_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
                .join(".local/state")
        })
        .join("hyprdesk")
        .join("launches")
}

pub type Usage = std::collections::HashMap<String, (u32, u64)>;

pub fn parse_usage(s: &str) -> Usage {
    s.lines()
        .filter_map(|l| {
            let mut it = l.rsplitn(3, ' ');
            let last = it.next()?.parse().ok()?;
            let n = it.next()?.parse().ok()?;
            Some((it.next()?.to_string(), (n, last)))
        })
        .collect()
}

pub fn load_usage() -> Usage {
    parse_usage(&std::fs::read_to_string(usage_path()).unwrap_or_default())
}

/// Note un lancement.
pub fn record_launch(id: &str) {
    let mut u = load_usage();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let e = u.entry(id.to_string()).or_insert((0, 0));
    e.0 = e.0.saturating_add(1);
    e.1 = now;
    let mut lines: Vec<String> = u.iter().map(|(k, (n, t))| format!("{k} {n} {t}")).collect();
    lines.sort();
    let p = usage_path();
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(p, lines.join("\n") + "\n");
}

/// Bonus d'usage : la fréquence compte (avec un plafond), la fraîcheur
/// aussi — une application lancée il y a des mois redescend.
pub fn usage_bonus(count: u32, last: u64, now: u64) -> i32 {
    let freq = (8.0 * (1.0 + count as f64).ln()).min(30.0);
    let days = now.saturating_sub(last) as f64 / 86_400.0;
    let fresh = if days < 1.0 {
        1.0
    } else if days < 7.0 {
        0.8
    } else if days < 30.0 {
        0.5
    } else {
        0.25
    };
    (freq * fresh).round() as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lettres_dans_l_ordre() {
        assert!(score("ffx", "Firefox").is_some());
        assert!(score("xff", "Firefox").is_none());
        assert!(score("EDITEUR", "Éditeur de texte").is_some());
        assert!(score("cafe", "Café").is_some());
        assert_eq!(score("", "Tout"), Some(0));
    }

    #[test]
    fn classement() {
        let names = [
            "Visionneuse d'images",
            "Firefox",
            "Fichiers",
            "Thunderbird",
            "LibreOffice Writer",
        ];
        // Début de nom et lettres consécutives devant ; à égalité, le nom le plus court.
        assert_eq!(rank("fi", &names, 2), vec![1, 2]);
        assert_eq!(rank("fire", &names, 1), vec![1]);
        // Début de mot à l'intérieur du nom.
        assert_eq!(rank("writer", &names, 1), vec![4]);
        // Sans requête : ordre alphabétique (accents ignorés), borné.
        assert_eq!(rank("", &names, 2), vec![2, 1]);
        assert!(rank("zzz", &names, 5).is_empty());
    }

    #[test]
    fn usage_et_classement() {
        let u = parse_usage("firefox.desktop 12 1000\nkitty.desktop 1 1000\nbad line\n");
        assert_eq!(u.get("firefox.desktop"), Some(&(12, 1000)));
        assert_eq!(u.len(), 2);
        let now = 1000 + 3600;
        assert!(usage_bonus(12, 1000, now) > usage_bonus(1, 1000, now));
        assert!(usage_bonus(12, 1000, 1000 + 90 * 86_400) < usage_bonus(12, 1000, now));
        assert!(usage_bonus(10_000, now, now) <= 30);
        // Sans requête : la plus lancée d'abord ; avec requête, le bonus départage.
        let names = ["Alacritty", "Firefox", "Fichiers"];
        assert_eq!(rank_with("", &names, &[0, 20, 0], 3), vec![1, 0, 2]);
        assert_eq!(rank_with("fi", &names, &[0, 0, 20], 3)[0], 2);
    }
}
