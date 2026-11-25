# Menus de la barre : un sous-menu d'un niveau, signalé par un chevron et ouvert au survol (HIG MENU-03)

Écrit contre : `d957308` (intégration `hig/plan-d-action`)

## Chaîne de preuve

- Surface : les cartes de menu ouvertes depuis la barre (menus « hyprdesk », « Fenêtre », « Espaces », menus des modules Lua, menus de la zone de notification), tous styles de barre.
- Problème : ni `MenuEntry` (`src/config/model.rs:857-883`) ni `Item` (`src/ui/menu.rs:108-118`) n'ont de sous-menu. L'arbre dbusmenu est aplani en une liste à en-têtes de chemin (« Fichier › Récents »), jusqu'à 256 lignes (`src/tray.rs:485-526`). « Déplacer vers l'espace » est un groupe en ligne arrêté à 10 espaces (`src/ui/plugins/bar/window.rs:115-131`).
- Preuve de design : `03-menus-and-actions.md`, menus › Submenus (« it's generally best to restrict them to a single level » ; « Prefer using a submenu to indenting menu items » ; « Make sure a submenu remains available even when its nested menu items are unavailable »), menus › Anatomie et cotes (« A menu item indicates the presence of a submenu by displaying a symbol — like a chevron — after its label »).
- Propriétaire : `MenuEntry` (`src/config/model.rs`), le composant de menu (`src/ui/menu.rs` : `menu_row`, `RowState`), la carte des menus (`src/ui/plugins/bar/popover.rs` : `Kind::Menu`, `pop_menu`, `pop_content`, `placement`, `refresh_popover_anchor`).
- Portée et surfaces touchées : `src/tray.rs` (`flatten`), `src/ui/plugins/bar/window.rs`, `src/ui/preview.rs` (scène), `card_height` (nouvelle variante).
- Incertitude : écran plus étroit que la carte et sa colonne (moins de 570 px) : la colonne est comprimée à la place disponible. La position de la carte à l'ouverture d'un sous-menu est supposée connue de `Popover` (`left`, `size`, `attached`, `position`) ; à vérifier par les tests de placement et la scène d'aperçu.

## Décision de design

1. **Modèle** : `MenuEntry::Submenu { label, icon, enabled, children }`. Un seul niveau : une `Submenu` dans `children` est ignorée à la lecture (ni construite par aucun appelant).
2. **Composant** : une ligne de sous-menu porte un chevron après son libellé (`go-next-symbolic`, repli « › »), à la couleur du texte de la ligne. `RowState.submenu` le demande ; `menu_row` le dessine. La liste parente, elle, n'a pas de nouvelle primitive.
3. **Ouverture** : dans la carte du menu, la colonne du sous-menu se dessine à côté de la liste parente, dans la même surface : à droite s'il reste de la place jusqu'au bord de l'écran (12 px), sinon à gauche. La carte s'élargit de `SUB_GAP + SUB_W` (8 + 216 px) ; la liste parente garde sa largeur (304 px) et **ne bouge pas** ; la colonne part à la hauteur de sa ligne (somme des hauteurs des entrées précédentes) ; la carte grandit en hauteur quand la colonne dépasse le bas (`fit_popover`, borne `max_height` et défilement existants). La ligne parente reste surlignée tant que sa colonne est ouverte.
   - Survol d'une ligne de sous-menu : ouverte après 180 ms. Survol d'une autre ligne de la liste, ou sortie de la carte : fermée (ou changée) après 300 ms. Entrer dans la colonne annule le changement en attente : un déplacement en diagonale depuis la ligne parente ne referme pas la colonne.
   - Clic sur une ligne de sous-menu : ouverte aussitôt (écran tactile, clavier).
   - Une entrée de la colonne s'active par `PMsg::SubActivate(j)` : jamais par `PMsg::Activate(i)`, dont l'index porte sur la liste parente.
4. **dbusmenu** : un sous-menu du premier niveau reste un sous-menu ; ses propres sous-menus sont aplatis en en-têtes, comme aujourd'hui, mais dans sa colonne. Un sous-menu vide reste masqué (`AboutToShow` le remplit, puis il paraît). Le plafond de 256 entrées compte les deux niveaux.
5. **« Déplacer vers l'espace »** : une entrée de sous-menu, sans plafond : tous les autres espaces normaux. Le trait et le titre de groupe disparaissent.

Cela résout la cause : le composant sait exprimer un niveau de sous-menu, et les deux arbres qui en avaient besoin l'emploient au lieu d'aplatir ou de tronquer.

## Réutilisation

- `menu_row`, `RowState`, `row_text`, `row_background` (`src/ui/menu.rs`) ; exemplaire : `pop_row`, `src/ui/plugins/bar/popover.rs:1472-1488`.
- `pop_menu` (appelée pour la colonne), `pop_content`, `fit_popover` (mesure de la hauteur), `placement` (largeur et position), `card_height` (hauteurs des entrées, extraites dans `entry_height`).
- `mouse_area::on_enter` / `on_exit`, comme `src/ui/plugins/bar/surface.rs:963-964` ; `want_icon`, `icon_named` ; `Fade`, `inert` du changement de contenu.
- Icône `go-next-symbolic`, déjà employée par le centre Continuité (`popover/continuity.rs:744`).

Aucune primitive nouvelle : pas de seconde surface, pas de type de menu parallèle. La colonne est un second appel de `pop_menu`, dans la carte existante.

## Changements

1. `src/config/model.rs`
   - Change : variante `MenuEntry::Submenu { label: String, icon: Option<String>, enabled: bool, children: Vec<MenuEntry> }`.
   - Preserve : les autres variantes, l'ordre des champs, `PartialEq` (les tests Lua comparent des entrées).
   - Verify : `cargo clippy` signale chaque `match` exhaustif à compléter.
2. `src/tray.rs` (`flatten`)
   - Change : au premier niveau (`path` vide), un nœud `submenu` visible, nommé et non vide devient `MenuEntry::Submenu` ; ses enfants sont aplatis dans `children` avec le chemin courant. Plafond de 256 entrées sur les deux niveaux.
   - Preserve : nœuds invisibles, séparateurs, coches, en-têtes des sous-menus imbriqués, nettoyage des traits de fin.
   - Verify : test unitaire (arbre à deux et trois niveaux).
3. `src/ui/plugins/bar/window.rs` (`entries`)
   - Change : `MenuEntry::Submenu { label: "Déplacer vers l'espace", children: tous les autres espaces }` après un trait ; plus de `take(10)`, plus de `Header`.
   - Preserve : actions `MOVE + n`, les cinq premières entrées, « Fermer toutes les fenêtres » et « Fermer » en fin de menu.
   - Verify : test unitaire existant, lu dans la colonne ; douze espaces donnent douze entrées.
4. `src/ui/menu.rs`
   - Change : `RowState.submenu` (faux par défaut) ; `menu_row` ajoute le chevron après le libellé et le détail ; `menu_chevron(ink, st)`.
   - Preserve : largeur et hauteur des lignes, `stacked`, les appelants du clic droit.
   - Verify : les tests du module passent ; scène d'aperçu.
5. `src/ui/plugins/bar/popover.rs`
   - Change : `Popover { sub: Option<usize>, sub_left, sub_width, sub_timer: Option<(jeton, cible)>, sub_gen }` ; `PMsg::MenuHover(Option<usize>)` (le survol d'une ligne : sous-menu ou ligne ordinaire), `PMsg::MenuEnter`, `PMsg::MenuLeave`, `PMsg::MenuTick(u64)`, `PMsg::SubActivate(usize)` ; `widened(screen, left, width, forced)` (position et largeur élargies, côté et largeur de la colonne), employée par `refresh_popover_anchor` à chaque ouverture et à chaque fermeture ; `attached_placement(…, extra)` ; `entry_height` / `entry_top` ; `pop_menu_card` dessine `row![liste, colonne]` (ou l'inverse) dans une seule `mouse_area` ; `menu_action(source, action)` tirée de `PMsg::Confirm`, avec `sub_action` ; `menu_icons` ; `open_popover` referme le sous-menu d'un menu remplacé ; `Msg::MenuRefreshed` referme une colonne dont la ligne a disparu.
   - Preserve : taille et position d'une carte sans sous-menu (identiques), tableaux de bord, cartes attachées, changement de contenu en fondu, fermeture.
   - Verify : tests (`widened` : à droite, à gauche, écran étroit ; `entry_top` ; un survol puis une entrée dans la colonne ; `SubActivate` rend l'action de la colonne, `Activate` reste sur la liste).
6. `src/ui/preview.rs`
   - Change : scène « bar-menu-window » : menu « Fenêtre », « Déplacer vers l'espace » ouvert sur quatre espaces.
   - Verify : `scripts/verify.sh --visual /tmp/hig-visual-C`, capture regardée.
7. Documentation : `docs/GUIDE.fr.md`, `docs/GUIDE.md` (menus de la barre : sous-menus au survol) ; `DESIGN.md` (règle du sous-menu) ; `03-menus-and-actions.md` ligne 586 (bornes dbusmenu) ; `locales/en.toml` si un libellé nouveau.

## Portée

- Héritent : tous les menus ouverts depuis la barre (`Kind::Menu`), tous styles ; rien ne change pour un menu sans sous-menu.
- À vérifier : menus de la zone de notification (arbre réel), menus Lua et du shell (aucun sous-menu : rendu inchangé), cartes attachées du style Convexe, barre en bas.
- Exclus : les menus du clic droit (`Line`, `Item` : aucun appelant n'en demande ; MENU-02 les traitera) ; les sous-menus déclarés en Lua (`{ menu = { … } }` : aucun plugin ne le fait) ; le défilement et le plafond des menus trop longs (MENU-02) ; la navigation au clavier par flèches dans les cartes (ACC).

## Validation

- Produit : menu « Fenêtre » d'une fenêtre sur l'espace 2 avec cinq espaces : « Déplacer vers l'espace › » ; au survol, la colonne liste « Espace 1 », « Espace 3 », « Espace 4 », « Espace 5 » ; un clic déplace la fenêtre et ferme la carte.
- Interface : colonne à droite, puis à gauche pour un menu ouvert près du bord droit ; colonne sous la hauteur de la carte (beaucoup d'espaces) ; sous-menu vide masqué ; entrée indisponible dans la colonne, grisée ; `transition = "none"`.
- Système : une seule façon de dessiner une ligne (le composant), aucune seconde surface.
- Dépôt : `CARGO_BUILD_JOBS=6 bash scripts/verify.sh` → code de sortie 0 ; `scripts/verify.sh --visual /tmp/hig-visual-C` → la scène « bar-menu-window » se regarde.

## Conditions d'arrêt

- Arrêter si la carte ne peut pas s'élargir sans déplacer la liste parente sur la barre du haut (style Continuité, configuration de la session) : le choix de la surface unique serait faux, à revoir avant d'aller plus loin.
- Arrêter si `fit_popover` ne suit pas la colonne : la hauteur de la carte serait fausse.

## Documentation de design

- Fait : `DESIGN.md`, règle des menus : « un niveau de sous-menu, chevron après le libellé, ouvert au survol, en colonne à côté de la liste » ; `docs/GUIDE.fr.md` et `docs/GUIDE.md` (menus de la barre). L'inventaire de l'audit dans `03-menus-and-actions.md` reste tel quel : il décrit l'état avant traitement.

## Réalisation

Écart du travail repris (`reprise-C.patch`) : le plan et les points 1 à 4 étaient écrits, la carte (point 5) restait à faire.

Écarts au plan, et pourquoi :

- Le délai est porté par un jeton (`sub_gen`) et une pause née dans la tâche ; une échéance `Instant` n'apportait rien, et une pause créée hors du réacteur faisait échouer les tests.
- Tiroirs attachés (style Convexe) : le tiroir s'élargit vers l'intérieur de l'écran ; celui du bord droit reçoit sa colonne à gauche ; ceux du haut et du bas restent centrés comme un tout, la liste glisse donc d'une demi-colonne le temps de l'ouverture. Leur largeur n'était propagée que par la marge : un changement de taille accompagne désormais.
- Sans ancre relevée (tout premier démarrage), la colonne attend le premier relevé, qui recalcule la carte.
- Une entrée à confirmation dans une colonne reste une ligne d'information : la confirmation ne s'ouvre que dans la liste parente. Aucun appelant n'en produit.
- Zone de notification : plafond de 256 entrées atteint, la suite du menu est coupée, comme celle d'une liste aplatie ; une colonne vide n'est pas affichée.
- Scène « bar-menu-window » ajoutée à la liste des captures de `scripts/verify.sh`.

Lecture du plan, à froid (pas d'agent indépendant disponible) :

- Qu'est-ce qui casse ? La carte est une surface à taille fixe : `fit_popover` ne suivait que la hauteur, d'où le recalcul de l'ancre à chaque ouverture et fermeture ; les tiroirs attachés ne propageaient pas la largeur.
- Que manque-t-il ? Les icônes d'une colonne n'étaient pas résolues à l'ouverture (`menu_icons`) ; un menu de la zone de notification est relu toutes les deux secondes, le rang de la colonne ouverte pouvait ne plus être un sous-menu (refermée).
- Le plan résout-il l'écart ? Oui : le composant a un niveau de sous-menu, signalé par un chevron et ouvert au survol ; l'arbre dbusmenu et « Déplacer vers l'espace » l'emploient au lieu d'aplatir ou de tronquer.

Relecture du diff, à froid (pas d'agent indépendant disponible) :

- Le diff fait-il exactement ce que l'écart demande ? Oui : un sous-menu d'un niveau au composant (chevron après le libellé, ouverture au survol), employé par l'arbre dbusmenu et par « Déplacer vers l'espace ». Au-delà de la colonne du composant : la propagation de la largeur des tiroirs attachés, qu'elle exige.
- Respecte-t-il `DESIGN.md` et les conventions du dépôt ? Oui : une seule façon de dessiner une ligne (`menu_row`), pas de seconde surface, accent réservé au survol et à la ligne ouverte, pas de police nouvelle ; les délais de survol sont des délais d'usage, non des transitions, et `transition = "none"` ne les touche pas ; français source, aucun libellé nouveau (`locales/en.toml` inchangé).
- Régression, test manquant ? Un défaut trouvé et corrigé : une ligne ordinaire survolée avant la fin du délai d'ouverture laissait la colonne s'ouvrir ; test ajouté. Les captures (1440 et 640 px) montrent la liste en place, la colonne à la hauteur de sa ligne, la carte grandie pour la recevoir.
