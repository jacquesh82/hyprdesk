# Les icônes d'application sont cherchées à la taille affichée et à l'échelle de l'écran

Écrit contre : `f982fff` (branche `hig/plan-A`)

## Chaîne de preuve

- Surface : icônes d'application du lanceur (grille 60 px, suggestions 64 px), du dock (réglable de 24 à 96 px), du bureau, du commutateur, de la vue d'ensemble, du menu de fenêtre, du sélecteur d'activité.
- Problème : la recherche dans le thème se fait à taille fixe et sans échelle d'écran. `want_icon` cherche à 32 (`src/ui/mod.rs`, `launch::lookup_icon(&n2, theme, 32)`), `resolve_class_icons` à 48 (`launch::icon_for_class(…, 48)`), les icônes du bureau à la taille de `reload_icons` ; `lookup_icon` appelle `with_size` sans `with_scale` (`src/launch.rs`). Un thème à bitmaps (OpenJDK : 48 px au plus) rend donc une image floue à 60 px, et deux fois floue sur un écran à l'échelle 1,6.
- Règle : images › Resolution — « Provide high-resolution assets for all bitmap images in your app, for every device you support. »
- Propriétaires : `launch::lookup_icon`, `launch::icon_for_class`, `launch::resolve` (recherche) ; `App::want_icon`, `App::resolve_class_icons`, `App::reload_icons` (demandes) ; `self.monitors` (`Monitor.scale`).
- Contrainte : `freedesktop-icons` 0.4 prend une taille entière (`with_size`) et une échelle entière (`with_scale`) ; une échelle fractionnaire (1,6) s'arrondit au-dessus.

## Décision

Chaque recherche d'icône d'application demande **la plus grande taille où elle s'affiche** et **l'échelle entière la plus haute parmi les écrans** :

- `launch::lookup_icon(name, theme, px)` cherche à la **taille en pixels effectifs** `px = ceil(taille affichée × échelle)` par `with_size(px)` seul, **sans `with_scale`** : la crate classe les dossiers par pixels effectifs (`taille × échelle`) et sa distance des dossiers `Scalable` est fausse (`max − demandé`), si bien qu'avec `with_scale(2)` un PNG 48 passerait devant le SVG ; `with_size(px)` donne le dossier matriciel le plus proche de la taille réelle, ou le SVG. `icon_for_class` et `resolve` transmettent `px`.
- `App::icon_scale()` : `ceil(max(Monitor.scale))`, borné à 1..=3 (pur, testé).
- Tailles par usage, en constantes nommées près de leur surface : icônes d'application nommées (lanceur : `max(grille, suggestions)` = 64) ; classes de fenêtres : `max(48, taille du dock × dock.magnify)` (le dock grossit au survol) ; bureau : la taille d'icône du bureau. Les icônes d'interface et de statut gardent leur taille (32, 24) avec l'échelle.
- Relecture ciblée, jamais à chaque `Update::Monitors` (il arrive à chaque changement d'espace) : seulement quand `icon_scale()` change (comparaison des entiers arrondis) ou quand la taille du dock, du lanceur (`grid.icon_size`, `launcher.size`) ou du bureau change ; pour les classes, vider `class_imgs` et relancer `resolve_class_icons` ; pour le lanceur, vider les clés `app:` de `named_icons` ; le bureau seul passe par `reload_icons` (qui vide tout), à son seul changement d'échelle ou de taille.

Les SVG ne changent pas (vectoriels) ; seules les matricielles gagnent en netteté.

## Réutilisation

- `with_size`, `with_scale` de `freedesktop-icons` (déjà dépendance) ; `icons_gen` et `reload_icons` pour la relecture ; `Monitor.scale`.
- Exemple : la relecture des icônes au changement de thème d'icônes.

## Changements

1. `src/launch.rs` : taille en pixels effectifs pour `lookup_icon`, `icon_for_class`, `resolve` ; appelants : `src/ui/mod.rs`, `src/ui/wizard.rs`, `src/ui/preview.rs`, `src/smartui.rs`, `src/panorama/icons.rs`, `src/apps.rs`, `src/ui/search.rs`, `src/ui/plugins/dock/surface.rs`.
2. `src/ui/mod.rs` : `icon_scale()` ; `want_icon` cherche une clé `app:` à la taille du lanceur et à l'échelle ; `resolve_class_icons` à `max(48, dock)` et à l'échelle ; `reload_icons` à la taille du bureau et à l'échelle ; relecture quand `icon_scale()` ou la taille du dock change (`Update::Monitors`, application de la configuration).
3. Tests : `icon_scale` (1, 1,6 → 2, 2, 3,5 → 3) ; tailles demandées par usage.

## Relecture du plan (agent indépendant), prise en compte

- `find()` ne rend jamais rien de vide pour une échelle absente, et sa distance `Scalable` est fausse : plus de `with_scale`, la taille en pixels effectifs seule.
- Trois appelants de `resolve` ajoutés (`src/apps.rs`, `src/ui/search.rs`, `src/ui/plugins/dock/surface.rs`).
- Le grossissement du dock au survol compte dans la taille demandée.
- Relecture ciblée (classes, clés `app:`, bureau séparément), jamais à chaque `Update::Monitors`, échelle comparée arrondie.
- `with_cache` : clé (thème, taille, échelle, nom), rien de périmé ; une absence est recherchée à nouveau (coût connu, inchangé).

## Portée

- Hérite : toutes les surfaces qui posent une icône d'application.
- Vérifier : aucune relecture en boucle (la relecture ne se fait que si l'échelle ou la taille change) ; SmartUI (ses icônes passent avant le thème, inchangées) ; Panorama (128, inchangé).
- Exclure : icônes d'interface du jeu de hyprdesk (vectorielles).

## Validation

- Produit : sur un écran à l'échelle 1,6, une icône d'application à bitmaps est nette au lanceur et au dock.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh` → 0.

## Conditions d'arrêt

- S'arrêter si la recherche à `px` rend, sur les thèmes de la session (Tela-circle, Adwaita, hicolor), une icône moins bonne qu'aujourd'hui (matricielle plus petite là où un SVG existait) : le noter et marquer « à reprendre ».

## Documentation

- Aucune page utilisateur ; une ligne dans `DESIGN.md` (icônes d'application cherchées à la taille affichée et à l'échelle).
