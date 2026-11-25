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

## Relecture indépendante (prise en compte)

- La clé `nom@graisse` aurait cassé `want_icon` (suffixe `-symbolic` perdu, recherche partie dans le thème) et les captures (variantes non préchargées) : abandonnée. Le `svg::Handle` est identifié par le hachage de ses octets ; la variante se construit au rendu, comme `icons::glyph`, et iced ne la rastérise qu'une fois. La condition d'arrêt est donc tenue sans cache propre.
- `icons::glyph` reçoit aussi sa variante, `glyph_weighted`.
- Le rôle `label` de la barre est une taille ; la graisse des libellés est `Medium`, désormais la constante `LABEL_WEIGHT` de `bar/surface.rs`.

## Étapes

Toutes faites (itération D suivante) :

1. `icons.rs` : `pub fn stroke(weight: Weight) -> f32` (table ci-dessus) ; `get_weighted(name, weight)` écrit `stroke-width` d'après elle ; `get(name)` = `get_weighted(name, Normal)`. Test : `get` inchangé octet pour octet ; Semibold écrit `2`.
2. `mod.rs` : `icon_named_weighted(name, size, tint, weight)` ; un glyphe du jeu résolu par `want_icon` (test commun `builtin_glyph`) prend `glyph_weighted`, une icône du thème reste telle quelle. Aucune entrée de cache ajoutée. Test : la variante n'ajoute ni entrée ni poignée ; un nom non résolu reste absent.
3. Appels : glyphes voisins d'un texte non régulier — en-têtes de section à icône des Réglages (`title_font`), lignes de confirmation des cartes de la barre (`Medium`), libellés de la barre (rôle `label` du thème), titres du tiroir. Réalisé : volume et luminosité de la barre (Medium), chevron du titre de fenêtre (Semibold), réglage à curseur des cartes de la barre (Medium), orbes des contrôles (Medium), Retirer des cartes de `hyprlists` et `curves` (`title_font`), entrées de la barre latérale du tiroir des Réglages (libellés en Medium, `panel.rs`), module média et modules Lua à icône et texte de la barre (`LABEL_WEIGHT`). Captures `reglages`, `bar-menu-*`, `bar-controls` à 640 et 1440.
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
