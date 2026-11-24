# SYS-02 — Chaque notification a un état « non lue », compté par un ovale sur l'horloge de la barre, remis à zéro quand une liste s'ouvre

Écrit contre : `15cb901` (intégration `hig/plan-d-action`). Plan écrit, non exécuté.

## Chaîne de preuve

- Surface : horloge de la barre (`WidgetKind::Clock`, `src/ui/plugins/bar/surface.rs`) ; listes de l'historique — carte Notifications de la barre (`view_notification_history`, `src/ui/toasts.rs`), centre Continuité, volet de Panorama (`panorama_notifs`) ; état publié `hyprdesk-notifications.txt` (`render_state`, `src/notifications.rs`).
- Problème : aucun état de non-lue. `count=` vaut la longueur de l'historique, donc jusqu'à 50, et ne retombe qu'à « Tout effacer ». Aucun compteur sur l'horloge ; le `badge` du dock compte les fenêtres ouvertes.
- Preuve de design : HIG notifications › Badging — « Update your app's badge as soon as people open the corresponding notifications » ; notifications › Anatomy — « a small, filled oval containing a number » ; notifications › Best practices — « incrementing a badge » ; `design-plans/barre-graphite.md` — « un badge ne déplace pas l'heure ». Colonne Correction du plan : « Ajouter un état "non lue" par notification et un compteur ovale sur l'horloge de la barre, remis à zéro à l'ouverture de la liste. »
- Propriétaires : `src/notifications.rs` (`Notification`, `render_state`), `src/ui/toasts.rs` (`Toasts.history`, `toasts_update`), `src/ui/plugins/bar/surface.rs` (horloge), scène d'aperçu de barre portant l'horloge.
- Relevé à `15cb901` — les numéros de ligne du plan datent de `00f6d69`. Le cadre cliquable de l'horloge est `module` (`src/ui/plugins/bar/surface.rs` : contenu, clics gauche, droit et milieu, info-bulle, état actif). Reste à relever avant d'écrire : la fonction qui ouvre la surface Panorama — le centre Continuité (`panorama/surface/dashboard.rs`) et le volet des notifications (`panorama_notifs`) sont deux vues de cette même surface, pas deux ouvertures ; la scène d'aperçu qui montre l'horloge de la barre.

## Décision de design

- Une notification est **non lue dès qu'elle entre dans l'historique** (`TMsg::Event(Event::Shown)`), qu'elle ait une bulle ou non : basse urgence, sous Silence, critique. Un remplacement par l'application (même id) la rend non lue à nouveau : son contenu a changé. Une notification transitoire (`transient`, pas d'historique) ne compte pas.
- **Une liste de l'historique qui s'ouvre lit tout** : carte Notifications de la barre, centre Continuité, volet de Panorama. Un message `TMsg::Read` pose « lue » sur tout l'historique.
- **Le compteur** : le nombre de non-lues, dans un petit ovale plein à chiffre (HIG : « a small, filled oval containing a number »), posé sur l'horloge de la barre, au coin le plus proche du bord de l'écran. Il ne change ni la largeur ni la hauteur de l'horloge : la mesure reste celle de l'heure (« un badge ne déplace pas l'heure »). Zéro non-lue : aucun ovale. Au-delà de 99 : « 99+ ».
- **Ton de l'ovale** : « attention », comme la bulle critique (FON-02) — l'accent reste réservé à la sélection et aux actions. Chiffre dans la couleur du texte sur ce ton. Une unité de la HIG vaut un pixel logique.
- Tous les styles de barre. Graphite : heure et date, ovale au coin. Island : l'ovale suit la pilule centrale, y compris quand une bulle vivante la remplace (SYS-23, identité). Sans horloge dans la barre : pas de compteur, les listes restent la source.
- État publié : `count=` garde son sens (longueur de l'historique, lu par les widgets Lua) ; une ligne `unread=N` s'ajoute.

Pourquoi cela résout la racine : l'état existe par notification, il est visible sans ouvrir de liste, et il tombe juste au moment où la HIG l'exige — l'ouverture des notifications.

## Réutilisation

- Le cadre cliquable de l'horloge (`module`, `src/ui/plugins/bar/surface.rs`) : la pile (`stack`) enveloppe le cadre rendu, l'ovale posé à son coin ; la mesure reste celle du cadre, donc de l'heure.
- `Toasts::history` et `publish()` : même chemin d'état que `Silence` et `Passage`.
- `crate::ui::tokens` (`ts`, graisse numérique de `label_numeric`), scène d'aperçu existante.

L'ovale chiffré n'existe nulle part : le `badge` du dock est un point sans chiffre, qui compte des fenêtres. Aucune primitive parallèle.

## Changements

1. `src/notifications.rs`
   - Change : champ `unread: bool` sur `Notification`, vrai dans `build` ; `render_state` écrit une ligne `unread=N`.
   - Preserve : `count=`, les lignes `n=`, le serveur.
   - Verify : test de `render_state` (la ligne `unread`, `count` inchangé).
2. `src/ui/toasts.rs`
   - Change : `Toasts::unread() -> usize` ; un remplacement (même id) remet `unread = true` ; `TMsg::Read` lit tout, publie l'état et redessine la barre.
   - Preserve : le Silence, le passage du Silence (PAT-05), le groupement de l'historique (LAY-02), `command`.
   - Verify : tests — trois notifications dont une transitoire : deux non-lues ; `Read` remet à zéro ; un remplacement est non lu ; le compteur d'une notification d'une application refusée sous Silence (PAT-05) compte quand même.
3. Lecture à l'ouverture
   - Change : deux ouvertures appellent `toasts_update(TMsg::Read)` : la carte Notifications de la barre (`open_popover(…, popover::Kind::Notifications)`, `src/ui/mod.rs`) et l'ouverture de la surface Panorama, qui montre le centre Continuité comme le volet des notifications.
   - Verify : un test par ouverture.
4. `src/ui/plugins/bar/surface.rs`
   - Change : `fn unread_badge(n: usize, fg) -> Option<Element>` — ovale de 18 px de haut au moins, chiffre de 11 px, rayon de la demi-hauteur ; le cadre devient la pile « cadre, ovale » quand `unread > 0`.
   - Preserve : le clic ouvre le calendrier ; Island ; la largeur mesurée de l'horloge (« horloge centrée géométriquement », `DESIGN.md`).
   - Verify : test de disposition — largeur de l'horloge avec ovale égale à la largeur sans.
5. Aperçu (`src/ui/preview.rs`)
   - Change : la scène de barre qui porte l'horloge pose trois non-lues ; les autres scènes ne montrent aucun ovale.
   - Verify : capture de la scène (porte visuelle), 1440 et 640.
6. `locales/en.toml`, `docs/GUIDE.fr.md`, `docs/GUIDE.md`
   - Change : le nom accessible de l'ovale (« {n} notifications non lues ») et sa traduction ; un paragraphe sous celui du Silence : non-lues, compteur sur l'horloge, lu à l'ouverture d'une liste, `unread=` dans l'état publié.

## Périmètre

- Héritent : cartes de l'historique (barre, Continuité, Panorama) — inchangées ; l'écart demande l'état et le compteur, pas de marque sur les cartes.
- À vérifier : largeur de l'horloge avec et sans ovale ; Island avec une bulle vivante ; « 99+ » ; barre sans horloge.
- Exclut : autorisation des badges par application (PAT-07, SYS-03) ; badge du dock, qui compte les fenêtres ouvertes (SYS-23, identité) ; le retrait des notifications liées quand le compteur tombe à zéro (note de la HIG que la colonne Correction ne reprend pas) ; toute option Lua ou commande nouvelle.

## Validation

- Produit : trois notifications sous Silence → ovale « 3 » sur l'horloge ; ouvrir la carte Notifications → plus d'ovale ; une nouvelle notification sous Silence → « 1 ».
- Interface : captures de la scène, 1440 et 640 ; anglais.
- Système : `count=` inchangé, `unread=` ajouté ; aucune option ni commande nouvelle.
- Dépôt : `CARGO_BUILD_JOBS=6 bash scripts/verify.sh` → code de sortie 0 ; `scripts/verify.sh --visual /tmp/hig-visual-A`.

## Conditions d'arrêt

- S'arrêter si la pile déplace l'heure dans un style de barre : la largeur mesurée diffère.
- S'arrêter si `DESIGN.md` ne nomme pas le ton « attention » : l'ovale prendrait l'accent, réservé à la sélection et aux actions.
- S'arrêter si l'ouverture d'une liste demande plus de deux points de `App` à toucher : relever ces points et réviser le plan avant d'écrire.

## Relue à froid (sans agent indépendant)

- Qu'est-ce qui casse ? `Notification` gagne un champ : ses constructions littérales hors de `build` cassent — deux à compléter (`src/sources/mod.rs`, `src/ui/preview.rs`) et le test de `src/notifications.rs`. La barre doit se redessiner quand le compte change : `Shown` le fait déjà ; `Read` doit déclencher le même rafraîchissement. L'ovale dans Island, où une bulle vivante remplace l'heure.
- Que manque-t-il ? La persistance : l'historique est en mémoire, l'état de non-lue l'est aussi — perdu au redémarrage, comme lui. Une marque par carte, hors périmètre, dit plus haut.
- Le plan résout-il l'écart ? Oui : un état par notification, un compteur ovale sur l'horloge de la barre, remis à zéro à l'ouverture des listes.

## Documentation de design

- Après acceptation : une phrase dans `docs/GUIDE.fr.md` et `docs/GUIDE.md` ; rien dans `DESIGN.md`, qui déjà ne réserve pas l'accent à un compteur.
