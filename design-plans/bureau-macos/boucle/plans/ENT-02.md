# ENT-02 — Le clavier va à chaque surface du shell : barre, dock, centre de notifications, volet des Réglages

Écrit contre : `4107de3` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : la session hors plein bureau (no-desktop, lanceur seul) ; plein bureau aussi pour la barre et le centre.
- Problème : la souris seule atteint la barre, le centre de notifications et le volet des Réglages ; le dock ne prend le clavier qu'en changeant de mode.
- Preuve de design : HIG keyboards › Standard keyboard shortcuts — « Control-F2 — Move focus to the menu bar », « Control-F3 — Move focus to the Dock », « Control-F6 — Move focus to the window » ; « People expect each of the following standard keyboard shortcuts to perform the action listed ».
- Recensement à HEAD (`4107de3`) : `Keys` (`src/config/model.rs`) n'a aucun raccourci de focus. La barre naît sans clavier (`src/ui/plugins/bar/surface.rs:700`, `KeyboardInteractivity::None`). Le dock : `launcher_grab_focused` (`src/ui/plugins/dock/keyboard.rs:109`), appelée par `ToggleLauncher`, `LauncherOnly` et `Cycle` (`src/ui/mod.rs:3918-3945`) après `show_with_windows()`. Tab ne descend dans le dock que sur le plein bureau (`mod.rs:5562`). Le volet a le clavier à la demande (`src/ui/panel.rs:389`) ; `keys.settings` l'ouvre ou le referme (`Command::ToggleSettings`). Les surfaces du centre de notifications naissent sans clavier (`src/ui/toasts.rs:1090`, `1159`). Les lignes du plan (relevées à `00f6d69`) ont bougé de quelques unités : preuve exacte.
- Propriétaires : `Keys` et `keymap::actions` (`src/keymap.rs`), `control::Command` (`src/control.rs`), le bloc `Msg::Control` de `src/ui/mod.rs`, `launcher_grab` (`dock/keyboard.rs`), `open_menu_by_type` et `Msg::OpenPopover` (`src/ui/mod.rs:2368`, `:2442`), l'enveloppe de focus `src/ui/plugins/bar/popover/continuity/keyboard.rs`.
- Périmètre et surfaces touchées : les douze fichiers que touche un raccourci de `Keys` (empreinte de `keys.overview` : `keymap.rs`, `hypr.rs`, `settings.rs`, `cli_config.rs`, `control.rs`, `config/lua.rs`, `ui/key_conflict.rs`, `ui/preview.rs`, `ui/panel.rs`, `ui/plugins/keys.rs`, `ui/plugins/mod.rs`), `src/ui/mod.rs`, `src/ui/plugins/bar/surface.rs`, `src/ui/plugins/bar/popover.rs`, `locales/en.toml`, `docs/GUIDE.fr.md`, `docs/GUIDE.md`, `DESIGN.md`.
- Incertitude : macOS n'a pas de raccourci pour le centre de notifications, et les touches F2, F3, F6, F8 peuvent demander la touche Fn d'un portable. Décisions ci-dessous.

## Décision de design

- Quatre raccourcis de `Keys`, sur le modèle de macOS (CONTROL joue le rôle de Control ; cf. `keys.settings` pour la portée « partout ») :

  | Clé Lua | Défaut | Action |
  | --- | --- | --- |
  | `keys.focus_bar` | `CTRL + F2` | Le clavier va à la barre de l'écran actif |
  | `keys.focus_dock` | `CTRL + F3` | Le clavier va au dock de l'écran actif |
  | `keys.focus_notifications` | `CTRL + F8` | Le centre de notifications s'ouvre et prend le clavier |
  | `keys.focus_settings` | `CTRL + F6` | Le volet des Réglages s'ouvre s'il est fermé, et prend le clavier |

  `CTRL + F8` : libre, et voisin des menus d'état de la barre, qui ouvrent le centre sur macOS.
- Aucun ne masque rien et aucun ne bascule, contrairement à `keys.settings`, qui referme le volet. Pressé deux fois : la barre ou le dock rend le clavier aux fenêtres ; le centre et le volet, qui restent ouverts, le gardent.
- Une seule commande : `hyprdesk focus <barre|dock|notifications|settings>` → `Command::Focus(String)` ; type inconnu : rien.
- Dock : bureau masqué → le dock paraît sous les fenêtres, en place, et prend le clavier (le chemin de `ToggleLauncher`, sans sa bascule : `show_with_windows()` puis `launcher_grab_focused()`) ; plein bureau → `launcher_grab(écran, false)` comme Tab ; lanceur seul → `launcher_grab(écran, true)`. Le dock tient déjà le clavier : il le rend (le chemin d'Échap dans le dock).
- Barre : la surface de la barre de l'écran actif prend le clavier (`KeyboardInteractivityChange { id, Exclusive }`, comme le dock déjà affiché à `dock/keyboard.rs:157-161`). Les modules activables sont enveloppés par l'enveloppe de focus existante (`continuity/keyboard.rs::press`) : anneau de focus, Entrée ou Espace active. Flèches, Tab et Maj + Tab passent d'un module au suivant (`focus_next`, `focus_previous`, bornés à la fenêtre de la barre) ; Échap rend le clavier. L'ouverture d'un menu rend le clavier à la barre.
- Module activable : celui qu'ouvre son clic gauche. Activation = le message de son clic : `Msg::OpenPopover(barre, clé, requête)`, la requête étant celle que construit déjà `open_menu_by_type` (workspaces, volume, luminosité, zone de notification, contrôles, horloge et calendrier, widget Lua à menu), plus le centre de notifications et, s'ils partagent ce chemin, les quatre menus de l'en-tête (décision D1). Un module sans ce chemin (batterie, réseau, média, widget Lua sans menu) reste hors du parcours : il ne reçoit pas l'enveloppe.
- Centre de notifications : `open_popover(barre, "notifications", Kind::Notifications)`, comme le clic du module ; la surface du popover naît avec `KeyboardInteractivity::Exclusive` (le chemin du champ de recherche, `popover.rs:1356`) et le premier contrôle focusable prend le focus (`focus_next` borné au popover). Recenser les contrôles du centre n'est pas ici : c'est ENT-01.
- Volet des Réglages : `open_panel()` s'il est fermé, puis `focus_next` borné au volet (premier champ). Ouvert, il ne se referme pas.

## Réutilisation

- `Keys`, `keymap::actions`, les deux tables de pose de `src/hypr.rs` (lignes 704 et 795) : un raccourci de plus, rien d'autre.
- `launcher_grab` et `launcher_grab_focused` (`src/ui/plugins/dock/keyboard.rs`) : le dock, tel quel.
- `continuity/keyboard.rs::press` : l'anneau de focus et l'activation par Entrée ou Espace de la barre, les mêmes que dans le centre de contrôle (HIG ENT-07, ENT-11).
- `open_menu_by_type` (`src/ui/mod.rs:2368`) : la table type → requête, extraite en `fn request_of(&self, barre, clé) -> Option<Request>` pour servir au clic comme au clavier.
- Aucune primitive nouvelle : ni widget, ni opération, ni token.

## Changements

1. `src/config/model.rs`, `src/config/lua.rs`, `src/settings.rs`, `src/cli_config.rs`, `src/ui/key_conflict.rs`
   - Change : quatre champs de `Keys` (`focus_bar`, `focus_dock`, `focus_notifications`, `focus_settings`), lus en Lua sous `keys.focus_*`, avec leurs défauts, lus et écrits en Lua (`keys.focus_bar = "CTRL + F2"`, `none` pour couper), exposés par `hyprdesk config`, pris en compte par `key_conflict.rs` — sur le modèle ligne à ligne de `keys.settings`.
   - Preserve : les options existantes ; un champ vide ne pose aucun raccourci.
   - Verify : `hyprdesk config get keys.focus_dock` rend `CTRL + F3` ; `keys.focus_dock = "none"` then le retire des raccourcis actifs.
2. `src/keymap.rs`, `src/hypr.rs`
   - Change : quatre libellés (« Clavier à la barre », « Clavier au dock », « Clavier au centre de notifications », « Clavier aux Réglages »), quatre actions (`focus-bar`, `focus-dock`, `focus-notifications`, `focus-settings`), portée partout ; posées par `exec_cmd("'/bin/hyprdesk' focus <type>")`, dans les deux tables (hors bureau et submap).
   - Preserve : les indices de `LABELS` : les portées y sont citées par numéro, les quatre libellés vont dans un second tableau `FOCUS_LABELS`, chaîné à `LABELS` là où se vérifie la traduction.
   - Verify : test comme `hypr.rs:2774` : le Lua généré contient les quatre `keep(hl.bind(…))`.
3. `src/control.rs`, `src/ui/mod.rs` (bloc `Msg::Control`)
   - Change : `Command::Focus(String)`, lue par `"focus <type>"` ; `Focus("dock")` selon le mode (décision ci-dessus) ; `Focus("bar")` → `bar_grab_focused()` ; `Focus("notifications")` → ouverture du centre avec clavier ; `Focus("settings")` → `open_panel()` puis `focus_next`.
   - Preserve : `keys.launcher`, `keys.settings`, `Command::Menu` inchangés.
   - Verify : tests d'état de `App` sur le modèle de `alt_tab_in_full_desktop` (`mod.rs:6859`) : dans chacun des trois modes, `Focus("dock")` ne change `DesktopMode` que masqué → lanceur seul ; `Focus("settings")` sur un volet ouvert le laisse ouvert.
4. `src/ui/mod.rs`, `src/ui/plugins/bar/surface.rs`
   - Change : `bar_grab_focused(&mut self)` : la barre de l'écran actif reçoit `KeyboardInteractivityChange { id, Exclusive }` et `focus_next` borné à sa fenêtre ; un champ `bar_keyboard: Option<Id>` dit quelle barre tient le clavier ; Échap sur cette fenêtre rend le clavier (`None`, champ effacé) ; flèches gauche et droite (haut et bas si la barre est verticale, `bar_vertical`) → `focus_previous` et `focus_next`. Dans la vue des modules (`surface.rs:2379`, `motion::anchor`), un module dont `request_of` rend une requête est enveloppé : `keyboard::press(module, Some(Msg::OpenPopover(..)), encre)`.
   - Preserve : la barre ne prend jamais le clavier d'elle-même : `KeyboardInteractivity::None` à la naissance, le clic ouvre les menus comme avant.
   - Verify : une barre de test (volume, horloge, batterie) : deux modules enveloppés, la batterie non ; `Focus("bar")` pose le focus sur le volume ; flèche droite → horloge ; Entrée → `Msg::OpenPopover(barre, identifiant de l'horloge, Request::Calendar)` ; Échap → clavier rendu.
5. `src/ui/plugins/bar/popover.rs`
   - Change : un drapeau `Popover.keyboard` : la surface du popover naît `Exclusive` et le premier contrôle prend le focus ; posé par `Focus("notifications")` seulement.
   - Preserve : les popovers ouverts au clic naissent sans clavier, sauf ceux qui en ont déjà besoin (recherche, mot de passe).
   - Verify : test sur le modèle de `popover.rs:338` : après `Focus("notifications")`, `popover.keyboard` vaut vrai.
6. `src/ui/plugins/keys.rs`, `src/ui/plugins/mod.rs`, `src/ui/panel.rs`, `src/ui/preview.rs`, `locales/en.toml`
   - Change : les quatre raccourcis dans la page Raccourcis de hyprdesk (rangées, capture, par défaut), dans l'onglet qui porte les autres (« ensemble »), les tests de `panel.rs:442` et de l'aperçu ; libellés anglais.
   - Preserve : le reste de la page.
   - Verify : `cargo test --locked --lib keys::` ; `libelles_traduits` de `keymap` reste vert.
7. `docs/GUIDE.fr.md`, `docs/GUIDE.md`, `DESIGN.md`
   - Change : un paragraphe « Le clavier à chaque surface » après « Navigation au clavier dans le centre de contrôle » ; dans `DESIGN.md`, la décision des quatre raccourcis.
   - Verify : les deux guides disent les mêmes quatre combinaisons.

## Ordre

Recenser d'abord les chemins d'ouverture (menus de l'en-tête, modules d'`open_menu_by_type`), puis 1 → 2 → 3 → 5 → 6 → 7. Ce premier lot — le pipeline, le dock, le centre et le volet — se publie seul s'il passe la porte : la ligne du registre reste `à faire`, remarque « lot 1 publié (hash) ; reste la barre ». Le second lot, 4 puis la fin de 6 (rangées de la barre) et de 7, passe la ligne `fait`.

Un écart tenu en deux lots a un précédent : ENT-07, itérations C13 et C14.

## Périmètre

- Héritent : la page Raccourcis des Réglages, `hyprdesk keys`, `hyprdesk config`, `binds.lua`, l'inventaire des raccourcis (`essentials::inventory`).
- À vérifier : `alt_tab_in_full_desktop` (Tab sur le plein bureau), les scènes d'aperçu `raccourcis`, `bar-controls`, `notifications`, `reglages`.
- Hors périmètre : les contrôles du centre de notifications (ENT-01), les gestes (ENT-03), le focus des fenêtres d'application (Hyprland), les modules sans chemin d'ouverture par type.

## Validation

- Produit : `CTRL + F3` bureau masqué → dock visible sous les fenêtres, première case choisie, flèches et Entrée marchent ; `CTRL + F2` → anneau sur le premier module de la barre ; `CTRL + F8` → centre ouvert, Tab circule, Échap ferme ; `CTRL + F6` → volet ouvert, premier champ focus.
- Interface : barre horizontale et verticale, plein bureau et no-desktop, un écran et deux (écran actif).
- Système : un seul chemin d'ouverture par type de module (le clic et le clavier passent par `request_of`) ; aucune enveloppe de focus parallèle.
- Dépôt : `CARGO_BUILD_JOBS=6 bash scripts/verify.sh --visual /tmp/hig-visual-C` → code de sortie 0.

## Conditions d'arrêt

- Arrêter si Hyprland refuse de donner le clavier à une surface de barre existante (`Exclusive` à chaud) : la barre devrait renaître comme le dock à la demande — trop lourd pour ce lot, marquer « bloqué » pour la barre et publier le reste.
- Arrêter si les menus de l'en-tête (D1) exigent leur propre chemin d'activation : les laisser hors parcours, le noter.

## Documentation de design

- Après acceptation : dans `DESIGN.md`, la règle « Chaque surface du shell a son raccourci de clavier : barre CONTROL+F2, dock CONTROL+F3, centre de notifications CONTROL+F8, volet des Réglages CONTROL+F6. Aucun ne referme ni ne bascule : pressé deux fois, la barre ou le dock rend le clavier, le centre et le volet le gardent ».

## Relecture à froid (sans outil d'agent)

Relu après l'inscription au registre, à `4107de3`.

- Qu'est-ce qui casse ? Deux incohérences de la première version : « presser deux fois rend le clavier » contredisait « le dock tient déjà le clavier : rien », et l'ordre des étapes publiait un lot partiel sans dire ce que devient la ligne du registre. Toutes deux tranchées ci-dessus. Risque réel qui reste : Hyprland peut refuser `Exclusive` à chaud sur une surface de barre existante — le dock déjà affiché l'obtient ainsi (`dock/keyboard.rs:157-161`), mais une barre n'a jamais pris le clavier ; c'est la première condition d'arrêt, et elle ne concerne que le second lot.
- Que manque-t-il ? La façon dont les quatre menus de l'en-tête (D1) s'ouvrent n'a pas été lue : le plan dit quoi faire dans les deux cas (chemin commun, ou hors parcours noté au registre). La liste des modules que `open_menu_by_type` sait ouvrir est close par le code lu (`mod.rs:2397-2437`) : batterie, réseau, média restent hors parcours, dit tel quel.
- Le plan résout-il l'écart ? Oui : quatre raccourcis de `Keys`, un par surface, qui posent le focus sans bascule de mode, avec la barre en second lot ; l'écart ne passe `fait` qu'après la barre.
