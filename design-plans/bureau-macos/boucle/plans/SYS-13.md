# SYS-13 — En style Island, le clic sur la pilule occupée par une notification ouvre sa vue étendue : corps, actions, accès à la carte des notifications

Écrit contre : `97f427f` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : pilule centrale de la barre de style Island, quand une bulle vivante remplace l'horloge (`WidgetKind::Clock`, `src/ui/plugins/bar/surface.rs`).
- Problème : la pilule ne montre qu'un glyphe et le résumé coupé à 26 signes ; le clic ouvre le calendrier, comme si l'horloge était là. Ni corps, ni action, ni passage vers l'historique.
- Preuve de design : HIG live-activities › Offering interactivity — « Take people directly to related details and actions » ; live-activities › Anatomy — « your Live Activity must support: » compact, minimal, expanded ; live-activities › Best practices — « For content people might consider sensitive or private, display an innocuous summary and let people tap the Live Activity to view the sensitive information in your app ». Colonne Correction du registre : « Ouvrir au clic sur la pilule une vue étendue de la notification : corps, actions, accès à la carte des notifications. » `design-plans/barre-island.md` : la carte « naît de la pilule ».
- Propriétaires : cartes de la barre (`src/ui/plugins/bar/popover.rs` : `Request`, `Kind`, `pop_content`, `card_height`), bulles et carte Notifications (`src/ui/toasts.rs`), horloge (`surface.rs`), scène d'aperçu.
- Relevé à `97f427f` — les numéros de ligne du plan datent de `00f6d69`. La prise de main est réelle : `surface.rs`, bras `WidgetKind::Clock` (`self.toasts.list.first()` quand le style est Island) ; le cadre cliquable `module` porte `Msg::OpenPopover(.., Request::Calendar)`. La carte Notifications existe (`Kind::Notifications`, `view_notification_history`) : titre et corps coupés, aucune action.
- Portée : le clic sur la pilule occupée et la carte qu'il ouvre. Surfaces touchées : none en dehors de la barre Island.
- Incertitude : hauteur de la carte selon le corps — le capteur de contenu des cartes (`PopoverMeasured` → `fit_popover`) la règle après l'ouverture, comme le widget Contrôles.

## Décision de design

- **Island, bulle vivante dans la pilule** : le clic ouvre la vue étendue de *cette* bulle — une carte de la barre ancrée à la pilule, ouverte comme toute carte de la barre (fondu de 180 ms, `transition = "none"` immédiat). Sans bulle vivante, l'horloge garde le calendrier. Autres styles : inchangés.
- **Le calendrier reste atteignable** : une bulle critique occupe la pilule jusqu'à sa fermeture, et la barre peut ne porter que l'horloge. Le pied de la vue offre donc « Calendrier » à côté de « Toutes les notifications », tous deux sous la même ancre.
- **Contenu de la vue étendue** (une colonne, écarts de 12, base 4) : icône de l'application (28 px, `notif_icon`, comme la carte Notifications) ; en face, le nom de l'application (11 px, secondaire) et, à droite, le bouton de fermeture (cible 28 px, info-bulle « Fermer », glyphe du jeu) ; le titre (14 px, texte, sans coupure) ; le corps (12 px, secondaire, huit lignes au plus) ; les actions de l'application, hors `default` (boutons sans bordure, 12 px d'écart, ton « attention » pour une action destructrice, icône d'action : le code de la bulle) ; un filet ; puis « Toutes les notifications », qui ouvre la carte Notifications sous la même ancre.
- Clic sur le bloc de la notification, hors boutons : action `default`, comme la bulle.
- Aperçus masqués (`notifications.hide_previews`, SYS-05) : le mot « Notification » au lieu du corps.
- **La vue est attachée à l'identifiant de la bulle cliquée** (`Kind::Activity(id)`). Une bulle qui arrive ou s'efface pendant qu'elle est ouverte n'en change pas le contenu : la notification se relit dans les bulles vivantes, sinon dans l'historique. Identifiant disparu (historique effacé) : « Aucune notification ».
- **Fermeture** : recliquer sur la pilule (bascule existante), « Fermer », une action invoquée, un clic hors de la carte. « Fermer » referme la bulle si elle vit encore ; passée à l'historique, il ne referme que la vue.
- Aucun fond ni rayon propres : la matière est celle des cartes de la barre. Aucune alerte, aucune lecture : ouvrir la vue ne marque rien comme lu (SYS-02 : seules les listes le font).

Pourquoi cela résout la racine : le clic mène au détail et aux actions de l'activité, et donne le passage vers l'historique, sans nouveau composant.

## Réutilisation

- `Kind`/`Request`/`pop_content`/`card_height` (`popover.rs`) : une sorte de carte de plus.
- `notif_icon`, `card_text`, `action_tone`, `Msg::Toasts(TMsg::Invoke | Close)` (`toasts.rs`) : le contenu de la bulle et de la carte Notifications.
- `module` (`surface.rs`), `pop_separator`, `button::text`, `crate::ui::tokens::ts`, `design::SMALL_RADIUS`.
- Exemplar : `view_toast` (`src/ui/toasts.rs`) pour la tête, le titre, le corps et les actions ; `view_notification_history` pour la carte vide.

Aucune primitive nouvelle.

## Changements

1. `src/ui/plugins/bar/popover.rs`
   - Change : `Request::Activity { id }` et `Kind::Activity(u32)` ; `card_height` : estimation d'ouverture ; `pop_content` : `Kind::Activity(id) => self.view_activity(alpha, id)` ; `PMsg::History` ouvre la carte Notifications sous la clé et la barre de la carte ouverte.
   - Preserve : toutes les cartes existantes, la bascule au recliquement.
   - Verify : `cargo clippy` sans avertissement ; scène `bar-island-activity`.
2. `src/ui/mod.rs`
   - Change : `Request::Activity { id }` donne `PopoverReady(.., Kind::Activity(id))`.
   - Verify : test `Msg::OpenPopover` → `Kind::Activity(1)`.
3. `src/ui/toasts.rs`
   - Change : `activity_notification(id)` (bulle, sinon historique), `activity_open(id)`, `view_activity(alpha, id)` ; `TMsg::Invoke` et `TMsg::Close` referment la vue ouverte de cette bulle.
   - Preserve : bulles, carte Notifications, Silence, état publié.
   - Verify : tests unitaires (relecture bulle puis historique ; action et fermeture referment la vue ; la carte d'une autre bulle reste ouverte).
4. `src/ui/plugins/bar/surface.rs`
   - Change : `clock_press(bar, module, style, toast)` — Island avec bulle : `Request::Activity` ; sinon `Request::Calendar`.
   - Preserve : calendrier de l'horloge dans tous les autres cas ; ovale des non-lues.
   - Verify : test unitaire.
5. `src/ui/preview.rs`, `scripts/verify.sh`, `DESIGN.md` (scènes)
   - Change : scène `bar-island-activity` (bulle avec corps et deux actions, vue ouverte), ajoutée au contrôle visuel.
   - Verify : `scripts/verify.sh --visual /tmp/hig-visual-A`, capture regardée.
6. `locales/en.toml`, `docs/GUIDE.fr.md`, `docs/GUIDE.md`, `DESIGN.md`
   - Change : « Toutes les notifications » ; le paragraphe des notifications dit la vue étendue de la pilule Island.
   - Verify : contrôle des traductions de `scripts/verify.sh`.

## Portée

- Héritent : rien hors de la pilule Island occupée.
- À vérifier : carte Notifications ouverte depuis la vue (même clé : fondu de contenu, bascule au clic) ; bulle critique résidente.
- Exclus : le survol de 250 ms, l'animation de prise de main et la carte « née de la pilule » (phase 2 de `barre-island.md`, « reste à écrire » à `97f427f` ; SYS-23 identité), PAT-02, PAT-07, SYS-03, PAT-20.

## Relève à froid

Relu sans agent indépendant, après vérification du code de `view_popover`.

- **Qu'est-ce qui casse ?** (1) Le plan affirmait une carte « née de la pilule » : rien de tel n'existe à `97f427f` (`view_popover` ne connaît pas le style Island) ; c'est la phase 2 de `barre-island.md`, hors de cet écart : mention corrigée. (2) Une bulle critique, résidente, occupait la pilule sans fin et privait de calendrier une barre sans item Calendrier : « Calendrier » entre au pied de la vue. (3) La même clé pour la vue et la carte Notifications fait ce qu'on veut d'elle : fondu de contenu à l'ouverture, bascule au recliquement.
- **Que manque-t-il ?** Rien de plus après (2) : traduction anglaise, scène d'aperçu, tests et journaux sont au plan.
- **Le plan résout-il l'écart ?** Oui : le clic sur la pilule occupée mène au corps, aux actions et à la carte des notifications, dans les propriétaires existants.

## Relèvements en exécutant

Relu à froid après la première mise en forme et le premier `clippy` (pas d'agent indépendant).

- **Le diff fait-il ce que l'écart demande ?** Oui. Écart au plan, dans son sens : les boutons d'action de la bulle sont sortis en `action_buttons` (`src/ui/toasts.rs`), partagé par la bulle et la vue — même arbre de widgets dans la bulle — plutôt que recopiés. `card_height` ne voit pas la notification : la vue s'ouvre à 176 et le capteur de contenu la règle.
- **`DESIGN.md` et les conventions ?** Écarts 4, 8 et 12 ; cibles de 28 px ; aucun accent (boutons de texte dans la couleur du texte) ; français source, anglais dans `locales/en.toml` ; aucun rayon ni fond propres.
- **Régression, test manquant ?** (1) Recliquer sur la pilule alors qu'une *autre* bulle l'occupe referme la vue au lieu d'ouvrir l'autre : c'est la bascule commune à tous les modules, tenue pour acquise. (2) Une notification transitoire dont la bulle s'efface n'a pas d'entrée d'historique : la vue tombe sur « Aucune notification ». (3) `TMsg::Invoke` et `TMsg::Close` referment la vue seulement pour la bulle de cette vue (`activity_open(id)`), testé.

## Validation

- Produit : notification avec corps et actions en style Island → clic sur la pilule → vue étendue ; « Toutes les notifications » → carte Notifications ; recliquer referme.
- Interface : titre long, corps de huit lignes et plus, aucune action, action destructrice, aperçus masqués, `transition = "none"`, barre en bas, bulle effacée pendant l'ouverture.
- Système : une seule sorte de carte ajoutée, les propriétaires existants fournissent tout le reste.
- Dépôt : `CARGO_BUILD_JOBS=6 bash scripts/verify.sh` → code de sortie 0.

## Conditions d'arrêt

- S'arrêter si `Kind::Activity` exige un propriétaire nouveau (surface, dessin, mouvement) ; si la carte Notifications ne peut pas partager l'ancre de la vue ; si un autre style de barre voit son comportement changer.

## Documentation de design

- Après acceptation : `DESIGN.md` (Island : le clic sur la pilule occupée ouvre la vue étendue ; scène `bar-island-activity`), guides français et anglais.
