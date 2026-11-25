# TYP-08 — Le tiroir des Réglages suit le texte agrandi

Écrit contre : `06a2180` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : tiroir des Réglages, texte de l'utilisateur agrandi (Réglages → Thème → Accessibilité, de 75 % à 200 %).
- Problème : à 200 % (aperçu `HYPRDESK_PREVIEW_TEXT_SCALE=2`, fenêtre de 1440 px), le titre du paquet passe sous le champ de recherche (« Performance », « Notifications ») ; dans la navigation, les libellés passent sous leurs compteurs (« Desktop 4 », « Bar 2 ») ; dans Son, la valeur du curseur se coupe (« 45 » / « % ») ; dans Layouts, les listes « Par défaut » recouvrent « Workspace 1 ».
- Preuve de design : HIG layout › Adaptability — « Be prepared for text-size changes. » ; « horizontally adjacent views may need to stack vertically to provide more space for text » ; « table rows or other containers may need to grow in height so that text isn't cropped or doesn't overlap other content ».
- Revérification à HEAD : la preuve du plan, relevée à `00f6d69`, est inexacte sur deux points. 1) Les 45 `Wrapping::None` explicites de `src/ui` portent des nombres (valeurs de curseur, compteurs) ou des surfaces du Panorama et de la barre : aucun libellé ni aucune description de réglage. Sous Iced 0.14, un `text` enveloppe déjà par mots (`Wrapping::Word` est le défaut) : libellés et descriptions passent à la ligne à 200 % ; seul un mot plus long que la ligne déborde. 2) Les listes à largeur fixe (170 à 320 px) sont bornées par leur conteneur et ne débordent pas. L'écart est réel autrement : des largeurs et des seuils écrits pour du texte à 100 % ne suivent pas l'échelle — seuil de l'en-tête, largeur de la navigation, case de 110 px du libellé « Workspace n » (Layouts, Workspaces), case de 48 px de la valeur du son.
- Propriétaires : `App::marked_with` et `App::field` (`src/ui/panel.rs`), par lesquels passent tous les libellés et titres de réglage ; l'en-tête et la navigation du tiroir (`src/ui/panel.rs`) ; `App::small` et l'origine d'une option (`src/ui/hyprset.rs`) ; `tokens::ts` (`src/ui/tokens.rs`), qui porte l'échelle.
- Périmètre et surfaces touchées : `src/ui/tokens.rs`, `src/ui/panel.rs`, `src/ui/hyprset.rs`, `src/ui/plugins/sound.rs`, `src/ui/plugins/layouts.rs`, `src/ui/plugins/workspaces.rs`, `src/ui/preview.rs`, `DESIGN.md`.
- Incertitude : la navigation du tiroir s'élargit avec le texte ; la colonne des réglages y perd autant (166 px à 200 %), d'où son plafond (360 px gardés à la colonne). Les barres d'outils de la page Raccourcis (bascule de mode, filtres) restent hors de la correction, notées au registre.

## Décision de design

- Au repos (100 %), rien ne change : les captures d'avant et d'après sont identiques, hors horloge, curseur de saisie et survols.
- Libellés et titres de réglage enveloppent par mots, puis par signes (`WordOrGlyph`) : un mot plus long que la ligne se coupe et la rangée grandit en hauteur au lieu d'être rogné.
- Ce qui tient côte à côte à 100 % passe à la ligne quand le texte agrandi ne le permet plus : l'avis « surchargé dans hyprdesk.lua » passe sous son libellé ; la ligne des actions de l'en-tête passe sous le titre du paquet (le repli déjà prévu pour le tiroir étroit).
- Les largeurs et seuils écrits pour du texte suivent le texte, par `tokens::text_growth()` : largeur de la navigation, seuil de l'en-tête, case de la valeur du son, case du libellé « Workspace n » et seuil des deux colonnes de Layouts et Workspaces.

Pourquoi cela résout la racine : la disposition du tiroir ne dépend plus de tailles figées pour un texte de 100 % ; ce que le texte agrandi ne tient plus passe à la ligne, et les rangées grandissent en hauteur.

## Réutilisation

- `tokens::ts` : `text_growth()` en est le rapport à la taille de référence du corps (`design::REF_BODY`).
- `Row::wrap` d'Iced, déjà employé par `subtab_bar` (`src/ui/panel.rs`) pour les onglets en pastilles.
- Le repli « deux lignes » de l'en-tête (`one_line`, `src/ui/panel.rs`), déjà en place pour le tiroir étroit.
- `detail_width()` (`src/ui/panel.rs`), dont Layouts et Workspaces tirent déjà leur nombre de colonnes.

Pas de primitive nouvelle.

## Changements

1. `src/ui/tokens.rs`
   - Changement : `pub fn text_growth() -> f32` — croissance du corps depuis sa taille de référence (rôle du thème, écran, réglage de l'utilisateur).
   - Préserver : `ts`, `ts_label`, `ts_metric`, le plancher.
   - Vérifier : test unitaire (100 %, 200 %).
2. `src/ui/panel.rs`
   - Changement : `marked_with` enveloppe en `WordOrGlyph` (texte plein et texte enrichi) ; la tête de `field` est une rangée qui replie ; `nav_width(largeur, croissance)` suit le texte et sert à `detail_width()` ; `header_one_line(largeur, croissance)` suit le texte.
   - Préserver : le rendu à 100 %, libellés de la navigation compris (leur enveloppe par défaut ne change pas).
   - Vérifier : test unitaire de `nav_width` et `header_one_line` ; captures à 100 % identiques, captures à 200 %.
3. `src/ui/hyprset.rs`
   - Changement : l'origine (« hyprland.lua, ligne n », « valeur par défaut », « réglé ici »), les légendes (`small`) et le message d'erreur enveloppent en `WordOrGlyph`.
   - Préserver : largeur des listes et des saisies, messages, sélecteur de couleur.
   - Vérifier : scène `animations` à 100 % inchangée.
4. `src/ui/plugins/sound.rs`, `src/ui/plugins/layouts.rs`, `src/ui/plugins/workspaces.rs`
   - Changement : la case de la valeur du son suit le texte et garde « 45 % » sur une ligne ; la case du libellé « Workspace n » et le seuil des deux colonnes suivent le texte ; le libellé enveloppe en `WordOrGlyph`.
   - Préserver : alignement des valeurs à 100 % (48 px), deux colonnes à 100 %.
   - Vérifier : captures `son`, `layouts`, `workspaces` à 100 % et 200 %.
5. `src/ui/preview.rs`
   - Changement : `HYPRDESK_PREVIEW_TEXT_SCALE=2` pose l'échelle de texte de l'utilisateur, posée aux jetons comme le fait le démon.
   - Préserver : sans la variable, 100 %.
   - Vérifier : captures à 200 %.
6. `DESIGN.md`
   - Changement : une ligne sur le tiroir et le texte agrandi, une sur `HYPRDESK_PREVIEW_TEXT_SCALE`.

## Portée

- Héritent : tous les paquets et onglets des Réglages (libellés, titres de section et de divulgation, navigation, en-tête), les options de Hyprland.
- À vérifier : les deux pages qui tirent leurs colonnes de `detail_width()`.
- Exclut : les barres d'outils de la page Raccourcis (`plugins/keys.rs` : bascule de mode, filtres, rangées de raccourcis), les cartes de thèmes et de profil, le Panorama et la barre — hors des lignes de réglage.

## Validation

- Produit : à 200 %, le titre du paquet, la navigation, les valeurs du son et les libellés « Workspace n » ne sont plus recouverts ; à 100 %, aucune scène ne change.
- Interface : scènes `layouts`, `animations`, `workspaces`, `son`, `performance`, `reglages-notifications`, `clavier-souris`, à 1440 px, 100 % et 200 % ; 640 px à 200 %.
- Système : un seul rapport de croissance (`tokens::text_growth`), une seule rangée qui replie pour la tête de `field`.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh` → code de sortie 0.

## Conditions d'arrêt

- Arrêter si la navigation élargie vide la colonne des réglages sous 400 px à 200 % sur 1440 px, ou si une scène à 100 % change.

## Relecture à froid (sans outil d'agent)

Pas d'agent indépendant disponible.

### Lecture du plan

Relu après le relevé des sites (`detail_width`, ses deux usages ; `166` et `46`, écrits deux fois dans `panel.rs` ; `REF_BODY`).

- **Qu'est-ce qui casse ?** La navigation élargie pourrait vider la colonne des réglages sur un tiroir étroit : sa largeur est bornée pour laisser 360 px à la colonne (`nav_width`, testée). `Row::wrap` change la tête de `field` en un widget `Wrapping` : même largeur au repos quand les deux éléments tiennent.
- **Que manque-t-il ?** Des tests sur ce qui est mesurable (`text_growth`, `nav_width`, `header_one_line`) ; la façon de rendre à 200 % dans l'aperçu, d'où `HYPRDESK_PREVIEW_TEXT_SCALE`. Les guides ne parlent pas de la taille du texte : rien à y écrire.
- **Le plan résout-il l'écart ?** Oui pour ce que demande la correction — libellés et descriptions de réglage enveloppés, rangées qui grandissent en hauteur — et pour les recouvrements vus à 200 % (en-tête, navigation, Son, Layouts). Non pour la page Raccourcis et les cartes : hors des lignes de réglage, notés au registre.

### Relecture du diff

Relu à froid sur `git diff HEAD`, puis sur les captures à 100 % et 200 %, avant et après (`git stash` pour les captures d'avant). Trois défauts trouvés et corrigés :

1. Les libellés de la navigation en `WordOrGlyph` coupaient « Performance » en deux lignes à 100 % (le mot débordait de peu sous son compteur, sans se voir). Retiré : la navigation élargie suffit, leur enveloppe ne change pas.
2. Les listes rendues « souples, plafonnées » par un conteneur `max_width` n'étaient pas plafonnées directement dans une rangée (la troisième passe d'Iced donne à un élément souple une largeur minimale égale à la place restante, et `max_width` ne descend pas sous le minimum) : les listes de Layouts s'élargissaient à 100 %. Retiré partout : une liste à largeur fixe est déjà bornée par son conteneur (`Limits::width(Fixed)` la ramène à la place disponible).
3. `plugins/workspaces.rs` portait la même rangée que Layouts (libellé de 110 px, liste de 170 px) : même correction.

Après correction : le diff fait ce que demande l'écart et ce que la règle de la HIG citée impose (rien de rogné, rien de recouvert), sans toucher au rendu à 100 % ; il respecte `DESIGN.md` (aucun style, aucune couleur, aucune primitive nouvelle ; commentaires en français, référence à l'écart) et les conventions du dépôt ; les tests couvrent `text_growth`, `nav_width` et `header_one_line`, le reste se voit sur les captures.

## Documentation de design

- Après acceptation : une ligne dans `DESIGN.md` (tiroir des Réglages et texte agrandi) et une dans sa section « Vérification visuelle » (`HYPRDESK_PREVIEW_TEXT_SCALE`).
