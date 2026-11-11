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
pub fn rank(query: &str, names: &[&str], limit: usize) -> Vec<usize> {
    let mut hits: Vec<(i32, usize)> = names
        .iter()
        .enumerate()
        .filter_map(|(i, n)| score(query, n).map(|s| (s, i)))
        .collect();
    if query.trim().is_empty() {
        hits.sort_by(|a, b| {
            let fa: String = names[a.1].chars().map(fold).collect();
            let fb: String = names[b.1].chars().map(fold).collect();
            fa.cmp(&fb)
        });
    } else {
        hits.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    }
    hits.into_iter().take(limit).map(|(_, i)| i).collect()
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
}
