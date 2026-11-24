# SEL-02 — La pastille de la couleur en cours est un puits qui ouvre le sélecteur de couleur

Rédigé contre : `d957308`

## Chaîne de preuve

- Surface : tiroir Réglages — Thème (couleurs), Barre (couleurs de la barre et des modules), Fond d'écran (source « Couleur »), options de Hyprland (registry `hyprset`), Panorama (design).
- Problème : le sélecteur de couleur (`src/ui/color_picker.rs`) n'est ouvert que par le bouton « Choisir… » des couleurs de la barre. Ailleurs, la pastille de la couleur en cours n'ouvre rien (Thème : `dot(value, 22.0, false)`), la couleur se règle par pastilles prédéfinies et champ hex (Thème, Fond d'écran), ou par un champ texte et des aperçus de 16 px (options de Hyprland, Panorama).
- Preuve de design : `design-plans/bureau-macos/04b-selection-and-input.md`, première partie, « Color wells » — « A color well displays a color picker when people tap or click it » ; « When people click a color well, it receives a highlight to provide visual confirmation that it's active. It then opens a color picker so they can choose a color. After they make a selection, the color well updates to show the new color » ; « pastille montrant la couleur courante, qui ouvre un sélecteur unique pour tout le shell » ; état actif visible dès le clic, pastille mise à jour aussitôt après le choix.
- Propriétaire : `src/ui/color_picker.rs` (`Picker`, `Picker::view`), déjà relié dans `src/ui/plugins/bar.rs` (`Msg::OpenPicker`, `Picker`, `PickerApply`, `PickerCancel`).
- Périmètre et surfaces touchées : `src/ui/color_picker.rs`, `src/ui/plugins/theme.rs`, `src/ui/plugins/bar.rs`, `src/ui/plugins/wallpaper.rs`, `src/ui/hyprset.rs`, `src/ui/mod.rs`, `src/ui/plugins/panorama/design.rs`, `docs/GUIDE.fr.md`, `docs/GUIDE.md`.
- Incertitude : `src/ui/hyprlists.rs` porte peut-être des champs de couleur dans ses éléments — à vérifier à l'étape 2, qui les prend s'il y en a. Le glisser-déposer d'un puits à l'autre (HIG, macOS) n'est pas traité : il relève des `DAT` du glisser-déposer.

## Décision de design

Partout où une couleur se règle, la pastille de la couleur en cours devient un **puits** : un bouton qui ouvre le sélecteur existant dessous sa rangée, et le referme si on le clique à nouveau.

- **Surligné** : le puits porte un anneau de 2 px dans la couleur d'accent du thème (Thème) ou d'encre (autres surfaces) dès qu'il est survolé ou pressé, et tant que son sélecteur est ouvert.
- **Mis à jour** : « Appliquer » pose la couleur choisie par le chemin déjà en place de la surface (`set_theme_color`, `set_bar_color`, `set_wallpaper`, saisie puis validation du champ) ; le puits montre alors la nouvelle couleur. « Annuler » referme sans rien changer. Tout autre changement de la couleur (pastille prédéfinie, champ hex) referme le sélecteur, comme le fait déjà la barre.
- **Inchangé** : pastilles prédéfinies, champ hex, bouton « Choisir… » de la barre (même action que le puits), « Selon le mode », retour au thème, options Lua et commandes en ligne.

## Réutilisation

- `Picker`, `PickerMsg`, `Picker::view`, `color` : `src/ui/color_picker.rs`.
- Idiome d'état « `(clé, Picker)` dans l'état du paquet » : `Panel.bar.picker` et `Msg::OpenPicker`…`PickerCancel` (`src/ui/plugins/bar.rs`).
- Pastilles en place : fermetures `dot` de `theme.rs` et `bar.rs` (cercle plein, contour discret).

Une primitive nouvelle, `color_picker::well`, tient lieu de puits : les deux fermetures `dot` sont locales à leur fichier et ne sont pas des boutons ; elle vit avec le sélecteur, dans son propriétaire. Les pastilles prédéfinies restent des `dot`.

## Changements

1. `src/ui/color_picker.rs` — `well(c, taille, actif, anneau, appuyé) -> Element<M>` : bouton rond plein, contour 1 px discret, anneau de 2 px au survol, au pressage et quand `actif`. Test : style inactif, actif, pressé.
2. `src/ui/plugins/theme.rs` — `State::picker : Option<(Slot, Picker)>` ; `Msg::OpenPicker(Slot)`, `Picker`, `PickerApply`, `PickerCancel` ; la pastille de la couleur en cours des quatre emplacements (texte, fond, accent, fond d'écran) devient `well` ; le sélecteur se dessine sous la rangée. « Appliquer » → `Msg::Color(emplacement, couleur)`.
3. `src/ui/plugins/bar.rs` — la pastille de la couleur en cours (barre et modules) devient `well`, `actif` quand son sélecteur est ouvert.
4. `src/ui/plugins/wallpaper.rs` — `State::picker : Option<Picker>` ; mêmes quatre messages ; un puits de la couleur en place ouvre le sélecteur sous la rangée « Couleur ». « Appliquer » → `Msg::Color(couleur)`.
5. `src/ui/hyprset.rs`, `src/ui/mod.rs` — voir étape 2.
6. `src/ui/plugins/panorama/design.rs` — voir étape 3.
7. Documentation : `docs/GUIDE.fr.md`, `docs/GUIDE.md`.

## Étapes

1. **Primitive, Thème, Barre, Fond d'écran** (changements 1 à 4 et 7). — faite (`7ad04d2`, itération B11).
2. **Options de Hyprland** : `App::opt_picker : Option<(clé « magasin · chemin », Picker)>` ; `Msg::OptPickerOpen`, `OptPicker`, `OptPickerApply`, `OptPickerCancel` ; dans `opt_field`, la rangée d'une `Kind::Color` reçoit un puits avant le champ, une `Kind::Gradient` un puits par couleur de son aperçu ; « Appliquer » écrit le brouillon (`Msg::Draft`) puis le valide (`Msg::Submit`), comme la saisie suivie de Entrée. Les aperçus de 16 px restent pour les dégradés. — à faire.
3. **Panorama** : `design_picker : Option<(clé, Picker)>` dans l'état du panneau Panorama ; `Msg::DesignPickerOpen`, `DesignPicker`, `DesignPickerApply`, `DesignPickerCancel` ; `panorama_design_input` reçoit un puits pour les clés de couleur (`background`, `foreground`, `accent`) ; « Appliquer » écrit le brouillon (`Msg::DesignText`) puis l'applique (`Msg::DesignApply`). — à faire.

## Périmètre

- Héritent : toutes les pages de réglage qui règlent une couleur.
- À vérifier : Thème (quatre emplacements, choix puis annulation), Barre (puits et « Choisir… » ouvrent le même sélecteur, couleur par mode), Fond d'écran (source « Couleur », puis retour à une image), scènes d'aperçu des Réglages.
- Exclu : glisser-déposer des couleurs, sélecteur du système, contenu du sélecteur, réglages des couleurs d'écran (ICC, gamma), tout autre écart.

## Validation

- Produit : cliquer la pastille de la couleur en cours → elle se surligne et le sélecteur s'ouvre dessous ; choisir, « Appliquer » → le puits montre la nouvelle couleur ; cliquer le puits, changer d'avis, « Annuler » → rien n'a changé.
- Interface : Thème (texte, fond, accent, fond d'écran), Barre (une couleur par mode), Fond d'écran (source « Couleur »), configuration de base.
- Système : un seul puits (`color_picker::well`) et un seul sélecteur ; aucune pastille de couleur en cours qui ne soit pas un puits.
- Dépôt : `CARGO_BUILD_JOBS=6 bash scripts/verify.sh` → code de sortie 0 ; `scripts/verify.sh --visual /tmp/hig-visual-B` → captures des scènes de réglages regardées.

## Conditions d'arrêt

- S'arrêter si une surface ne peut pas porter l'état `Picker` sans toucher à un propriétaire partagé hors de ceux listés, ou si « Appliquer » ne peut pas passer par le chemin existant de la surface.

## Documentation de design

- Après acceptation : `docs/GUIDE.fr.md` et `docs/GUIDE.md` disent que la pastille de la couleur en cours ouvre le sélecteur de couleur.

## Relecture à froid (sans outil d'agent)

Relu après la lecture de `color_picker.rs`, des trois paquets et de `hyprset.rs`, avant d'écrire le code.

- **Qu'est-ce qui casse ?** Deux points, pris en compte dans le plan. (1) Un sélecteur resté ouvert pendant que la couleur change par une pastille prédéfinie ou le champ hex montrerait un « avant » périmé : tout autre changement de la couleur le referme, comme `Msg::SetColor` le fait déjà pour la barre. (2) Au Fond d'écran, le champ hex est un brouillon : le puits montre la couleur en place (celle de la règle), pas le brouillon. Rien d'autre : l'état et les messages vivent dans l'état et les messages de chaque paquet, comme celui de la barre ; `well` est une fonction sans état.
- **Que manque-t-il ?** Aucune chaîne nouvelle : le sélecteur dit déjà « Appliquer » et « Annuler ». L'effort : cinq surfaces dans six fichiers, dont l'état central (`App`) pour les options de Hyprland — l'écart relevé M devient L, en trois étapes. La documentation des guides. `bar.rs` est un paquet, `panel.rs` n'est pas touché à l'étape 1 ; relecture du diff obligatoire (écart L).
- **Le plan résout-il l'écart ?** Oui, une fois les trois étapes faites : chaque pastille de couleur en cours ouvre le sélecteur existant, se surligne au clic et se met à jour après le choix.

### Relecture du diff (étape 1 ; sans outil d'agent)

Relu à froid, après le contrôle rapide (`fmt`, `clippy` à zéro avertissement), sur le diff avant le commit, puis sur les captures de `scripts/verify.sh --visual` (`theme` et `wallpaper`, 1440 px).

- **Le diff fait-il exactement ce que l'écart demande ?** Oui, pour les trois surfaces de l'étape : Thème (quatre emplacements), Barre (le puits et « Choisir… » envoient le même `Msg::OpenPicker`, qui bascule), Fond d'écran (source « Couleur »). `well` vit avec le sélecteur, dans `color_picker.rs` ; test `puits_surligne`. Les options de Hyprland et Panorama restent pour les étapes 2 et 3.
- **Respecte-t-il `DESIGN.md` et les conventions ?** Une famille de police, aucun texte nouveau, aucun mouvement ; l'anneau prend l'accent sur la page Thème (action et sélection), l'encre ailleurs ; commentaires en français ; guides français et anglais complétés.
- **Régression, test manquant ?** Un point, corrigé avant la porte : au Fond d'écran, un sélecteur resté ouvert quand la source devenait une image ou un diaporama réapparaissait ouvert au retour à « Couleur » ; `Msg::PickImage` et `Msg::Kind` le referment. Laissé tel quel, à dessein : sur la page Thème, poser un thème prédéfini pendant qu'un sélecteur est ouvert ne le referme pas ; son « avant » périmé est sans conséquence (« Appliquer » pose la couleur choisie, « Annuler » ne change rien). Les rangées de couleurs ne sont dans aucune scène d'aperçu : les captures ne font que confirmer que les deux pages se dessinent.
