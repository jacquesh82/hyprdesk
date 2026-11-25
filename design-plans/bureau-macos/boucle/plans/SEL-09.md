# SEL-09 — Les curseurs des Réglages disent leur échelle : minimum et maximum étiquetés

Écrit contre : `978e08c` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : tiroir des Réglages, chaque champ à curseur ; la luminosité du sélecteur de couleurs.
- Problème : un curseur à pas dit rarement son échelle — ni graduation, ni valeur de bornes (`Kind::Num { min, max, step }` : `.step(*step as f32)` sans borne affichée ; taille du curseur `16.0..=96.0`, pas de 2 ; luminosité du sélecteur, `0..=100u32`). On tire la poignée pour savoir où elle s'arrête.
- Preuve de design : HIG sliders › Platform considerations › macOS — « Use tick marks to increase clarity and accuracy. » ; « In many cases, labeling only the minimum and maximum values is sufficient. »
- Recensement à HEAD : les numéros de ligne du plan (relevés à `00f6d69`) ont bougé — `hyprset.rs:252-257` est la rangée « Rétablir », `:686-690` le rangement des sections, `color_picker.rs:186-190` la zone de teinte. Retrouvés par le contenu. Pas explicite : `hyprset.rs` (`Kind::Num` ; taille du curseur), `curves.rs` (`.step(step)`), `theme.rs` (entier : 1 ; flottant : centième du domaine), `effects.rs` (10 ; 50), `widgets.rs` (2 ; 2 ; 1), `animations.rs` (0,5), `colors.rs` (0,01). Pas entier implicite sur les domaines entiers (`24..=128u16`, `0..=100u32`…).
- Aucun curseur du tiroir ne compte moins de vingt pas : les plus courts sont `0..20` par 1 et `0..1` par 0,05 (vingt pas), `1..20` par 0,5 (trente-huit) ; la taille du curseur en compte quarante. À 200 px, vingt pas mettent dix pixels entre deux marques.
- Propriétaires : `App::slider` et `SliderSpec { range, step }` (`src/ui/panel.rs`, HIG SEL-08) — un seul endroit pour tout curseur converti ; les sites encore en `row![slider(…), text(…)]` (quarante-quatre : étapes 2 et 3 de SEL-08) ; le sélecteur (`src/ui/color_picker.rs`), hors tiroir.
- Périmètre et surfaces touchées : `src/ui/panel.rs`, `src/ui/color_picker.rs`, `src/ui/preview.rs`, `scripts/verify.sh`, `DESIGN.md`.
- Incertitude : la HIG ne fixe ni le nombre de pas qui appelle des graduations, ni leur dessin. Décision ci-dessous.

## Décision de design

- Sous chaque curseur du tiroir (`App::slider`), une ligne d'échelle : le minimum à gauche, le maximum à droite, dans le format du site (`affiche(min)`, `affiche(max)` : `24 px` et `96 px`), 11 px, encre secondaire (`App::ink().dim`). Elle a la largeur du curseur seul : la valeur saisissable reste à droite, sur la ligne du curseur.
- Pas de graduation à dessiner : aucun curseur du tiroir ne compte dix pas au plus, et la HIG juge l'étiquetage des bornes suffisant. Règle retenue pour la suite, notée dans `DESIGN.md` : un domaine de dix pas au plus porte un trait sous chaque pas, dès sa première occurrence.
- Les curseurs sans pas explicite étiquettent leurs bornes de même : l'échelle se lit, la précision reste celle du curseur.
- Sélecteur de couleurs, luminosité (`0..=100`) : `0 %` et `100 %` sous le curseur. Pas de graduation (cent une valeurs).
- Hors correction : les curseurs des menus de la barre (volume, luminosité), dont les icônes aux extrémités disent déjà l'échelle, dans des rangées de 32 px ; l'assistant de premier démarrage.

Pourquoi cela résout la racine : chaque curseur dit son échelle d'un coup d'œil, par un seul propriétaire, sans mesure ni texte ajoutés site par site.

## Réutilisation

- `App::slider` (`panel.rs`) : la ligne d'échelle se pose sous `row![slider, value]`, dans un `column![…].spacing(4)` ; sa largeur est celle du curseur (`width` reçue).
- Texte secondaire du tiroir : `App::ink().dim`, `crate::ui::tokens::ts(11.0)`, comme les aides de `App::field`.
- Sélecteur : la rangée `lum` de `color_picker.rs`, même ligne sous le curseur, dans son encre secondaire (`dim`, tenue à 4,5:1).

Pas de primitive nouvelle : ni jeton, ni style.

## Changements

1. `src/ui/panel.rs`
   - Changement : `App::slider` rend `column![row![slider, value], échelle].spacing(4)` ; `échelle` est la ligne `row![text(affiche(min)), Space::fill(), text(affiche(max))]`, largeur celle du curseur (`width` reçue).
   - Préserver : la place du curseur et de sa valeur sur leur ligne, le glissé, le champ de saisie et ses tests (SEL-08).
   - Vérifier : les scènes des paquets à curseur gagnent une ligne sous chaque curseur, rien d'autre ne bouge.
2. `src/ui/color_picker.rs`
   - Changement : la luminosité porte `0 %` et `100 %` sous son curseur, 11 px, dans la teinte secondaire du sélecteur.
   - Préserver : le dégradé, la largeur `Fill`.
   - Vérifier : la scène d'aperçu du sélecteur, si elle existe ; sinon, à la mesure.
3. `src/ui/preview.rs`, `scripts/verify.sh`
   - Changement : scène `reglages-echelle` (paquet Dock, « Taille des icônes » : `24 px` et `96 px` lisibles) ; `SCENES` gagne une entrée ; la scène entre dans la liste de la porte visuelle.
   - Préserver : les scènes existantes et leur ordre.
   - Vérifier : la capture existe aux trois largeurs.
4. `DESIGN.md`
   - Changement : une ligne — « Sous chaque curseur des Réglages, le minimum et le maximum, dans le format du réglage ; des graduations seulement sous un domaine de dix pas au plus. » Les guides ne bougent pas : aucun geste nouveau.

## Ordre

- Après SEL-08, étapes 2 et 3 : tant que les quarante-quatre curseurs restent hors de `App::slider`, l'échelle ne paraîtrait que sous « Taille des icônes ». Si SEL-08 vient à être bloqué, SEL-09 s'applique aux seuls sites convertis et le dit au registre.

## Périmètre

- Héritent : tous les curseurs passés par `App::slider` (quarante-cinq à terme) ; la luminosité du sélecteur.
- À vérifier : `reglages-echelle` à 1440, 900 et 640 px ; les scènes des paquets à curseur (bureau, barre, thème, effets, widgets, animations, écrans…).
- Exclus : les menus de la barre ; l'assistant de premier démarrage ; les graduations des curseurs de plus de dix pas.

## Validation

- Produit : Réglages → Dock → « Taille des icônes » : `24 px` au début du curseur, `96 px` à sa fin.
- Interface : la valeur saisissable reste sur la ligne du curseur ; glissé, Entrée et Échap inchangés (tests de SEL-08).
- Système : un seul endroit pose l'échelle (`App::slider`) ; aucune chaîne nouvelle à traduire, les bornes passent par `affiche`.
- Dépôt : `CARGO_BUILD_JOBS=6 bash scripts/verify.sh` → code 0 ; `CARGO_BUILD_JOBS=6 bash scripts/verify.sh --visual /tmp/hig-visual-B` → code 0, `reglages-echelle` regardée aux trois largeurs.

## Conditions d'arrêt

- Arrêter si la ligne d'échelle pousse la valeur hors de sa rangée à 640 px ; si `affiche` ne suffit pas à écrire les bornes d'un site (format qui dépend d'un état, pas de la seule valeur) ; si le sélecteur n'a pas de teinte secondaire propre.

## Documentation de design

- Après acceptation : la ligne de `DESIGN.md` citée au changement 4.

## Relecture à froid (sans outil d'agent)

Relu après avoir revérifié trois appuis dans le code : l'encre secondaire du sélecteur (`dim`, `color_picker.rs`, tenue à 4,5:1), l'absence de scène d'aperçu du sélecteur (`preview.rs`), les champs `fg` et `dim` de `Ink` (`panel.rs`).

- **Qu'est-ce qui casse ?** Deux risques, tranchés. Chaque rangée à curseur gagne environ dix-neuf pixels (une ligne de 11 px, un espacement de 4) : voulu, c'est l'information nouvelle ; à 640 px la valeur saisissable doit rester sur la ligne du curseur — la ligne d'échelle a la largeur du curseur seul, d'où la première condition d'arrêt. Une borne mal écrite : un site dont `affiche` dépend d'un état et non de la seule valeur écrirait deux fois la même étiquette — bon sur le site pilote (`{} px`), à surveiller aux étapes 2 et 3 de SEL-08, d'où la seconde.
- **Que manque-t-il ?** Les graduations elles-mêmes : aucun curseur du tiroir n'a dix pas au plus, et un dessin qu'aucun réglage ne peut atteindre ne se vérifie pas — il est renvoyé à la première occurrence, la règle notée dans `DESIGN.md`. Le sélecteur n'a pas de scène : sa ligne se vérifie à la mesure. La dépendance d'ordre : sans les étapes 2 et 3 de SEL-08, l'échelle ne paraît que sous un curseur.
- **Le plan résout-il l'écart ?** Oui pour « étiqueter leur minimum et leur maximum », sur tout curseur converti ; non pour « graduer », et il le dit : à l'examen du propriétaire, avec la ligne du registre.

## Exécution (B23, `e4b73e0`)

Plan relu par un agent indépendant avant exécution ; trois corrections apportées au plan, exécutées avec lui :

- **La prémisse « aucun curseur n'a dix pas au plus » était fausse** : « Passes » du flou (1 à 6), « Atténuation » (1 à 4, deux fois), colonnes des réglages rapides (1 à 4), nombre de notifications visibles (1 à 8). La règle du plan s'applique donc dès maintenant : `App::slider` dessine un trait de 1 × 4 px sous chaque pas, au droit de la poignée (rail parcouru de 7 px à `largeur - 7 px`, poignée par défaut d'iced), quand le domaine compte dix pas au plus (`graduations`, testé).
- **L'échelle a la largeur du curseur seul** : `column![curseur, graduations?, bornes]` à la largeur du curseur (le plafond d'un curseur `capped` s'applique à la colonne), non `column![row![curseur, valeur], échelle]`, qui aurait mis le maximum sous la valeur pour les curseurs souples.
- **Les voisins d'une rangée centrée** (libellé et ↺ des couleurs, interrupteurs Flou et Adaptatif du thème, interrupteur et origine d'une animation, bouton muet du son, « Taille » de la zone Panorama, origine d'une option numérique de Hyprland, « Luminosité » du sélecteur ; puis, relevés par la relecture du diff : bouton « Thème » de l'opacité du fond de la barre, « Taille du texte », interrupteur « Flou » du verrouillage, libellés des paramètres de courbe) descendaient d'environ 9 px sous le rail : ils gardent sous eux la place de l'échelle (`beside_slider`, `slider_band`, `panel.rs`), comme la valeur.

Écarts au plan : la scène `reglages-echelle` montre Thème → Composants (bornes et graduations), non le Dock (bornes seules, déjà visibles dans `reglages-valeur-saisie`). Zones de tuile du Panorama : le maximum est la place restante (100 moins l'autre dimension) ; ses bornes la disent et bougent avec elle — assumé, c'est le domaine réel. Repère du neutre sur les domaines signés : non traité, hors correction.
