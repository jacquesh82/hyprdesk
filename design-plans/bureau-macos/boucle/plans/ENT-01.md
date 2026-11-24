# ENT-01 — Accès complet au clavier : chaque commande du tiroir et du centre prend le focus par Tab

Écrit contre : `90c881b` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : le tiroir des Réglages (toutes ses pages), le centre de contrôle et de notifications (popover de la barre), la barre (faite en ENT-02).
- Problème : hors des champs de saisie, aucune commande du tiroir ne prend le focus. Tab y appelle `iced::widget::operation::focus_next()` sans borne (`src/ui/mod.rs`, bras « Le panneau a ses propres champs de saisie », vers la ligne 5766) : seuls les `text_input` sont focalisables. Les boutons, interrupteurs, listes et curseurs natifs d'iced ne le sont pas (`src/ui/plugins/bar/popover/continuity/keyboard.rs:1`). Seul le centre de contrôle enveloppe ses commandes (`keyboard::press` et `keyboard::adjust`, neuf appels dans `continuity.rs`), et la barre ses modules (ENT-02). Aucun mode d'accès clavier, aucun raccourci pour le basculer.
- Preuve de design : HIG keyboards › Best practices, « Support Full Keyboard Access, when possible » ; focus-and-selection › Best practices, « a full keyboard access mode helps people use the keyboard to reach every control » ; keyboards › Standard keyboard shortcuts, Control-F1 bascule l'accès clavier complet.
- Recensement à HEAD (`90c881b`) dans les plugins des Réglages (hors barre, dock, Panorama, lanceur) : 68 `toggler(`, 221 `button(`, 32 `pick_list(`, 34 `slider(`, 31 `text_input(`. Le composant `App::field` (`src/ui/panel.rs:1594`) et le curseur `App::slider` (`panel.rs:1633`, HIG SEL-08) sont partagés ; `field` reçoit souvent une rangée composée (plusieurs commandes), il ne peut pas envelopper son contrôle d'un bloc.
- Propriétaires : l'enveloppe de focus (`continuity/keyboard.rs` : `Keyboard`, `Action::Press`, `Action::Adjust`, `step`, l'anneau) ; `App::panel_focus_step` et `panel_focus_first` (`panel.rs:563-578`, bornés au volet, ENT-02) ; `Keys`, `keymap::actions`, `control::Command` ; l'OSD (`src/ui/osd.rs`).
- Périmètre : le tiroir, le centre ; la barre a déjà son parcours (ENT-02) ; les bulles (`toasts.rs:1260`, `1329`, `KeyboardInteractivity::None`) à la dernière étape.
- Incertitude : (1) une activation simulée (appui puis relâchement du bouton gauche au centre du contrôle) suffit-elle pour `button`, `toggler`, `checkbox`, `radio`, `pick_list` d'iced 0.14 ? À vérifier par un test d'état dès l'étape 1, condition d'arrêt. (2) Le menu ouvert d'un `pick_list` se pilote-t-il aux flèches et à Entrée dans iced 0.14.2 ? À lire dans `iced_widget-0.14.2/src/overlay/menu.rs` avant l'étape 3.

## Décision de design

- **Un mode d'accès clavier complet**, `general.keyboard_access` (booléen, **vrai par défaut** : une fonction livrée se voit sans réglage, et l'anneau ne paraît qu'au focus clavier), basculé par `keys.keyboard_access` = `CTRL + F1` (modèle Control-F1) et `hyprdesk keyboard-access` ; l'OSD dit « Accès clavier complet : activé » ou « désactivé ». Réglable dans **Réglages → Clavier et souris**.
  - Activé : Tab et Maj + Tab parcourent toutes les commandes du tiroir, dans l'ordre de lecture ; Entrée ou Espace active ; les flèches règlent un curseur.
  - Désactivé : Tab ne va qu'aux champs de saisie, comme aujourd'hui.
  - La barre (ENT-02) et le centre de contrôle (ENT-07, ENT-11) gardent leur parcours dans les deux cas : ils sont atteints par un raccourci explicite.
- **L'enveloppe se généralise, elle ne se duplique pas** : `continuity/keyboard.rs` devient le propriétaire partagé `src/ui/focus.rs` (même `Keyboard`, même anneau, mêmes `press`, `adjust`, `step`), et gagne `Action::Click` : Entrée ou Espace envoie à l'enfant un appui puis un relâchement du bouton gauche au centre de ses bornes. Une seule enveloppe sert ainsi toute commande native sans connaître son message.
  - `focus::click(child, ring, on)` : `on` faux (mode désactivé) rend l'enfant tel quel.
  - Pour les sites qui connaissent leur message, `press` reste préféré (pas de clic simulé).
- **Les commandes du tiroir s'enveloppent à la feuille**, par un helper d'`App` : `self.focusable(control)` = `focus::click(control, anneau du tiroir, self.cfg.general.keyboard_access)`. Le curseur partagé `App::slider` prend `adjust` (flèches, pas du réglage). Un champ de saisie ne s'enveloppe jamais : il est déjà focalisable.
- **Tab du tiroir borné au volet** : le bras de `mod.rs` appelle `panel_focus_step(shift)` au lieu de `focus_next()` global.
- **Centre de notifications** : les commandes des cartes (actions, fermeture, menu) et « Effacer l'historique » s'enveloppent comme celles du centre de contrôle ; Tab borné au popover.

## Réutilisation

- `continuity/keyboard.rs` : `Keyboard`, `Focus`, l'anneau, `step` ; l'exemple d'`adjust` dans `continuity.rs:879`.
- `App::panel_focus_step` (borne au volet), `App::field`, `App::slider`.
- Le modèle ligne à ligne d'un raccourci de `Keys` : `keys.force_quit` (ENT-10) et `keys.focus_bar` (ENT-02) — douze fichiers.
- `show_osd` pour l'annonce du mode.
- Aucune primitive nouvelle : l'enveloppe existante change de place et gagne une action.

## Changements

1. `src/ui/focus.rs` (déplacé de `continuity/keyboard.rs`), `continuity.rs`, `bar/surface.rs`
   - Change : le module déplacé ; `Action::Click` ; `focus::click`.
   - Preserve : `press`, `adjust`, `step`, `GROUPS`, Ctrl+Tab (ENT-11), PageUp/PageDown du centre ; les appels existants ne changent que de chemin.
   - Verify : test d'état — un `button` enveloppé, Entrée avec le focus, publie son message ; un `toggler` aussi ; sans focus, rien.
2. `src/config/model.rs`, `src/config/lua.rs`, `src/settings.rs`, `src/cli_config.rs`, `src/defaults.rs`, `assets/hyprdesk.meta*.lua`, `src/keymap.rs`, `src/hypr.rs`, `src/control.rs`, `src/ui/key_conflict.rs`, `src/ui/plugins/keys.rs`, `src/ui/panel.rs`
   - Change : `general.keyboard_access` ; `keys.keyboard_access` (`CTRL + F1`, partout) ; `Command::KeyboardAccess` (`keyboard-access`) ; bascule + OSD ; l'interrupteur dans Clavier et souris.
   - Preserve : les indices de `State::keys` (case 16 ajoutée à la suite) ; `keys.focus_*`.
   - Verify : `hyprdesk config get keys.keyboard_access` rend `CTRL + F1` ; le raccourci posé dans les deux tables ; la bascule change le réglage et montre l'OSD.
3. `src/ui/mod.rs`, `src/ui/panel.rs`
   - Change : Tab du tiroir par `panel_focus_step` ; `App::focusable` ; `App::slider` enveloppé par `adjust` ; les commandes communes du tiroir (en-tête, navigation des pages, triangles des sections avancées).
   - Verify : test — mode activé, deux Tab depuis l'ouverture vont à la deuxième commande du volet ; désactivé, au deuxième champ de saisie.
4. Les plugins des Réglages, par paquets (étapes 2 à 4) : chaque `toggler`, `button`, `pick_list` hors champ de saisie passe par `self.focusable(…)`.
5. `continuity.rs` (cartes de notification, « Effacer l'historique »).
6. Documentation : `docs/GUIDE.fr.md`, `docs/GUIDE.md`, `docs/CONTROL.md`, `docs/PLUGINS*.md`, `README*.md`, `DESIGN.md`, `locales/en.toml` ; scènes d'aperçu `acces-clavier` (anneau sur une commande du tiroir) et `acces-clavier-centre`.

## Étapes

1. **Fait (C20).** L'enveloppe partagée : `src/ui/focus.rs` (déplacée de `continuity/keyboard.rs`, qui garde le périmètre et les groupes du centre), générique sur le moteur de rendu pour être testée avec le moteur nul `()` ; `Action::Click`, chaque événement simulé dans son propre `Shell` (sinon l'appui capturé masque le relâchement au bouton) ; `adjust_by` (domaine et pas quelconques) ; `overlay` transmis à l'enfant (menu d'une liste, infobulle) ; un clic de souris retire l'anneau, une touche déjà capturée est ignorée. `App::focusable` ; Tab du tiroir borné au volet (la conversation garde son parcours) ; les commandes de l'en-tête du tiroir enveloppées. Tests : Entrée active un bouton et un interrupteur, une fois ; sans focus, rien ; un clic retire l'anneau.
2. **Fait (C21).** Le mode : `general.keyboard_access`, `keys.keyboard_access` (`CTRL + F1`), `hyprdesk keyboard-access`, OSD, interrupteur dans Clavier et souris ; `App::focusable` le consulte. Documentation de ce qui est livré.
3. Le parcours du tiroir : défilement jusqu'à la commande focalisée (opération en deux passes : bornes du focus et translation du `scrollable`, puis `scroll_to` ; le `scrollable` du contenu reçoit un id) ; deux groupes (navigation, contenu) et Ctrl+Tab de l'un à l'autre — `jump` prend ses ids en paramètre au lieu de `GROUPS` ; la navigation des pages enveloppée ; le curseur partagé `App::slider` : seul le `slider` est enveloppé (`adjust_by`, commit = `spec.save`), Entrée ouvre le champ de valeur (`PanelMsg::SliderEdit`), le champ ouvert n'est jamais enveloppé.
4. Les commandes des pages Général, Apparence, Bureau, Barre, Dock, Lanceur : `self.focusable(…)` à chaque feuille.
5. Les pages Clavier et souris, Raccourcis (le bouton de capture n'est pas enveloppé pendant une capture, sinon Entrée la relance), Écrans, Son, Réseau, Bluetooth, Énergie, Notifications ; les `pick_list` : le menu d'iced 0.14.2 ne se pilote pas au clavier (`overlay/menu.rs` n'écoute que la souris) — liste fermée, les flèches changent la valeur par `focus::choose(child, précédent, suivant)` qui publie l'`on_select` du site ; Échap, menu ouvert, ferme le menu avant le tiroir.
6. Les pages restantes (À propos, Applications, IA, mots de passe, SmartUI, extensions) ; recensement final : `rg 'toggler\(|button\(|pick_list\(' src/ui/plugins` hors barre, dock, Panorama, lanceur — chaque occurrence enveloppée ou justifiée en remarque.
7. Le centre de notifications : cartes et « Effacer l'historique » ; Tab borné au popover ; scène `acces-clavier-centre`.
8. Les bulles : `keys.focus_notifications` avec des bulles à l'écran donne le clavier à la pile (surface `Exclusive` le temps du parcours, comme la barre en ENT-02), Tab entre leurs actions, Échap rend. Condition d'arrêt : si la surface des bulles ne peut changer d'interactivité sans renaître, marquer l'étape « bloquée » et clore l'écart sans elle, en remarque.
9. DESIGN.md, guides, scène `acces-clavier`, relecture d'ensemble ; l'écart passe `fait`.

## Périmètre

- Héritent : toutes les pages du tiroir, le centre de notifications, `hyprdesk keys`, la page Raccourcis.
- À vérifier : ENT-02 (barre, dock, volet par raccourci) ; ENT-11 (Ctrl+Tab du centre) ; SEL-08 (curseurs saisissables : le champ s'ouvre au clic sur la valeur, l'enveloppe du curseur ne doit pas le gêner) ; Échap du tiroir ; la capture d'un raccourci dans la page Raccourcis (Tab ne doit pas y être volé).
- Hors périmètre : le dock et le lanceur (parcours aux flèches existant) ; le Panorama ; les fenêtres d'application (Hyprland) ; l'apparence de l'anneau (ENT-13, hors plan).

## Validation

- Produit : `CTRL + F2`, `CTRL + F6` puis Tab : chaque commande du tiroir prend l'anneau, Entrée l'active ; `CTRL + F1` : l'OSD annonce le mode, Tab ne va plus qu'aux champs.
- Interface : tiroir en plein bureau et bureau masqué ; pages longues (défilement jusqu'à la commande focalisée — à vérifier, sinon `scroll_to` à l'étape 1) ; texte agrandi.
- Système : une seule enveloppe (`focus.rs`) ; aucun widget parallèle.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh --visual /tmp/hig-visual-C` → code de sortie 0.

## Conditions d'arrêt

- Le clic simulé n'active pas `button` ou `toggler` d'iced 0.14 : s'arrêter, marquer « bloqué » avec l'observation (il faudrait alors envelopper par message, site par site, ce qui change l'effort).
- Le focus clavier ne fait pas défiler le tiroir jusqu'à la commande : traiter dans l'étape 1 par une opération de défilement ; impossible sans nouvelle dépendance : « bloqué : dépendance ».

## Documentation de design

- Après acceptation : dans `DESIGN.md`, « Accès clavier complet (HIG ENT-01) : activé par défaut, `CTRL + F1` le bascule ; Tab atteint chaque commande du tiroir et du centre, l'anneau de focus dit où l'on est ; une seule enveloppe de focus, `src/ui/focus.rs`. »

## Relecture du plan (itération C20, agent indépendant)

- Ce qui cassait : le clic simulé dans un seul `Shell` n'activait pas le bouton d'iced (l'appui capturé lui masque le relâchement, `button.rs:297`) — chaque événement a désormais son `Shell`, et un test le prouve ; l'enveloppe ne transmettait pas `overlay` (menu d'une liste, infobulle du curseur perdus) ; l'anneau restait après un clic dans un champ, et Espace y aurait activé la commande ; la capture d'un raccourci se relancerait à Entrée ; le curseur SEL-08 ne s'enveloppe pas en bloc (deux arrêts de Tab) ; la conversation a son propre Tab ; le défilement jusqu'au focus n'est pas fourni par iced. Tout est corrigé à l'étape 1 ou rangé dans une étape.
- Ce qui manquait : le clavier des `pick_list` (le menu d'iced ne l'écoute pas) — étape 5 ; les groupes du tiroir, sans lesquels chaque page commence par une trentaine de Tab — étape 3 ; un harnais de test — l'enveloppe est générique sur le moteur de rendu, testée avec le moteur nul.
- L'étape 1 d'origine ne tenait pas en une itération : elle est découpée (1, 2, 3).
- Le plan résout-il l'écart ? Oui, avec les `pick_list` pilotés aux flèches : chaque commande du tiroir et du centre devient atteignable par Tab, le mode se bascule par `CTRL + F1`.

