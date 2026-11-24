# ICO-12 — La graisse des glyphes suit celle du texte voisin

Écrit contre : `05d5ec4` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : tous les glyphes du jeu de hyprdesk (`src/ui/icons.rs`), rendus par `App::icon_named` (`src/ui/mod.rs`) et `icons::glyph`.
- Problème : revérifié à HEAD. L'enveloppe SVG de `icons::get` écrit un seul `stroke-width="1.7"` pour tout le jeu ; le texte voisin emploie plusieurs graisses (`ui_font(Weight::Medium)` des lignes de confirmation, `title_font` des titres en semibold, `page_title_font` en bold, rôle `label` de la barre). Un glyphe à côté d'un titre semibold paraît maigre.
- Preuve de design : HIG icons › Best practices — « In general, match the weights of interface icons and adjacent text. » ; sf-symbols › Weights and scales.
- Propriétaire : `src/ui/icons.rs` (`get`), `App::want_icon` et `App::icon_named` (`src/ui/mod.rs`, cache `named_icons` / `vectors` par nom).
- Incertitude : correspondance graisse de police → épaisseur de trait à régler aux captures ; l'atlas des SVG grandit d'une entrée par couple (nom, graisse) réellement demandé.

## Décision de design

Le trait d'un glyphe dérive de la graisse du texte voisin, sur une table unique : Light 1,5 ; Normal 1,7 (inchangé, la grande majorité des usages) ; Medium 1,85 ; Semibold 2,0 ; Bold et plus 2,2. Les glyphes qui accompagnent un texte régulier ne changent pas ; ceux qui accompagnent un titre ou un libellé plus gras prennent son trait. Les tracés à `stroke-width` propre (éclair de la batterie) gardent le leur.

## Réutilisation

- `tokens::weight` (graisse du thème), `App::title_font`, `ui_font`.
- `icons::get` et son enveloppe ; le cache d'icônes vectorielles de `App`.
- Exemple : `tokens::glyph` (ICO-13) pour la taille, même principe pour la graisse.

## Étapes

1. `icons.rs` : `pub fn stroke(weight: Weight) -> f32` (table ci-dessus) ; `get_weighted(name, weight)` écrit `stroke-width` d'après elle ; `get(name)` = `get_weighted(name, Normal)`. Test : `get` inchangé octet pour octet ; Semibold écrit `2`.
2. `mod.rs` : `want_icon` et `icon_named_weighted(name, size, tint, weight)` résolvent et mettent en cache la variante sous la clé `nom@graisse` (Normal garde la clé nue, rien ne change pour les appels existants). Test : deux graisses, deux entrées ; Normal partage l'entrée existante.
3. Appels : glyphes voisins d'un texte non régulier — en-têtes de section à icône des Réglages (`title_font`), lignes de confirmation des cartes de la barre (`Medium`), libellés de la barre (rôle `label` du thème), titres du tiroir. Relever les appels par `grep` de `title_font()`/`ui_font(` à côté d'un `icon_named` dans la même rangée. Captures `reglages`, `bar-menu-*`, `bar-controls` à 640 et 1440.
4. `DESIGN.md` (icônes) : la table graisse → trait.

## Portée

- Hérite : tout glyphe du jeu affiché à côté d'un texte.
- Exclure : icônes d'applications et du thème (pas de trait), taille des glyphes (ICO-13, fait).

## Validation

- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh --visual /tmp/hig-visual-D` → 0 ; captures regardées.

## Conditions d'arrêt

- S'arrêter si le cache par (nom, graisse) fait recréer des poignées SVG à chaque image : la variante doit être résolue une fois, comme les autres icônes.

## Documentation

- `DESIGN.md` : table graisse → trait (étape 4).
