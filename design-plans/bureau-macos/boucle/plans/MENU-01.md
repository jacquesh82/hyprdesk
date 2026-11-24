# Barre des menus : « hyprdesk » toujours en tête, puis Fenêtre et Espaces (HIG MENU-01)

Écrit contre : `cae8c8c` (intégration `hig/plan-d-action`, arbre `904a00a`)

## Chaîne de preuve

- Surface : la barre de hyprdesk, tous styles, zone amont (gauche en haut ou en bas, haut d'une barre verticale).
- Problème : aucun menu de premier niveau. Les commandes du shell sont dispersées : clic sur le titre de la fenêtre (`src/ui/plugins/bar/window.rs`), carte des espaces (`popover/bar_menus.rs`), centre de contrôle (`popover/continuity.rs`), bouton Réglages du dock, clics droits. Mise en veille, redémarrage et extinction n'existent que sur l'écran de connexion et dans Panorama.
- Preuve de design : `03-menus-and-actions.md`, the-menu-bar › Anatomie et cotes (« When present in the menu bar, the following menus appear in the order listed below » ; le menu Apple « always the first item on the leading side »), App-specific menus (« Even when commands are available elsewhere in your app, it's important to list them in the menu bar »), Best practices (« Always show the same set of menu items »). Décision D1 du propriétaire : « Un menu « hyprdesk » en tête de la barre, puis les menus Fenêtre, Espaces et Aide ».
- Propriétaire : `src/ui/plugins/bar/surface.rs` (composition de la barre, `App::module`), `src/ui/plugins/bar/popover.rs` (`Request`, `Kind::Menu`, `MenuSource`, `pop_menu`), `src/ui/plugins/bar/layout.rs` (bandeau et débordement), module nouveau `src/ui/plugins/bar/menus.rs` (contenu et actions des menus du shell).
- Portée et surfaces touchées : barre de tous les styles ; cartes ouvertes depuis la barre ; scènes d'aperçu « bar » (`src/ui/preview.rs`).
- Incertitude : les index plats du bandeau (aperçu `bar_preview`, modules débordés `bar_hidden`) sont décalés par les titres ; à compenser à chaque endroit qui les énumère (étape 1, vérification par tests).

## Décision de design

La barre gagne des titres de menus, rendus par elle-même avant le premier module de la zone amont, dans l'ordre fixe « hyprdesk », « Fenêtre », « Espaces ». Ce ne sont pas des modules : ils n'entrent ni dans `BarItem`, ni dans `BAR_ITEMS`, ni dans les zones de `hd.bar` ; on ne peut donc ni les retirer, ni les déplacer, ni les voir partir dans le débordement. « hyprdesk » rassemble les commandes du shell : À propos de hyprdesk ; Réglages… ; Verrouiller ; Espaces de travail, Organiser les fenêtres… ; Mise en veille…, Redémarrer…, Éteindre…. « Fenêtre » et « Espaces » ouvrent les menus qui existent déjà ailleurs (clic sur le titre, carte des espaces) : la HIG demande de les recenser aussi dans la barre, pas de les déplacer. Les commandes restent là où elles sont.

Cela résout la cause : les commandes du shell ont un endroit unique, toujours premier, et l'ensemble des menus ne varie pas avec la configuration des modules.

## Réutilisation

- `App::module` (bouton d'un module de barre, tous styles), `motion::anchor` (ancre de carte), `bar_menu_open` (état ouvert) ; exemplaire : `BarItem::Window`, `src/ui/plugins/bar/surface.rs`.
- `Msg::OpenPopover`, `Msg::PopoverReady`, `popover::Kind::Menu`, `pop_menu` (lignes, séparateurs, confirmation) ; exemplaire : `Request::Window`, `src/ui/mod.rs`.
- `MenuEntry`, `crate::i18n::tr`, `locales/en.toml`.
- `control::Command::Settings`, `Command::Overview`, `Msg::BarOrganize`, `run_from_desktop` (`hyprdesk lock`) ; `systemctl suspend|reboot|poweroff`, comme l'écran de connexion (`src/greeter_ui.rs`).
- `layout::strip` et `layout::fit` : priorité de module.

Aucune primitive nouvelle. Le module `menus.rs` ne contient que des données (entrées, titres) et la correspondance indice → message.

## Étapes

1. **Menu « hyprdesk ».** `Request::Shell` et `MenuSource::Shell` ; `menus.rs` (entrées, actions, titre) ; le titre rendu en tête de la zone amont dans les deux branches de mise en page de `surface.rs` (emplacements, `bar_zone`) ; priorité `layout::PINNED`, jamais débordée ; index plats compensés (`bar_overflow_items`, `bar_hidden`, `preview.rs`). Vérifiable : test unitaire des entrées et des actions ; `scripts/verify.sh`.
2. **Menus titrés « Fenêtre » et « Espaces »**, à la suite : ils ouvrent `Request::Window` et `Request::Workspaces`. Même mécanisme, deux titres de plus. Vérifiable : test de l'ordre et des requêtes des titres ; `scripts/verify.sh`.
3. **Écran d'aperçu et état final** : une scène d'aperçu du menu « hyprdesk » ouvert (`src/ui/preview.rs`), pour que son rendu soit vérifiable sans session ; `scripts/verify.sh --visual` ; l'écart passe `fait`. Vérifiable : la scène existe, la capture se regarde.

## Avancement des étapes

- Étape 1 : faite (itération C5). Le groupe de puissance ne porte pas d'icône, faute d'icône sûre pour la mise en veille (un groupe en porte à toutes ses lignes ou à aucune).
- Étape 2 : faite (itération C5). Sans fenêtre active, « Fenêtre » reste affiché et inactif.
- Documentation faite (itération C5) : `docs/GUIDE.fr.md`, `docs/GUIDE.md`, `DESIGN.md`, `locales/en.toml`.
- Étape 3 : à faire.

## Changements (étape 1)

1. `src/ui/plugins/bar/menus.rs` (nouveau)
   - Change : `TITLES` (clé d'ancre, libellé, `Request`), dans l'ordre de la barre ; `entries()` → `Vec<MenuEntry>` du menu « hyprdesk » ; `act(action)` → `Task<Msg>`.
   - Preserve : les commandes existantes gardent leur emplacement d'origine.
   - Verify : test unitaire (ordre, séparateurs, confirmation sur redémarrer et éteindre uniquement).
2. `src/ui/plugins/bar/popover.rs`
   - Change : `Request::Shell`, `MenuSource::Shell` ; `PMsg::Confirm` les traite.
   - Preserve : Lua, zone de notification, fenêtre.
   - Verify : compilation ; les menus existants passent toujours leurs tests.
3. `src/ui/mod.rs`
   - Change : `Request::Shell` donne `Kind::Menu { entries: menus::entries(), source: MenuSource::Shell }` sans lecture asynchrone ; `bar_hidden` reçoit des index comptant les titres.
   - Verify : test de `bar_overflow_items` avec un module débordé.
4. `src/ui/plugins/bar/surface.rs`
   - Change : `bar_menu_title` rend un titre avec `module` + `anchor` ; les titres sont insérés en tête de la zone amont dans la branche à emplacements et dans `bar_zone`.
   - Preserve : catalogues, glisser-déposer des modules, autres styles.
   - Verify : `scripts/verify.sh --visual`, scène « bar ».
5. `src/ui/plugins/bar/layout.rs`
   - Change : `PINNED` : un emplacement de cette priorité n'est jamais masqué par `fit`.
   - Verify : test de `fit`.
6. `src/ui/preview.rs`, `locales/en.toml`
   - Change : index d'aperçu décalés du nombre de titres ; traductions anglaises.
   - Verify : `scripts/verify.sh --visual`.

## Relecture à froid (sans agent indépendant)

- **Qu'est-ce qui casse ?** Les index plats du bandeau : `bar_preview` (index 6 en dur dans `preview.rs`), `bar_hidden` (rapport de `layout.rs`), `bar_overflow_items` (énumère les modules seuls) et `eligible` (un drapeau par enfant du rail). Chacun compte les titres ou est compensé ; un test couvre le débordement. Les clefs d'ancre des titres portent le préfixe `menus:`, qu'aucun module ne porte ; l'animation de pression de `motion::anchor` est gardée, comme sur la plupart des modules.
- **Que manque-t-il ?** Aucune fonction de la session ne met en veille, ne redémarre ni n'éteint : `menus.rs` lance `systemctl` comme l'écran de connexion, sur un fil bloqué, et signale l'échec dans le journal sans être bloquant. Les chaînes nouvelles vont dans `locales/en.toml`.
- **Le plan résout-il l'écart ?** Oui : « hyprdesk » est toujours premier et non retirable, il rassemble les commandes du shell, et les menus de la fenêtre et des espaces le suivent comme menus titrés. La corbeille et « Fermer les N fenêtres » restent à MENU-04, le menu Aide à MENU-05.

## Portée

- Héritent : tous les styles de barre, les barres verticales, le mode réglage (titres visibles, non déplaçables).
- À vérifier : débordement (« Autres commandes »), aperçu d'un module dans les Réglages, cartes ancrées.
- Exclu : menu Aide (MENU-05), sous-menus (MENU-03), corbeille et « Fermer les N fenêtres » (MENU-04), nom de l'application en gras (MENU-07), focus clavier de la barre (ENT-02).

## Validation

- Produit : un clic sur « hyprdesk » ouvre son menu ; « Réglages… » ouvre les Réglages ; « Verrouiller » verrouille ; « Espaces de travail » ouvre la vue d'ensemble ; redémarrer et éteindre demandent confirmation.
- Interface : trois largeurs d'aperçu (1440, 900, 640) ; barre pleine (débordement actif) ; mode réglage.
- Système : aucune primitive parallèle ; les menus existants sont inchangés.
- Dépôt : `CARGO_BUILD_JOBS=6 bash scripts/verify.sh` → code de sortie 0.

## Conditions d'arrêt

- S'arrêter si les index plats du bandeau sont énumérés ailleurs que dans les endroits compensés (un décalage apparaîtrait) ; si `module` ne rend pas correctement un titre sans icône dans un style.

## Documentation de design

- Après acceptation : `DESIGN.md` — la barre commence par ses menus, « hyprdesk » toujours premier ; `docs/GUIDE.fr.md` et `docs/GUIDE.md` — où trouver les commandes du shell.
