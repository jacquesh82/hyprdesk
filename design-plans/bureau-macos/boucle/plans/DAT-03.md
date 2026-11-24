# DAT-03 — Le résultat de fichier sélectionné porte un aperçu, et un échec d'ouverture dit sa cause

Écrit contre : `a210057` (intégration `hig/plan-d-action`).

## Relecture du plan

Relu par un agent indépendant avant exécution : hauteur fixe de la carte, garde du mode `:f` dans `search_step`, travail non borné d'une flèche tenue, bombe de décodage, FIFO, chemin dans l'erreur, code 5 — tous intégrés ci-dessous. Diff relu de même : ligne de texte coupée au bord au lieu de déborder sur la date, rayon ramené à 10 px ; `open_path` garde son appel immédiat pour `about.rs` et `bar/menus.rs` (fil de l'interface), l'attente passe par `open_path_checked`.

## Chaîne de preuve

- Surface : lanceur en mode fichiers (`:f`), liste des résultats du démon d'index (`App::view_files`, `src/ui/search.rs`).
- Problème : un résultat se réduit à une icône de type, au nom et à « taille · dossier » ; rien ne montre ce que contient le fichier avant de l'ouvrir. L'ouverture (`crate::launch::open_path`) lance `xdg-open` sans attendre son code de sortie : seul l'échec du `spawn` est signalé. `xdg-open` qui ne trouve pas d'application (code 3) ou dont l'action échoue (code 4) passe pour un succès.
- Recensement à HEAD : la moitié « signaler l'échec à l'écran » est déjà en partie faite par PAT-11 (notification « Ouverture impossible : … » dans `SMsg::Launch` du mode fichiers) ; elle ne couvre que l'échec du lancement de `xdg-open`, pas celui de `xdg-open` lui-même. Lignes relevées à `00f6d69` déplacées, retrouvées par le contenu.
- Preuve de design : HIG file-management › Quick Look previews — « Use a Quick Look viewer to let people preview a file even when your app can't open it. »
- Propriétaires : `App::view_files` et `SMsg` (`src/ui/search.rs`) ; `crate::launch::open_path` (`src/launch.rs`) ; vignette d'image : `wizard::scene_thumb` (`src/ui/wizard.rs`, décodage hors du fil de l'interface, recadrage 384 × 216).
- Périmètre : `src/ui/search.rs`, `src/launch.rs`, `src/ui/wizard.rs` (visibilité de `scene_thumb` inchangée : `pub(super)` suffit, `search` est un module frère), `locales/en.toml`, `docs/GUIDE*.md`.
- Incertitude : `xdg-open` en environnement générique (Hyprland) attend parfois la fin de l'application qu'il lance ; on ne peut pas attendre son code sans borne. Décision ci-dessous.

## Décision de design

- **Aperçu du résultat sélectionné**, sous sa ligne, dans le même bouton (un clic ouvre toujours) : la ligne sélectionnée passe de `ROW` (46 px) à `ROW + 48` px ; une rangée d'aperçu de 40 px s'y ajoute, retrait aligné sur le nom (icône 22 px + espacement 14 px).
  - Image (catégorie `CAT_IMAGES` de `fileindex::index::categorie_de`, fichier ordinaire de 32 Mio au plus, 40 mégapixels au plus d'après `image::image_dimensions` lu avant tout décodage) : vignette 64 × 36 px (`scene_thumb`, recadrée 16:9), rayon 10 px (échelle de `DESIGN.md`) — à gauche ; à droite, la date. Format que la caisse `image` ne décode pas (svg, heic, raw, psd…) : la date seule.
  - Texte (fichier ordinaire — `metadata().is_file()`, une FIFO n'est jamais ouverte — dont les 4 096 premiers octets sont de l'UTF-8 sans octet nul, une coupure en fin de tampon tolérée) : ses deux premières lignes non vides, 80 caractères au plus chacune, 12 px, encre secondaire, la date à droite de la première ; une ligne trop longue pour la place est coupée au bord (`container::clip`).
  - Sinon (dossier, lien, binaire, gros fichier, lecture impossible) : la date seule.
  - La date : « Modifié le JJ/MM/AAAA HH:MM » (heure locale, `Hit::mtime`) ; `mtime == 0` : rien.
  - Pendant le chargement, la rangée garde sa hauteur (la date seule) : la liste ne saute pas quand l'aperçu arrive.
- **Chargement** : un seul aperçu à la fois, celui de la sélection, lu hors du fil de l'interface (`spawn_blocking`). `Search::apercu : Option<(String, Apercu)>` garde le chemin et son aperçu ; un aperçu arrivé pour un autre chemin que la sélection est jeté (règle pure `Search::garde_apercu`, testée). Deux points d'entrée suffisent : `App::search_step` (flèches, Tab ; tous les modes, d'où la garde « mode `:f` seulement ») et la réception `SMsg::Files` (nouveaux résultats, sélection remise à 0). Une flèche tenue ne décode pas une image par pas : la demande part après 120 ms (`SMsg::ApercuVoulu(chemin)`), et seulement si ce chemin est encore la sélection.
- **Échec d'ouverture** : `open_path` attend `xdg-open` au plus 1,5 s (`try_wait` toutes les 50 ms). Sorti avant : un code non nul devient une erreur nommée, chemin compris (2 : fichier introuvable ; 3 : aucune application pour ce type ; 4 : l'ouverture a échoué ; 5 : lecture refusée ; autre : code n). Limite assumée : en environnement générique, une application lancée par `xdg-open` qui échoue avant 1,5 s rend son propre code. Toujours en cours : succès présumé, un fil récupère le processus à sa fin (pas de zombie). L'erreur emprunte le chemin de PAT-11 (notification « Ouverture impossible : … ») : le lanceur s'est déjà refermé, la notification est la place visible. Le lanceur ne reste pas ouvert 1,5 s.
- Hors correction : le suivi de la sélection dans la carte de hauteur fixe (aucun `scroll_to` dans `search.rs`, défaut antérieur que la ligne plus haute rend un peu plus visible : à reprendre à part) ; aperçu des modes applications, presse-papiers, coffre, réglages ; aperçu plein (Quick Look) au clavier.

## Réutilisation

- `wizard::scene_thumb` pour la vignette (décodage `image`, déjà dépendance).
- `crate::fileindex::index::{categorie_de, CAT_IMAGES, KIND_FILE}` pour la nature.
- Format de date : l'idiome `chrono::DateTime::from_timestamp(t, 0)…with_timezone(&chrono::Local).format("%d/%m/%Y %H:%M")` de `mymfa_fenetre.rs`.
- Encres et taille : `dim`, `crate::ui::tokens::ts(12.0)`, `self.font` ; style de ligne `crate::ui::tokens::item` inchangé.
- Notification d'échec : celle de PAT-11, inchangée.

Pas de primitive nouvelle.

## Changements

1. `src/launch.rs`
   - Changement : `open_path` attend `xdg-open` au plus 1,5 s et traduit son code de sortie en erreur ; au-delà, un fil `wait()` le récupère. Fonction pure `xdg_open_error(code) -> Option<String>` pour le libellé.
   - Préserver : signature `Result<(), String>`, flux standard à `null`, détachement.
   - Vérifier : test unitaire de `xdg_open_error` (0 → None, 3 → « aucune application »).
2. `src/ui/search.rs`
   - Changement : `enum Apercu { Attente, Image(Handle), Texte(Vec<String>), Rien }` ; champ `Search::apercu` ; `SMsg::Apercu(String, Apercu)` ; `App::file_preview()` (tâche pour la sélection si son chemin n'est pas celui de `apercu`) appelé depuis `search_step` et `SMsg::Files` ; fonction pure `texte_apercu(&[u8]) -> Option<Vec<String>>` ; rendu dans `view_files`.
   - Préserver : classement, puces, ouverture au clic et à Entrée, autres modes.
   - Vérifier : tests de `texte_apercu` (UTF-8 coupé, octet nul, lignes vides, coupe à 80) et de `Search::garde_apercu` (un aperçu pour un autre chemin est jeté). Le littéral `Search { … }` reçoit `apercu: None`.
3. `locales/en.toml` : « Modifié le {d} », libellés d'erreur de `xdg-open`.
4. `docs/GUIDE.fr.md`, `docs/GUIDE.md` : une phrase dans la section de la recherche de fichiers.

## Portée

- Hérite : le mode `:f` du lanceur, en carte flottante comme en surface convexe.
- Vérifier : les scènes d'aperçu du lanceur (aucune ne montre `:f` avec résultats : pas de changement de capture attendu).
- Exclut : les autres modes du lanceur ; le dossier Bureau.

## Validation

- Produit : taper `:f rapport`, descendre aux flèches : l'aperçu suit la sélection ; une image montre sa vignette ; ouvrir un fichier sans application associée montre « Ouverture impossible : aucune application pour ce type de fichier ».
- Interface : ligne sélectionnée plus haute, aucune autre ligne ne change ; carte de hauteur fixe : les lignes du bas défilent (surface convexe) ou sont coupées comme aujourd'hui (carte flottante).
- Système : pas de cache d'aperçus, pas de nouveau composant ; vignette par `scene_thumb`.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh` → code 0.

## Conditions d'arrêt

- Arrêter si l'aperçu demande une nouvelle dépendance, ou si la ligne plus haute casse une scène d'aperçu existante.

## Documentation de design

- Aucune entrée nouvelle dans `DESIGN.md` : le guide décrit l'aperçu.
