# LAY-02 — L'historique des notifications est groupé par application

Écrit contre : `93a7b71` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : les trois rendus de l'historique — carte Notifications de la barre (`src/ui/toasts.rs`, `view_notification_history`) ; carte « Notifications » du centre de contrôle Continuité (`src/ui/plugins/bar/popover/continuity.rs`, `dashboard_notifications`) ; volet droit de Panorama (`src/ui/plugins/panorama/surface/panes.rs`, `panorama_notifs`).
- Problème : chaque rendu est une liste à plat. Le nom de l'application est répété dans chaque carte (en tête à la barre et dans Panorama, en detail à défaut de corps dans Continuité) ; rien ne dit que deux cartes voisines viennent de la même application, ni où finissent celles-ci.
- Preuve de design : HIG lists-and-tables › Discussion — « A table or list can represent data that’s organized in groups or hierarchies » ; boxes › Discussion — « By default, a box uses a visible border or background color to separate its contents from the rest of the interface », titre facultatif ; boxes › Best practices — pas de boîtes imbriquées, les sous-groupes passent par la marge interne et l'alignement ; boxes › Content — titre du groupe en majuscules de phrase ; plan `02-layout.md` — « Cartes de notification et widgets : un seul niveau de cadre ».
- Propriétaires : les fonctions partagées des cartes de l'historique, dans `src/ui/toasts.rs` (`card_text`, `card_press`), déjà utilisées par les trois rendus. Existant le plus proche : les résultats de recherche des Réglages, un en-tête (`panel.rs`, `section`) quand le groupe change.
- Périmètre et surfaces touchées : `src/ui/toasts.rs`, `continuity.rs`, `panes.rs` (les trois seules vues) ; scène d'aperçu « bar-notifications » (`src/ui/preview.rs`, `scripts/verify.sh`) ; guides, `DESIGN.md`. Aucun modèle, aucun message, aucune chaîne à traduire.
- Incertitude : le modèle ne porte aucune date (`src/notifications.rs`) : aucun groupe par heure, l'écart ne retient que le groupe par application.

## Décision de design

Une fonction pure, `app_groups`, découpe un historique (les plus récentes d'abord) en groupes d'application ; les trois rendus dessinent **une boîte par application, son nom en en-tête, ses notifications à la suite**.

- Ordre : les applications dans l'ordre de leur notification la plus récente ; dans une boîte, l'ordre de l'historique. La plus récente reste en haut.
- Clé du groupe : le nom montré en en-tête (`card_app`) — celui de l'application, « Notification » quand elle n'en donne pas. `desktop-entry` n'est pas la clé : il ne se montre pas, et deux boîtes de même en-tête se liraient comme une faute ; les applications sans nom forment une seule boîte.
- La ligne « application » quitte la carte pour l'en-tête de sa boîte : elle n'y est plus répétée. Icône, titre, corps borné (`card_text`), clic (`card_press`) et masque des aperçus (HIG SYS-05) ne changent pas.
- Un seul niveau de cadre : la boîte est le cadre ; les notifications qu'elle contient n'en ont pas. Là où chaque carte avait le sien (Continuité, Panorama), il devient celui de sa boîte — même matière, même rayon. Les cartes de la barre, nues jusqu'ici, prennent un fond de la couleur du texte à 6 % (`GROUP_BOX`), rayon 10 (`design::SMALL_RADIUS`).
- En-tête : 11 px dans la couleur secondaire de sa surface (`dim`, `pal.muted`), en capitales telles quelles (nom propre, majuscules de phrase) ; dans Continuité, la police de titres du tableau de bord, comme les titres de ses tuiles.
- Continuité : les tuiles gardent leur retour de survol et d'appui (`pal.hover`), sans fond au repos ; un corps vide ne donne plus de ligne de détail (c'était le nom de l'application).
- Panorama : l'en-tête reprend la place et la taille de l'ancienne ligne « application » (0,75 × taille).

Pourquoi pas un en-tête sans boîte, comme la recherche des Réglages : là, chaque résultat est déjà une tuile ; ici, trois rendus compacts, dont deux de cartes nues — un en-tête seul n'y séparerait rien (boxes › Discussion).
Pourquoi pas des boîtes autour des cartes encadrées existantes : deux niveaux de cadre, que le plan et la HIG écartent.

## Réutilisation

- `crate::ui::toasts::{card_text, card_press, notif_icon}` — texte, clic et icône d'une carte, inchangés.
- `crate::ui::design::SMALL_RADIUS` ; `crate::ui::tokens::ts` ; `dim` (`DIM`) de `toasts.rs`.
- Continuité : `Palette::surface`, `Palette::inset`, `Palette::hover`, `dash_label`, `title_font`, `keyboard::press`.
- Panorama : le style de carte existant (`fx::alpha(fg, 0.06)`, rayon `size * 0.6`).
- Exemple : `src/ui/panel.rs:1229` (`section`), `src/ui/preview.rs` scène « bar-menu-workspaces ».

Aucune primitive nouvelle de rendu : le découpage est une fonction de données, à côté de celles qu'elle remplace en fait de source.

## Changements

1. `src/ui/toasts.rs`
   - `pub(super) fn card_app(n) -> String` ; `pub(super) fn app_groups(history: &[Notification]) -> Vec<(String, Vec<usize>)>` (en-tête, indices dans l'historique).
   - `view_notification_history` : `app_groups` sur les 30 premières ; par groupe, `container(column![en-tête, cartes])`, marge `[8, 12]`, espacement 8, fond `GROUP_BOX`, rayon 10 ; cartes sans la ligne « application » ; espacement des boîtes 12.
   - Preserve : titre « Notifications », état vide, icône, tailles et couleurs, clic, corps masqué.
   - Verify : tests unitaires du découpage ; scène « bar-notifications ».
2. `src/ui/plugins/bar/popover/continuity.rs` — `dashboard_notifications` : `app_groups` sur les 12 premières ; par groupe, une boîte `pal.surface(pal.inset, 12.0)`, marge 4 (grille de 4 px), en-tête 11 px (marge `[2, 10]`, 4 px sous lui) ; tuiles sans fond au repos, survol `pal.hover`, sans écart entre elles (leur marge propre de 8 px les sépare) ; détail = corps seulement.
   - Preserve : en-tête du tableau de bord, « Effacer l'historique », état vide, clavier, troncatures 42/74.
   - Verify : scène « bar-controls ».
3. `src/ui/plugins/panorama/surface/panes.rs` — `panorama_notifs` : `app_groups` sur les 30 premières ; la carte existante devient la boîte du groupe (en-tête, puis ses cartes).
   - Preserve : « Tout effacer », fermeture, état vide, largeur du volet.
4. `src/ui/preview.rs`, `scripts/verify.sh` — scène « bar-notifications » : la carte Notifications ouverte sur un historique de trois applications.
5. `docs/GUIDE.fr.md`, `docs/GUIDE.md`, `DESIGN.md` — une phrase, une règle.

## Périmètre

- Héritent : rien d'autre (les bulles, `src/notifications.rs`, la commande `hyprdesk notifications` ne changent pas).
- À vérifier : les trois rendus vides, à une seule application, à applications sans nom ; Continuité en 900 et 640 px.
- Exclus : compteur ou effacement par groupe ; groupe par heure (le modèle n'a pas de date) ; couleurs des libellés de la carte de la barre (LAY-03) ; état « non lue » (SYS-02).

## Validation

- Produit : trois notifications de deux applications s'affichent en deux boîtes, chacune nommée, ses notifications de la plus récente à la plus ancienne ; un clic sur une carte fait ce qu'il faisait.
- Interface : scènes « bar-notifications » et « bar-controls », 1440, 900 et 640 px ; état vide.
- Système : un seul découpage pour les trois rendus ; aucun cadre dans un cadre ; aucune chaîne ajoutée.
- Dépôt : `CARGO_BUILD_JOBS=6 cargo fmt --all && CARGO_BUILD_JOBS=6 cargo clippy --locked --workspace --all-targets -- -D warnings`, puis `CARGO_BUILD_JOBS=6 bash scripts/verify.sh` → code de sortie 0 ; `scripts/verify.sh --visual /tmp/hig-visual-A` → les deux scènes montrent des boîtes par application.

## Conditions d'arrêt

- Arrêter si la carte de la barre (420 px, défilante) ne montre plus aucune carte entière avec en-têtes et marges : réduire la marge des boîtes, pas les regrouper.
- Arrêter si les tuiles de Continuité perdent leur retour de clavier ou de survol : rendre son fond à la tuile et garder la boîte sans fond.
- Arrêter si le découpage doit toucher au modèle (`src/notifications.rs`) : hors de l'écart.

## Documentation de design

- Après acceptation : une règle dans `DESIGN.md` — l'historique des notifications est groupé par application, une boîte par application avec son nom en en-tête, les plus récentes en haut, un seul niveau de cadre ; une phrase dans les deux guides.

## Relecture à froid (sans outil d'agent)

Relu après l'écriture de la scène d'aperçu, contre le code des trois vues.

- **Qu'est-ce qui casse ?** (1) Un groupe n'est pas un bloc contigu de l'historique (deux applications alternent) : le découpage rend donc, par groupe, la liste des indices, pas une borne. Corrigé dans le plan. (2) La ligne « application » était vide pour une application sans nom : l'en-tête dit « Notification », comme la tête d'une bulle. (3) La carte de la barre passe de 12 px d'écart entre cartes nues à des boîtes de 8 px de marge : plus haute d'environ 16 px par application ; la carte défile déjà au-delà de 420 px.
- **Que manque-t-il ?** La carte Notifications de la barre n'avait aucune scène : sans elle, le premier rendu ne se vérifie pas. Ajoutée (changement 4). La limite (30, 12) s'applique avant le découpage, pour qu'une notification coupée ne rouvre pas un groupe.
- **Le plan résout-il l'écart ?** Oui : les trois rendus groupent par application, une boîte nommée par application, par une seule fonction de données ; le groupe par heure reste hors de portée du modèle, et l'écart ne le retient pas.
- Repris après lecture de `popover.rs` : la carte Notifications a une hauteur fixe de 420 px (défilante) et se passe d'ancre (`open_popover` la centre) ; la scène n'a qu'à ouvrir la carte. Aucune taille de carte ne dépend du nombre de notifications.
