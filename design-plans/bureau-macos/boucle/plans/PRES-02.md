# PRES-02 — Les cartes de la barre changent de taille en douceur

Écrit contre : `2d66186` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : les cartes de la barre hors tiroirs Convex (`src/ui/plugins/bar/popover.rs`) — Son, Luminosité, Calendrier, menus des modules et du tray, débordement, widgets.
- Problème : revérifié à HEAD. D'une carte à l'autre, le contenu se fond en 160 ms (`p.change`, `view_popover`), mais la surface et le cadre prennent leur taille finale d'un bloc : `open_popover` écrit `p.size` (estimation `card_height`), puis `fit_popover` écrit `p.size.height = height` à la mesure du contenu. Ouverture d'une confirmation (`PMsg::Activate` → `p.confirm`), retour à la ligne, section audio qui apparaît : même saut, sans fondu du tout. Seuls les tiroirs rattachés (`p.attached`) interpolent leur hauteur (`Attached::from_height` → `height`, au rythme de `p.change`), et seulement pendant un changement de carte.
- Preuve de design : HIG popovers › Best practices — « Provide a smooth transition when changing the size of a popover. »
- Propriétaire : `Popover` et `App::fit_popover`, `open_popover`, `popover_frame`, `view_popover` (`src/ui/plugins/bar/popover.rs`).
- Périmètre : cartes non rattachées, hors tableau Continuité (`p.dashboard`, qui suit `dashboard_height`).
- Incertitude : la surface layer-shell a une taille fixe : on ne peut pas l'animer image par image sans un aller-retour de configuration par image. On l'anime donc comme les tiroirs : la surface prend d'emblée la plus grande des deux hauteurs, le cadre dessiné interpole, la surface se resserre à la fin.

## Décision de design

Toute variation de la hauteur d'une carte non rattachée (changement de carte, mesure du contenu, confirmation) s'interpole en 160 ms (`bar_duration(160)`, la durée du fondu de contenu), depuis la hauteur dessinée à l'instant du changement — un changement en cours repart de là où il en est, il est annulable. Sans transition (`transition = "none"`) ou avant le premier rendu, la taille est immédiate, comme aujourd'hui.

Le cadre se colle au bord ancré de la surface : en haut pour une barre en haut ou verticale, en bas pour une barre en bas (la surface y est ancrée au bas, la carte grandit vers le haut). Pendant la croissance, le contenu dépasse brièvement le cadre : la barre de défilement est masquée pendant l'interpolation, pour ne pas clignoter ; le widget reste le même (l'état du défilement et du champ de saisie se conserve).

## Réutilisation

- `crate::ui::Fade` (déjà le moteur de `visibility` et `change`), `bar_duration`.
- Exemple : l'interpolation `Attached::from_height` → `height` et le resserrement de la surface en fin de `change` dans `popover_frame`.

Aucune primitive nouvelle : un champ `grow: Option<Grow>` sur `Popover` (`from`, `to`, `Fade`), et une méthode `Popover::drawn_height(now)` qui rend la hauteur dessinée.

## Changements

1. `src/ui/plugins/bar/popover.rs`
   - `struct Grow { from, to, fade }` ; champ `grow` dans `Popover`, `None` à la création.
   - `Popover::drawn_height(now)` : `grow` interpolé, sinon `size.height` ; comme `size.height`, elle inclut l'ombre (`2 × SHADOW`).
   - `Popover::target_height()` : `attached.height`, sinon `grow.to`, sinon `size.height`. La garde de `fit_popover` compare la mesure à cette cible, pas à la surface (qui vaut `max(from, to)` pendant l'interpolation).
   - `Popover::retarget(target, now, dur)` : sans durée, sans `shown`, à la fermeture, carte rattachée ou tableau : `size.height = target`, `grow = None`. Sinon `from = drawn_height(now)`, `to = target`, surface `max(from, target)`.
   - `fit_popover` (non rattaché) et la branche de réemploi d'`open_popover` (carte simple remplacée, hors tableau) passent par `retarget` ; le `SizeChange` de cette branche envoie la surface (`p.size`), plus l'estimation.
   - `refresh_popover_anchor` (non rattaché) recopie `p.size.height`, la surface : inchangé.
   - `popover_frame` : `grow` achevé (ou transitions coupées) → `size.height = to`, `grow = None`, `SizeChange` si la surface change.
   - `animating` : vrai tant que `grow` est en cours (la souscription d'images de `src/ui/mod.rs` le suit déjà pour une carte non rattachée).
   - `view_popover` : pendant `grow`, le conteneur de la carte (marge d'ombre comprise) a la hauteur `drawn_height`, aligné sur le bord ancré ; barre de défilement masquée.
   - Preserve : tiroirs Convex (inchangés), tableau Continuité, glissement horizontal `slide_from`, fondu de contenu, `transition = "none"`.
   - Verify : tests unitaires ci-dessous.

## Portée

- Hérite : toutes les cartes non rattachées de la barre.
- Vérifier : les tiroirs Convex (tests `convex_*` existants), le tableau Continuité (`continuity_controls_*`), les scènes d'aperçu (transitions coupées : taille immédiate).
- Exclure : les tiroirs rattachés hors changement de carte (une confirmation y saute encore : le tiroir a sa propre mécanique, qu'il faudrait reprendre à part). Le fond réfracté (`behind`) garde l'origine relevée à la capture : une surface ancrée en bas qui change de hauteur décalait déjà sa réfraction avant ce plan ; l'état final est le même qu'avant, l'interpolation ne fait que l'étaler. Barre verticale : la marge haute (`side_top`) se borne avec la hauteur visée ; pendant une croissance près du bas de l'écran, la surface peut dépasser brièvement, sans effet sur le cadre dessiné. Pendant un rétrécissement, la bande transparente de la surface prend les clics 160 ms au lieu du capteur : mineur, accepté.

## Validation

- Produit : ouvrir Son puis Calendrier ; ouvrir une entrée à confirmation : le cadre glisse vers sa taille, sans saut.
- Tests : une mesure plus haute garde la surface à la plus grande hauteur et dessine l'ancienne, puis se resserre en fin d'interpolation ; une mesure plus basse garde la surface jusqu'à la fin ; un second changement en cours repart de la hauteur dessinée ; sans transition, taille immédiate.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh` → sortie 0.

## Relecture du plan et du diff

Relu par un agent indépendant : garde de `fit_popover` à comparer à la cible, ombre à ne pas compter deux fois, `SizeChange` de la branche de réemploi, fermeture — corrigés ci-dessus ; fond réfracté et clics de la bande transparente — notés en Portée. Relecture du diff : rétrécissement complet et fermeture ajoutés au test ; barre verticale notée en Portée.

## Conditions d'arrêt

- S'arrêter si la surface layer-shell ne peut pas être agrandie avant le dessin (saut visible inverse) : marquer `à reprendre`.

## Documentation

- `DESIGN.md`, section mouvement : « une carte qui change de taille interpole sa hauteur (160 ms), la surface prend la plus grande des deux ».
