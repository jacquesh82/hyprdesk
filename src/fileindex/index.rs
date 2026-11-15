//! Le moteur : index inversé de trigrammes sur les noms de fichiers.
//!
//! Même idée que plocate ou le code search de GitHub : chaque nom, replié en
//! minuscules, alimente un index `trigramme → indices d'entrées` ; une
//! requête ne consulte que les postings des trigrammes qu'elle contient —
//! le plus rare prune déjà presque tout — puis vérifie exactement les
//! candidats survivants. Recherche « par nom » par défaut ; une requête avec
//! `/` (ex. `docs/hypr`) cherche dans le chemin complet.
//!
//! Mises à jour incrémentales par ajout en bout (les indices restent
//! croissants, les postings restent triés pour la recherche dichotomique) ;
//! les suppressions posent des pierres tombales (entrée marquée morte, ses
//! postings laissés en place et filtrés à la recherche), compactées quand
//! elles s'accumulent ou effacées par le rescan suivant.

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};

use serde::{Deserialize, Serialize};

pub const KIND_FILE: u8 = 0;
pub const KIND_DIR: u8 = 1;
pub const KIND_LINK: u8 = 2;

// Catégories de fichiers : déduites de l'extension du nom au vol (rien
// n'est stocké — pas de réindexation à ajouter une catégorie).
pub const CAT_DOCUMENTS: u8 = 0;
pub const CAT_IMAGES: u8 = 1;
pub const CAT_VIDEOS: u8 = 2;
pub const CAT_PROGRAMMES: u8 = 3;
pub const CAT_AUDIO: u8 = 4;
pub const CAT_ARCHIVES: u8 = 5;
pub const CAT_AUTRES: u8 = 6;

/// Toutes les catégories nommées, dans l'ordre des puces de l'UI.
pub const CATEGORIES: &[(u8, &str)] = &[
    (CAT_DOCUMENTS, "Documents"),
    (CAT_IMAGES, "Images"),
    (CAT_VIDEOS, "Vidéos"),
    (CAT_PROGRAMMES, "Programmes"),
    (CAT_AUDIO, "Audio"),
    (CAT_ARCHIVES, "Archives"),
    (CAT_AUTRES, "Autres"),
];

/// Code d'une catégorie par nom (réglage `index.exclude_cat`), insensible
/// à la casse et aux accents majuscules (« vidéos » → [`CAT_VIDEOS`]).
pub fn categorie_code(nom: &str) -> Option<u8> {
    let n = nom.trim().to_lowercase();
    CATEGORIES
        .iter()
        .find(|(_, x)| x.to_lowercase() == n)
        .map(|(c, _)| *c)
}

/// La catégorie d'un nom de fichier (dossiers et liens : jamais appelé —
/// pas de catégorie). Sans extension connue : [`CAT_AUTRES`].
pub fn categorie_de(nom: &str) -> u8 {
    let ext = match nom.rfind('.') {
        Some(i) => nom[i + 1..].to_lowercase(),
        None => return CAT_AUTRES,
    };
    match ext.as_str() {
        "pdf" | "doc" | "docx" | "odt" | "ods" | "odp" | "rtf" | "txt" | "md" | "markdown"
        | "csv" | "tsv" | "xls" | "xlsx" | "ppt" | "pptx" | "epub" | "mobi" | "djvu" | "tex"
        | "bib" | "abw" | "pages" => CAT_DOCUMENTS,
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "tif" | "tiff" | "heic"
        | "heif" | "avif" | "jxl" | "raw" | "cr2" | "nef" | "psd" | "xcf" | "ai" | "ico" => {
            CAT_IMAGES
        }
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "wmv" | "flv" | "mpg" | "mpeg" | "m4v" | "ts"
        | "ogv" | "3gp" => CAT_VIDEOS,
        "exe" | "msi" | "appimage" | "deb" | "rpm" | "pkg" | "dmg" | "bin" | "elf" | "apk"
        | "com" | "bat" | "cmd" | "app" | "snap" | "so" | "dll" => CAT_PROGRAMMES,
        "mp3" | "flac" | "ogg" | "wav" | "m4a" | "opus" | "aac" | "wma" | "mid" | "midi"
        | "aiff" => CAT_AUDIO,
        "zip" | "tar" | "gz" | "bz2" | "xz" | "zst" | "7z" | "rar" | "iso" | "img" | "cab"
        | "jar" | "tgz" | "tbz2" => CAT_ARCHIVES,
        _ => CAT_AUTRES,
    }
}

/// Un résultat de recherche, servi au lanceur.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hit {
    /// Chemin complet du volume (ex. `/media/jacques/SAUVEGARDES/x.ntfs`).
    pub path: String,
    /// Dernier composant du chemin.
    pub name: String,
    /// [`KIND_FILE`], [`KIND_DIR`] ou [`KIND_LINK`].
    pub kind: u8,
    /// Octets (taille logique du lien pour les liens).
    pub size: u64,
    /// Époque en secondes (0 si inconnue).
    pub mtime: i64,
    /// Type du système de fichiers du volume (ext4, ntfs, vfat…).
    pub fs: String,
}

/// Un volume indexé : une racine de parcours et son type de FS.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Volume {
    pub root: String,
    pub fs: String,
}

/// Méta d'une entrée à pousser dans l'index.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EntryMeta {
    pub kind: u8,
    pub size: u64,
    pub mtime: i64,
    pub vol: u8,
}

/// L'index en mémoire : un blob de chemins concaténés (localité cache),
/// les métadonnées par entrée et les postings de trigrammes.
/// Champs visibles au crate : `store` sérialise les parties directement.
#[derive(Debug, Default)]
pub struct Index {
    /// Tous les chemins, concaténés (UTF-8, pertes tolérées au scan).
    pub(crate) blob: Vec<u8>,
    /// Début de chaque chemin dans le blob.
    pub(crate) starts: Vec<u32>,
    pub(crate) kinds: Vec<u8>,
    pub(crate) sizes: Vec<u64>,
    pub(crate) mtimes: Vec<i64>,
    pub(crate) vols: Vec<u8>,
    /// Faux après `remove_path` jusqu'à la prochaine compaction.
    pub(crate) alive: Vec<bool>,
    /// `trigramme → indices`, triés croissants. Une entrée retirée y reste
    /// jusqu'à la compaction (pierre tombale : `alive` la filtre).
    pub(crate) postings: HashMap<[u8; 3], Vec<u32>>,
    /// Empreinte du chemin complet → indice, pour les mises à jour
    /// incrémentales. Le chemin n'est pas dupliqué : il est relu dans le
    /// blob pour confirmer la correspondance.
    pub(crate) by_path: HashMap<u64, u32>,
    /// Chemins dont l'empreinte est déjà prise par un autre (collision de
    /// hachage 64 bits : vide en pratique).
    pub(crate) collisions: HashMap<Box<[u8]>, u32>,
    pub(crate) volumes: Vec<Volume>,
    pub(crate) tombstones: usize,
}

/// Compacter quand les pierres tombales dépassent un tiers des entrées
/// vivantes (et au moins quelques dizaines, pour éviter les rebuilds pour rien).
const COMPACT_RATIO: usize = 3;
const COMPACT_MIN: usize = 64;
/// Plafond de candidats collectés avant classement.
const MAX_COLLECT: usize = 20_000;

/// Un trigramme n'existe que pour un nom d'au moins 3 octets repliés ; en
/// dessous, la requête tombe sur un parcours linéaire (rare et plafonné).
const MIN_TRIGRAM: usize = 3;

/// Empreinte d'un chemin pour `by_path` : SipHash à clés fixes (rien n'est
/// persisté, seule compte la stabilité au sein du processus).
fn empreinte(path: &[u8]) -> u64 {
    BuildHasherDefault::<DefaultHasher>::default().hash_one(path)
}

/// Caractères qui bornent un « mot » dans un nom de fichier.
fn borne(b: u8) -> bool {
    b.is_ascii_whitespace()
        || matches!(
            b,
            b'/' | b'-' | b'_' | b'.' | b'(' | b'[' | b'{' | b'\'' | b'@' | b'+' | b'#'
        )
}

/// Trigrammes uniques d'une chaîne repliée, sur les octets (le repli est le
/// même à l'indexation et à la requête, donc un trigramme à cheval sur un
/// caractère multi-octets reste cohérent).
fn trigrammes(folded: &str) -> Vec<[u8; 3]> {
    let b = folded.as_bytes();
    if b.len() < MIN_TRIGRAM {
        return Vec::new();
    }
    let mut set = HashSet::with_capacity(b.len());
    for w in b.windows(MIN_TRIGRAM) {
        set.insert([w[0], w[1], w[2]]);
    }
    set.into_iter().collect()
}

impl Index {
    /// Index vide, prêt à recevoir les volumes et les entrées.
    pub fn new() -> Self {
        Self::default()
    }

    /// Reconstruit un index depuis ses parties (chargement du snapshot,
    /// compaction) : les postings et la table chemin→indice sont recalculés.
    pub fn from_parts(
        blob: Vec<u8>,
        starts: Vec<u32>,
        kinds: Vec<u8>,
        sizes: Vec<u64>,
        mtimes: Vec<i64>,
        vols: Vec<u8>,
        volumes: Vec<Volume>,
    ) -> Self {
        let n = starts.len();
        let mut idx = Self {
            blob,
            starts,
            kinds,
            sizes,
            mtimes,
            vols,
            alive: vec![true; n],
            postings: HashMap::new(),
            by_path: HashMap::with_capacity(n),
            collisions: HashMap::new(),
            volumes,
            tombstones: 0,
        };
        for i in 0..n {
            let path = idx.path(i).into_owned();
            for t in trigrammes(&dernier_composant(&path).to_lowercase()) {
                idx.postings.entry(t).or_default().push(i as u32);
            }
            idx.inscrire(empreinte(path.as_bytes()), path.as_bytes(), i as u32);
        }
        idx.ajuster();
        idx
    }

    /// Construit un index complet depuis un scan (une entrée par fichier).
    pub fn build(
        entries: impl IntoIterator<Item = (String, EntryMeta)>,
        volumes: Vec<Volume>,
    ) -> Self {
        let mut idx = Self::new();
        idx.volumes = volumes;
        for (path, meta) in entries {
            idx.push(&path, meta);
        }
        idx.ajuster();
        idx
    }

    /// Rend la capacité excédentaire une fois la construction finie (un
    /// `Vec` double en grandissant : jusqu'à moitié de vide par posting).
    fn ajuster(&mut self) {
        for l in self.postings.values_mut() {
            l.shrink_to_fit();
        }
        self.postings.shrink_to_fit();
        self.blob.shrink_to_fit();
    }

    // ——— table chemin → indice (empreinte, confirmée dans le blob) ———

    /// Indice d'un chemin indexé (vivant).
    fn indice_de(&self, path: &[u8]) -> Option<u32> {
        self.chercher(empreinte(path), path)
    }

    fn chercher(&self, h: u64, path: &[u8]) -> Option<u32> {
        if let Some(&i) = self.by_path.get(&h)
            && self.chemin_brut(i as usize) == path
        {
            return Some(i);
        }
        if self.collisions.is_empty() {
            return None;
        }
        self.collisions.get(path).copied()
    }

    /// Inscrit un chemin encore absent de la table.
    fn inscrire(&mut self, h: u64, path: &[u8], i: u32) {
        match self.by_path.entry(h) {
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(i);
            }
            std::collections::hash_map::Entry::Occupied(_) => {
                self.collisions.insert(path.to_vec().into_boxed_slice(), i);
            }
        }
    }

    /// Désinscrit le chemin d'indice `i`.
    fn desinscrire(&mut self, h: u64, path: &[u8], i: u32) {
        if self.by_path.get(&h) == Some(&i) {
            self.by_path.remove(&h);
        } else {
            self.collisions.remove(path);
        }
    }

    /// Volumes enregistrés.
    pub fn volumes(&self) -> &[Volume] {
        &self.volumes
    }

    /// Nombre d'entrées vivantes.
    pub fn len(&self) -> usize {
        self.alive.len() - self.tombstones
    }

    /// Faut-il compacter ?
    pub fn needs_compaction(&self) -> bool {
        self.tombstones > COMPACT_MIN && self.tombstones * COMPACT_RATIO > self.alive.len()
    }

    /// Compaction : reconstruit les parties vivantes.
    pub fn compact(&mut self) {
        if !self.needs_compaction() {
            return;
        }
        let mut blob = Vec::with_capacity(self.blob.len());
        let mut starts = Vec::with_capacity(self.len());
        let mut kinds = Vec::with_capacity(self.len());
        let mut sizes = Vec::with_capacity(self.len());
        let mut mtimes = Vec::with_capacity(self.len());
        let mut vols = Vec::with_capacity(self.len());
        for i in 0..self.starts.len() {
            if !self.alive[i] {
                continue;
            }
            let (s, e) = self.bounds(i);
            starts.push(blob.len() as u32);
            blob.extend_from_slice(&self.blob[s..e]);
            kinds.push(self.kinds[i]);
            sizes.push(self.sizes[i]);
            mtimes.push(self.mtimes[i]);
            vols.push(self.vols[i]);
        }
        let volumes = std::mem::take(&mut self.volumes);
        *self = Self::from_parts(blob, starts, kinds, sizes, mtimes, vols, volumes);
    }

    /// Ajoute une entrée (append en bout : les postings restent triés).
    pub fn push(&mut self, path: &str, meta: EntryMeta) {
        let h = empreinte(path.as_bytes());
        if let Some(i) = self.chercher(h, path.as_bytes()) {
            // déjà présent : mise à jour des attributs suffit
            self.touch(i, meta);
            return;
        }
        let i = self.starts.len();
        self.starts.push(self.blob.len() as u32);
        self.blob.extend_from_slice(path.as_bytes());
        self.kinds.push(meta.kind);
        self.sizes.push(meta.size);
        self.mtimes.push(meta.mtime);
        self.vols.push(meta.vol);
        self.alive.push(true);
        for t in trigrammes(&dernier_composant(path).to_lowercase()) {
            self.postings.entry(t).or_default().push(i as u32);
        }
        self.inscrire(h, path.as_bytes(), i as u32);
    }

    /// Mise à jour d'attributs sans changement de nom (taille, mtime…).
    pub fn touch(&mut self, i: u32, meta: EntryMeta) {
        let i = i as usize;
        if i >= self.kinds.len() || !self.alive[i] {
            return;
        }
        self.kinds[i] = meta.kind;
        self.sizes[i] = meta.size;
        self.mtimes[i] = meta.mtime;
    }

    /// Nature ([`KIND_FILE`], [`KIND_DIR`], [`KIND_LINK`]) d'une entrée
    /// indexée.
    pub fn kind_of(&self, path: &str) -> Option<u8> {
        self.indice_de(path.as_bytes())
            .map(|i| self.kinds[i as usize])
    }

    /// Retire une entrée disparue (veille) ; renvoie le nombre d'entrées
    /// retirées. Seul un dossier emporte ses descendants : la nature est
    /// lue dans l'index avant la suppression (le chemin n'existe plus, on
    /// ne peut plus le stat-iser). Un fichier ne coûte qu'une recherche
    /// dans la table, jamais un parcours de l'index.
    pub fn remove(&mut self, path: &str) -> usize {
        let Some(kind) = self.kind_of(path) else {
            return 0;
        };
        self.remove_path(path);
        if kind == KIND_DIR {
            1 + self.remove_prefix(path)
        } else {
            1
        }
    }

    /// Retire tout ce qui se trouve sous `dir` (dossier supprimé ou déplacé
    /// hors d'un volume indexé), `dir` lui-même exclu. Les enfants ont
    /// chacun leur entrée ; un dossier déplacé hors du volume ne génère pas
    /// d'événement par fichier, d'où la purge en bloc. Parcourt le blob une
    /// fois, sans allocation par entrée : réservé aux dossiers (voir
    /// [`Index::remove`]).
    pub fn remove_prefix(&mut self, dir: &str) -> usize {
        let prefix = if dir.ends_with('/') {
            dir.to_string()
        } else {
            format!("{dir}/")
        };
        let sous = prefix.as_bytes();
        let touches: Vec<usize> = (0..self.starts.len())
            .filter(|&i| self.alive[i] && self.chemin_brut(i).starts_with(sous))
            .collect();
        for &i in &touches {
            self.enterrer(i);
        }
        touches.len()
    }

    /// Retire une entrée par chemin (renvoie vrai si elle était indexée).
    /// Pierre tombale : l'entrée est marquée morte et quitte la table des
    /// chemins ; ses postings restent jusqu'à la compaction (la recherche
    /// filtre `alive`), plutôt que des `Vec::remove` en O(longueur) sur
    /// chacun de ses trigrammes.
    pub fn remove_path(&mut self, path: &str) -> bool {
        let Some(i) = self.indice_de(path.as_bytes()) else {
            return false;
        };
        self.enterrer(i as usize);
        true
    }

    /// Pose la pierre tombale de l'entrée vivante `i`.
    fn enterrer(&mut self, i: usize) {
        // copie du chemin (courte) : `desinscrire` emprunte `self` en écriture
        let path = self.chemin_brut(i).to_vec();
        self.desinscrire(empreinte(&path), &path, i as u32);
        self.alive[i] = false;
        self.tombstones += 1;
    }

    /// Recherche : trigrammes du needle → intersection des postings →
    /// vérification exacte → classement heuristique (préfixe de nom d'abord).
    /// Recherche, éventuellement restreinte à certaines natures
    /// ([`KIND_FILE`], [`KIND_DIR`], [`KIND_LINK`]) et à une catégorie
    /// ([`categorie_de`]) — `None` : tout.
    pub fn search(
        &self,
        query: &str,
        limit: usize,
        kinds: Option<&[u8]>,
        cat: Option<u8>,
    ) -> Vec<Hit> {
        let fq = query.trim().to_lowercase();
        if fq.is_empty() || limit == 0 {
            return Vec::new();
        }
        let path_mode = fq.contains('/');
        // Les trigrammes portent sur les noms : une requête avec « / »
        // s'appuie sur son dernier composant s'il est assez long
        // (« docs/lisez » → trigrammes de « lisez », vérification sur le
        // chemin) ; un préfixe de dossier (« docs/ ») tombe sur le repli
        // linéaire, les noms ne contiennent jamais de « / ».
        let (needle, chemin_seul) = if path_mode {
            match fq.rsplit_once('/') {
                Some((_, tail)) if tail.len() >= MIN_TRIGRAM => (tail.to_string(), false),
                _ => (fq.clone(), true),
            }
        } else {
            (fq.clone(), false)
        };

        let candidats = if !chemin_seul && needle.len() >= MIN_TRIGRAM {
            self.candidats_trigrammes(&needle)
        } else {
            self.candidats_lineaires(&fq, path_mode)
        };

        let mut marques: Vec<(u8, usize, String, Hit)> = Vec::new();
        for i in candidats {
            if !self.alive[i as usize] {
                continue;
            }
            if let Some(ks) = kinds
                && !ks.contains(&self.kinds[i as usize])
            {
                continue;
            }
            let raw = self.path(i as usize);
            let name = dernier_composant(&raw);
            if let Some(c) = cat
                && categorie_de(&name) != c
            {
                continue;
            }
            let name = dernier_composant(&raw);
            let cible: &str = if path_mode {
                raw.as_ref()
            } else {
                name.as_ref()
            };
            let folded_name = name.to_lowercase();
            if !cible.to_lowercase().contains(&fq) {
                continue;
            }
            let score = score_nom(&folded_name, &needle, path_mode);
            marques.push((score, name.len(), folded_name, self.hit(i as usize)));
        }
        // départage déterministe : score, longueur du nom, ordre alphabétique
        // replié (les ex æquo sont fréquents sur les préfixes courts)
        marques.sort_unstable_by(|a, b| (a.0, a.1, &a.2).cmp(&(b.0, b.1, &b.2)));
        marques.truncate(limit);
        marques.into_iter().map(|(_, _, _, h)| h).collect()
    }

    /// Candidats par intersection de postings : on itère la liste du
    /// trigramme le plus rare et on vérifie les autres par dichotomie.
    fn candidats_trigrammes(&self, needle: &str) -> Vec<u32> {
        let tris = trigrammes(needle);
        if tris.is_empty() {
            return Vec::new();
        }
        let mut listes: Vec<&[u32]> = Vec::with_capacity(tris.len());
        for t in &tris {
            match self.postings.get(t) {
                Some(l) if !l.is_empty() => listes.push(l),
                _ => return Vec::new(), // un trigramme absent : rien ne peut correspondre
            }
        }
        listes.sort_by_key(|l| l.len());
        let (premiere, autres) = listes.split_first().expect("au moins une liste");
        let mut out = Vec::new();
        for &i in *premiere {
            // pierre tombale : ne prend pas de place sous le plafond
            if !self.alive[i as usize] {
                continue;
            }
            if autres.iter().all(|l| l.binary_search(&i).is_ok()) {
                out.push(i);
                if out.len() >= MAX_COLLECT {
                    break;
                }
            }
        }
        out
    }

    /// Repli pour les requêtes courtes (< 3 octets) : parcours linéaire
    /// plafonné. Coûteux sur un gros index, mais ces requêtes sont rares.
    fn candidats_lineaires(&self, fq: &str, path_mode: bool) -> Vec<u32> {
        let mut out = Vec::new();
        for i in 0..self.starts.len() {
            if !self.alive[i] {
                continue;
            }
            let raw = self.path(i);
            let name = dernier_composant(&raw);
            let cible: &str = if path_mode {
                raw.as_ref()
            } else {
                name.as_ref()
            };
            if cible.to_lowercase().contains(fq) {
                out.push(i as u32);
                if out.len() >= MAX_COLLECT {
                    break;
                }
            }
        }
        out
    }

    /// Construit le résultat d'un indice.
    fn hit(&self, i: usize) -> Hit {
        let raw = self.path(i);
        Hit {
            name: dernier_composant(&raw).into_owned(),
            path: raw.into_owned(),
            kind: self.kinds[i],
            size: self.sizes[i],
            mtime: self.mtimes[i],
            fs: self
                .volumes
                .get(self.vols[i] as usize)
                .map(|v| v.fs.clone())
                .unwrap_or_default(),
        }
    }

    /// Chemin d'une entrée, lu dans le blob.
    fn path(&self, i: usize) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(self.chemin_brut(i))
    }

    /// Octets du chemin d'une entrée, tels que dans le blob.
    fn chemin_brut(&self, i: usize) -> &[u8] {
        let (s, e) = self.bounds(i);
        &self.blob[s..e]
    }

    fn bounds(&self, i: usize) -> (usize, usize) {
        let s = self.starts[i] as usize;
        let e = if i + 1 < self.starts.len() {
            self.starts[i + 1] as usize
        } else {
            self.blob.len()
        };
        (s, e)
    }
}

/// Dernier composant d'un chemin.
fn dernier_composant(p: &str) -> std::borrow::Cow<'_, str> {
    match p.rfind('/') {
        Some(i) => std::borrow::Cow::Borrowed(&p[i + 1..]),
        None => std::borrow::Cow::Borrowed(p),
    }
}

/// Score de classement : préfixe du nom (0) > début de mot (1) >
/// sous-chaîne du nom (2) > correspondance de chemin seulement (3).
fn score_nom(folded_name: &str, needle: &str, path_mode: bool) -> u8 {
    if path_mode && !folded_name.contains(needle) {
        return 3;
    }
    if folded_name.starts_with(needle) {
        return 0;
    }
    if folded_name
        .bytes()
        .enumerate()
        .any(|(i, b)| b == needle.as_bytes()[0] && (i == 0 || borne(folded_name.as_bytes()[i - 1])))
        && folded_name.contains(needle)
    {
        return 1;
    }
    if folded_name.contains(needle) {
        return 2;
    }
    3
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta(kind: u8) -> EntryMeta {
        EntryMeta {
            kind,
            size: 42,
            mtime: 1_700_000_000,
            vol: 0,
        }
    }

    fn index_exemple() -> Index {
        let mut idx = Index::new();
        idx.volumes = vec![Volume {
            root: "/home/test".into(),
            fs: "ext4".into(),
        }];
        for (p, k) in [
            ("/home/test/hyprdesk.lua", KIND_FILE),
            ("/home/test/HyprDesk.rs", KIND_FILE),
            ("/home/test/docs/lisez-moi.md", KIND_FILE),
            ("/home/test/docs", KIND_DIR),
            ("/home/test/lien-linux", KIND_LINK),
        ] {
            idx.push(p, meta(k));
        }
        idx
    }

    #[test]
    fn recherche_nom_insensible_a_la_casse() {
        let idx = index_exemple();
        let hits = idx.search("hyprdesk", 10, None, None);
        assert_eq!(hits.len(), 2); // hyprdesk.lua + HyprDesk.rs (ex æquo : préfixes)
        assert_eq!(hits[0].size, 42);
        assert_eq!(hits[0].fs, "ext4");
        let noms: std::collections::HashSet<&str> = hits.iter().map(|h| h.name.as_str()).collect();
        assert!(noms.contains("hyprdesk.lua"));
        assert!(noms.contains("HyprDesk.rs"));
    }

    #[test]
    fn recherche_prefixe_classe_en_premier() {
        let mut idx = index_exemple();
        // un concurrent qui ne fait que contenir « hy » doit perdre face
        // aux deux préfixes
        idx.push("/home/test/essai-hy.txt", meta(KIND_FILE));
        let hits = idx.search("hy", 10, None, None);
        assert!(hits.iter().any(|h| h.name == "essai-hy.txt"));
        // « HyprDesk.rs » comme « hyprdesk.lua » : préfixes, repliés
        assert!(hits[0].name.to_lowercase().starts_with("hy"));
    }

    #[test]
    fn recherche_par_mot_dans_le_nom() {
        let idx = index_exemple();
        // « moi » commence un mot dans lisez-moi.md
        let hits = idx.search("moi", 10, None, None);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "lisez-moi.md");
    }

    #[test]
    fn recherche_chemin_avec_slash() {
        let idx = index_exemple();
        let hits = idx.search("docs/lisez", 10, None, None);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, "/home/test/docs/lisez-moi.md");
        // rien ne correspond dans le nom seul : le slash cible le chemin
        assert!(idx.search("docs/inconnu", 10, None, None).is_empty());
    }

    #[test]
    fn requete_courte_repli_lineaire() {
        let idx = index_exemple();
        let hits = idx.search("md", 10, None, None);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "lisez-moi.md");
    }

    #[test]
    fn suppression_et_compaction() {
        let mut idx = index_exemple();
        assert!(idx.remove_path("/home/test/HyprDesk.rs"));
        assert!(!idx.remove_path("/home/test/absent.rs"));
        assert!(idx.search("HyprDesk.rs", 10, None, None).is_empty());
        assert_eq!(idx.len(), 4);
        // beaucoup de suppressions pour déclencher la compaction
        for i in 0..200 {
            idx.push(&format!("/home/test/tmp{i}.txt"), meta(KIND_FILE));
        }
        for i in 0..200 {
            assert!(idx.remove_path(&format!("/home/test/tmp{i}.txt")));
        }
        assert!(idx.needs_compaction());
        idx.compact();
        assert_eq!(idx.tombstones, 0);
        assert_eq!(idx.len(), 4);
        // l'index reste fonctionnel après compaction
        assert_eq!(idx.search("hyprdesk", 10, None, None).len(), 1);
        assert!(idx.search("lisez", 10, None, None).len() == 1);
    }

    #[test]
    fn prefixe_de_dossier_en_repli_lineaire() {
        let idx = index_exemple();
        // Pas de composant final exploitable (« docs/ ») : repli linéaire sur
        // le chemin. Le dossier « docs » lui-même ne correspond pas (son
        // chemin stocké n'a pas de « / » final), les fichiers dedans oui.
        let hits = idx.search("home/test/docs/", 10, None, None);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "lisez-moi.md");
    }

    #[test]
    fn categories_par_extension() {
        assert_eq!(categorie_de("rapport.pdf"), CAT_DOCUMENTS);
        assert_eq!(categorie_de("PHOTO.JPG"), CAT_IMAGES);
        assert_eq!(categorie_de("film.mkv"), CAT_VIDEOS);
        assert_eq!(categorie_de("installeur.AppImage"), CAT_PROGRAMMES);
        assert_eq!(categorie_de("musique.flac"), CAT_AUDIO);
        assert_eq!(categorie_de("archive.tar.gz"), CAT_ARCHIVES); // dernier suffixe
        assert_eq!(categorie_de("Makefile"), CAT_AUTRES);
        assert_eq!(categorie_de("inconnu.xyz"), CAT_AUTRES);
    }

    #[test]
    fn filtre_par_categorie() {
        let idx = index_exemple();
        // l'exemple ne contient que .lua, .rs, .md et un lien
        let hits = idx.search("lisez", 10, None, Some(CAT_DOCUMENTS));
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "lisez-moi.md");
        assert!(idx.search("lisez", 10, None, Some(CAT_IMAGES)).is_empty());
        // le filtre de catégorie implique fichiers : le dossier est écarté
        let hits = idx.search("docs", 10, Some(&[KIND_DIR]), Some(CAT_DOCUMENTS));
        assert!(hits.is_empty());
    }

    #[test]
    fn filtre_par_nature() {
        let idx = index_exemple();
        // dossiers seuls
        let hits = idx.search("docs", 10, Some(&[KIND_DIR]), None);
        assert!(hits.iter().all(|h| h.kind == KIND_DIR));
        assert_eq!(hits.len(), 1);
        // fichiers seuls : le dossier « docs » est écarté
        let hits = idx.search("docs", 10, Some(&[KIND_FILE]), None);
        assert!(hits.iter().all(|h| h.kind == KIND_FILE));
        // liens seuls
        let hits = idx.search("linux", 10, Some(&[KIND_LINK]), None);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].kind, KIND_LINK);
        // nature sans résultat
        assert!(
            idx.search("hyprdesk", 10, Some(&[KIND_DIR]), None)
                .is_empty()
        );
    }

    #[test]
    fn mise_a_jour_sans_dupliquer() {
        let mut idx = index_exemple();
        idx.push(
            "/home/test/hyprdesk.lua",
            EntryMeta {
                size: 99,
                ..meta(KIND_FILE)
            },
        );
        assert_eq!(idx.len(), 5);
        let hits = idx.search("hyprdesk.lua", 10, None, None);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].size, 99);
    }

    #[test]
    fn suppression_d_un_fichier_sans_purge_de_prefixe() {
        let mut idx = index_exemple();
        // entrée fantôme « sous » un fichier : impossible sur disque, elle
        // ne survit que si la suppression du fichier ne purge aucun préfixe
        idx.push("/home/test/hyprdesk.lua/fantome", meta(KIND_FILE));
        assert_eq!(idx.kind_of("/home/test/hyprdesk.lua"), Some(KIND_FILE));
        assert_eq!(idx.remove("/home/test/hyprdesk.lua"), 1);
        assert_eq!(idx.kind_of("/home/test/hyprdesk.lua"), None);
        assert_eq!(idx.search("fantome", 10, None, None).len(), 1);
        // chemin inconnu : rien, et surtout pas de purge
        assert_eq!(idx.remove("/home/test/absent"), 0);
        assert_eq!(idx.search("fantome", 10, None, None).len(), 1);
    }

    #[test]
    fn suppression_d_un_dossier_emporte_ses_enfants() {
        let mut idx = index_exemple();
        idx.push("/home/test/docs/sous/profond.txt", meta(KIND_FILE));
        // voisin au nom proche : hors du sous-arbre
        idx.push("/home/test/docs-bis.txt", meta(KIND_FILE));
        let avant = idx.len();
        // docs + lisez-moi.md + sous/profond.txt (sous/ n'a pas d'entrée)
        assert_eq!(idx.remove("/home/test/docs"), 3);
        assert_eq!(idx.len(), avant - 3);
        assert!(idx.search("lisez", 10, None, None).is_empty());
        assert!(idx.search("profond", 10, None, None).is_empty());
        assert_eq!(idx.search("docs-bis", 10, None, None).len(), 1);
    }

    #[test]
    fn pierre_tombale_sans_toucher_aux_postings() {
        let mut idx = index_exemple();
        let postings_avant: usize = idx.postings.values().map(Vec::len).sum();
        assert!(idx.remove_path("/home/test/hyprdesk.lua"));
        let postings_apres: usize = idx.postings.values().map(Vec::len).sum();
        assert_eq!(postings_avant, postings_apres);
        let hits = idx.search("hyprdesk", 10, None, None);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].name, "HyprDesk.rs");
        // recréé : nouvelle entrée, retrouvée
        idx.push("/home/test/hyprdesk.lua", meta(KIND_FILE));
        assert_eq!(idx.search("hyprdesk", 10, None, None).len(), 2);
        assert_eq!(idx.len(), 5);
    }

    #[test]
    fn collision_d_empreintes_resolue() {
        let mut idx = index_exemple();
        let a = b"/home/test/hyprdesk.lua";
        let b = b"/home/test/HyprDesk.rs";
        let ia = idx.indice_de(a).expect("a");
        let ib = idx.indice_de(b).expect("b");
        // même empreinte forcée pour deux chemins distincts
        idx.by_path.clear();
        idx.inscrire(7, a, ia);
        idx.inscrire(7, b, ib);
        assert_eq!(idx.chercher(7, a), Some(ia));
        assert_eq!(idx.chercher(7, b), Some(ib));
        idx.desinscrire(7, a, ia);
        assert_eq!(idx.chercher(7, a), None);
        assert_eq!(idx.chercher(7, b), Some(ib));
        idx.desinscrire(7, b, ib);
        assert_eq!(idx.chercher(7, b), None);
        assert!(idx.collisions.is_empty());
    }

    #[test]
    fn requete_vide_et_limites() {
        let idx = index_exemple();
        assert!(idx.search("", 10, None, None).is_empty());
        assert!(idx.search("   ", 10, None, None).is_empty());
        assert_eq!(idx.search("hyprdesk", 1, None, None).len(), 1);
    }
}
