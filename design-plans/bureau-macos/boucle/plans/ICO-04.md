# Les icônes d'application prennent un masque unique en rectangle arrondi

Écrit contre : `8cd7dc5` (branche `hig/plan-A`)

## Chaîne de preuve

- Surface : icônes d'application du dock, du lanceur (grille, suggestions), du bureau, du commutateur, de la vue d'ensemble, de « Forcer à quitter », de l'assistant.
- Problème : aucune découpe. `handle_for` charge l'image brute (`src/ui/mod.rs`), `icon_named` et `img_element` la posent telle quelle ; la forme vient du thème installé (cercles avec Tela-circle) ou de la ressource SmartUI (ICO-05).
- Règle : app-icons › Icon shape — « provide square layers so the system can apply rounded corners » ; « the system applies masking to produce rounded corners that precisely match the curvature of other rounded interface elements throughout the system ».
- Décision du propriétaire D4 : un masque unique, rectangle arrondi ; SmartUI génère un carré plein cadre (ICO-05).
- Propriétaires : `handle_for` (chargement), `icon_named` et `img_element` (rendu), `src/ui/tokens.rs` (rapport du masque).
- Contrainte : iced 0.14 arrondit une image matricielle (`image::border_radius`) mais pas un SVG, et `resvg` n'est pas une dépendance directe (en ajouter une est interdit). Le SVG se découpe donc dans sa source : un `clipPath` en rectangle arrondi enveloppe son contenu, avant de devenir un `svg::Handle` en mémoire.

## Décision

Un rapport unique `icons::APP_ICON_MASK` (dans `src/ui/icons.rs`, propriétaire des icônes ; la tuile de remplacement le prend aussi, à la place de son rayon de 25 %) (0,2237 : le rayon vaut 22,37 % du côté, la courbure des icônes macOS) :

- **SVG d'application** : `mask_svg(source)` insère, juste après la balise racine, `<defs><clipPath id="hyprdesk-masque"><rect … rx ry/></clipPath></defs><g clip-path="url(#hyprdesk-masque)">` et ferme le groupe avant `</svg>` ; le rectangle reprend la `viewBox` (sinon `width` et `height`). Source illisible, compressée (`svgz`), racine auto-fermée ou sans dimensions : l'icône reste telle quelle.
- **Image matricielle d'application** : rendue avec `border_radius(côté × APP_ICON_MASK)` ; ses indices sont gardés dans `app_rasters`.
- **Quelles icônes** : celles chargées comme icônes d'application — classes de fenêtres (`ClassIcons`), icônes du bureau (`icon_imgs`), icônes nommées demandées comme icônes d'application, sous une clé à part (`App::app_icon_key`, préfixe `app:`) : le lanceur demande et pose cette clé, le même nom demandé par une notification ou un module de statut reste sans masque. Les icônes d'interface, symboliques, de statut et de notification ne sont pas masquées.

Une icône déjà ronde (thème circulaire) reste ronde : le masque borne la forme, il ne la remplit pas.

## Réutilisation

- `handle_for`, `IconImg`, `icon_named`, `img_element` ; `image::border_radius` (déjà utilisé par la pochette du média).
- Exemple : la relecture en mémoire des icônes SmartUI dans `handle_for`.

## Changements

1. `src/ui/icons.rs` : `APP_ICON_MASK`.
2. `src/ui/icons.rs` (ou module voisin) : `mask_svg(&str, ratio) -> Option<String>`, pur, testé (viewBox, largeur et hauteur, racine auto-fermée, `>` entre guillemets, sans dimensions).
3. `src/ui/mod.rs` : `handle_for_app` (SVG lu et masqué, matricielle marquée dans `app_rasters`) ; appelée pour `ClassIcons`, `icon_imgs` et les `IconResolved` d'une icône d'application ; `app_rasters` vidé avec `rasters` ; `icon_named` et `img_element` arrondissent une matricielle marquée.
4. `src/ui/preview.rs` : les icônes des scènes suivent le même chargement.
5. Documentation : `DESIGN.md` (masque des icônes d'application), guides.

## Relecture du plan (agent indépendant), prise en compte

- `named_icons` est indexé par nom : « firefox » sert au lanceur et aux bulles. Une clé à part (`app:<nom>`) sépare l'icône d'application du même nom demandé ailleurs ; elle ne dépend plus de l'ordre d'arrivée de la liste des applications.
- Icônes du bureau : seules celles de type `App` sont masquées (pas les fichiers, dossiers ni commandes).
- Repli : la découpe en source ne passe pas par un analyseur ; une source qu'elle rendrait invalide (sélecteurs `<style>` structurels, `<svg` dans un commentaire avant la racine) est un risque accepté, rare dans les thèmes d'icônes ; `svgz` est déjà illisible pour iced et reste tel quel.
- Hors de ce lot : l'assistant (`assistant/services.rs`) et les vignettes du Panorama (`panorama_icon_thumb`) chargent leurs icônes sans `handle_for` : non masquées, à reprendre avec ICO-10.

## Relecture du diff (agent indépendant), prise en compte

- Le préfixe `app:` faussait le test « symbolique » : `want_icon` et `icon_named` jugent désormais sur le nom nu ; une icône symbolique demandée comme icône d'application garde le glyphe de hyprdesk, teinté, sans masque.
- Test d'aiguillage `icones_d_application_au_masque_unique` (SVG découpé, matricielle marquée, même image hors application non marquée, clé d'une icône symbolique).
- Lecture des SVG d'application sur le fil de l'interface : même idiome que la relecture des icônes SmartUI dans `handle_for`, une fois par icône (les poignées sont gardées) ; la passer dans les tâches de résolution reste une amélioration possible, non faite ici.
- `mask_svg` passé (par l'agent, en portage) sur 39 621 SVG d'applications installés : 39 606 découpables, les 15 autres illisibles en UTF-8 restent tels quels ; les `<style>` rencontrés n'ont que des sélecteurs de classe.

## Portée

- Hérite : toutes les surfaces qui posent une icône d'application par `icon_named` ou `img_element`.
- Vérifier : scènes `desktop`, `launcher`, `applications`, `fenetres` ; icônes symboliques inchangées.
- Exclure : barre d'état (tray), notifications, icônes d'interface.

## Validation

- Produit : une icône carrée (SmartUI, Firefox matriciel) a ses coins arrondis dans le dock et le lanceur ; une icône ronde reste ronde.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh` → 0 ; captures des scènes concernées.

## Conditions d'arrêt

- S'arrêter si le masquage en source casse le rendu d'un SVG courant (dégradés, `<style>`) d'une façon que le repli ne couvre pas.

## Documentation

- `DESIGN.md` et guides.
