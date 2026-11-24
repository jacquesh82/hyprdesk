# ENT-03 — Gestes système : vue d'ensemble, applications, notifications, bureau

Écrit contre : `60e08dc` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : Réglages → Gestes (`src/ui/hyprlists.rs`, `gestures_page`) ; le fichier des gestes tenu par hyprdesk (`hl.gesture`, domaine `Inputs` de `src/hyprset.rs`).
- Problème : les actions d'un geste sont celles de Hyprland seules (`hyprlists.rs`, `ACTIONS` : workspace, special, move, resize, close, fullscreen, float, cursorZoom). Aucun geste ne mène à une surface de hyprdesk : vue d'ensemble, grille des applications, centre de notifications, bureau. Seul le balayement d'espaces est réglable.
- Preuve de design : pointing-devices › macOS, tableau des interactions standard (Mission Control : trois doigts vers le haut ; App Exposé : trois doigts vers le bas ; Launchpad : pincer avec le pouce et trois doigts ; Afficher le bureau : écarter ; Centre de notifications : deux doigts depuis le bord droit) ; « people expect systemwide gestures to be available ».
- Propriétaires : `hyprlists.rs` (`ACTIONS`, `DIRECTIONS`, champs conditionnels du formulaire), `hyprset.rs` (`gestures`, `calls_lua`, `json_lua`, `gesture_complete`), `src/adopt.rs` (relecture de `hl.gesture`, déjà tolérante à `action = function() … end`), les commandes de `src/control.rs` (`overview`, `apps`, `focus notifications`, `show`).
- Incertitude : (1) `hl.gesture` accepte une fonction pour `action` (la syntaxe est relue par `adopt.rs:739`) ; qu'il l'exécute au déclenchement n'est vérifiable qu'en session : essai manuel du propriétaire, consigné en remarque. Au déclenchement, l'appel est `hl.dispatch(hl.dsp.exec_cmd(…))` (`hypr.rs`). (2) Hyprland ne connaît pas de geste « depuis le bord » : le centre de notifications prend un geste à quatre doigts.

## Décision de design

- **Quatre actions hyprdesk** dans `ACTIONS`, en tête : « Vue d'ensemble », « Applications », « Centre de notifications », « Afficher le bureau ». Stockées dans le modèle comme une chaîne (`"action": "hyprdesk:overview"`), écrites par `calls_lua` sous forme d'une fonction : `action = function() hl.dispatch(hl.dsp.exec_cmd("hyprdesk overview")) end` (`hyprdesk` nu, voir l'étape 1). Un geste à fonction écrit à la main dans `hyprland.lua` y reste, comme aujourd'hui.
  - `overview` → `hyprdesk overview` ; `apps` → `hyprdesk apps` ; `notifications` → `hyprdesk focus notifications` ; `show-desktop` → `hyprdesk show`.
- **Gestes par défaut, façon macOS**, posés quand la liste des gestes ne tient rien sur la même combinaison (doigts + direction) : trois doigts vers le haut → vue d'ensemble (Mission Control) ; quatre doigts, pincer → applications (Launchpad) ; quatre doigts, écarter → afficher le bureau ; quatre doigts vers la gauche → centre de notifications (le bord n'existe pas pour `hl.gesture`). Trois doigts vers le bas restent libres : App Exposé n'a pas d'équivalent. Le balayement horizontal à trois doigts (espaces) reste. Une fonction livrée se voit : ces gestes paraissent dans Réglages → Gestes, chacun modifiable ou supprimable ; un geste supprimé n'est pas reposé (marque `gestures_defaults` dans les réglages, comme `essentials_off`).

## Réutilisation

- `hyprlists.rs` : le formulaire générique des gestes (choix, champs conditionnels) ; `ACTIONS` prend quatre entrées.
- `hyprset.rs` : `calls_lua` (une branche pour la chaîne `hyprdesk:`), `json_lua`.
- `adopt.rs` : la relecture des appels `hl.gesture`.
- Les commandes existantes de `control.rs` : aucune commande nouvelle.

## Changements

1. `src/ui/hyprlists.rs`, `src/hyprset.rs`, `locales/en.toml`
   - Change : les quatre actions ; l'écriture en fonction ; les recouvrements de directions.
   - Preserve : les gestes de Hyprland, leurs champs conditionnels, l'ordre des gestes.
   - Verify : test — `{ fingers = 3, direction = "up", action = "hyprdesk:overview" }` s'écrit `hl.gesture({ … action = function() hl.dispatch(hl.dsp.exec_cmd("hyprdesk overview")) end })` et la chaîne survit à l'aller-retour par `settings.lua`.
2. `src/hyprset.rs` (ou l'endroit où le fichier des entrées est composé), `src/config` si la marque est un réglage
   - Change : les gestes par défaut, posés à la composition quand la combinaison est libre ; la marque des gestes retirés.
   - Verify : test — liste vide : quatre gestes ; un geste à trois doigts vers le haut déjà réglé : il reste, les trois autres sont posés ; un défaut supprimé n'est pas reposé.
3. Documentation : `docs/GUIDE.fr.md`, `docs/GUIDE.md` (Gestes), `README*.md` (tableau des gestes s'il existe), `DESIGN.md`.

## Étapes

1. Les quatre actions hyprdesk : `ACTIONS` (libellés, traductions), l'écriture par `calls_lua` d'une chaîne `hyprdesk:<commande>` en `action = function() hl.dispatch(hl.dsp.exec_cmd("hyprdesk <commande>")) end` — `hyprdesk` nu, trouvé dans le `PATH` : un chemin absolu écrit dans `inputs.lua` serait périmé après une réinstallation et voyagerait tel quel dans un export (`share.rs`). Tests : l'écriture ; l'aller-retour par `settings.lua` (`hyprset::to_lua`/`parse`) garde la chaîne.
2. Les recouvrements de directions : une fonction `covers(a, b)` (« vertical » couvre « up » et « down », « horizontal » couvre « left » et « right », « pinch » couvre « pinchin » et « pinchout », « swipe » couvre tout ; mêmes doigts, mêmes `mods`), utilisée par la détection des doublons de `hyprlists.rs` (aujourd'hui par chaîne exacte) ; tests.
3. Les gestes par défaut, inscrits une fois dans `cfg.general.hypr.gestures` au chargement (migration : ils paraissent dans Réglages → Gestes) quand aucun geste, ni de hyprdesk ni laissé dans `hyprland.lua` (`left_in_hyprland("gesture")`), ne les couvre ; la marque des défauts déjà proposés (`settings.rs`, `config/lua.rs`, `cli_config.rs` sur le modèle de `keys.essentials_off`) : un défaut supprimé n'est pas reposé ; la remise par défaut de la section Entrées (`defaults.rs`) remet aussi la marque. Vérifier que `inputs.lua` est bien écrit quand il est tenu par hyprdesk (`genfiles.rs:488`). Tests.
4. Documentation, scène d'aperçu de Réglages → Gestes avec les gestes par défaut ; l'écart passe `fait`.

## Relecture du plan (itération C26, agent indépendant)

- Ce qui cassait : la relecture d'un geste écrit à la main avec une fonction (`hyprsrc.rs:292` réduit le corps à `"<fonction>"`, `adopt.rs` laisse la ligne) — hors périmètre : les gestes de hyprdesk vivent dans `settings.lua` sous forme de chaîne, la fonction n'est que leur écriture ; l'appel au déclenchement est `hl.dispatch(hl.dsp.exec_cmd(…))`, pas `hl.exec_cmd` (démarrage) ; le chemin absolu de l'exécutable ; « afficher le bureau » est `hyprdesk show`, pas `toggle` (qui masque un bureau déjà affiché) ; la détection des doublons par chaîne exacte ne voit pas les recouvrements ; les défauts posés à la seule composition seraient invisibles dans Réglages.
- Ce qui manquait : `genfiles.rs`, `settings.rs`, `config/lua.rs`, `cli_config.rs`, `defaults.rs`, `share.rs` ; la migration ; la remise par défaut ; l'étape 2 d'origine, coupée en deux (2 et 3).
- Limite acceptée : bureau masqué (sans barre), le geste du centre de notifications est sans effet, comme son raccourci (`focus_notifications`).
- Le plan résout-il l'écart ? Oui pour la vue d'ensemble, les applications, le bureau (`show`) ; le centre de notifications, partout où il existe.

## Périmètre

- Héritent : Réglages → Gestes, le fichier des entrées tenu par hyprdesk.
- À vérifier : `hyprdesk import`/`export` (les gestes voyagent avec les entrées) ; `adopt.rs` (reprise d'un `hyprland.lua` existant) ; les gestes déjà réglés par l'utilisateur.
- Hors périmètre : un geste depuis le bord (non pris en charge par `hl.gesture`) ; App Exposé (pas de vue des fenêtres d'une seule application).

## Validation

- Produit : trois doigts vers le haut ouvrent la vue d'ensemble ; quatre doigts pincés, les applications ; écartés, le bureau.
- Interface : Réglages → Gestes montre les quatre gestes, libellés en français et en anglais.
- Système : aucune commande nouvelle ; une seule écriture des gestes (`calls_lua`).
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh` → code de sortie 0.

## Conditions d'arrêt

- `hl.gesture` refuse une fonction pour `action` dans la version de Hyprland visée (à lire dans la documentation de l'API Lua ou les tests existants) : marquer « bloqué » avec l'observation.

## Documentation de design

- Après acceptation : dans `DESIGN.md`, « Gestes système (HIG ENT-03) : trois doigts vers le haut, la vue d'ensemble ; quatre doigts pincés, les applications ; écartés, le bureau ; vers la gauche, le centre de notifications. Réglables dans Réglages → Gestes. »
