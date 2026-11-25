# Les icônes d'application suivent l'apparence du thème : par défaut, sombre, teintée

Écrit contre : `f982fff` (branche `hig/plan-A`)

## Chaîne de preuve

- Surface : icônes d'application (dock, lanceur, bureau, commutateur, vue d'ensemble), celles qui prennent le masque unique d'ICO-04.
- Problème : le moteur de thème ne choisit que le nom du thème d'icônes (`src/theme_engine/resolve.rs`, champ `icons`) ; aucune variante sombre ni teintée, aucun réglage d'apparence (`src/design_options.rs` n'a pas de clé `icon`).
- Règle : app-icons › Appearances — « Keep your icon's features consistent across appearances » ; app-icons › Specifications (apparences par défaut, sombre, teintée).
- Propriétaires : le chargement des icônes d'application (`App::app_handle_for`, ICO-04), `icons::mask_svg` (découpe en source, ICO-04), `design_options` (réglages déclarés), la variante du thème (claire ou sombre).
- Contrainte : aucune dépendance nouvelle. Les SVG se transforment dans leur source (un filtre `feColorMatrix` injecté, comme le `clipPath` d'ICO-04 : resvg, moteur d'iced, les rend) ; les matricielles se transforment avec `image` 0.25, déjà dépendance, hors du fil de l'interface, en `image::Handle::from_rgba`.

## Décision

Un réglage `icons.appearance` : `auto` (défaut : suit la variante du thème, claire → par défaut, sombre → sombre), `default`, `dark`, `tinted`.

- **Par défaut** : l'icône telle quelle (avec son masque).
- **Sombre** : dérivée de l'icône claire, sans en changer les traits : luminance abaissée et contraste préservé (matrice de couleur fixe, la même pour SVG et matricielles), pour qu'une icône claire ne détonne pas sur un bureau sombre.
- **Teintée** : l'icône en niveaux de gris, puis colorée de l'accent du thème (matrice de couleur dérivée de l'accent), comme les icônes teintées de macOS.

Les icônes d'interface, symboliques, de statut et de notification ne changent pas.

## Réutilisation

- `app_handle_for`, `mask_svg` (ICO-04) ; `design_options` (déclaration, Lua, CLI, Réglages) ; `icons_gen` pour la relecture ; la variante du thème actif.
- Exemple : la découpe en source d'ICO-04.

## Changements

Voir « Étapes ».

## Portée

- Hérite : toutes les icônes d'application masquées.
- Vérifier : SmartUI (ses icônes passent par le même chargement), la tuile de remplacement (teintée : elle prend déjà l'accent ; sombre : inchangée), le coût (transformation une fois par icône, gardée avec sa poignée).
- Exclure : thèmes d'icônes du système (le moteur continue d'en choisir le nom), applications tierces.

## Relecture du plan (agent indépendant), prise en compte

- resvg 0.45.1 (moteur d'iced) rend `feColorMatrix` ; le filtre porte `color-interpolation-filters="sRGB"`, pour que la matrice donne le même résultat sur un SVG et sur une matricielle passée par `image`.
- La variante du thème n'atteint pas l'interface (`Outcome` de `src/ui/engine.rs` ne garde que palette et design) et `on_theme_engine` ne relit pas les icônes : l'étape 1 transporte la variante et relit les icônes d'application quand la variante, l'accent ou le réglage change (`icons_changed` les ignore aujourd'hui).
- Les SVG que `mask_svg` refuse et les `.svgz` ne prendraient pas l'apparence : accepté, noté au guide.
- Les matricielles se chargent sur le fil de l'interface (`app_handle_for`, chemins seulement) : le transport des pixels dans `IconResolved`, `ClassIcons` et `reload_icons` est une étape à part, avant la matrice.
- Le champ des Réglages a des libellés traduits : champ dédié, pas `design_fields` (valeurs brutes).
- Vérifier qu'aucun pack ne pose `[icons] appearance` (table `[icons]` des manifestes) : la clé vit dans `design_options`, pas dans le manifeste.

## Étapes

1. **Réglage et variante** : `icons.appearance` (`auto`, `default`, `dark`, `tinted`) dans `design_options` ; la variante du thème transportée jusqu'à l'interface (`Outcome`, `on_theme_engine`) ; `App::icon_appearance()` résout `auto` ; relecture des icônes d'application quand le réglage, la variante ou l'accent change. Tests : résolution d'`auto`, relecture déclenchée. Aucun changement visible.
2. **SVG** : `icons::appearance_svg(src, appearance, accent)` injecte, avec le masque, un filtre `feColorMatrix` en sRGB (sombre ou teinte) ; tests sur la source ; scènes `icones-sombres`, `icones-teintees`.
3. **Transport des pixels** : les tâches de résolution (`want_icon`, `resolve_class_icons`, `reload_icons`) décodent les matricielles d'application avec `image` hors du fil de l'interface et rendent des RGBA ; poignées `from_rgba`. Aucun changement visible.
4. **Matrice des matricielles** : la même matrice appliquée aux RGBA ; test sur une image de 2 × 2 ; mêmes scènes.
5. **Réglages** : « Apparence des icônes » (Automatique, Par défaut, Sombre, Teintée), champ dédié dans Réglages › Thème ; libellés traduits ; scène `reglages-icones`.
6. **Documentation** : guides (FR, EN), `DESIGN.md` ; l'écart passe `fait`.

État : étape 1 faite (`d5bf48e`) ; prochaine : étape 2.

## Validation

- Produit : en thème sombre, les icônes du dock s'assombrissent sans changer de dessin ; en « Teintée », toutes prennent l'accent.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh` → 0 à chaque étape ; captures des scènes aux étapes 2 à 4.

## Conditions d'arrêt

- S'arrêter si resvg, dans iced, ne rend pas le filtre injecté : marquer l'étape « à reprendre » et le noter.

## Documentation

- Étape 5.
