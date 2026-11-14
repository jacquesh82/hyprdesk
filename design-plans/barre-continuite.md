# Continuité — une barre calme, des interactions tangibles

Direction UI/UX retenue et implémentée dans l’application native le 13 novembre 2026. Spécification initialement écrite contre le commit `1940626d18fd0ba0bc00a843ccc4bbcdd5fe95df`.

## Réalisation native

- `src/ui/bar_motion.rs` : transformations peintes, capsule et survol partagés ; interpolation réversible, cibles fixes et arrêt des redessins au repos.
- `src/ui/bar_layout.rs` : mesure intrinsèque, centre géométrique, priorités et débordement accessible. Les longues listes de workspaces gardent un accès à la liste complète et l'actif reste visible.
- `src/ui/popover.rs` : ancres mesurées, surface conservée lors des changements, fondu croisé des contenus, fermeture réversible et dernière image validée même après une présentation lente. Le capteur de clic extérieur laisse la barre recevoir ses événements.
- `src/ui/controls.rs` et `src/widgets/controls.rs` : widget intégré du catalogue, sur le bureau ou dans la barre, composant les plugins existants, son/luminosité natifs et modes Bureau ; état confirmé, attente et erreurs. Le contrat Lua facultatif `on_toggle` / `toggled` ne remplace pas les clics ni les menus existants. Les sources sont résolues après tous les chunks pour réutiliser aussi les widgets déclarés après la barre.
- Palette, police et hauteur utilisateur conservées ; encre d'accent adaptée aux palettes claires et sombres. `controls` est ajouté aux nouveaux exemples et à la première activation, et reste activable dans Réglages → Barre → Modules pour une configuration existante.
- Dix nouvelles scènes natives, sans commandes système en aperçu, et documentation française/anglaise actualisée.

Les sections suivantes conservent la spécification de référence. Les vérifications natives sont consignées en fin de document ; la maquette HTML reste une illustration de l'intention.

## Chaîne de preuves et périmètre

- Surface : barre layer-shell par écran, workspaces, commandes système et menus.
- Parcours réel : `src/ui/mod.rs:4552` → `App::view_bar` → `bar_zone` → `bar_item` / `module` ; `Msg::OpenPopover` → `App::open_popover` → `view_popover`.
- État actuel : trois zones ; mode compact automatique sous 1600 px ; rognage des zones latérales ; capsules de survol indépendantes ; ouverture des menus avec échelle 0,98 → 1 et translation de 4 px ; fermeture immédiate des surfaces ; positionnement du menu autour du curseur.
- Sources de design : `DESIGN.md`, `src/ui/design.rs`, capture de référence `assets/screenshots/bar.png` inspectée pendant cette proposition.
- Contrats actuels : palette Lua conservée ; commandes de rayon 10 px, cartes 20 px ; une famille de police ; accent réservé aux sélections/actions ; retour au pressage ; mouvement désactivable ; menus mesurés et bornés à l’écran.
- Cette proposition répond à une demande explicite de nouvelle direction et d’animation. Les nouvelles décisions ci-dessous sont des propositions, pas des violations attribuées au produit actuel.
- Propriétaires : `src/ui/bar.rs`, `src/ui/popover.rs`, `src/ui/design.rs`, `src/ui/mod.rs`, `src/ui/plugins/bar.rs`, `src/config/model.rs`, `src/config/lua.rs`, `src/settings.rs`, `src/ui/preview.rs`.
- Incertitudes : contraste et flou derrière de vraies fenêtres ; coût des animations dans Iced ; mesure des ancres de chaque module ; ajustement final de densité avec les modules réels et une zone de notification chargée. La maquette navigateur ne les valide pas.

## Décision de design

Une barre fine bord à bord, avec trois groupes visuels stables. Les transitions relient un état au suivant : la sélection se déplace, le survol accompagne le pointeur et le menu apparaît depuis sa commande. Le centre de contrôle donne accès aux réglages fréquents, avec une hiérarchie explicite entre état, action et détails.

L’inspiration Apple porte sur la retenue, la précision typographique, les matières et la continuité des gestes. L’identité reste celle de hyprdesk : Mindlog, accent iris, organisation par workspaces, configuration Lua et icônes système/symboliques.

### Composition et matière

- Démonstrateur à 44 px de hauteur ; respecter la hauteur utilisateur de 24 à 64 px dans le produit. Les proportions doivent fonctionner à 36, 44 et 48 px avant de retenir un défaut.
- Barre sans marge extérieure ; séparation basse d’un pixel discret. Verre fumé d’opacité issue du thème ; une lumière de bord et une ombre douce. La maquette utilise un fond opaque pour représenter la matière de manière reproductible ; le flou réel appartient à Hyprland.
- Gauche : workspaces avec une seule capsule mobile, puis titre d’application secondaire. Le `h.` décoratif de la maquette est un repère conceptuel, pas une nouvelle commande ni un logo à introduire sans décision de marque.
- Centre : heure tabulaire de graisse medium ; date secondaire quand l’espace le permet. Le centre reste géométriquement centré même si les zones latérales diffèrent.
- Droite : accès directs aux états essentiels configurés, puis centre de contrôle. Les modules personnalisés et la zone de notification restent configurables ; les regroupements proposés ne les suppriment pas.
- Commandes : `design::SMALL_RADIUS` (10 px). Cartes : `design::RADIUS` (20 px). Espacement intérieur de carte 16–20 px ; groupes 8–12 px ; icônes 16–18 px, sur les boîtes de clic existantes.
- Texte courant normal, libellés medium, titres semibold. Réserver le gras aux cas justifiés. Utiliser `App::ui_font` plutôt qu’appliquer `title_font()` à tous les libellés de barre. Mindlog et les choix de famille utilisateur restent prioritaires ; la maquette emploie une police système de substitution.
- Accent iris pour la sélection, les curseurs et les connexions activées. Texte principal issu de `fg`, texte secondaire suffisamment couvrant ; avertissements/états critiques conservent leurs couleurs sémantiques.

### Architecture des menus

- Ajouter un module optionnel « Centre de contrôle ». Ne pas remplacer d’office la configuration Lua existante. Son activation et son emplacement passent par Réglages → Barre → Modules.
- Contenu : Wi-Fi/Bluetooth en premier, luminosité/son ensuite, puis mode Bureau et état batterie. Masquer un contrôle matériel indisponible plutôt qu’afficher une commande inopérante. Fournir les états chargement, indisponible et erreur à proximité de la commande.
- Réutiliser les sources d’état et les actions déjà possédées par les modules réseau/Bluetooth/audio/luminosité. Avant réalisation, tracer leurs propriétaires exacts. Le centre de contrôle compose ces données ; il ne lance pas un deuxième service de surveillance.
- Son reste accessible directement depuis sa propre icône : volume, sortie choisie, autres périphériques. Même principe pour réseau et calendrier.
- Ancrer la carte au rectangle rendu de son bouton, pas à la position du curseur. Garder un écart de 8–10 px ; aligner près du bouton et borner à 12–16 px du bord de l’écran. En bas, inverser origine et translation. Les déclencheurs clavier utilisent la même ancre.
- Un seul menu actif par écran concerné ; le bouton reste sélectionné tant que sa carte est ouverte. Un second clic ferme la carte. Échap et clic extérieur ferment ; le retour du focus dépend du mode d’ouverture. Naviguer vers une autre commande ne laisse aucune carte orpheline.
- Mesurer la hauteur réelle du contenu avec le mécanisme `PopoverMeasured` / `fit_popover`. Conserver les menus longs défilants. La transition ne doit pas animer une hauteur estimée ni redimensionner la fenêtre à chaque image.

### Chorégraphie proposée

| Geste | Réponse | Durée cible | Règle |
| --- | --- | --- | --- |
| Entrer sur une commande | Fond neutre discret, un seul highlight mobile dans sa zone | 100–140 ms | Ne modifie ni la taille ni la position des cibles |
| Passer à la commande voisine | Le highlight glisse vers sa nouvelle boîte | 140 ms | Utiliser la position actuellement affichée comme départ |
| Presser | Compression visuelle 0,97 + fond plus dense | 80–90 ms | Le contenu peint change ; la boîte de clic reste fixe |
| Changer de workspace | Capsule iris vers la nouvelle position | 180 ms | Les numéros et les zones restent immobiles |
| Ouvrir un menu | Opacité 0 → 1, échelle 0,975 → 1, translation de 6 px | 180 ms | Origine au bouton, aucune oscillation |
| Changer de menu ouvert | Déplacement court et fondu croisé du contenu | 160 ms | Bornage final immédiat ; pas de fenêtre intermédiaire hors écran |
| Fermer | Opacité 1 → 0, retrait de 3 px, échelle 1 → 0,985 | 120 ms | Détruire les surfaces après la sortie ; une réouverture l’inverse |
| Changer une connexion | État textuel et aplat d’icône mis à jour | 120–140 ms | Le retour système fait foi ; attente distincte d’un succès |
| Régler un curseur | Valeur et remplissage suivent la saisie | Immédiat | Ne pas interpoler la valeur réelle du réglage |

Courbe de déplacement sans rebond, de type décélération : `cubic-bezier(.2,.8,.2,1)` dans la maquette ; choisir l’équivalent dans le moteur existant. Aucune animation permanente au repos. `transition = "none"` supprime déplacement, échelle et interpolation ; tous les états restent explicites et les actions immédiates.

Le prototype illustre l’ouverture, la fermeture et le déplacement court entre cartes. Il ne conserve pas les deux contenus lors d’un changement de menu : le fondu croisé des contenus est à réaliser dans le produit et à valider séparément.

### Adaptation et densité

- Décider de la compacité à partir de la largeur disponible et de la taille mesurée des groupes, en conservant la préférence utilisateur `auto / always / never`.
- Réduire d’abord le titre de fenêtre, puis la date, puis les pourcentages non essentiels. Garder l’heure et les commandes de contrôle.
- En mode automatique, les modules secondaires qui ne tiennent plus doivent rester accessibles dans un menu de débordement explicite, dans leur ordre configuré. Ne pas rogner silencieusement une commande interactive. Priorités de conservation : centre de contrôle, heure, workspace actif, commandes marquées prioritaires ; conserver une entrée vers les autres workspaces si le groupe déborde.
- Cette gestion du débordement demande une évolution de composition : ne pas simuler son existence en ajoutant seulement une ellipse décorative.
- La maquette masque réseau/batterie directs aux petites largeurs ; leurs états figurent dans le centre de contrôle. Elle ne représente pas le débordement de tous les modules Lua possibles.
- Écrans portrait : respecter les zones `portrait` configurées, appliquer la même matière et la même chorégraphie. En multi-écran, vérifier sélection et ancrage sur l’écran du déclencheur.

## Réutilisation et nouveaux composants nécessaires

- Palette : `App::bar_palette`, `App::bar_item_color`, `App::state_color`. Ne pas coder la palette de la maquette en dur dans Rust.
- Géométrie : `design::SMALL_RADIUS`, `design::RADIUS` ; variantes de matière dans `src/ui/design.rs`.
- Police et icônes : `App::ui_font`, `App::icon_named`, système de secours de `src/ui/icons.rs`.
- Composition : conserver `view_bar`, `bar_zone`, `bar_item` et `module` comme propriétaires ; composer les contrôles audio/luminosité existants.
- Menus : `Request`, `Kind`, `Popover`, `PopoverMeasured`, `fit_popover`, catcher et fermeture existants dans `src/ui/popover.rs`.
- Deux ajouts sont nécessaires : une ancre mesurée par module, car le curseur actuel ne décrit pas son rectangle ; un état de transition de menu réversible, car `close_popover` détruit immédiatement ses fenêtres. Les placer dans les propriétaires existants, sans créer une seconde pile de menus.
- Un centre de contrôle et un débordement constituent de nouvelles compositions. Avant de coder, tracer les contrats des plugins concernés ; arrêter uniquement ces sous-parties si une action ne peut pas être réutilisée proprement.

## Étapes de réalisation

1. `src/ui/design.rs` : définir la variante de barre et de carte en réutilisant palette et rayons. Vérifier fonds clair, sombre et contrasté, sans multiplier les ombres.
2. `src/ui/bar.rs` : appliquer la hiérarchie typographique, mesurer les boîtes rendues, dessiner capsule et highlight derrière les commandes, rétablir le retour pressé des modules actifs. Conserver clics gauche/droit/milieu, molette et glisser-déposer de configuration.
3. `src/ui/popover.rs` et `src/ui/mod.rs` : porter l’ancre et l’état de transition ; ouverture/fermeture réversibles, origine dépendante du bord ; annuler les tâches de transition périmées. Ne pas recréer les surfaces de barre lors d’un survol.
4. Modules, configuration et réglages : ajouter le centre de contrôle optionnel et le débordement explicite, avec une migration qui conserve les zones existantes. Relever les propriétaires de sérialisation avant modification.
5. `src/ui/preview.rs` : ajouter des scènes reproductibles `bar-idle`, `bar-hover`, `bar-pressed`, `bar-workspace-switch`, `bar-controls`, `bar-menu-switch`, `bar-menu-closing`, `bar-bottom`, `bar-overflow`, `bar-still`. Les données restent locales et les actions sans effet sur la session.

## Périmètre

- Héritent : modules natifs, widgets Lua et icônes de notification passant par `module`, workspaces et menus de barre.
- À vérifier : boutons du mode Bureau, infobulles, menus longs, trois clics, molette, configuration par déplacement, couleurs par module, titres longs, périphériques multiples, écrans débranchés, barre basse, portrait, DPI fractionnaire.
- Hors périmètre : refonte de LYRA, bureau, dock, écran de connexion ou services réseau/audio. Les autres surfaces pourront adopter les règles retenues après validation de la barre.

## Validation

- Produit : ouvrir Son, régler le volume, changer la sortie, passer au réseau puis fermer ; changer rapidement de workspace ; interrompre et inverser chaque animation. L’état affiché et l’état système restent synchronisés.
- Interface : captures à 1440, 900 et 640 px, facteurs 1 / 1,25 / 1,5 / 2 ; palette claire/sombre/personnalisée ; modules nombreux, labels longs, notifications critiques ; vérifier aucune cible masquée sans accès de remplacement.
- Mouvement : séquences enregistrées à 60 Hz ; viser le budget de 16,7 ms par image sur le matériel de référence, sans prétendre que la maquette le prouve. Vérifier l’absence de ticks/reconstruction de contenu au repos. Suspendre les animations hors visibilité ; une seule autorité de transition pour éviter le cumul Iced/compositeur.
- Système : parcourir à la souris et au clavier ; Échap, clic extérieur, focus restauré ; `transition = "none"` immédiat ; aucune ouverture de carte au simple survol.
- Dépôt, lors de l’implémentation native : `bash scripts/verify.sh --visual /tmp/hyprdesk-continuite-native` → format, Clippy, tests, build et captures sans régression ; examiner les captures et les enregistrements en session Hyprland.
- La maquette interactive conserve des données de démonstration. La réalisation native et ses vérifications sont décrites ci-dessous ; les mesures de cadence sur le matériel réel restent distinctes des captures isolées.

## Vérifications de la maquette effectuées

- Chromium : clair et sombre, fenêtres de 1100, 760, 640, 390 et 320 px (largeur utile réduite par les marges du conteneur d’aperçu). Aucun débordement horizontal ni chevauchement entre boutons de barre visibles ; carte de contrôle contenue.
- Interactions : volume, sortie audio, Wi-Fi, modes du bureau, workspace, calendrier, fermeture Échap avec retour de focus, réouverture. Aucun `pageerror` pendant ces parcours.
- Mouvement réduit : changement de menu sans animation active ; captures sombre, claire et étroite inspectées.
- Ces vérifications portent sur la proposition interactive, sans connexion aux services système. Les contrôles natifs de performance, de flou, de matériel et de multi-écran restent à réaliser.

## Conditions d’arrêt

- Si les ancres des boutons ne peuvent pas être mesurées dans la composition actuelle, résoudre cette mesure avant de figer les transitions. Ne pas revenir à des coordonnées de curseur comme approximation silencieuse.
- Si le regroupement du centre de contrôle exige de réécrire un service, maintenir les accès individuels et spécifier d’abord un contrat de composition ; ne pas élargir la refonte aux services.
- Si le coût de mouvement dépasse le budget, conserver la hiérarchie, les cibles et le fondu, puis réduire la complexité de dessin. Ne pas ajouter de flou animé.

## Documentation après validation

Composition, typographie, matière, durées, ancrage, débordement et comportement sans mouvement consignés dans `DESIGN.md`. Exemples et annotations Lua français/anglais, référence des plugins et documentation utilisateur actualisés.

## Vérifications natives effectuées

- `cargo clippy --locked --all-targets -- -D warnings` et `cargo build --locked` : réussis sur la version finale.
- `cargo test --locked --quiet` : **371 réussis, 0 échec, 6 ignorés**. Les tests couvrent notamment partage des sources, handles de toggle périmés, géométrie du débordement, interpolation interrompue, ancrage sans pointeur et recalage après une mesure tardive, réouverture pendant la fermeture et validation de la dernière image après une présentation lente.
- `bash scripts/verify.sh --visual /tmp/hyprdesk-continuite-validation` : terminé avec succès, dont les deux contrôles D-Bus isolés et **261 captures** à 1440, 900 et 640 px. Journal : `/tmp/hyprdesk-continuite-verify.log`.
- Dernière passe ciblée : dix états à 1440 et 640 px, palette claire à 900 px et échelle 1,5 à 900 px, sous `/tmp/hyprdesk-continuite-final/`. Les poses statiques sont figées à leur état final dans l'export d'aperçu ; les scènes de geste jouent les transitions réelles. La version interactive conserve les apparitions réelles.
- Gestes exercés avec XTest dans une fenêtre Xvfb isolée : ouverture du centre de contrôle, toggle Wi-Fi de démonstration, passage au son, Échap, changement de workspace, clic extérieur. Captures : `/tmp/hyprdesk-continuite-interactions/`.
- Cas dense exercé : 40 workspaces, actif visible, ouverture de la liste complète, défilement jusqu'en bas, sélection du workspace 40 et fermeture de la carte. Captures : `/tmp/hyprdesk-continuite-overflow-qa/`.
- Limites : les captures Xvfb ne reproduisent pas le flou Hyprland. Le rendu du compositeur, les périphériques physiques et la cadence sur le matériel de la session n'ont pas été mesurés par ces vérifications. Aucun réglage ni programme de la session active n'a été modifié par les aperçus.

### Correction de l’architecture du centre de contrôle

`controls` appartient au registre des widgets intégrés et utilise `WidgetKind::Controls`. Aucun `BarItem::Controls` ni module spécial de barre ne subsiste. Le chargement Lua, le catalogue, le placement, la sauvegarde et la désactivation suivent le parcours des autres widgets. Les sources partagées sont créées après les désactivations et relevées seulement quand leur widget utilisateur est visible. Le menu utilise le conteneur générique des menus de widgets ; l’ancrage des sous-menus suit leur parent.

Validation de cette correction : `cargo clippy --locked --all-targets -- -D warnings` ; suite Rust : 374 réussites, 6 ignorés. Tests dédiés au catalogue, à la désactivation, à la visibilité, aux sources partagées et à la sauvegarde/relecture du widget sur le bureau et dans la barre. Huit captures natives à 1440/640 px, dont `controls-desktop`. Gestes vérifiés sous Xvfb : ouverture, interrupteur Wi-Fi de démonstration, changement de menu, Échap, workspace et clic extérieur.
