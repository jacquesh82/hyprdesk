# ICO-06 — Les menus et le Panorama prennent tous leurs glyphes au jeu de hyprdesk

Écrit contre : `d5639ac` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : menus du bureau, des icônes et des fichiers (`src/ui/context.rs`), menu Fenêtre de la barre (`src/ui/plugins/bar/window.rs`), commandes du Panorama (`src/panorama/design.rs`).
- Problème : revérifié à HEAD. `icons::get` ne dessine que les noms de son jeu ; les autres passent au thème d'icônes (`want_icon`). Relevé par script des noms `…-symbolic` hors code de test de ces trois fichiers : 21 noms sans glyphe — `audio-x-generic`, `document-open`, `document-properties`, `edit-copy`, `edit-cut`, `edit-select-all`, `focus-windows`, `go-jump`, `list-remove`, `mail-send`, `media-seek-backward`, `media-seek-forward`, `phone`, `preferences-desktop-wallpaper`, `send-to`, `video-x-generic`, `view-fullscreen`, `view-pin`, `window-maximize`, `window-minimize`, `window-new`. Un même menu mêle donc le trait de hyprdesk et celui du thème. Le test `interface_entierement_dessinee` ne lit que quatre fichiers (barre, popover, widgets, OSD). (`applications-science` n'apparaît que dans un test de `context.rs` : c'est l'icône d'une action fournie par une application, hors jeu par nature.)
- Preuve de design : HIG icons › Best practices — « Maintain visual consistency across all interface icons in your app. » ; sf-symbols › Custom symbols — cohérence de détail, de graisse optique, d'alignement.
- Propriétaire : `src/ui/icons.rs` (`get`, grille 24 px, trait 1,7, bouts arrondis).
- Incertitude : la lisibilité des nouveaux glyphes se vérifie aux captures (`menu-fenetre`, `proprietes`, `menu-icone`, Panorama).

## Décision de design

Dessiner les 21 glyphes dans `icons::get` (19 dessins, 2 alias), sur la grille et le trait du jeu (24 px, 1,7, bouts et jointures arrondis, rayons 2 à 3 comme les rectangles existants), en réemployant les formes déjà présentes : rectangle de fenêtre arrondi (`window-maximize`, `window-new`), triangles de `media-skip-*` (`media-seek-*`), dossier de `folder` (`document-open`), trait de `list-add` (`list-remove`). Étendre le test à `context.rs`, `bar/window.rs` et `panorama/design.rs`, en ignorant le code de test des fichiers lus (après `#[cfg(test)]`), où figurent des noms d'applications.

## Réutilisation

- `icons::get` et son enveloppe SVG ; `used_names` et `interface_entierement_dessinee`.
- Exemples : `list-add`, `media-skip-forward`, `folder`, `edit-paste`, `user-trash`.

Aucune primitive nouvelle.

## Changements

1. `src/ui/icons.rs`
   - Change : 21 nouveaux bras de `match` (un glyphe par nom, alias `|` quand deux noms partagent un dessin).
   - Preserve : les glyphes existants, l'ordre de résolution (jeu, puis thème pour les icônes d'applications).
   - Verify : `interface_entierement_dessinee` étendu aux trois fichiers passe ; captures.
2. `src/ui/icons.rs` (tests) : `used_names` lit aussi `src/ui/context.rs`, `src/ui/plugins/bar/window.rs`, `src/panorama/design.rs`, coupe chaque fichier à son `#[cfg(test)]`, et ignore les gabarits de `format!` (un nom qui contient `{` : `action_icon` de `context.rs` compose le nom de l'icône d'une action d'application).

## Portée

- Hérite : tous les menus et commandes qui nomment ces glyphes.
- Vérifier : scènes `menu-fenetre`, `menu-icone`, `proprietes`, `menu-dock-long`, `bureau` et le Panorama.
- Exclure : les autres surfaces que l'écart ne cite pas (bulles, Panorama `surface/*`, dock, Réglages) et les noms sans suffixe `-symbolic` que `want_icon` envoie au thème ; les icônes d'actions fournies par les applications (fichiers `.desktop`), les icônes d'applications, la graisse et la taille des glyphes (ICO-12, ICO-13).

## Validation

- Produit : ouvrir le menu d'une icône du bureau et le menu Fenêtre : un seul trait, une seule graisse.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh --visual /tmp/hig-visual-D` → sortie 0, captures regardées.

## Relecture du plan

Agent indépendant : le gabarit `format!("{}-symbolic")` de `action_icon` aurait fait échouer le test étendu — filtré (changement 2). `video-x-generic` et `audio-x-generic` (`profile_label`) ne sont aujourd'hui pas affichés (l'icône est jetée par ses deux appelants) : dessinés quand même, deux glyphes de type de fichier qui gardent le test simple et servent dès qu'un appelant les montre. Couverture partielle de l'interface et graisse, taille (ICO-12, ICO-13) : notées en Portée.

## Relecture du diff

Agent indépendant : couper chaque fichier à son premier `#[cfg(test)]` aurait retiré de la lecture le code de `popover.rs` qui suit son module de tests — la coupe ne vaut que pour les trois fichiers ajoutés, dont les tests sont en fin de fichier. `document-properties` devient un alias du « i » de `help-about`, `send-to` un alias de `document-send` (même dessin à 1 px près). Écarts resserrés entre traits voisins (`media-seek-*`, `window-new`) ; `focus-windows` ouvre le contour de la fenêtre de derrière, comme `edit-copy`. Test `glyphes_des_menus` : les 21 noms, indépendamment de ce que les fichiers lus contiennent.

## Conditions d'arrêt

- S'arrêter si un nom relevé est produit dynamiquement par une application (pas un nom de l'interface) : l'exclure du test plutôt que de le dessiner.

## Documentation

- `DESIGN.md` : rien de neuf (la règle « toute icône symbolique de l'interface vient du jeu » y est déjà) ; vérifier.
