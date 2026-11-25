# ICO-11 — Un fichier du bureau prend l'icône de son type

Écrit contre : `a83bd07` (intégration `hig/plan-d-action`), ICO-12 en cours dans le même couloir.

## Chaîne de preuve

- Surface : icônes de fichiers du bureau (`desktop_files`, `IconKind::Path`), cellule `App::icon_cell` (`src/ui/mod.rs`), résolution `launch::resolve` (`src/launch.rs`).
- Problème : revérifié à HEAD. `launch::path_icon_name` ne connaît que cinq familles d'extensions (image, PDF, audio, vidéo) ; tout le reste (archive, tableur, script, document bureautique…) tombe sur `text-x-generic`. Aucune icône de l'application par défaut, aucune extension montrée.
- Preuve de design : HIG icons › Document icons — « macOS creates one for you by compositing your app icon and the file's extension onto the canvas. »
- Propriétaire : `src/launch.rs` (`path_icon_name`, `resolve`, `Resolved`) ; `App::icon_cell` pour le rendu.
- Incertitude : les données MIME viennent de shared-mime-info (`/usr/share/mime/globs2`, `generic-icons`), présentes sur tout bureau freedesktop ; absentes, le comportement actuel reste.

## Décision de design

Le type d'un fichier se lit d'après la base MIME freedesktop, sans dépendance nouvelle : `globs2` donne le type d'après l'extension (motifs `*.ext`, poids le plus fort, casse ignorée), lu une fois. L'icône se cherche dans le thème dans l'ordre freedesktop : nom du type (`application/x-zip` → `application-x-zip`), icône générique de `generic-icons` (`package-x-generic`), puis `<média>-x-generic`. Si le thème n'a aucune de ces icônes, l'icône de l'application par défaut du type (`mimeapps.list`, via `xdg-mime query default`) est posée, l'extension en capsule sur son bas : c'est la composition décrite par la HIG. À défaut de tout, `text-x-generic`, comme aujourd'hui. Les dossiers, le dossier personnel et les cinq familles déjà connues gardent leur icône (le nom du type passe d'abord, l'actuel en repli).

## Réutilisation

- `launch::lookup_icon` (thème, SmartUI, taille réelle) ; `launch::find_desktop_file` et `parse_desktop_entry` pour l'icône de l'application par défaut.
- `App::icon_cell` : `stack!` sur l'image ; capsule au rayon 10 (DESIGN.md), fond `th.bg` à 0,85, texte `th.fg`, `ui_font(Semibold)`, `tokens::ts(10.0)`.
- Exemple : `bar::style` (pastille d'alerte posée sur un glyphe) pour la superposition.

## Relecture indépendante (prise en compte)

- SmartUI : il génère ses icônes d'après `path_icon_name` (`smartui_desktop_names`). Si SmartUI dessine le nom actuel du fichier, il passe avant la chaîne du type : le style SmartUI reste sur les fichiers du bureau.
- `xdg-mime` est un script shell : l'icône de l'application par défaut est mise en cache par type (`OnceLock<Mutex<HashMap>>`), pour toute la durée du démon.
- `path_icon_name` garde sa signature (SmartUI, recherche) ; le repli « sans `text-x-generic` » est un test sur sa valeur.
- Aperçu : `resolve` y est synchrone et l'hôte varie. La scène ne résout pas le fichier : un `Resolved` écrit en dur (image de l'éditeur, badge `MD`) pose un fichier `notes.md` sur le bureau d'aperçu, sans base MIME ni `xdg-mime`.
- Rendu : le `stack!` se pose dans le `float` (le badge grossit avec l'image au survol) et suit `alpha` (fantôme du glisser). L'icône d'application sous le badge ne prend pas le masque ICO-04 : c'est une icône de document.
- Hors portée, noté : fichiers sans extension (Makefile) et drapeau `cs` de `globs2` (casse ignorée) ; le `xdg-mime` de la carte des propriétés (`context.rs`) reste tel quel.

## Changements

1. `src/launch.rs`
   - `mime_for(path) -> Option<String>` : table extension → (poids, type), lue une fois (`OnceLock`) dans `$XDG_DATA_HOME/mime/globs2` puis chaque `$XDG_DATA_DIRS/mime/globs2` (défaut `/usr/local/share:/usr/share`) ; seuls les motifs `*.ext` sans autre joker ; la plus longue extension composée gagne (`.tar.gz` avant `.gz`).
   - `mime_icon_names(mime) -> Vec<String>` : nom du type, entrée de `generic-icons`, `<média>-x-generic`.
   - `default_app_icon(mime) -> Option<String>` : `xdg-mime query default <type>` (absent : `None`), puis l'`Icon` de son fichier `.desktop`.
   - `Resolved` reçoit `badge: Option<String>` (extension en majuscules, quatre caractères au plus), renseigné seulement quand l'image vient de l'application par défaut. Les quatre constructeurs l'initialisent.
   - `resolve`, branche `Path` : `def.icon` d'abord ; dossier et dossier personnel inchangés ; puis noms du type, puis `path_icon_name` (repli actuel, sans `text-x-generic`), puis l'application par défaut avec badge, puis `text-x-generic`, puis `folder`.
   - Préserver : `path_icon_name` (utilisé par SmartUI) ; icône explicite d'un `IconDef`.
   - Vérifier : tests unitaires sur une `globs2` écrite en dossier temporaire (poids, casse, extension composée) et sur `mime_icon_names`.
2. `src/ui/mod.rs`, `icon_cell`
   - Change : si `self.icons[idx].badge` est `Some`, l'image est surmontée de la capsule, centrée en bas.
   - Préserver : survol, sélection, glisser, icône de remplacement.
3. `src/ui/preview.rs` : un fichier `notes.md` à badge, résolu en dur, sur le bureau d'aperçu (voir la relecture).
4. Documentation : `DESIGN.md` (icônes), `docs/GUIDE.fr.md` et `docs/GUIDE.md` (bureau de fichiers), une ligne chacun.

## Portée

- Hérite : toutes les icônes de fichiers du bureau.
- Exclure : icônes d'application (ICO-04), vignettes d'images (non demandées), dossiers.

## Validation

- Dépôt : `CARGO_BUILD_JOBS=4 bash scripts/verify.sh --visual /tmp/hig-visual-D` → 0 ; scène du bureau regardée.

## Conditions d'arrêt

- S'arrêter si la lecture de `globs2` demande une dépendance, ou si `xdg-mime` devait être appelé dans le fil de l'interface : la résolution reste dans `reload_icons`, hors du fil UI.
