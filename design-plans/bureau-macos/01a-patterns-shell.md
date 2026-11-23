# Patterns : comportements du shell — macOS d'après la HIG, face à hyprdesk

> Proposition de design, rien d'implémenté. Plan d'ensemble et priorités : `plan-d-action.md` ; méthode et périmètre : `README.md`.

> Référentiel : Human Interface Guidelines d'Apple, export Markdown officiel relevé le 23 novembre 2026. Code : commit `00f6d69` (arbre de travail du 23 novembre 2026, avec des modifications non commitées).

## Première partie — Spécification d'après la HIG

### Résumé

- Corpus : dix pages de patterns généraux, écrites du point de vue d'une app ou d'un jeu ; le bureau n'y apparaît que par les blocs macOS et quelques exemples (Finder, Dock, menus, fenêtres).
- Réglages (Settings) : des valeurs par défaut qui conviennent au plus grand nombre, peu de réglages, les options de tâche au plus près de la tâche ; sur macOS, fenêtre à volets ouverte depuis le menu de l'app, à barre d'outils fixe et titre suivant le volet.
- Recherche (Searching) : un point d'entrée unique par app, un périmètre affiché, des suggestions, un historique effaçable ; Spotlight trouve du contenu dans toutes les apps et sur le web.
- Notifications (Managing notifications) : permission préalable, niveaux d'interruption définis par le système, modes Concentré (Focus) et livraison programmée, jamais de marketing au niveau sensible au temps.
- Retour, chargement, modalité : un retour proportionné à l'importance de l'information, quelque chose d'affiché dès que possible, une vue modale réservée aux cas utiles et facile à refermer.
- Multitâche et plein écran : toute app doit savoir enregistrer et restaurer son contexte ; sur Mac le multitâche est la norme ; en plein écran les contrôles système se masquent, mais le Dock reste atteignable hors jeux.
- Lancement et annulation : démarrer sans attendre et restaurer l'état précédent ; Annuler et Rétablir figurent en haut du menu Édition, avec leurs raccourcis standards.
- Limites du corpus : plusieurs sujets attendus ne figurent pas dans ces pages (raccourci et résultats de Spotlight, Centre de notifications, bannières et alertes, regroupement, espaces, Stage Manager, Mission Control, lancement depuis le Dock). Chaque section le signale ; rien n'est complété par une autre source.
- Cotations : aucune taille ni aucun délai chiffré ; les seules valeurs sont des raccourcis, des seuils en langage naturel et des noms de réglages ou de commandes.

### Settings (`settings`)

**Rôle** : Définir quels réglages offrir, où les placer (app Réglages du système, zone de réglages de l'app ou cœur de la tâche) et comment les présenter pour que l'expérience fonctionne d'emblée.

**Anatomie et cotes**
- Fenêtre de réglages (custom settings window) sur macOS : une barre d'outils (toolbar) dont chaque bouton ouvre un volet (pane) regroupant des réglages apparentés — "a toolbar that includes buttons for switching between views — called *panes* — that each contain a group of related settings" — settings › Platform considerations › macOS
- La fenêtre épouse la taille du volet courant ; ses boutons réduire et agrandir sont désactivés (détail en Spécifique macOS).
- Tailles, espacements, nombre de volets, largeur de la barre d'outils, durées : non chiffré dans la HIG.

**Règles**
- **Valeurs par défaut** : viser des réglages par défaut qui donnent la meilleure expérience au plus grand nombre, afin que personne n'ait à ajuster avant de commencer — "Aim to provide default settings that give the best experience to the largest number of people." — settings › Best practices
- **Parsimonie** : réduire le nombre de réglages ; trop de réglages rend l'expérience moins accessible et un réglage donné difficile à trouver — "Minimize the number of settings you offer." — settings › Best practices
- **Voies attendues** : offrir les réglages par les moyens attendus ; avec un clavier physique connecté, le raccourci standard Commande-Virgule (Command-Comma) ouvre les réglages d'une app — "when a physical keyboard is connected, people often use the standard Command-Comma (,) keyboard shortcut to open an app’s settings" — settings › Best practices
- **Détection plutôt que saisie** : ne pas demander par un réglage une information d'installation obtenable autrement (contrôleur connecté, mode sombre en cours) — "Avoid using settings to ask for setup information you can get in other ways." — settings › Best practices
- **Réglages globaux** : respecter les réglages système (accessibilité, défilement, méthodes d'authentification) et ne pas en proposer de doublon dans une zone de réglages personnalisée — "Respect people’s systemwide settings and avoid including redundant versions of them in your custom settings area." — settings › Best practices
- **Réglages généraux** : placer dans la zone de réglages les options générales qui changent rarement (configuration des fenêtres, comptes, comportement de sauvegarde) — "Put general, infrequently changed settings in your custom settings area." — settings › General settings
- **Options de tâche** : proposer les options propres à une tâche (afficher ou masquer une partie de la vue, réordonner, filtrer) dans l'écran qu'elles affectent, non dans la zone de réglages — "When possible, prefer letting people modify task-specific options without going to your settings area." — settings › Task-specific options
- **Réglages système d'une app** : n'ajouter à l'app Réglages fournie par le système que les options qui changent le plus rarement ; un bouton peut l'ouvrir directement — "Add only the most rarely changed options to the system-provided Settings app." — settings › System settings

**Spécifique macOS**
- Accès : l'élément Réglages figure dans le menu de l'app (App menu) et ouvre la fenêtre de réglages personnalisée ; les options propres à un document vont dans le menu Fichier (File menu) — "When people choose the Settings item in your app’s or game’s App menu, your custom settings window opens." — settings › Platform considerations › macOS
- Pas de bouton dédié : éviter un bouton Réglages dans la barre d'outils d'une fenêtre, qui retirerait de la place aux commandes essentielles — "Avoid adding settings buttons to a window’s toolbar, because doing so decreases the space available for essential commands that people use frequently." — settings › Platform considerations › macOS
- Boutons de fenêtre : réduire et agrandir sont désactivés, car la fenêtre s'ouvre par Commande-Virgule, n'a pas besoin de rester dans le Dock et épouse la taille du volet courant — "Dim a settings window’s minimize and maximize buttons." — settings › Platform considerations › macOS
- Barre d'outils : non personnalisable, toujours visible, avec le bouton actif toujours indiqué — "In your settings window, use a noncustomizable toolbar that remains visible and always indicates the active toolbar button." — settings › Platform considerations › macOS
- Titre : le titre de la fenêtre reflète le volet visible ; sans volets multiples, il vaut *App Name* Settings — "If your settings window doesn’t have multiple panes, use the title *App Name* Settings." — settings › Platform considerations › macOS
- Mémoire : rouvrir sur le dernier volet consulté, car on ajuste souvent plusieurs fois des réglages liés — "Restore the most recently viewed pane." — settings › Platform considerations › macOS
- Absent de la page : champ de recherche ou indexation des réglages, organisation interne des volets, réglages propres au bureau (Bureau et Dock, écrans) ; rien à en tirer.

**À transposer dans un shell Linux**
- Fenêtre Réglages de hyprdesk : barre d'outils de volets fixe et toujours visible, volet actif marqué, titre de fenêtre suivant le volet, dernier volet restauré à l'ouverture.
- Réduire le nombre de réglages : des valeurs par défaut tenant lieu de configuration initiale, et les options propres à une surface (affichage d'un widget, filtre d'une liste) placées dans cette surface plutôt que dans Réglages.
- Un seul endroit par réglage global : ne pas dupliquer dans le shell ce que le bureau ou les apps définissent déjà par ailleurs.
- Un raccourci clavier unique et documenté pour ouvrir les Réglages, avec une entrée équivalente dans les surfaces du shell qui offrent un menu.
- Recherche dans les Réglages : la HIG ne la traite pas ; la concevoir sans référence macOS en reprenant les règles de la page Searching (périmètre affiché, suggestions, historique effaçable).

### Searching (`searching`)

**Rôle** : Permettre de trouver du contenu dans une app, dans un document et à l'échelle du système, avec le moins de frappe possible.

**Anatomie et cotes**
- Élément de base : un champ de recherche (search field) dans l'app ; une barre de périmètre (scope bar) et des jetons (tokens) peuvent limiter la recherche à des attributs tels que date de création, taille ou type de fichier.
- Recherche système : l'outil Spotlight, qui aide à trouver du contenu dans toutes les apps du système et sur le web — "In iOS, iPadOS, and macOS, Spotlight helps people find content across all apps in the system and on the web." — searching › Discussion
- L'expérience peut se personnaliser : recherches récentes, suggestions, compléments ou corrections fondés sur les termes déjà cherchés dans l'app.
- Taille du champ, nombre de résultats, délai d'affichage, raccourci d'ouverture, ordre des résultats : non chiffré dans la HIG.

**Règles**
- **Position** : si la recherche est importante, lui donner une position principale dans l'app ou la vue — "If search is important, give it a primary position in your app or view." — searching › Best practices
- **Point unique** : rendre le contenu de l'app cherchable depuis un seul emplacement clairement identifié ; une recherche locale subsiste dans les sections distinctes, comme filtre de la vue courante — "Aim to make your app’s content searchable through a single location." — searching › Best practices
- **Périmètre** : afficher clairement le périmètre courant par un texte indicatif, une barre de périmètre ou un titre — "Clearly display the current scope of a search." — searching › Best practices
- **Suggestions** : afficher les recherches récentes avant la frappe et des suggestions prédictives pendant la frappe — "offer predictive search suggestions while they’re typing, you can help people search faster and type less" — searching › Best practices
- **Confidentialité** : peser la vie privée avant d'afficher l'historique ; si on l'affiche, offrir un moyen de l'effacer — "If you do show search history, provide a way for people to clear it if they want." — searching › Best practices
- **Indexation** : rendre le contenu de l'app indexable dans Spotlight, en fournissant des métadonnées descriptives — "Make your app’s content searchable in Spotlight." — searching › Systemwide search
- **Types de fichiers** : définir les métadonnées des types de fichiers personnalisés et fournir un générateur d'aperçu (Quick Look generator) pour que Spotlight et les autres apps montrent des aperçus — "Implement a Quick Look generator if your app produces custom file types." — searching › Systemwide search
- **Ouverture et enregistrement** : préférer les vues système d'ouverture et d'enregistrement, qui contiennent un champ de recherche — "The system-provided open and save views generally include a built-in search field that people can use to search and filter the entire system." — searching › Systemwide search

**Spécifique macOS**
- Depuis une app : une app peut lancer une recherche Spotlight à partir de la sélection courante, puis afficher les résultats dans une vue personnalisée — "Use Spotlight to offer advanced file-search capabilities within the context of your app." — searching › Systemwide search
- Dans un document : la Discussion évoque la recherche du contenu d'une fenêtre ou d'une page ouverte — "implementing ways to find content in a window or page in your iOS, iPadOS, or macOS app" — searching › Discussion
- Bloc Platform considerations : aucune considération supplémentaire pour macOS — "No additional considerations for iOS, iPadOS, macOS, tvOS, visionOS, or watchOS." — searching › Platform considerations
- Absent de la page : raccourci clavier de Spotlight, présentation et ordre des résultats, catégories, recherche de réglages ou de commandes, recherche depuis la barre des menus (menu bar) ; rien à en tirer.

**À transposer dans un shell Linux**
- Lanceur et recherche de hyprdesk : un point d'entrée unique pour apps, fichiers, réglages et commandes du shell.
- Afficher le périmètre ou le filtre courant (tout, applications, fichiers, réglages) par un libellé, un texte indicatif ou un titre toujours visible.
- Suggestions pendant la frappe et recherches récentes avant la frappe, avec une commande d'effacement de l'historique et aucune exposition de celui-ci là où d'autres peuvent le voir.
- Indexer fichiers et réglages avec des métadonnées descriptives (nom, type, date) et associer un aperçu à chaque résultat quand il en existe un.

### Managing notifications (`managing-notifications`)

**Rôle** : Définir quand et comment interrompre : permission préalable, niveaux d'urgence définis par le système, filtrage par Concentré et livraison programmée.

**Anatomie et cotes**
- Niveaux : quatre niveaux d'interruption (interruption levels) définis par le système pour les notifications autres que les communications ; Active est le niveau par défaut — "*Active* (the default). Information people might appreciate knowing about when it arrives, like a score update on their favorite sports team." — managing-notifications › Integrating with Focus

| Niveau | Outrepasse la livraison programmée | Traverse un Concentré | Outrepasse l'interrupteur Sonnerie/Silence (iPhone et iPad) |
|---|---|---|---|
| Passive | Non | Non | Non |
| Active | Non | Non | Non |
| Time Sensitive | Oui | Oui | Non |
| Critical | Oui | Oui | Oui |

- Sens des niveaux : Passive, à consulter à loisir ; Time Sensitive, information essentielle appréciée immédiatement ; Critical, urgent et rare, typiquement gouvernemental ou lié à la santé ou au logement — "Critical notifications are extremely rare and typically come from governmental and public agencies or apps that help people manage their health or home." — managing-notifications › Integrating with Focus
- Critical exige une habilitation (entitlement) pour être envoyé — "Because a Critical notification can override the Ring/Silent switch and break through scheduled delivery and Focus, you must get an entitlement to send one." — managing-notifications › Integrating with Focus
- Durées d'affichage, dimensions, position à l'écran, délais de regroupement : non chiffré dans la HIG.

**Règles**
- **Permission** : obtenir l'autorisation avant toute notification ; la personne peut changer d'avis dans les réglages, voire imposer le silence à tout — "You need to get permission before sending any notification." — managing-notifications › Discussion
- **Confiance** : représenter fidèlement l'urgence de chaque notification, sous peine de voir toutes les notifications coupées — "Build trust by accurately representing the urgency of each notification." — managing-notifications › Best practices
- **Sensible au temps (Time Sensitive)** : réserver ce niveau à une information utile dans l'instant, un événement en cours ou à venir dans l'heure — "make sure the notification is about an event that’s happening now or will happen within an hour" — managing-notifications › Best practices
- **Première fois** : à la première notification de ce niveau venue d'une app, le système explique son fonctionnement et propose de la désactiver, puis réitère périodiquement — "The first time a Time Sensitive notification arrives from your app, the system describes how such a notification works and gives people a way to turn it off" — managing-notifications › Best practices
- **Concentré (Focus)** : filtrer les notifications sur une période réservée à une activité (dormir, travailler, lire, conduire) ; la personne désigne les contacts et apps autorisés à le traverser — "A Focus helps people filter notifications during a time period they reserve for an activity like sleeping, working, reading, or driving." — managing-notifications › Integrating with Focus
- **Livraison programmée (delivery scheduling)** : recevoir l'alerte immédiatement ou dans un résumé remis aux heures choisies — "Delivery scheduling lets people choose whether to receive notification alerts immediately or in a summary that’s delivered at times they choose." — managing-notifications › Integrating with Focus
- **Retard d'alerte** : un Concentré peut retarder l'alerte, mais la notification elle-même est disponible dès son arrivée — "Even though a Focus might delay the delivery of a notification alert, the notification itself is available as soon as it arrives." — managing-notifications › Integrating with Focus
- **Marketing** : ne pas envoyer de contenu promotionnel sans accord explicite, obtenu par une interface décrivant les types d'information et offrant un choix clair — "Get people’s permission if you want to send them promotional or marketing notifications." — managing-notifications › Sending marketing notifications
- **Jamais urgent** : une notification marketing ne doit jamais traverser un Concentré ni un réglage de livraison programmée — "such a notification must never break through a Focus or scheduled delivery setting" — managing-notifications › Sending marketing notifications
- **Gestion** : permettre aussi de gérer les réglages de notification de l'app dans l'app — "Make sure people can manage their notification settings within your app." — managing-notifications › Sending marketing notifications

**Spécifique macOS**
- Bloc Platform considerations : aucune considération supplémentaire pour macOS ; permission, niveaux, Concentré et livraison programmée y sont décrits pour toutes les plateformes — "No additional considerations for iOS, iPadOS, macOS, tvOS, or visionOS." — managing-notifications › Platform considerations
- Absent de la page : Centre de notifications (Notification Center), distinction entre bannière (banner) et alerte (alert), durées d'affichage, regroupement (grouping), position à l'écran, styles par app ; rien à en tirer.

**À transposer dans un shell Linux**
- Démon de notifications de hyprdesk : autorisation par application avant tout affichage, modifiable dans Réglages, avec un silence global.
- Classer chaque notification entrante selon les niveaux du tableau ci-dessus et appliquer ce tableau au mode ne pas déranger et à la livraison programmée.
- Modes ne pas déranger inspirés du Concentré : liste d'apps autorisées, périodes, résumé programmé pour les notifications non urgentes.
- Historique du shell : appliquer la règle Retard d'alerte, qui garantit que la notification y figure dès son arrivée, même quand l'alerte est filtrée.
- Page de gestion par app dans Réglages : autorisation, niveau d'interruption, révocation.
- Styles d'affichage (bannière, alerte persistante, durée) : non traités ici, à spécifier sans référence à cette page.

### Feedback (`feedback`)

**Rôle** : Faire savoir ce qui se passe, ce qui est possible ensuite, ce que les actions ont produit et comment éviter les erreurs.

**Anatomie et cotes**
- Le retour peut véhiculer l'état courant d'une chose, le succès ou l'échec d'une tâche importante, un avertissement avant une action aux conséquences négatives, l'occasion de corriger une erreur.
- Canaux cités : couleur, texte, son, retour haptique (haptics), lecteur d'écran VoiceOver ; l'alerte (alert) interrompt le contexte courant.
- Durées d'affichage, dimensions, positions, niveaux sonores : non chiffré dans la HIG.

**Règles**
- **Proportion** : accorder le mode de livraison à l'importance de l'information ; un état s'affiche passivement, un avertissement de perte de données interrompt — "The most effective feedback tends to match the significance of the information to the way it’s delivered." — feedback › Discussion
- **Accessibilité** : multiplier les canaux pour que le retour passe même appareil silence, regard ailleurs ou lecteur d'écran — "when you provide feedback using color, text, sound, and haptics, people can receive it whether they silence their device, look away from the screen, or use VoiceOver" — feedback › Best practices
- **État intégré** : placer l'état près des éléments qu'il décrit, sans action à mener ni contexte à quitter — "When status feedback is available near the items it describes, people get important information without having to take action or leave their current context." — feedback › Best practices
- **Alertes** : réserver les alertes aux informations critiques, idéalement actionnables ; trop d'alertes émoussent leur effet — "Alerts can lose their impact if you use them too often or to deliver unimportant information." — feedback › Best practices
- **Pertes de données** : avertir d'une perte inattendue et irréversible, jamais d'une perte attendue — "Warn people when they initiate a task that can cause data loss that’s unexpected and irreversible." — feedback › Best practices
- **Confirmation** : confirmer seulement l'achèvement d'une action significative ; on attend la réussite, il faut savoir quand elle manque — "because people typically expect their action or task to succeed, they only need to know when it doesn’t" — feedback › Best practices
- **Échec** : montrer qu'une commande ne peut pas être exécutée et expliquer pourquoi — "Show people when a command can’t be carried out and help them understand why." — feedback › Best practices

**Spécifique macOS**
- Exemple Finder : pas d'avertissement à chaque mise à la corbeille, car la suppression est le résultat attendu — "the Finder doesn’t warn people every time they throw away a file because deleting the file is the expected result" — feedback › Best practices
- Exemple Maps : sans destination distincte, Maps indique qu'elle ne peut pas calculer un trajet entre deux lieux identiques.
- Bloc Platform considerations : aucune considération supplémentaire pour macOS ; rien sur la barre des menus, le Dock ou les fenêtres — "No additional considerations for iOS, iPadOS, macOS, tvOS, or visionOS." — feedback › Platform considerations

**À transposer dans un shell Linux**
- Retours d'état du shell (volume, luminosité, changement d'espace, réseau) : appliquer la règle État intégré, en passif et près de la zone concernée quand c'est possible.
- Pas d'alerte modale pour un simple changement d'état ; la fenêtre modale se réserve aux pertes de données inattendues et irréversibles.
- Pas de confirmation bruyante des actions dont la réussite va de soi ; signaler en revanche explicitement les commandes impossibles, avec leur cause.
- Plusieurs canaux par retour : texte lisible, variation de forme ou d'icône en plus de la couleur, son en option.

### Loading (`loading`)

**Rôle** : Charger du contenu sans que l'attente se remarque, et rendre l'attente lisible quand elle est inévitable.

**Anatomie et cotes**
- Indicateurs de progression (progress indicators) fournis par le système : déterminé (determinate) quand la durée est connue, indéterminé (indeterminate) sinon.
- Seuil d'emploi : un chargement de plus d'un moment ou deux — "for situations where loading takes more than a moment or two, you can use system-provided components — called *progress indicators* — to show that loading is ongoing" — loading › Showing progress
- Dimensions, épaisseurs, périodes des animations, seuils en millisecondes : non chiffré dans la HIG.

**Règles**
- **Objectif** : la meilleure expérience de chargement s'achève avant qu'on ne s'en aperçoive (citation du chapeau de la page, placé juste au-dessus de la section Discussion) — "The best content-loading experience finishes before people become aware of it." — loading › Discussion
- **Affichage immédiat** : montrer quelque chose aussitôt (texte, graphisme ou animation provisoire) et remplacer ces éléments à mesure que le contenu arrive ; un écran vide passe pour une panne — "consider showing placeholder text, graphics, or animations as content loads, replacing these elements as content becomes available" — loading › Best practices
- **Non bloquant** : laisser faire autre chose pendant le chargement — "Loading content in the background helps give people access to other actions." — loading › Best practices
- **Attente longue** : offrir quelque chose d'intéressant à regarder et estimer le temps restant avec précision — "Gauge the remaining loading time as accurately as possible" — loading › Best practices
- **Progression** : dire clairement que le contenu charge et combien de temps cela peut prendre — "Clearly communicate that content is loading and how long it might take to complete." — loading › Showing progress
- **Choix de l'indicateur** : déterminé si la durée est connue, indéterminé sinon — "In general, you use a *determinate* progress indicator when you know how long loading will take, and you use an *indeterminate* progress indicator when you don’t." — loading › Showing progress
- **Téléchargements** : alléger l'installation et le lancement en téléchargeant les gros éléments en arrière-plan — "Improve installation and launch time by downloading large assets in the background." — loading › Best practices

**Spécifique macOS**
- Bloc Platform considerations : aucune considération supplémentaire pour macOS — "No additional considerations for iOS, iPadOS, macOS, tvOS, or visionOS." — loading › Platform considerations
- Aucun indicateur propre au Mac (curseur, progression dans le Dock, barre des menus) n'est décrit ; non traité.

**À transposer dans un shell Linux**
- Barre, dock, widgets et lanceur : appliquer la règle Affichage immédiat, avec la structure statique affichée dès le premier cadre et les contenus différés (heure, état réseau, météo, liste des apps) placés dans des emplacements réservés.
- Attente dépassant un instant (recherche, indexation, ouverture des Réglages) : indicateur déterminé si la fin est prévisible, indéterminé sinon.
- Ne jamais bloquer le shell pendant un chargement : changer d'espace ou lancer une autre app reste possible.
- Les seuils en millisecondes ne sont pas donnés par la HIG : les fixer dans la spécification du shell et les garder cohérents entre surfaces.

### Modality (`modality`)

**Rôle** : Présenter un contenu dans un mode dédié qui bloque la vue parente et se referme par une action explicite, seulement quand cela sert.

**Anatomie et cotes**
- Définition, donnée par le chapeau de la page juste au-dessus de la section Discussion : un mode séparé qui empêche l'interaction avec la vue parente et exige une action explicite pour être refermé — "Modality is a design technique that presents content in a separate, dedicated mode that prevents interaction with the parent view and requires an explicit action to dismiss." — modality › Discussion
- Composants cités : alerte (alert), feuille (sheet), pop-over (popover), vue d'activité (activity view), dialogue de confirmation ou feuille d'actions (action sheet), fenêtre séparée, vue modale plein écran.
- Dimensions, rayons, opacité du voile, durées d'animation : non chiffré dans la HIG.

**Règles**
- **Utilité** : ne présenter modalement qu'en cas de bénéfice clair (information critique, confirmation, tâche courte, immersion) — "Present content modally only when there’s a clear benefit." — modality › Best practices
- **Simplicité** : garder la tâche modale simple, courte et linéaire, sans quoi on perd le fil de la tâche suspendue — "Aim to keep modal tasks simple, short, and streamlined." — modality › Best practices
- **Pas d'app dans l'app** : éviter une hiérarchie de vues dans une tâche modale ; si des sous-vues s'imposent, un seul chemin — "provide a single path through the hierarchy" — modality › Best practices
- **Fermeture** : toujours offrir un moyen évident de refermer, selon les conventions connues de la plateforme — "Always give people an obvious way to dismiss a modal view." — modality › Best practices
- **Perte de données** : si la fermeture peut perdre un contenu créé par la personne, expliquer la situation et proposer des issues — "if closing the view could result in the loss of user-generated content, be sure to explain the situation and give people ways to resolve it" — modality › Best practices
- **Titre** : nommer la tâche de la vue modale, ou la décrire, pour aider à retrouver son contexte — "Make it easy to identify a modal view’s task." — modality › Best practices
- **Empilement** : laisser refermer une vue modale avant d'en présenter une autre ; jamais deux alertes à la fois — "you never want to display more than one alert at the same time" — modality › Best practices

**Spécifique macOS**
- Composants : une tâche distincte passe par une feuille ou un pop-over, ou simplement par une fenêtre séparée — "iOS, iPadOS, and macOS apps tend to use sheets or popovers" — modality › Discussion
- Fermeture : sur macOS, on s'attend à trouver le bouton de fermeture dans la vue de contenu principale — "in macOS and tvOS apps, people expect to find a button in the main content view" — modality › Best practices
- Bloc Platform considerations : aucune considération supplémentaire pour macOS — "No additional considerations for iOS, iPadOS, macOS, tvOS, visionOS, or watchOS." — modality › Platform considerations

**À transposer dans un shell Linux**
- Écran de verrouillage (lock screen) : vue modale plein écran à chemin unique vers la session (saisie, puis déverrouillage), consigne claire, aucune alerte superposée.
- Fenêtres de Réglages et confirmations du shell : une seule modale à la fois, bouton de fermeture visible dans la zone de contenu, confirmation seulement avant une perte réelle.
- Menus, listes et pop-overs de la barre (horloge, réseau, batterie) : tâches courtes et linéaires, sans sous-niveaux.
- Les cotes (rayon, voile, durée) ne figurent pas ici : les reprendre des chapitres sur les composants.

### Multitasking (`multitasking`)

**Rôle** : Permettre de passer vite d'une app à l'autre en conservant le contexte de chacune.

**Anatomie et cotes**
- Aucun composant coté ; sur Mac, les fenêtres ouvertes se distinguent par des ombres portées (drop shadows) et d'autres effets visuels selon leur état.
- Tailles, durées, dispositions : non chiffré dans la HIG.

**Règles**
- **Attente** : le multitâche est attendu ; sauf exceptions (certains jeux), toute app doit s'y prêter, sans quoi on croit qu'elle dysfonctionne — "People expect to use multitasking on their devices, and they may think something is wrong if your app doesn’t allow it." — multitasking › Discussion
- **Contexte** : toujours être prêt à enregistrer et restaurer le contexte, car on ne sait pas quand le multitâche commence — "Because you don’t know when people will initiate multitasking, your app or game always needs to be prepared to save and restore their context." — multitasking › Best practices
- **Pause** : suspendre les activités qui demandent attention ou participation active quand on s'éloigne, et reprendre comme si de rien n'était — "Pause activities that require people’s attention or active participation when they switch away." — multitasking › Best practices
- **Audio** : réagir sans heurt aux interruptions audio — "Respond smoothly to audio interruptions." — multitasking › Best practices
- **Tâches en cours** : terminer en arrière-plan les tâches lancées par la personne qui n'exigent plus d'entrée — "Finish user-initiated tasks in the background." — multitasking › Best practices
- **Notifications** : en émettre avec parcimonie ; seule la fin d'une tâche importante ou sensible au temps en justifie une — "Use notifications sparingly." — multitasking › Best practices

**Spécifique macOS**
- Sur Mac, le multitâche est l'expérience par défaut : plusieurs apps tournent à la fois et l'on passe d'une fenêtre ou d'une tâche à l'autre — "On Mac, multitasking is the default experience because people typically run more than one app at a time, switching between windows and tasks as they work." — multitasking › Platform considerations › macOS
- Fenêtres ouvertes : des ombres portées les font paraître empilées sur le bureau — "macOS applies drop shadows that make the windows appear layered on the desktop" — multitasking › Platform considerations › macOS
- D'autres effets visuels aident à distinguer les états des fenêtres (page Windows de la HIG) — "applies other visual effects to help people distinguish different window states" — multitasking › Platform considerations › macOS
- Absent de la page : espaces (Spaces), Stage Manager, Mission Control, comportement de la barre des menus et du Dock, réglages Bureau et Dock ; la page traite le multitâche du point de vue de l'app, non des surfaces qui l'orchestrent.

**À transposer dans un shell Linux**
- Aperçu des espaces et sélecteur de fenêtres de hyprdesk : chaque fenêtre y apparaît avec son contexte intact ; le shell la montre, l'app conserve son état.
- État des fenêtres lisible dans la vue normale comme dans l'aperçu : la fenêtre active se distingue des inactives.
- Changer d'espace ou masquer une fenêtre ne fait rien perdre à l'app : pas de fermeture implicite ni de rechargement.
- Tâches de fond du shell (indexation, application d'un thème) : appliquer les règles Tâches en cours et Notifications.
- L'orchestration des espaces (nombre, ordre, animation) n'est pas traitée par cette page.

### Going full screen (`going-full-screen`)

**Rôle** : Étendre une fenêtre à tout l'écran en masquant les contrôles système, pour un environnement sans distraction.

**Anatomie et cotes**
- Effet : la fenêtre remplit l'écran et les contrôles système se masquent (citation du chapeau de la page, placé juste au-dessus de la section Discussion) — "hiding system controls and providing a distraction-free environment" — going-full-screen › Discussion
- Actions de rappel citées pour restaurer des éléments masqués : toucher, balayer vers le bas, amener le curseur en haut de l'écran.
- Durées d'apparition, hauteur de la zone de déclenchement : non chiffré dans la HIG.

**Règles**
- **Opportunité** : offrir le plein écran quand l'expérience s'y prête (jeu, média, tâche de fond demandant de la concentration) — "Support full-screen mode when it makes sense for your experience." — going-full-screen › Best practices
- **Sortie volontaire** : laisser la personne choisir quand sortir du plein écran, qui ne s'achève pas de lui-même — "Let people choose when to exit full-screen mode." — going-full-screen › Best practices
- **Reprise** : après un changement d'expérience, aider à reprendre où l'on en était ; un jeu ou un diaporama se met en pause de lui-même — "After people switch away from your full-screen experience, help them resume where they left off when they return." — going-full-screen › Best practices
- **Contrôles essentiels** : les garder accessibles sans sortir du plein écran — "Continue to provide access to essential features and controls so people can complete their task without exiting full-screen mode." — going-full-screen › Best practices
- **Mise en page** : l'ajuster si nécessaire, sans redimensionner la fenêtre par programme ni créer de transition visuellement brutale — "If necessary, adjust your layout in full-screen mode, but don’t programmatically resize your window." — going-full-screen › Best practices
- **Masquage** : masquer temporairement barres d'outils et contrôles de navigation au profit du contenu, avec un moyen familier de les restaurer — "Prioritize content by temporarily hiding toolbars and navigation controls." — going-full-screen › Best practices

**Spécifique macOS**
- Dock : hors jeux, on doit pouvoir le faire apparaître depuis une app plein écran, pour ouvrir vite d'autres apps ou éléments du Dock — "Except in games, let people reveal the Dock while your iPadOS or macOS app is in full-screen mode." — going-full-screen › Best practices
- Dans un jeu plein écran, le Dock peut être masqué entièrement sur macOS — "hide the Dock entirely in macOS" — going-full-screen › Best practices
- Implémentation : utiliser le plein écran fourni par le système, qui tient compte, par exemple, du logement de caméra en haut et au centre de certains écrans de Mac — "Use the system-provided full-screen experience." — going-full-screen › Platform considerations › macOS
- Entrée : toujours au choix de la personne, par le bouton de plein écran de la fenêtre (Enter Full Screen button), un élément du menu Présentation (View menu) ou le raccourci Contrôle-Commande-F (Control-Command-F) — "Prefer letting people use your window’s Enter Full Screen button, View menu item, or the Control-Command-F keyboard shortcut." — going-full-screen › Platform considerations › macOS
- Pas de menu personnalisé de modes de fenêtre ; dans un jeu, ne pas changer le mode d'affichage au passage en plein écran — "Avoid offering a custom menu of window modes." — going-full-screen › Platform considerations › macOS
- Absent de la page : comportement précis de la barre des menus en plein écran (masquage, révélation, délai) et raccourci de sortie ; seuls les contrôles système en général et le Dock sont nommés.

**À transposer dans un shell Linux**
- Plein écran sous Hyprland : la barre et le dock se masquent avec les autres contrôles mais restent atteignables par un geste du pointeur (bord supérieur), hors jeux.
- Plein écran géré par le compositeur et non par chaque app : une seule implémentation pour toutes les fenêtres, qui réserve les découpes matérielles de l'écran s'il en a.
- Jamais de sortie automatique du plein écran lors d'un changement d'espace ou de focus ; la sortie reste une action explicite.
- Au retour sur une fenêtre plein écran, lui rendre son état ; le shell n'impose pas de pause mais n'en perd pas le contexte.
- Pas de menu de modes de fenêtre maison dans la barre : un seul plein écran, un seul raccourci.

### Launching (`launching`)

**Rôle** : Faire commencer l'utilisation immédiatement, de l'ouverture au premier écran prêt.

**Anatomie et cotes**
- Périmètre : le lancement commence à l'ouverture, inclut un éventuel téléchargement initial et finit quand le premier écran est prêt — "Launching begins when someone opens your app or game, includes an initial download, and ends when the first screen is ready." — launching › Discussion
- Attente admissible : pas plus de quelques secondes — "sometimes they don’t want to wait more than a couple of seconds" — launching › Best practices
- Durée d'un écran de lancement, taille des images, animations : non chiffré dans la HIG.

**Règles**
- **Rapidité** : lancer immédiatement — "Launch instantly." — launching › Best practices
- **Restauration** : restaurer l'état précédent au redémarrage, dans le détail : position de défilement, état et emplacement des fenêtres — "Restore the previous state when your app restarts so people can continue where they left off." — launching › Best practices
- **Écran de lancement (launch screen)** : requis seulement si la plateforme l'exige ; quasi identique au premier écran, sans texte ni publicité — "Design a launch screen that’s nearly identical to the first screen of your app or game." — launching › Launch screens
- **Écran de présentation (splash screen)** : si nécessaire, l'afficher au début du parcours d'accueil (onboarding) — "If you need a splash screen, consider displaying it at the beginning of your onboarding flow." — launching › Best practices
- **Suite** : après le lancement peut venir un parcours d'accueil qui donne une vue d'ensemble de l'app — "experience, which can give people a high-level view of your app or game." — launching › Discussion

**Spécifique macOS**
- macOS n'exige pas d'écran de lancement — "macOS, visionOS, and watchOS don’t require launch screens." — launching › Best practices
- Section Launch screens : non applicable à macOS — "Not applicable for macOS, visionOS, or watchOS." — launching › Launch screens
- Bloc Platform considerations : aucune considération supplémentaire pour macOS — "No additional considerations for macOS or watchOS." — launching › Platform considerations
- Absent de la page : lancement depuis le Dock (clic, voyant d'applications lancées, animation, progression), lanceur, barre des menus ; rien à en tirer.

**À transposer dans un shell Linux**
- Démarrage de session de hyprdesk : barre, dock et bureau s'affichent avec leur structure statique dès que possible ; aucun écran de présentation ni logo en chemin.
- Lancement d'une app depuis le dock ou le lanceur : retour immédiat à l'action (élément marqué, état pressé) plutôt qu'un écran d'attente.
- Réglages du shell : rouverture sur le dernier volet consulté ; les autres surfaces retrouvent leur état et leur emplacement après relance du démon.
- Les effets propres au Dock (voyant, animation, progression) ne sont pas traités par la HIG ici : les décider sans référence macOS dans ce chapitre.

### Undo and redo (`undo-and-redo`)

**Rôle** : Offrir un moyen facile d'annuler et de rétablir de nombreuses actions, ce qui invite à explorer sans risque.

**Anatomie et cotes**
- Emplacements attendus sur Mac : en haut du menu Édition (Edit menu) ; boutons seulement si nécessaire, dans une barre d'outils, avec les symboles du système.
- Profondeur d'historique : aucune valeur chiffrée ; la HIG déconseille les limites inutiles.
- Autres cotes : non chiffré dans la HIG.

**Règles**
- **Prévisibilité** : aider à prédire le résultat d'une annulation ; les libellés de menu peuvent nommer l'action visée (Undo Typing, Redo Bold) — "If you provide undo and redo menu items, you can modify the menu item labels to identify the result." — undo-and-redo › Best practices
- **Résultat visible** : mettre en évidence le résultat de chaque annulation ou rétablissement, surtout si la zone concernée n'est plus à l'écran — "Show the results of an undo or redo." — undo-and-redo › Best practices
- **Répétition** : on cherche souvent à annuler plusieurs fois de suite jusqu'à ce que quelque chose change — "they’re likely to try undoing — often multiple times — until something changes" — undo-and-redo › Discussion
- **Profondeur** : permettre d'annuler plusieurs fois, sans limite inutile, jusqu'à une étape logique (ouverture d'un document, enregistrement) — "Avoid placing unnecessary limits on the number of times people can undo or redo." — undo-and-redo › Best practices
- **Groupe** : envisager d'annuler en une fois une série d'actions liées (ajustements successifs d'une même propriété) — "Consider giving people the option to revert multiple changes at once." — undo-and-redo › Best practices
- **Boutons** : n'en proposer que si nécessaire ; on attend de passer par les voies du système — "Provide undo and redo buttons only when necessary." — undo-and-redo › Best practices

**Spécifique macOS**
- Les commandes Annuler et Rétablir se placent dans le menu Édition, avec les raccourcis standards — "Place undo and redo commands in the Edit menu and support the standard keyboard shortcuts." — undo-and-redo › Platform considerations › macOS
- Emplacement attendu : en haut du menu Édition — "Mac users expect to find undo and redo at the top of the Edit menu" — undo-and-redo › Platform considerations › macOS
- Raccourcis attendus : Commande-Z (Command–Z) pour annuler, Maj-Commande-Z (Shift–Command–Z) pour rétablir — "they also expect to use Command–Z and Shift–Command–Z to perform undo and redo, respectively." — undo-and-redo › Platform considerations › macOS
- Rien d'autre pour macOS ; la barre des menus n'apparaît qu'à travers le menu Édition.

**À transposer dans un shell Linux**
- Actions réversibles du shell (masquer un widget, réorganiser le dock ou la barre, supprimer un espace) : prévoir une annulation, dont le libellé nomme l'action quand un menu ou une notification l'expose.
- Après une annulation, ramener la vue sur l'élément restauré et le mettre en évidence ; une annulation sans effet visible pousse à répéter l'action.
- Pas de limite arbitrairement basse sur le nombre d'annulations dans un même contexte de travail.
- Pas de boutons dédiés tant qu'un raccourci ou un menu existe ; si boutons, icônes standard du thème.
- La page ne donne ni profondeur d'historique ni raccourci propre au shell : à spécifier dans le chapitre clavier.

### Valeurs de référence

| Propriété | Valeur | Source (slug › section) |
|---|---|---|
| Raccourci d'ouverture des réglages d'une app | Command-Comma (,) | settings › Best practices |
| Titre d'une fenêtre de réglages sans volets multiples | App Name Settings | settings › Platform considerations › macOS |
| Boutons réduire et agrandir d'une fenêtre de réglages | désactivés (dimmed) | settings › Platform considerations › macOS |
| Niveaux d'interruption système | Passive, Active (défaut), Time Sensitive, Critical | managing-notifications › Integrating with Focus |
| Outrepassement (livraison programmée, Concentré, interrupteur Sonnerie/Silence) | Passive et Active : non, non, non ; Time Sensitive : oui, oui, non ; Critical : oui, oui, oui | managing-notifications › Integrating with Focus |
| Envoi d'une notification Critical | entitlement obligatoire | managing-notifications › Integrating with Focus |
| Fenêtre temporelle d'une notification Time Sensitive | événement en cours ou dans l'heure | managing-notifications › Best practices |
| Seuil d'emploi d'un indicateur de progression | chargement de plus d'un moment ou deux | loading › Showing progress |
| Attente admissible au lancement | pas plus de quelques secondes (a couple of seconds) | launching › Best practices |
| Écran de lancement sur macOS | non requis, non applicable | launching › Best practices ; launching › Launch screens |
| Entrée en plein écran | bouton de la fenêtre, élément du menu View, Control-Command-F | going-full-screen › Platform considerations › macOS |
| Annuler et rétablir | Command–Z et Shift–Command–Z, en haut du menu Edit | undo-and-redo › Platform considerations › macOS |

## Seconde partie — hyprdesk aujourd'hui, écarts et actions

### Ce que hyprdesk fait aujourd'hui
| Surface | Ce qui existe | Valeurs clés | Preuves |
| --- | --- | --- | --- |
| Réglages | Tiroir latéral sur l'écran actif, navigation par groupes et paquets, entrée active marquée (icône accent, fond teinté), croix « Fermer · Échap », recherche plein texte des libellés et de l'aide, partage base / configuration avancée | 37 paquets ; avancé coupé par défaut ; 18 onglets réservés à l'avancé ; raccourci SUPER + SHIFT + D, unique | `src/ui/panel.rs:137, 199, 719-723, 800-811, 913, 964` ; `src/ui/plugins/mod.rs:72-110, 184-204` ; `src/config/model.rs:1519, 1735` |
| Historique de configuration | Instantanés datés avant chaque écriture, retour arrière, motif nommé | 30 jours ; un instantané par série de 60 s | `src/history.rs:1-20, 55-57` |
| Lanceur et recherche | Une carte, quatre modes (applications, `:f` fichiers, `:clip`, `:2fa` et `:mdp`), recherche floue à chaque frappe, suggestions (les plus lancées) avant la frappe, démon d'index séparé | 8 résultats visibles ; index : nom, nature, chemin, taille, date | `src/ui/search.rs:103-114, 242, 264-265, 1127, 1192-1195, 1212` ; `src/search.rs:25-70` ; `src/fileindex/index.rs:85-95` ; `src/ui/plugins/launcher/grid.rs:343` |
| Notifications | Serveur freedesktop 1.2, trois urgences, bulles sur le calque `overlay`, historique, Silence global persistant (seule la critique passe) | 6 s par défaut (réglable) ; 4 bulles visibles ; 50 en historique ; critique sans délai | `src/notifications.rs:27-35, 120-134, 228-256` ; `src/ui/toasts.rs:28-30, 128-137, 179-198, 252-262, 473` |
| Espaces et fenêtres | Aperçu « Espaces de travail » (carte par espace, fenêtres en vignettes saisissables, renommage, « + Nouvel espace »), commutateur Alt+Tab, menu de fenêtre | 30 espaces au plus | `src/ui/overview.rs:447-451, 690, 730-731, 966` ; `src/ui/plugins/bar/window.rs:53-135` ; `src/config/model.rs:1845` |
| Plein écran | Géré par Hyprland (SUPER + F) ; l'ouverture d'un tiroir (Réglages, LYRA) retient la fenêtre en « agrandi » sans lui faire quitter le plein écran | barre `always` + calque `top` par défaut ; dock sur calque `top` | `src/config/model.rs:1050-1067, 1108, 1113` ; `src/ui/plugins/bar/surface.rs:645-648` ; `src/ui/plugins/dock/surface.rs:356` ; `src/hypr.rs:1456-1493` |
| Lancement | Dock (pulse de l'icône jusqu'à la fenêtre ou 5 s), grille d'applications, menus | survol 180 ms ; délai de lancement 5 s | `src/ui/plugins/dock/lifecycle.rs:13-14, 244-273` ; `src/ui/plugins/dock/surface.rs:579-590` |
| Retour et attente | OSD volume / luminosité (icône, jauge, pourcentage), textes d'attente dans le lanceur, compteur d'indexation | OSD 1,5 s après la dernière touche | `src/ui/osd.rs:1-3, 21-22` ; `src/ui/search.rs:1192-1195, 1384, 1490` ; `src/ui/plugins/index.rs:36-37, 285-289` |
| Alertes | Fenêtre « Problème » unique, remplacée quand une autre arrive | calque `overlay`, clavier à la demande | `src/ui/problem.rs:57-61` |

### Écarts
| # | Composant | Nature | Force | Règle HIG (slug › section) | Réel dans hyprdesk (preuve) | Action proposée | Effort | Confiance |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| PAT-01 | Dock en plein écran | ÉCART | impératif (« Except in games, let people reveal the Dock while your iPadOS or macOS app is in full-screen mode ») | going-full-screen › Best practices (bloc macOS) | Le dock est créé sur le calque `top`, sans option de calque ni branche plein écran (aucune occurrence de `fullscreen` dans `src/ui/plugins/dock/`) : `src/ui/plugins/dock/surface.rs:356`. Ce calque est « sous une fenêtre en plein écran » (`src/config/model.rs:1060-1062`). Le masquage hors survol (`dock.autohide`) rétrécit la même surface `top` (`src/ui/plugins/bar/surface.rs:985-987, 1039-1050`) : sous une fenêtre plein écran, aucun geste du pointeur ne rappelle le dock. | Quand l'espace affiché compte une fenêtre plein écran (état déjà suivi en `src/ui/mod.rs:3163-3165`), passer la surface du dock sur le calque `overlay`, réduite à sa bande de rappel et révélée au survol du bord. | M | haute |
| PAT-02 | Autorisation des applications | ABSENT | impératif (« You need to get permission before sending any notification ») | managing-notifications › Discussion | `Server::notify` enregistre et émet toute notification reçue, sans regard sur l'émetteur (`src/notifications.rs:228-256`) ; l'interface l'affiche sauf Silence (`src/ui/toasts.rs:260-262`). Recherche d'une liste d'applications autorisées ou bloquée dans `src/notifications.rs` et `src/ui/toasts.rs` : rien (seuls des `#[allow(...)]`). Le seul filtre est le Silence global. | Demander l'autorisation à la première notification de chaque application (autoriser, refuser), la garder et la révoquer dans Réglages. | L | haute |
| PAT-03 | Barre en plein écran | PISTE | impératif, atténué par une option qui le satisfait (« Continue to provide access to essential features and controls so people can complete their task without exiting full-screen mode ») | going-full-screen › Best practices | Par défaut `BarVisibility::Always` et `BarLayer::Top` (`src/config/model.rs:1108, 1113` ; valeurs de session identiques) : la barre passe sous la fenêtre plein écran et n'a pas de zone de rappel, qui n'existe qu'en `AutoHide` (`src/ui/plugins/bar/surface.rs:1006-1008, 1059-1064`). Falsification : `overlay` + `autohide` (`src/config/model.rs:1054, 1064`) satisferait la règle, mais ce n'est le comportement d'aucun réglage par défaut. | Réserver le même traitement que le dock (PAT-01) à la barre pendant le plein écran. | M | haute |
| PAT-04 | Durée du Silence | ABSENT | descriptif, à peser (« A Focus helps people filter notifications during a time period they reserve for an activity ») | managing-notifications › Integrating with Focus | Le Silence est un booléen gardé d'une session à l'autre (`src/notifications.rs:87-90`, `src/ui/toasts.rs:179-198`), basculé par un bouton (`src/ui/plugins/bar/popover/continuity.rs:568-580`) ou `hyprdesk notifications dnd` ; ni durée ni fin, ni liste d'applications autorisées. | Faire choisir une fin à l'activation (une heure, jusqu'à ce soir, jusqu'à désactivation). | S | haute |
| PAT-05 | Niveaux d'interruption | ABSENT | tableau des quatre niveaux ; « The first time a Time Sensitive notification arrives from your app, the system describes how such a notification works and gives people a way to turn it off » | managing-notifications › Integrating with Focus ; Best practices | Trois niveaux seulement, lus dans l'indice de l'application (`src/notifications.rs:120-134`) : « sensible au temps » n'existe pas. Une notification critique se déclare elle-même, traverse le Silence (`src/ui/toasts.rs:260-262`), ne s'efface jamais (`src/ui/toasts.rs:128-131`) et n'est expliquée à personne. | Accorder le passage du Silence application par application, à la première notification critique (explication et refus possibles). | M | haute |
| PAT-06 | Urgence basse | ÉCART | tableau (Passive : à consulter à loisir) | managing-notifications › Integrating with Focus | `Urgency::Low` est produite (`src/notifications.rs:129`) et jamais lue dans l'interface (aucune occurrence dans `src/ui/`) : même bulle et même délai que la normale (`src/ui/toasts.rs:128-137`) ; seule la critique a un traitement (`:129, 260, 498`). | Livrer une notification basse « discrètement » : dans l'historique, sans bulle (le chemin déjà pris sous Silence). | S | haute |
| PAT-07 | Réglages des notifications | ABSENT | impératif (« Make sure people can manage their notification settings within your app ») | managing-notifications › Sending marketing notifications | Les seules options sont largeur, nombre visible, coin, sens d'empilement et durée (`src/ui/toasts.rs:224-228`) ; le registre `BUILTIN` n'a pas de paquet notifications (`src/ui/plugins/mod.rs:72-110`) ; l'effacement de l'historique n'existe que dans le centre de contrôle Continuité (`src/ui/plugins/bar/popover/continuity.rs:1085-1087`). | Ajouter un paquet « Notifications » aux Réglages : par application (autorisation, niveau, révocation), effacement de l'historique. | L | haute |
| PAT-08 | Échec d'un lancement | ÉCART | impératif (« Show people when a command can't be carried out and help them understand why ») | feedback › Best practices | Depuis le dock ou la grille : application irrésolue, le pulse s'arrête et rien d'autre (`src/ui/mod.rs:4163-4166`) ; lancement refusé, l'erreur va dans le journal puis le pulse s'arrête (`src/ui/mod.rs:3588-3596, 4171-4176`). | Afficher une bulle nommant l'application et la cause (le chemin de `src/ui/overview.rs:441-443`). | S | haute |
| PAT-09 | Dernier volet des Réglages | ÉCART | impératif (« Restore the most recently viewed pane ») | settings › Platform considerations › macOS | Chaque ouverture reconstruit `Panel::new(win)` (`src/ui/panel.rs:269`), dont l'onglet vaut toujours le paquet « Bureau » (`src/ui/panel.rs:137`). Seul le sous-onglet est gardé sur `App` (`src/ui/panel.rs:449-451`) ; recherche de `last_tab`, `last_page`, `panel_tab` dans `src` : rien. | Garder le dernier paquet consulté sur `App`, comme les sous-onglets, et rouvrir dessus. | S | haute |
| PAT-10 | « + Nouvel espace » | ÉCART | impératif (« Show people when a command can't be carried out and help them understand why ») | feedback › Best practices | Le bouton est toujours actif (`src/ui/overview.rs:747-748`) ; au-delà de `MAX_WORKSPACES` (30), l'action rend `Task::none()` sans rien dire (`src/ui/overview.rs:447-451`). | Désactiver le bouton à la limite et porter le maximum (30) en infobulle. | S | haute |
| PAT-11 | Échecs sans message | ÉCART | impératif (même règle) | feedback › Best practices | Copie d'un code ou d'un identifiant myMFA : journal seulement (`src/ui/search.rs:739-744`). Ouverture d'un fichier depuis la recherche : journal seulement (`src/ui/search.rs:907-912`). Action ou fermeture d'une notification refusée : journal seulement (`src/ui/toasts.rs:103-105`). | Afficher l'erreur et sa cause dans une bulle, comme le renommage d'espace (`src/ui/overview.rs:441-443`). | S | haute |
| PAT-12 | Annulation des actions du shell | ABSENT | rôle de la page (« Offer an easy way to undo and redo many actions ») | undo-and-redo › Discussion ; Best practices | Le seul retour arrière est celui de la configuration (`src/history.rs:1-20`). « Retirer du bureau » (`src/ui/context.rs:527, 684`), « Retirer du dock » (`src/ui/context.rs:716`) et le dépôt d'une fenêtre sur un autre espace (`src/ui/overview.rs:459-460`) n'offrent aucune annulation : aucun `Named::Z` dans `src/ui`, `src/config`, `src/hypr.rs` ; « Annuler » n'existe que comme bouton d'annulation de formulaire. | Offrir « Annuler » dans une bulle après ces actions, avec le nom de l'action. | M | moyenne |
| PAT-13 | Point d'entrée de la recherche | PISTE | recommandation (« Aim to make your app's content searchable through a single location ») | searching › Best practices | Le lanceur connaît quatre modes (`src/ui/search.rs:103-114`) et aucun ne touche aux réglages ; la recherche des réglages n'est appelée que par le tiroir (`src/ui/panel.rs:719`). Les commandes du shell n'ont pas de mode. | Ajouter les résultats des réglages au lanceur, en réutilisant `settings_search`. | M | haute |
| PAT-14 | Raccourci des Réglages | PISTE | voie attendue (« people often use the standard Command-Comma (,) keyboard shortcut to open an app's settings ») | settings › Best practices | Les Réglages s'ouvrent sur SUPER + SHIFT + D (`src/config/model.rs:1519` ; unique en session, sans conflit) ; SUPER + virgule est pris par hyprdesk pour l'écran suivant (`src/tiling.rs:1190`). | Donner SUPER + virgule aux Réglages et déplacer l'écran suivant. | S | haute |
| PAT-15 | Périmètre de la recherche | PISTE | impératif, atténué (le préfixe tapé reste lisible) (« Clearly display the current scope of a search ») | searching › Best practices | En mode `:clip`, `:2fa` et `:mdp`, le périmètre n'apparaît que comme préfixe tapé ; le texte indicatif dit « Rechercher une application… » (`src/ui/search.rs:1162, 1371`) et n'est jamais montré, puisque le champ contient le préfixe ; les listes de résultats n'ont pas de titre (`src/ui/search.rs:1484-1503`). En mode fichiers, les puces de nature portent le périmètre (`src/ui/search.rs:1212`). | Donner un titre à la liste de chaque mode (« Presse-papiers », « Codes », « Identifiants »). | S | moyenne |
| PAT-16 | Titre du tiroir | PISTE | descriptif (« the title of the window reflects the visible pane ») | settings › Platform considerations › macOS | L'en-tête porte le titre constant « Réglages » (`src/ui/panel.rs:958-972`) ; le volet n'est indiqué que dans la navigation (`src/ui/panel.rs:800-811`). | Afficher le libellé du paquet actif comme titre, « Réglages » restant en surligne. | S | haute |
| PAT-17 | Fenêtre active dans l'aperçu | PISTE | descriptif (« applies other visual effects to help people distinguish different window states ») | multitasking › Platform considerations › macOS | Dans une carte, toutes les vignettes de fenêtre reçoivent le même style (`src/ui/overview.rs:966-1016`) ; `focused` n'y est lu que pour l'écran (`:128`). Seule la carte de l'espace actif est marquée (`:877, 918, 926-935`). | Marquer d'un contour accent la vignette de la fenêtre active. | S | haute |
| PAT-18 | Attente d'indexation | PISTE | impératif (« Clearly communicate that content is loading and how long it might take to complete ») ; recommandation (« Gauge the remaining loading time as accurately as possible ») | loading › Showing progress ; Best practices | Pendant le premier parcours, le lanceur n'affiche qu'une ligne de 13 px « Indexation en cours… (N) » (`src/ui/search.rs:1195, 1203-1208`) ; Réglages › Index, le même compte et une pastille (`src/ui/plugins/index.rs:285-289`). Le démon ne rend public que le nombre d'entrées vues (`src/ui/plugins/index.rs:36-37`). | Faire rendre au démon une estimation de la fin et afficher une barre de progression aux deux endroits. | M | haute |
| PAT-19 | Historique d'utilisation | PISTE | impératif sur l'historique de recherche (« If you do show search history, provide a way for people to clear it ») | searching › Best practices | Les suggestions d'avant frappe sont les applications les plus lancées, classées avec un bonus d'usage (`src/ui/search.rs:242, 264-265` ; `src/search.rs:97, 139`). Aucune commande d'effacement trouvée (rien dans `src/ui/plugins/launcher.rs`, aucune fonction `usage` d'effacement dans `src`). | Ajouter « Effacer l'historique d'utilisation » dans Réglages › Lanceur. | S | moyenne |
| PAT-20 | Résumé programmé | PISTE | descriptif (« Delivery scheduling lets people choose whether to receive notification alerts immediately or in a summary that's delivered at times they choose ») | managing-notifications › Integrating with Focus | Sous Silence, les notifications non critiques vont à l'historique sans être signalées (`src/ui/toasts.rs:259-261`) ; il n'existe ni résumé ni remise différée. | Proposer, à la fin du Silence, les notifications retenues. | M | moyenne |
| PAT-21 | Volume des Réglages | IDENTITÉ | impératif, non coté (« Minimize the number of settings you offer ») | settings › Best practices | 37 paquets (`src/ui/plugins/mod.rs:72-110`), environ 290 contrôles dans `src/ui/plugins` (160 bascules, 54 curseurs, 44 champs, 35 listes). Garde-fou déjà en place : partage base / avancé, avancé coupé par défaut (`src/config/model.rs:1735`). Suit l'identité « configuration de bout en bout » (orientation § 4). | — (à signaler, sans correction). | — | haute |

### Ce qui est déjà conforme
- settings › Platform considerations › macOS — barre d'outils fixe, toujours visible, bouton actif indiqué | `src/ui/panel.rs:800-811` (entrée active : icône accent, fond accent 16 %), `:815-891` (navigation toujours construite).
- settings › Best practices — valeurs par défaut, options d'expert à part | `src/config/model.rs:1735`, `src/ui/plugins/mod.rs:184-204`.
- settings › Best practices — un raccourci unique et documenté | `src/config/model.rs:1519` ; `hyprdesk keys` : « SUPER + SHIFT + D — Open settings », « No conflict ».
- modality › Best practices — moyen évident de refermer | `src/ui/panel.rs:913` (« Fermer · Échap »), `src/ui/overview.rs:730` (aide « Échap Fermer ») et `:560`.
- modality › Best practices — une seule modale à la fois | `src/ui/panel.rs:199-203` (ouvrir les Réglages ferme l'assistant), `src/ui/problem.rs:57-61` (alerte remplacée, jamais empilée).
- modality › Best practices — titre de la tâche | `src/ui/overview.rs:690` (« Espaces de travail »).
- modality › Best practices — perte de données expliquée avant l'irréversible | `src/ui/panel.rs:497-512` (« Par défaut… » demande, puis confirme).
- feedback › Best practices — échec expliqué, avec sa cause | `src/ui/overview.rs:441-443`, `src/ui/context.rs:1586-1588, 1606`.
- feedback › Discussion — retour proportionné | `src/ui/osd.rs:1-3, 21-22` (changement d'état passif, sans alerte).
- feedback › Best practices — plusieurs canaux | `src/ui/osd.rs:1-3` (icône, jauge, pourcentage).
- loading › Best practices — non bloquant | `src/fileindex/daemon.rs` (démon séparé), `src/ui/search.rs:806-830` (interrogation asynchrone).
- loading › Best practices — quelque chose d'affiché aussitôt | `src/ui/search.rs:1192-1195, 1384, 1490`.
- searching › Best practices — suggestions pendant et avant la frappe | `src/search.rs:25-70`, `src/ui/search.rs:264-265`, `src/ui/plugins/launcher/grid.rs:343`.
- searching › Systemwide search — indexation avec métadonnées | `src/fileindex/index.rs:85-95`, `src/ui/settings_search.rs:1-6`.
- managing-notifications › Integrating with Focus — retard d'alerte | `src/ui/toasts.rs:252-258` (historique alimenté avant le filtre du Silence).
- managing-notifications › Integrating with Focus — colonne « traverse un Concentré » | `src/ui/toasts.rs:260-262`.
- launching › Best practices — lancement instantané, retour immédiat | `src/ui/plugins/dock/surface.rs:579-590`, `src/ui/plugins/dock/lifecycle.rs:13-14, 244-262`.
- launching › Launch screens — pas d'écran de présentation | `src/greeter_setup.rs:418` (`disable_splash_rendering = true`).
- undo-and-redo › Best practices — profondeur, groupe, prévisibilité (réglages) | `src/history.rs:1-20, 55-57` (30 jours, une série par motif de 60 s, motif nommé).
- multitasking › Best practices — masquer sans rien perdre | `src/ui/plugins/bar/window.rs:56-63, 130-134` (« Réduire » déplace la fenêtre vers l'espace dédié, sans la fermer).
- multitasking › Best practices — tâches de fond, notifications parcimonieuses | `src/fileindex/` (indexation dans un démon, aucune notification émise).
- multitasking › Platform considerations › macOS — état lisible | `src/ui/overview.rs:877, 918, 926-935` (carte de l'espace actif).
- going-full-screen › Best practices — sortie au choix de la personne | `src/hypr.rs:1476-1486` (plein écran rendu à la fermeture du tiroir ; Super+F vaut sortie).
- going-full-screen › Platform considerations › macOS — un plein écran, un raccourci | `src/ui/plugins/bar/window.rs:55-72` (« Agrandir », « Plein écran »).

SANS OBJET : boutons réduire et agrandir désactivés, bouton Réglages dans la barre d'outils d'une fenêtre (le tiroir n'a pas de chrome) ; générateur d'aperçu (hyprdesk ne produit pas de type de fichier) ; styles bannière et alerte, durées, regroupement, position (absents de la page, spec l. 116) ; menu Édition et raccourcis Command-Z (pas de menu Édition : à spécifier au chapitre clavier).

### À vérifier sur un rendu
- Silence activé : une marque durable reste-t-elle dans la barre ? L'état n'est dessiné que sur le bouton du centre de contrôle (`src/ui/plugins/bar/popover/continuity.rs:568-580`) ; à relever selon le module de barre en service.
- Bulles (calque `overlay`, `src/ui/toasts.rs:473`) et fenêtre Problème au-dessus de l'écran de verrouillage.
- Fenêtre Problème et demande polkit ouvertes ensemble : empilement visuel de deux fenêtres `overlay`.
- Premier cadre de session : barre, dock et widgets dessinés avec leur structure avant les contenus différés (heure, réseau, liste des applications).
- Échap pendant le glisser d'une fenêtre dans l'aperçu (`src/ui/overview.rs:560` ferme l'écran) : le déplacement est-il abandonné ?
- Retour au pressage des boutons stylés à la main (exemple : « + Nouvel espace », `src/ui/overview.rs:748-752`, seul le survol est stylé).
- Commutateur Alt+Tab : la fenêtre active se distingue-t-elle de la simple sélection ?
- Barre `overlay` + `autohide` devant une fenêtre plein écran : rappel effectif au bord (la combinaison qui satisferait PAT-03).
