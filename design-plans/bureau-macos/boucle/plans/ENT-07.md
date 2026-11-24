# Menus contextuels courts : espaces de l'aperçu, cartes de notification, modules de barre (HIG ENT-07)

Écrit contre : `230127a` (intégration `hig/plan-d-action`), avec le menu de la fenêtre de l'aperçu (MENU-06, même carte)

## Chaîne de preuve

- Surface : l'aperçu des espaces (titre et fond d'une carte d'espace) ; les cartes du centre de notifications (carte « Notifications » de la barre) ; les modules de barre que l'on n'atteint que d'un clic gauche ou du clic milieu (volume, luminosité, horloge et calendrier, module de notifications), tous styles de barre.
- Problème : le clic droit n'y fait rien. `module()` (`src/ui/plugins/bar/surface.rs`, argument `right: Option<Msg>`) reçoit `None` pour le volume, la luminosité, l'horloge-cadran et le calendrier ; le titre de fenêtre aussi. Seuls la zone de notification (menu dbusmenu) et les widgets Lua (`on_click` et `menu`) en ont un. Les cartes de l'historique n'ont pas de `on_right_press` (`src/ui/toasts.rs`, `view_notification_history`) ; ni le titre ni le fond d'une carte d'espace (`src/ui/overview.rs`). Le clic milieu du volume (couper le son) est une commande cachée.
- Preuve de design : `08-entrees.md`, pointing-devices › macOS (« Secondary click — Révéler les menus contextuels »), pointing-devices › Touch support for hidden controls (« A context menu gives people a dependable way to reach a control without finding the spot where it appears. » ; « Include the controls people need most, and keep the menu short. ») ; `03-menus-and-actions.md`, context-menus › Best practices (« Always make context menu items available in the main interface, too. » ; « Hide unavailable menu items, don’t dim them. »), ligne « Menus contextuels du bureau, du dock et des fenêtres : mêmes intitulés et mêmes icônes que dans la barre ».
- Propriétaire : la carte du clic droit (`src/ui/context.rs` : `IconMenu`, `MenuTarget`, `spawn_menu`) pour l'aperçu et les notifications ; la carte des menus de la barre (`src/ui/plugins/bar/popover.rs` : `Kind::Menu`, `MenuSource`), comme « hyprdesk » et « Aide », pour les modules.
- Portée et surfaces touchées : `src/ui/context.rs`, `src/ui/overview.rs`, `src/ui/toasts.rs`, `src/ui/plugins/bar/surface.rs`, `src/ui/plugins/bar/popover.rs`, `src/ui/plugins/bar/menus.rs`, `src/ui/mod.rs` (`popover::Request`), `src/ui/preview.rs` et `scripts/verify.sh` (scènes), les deux guides.
- Incertitude : `toasts.rs` vient d'être relu par le couloir A (SYS-13 : vue étendue du style Island). Ses cartes y passent par la même fonction ou par une autre : à lire avant d'y ajouter le clic droit. La bascule « Ne pas déranger » du module de notifications dépend de ce que `Request::Silence` expose : condition d'arrêt si elle n'est pas réutilisable telle quelle.

## Décision de design

1. **Aucune surface nouvelle.** L'aperçu et les notifications prennent la carte du clic droit, avec deux cibles nouvelles : `MenuTarget::Space(SpaceRef)` et `MenuTarget::Notification(NotificationRef)`. Les modules de barre prennent la carte des menus de la barre, ancrée au module comme ses autres cartes.
2. **Deux à quatre entrées, toutes prises parmi des commandes déjà présentes dans l'interface** (HIG MENU-04 : jamais un menu contextuel seule voie).
   - Carte d'espace (clic droit sur son titre ou son fond) : « Afficher l'espace » (le clic sur le fond), « Renommer » (le double-clic sur le titre). Clic droit sur une fenêtre de la carte : le menu de la fenêtre (MENU-06), inchangé.
   - Carte de notification du centre : « Ouvrir » (le clic : action par défaut, sinon l'application ; masqué quand `card_press` ne rend rien), « Fermer » (`TMsg::Close`).
   - Modules de barre :
     - volume : « Couper le son » ou « Rétablir le son » selon l'état (le clic milieu, `Msg::VolumeMute`), « Réglages du son… » (`Command::Settings("son")`) ;
     - luminosité : « Réglages d'énergie… » (`Command::Settings("energie")`) ;
     - horloge et calendrier : « Réglages de date et d'heure… » (`Command::Settings("date-heure")`) ;
     - titre de fenêtre et module « fenêtre » : le menu de la fenêtre, le même qu'au clic gauche (`Request::Window`), sans entrée nouvelle ni carte nouvelle : le clic droit y fait ce que fait le clic gauche ;
     - module de notifications : « Tout marquer comme lu » (`TMsg::Read`, masqué sans notification non lue), « Ne pas déranger » (bascule, celle du centre de contrôle).
   - Inchangés : débordement (c'est déjà une liste de commandes), widgets Lua (leur fonction `menu`), zone de notification (dbusmenu), module système sans interaction.
3. **Une commande sans effet dans le contexte est masquée, pas grisée** : règle propre aux menus contextuels (HIG context-menus › Best practices). Le menu de la fenêtre fait exception : il reste celui de la barre, entrées inactives comprises.
4. **Le clic droit ne bascule pas la carte du clic gauche.** La carte d'un module porte la clé `"<clé>!menu"` : cliquer le module à gauche ferme le menu et ouvre sa carte, comme aujourd'hui entre deux cartes ; un clic droit sur une carte ouverte la remplace par le menu. Exception : le titre et le module « fenêtre », qui gardent la clé de leur clic gauche (un même contenu, une seule carte : deux clés feraient clignoter la carte d'un clic droit, puis l'un des deux clics ne fermerait plus rien).
5. **Le menu part avec sa surface** : fermer l'aperçu ferme le menu de son espace, comme celui de ses fenêtres (`destroy_overview`) ; une notification fermée par ailleurs ne laisse pas de carte (le message porte l'id : `TMsg::Close` sur un id inconnu ne fait rien).

## Réutilisation

- `IconMenu`, `spawn_menu`, `reopen_icon_menu`, `icon_menu_key` (`src/ui/context.rs`) ; exemplaire : `MenuTarget::Window`, `open_window_menu` (MENU-06).
- `Msg::OpenPopover(bar, clé, Request)` vers `Kind::Menu { entries, source }` ; exemplaires : `Request::Shell`, `menus::entries` et `menus::act` (`src/ui/plugins/bar/menus.rs`), `MenuSource::Window`.
- `card_press`, `card_click` (`src/ui/toasts.rs`) ; `OMsg::Pick`, `OMsg::RenameStart` (`src/ui/overview.rs`) ; `command(Command::Settings(…))` (`src/ui/plugins/bar/popover/continuity.rs`).
- Icônes déjà nommées dans le code : `audio-volume-muted-symbolic`, `audio-volume-high-symbolic`, `preferences-system-notifications-symbolic`, `window-close-symbolic`, `go-jump-symbolic` (à confirmer dans le thème au moment de l'exécution).

## Changements

1. `src/ui/context.rs`
   - Change : `MenuTarget::Space(SpaceRef { card: usize, name: String })`, `MenuTarget::Notification(NotificationRef { id: u32, openable: bool })` ; `CMsg::SpaceShow`, `CMsg::SpaceRename`, `CMsg::NotifOpen`, `CMsg::NotifClose` ; `open_space_menu`, `open_notification_menu` ; les deux cibles comptent pour `over` du capteur ; branches de `icon_menu_title`, `properties`.
   - Preserve : tous les menus existants, le menu de la fenêtre de l'aperçu.
   - Verify : test `menu_d_une_carte_d_espace`, `menu_d_une_carte_de_notification` (entrées, entrée « Ouvrir » absente sans cible).
2. `src/ui/overview.rs`
   - Change : `OMsg::CardContext(usize)` sur `on_right_press` du titre et du fond de carte. Une fenêtre sous le curseur rend la main à `OMsg::Context` (MENU-06).
   - Preserve : glisser-déposer, renommage, choix au clavier.
   - Verify : test unitaire de l'aiguillage (fenêtre sous le curseur ou non).
3. `src/ui/toasts.rs`
   - Change : `on_right_press(Msg::Toasts(TMsg::Menu(id)))` sur les cartes de l'historique ; `toasts_update(TMsg::Menu)` ouvre la carte, dans la surface qui la montre.
   - Preserve : clic de carte, bulles (leurs boutons d'action sont visibles : pas de menu), Silence.
   - Verify : test `clic_droit_d_une_carte_sans_cible_n_ouvre_rien`.
4. `src/ui/plugins/bar/surface.rs`, `popover.rs`, `menus.rs`, `src/ui/mod.rs`
   - Change : `Request::ModuleMenu(String)` ; `MenuSource::Module` ; `module()` reçoit `right` pour volume, luminosité, horloge et calendrier, module de notifications, titre et module « fenêtre » ; `menus::module_entries(clé, état)` et `menus::module_act`.
   - Preserve : clics gauche et milieu, molette, info-bulle, styles de barre, ancrage de la carte.
   - Verify : test `menu_du_module_de_volume_selon_l_etat` ; scènes `bar-menu-module` à 1440, 900 et 640 px.
5. `src/ui/preview.rs`, `scripts/verify.sh`
   - Change : scènes `menu-espace`, `bar-menu-notification`, `bar-menu-module`.
   - Verify : les trois scènes capturées aux trois largeurs.
6. `docs/GUIDE.fr.md`, `docs/GUIDE.md`, `locales/en.toml`
   - Change : une phrase par surface ; traduction anglaise de chaque nouveau libellé (le test `catalogue_complet` y veille).

## Portée

- Héritent : rien hors des surfaces citées ; la carte du clic droit et celle des menus de barre servent aussi leurs menus actuels, sans changement visible.
- À vérifier : les scènes existantes de l'aperçu, des notifications (`notifications`, `bar-notifications`, `bar-island-*`) et de la barre à 640 px.
- Exclus : bulles (toutes leurs commandes sont visibles : actions, fermeture, refus du Silence ; à revoir si l'exécution en trouve une de cachée), toucher et maintenir (gestes du tactile, hors de cet écart), « Fermer l'espace » (aucune commande de fermeture d'espace n'existe par ailleurs : elle enfreindrait MENU-04).

## Validation

- Produit : clic droit sur une carte d'espace, sur une carte du centre, sur le module de volume : la carte attendue s'ouvre au curseur, ou sous le module ; chaque entrée agit comme la commande dont elle vient ; Échap et un clic à côté ferment.
- Interface : les quatre styles de barre ; barre en bas ; carte près du bord de l'écran (-bornée, comme les autres cartes) ; centre vide ou sans non-lue ; aperçu en cours de renommage (pas de menu).
- Système : une seule carte par famille de surface (celle du clic droit, celle de la barre) ; aucun type de menu parallèle.
- Dépôt : `CARGO_BUILD_JOBS=6 bash scripts/verify.sh --visual /tmp/hig-visual-C` → code de sortie 0.

## Conditions d'arrêt

- S'arrêter si `toasts.rs` a changé de propriétaire de vue entre l'écriture de ce plan et son exécution (réécrire le changement 3).
- S'arrêter si une entrée n'a pas de commande équivalente visible ailleurs : elle ne rentre pas dans le menu.
- S'arrêter si la carte d'un module ne peut pas s'ancrer à une clé distincte sans dupliquer `placement` : le menu des modules attend, les deux autres surfaces se font.

## Documentation de design

- Après acceptation : `DESIGN.md`, une phrase à la règle des menus (« un clic droit ouvre un menu court sur tout élément du shell : icône, widget, fenêtre et espace de l'aperçu, carte de notification, module de barre ») ; les deux guides.

## Lecture du plan, à froid

Pas d'agent indépendant disponible ; relu après l'écriture du registre de l'itération C12.

- Qu'est-ce qui casse ? Le titre de fenêtre : ouvert sous une seconde clé, le menu qu'ouvre déjà le clic gauche aurait fait clignoter la carte, puis un des deux clics n'aurait plus rien fermé. Corrigé (décision 4) : même clé, même carte. Ailleurs : `toasts.rs` et `panel.rs` bougent avec les couloirs A et B pendant que ce plan attend son exécution — conflit probable au rebasage, à résoudre par relecture des deux changements, jamais en écrasant.
- Que manque-t-il ? L'état « coupé » du volume, pour choisir entre « Couper le son » et « Rétablir le son » (le modèle le lit déjà pour l'icône du module : à retrouver là) ; la bascule « Ne pas déranger » hors du centre de contrôle (condition d'arrêt) ; la fonction qui montre les cartes de notification dans le style Island (incertitude). Les trois se lèvent en premier, avant d'écrire le code.
- Le plan résout-il l'écart ? Oui pour les trois surfaces de la colonne Correction : chaque carte d'espace, chaque carte de notification du centre et chaque module cité répond au clic droit par un menu court, dont les commandes existent déjà ailleurs. La bulle, citée par la preuve, reste sans menu : toutes ses commandes sont visibles (actions, fermeture, refus du Silence) ; à noter en remarque du registre à l'exécution, et à corriger si une commande cachée s'y trouve.
