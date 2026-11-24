# TYP-04 — Les corps de texte suivent l'échelle de la HIG

Écrit contre : `a237df3` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : tout le texte de l'interface (`src/ui`), dont la taille s'écrit `.size(tokens::ts(X))`.
- Problème : revérifié à HEAD, `src/ui` écrit 23 tailles littérales distinctes par `ts()`. Hors de l'échelle de la HIG : 11,5 (2), 12,5 (7), 13,5 (3), 14 (121), 16 (24), 18 (6), 19 (1), 20 (7), 24 (3), 25 (1), 27 (1), 28 (2), 32 (1), 36 (1), 96 (1). La preuve du plan est exacte, à deux précisions près : 7 n'est pas un corps (six marges de `src/ui/toasts.rs`) ; une partie des 14 et 16 ne sont pas du texte (taille d'un glyphe d'icône `icons::glyph(…, ts(14.0), …)`, taille de la pastille d'un `radio`).
- Preuve de design : HIG typography › Specifications, tableau macOS : Large Title 26, Title 1 22, Title 2 17, Title 3 15, Headline 13, Body 13, Callout 12, Subheadline 11, Footnote 10, Caption 1 10, Caption 2 10. « Consider using the built-in text styles. » Une unité de la HIG vaut un pixel logique.
- Propriétaire : `tokens::ts` (`src/ui/tokens.rs`), qui déduit le rôle du thème (légende, corps, titre, affichage) de la taille écrite.
- Périmètre et surfaces touchées : les appels `.size(ts(X))` de `src/ui` dont le widget est un texte (`text`, `text_input`, `text_editor`, `rich_text`, `span`) ; les tailles passées en argument à un assistant qui les rend par `ts` (`SliderLook::fixed` et `fill`, `dash_label` de la carte Continuité, `marked` et `marked_with` du tiroir, `apps_field` du lanceur) ; les `.size(X * échelle)` des modules du Panorama ; `tokens::ts` (seuil du rôle titre) ; `DESIGN.md`.
- Incertitude : 14 est à égale distance de 13 et 15 ; le choix (13) est argumenté plus bas. Les écarts d'un pixel changent légèrement la largeur de certains libellés : les captures le vérifient.

## Décision de design

Une seule échelle de corps, celle de la HIG : 10, 11, 12, 13, 15, 17, 22, 26 px. Chaque taille hors échelle va au corps le plus proche ; à égalité, au plus petit, qui est le style de base de son niveau (13 est le corps de macOS, Body et Headline) :

| Écrit | Devient | Style HIG |
| --- | --- | --- |
| 11,5 | 11 | Subheadline |
| 12,5 | 12 | Callout |
| 13,5, 14 | 13 | Body, Headline |
| 16 | 15 | Title 3 |
| 18, 19 | 17 | Title 2 |
| 20, 24 | 22 | Title 1 |
| 25, 27, 28 | 26 | Large Title |

Exceptions d'affichage, gardées et nommées : 32 (le nom « LYRA » en tête de la discussion, un logotype), 36 (l'horloge de la niche Convex), 96 (le chiffre d'identification d'un écran). Ce ne sont pas des styles de texte courant mais des affichages, comme l'horloge de l'écran verrouillé de macOS ; la HIG n'en fixe pas la taille.

Le rôle du thème suit l'échelle : `ts` lit 15 comme un titre (Title 3) et non plus comme un corps — seuil du rôle titre ramené de 15,5 à 14,5. Sans typographie dans le pack, rien ne change à ce seuil ; avec un pack, les 15 px (anciens 15 et 16) prennent le rapport du rôle titre, ce qu'ils sont.

Ne changent pas : la taille des glyphes d'icônes (`icons::glyph`), celle des pastilles de `radio`, `checkbox`, `toggler`, les marges écrites en `ts(7.0)`, et les programmes à part (`src/lock`, greeter), hors de `src/ui`.

Relecture du plan par un agent indépendant (lecture seule) : aucun site mal classé par l'heuristique ; il manquait une trentaine de tailles passées en argument (27, 21, 20, 18, 16, 14) et quatre `.size(X * t)` du Panorama, désormais dans le périmètre et dans le test ; 23 sites à 15 px et trois à 24 → 22 changent de rôle (corps → titre, affichage → titre), invisible sans typographie dans le pack, le dépôt n'en livre aucune ; les glyphes d'icônes voisins (14, 16) gardent leur taille, une icône n'est pas un corps ; `verify.sh --visual` ne vérifie que l'existence des captures : il faut les regarder.

Relecture du diff par un second agent indépendant : rien de changé hors du texte ; deux restes corrigés, `launcher_field(…, 20.0, …)` des modes du lanceur (`src/ui/search.rs`, passés à 22 comme le Spotlight) et le corps Markdown des réponses de LYRA (`markdown::Settings::with_text_size(14)`, `src/ui/chat.rs`, passé à 13 comme la bulle) ; le test lit aussi `launcher_field`, `with_text_size` et `selectable`. Reste connu : les pastilles `radio(…).size(ts(15.0))` de `src/ui/plugins/bar.rs` prennent le rapport du rôle titre avec un pack typographique (aucun n'est livré) ; le test lit ligne à ligne et ne voit ni un `.size(` coupé par un retour à la ligne ni `ts_label`, `ts_metric`, `ts_tooltip`, tous dans l'échelle aujourd'hui.

Pourquoi cela résout la racine : l'interface n'écrit plus que des corps de la HIG ; un test empêche d'en réintroduire d'autres.

## Réutilisation

- `tokens::ts` et ses rôles ; aucun nouveau jeton.
- Exemplaire du test qui lit les sources : `marked()` de `src/i18n.rs` (catalogue des textes marqués).

Pas de primitive nouvelle.

## Changements

1. `src/ui/**/*.rs` (environ 155 appels, une quarantaine de fichiers)
   - Changement : remplacer la valeur littérale de `.size(ts(X))` selon le tableau, quand le widget le plus proche en amont est un texte.
   - Préserver : `radio`, `checkbox`, `toggler`, glyphes d'icônes, marges ; les tailles déjà dans l'échelle ; les trois exceptions d'affichage.
   - Vérifier : `grep` des valeurs hors échelle dans `.size(ts(…))` ne trouve plus que les pastilles de `radio` et les trois exceptions.
2. `src/ui/tokens.rs`
   - Changement : seuil du rôle titre à 14,5 dans `ts` ; doc de `ts` citant l'échelle ; test `corps_dans_l_echelle_de_la_hig` qui parcourt `src/ui`, relève chaque `.size(ts(X))` et `.size(X * échelle)` littéral dont le widget est un texte, et la taille littérale passée aux assistants (`SliderLook`, `dash_label`, `marked`, `marked_with`, `apps_field`), et échoue sur une valeur hors de l'échelle et des exceptions ; assertion que `ts(15.0)` prend le rôle titre.
   - Préserver : `ts_label`, `ts_metric`, `ts_tooltip`, le plancher `MIN_TEXT`, l'échelle de l'écran et de l'utilisateur.
   - Vérifier : `cargo test` vert.
3. Assistants et Panorama : `SliderLook::fixed/fill(14.0, …)` → 13 ; `dash_label` 27 → 26, 21 → 22, 14 → 13 ; `marked`/`marked_with` 14 → 13, 16 → 15 ; `apps_field` 18 → 17, 20 → 22, 16 → 15 ; `.size(16.0 * t)` → 15, `.size(14.0 * t)` → 13.
4. `DESIGN.md`
   - Changement : la ligne Typographie énonce l'échelle des corps et ses trois exceptions d'affichage.

## Périmètre

- Hérite : tout texte de `src/ui` écrit à une taille hors échelle.
- Vérifier : tiroir des Réglages (titre de page 27 → 26), dialogues (titres 20 → 22 : problème, polkit, conflit de raccourcis, myMFA), menus (11,5 et 13,5), cartes de la barre (12,5 et 13,5), assistant de premier démarrage (18, 20, 24), Panorama.
- Exclure : glyphes d'icônes, contrôles, marges ; `src/lock` et les binaires dédiés ; l'unification des deux corps de base 12 et 13, tous deux dans l'échelle (Callout et Body).

## Validation

- Produit : l'interface se lit comme avant, à un pixel près.
- Interface : `scripts/verify.sh --visual` ; regarder `reglages`, `bar-menu-shell`, `menu-fenetre`, `polkit`, `probleme`, `tour`, `assistant`, `notifications`, `bar-controls`, à 1440, 900 et 640 px : aucun libellé coupé ni débordé.
- Système : aucune primitive nouvelle ; `ts` reste le seul propriétaire de la taille.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh --visual /tmp/hig-visual-D` → code de sortie 0.

## Conditions d'arrêt

- S'arrêter si un passage de 20 à 22 ou de 16 à 15 coupe un libellé dans une capture : garder alors le corps inférieur de l'échelle pour ce site et le noter.
- S'arrêter si le changement de seuil fait échouer un test de thème existant pour une raison autre que le seuil lui-même.

## Documentation de design

- `DESIGN.md`, ligne Typographie : « Corps de l'échelle de la HIG seulement : 10, 11, 12, 13, 15, 17, 22, 26 px ; trois affichages hors échelle (logotype LYRA 32, horloge Convex 36, chiffre d'identification d'un écran 96). »
