# PAT-20 — Résumé du Silence : à la fin du Silence, une bulle résume les notifications retenues et un clic ouvre leur liste

Écrit contre : `ae536ce` (intégration `hig/plan-d-action`). Écart M, nature « PISTE » : recommandation descriptive de la HIG, sans mesure. Priorité moyenne.

## Chaîne de preuve

- Surface : la fin du Silence — `Toasts::silence_expire` (`TMsg::Expire`) et `Toasts::command` (bras `dnd off`) ; la réception sous Silence — bras `Event::Shown` de `toasts_update`, branche `self.toasts.dnd && !passes`, qui range la notification dans l'historique sans bulle (`src/ui/toasts.rs`).
- Problème : sous Silence, les notifications non critiques vont à l'historique sans être signalées ; quand le Silence s'éteint, à l'échéance ou à la main, rien ne dit ce qui est arrivé. Il faut ouvrir une liste pour l'apprendre.
- Preuve de design : HIG managing-notifications › Integrating with Focus — « Delivery scheduling lets people choose whether to receive notification alerts immediately or in a summary that's delivered at times they choose » ; règle « Livraison programmée », première partie de `01a-patterns-shell.md`. Colonne Correction du registre : « Proposer, à la fin du Silence, les notifications retenues. »
- Propriétaires : `src/ui/toasts.rs` (`Toasts`, `Toast`, `set_silence`, `silence_expire`, `command`, `view_toast`, `height_of`, `sync_toasts`) ; `src/notifications.rs` (`Notification`, `build`) ; la liste des notifications de la barre (`popover::Kind::Notifications`, `App::open_popover`).
- Relevé à `ae536ce` : la fin programmée passe par `TMsg::Expire` → `silence_expire()`, vrai quand le Silence s'éteint ; la fin à la main par `Msg::Control(Command::Notifications("dnd off"))` — tuile du centre (`popover/continuity.rs:615`), menu (`bar/menus.rs:343`), ligne de commande — puis `Toasts::command` ; les deux chemins passent par `set_silence`. `Notification` porte sa date nulle part : une retenue ne se reconnaît pas dans l'historique. Le serveur de hyprdesk numérote à partir de 1 (`next: AtomicU32::new(1)`) : l'identifiant 0 n'est jamais celui d'une notification reçue. `config reset all` enchaîne `notifications clear` puis `dnd off` (`src/ui/mod.rs:1197`).
- Certitude relevée : une notification retenue sous Silence est déjà à l'historique, non lue (`deliver`). Le résumé n'a rien à remettre : il signale, puis mène aux notifications.

## Décision de design

- **Les retenues** : pendant le Silence, chaque notification accordée, non critique et non `transient`, rangée dans l'historique sans bulle parce que le Silence est allumé, est comptée dans `Toasts::held` (ses identifiants, en mémoire). La liste se vide quand le Silence s'allume. Ne comptent pas les notifications sans bulle par style de remise « Aucune » (HIG SYS-03) ou par basse urgence (HIG PAT-06) : c'est un choix de l'application ou de l'utilisateur, non une retenue du Silence.
- **Le résumé** : quand le Silence s'éteint — à l'échéance comme à la main — et que `held` n'est pas vide, une bulle de résumé se pose, en tête des bulles : titre « N notifications pendant le Silence », corps « de M applications » quand M est supérieur à 1, icône : la cloche. Elle suit les règles d'une bulle ordinaire (son délai, « Fermer »), sans action d'application. Un clic dessus ouvre la liste des notifications — la carte « Notifications » de la barre, par le chemin de `open_popover(.., Kind::Notifications)` — et la referme ; l'ouverture marque l'historique lu, comme aujourd'hui (HIG SYS-02). Sa carte entière est le bouton : la bulle ordinaire ne se clique pas hors de ses actions et de « Fermer ». La couche des bulles ne prend pas le clavier (`KeyboardInteractivity::None`) : au clavier, la liste reste celle de la barre.
- **Une seule bulle** : éteindre le Silence ne rejoue pas les notifications retenues, il les propose. Silence éteint sans retenue : rien.
- **Porteur** : le résumé est un `Toast` de plus dans `Toasts::list`, marqué `Toast::summary`, porteur d'une `Notification` de hyprdesk (identifiant 0, réservé, `transient`, sans action). Les messages qui parlent d'une bulle (`Invoke`, `Close`) le reconnaissent à ce marqueur et n'écrivent jamais à une application.
- **Remise à zéro** : `config reset all` vide `held` avant d'éteindre le Silence : un historique effacé n'a rien à proposer.

Pourquoi cela résout la racine : ce que le Silence a retenu est proposé quand il s'éteint, en une bulle, et un clic mène aux notifications retenues.

## Réutilisation

- `Toast { n, since, request, alert }` (un champ de plus), `Toasts::list`, `view_toast` (une bulle porte déjà icône, titre, corps, « Fermer »), `card_height` et `height_of` (le résumé est la carte de base : un corps, aucune action, hauteur `CARD`), `icon_or_bell`, `Toasts::deliver`.
- `notifications::build` (fabrique la `Notification` du résumé), `toasts::card_app` (les noms d'application du corps).
- `set_silence` (l'unique point de passage de l'allumage et de l'extinction), `sync_toasts` (la couche se redimensionne), `App::open_popover`, `popover::Kind::Notifications`, `TMsg::Read`.
- Scène d'aperçu des bulles (`notifications`) : une scène de plus, `notifications-resume`.

Une bulle de plus dans une liste existante : aucune primitive d'interface nouvelle.

## Changements

1. `src/ui/toasts.rs`
   - Change : `Toasts::held: Vec<u32>` ; dans la branche Silence du bras `Shown`, l'identifiant entre dans `held` ; `set_silence(true)` vide `held` ; à l'extinction avec des retenues, `Toasts::summary()` pose la bulle marquée et vide `held` ; `Toast::summary` ; la carte d'une bulle résumé est un bouton (toute la carte, les autres ne changent pas) qui ouvre la liste et ferme la bulle ; les bras `Expire` et `Command::Notifications` rendent `sync_toasts()` quand une bulle est posée.
   - Preserve : HIG PAT-04 (les fins du Silence et leur état publié), PAT-05 (passage du Silence), PAT-02 (demande d'autorisation), SYS-03, PAT-06, l'historique, la vue étendue de la pilule Island (SYS-13), `notifications.*`.
   - Verify : tests sur `toasts_update` et `toasts_command` — retenue comptée sous Silence, pas hors Silence ni par style « Aucune » ; résumé à l'expiration et à `dnd off`, pas sans retenue, pas à l'allumage, pas après une remise à zéro ; le clic ouvre la liste, ferme la bulle et ne rend rien à une application.
2. `src/ui/mod.rs` : la remise à zéro vide `held` ; le bras `Command::Notifications` resynchronise la couche.
3. `src/ui/preview.rs`, `scripts/verify.sh` : scène `notifications-resume`.
4. `locales/en.toml`, `docs/GUIDE.fr.md`, `docs/GUIDE.md`, `DESIGN.md` (un point « Résumé du Silence »).

Deux étapes, réalisables chacune en une itération : (1) retenues, résumé, tests, porte ; (2) clic vers la liste, scène d'aperçu, guides, `DESIGN.md`.

## Portée

- Héritent : les hôtes qui montrent `toasts.list` autrement que par `view_toast` (niche Convexe, volet Panorama, pilule Island) : le résumé y apparaît comme une bulle de plus ; son clic passe par le même message.
- À vérifier : Silence rallumé avant le clic (la bulle vivante se ferme, `held` se vide) ; Silence expiré pendant une veille (`silence_expire` est rappelé au bras `Shown`) ; style « Alertes » sous Silence (une retenue comme une autre) ; `transient` sous Silence (fermée, non comptée) ; `transition = "none"` ; la vue étendue de la pilule Island, qui suit l'identifiant de la bulle puis son entrée d'historique — le résumé n'en a pas.
- Exclus : la remise à heures choisies hors Silence et le choix des applications du résumé (livraison programmée complète) ; toute option Lua ou commande en ligne nouvelle ; PAT-04, PAT-05.

## Validation

- Produit : Silence allumé, trois notifications non critiques arrivent ; à l'échéance une bulle « 3 notifications pendant le Silence » ; un clic ouvre la liste sur les trois ; Silence coupé à la main : même bulle ; Silence sans notification : rien ; `config reset all` : rien.
- Interface : la bulle avec un seul corps, avec « de 3 applications » ; en anglais, plus long (« 3 notifications during Do Not Disturb ») ; à 1440, 900 et 640 px.
- Système : aucune écriture de fichier nouvelle — `held` vit en mémoire ; après un redémarrage sous Silence, les retenues d'avant ne sont pas proposées.
- Dépôt : `CARGO_BUILD_JOBS=6 bash scripts/verify.sh --visual /tmp/hig-visual-A` → code de sortie 0.

## Conditions d'arrêt

- S'arrêter si l'identifiant 0 est déjà pris, dans un hôte, pour désigner autre chose ; si le clic de la bulle ne peut pas ouvrir la liste sans un chemin propre à chaque style de barre — le clic ouvre alors ce que la pilule ouvre, ou rien, et l'écart passe `bloqué` ; si les retenues devaient survivre à la session (un fichier de plus : hors de la Correction).

## Relève à froid

Relu sans agent indépendant, après relevé de `silence_expire`, des deux appels de `set_silence`, de la numérotation du serveur et de la remise à zéro, puis une seconde fois, après autre chose, contre `card_height` et `view_toast`.

- **Qu'est-ce qui casse ?** (1) Les tests existants de `toasts.rs` qui allument puis éteignent le Silence autour de notifications non critiques recevraient une bulle de plus dans `list` : ceux qui comptent les bulles devront la fermer ou vider `held`. (2) La remise à zéro éteint le Silence juste après avoir effacé l'historique : sans vider `held` d'abord, le résumé proposerait des notifications qui n'existent plus. (3) La bulle ordinaire n'est pas cliquable — `view_toast` ne pose que ses actions et « Fermer » : le clic du résumé est un bouton à poser sur la seule carte marquée. (4) La vue étendue de la pilule Island (SYS-13) suit l'identifiant de la bulle, puis son entrée d'historique : le résumé n'en a pas — il s'y montre comme toute bulle tant qu'il vit, et ne survit pas à sa fermeture. (5) Les hôtes autres que `view_toast` montrent une bulle de plus : rien à casser, à capturer.
- **Que manque-t-il ?** Les textes exacts et leur traduction ; la scène d'aperçu. Levés par la seconde lecture : la hauteur du résumé — `card_height` donne à la carte de base (`CARD`, 104 px) un corps sans action, aucun surcroît ; le clic, qui demande un bouton sur la carte. Rien qui empêche d'écrire l'étape 1.
- **Le plan résout-il l'écart ?** Oui, pour la Correction inscrite au registre — proposer, à la fin du Silence, les notifications retenues. La livraison à heures choisies, hors Silence, reste hors de cette correction.

## Documentation de design

- Après acceptation : `DESIGN.md` (un point « Résumé du Silence ») ; guides français et anglais ; scène d'aperçu `notifications-resume`.

## Exécution

- **Étape 1 — faite** (`c471a46`, A14) : `Toasts::held` (les identifiants retenus, mémoire seule), `Toasts::summary`, `resume_notification`, `Toast::summary` ; les bras `Expire`, `Shown` (échéance passée pendant une veille) et `Command::Notifications` resynchronisent la couche ; `Invoke`, `Close` et `Tick` ne parlent jamais à une application pour le résumé. Tests : `le_silence_retient_ce_qu_il_passe_sans_bulle`, `le_silence_qui_s_eteint_resume_ses_retenues`.
- **Étape 2 — faite** (même commit) : `TMsg::Summary` et `App::open_notifications` (carte « Notifications » de la barre de l'écran actif, à défaut la première ; rien sans barre) ; la carte du résumé est un bouton, au filet renforcé au survol ; scène `notifications-resume` ; guides français et anglais ; `DESIGN.md`.

Écarts au plan, notés au registre :

- `held` est vidé par `notifications clear` lui-même, non par le seul `config reset all` : « Effacer l'historique » (Réglages, centre Continuité) et le volet Panorama passent par là.
- Un résumé vivant s'efface quand le Silence se rallume (point « à vérifier » du plan).
- Ne comptent pas comme retenues : une critique dont on a refusé le passage, une demande d'autorisation (non remise).
- La carte-bouton est celle de la surface des bulles (`view_toast`). La pilule Island montre le résumé comme sa dernière bulle ; son clic ouvre la vue étendue (HIG SYS-13), dont le pied mène à l'historique.

## Relecture du diff

Relu sans agent indépendant (`c471a46`), après les captures de la scène `notifications-resume` (1440 et 640 px, en anglais : « 5 notifications during Do Not Disturb », « from 3 apps »).

- **Le diff fait-il exactement ce que l'écart demande ?** Oui : à l'extinction du Silence — échéance, `dnd off`, échéance passée pendant une veille — une seule bulle de hyprdesk propose les notifications retenues, sa carte ouvre la liste et la referme ; sans retenue, rien. Quatre écarts au plan, ci-dessus.
- **Respecte-t-il `DESIGN.md` et les conventions du dépôt ?** Oui : aucune primitive nouvelle (un `Toast` de plus dans la liste, le `button` existant), pas d'accent (filet neutre au survol), pas de mouvement ajouté, espacement et rayons inchangés ; français source, trois chaînes dans `locales/en.toml` ; guides et `DESIGN.md` à jour.
- **Régression, test manquant ?** Corrigé en relisant : le commentaire « Basse urgence » s'était trouvé séparé de son `if` par le bloc de retenue ; deux chemins sans resynchronisation de la couche (retour anticipé de `Shown`, bras `Command::Notifications`) sont couverts. Non testé : l'allumage (`dnd on` écrit l'état réel du Silence — `held` vidé, résumé vivant effacé), le garde de `Invoke` sur le résumé, le clic sans barre.
