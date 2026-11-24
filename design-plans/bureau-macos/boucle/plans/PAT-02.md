# PAT-02 — Autorisation des applications : demander à la première notification d'une application, garder et révoquer dans Réglages ; avec elle, le style de remise (SYS-03) et le paquet « Notifications » (PAT-07)

Écrit contre : `230127a` (intégration `hig/plan-d-action`). Écart L, mené avec `PAT-07` et `SYS-03`, qui décrivent le même changement (décision D5) : un seul changement, les trois lignes du registre passeront `fait` avec le même hash et un renvoi mutuel.

## Chaîne de preuve

- Surface : réception d'une notification (`TMsg::Event(Event::Shown)`, `src/ui/toasts.rs` ; `Server::notify`, `src/notifications.rs`) ; Réglages (paquets de `src/ui/plugins/`).
- Problème : toute notification est acceptée et montrée, sans regard sur l'émetteur ; le seul filtre est le Silence global. Aucune autorisation ne s'accorde ni ne se révoque ; le style de remise dépend du seul code (critique ou bannière) ; les Réglages n'ont pas de paquet Notifications, et l'historique ne s'efface qu'au centre Continuité.
- Preuve de design : HIG managing-notifications › Discussion — « You need to get permission before sending any notification » ; managing-notifications › Sending marketing notifications — « Make sure people can manage their notification settings within your app » ; notifications › Discussion — « Before you can send any notifications to people, you have to get their consent » ; « people generally use settings to specify the styles of notification they want to receive ». Colonnes Correction du registre : PAT-02 — « Demander l'autorisation à la première notification de chaque application (autoriser, refuser), la garder et la révoquer dans Réglages » ; PAT-07 — « Ajouter un paquet "Notifications" aux Réglages : par application (autorisation, niveau, révocation), effacement de l'historique » ; SYS-03 — « Un paquet de réglages Notifications : par application (`desktop-entry`), l'autorisation d'afficher et le style de remise selon l'urgence ».
- Propriétaires : `src/notifications.rs` (l'état gardé : `Passage` est le modèle), `src/ui/toasts.rs` (réception, bulles, `Explain` du passage du Silence — même mécanisme), `src/ui/plugins/` (un paquet de plus : `windows.rs`, `datetime.rs` sont les modèles ; rangées `App::field`, `App::section` de `src/ui/panel.rs`), `src/defaults.rs` (ce que `hyprdesk config reset all` oublie).
- Relevé à `230127a` : la clé d'application existe (`passage_key` : `desktop-entry`, sinon le nom de l'émetteur) ; `hyprdesk notifications clear` passe par `Toasts::command` ; `notifications.*` (largeur, nombre, coin, empilement, durée, aperçus) sont des options Lua sans page aux Réglages.
- Certitude relevée : une notification refusée est fermée pour l'application avec `Reason::Expired` ; `hyprdesk config reset all` efface l'état des notifications dans `Held::Notifications` (`src/defaults.rs`).

## Décision de design

- **Clé d'application** : celle de `passage_key`, partagée avec le passage du Silence (PAT-05). Rien ne change à cette clé.
- **État gardé** (`src/notifications.rs`) : `Authorizations`, fichier `notifications-apps` dans le dossier d'état, même idiome que `passage-silence` — une ligne par application, `clé=1` (accordée, bannières), `clé=1:alerte`, `clé=1:silencieux`, `clé=0` (refusée). Application absente : elle n'a encore jamais demandé. `hyprdesk config reset all` oublie le fichier.
- **La demande (PAT-02)** : toute notification d'une application sans décision porte la demande dans sa bulle — « <Application> veut vous envoyer des notifications », boutons « Refuser » et « Autoriser » — au-dessus de son contenu, pour qu'on sache ce qu'on accorde. Mécanisme : celui de `Explain` (PAT-05), élargi. La bulle qui porte une demande reste jusqu'à la réponse ou à sa fermeture. Fermée sans réponse : rien n'est inscrit, la notification suivante de l'application redemande. Tant qu'il n'y a pas de décision, rien de cette application n'entre dans l'historique : non accordée, une notification n'est pas remise.
- **Refusée** : ni bulle ni historique ; la notification est fermée aussitôt pour l'application. Accordée : comme aujourd'hui.
- **Style de remise (SYS-03, le « niveau » de PAT-07)** : par application — « Bannières » (défaut : la bulle de son délai), « Alertes » (la bulle reste jusqu'à sa fermeture, comme une critique), « Aucune » (l'historique seul, sans bulle, comme une basse urgence). Une notification critique garde ses règles : elle reste, et son passage du Silence suit PAT-05.
- **La demande prime** : sur une bulle qui porte une demande, le passage du Silence ne s'explique pas ; il s'expliquera à la première critique qui suit l'autorisation.
- **Paquet « Notifications » des Réglages** (PAT-07, D5) : id `notifications`, groupe « Système », une page. Section « Applications » : une rangée par application connue (décisions, historique, passage du Silence) — icône et nom, style de remise (choix des trois), bascule « Autoriser » ; bascule coupée = autorisation révoquée. Section « Historique » : « Effacer l'historique », commande destructive à confirmation, même chemin que le centre Continuité.
- Les options `notifications.*` restent des options Lua et CLI : la page ne les déplace pas.

Pourquoi cela résout la racine : la permission existe, elle se demande au premier envoi, elle se garde et se révoque ; le style de remise se règle par application ; tout cela se trouve dans un paquet des Réglages.

## Réutilisation

- `Passage` (`load`, `read`, `save`, `render`, `granted`, `set`) : le modèle du fichier d'état ; `passage_key`, `app_id`.
- `Explain`, `explain_height`, `card_height`, le bouton « Bloquer pendant le Silence » de `view_toast` : la demande a la même place dans la bulle.
- `Toasts::command("clear")` ; `App::field`, `App::section` ; `windows.rs`, `datetime.rs` ; `BUILTIN`, `GROUPS`, l'index de recherche de `src/ui/plugins/mod.rs` ; scène d'aperçu d'une page de Réglages.
- Exemplar : `src/ui/toasts.rs`, `toasts_update` (bras `Event::Shown`) ; `src/ui/plugins/datetime.rs`.

Un fichier d'état de plus, sur l'idiome existant : aucune primitive d'interface nouvelle.

## Changements

1. `src/notifications.rs`, `src/defaults.rs`
   - Change : `Delivery` (Bannières, Alertes, Aucune), `Authorization { granted, delivery }`, `Authorizations` (`load`, `granted`, `delivery`, `set`, `save`), `authorizations_path()` ; le fichier entre dans ce que la remise à zéro oublie.
   - Preserve : `passage-silence`, l'état publié, le serveur.
   - Verify : tests de lecture, d'écriture et de remplacement.
2. `src/ui/toasts.rs`
   - Change : à la réception, décision de l'application — sans décision : bulle de demande, résidente, sans historique ; refusée : fermée aussitôt ; accordée : style de remise appliqué. `TMsg::Allow(id)` et `TMsg::Deny(id)`. La vue étendue de la pilule Island (SYS-13) montre la demande comme la bulle.
   - Preserve : Silence, passage du Silence (PAT-05), urgence basse, historique, état publié.
   - Verify : tests sur `toasts_update` ; scène d'aperçu de la bulle de demande.
3. `src/ui/plugins/notifications.rs` (nouveau), `src/ui/plugins/mod.rs`
   - Change : la page, son enregistrement, ses clés de recherche.
   - Verify : scène d'aperçu de la page ; test du registre des paquets.
4. `locales/en.toml`, `docs/GUIDE.fr.md`, `docs/GUIDE.md`, `docs/PLUGINS.md`, `DESIGN.md`.

## Étapes

1. **État** — faite (`ac7c95f`, A9) : `Delivery`, `Authorization`, `Authorizations` et leur fichier, oubliés par la remise à zéro ; tests (`autorisations_par_application`). Aucun comportement ne change. Relevé : `get(clé)` remonte la décision entière et `granted(clé)` en dérive — pas de `delivery` séparé ; le libellé de la remise à zéro (`Held::Notifications`) dit les autorisations, traduction suivie.
2. **Demande et refus** (PAT-02) — faite (`a7b7c86`, A10) : la bulle de demande, ses deux réponses, le refus sans bulle ni historique, la demande qui prime sur l'explication du Silence ; quatre tests ; guides. Relevés : la demande est portée par `Toast::request` et la décision de l'interface par `Toasts::reception` — sans le service (`Toasts::active`, aperçu ou autre démon), rien ne se décide et les tests existants ne changent pas ; « Autoriser » fait de la bulle une bulle ordinaire (délai reparti, notification dans l'historique), « Refuser » la ferme comme « Fermer » (`Reason::Dismissed`) ; sans bulle à poser la demande (urgence basse, Silence), la notification se ferme pour l'application et la suivante redemande ; une demande critique passe le Silence ; la vue étendue de la pilule Island montre la demande et l'on y répond ; scène d'aperçu `notifications` (troisième bulle).
3. **Style de remise** (SYS-03) — faite (`a285950`, A10) : « Alertes » garde la bulle, « Aucune » passe par l'historique seul ; test. Relevés : la décision et le style arrivent ensemble (`Toasts::reception` → `Reception::Delivered(Delivery)`) ; « Alertes » est porté par `Toast::alert`, qui rend le délai nul (`lifetime_or`) ; « Aucune » suit le chemin de la basse urgence ; une critique garde ses règles quel que soit le style. Rien de visible tant que la page de l'étape 4 ne règle pas le style : le fichier `notifications-apps` le porte déjà (`clé=1:alerte`, `clé=1:silencieux`).
4. **Paquet « Notifications »** (PAT-07, D5) : la page et ses rangées, la révocation, « Effacer l'historique » ; enregistrement, recherche, scène d'aperçu ; `docs/PLUGINS.md`.
5. **Documentation de design** : `DESIGN.md`, guides revus, `locales/en.toml` ; au registre, PAT-02, PAT-07 et SYS-03 passent `fait` ensemble.

### Notes pour l'étape 4 (relevées à `a285950`, A10 — rien d'écrit dans le code)

- **Modèle** : `src/ui/plugins/datetime.rs` dans son entier (230 lignes) — `INFO`, `wrap`, `State`, `Msg`, `load`, `impl App { page, update }` ; la page s'assemble avec `self.section(…)`, `self.field(libellé, "", contrôle)`, `pick_list` et `toggler`, texte de statut en `self.ink().dim`.
- **Un paquet touche à** (`datetime` sert de fil) : `src/ui/plugins/mod.rs` — `pub mod` (l. 25), liste des `INFO` (l. 100), `GROUPS`, groupe « Système » (l. 156), mots de recherche (l. 586), variante de `PluginMsg` (l. 623), dispatch de `update` (l. 662), de la page (l. 732), de `load` (l. 775) ; `src/ui/panel.rs` — le champ d'état du panneau (l. 85) et son initialisation (l. 161) ; `src/ui/settings_search.rs` — la paire `(id, include_str!)` du source du paquet (l. 24) ; `src/ui/preview.rs` — l'état de la scène (l. 853) et l'onglet de la scène `date-heure` (l. 1683), à imiter pour une scène `reglages-notifications`, à ajouter aussi à la liste codée dans `scripts/verify.sh` ; les tests du registre des paquets dans `mod.rs`.
- **Rangée d'une application** : `Authorizations::load().apps` donne la clé ; le nom et l'icône viennent de l'historique (`self.toasts.history`, `desktop_entry` → `self.named_icons`) — une application refusée, sans historique, garde sa clé seule : prévoir le nom de la clé en repli. Contrôles : `pick_list` des trois `Delivery`, `toggler` « Autoriser » qui écrit `Authorization { granted, delivery }`.
- **« Effacer l'historique »** : même chemin que le centre Continuité (`Msg::Control(Command::Notifications("clear".into()))`, `popover/continuity.rs:1107`) ; confirmation en deux temps comme `about.rs` (`confirm_reset`) et `performance.rs` (`confirm_exit`).
- **À trancher en écrivant** : une bascule coupée révoque (`granted: false`) et sa bulle vivante, s'il y en a une, reste jusqu'à sa fermeture ; la page relit l'état à chaque ouverture (`load`), pas à chaque bulle.
- **Tests du registre à faire vivre** (`mod.rs`, l. 958–1065 à `a285950`) : `chaque_paquet_a_son_icone`, `registre`, `registres_traduits` (libellé et résumé traduits dans `locales/en.toml`), `groupes`, `onglets`. La scène d'aperçu passe par `SCENES` (`preview.rs:27`, 169 noms : passer à 170) et la liste de `scripts/verify.sh`.

## Portée

- Héritent : bulles, historique, cartes qui le listent ; la vue étendue de la pilule Island (SYS-13) ; widgets Lua qui lisent l'état publié.
- À vérifier : notification critique d'une application sans décision ; application qui remplace sa notification (même id) ; session sans fichier d'état ; `transition = "none"`.
- Exclus : commande en ligne ou option Lua nouvelle pour les autorisations ; passage des options `notifications.*` dans la page ; PAT-20 (résumé programmé) ; PAT-05.

## Validation

- Produit : première notification d'une application → bulle de demande ; « Autoriser » → remise normale, l'application figure aux Réglages ; « Refuser » → plus rien de cette application ; Réglages → Notifications → couper la bascule la révoque ; « Effacer l'historique » vide l'historique.
- Interface : bulle de demande avec et sans corps ni actions, avec une critique ; page sans application, à une, à dix applications ; texte anglais plus long.
- Système : un seul fichier d'état de plus, un seul paquet de plus, aucune primitive parallèle.
- Dépôt : `CARGO_BUILD_JOBS=6 bash scripts/verify.sh` → code de sortie 0, à chaque étape qui touche au code.

## Conditions d'arrêt

- S'arrêter si la demande ne peut pas vivre dans la bulle existante (hauteur, clef de la carte) ; si fermer une notification refusée pour l'application casse un client (raison `Reason` inadéquate) ; si le paquet exige un type de rangée qui n'existe pas.

## Relève à froid

Relu sans agent indépendant, après relevé de `Reason`, de `Held::Notifications` et des lignes du registre.

- **Qu'est-ce qui casse ?** (1) Les tests existants de `toasts.rs` reçoivent des notifications d'applications sans décision (« a », « Signal ») et comptent sur l'historique : dès l'étape 2, ils passeraient par la demande. Ils devront accorder l'application d'abord (une aide de test), comme ils prennent le verrou de `passage-silence` pour PAT-05. (2) Le délai d'une bulle se calcule sur la seule notification (`Toast::lifetime_or`) : la bulle de demande, résidente, impose d'y joindre la demande, comme la hauteur de carte joint `Explain`. (3) Les hôtes qui montrent une bulle autrement que par `view_toast` (niche Convexe, volet Panorama, pilule Island) montreront une demande sans ses boutons : ils la montrent comme toute bulle, la réponse se donne sur la carte de la bulle et sur la vue étendue (SYS-13). (4) Les tests lisent et écrivent le vrai dossier d'état : clefs uniques et verrou commun, l'idiome de PAT-05.
- **Que manque-t-il ?** La raison d'une fermeture pour refus (`Expired`, décidé) ; l'inscription du fichier dans la remise à zéro (`Held::Notifications`, étape 1) ; la prise en main de la pilule Island par une bulle de demande (étape 2). Rien d'autre.
- **Le plan résout-il l'écart ?** Oui : la permission se demande au premier envoi, se garde et se révoque (PAT-02) ; le style de remise se règle par application (SYS-03) ; tout tient dans un paquet des Réglages, qui efface aussi l'historique (PAT-07).

Relecture du diff de l'étape 2 (`a7b7c86`), sans agent indépendant, après la capture de la scène `notifications`.

- **Le diff fait-il exactement ce que l'écart demande ?** Oui, pour l'étape 2 : la demande à la première notification, ses deux réponses, le refus sans bulle ni historique, la demande qui prime sur l'explication du Silence. Garder et révoquer aux Réglages reste aux étapes 3 et 4.
- **Respecte-t-il `DESIGN.md` et les conventions du dépôt ?** Oui : la demande occupe la place de l'explication (même mesure `explain_height`, deux boutons de 28 px, HIG ACC-04, espacement de 4 px), boutons sans accent comme celui du passage du Silence, une seule famille de police ; français source, anglais dans `locales/en.toml`.
- **Régression, test manquant ?** Les 22 tests antérieurs de `toasts.rs` passent, assertions inchangées (la signature de `card_height` et le champ `request` suivent) ; sans le service, rien ne se décide. Relevé, non traité ici : sur la capture, l'icône de l'application de la demande (Thunderbird) tombe sur la cloche de repli, faute d'icône au thème de cette machine — repli prévu (ICO-03), sans rapport avec l'écart.

Relecture du diff de l'étape 3 (`a285950`), sans agent indépendant, après le test dédié.

- **Le diff fait-il exactement ce que l'écart demande ?** Oui, pour l'étape 3 : le style de remise par application s'applique à la réception. Le régler depuis les Réglages est l'étape 4.
- **Respecte-t-il `DESIGN.md` et les conventions du dépôt ?** Oui : aucun changement d'interface ; un champ et une variante de plus sur les propriétaires existants ; l'état se relit par `Authorizations::load()`, comme `Passage::load()`.
- **Régression, test manquant ?** Les 26 tests antérieurs de `toasts.rs` passent. Sans le service, `reception` rend `Delivery::default()` : aperçu et autre démon ne changent pas. Test ajouté pour les trois styles et la critique.

## Documentation de design

- Après acceptation : `DESIGN.md` (autorisation par application, demande à la première notification, styles de remise, paquet Notifications) ; guides français et anglais ; `docs/PLUGINS.md`.
