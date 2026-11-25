# Les surfaces de hyprdesk se décrivent à un lecteur d'écran (AT-SPI)

Écrit contre : `7ea3f02` (branche `hig/plan-A`)

## Chaîne de preuve

- Surface : toutes les surfaces du démon — barre, dock, lanceur, cartes de la barre, Réglages, verrouillage.
- Problème : aucun arbre d'accessibilité. Ni `accesskit`, ni `atspi` ; iced 0.14 n'en fournit pas (`Cargo.toml`, `iced` sans fonctionnalité d'accessibilité). Orca ne voit ni nom, ni rôle, ni focus.
- Règle : accessibility › Vision — « Describe your app's interface and content for VoiceOver. » ; « label interface elements appropriately ».
- Propriétaire : un module nouveau `src/a11y/`, seul propriétaire du pont ; les libellés viennent des propriétaires existants (modules de la barre, éléments du dock, résultats du lanceur, pages des Réglages), jamais recopiés.
- Ressource : `zbus` 5 (fonctionnalité `tokio`) est déjà dépendance directe, et sait servir des interfaces (`#[zbus::interface]`, comme `src/notifications.rs`, `src/tray.rs`, `src/polkit.rs`) et se connecter à une adresse arbitraire (`connection::Builder::address`). Aucune dépendance nouvelle.
- Incertitude : la conformité à Orca ne se vérifie qu'en session réelle ; la boucle vérifie le protocole (tests sur un bus privé `dbus-run-session`) et l'arbre (instantané sérialisé).

## Décision

Un pont AT-SPI 2 minimal, propre à hyprdesk, en quatre couches :

1. **Modèle** (`src/a11y/model.rs`) : un arbre pur, `Node { role, name, description, states, children, action }`, identifiants stables (`/org/a11y/atspi/accessible/<n>`), rôles et états AT-SPI (`Role`, `StateSet` en deux mots de 32 bits), sans D-Bus : testable seul.
2. **Bus** (`src/a11y/bus.rs`) : objets servis sur une connexion donnée (`start(conn)`, comme `notifications::start`) : racine au chemin fixe `/org/a11y/atspi/accessible/root`, avec `org.a11y.atspi.Application` (`ToolkitName = "hyprdesk"`, `Version`, `AtspiVersion = "2.1"`, `Id` en lecture **et écriture**, i32, que le registre pose par `Properties.Set`) et `org.a11y.atspi.Accessible` ; chaque autre nœud `/org/a11y/atspi/accessible/<n>` avec `Accessible` (`Name`, `Description`, `Parent`, `ChildCount`, `Locale`, `AccessibleId`, `GetChildAtIndex`, `GetChildren`, `GetIndexInParent`, `GetRole`, `GetRoleName`, `GetLocalizedRoleName`, `GetState`, `GetAttributes`, `GetApplication`, `GetInterfaces` exact — libatspi s'y fie —, `GetRelationSet` vide). zbus 5 n'a pas de chemins génériques : chaque nœud est un objet enregistré, retiré à chaque différence. Ordre : servir la racine, **puis** `org.a11y.atspi.Socket.Embed((so)) -> (so)` sur `org.a11y.atspi.Registry`, chemin `/org/a11y/atspi/accessible/root` ; la référence rendue devient le `Parent` de la racine. `Unembed` à l'arrêt, nouvel `Embed` si le registre renaît.
   **Démarrage** : `GetAddress` active toujours le lanceur du bus d'accessibilité, il ne dit donc pas si un lecteur écoute. Le critère est `org.a11y.Status.IsEnabled` (ou `ScreenReaderEnabled`), propriété de `org.a11y.Bus` à `/org/a11y/bus`, **suivie en continu** (Orca démarre souvent après le démon) ; l'adresse vient d'`AT_SPI_BUS_ADDRESS` si elle est posée, sinon de `GetAddress`. Désactivé : rien n'est servi, aucun coût.
3. **Alimentation** (`App::a11y_tree`) : l'arbre est calculé depuis l'état de l'`App` (pas depuis l'arbre iced, qui n'a pas de sémantique) : une `Frame` par surface ouverte, ses commandes en `PushButton`, `MenuItem`, `ListItem`, `Label`… nommées par les libellés et infobulles déjà affichés. Recalculé après chaque mise à jour qui change les surfaces, publié au pont seulement s'il diffère (`ChildrenChanged`, `PropertyChange:accessible-name`).
4. **Focus et actions** : le focus clavier existant (lanceur, dock au clavier, menus de la barre, parcours au clavier des Réglages) émet `org.a11y.atspi.Event.Object.StateChanged`, détail « focused », corps `(siiva{sv})`, depuis le chemin du nœud (l'ancien `Event.Focus` n'est plus écouté par Orca) ; la surface qui prend le clavier émet `Event.Window.Activate` (et `Deactivate` en le rendant) et porte l'état `Active` : sous Wayland, Orca n'a pas d'autre moyen de connaître la fenêtre active. `org.a11y.atspi.Action` complet (`NActions`, `GetName`, `GetLocalizedName`, `GetDescription`, `GetKeyBinding`, `GetActions`, `DoAction`) ; `DoAction` renvoie le `Msg` du clic par un canal du pont vers l'interface (abonnement iced).

## Réutilisation

- `zbus::interface`, le démarrage des serveurs D-Bus du démon (abonnement, comme les notifications) ; `crate::i18n::tr` pour les noms ; les libellés des modules (`bar_item_key`, infobulles), du dock (`AppInfo::name`), des Réglages (titres des pages et des rangées).
- Exemple : `src/notifications.rs` (serveur, test `serveur_sur_le_bus` lancé par `scripts/verify.sh` sous `dbus-run-session`).

## Changements

Voir « Étapes ».

## Portée

- Hérite : toutes les surfaces du démon, au fil des étapes.
- Vérifier : aucun coût sans lecteur d'écran (pas de connexion au bus d'accessibilité s'il est absent) ; aucun blocage de l'interface (le pont vit dans une tâche, l'interface lui envoie des instantanés).
- Exclure : les binaires à part (greeter, mymfa) ; le texte riche (`org.a11y.atspi.Text`), hors de ce plan.

## Étapes

1. **Modèle et racine servie (1a)** : `src/a11y/model.rs` (rôles et états AT-SPI, nœud, arbre aplati, chemins, rang dans le parent, ensemble d'états en deux mots) et `src/a11y/bus.rs` (`start(conn)` sert la racine `Application` + `Accessible`, `Id` en écriture, puis appelle `Embed` sur le registre et garde la référence rendue comme `Parent`). Tests : modèle ; sous `dbus-run-session`, un test ignoré qui fait tourner un **faux registre** (`org.a11y.atspi.Socket.Embed`) sur une seconde connexion, vérifie l'`Embed`, pose `Id` par `Properties.Set`, relit `Name`, `GetRole`, `GetChildren`, `Parent` ; ajouté à `scripts/verify.sh`. Le démon ne le lance pas encore.
2. **Démarrage dans le démon (1b)** : suivi de `org.a11y.Status.IsEnabled` et `ScreenReaderEnabled` (`PropertiesChanged`), adresse (`AT_SPI_BUS_ADDRESS` ou `GetAddress`), connexion, `start`, `Unembed` à l'arrêt ou à la désactivation, nouvel `Embed` si le registre renaît (`NameOwnerChanged`). Test : la logique de décision (activé, désactivé, registre parti) sur un modèle pur.
3. **Barre et dock, arbre statique** : `App::a11y_tree` donne une `Frame` par barre (« Barre des menus ») et par dock (« Dock »), leurs modules et applications en `PushButton` nommés (libellé ou infobulle) ; le pont enregistre et retire les objets des nœuds ; `hyprdesk a11y dump` imprime l'arbre (test d'instantané).
4. **Différence et événements de structure** : publication par différence (`ChildrenChanged`, `PropertyChange:accessible-name`), sans recréer les objets inchangés.
5. **Focus, lanceur et dock** : `StateChanged:focused` (états `Focusable`, `Focused`) sur le nœud actif, `Window:Activate`/`Deactivate` et état `Active` de la surface qui prend le clavier.
6. **Focus, menus et commutateur** : menus de la barre en `Menu`/`MenuItem`, commutateur de fenêtres, mêmes événements. Scindée : **6a** structure des menus (surface active, sous-menu déplié) et commutateur (fenêtre choisie au focus) ; **6b** focus clavier à l'intérieur des menus : l'`App` retient la ligne choisie au clavier pour la marquer `Focused` ; **6c** le dock logé dans la barre (style Graphite, `launcher_in_bar`) : sa `Frame` dans celle de la barre, ses cases nommées, la case choisie au clavier au focus, comme le dock seul.
7. **Actions** : `org.a11y.atspi.Action` complet sur les boutons et entrées de menu, `DoAction` par le canal vers l'interface ; `Component.GetExtents` des surfaces.
8. **Réglages et verrouillage** : pages, rangées et commandes des Réglages (`PageTab`, `PushButton`, `CheckBox` ; curseurs en `Slider` avec l'interface `Value` dans une étape à part si le temps manque), champ du mot de passe (`PasswordText`, nom « Mot de passe » ; sans l'interface `Text`, la frappe n'est pas annoncée : hors de ce plan, à dire au guide).
9. **Documentation** : guides (FR, EN) — Orca lit les surfaces de hyprdesk, et ce qu'il ne lit pas encore ; `DESIGN.md` : toute commande nouvelle a un nom accessible ; l'écart passe `fait`.

État : plan écrit et relu par un agent indépendant (corrigé : démarrage par `org.a11y.Status`, ordre racine puis `Embed`, `Id` en écriture, événements `StateChanged` et `Window:Activate` au lieu de `Focus`, `Action` complet, étapes 1 et 2 scindées, faux registre pour le test). Étape 1 faite (`7e38706`, relue par un agent indépendant : la locale annoncée vient de `i18n::current()`, pas de `LANG`) ; étape 2 faite (`e3eab1f`, relue par un agent indépendant, corrigée : tout changement de propriétaire du registre refait l'`Embed`, pause après fermeture du bus d'accessibilité, lecture d'`org.a11y.Status` sans activation, suivi des propriétés repris de `sources`). Pas d'`Unembed` à l'arrêt du démon : le registre retire l'application quand sa connexion tombe. Étape 3 faite (`8e9c555`, relue par un agent indépendant, corrigée : composition portrait sur un écran vertical, `a11y dump` sans réponse de l'interface rend « démon occupé », drapeau du pont remis à faux quand sa tâche s'arrête, objets comptés dès qu'ils sont servis). Étape 4 faite (`54ca074`, relue par un agent indépendant, corrigée : objets servis ou retirés d'après ce qui est servi, un échec se rattrape au remplacement suivant ; rang changé annoncé ; arbre recalculé seulement sur `Damage::All`). Étape 5 faite (`8017731`, relue par un agent indépendant, corrigée : mode du lanceur repris de sa vue, invite du mode pour nom du champ, focus au champ sans résultat choisi, `Deactivate` sous l'ancien nom et avant le retrait annoncé, perte du focus avant celle de la fenêtre). Reste hors de l'étape 5 : le dock logé dans la barre (style Graphite) n'a pas de Frame. Étape 6a faite (`fe7294d`, relue par un agent indépendant, corrigée : nom du menu tiré du titre ou du libellé du module, jamais d'une clé interne ; commutateur vide ou en lecture nommé par le texte affiché). Étape 6b faite (`8d018fa`) : les menus de la barre n'avaient aucun parcours au clavier, l'`App` le tient désormais (`Popover::keyed`, `keyed_sub`) ; relue par un agent indépendant, corrigée (menu Aide : le champ garde ses touches, haut et bas passent aux entrées ; présélection seulement si la barre de cet écran tient le clavier ; choix revalidé quand un menu de la zone de notification se rafraîchit ; flèche gauche referme un sous-menu ouvert au survol). Prochaine : étape 6c.

## Validation

- Produit : Orca annonce « Dock, Firefox, bouton » au clavier, le nom de la page des Réglages, le champ du mot de passe.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh` → 0 à chaque étape ; test sous `dbus-run-session` dès l'étape 1.

## Conditions d'arrêt

- S'arrêter et marquer `bloqué : dépendance` si une étape exige une fonctionnalité de `zbus` non activée ou un crate nouveau (`atspi`, `accesskit`).

## Documentation

- Étape 6.
