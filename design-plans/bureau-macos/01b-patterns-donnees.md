# Patterns : données, documents et accompagnement — macOS d'après la HIG, face à hyprdesk

> Proposition de design, rien d'implémenté. Plan d'ensemble et priorités : `plan-d-action.md` ; méthode et périmètre : `README.md`.

> Référentiel : Human Interface Guidelines d'Apple, export Markdown officiel relevé le 23 novembre 2026. Code : commit `00f6d69` (arbre de travail du 23 novembre 2026, avec des modifications non commitées).

## Première partie — Spécification d'après la HIG

### Résumé

- Corpus : huit pages de patterns généraux sur les données, les documents et l'accompagnement ; le bureau n'y apparaît que par les blocs macOS (Finder, fenêtres inactives, pointeur, infobulles) et par l'exemple du Finder.
- Glisser-déposer (Drag and drop) : source et destination ; déplacement dans un même conteneur, copie entre conteneurs ; image de glissement translucide, cibles signalées, annulation préférée à la confirmation.
- Fichiers (File management) : commandes de menu et raccourcis pour créer et ouvrir, enregistrement automatique, extensions masquées, aperçus Quick Look ; sur macOS, navigateur par défaut du Finder et marqueurs de modifications non enregistrées.
- Aide et accueil (Offering help, Onboarding) : une aide contextuelle facile à écarter, des astuces brèves et peu fréquentes, des infobulles très courtes ; un premier lancement facultatif et interactif, qui reporte la configuration non essentielle.
- Saisie (Entering data) : obtenir l'information du système plutôt que la demander, proposer des choix, valider dès la saisie, ne jamais préremplir un mot de passe.
- Graphiques et audio (Charting data, Playing audio) : un graphique simple, cohérent, décrit et accessible ; le volume du système prime, les commandes audio ne se détournent pas, la reprise dépend du type d'interruption.
- Comptes (Managing accounts) : un compte seulement s'il est indispensable, une connexion la plus tardive possible, une suppression offerte et annoncée.
- Limites du corpus : glisser vers le Dock ou la barre des menus, délais des infobulles, dimensions des graphiques et modèle audio propre à macOS ne figurent pas dans ces pages ; chaque section le signale, et rien n'est complété par une autre source.

### Drag and drop (`drag-and-drop`)

**Rôle** : Déplacer ou dupliquer une sélection en la traînant d'une source vers une destination, avec un retour continu et une annulation possible.

**Anatomie et cotes**
- Image de glissement (drag image) : représentation translucide du contenu traîné, qui laisse voir les destinations passées en survol et reste affichée jusqu'au dépôt — "It works well to create a translucent representation of the content people are dragging." — drag-and-drop › Providing feedback
- Seuil d'apparition de l'image : environ trois points de déplacement de la sélection — "Display a drag image as soon as people drag a selection about three points." — drag-and-drop › Providing feedback
- Badge (badge) : petit ovale plein contenant un nombre, pour un glissement de plusieurs éléments — "A badge is a small filled oval containing a number you can use to indicate the number of items people are dragging." — drag-and-drop › Platform considerations › macOS
- Pointeurs (pointers) cités : copy, drag link, disappearing item, operation not allowed ; touche Option pour forcer la copie.
- Opacité de l'image, taille du badge, délais : non chiffrés dans la HIG.

**Règles**
- **Généralisation** : soutenir le glisser-déposer partout où c'est possible, car on l'essaie partout — "As much as possible, support drag and drop throughout your app." — drag-and-drop › Best practices
- **Alternatives** : offrir d'autres moyens d'arriver au même résultat, par exemple des commandes de menu de copie et de déplacement — "Offer alternative ways to accomplish drag-and-drop actions." — drag-and-drop › Best practices
- **Déplacer ou copier** : même conteneur, déplacement ; conteneur différent, copie ; entre apps, toujours une copie — "As a general rule, dropping selected content within the same container moves it, whereas dropping content in a different container copies it." — drag-and-drop › Discussion
- **Annulation** : préférer l'annulation d'un glisser-déposer, un dépôt sur la mauvaise destination étant facile — "Prefer letting people undo a drag-and-drop operation." — drag-and-drop › Best practices
- **Confirmation** : quand l'opération est irréversible, demander confirmation, comme le Finder pour un dossier en écriture seule — "the Finder asks for confirmation when people drag a file into a write-only folder because they won’t be able to open the folder and remove the dropped item" — drag-and-drop › Best practices
- **Chargement par ressort (spring loading)** : activer certains contrôles (boutons, commandes segmentées) en traînant du contenu par-dessus — "Spring loading lets people activate certain controls, like buttons and segmented controls, by dragging selected content over them." — drag-and-drop › Best practices
- **Retour continu** : fournir un retour clair et continu pendant tout le processus — "it’s crucial to provide clear and continuous feedback throughout" — drag-and-drop › Providing feedback
- **Image prédictive** : modifier l'image de glissement si cela éclaire le résultat, sans la faire changer sans cesse — "avoid creating a distracting experience in which the drag image is constantly and radically changing" — drag-and-drop › Providing feedback
- **Cibles** : signaler les destinations capables d'accueillir le contenu, seulement pendant le survol, et une à la fois — "Display highlighting or other visual cues only while the content is positioned above the destination, removing the visual feedback when people drag the content away." — drag-and-drop › Providing feedback
- **Dépôt invalide** : donner un retour visuel en cas de dépôt sur une cible invalide ou d'échec : retour à la source, ou agrandissement et fondu — "When people drop an item on an invalid destination, or when dropping fails, provide visual feedback." — drag-and-drop › Providing feedback
- **Défilement** : faire défiler automatiquement le contenu d'une destination quand on y traîne un élément — "Scroll the contents of a destination when necessary." — drag-and-drop › Accepting drops
- **Contenu reçu** : choisir la version la plus riche acceptée par l'app et n'extraire que la portion pertinente — "When there’s a choice, pick the richest version of dropped content your app can accept." — drag-and-drop › Accepting drops
- **Option** : avec un clavier physique, vérifier la touche Option au moment du dépôt, qui force la copie dans un même conteneur — "When people hold the Option key while dragging, they can force a drag-and-drop operation within the same container to behave like a copy." — drag-and-drop › Accepting drops
- **Transfert long** : afficher la progression, et un emplacement réservé à l'endroit du dépôt — "Provide feedback when dropped content needs time to transfer." — drag-and-drop › Accepting drops
- **Tâche déclenchée** : si le dépôt lance une tâche (impression, par exemple), montrer qu'elle a commencé et suivre sa progression — "show people that the task has begun and keep them informed of its progress" — drag-and-drop › Accepting drops
- **Sélection** : après un dépôt, le contenu reste sélectionné dans la destination, et l'état de la source s'en déduit — "After a drop, maintain the content’s selection state in the destination, updating it in the source as needed." — drag-and-drop › Accepting drops

**Spécifique macOS**
- Sur Mac, le glisser-déposer se fait au dispositif de pointage, au Contrôle au clavier complet (Full Keyboard Access) ou à VoiceOver, ou directement à l'écran tactile — "On a Mac, people can use a pointing device, use Full Keyboard Access or VoiceOver, or interact directly with a touch display to perform drag and drop." — drag-and-drop › Discussion
- Contrôle universel (Universal Control) : du contenu se traîne entre un Mac et un iPad ; macOS permet aussi de traîner en groupe des éléments sélectionnés dans plusieurs apps — "macOS also lets people select multiple items from several apps and drag them as a group" — drag-and-drop › Best practices
- Vers le Finder : laisser traîner du contenu de l'app vers le Finder, dans un format que l'app saura rouvrir — "Consider letting people drag content from your app into the Finder." — drag-and-drop › Platform considerations › macOS
- Clipping : le contenu traîné peut sortir en clipping, conteneur temporaire ensuite traînable dans un champ de texte, sans rapport avec le presse-papiers (Clipboard) — "a drag-and-drop clipping isn’t related to the Clipboard" — drag-and-drop › Platform considerations › macOS
- Fenêtre inactive : une sélection s'en traîne sans activer la fenêtre ; elle prend l'apparence d'une sélection en arrière-plan (background selection), distincte de celle de la fenêtre active — "Selected content in an inactive window is known as a *background selection* and has a different appearance from selected content in the active window." — drag-and-drop › Platform considerations › macOS
- Élément isolé : un élément non sélectionné s'en traîne sans défaire la sélection en arrière-plan existante — "When possible, let people drag individual items from an inactive window without affecting an existing background selection." — drag-and-drop › Platform considerations › macOS
- Badge : l'afficher pour un glissement multiple et le mettre à jour si la cible n'accepte qu'une partie des éléments — "If a destination can accept only a subset of dragged items, update the badge to show the new number." — drag-and-drop › Platform considerations › macOS
- Pointeur : changer son apparence selon l'effet du dépôt — "In addition to using the *copy* pointer, you might want to use the *drag link*, *disappearing item*, and *operation not allowed* pointers, depending on the situation." — drag-and-drop › Platform considerations › macOS
- Geste unique : sélectionner et traîner en un seul mouvement, sans pause — "As much as possible, let people select and drag content with a single motion." — drag-and-drop › Platform considerations › macOS
- Chargement par ressort au Magic Trackpad : un bouton ou une commande segmentée s'active par clic forcé pendant qu'on tient le contenu — "On a Mac equipped with a Magic Trackpad, a button or segmented control can activate when people force-click it while continuing to hold the content" — drag-and-drop › Best practices
- Absent de la page : glisser vers le Dock ou la barre des menus (menu bar), déposer sur une icône du Dock, glisser depuis le bureau ; rien à en tirer.

**À transposer dans un shell Linux**
- Barre et dock comme cibles : la page ne les traite pas ; leur appliquer les règles Cibles, Dépôt invalide et Annulation ci-dessus.
- Lanceur, widgets et sélecteurs de fichiers du shell : image de glissement translucide dès le seuil coté plus haut, badge compteur pour une sélection multiple, curseur adapté à l'opération.
- Touche modificatrice de copie vérifiée au moment du dépôt, avec un curseur changé en conséquence.
- Traîner un élément non sélectionné sans perdre la sélection courante ; la sélection reste visible dans une fenêtre inactive.
- Toujours une alternative au glisser-déposer (commande de menu, copier-coller) ; progression visible pour un transfert long.

### File management (`file-management`)

**Rôle** : Permettre de créer, ouvrir, enregistrer et parcourir documents et fichiers d'une manière cohérente avec le reste du système.

**Anatomie et cotes**
- Composants cités : navigateur de fichiers (file browser), panneau d'ouverture (open panel), vue d'enregistrement (save view, Save dialog), vue accessoire (accessory view), aperçu Quick Look (Quick Look).
- Signes d'état sur macOS : point (dot) sur le bouton de fermeture de la fenêtre du document et à côté de son nom dans le menu Fenêtre ; suffixe Edited dans la barre de titre.
- Dimensions des panneaux, colonnes, durées : non chiffré dans la HIG.

**Règles**
- **Créer et ouvrir** : offrir des commandes de menu et des raccourcis clavier pour créer et ouvrir des documents — "Use app menus and keyboard shortcuts to give people convenient ways to create and open documents." — file-management › Creating and opening files
- **Bouton d'ajout** : prévoir aussi un bouton Ajouter (Add (+) button), même quand des raccourcis existent — "Regardless of the availability of keyboard shortcuts, include an Add (+) button to help people create a new document." — file-management › Creating and opening files
- **Navigateur personnalisé** : s'il s'impose, rester fidèle à l'idée que les gens se font du système de fichiers et laisser parcourir le reste — "If your app requires a custom file browser, support people’s understanding of the platform’s file system." — file-management › Creating and opening files
- **Préservation** : garantir que le travail est toujours conservé, sauf annulation ou suppression — "Help people be confident that their work is always preserved unless they cancel or delete it." — file-management › Saving work
- **Enregistrement automatique** : éviter d'imposer un enregistrement explicite ; sauvegarder périodiquement pendant l'édition, à la fermeture d'un fichier et au passage à une autre app — "automatically perform periodic saves while they’re editing and when they close a file or switch to another app" — file-management › Saving work
- **Extensions** : masquer les extensions de fichier par défaut, laisser optionnellement les voir, et refléter ce choix dans toutes les interfaces d'enregistrement et d'ouverture — "Hide file extensions by default, but let people view them if they choose." — file-management › Saving work
- **Aperçu** : permettre de prévisualiser un fichier même si l'app ne peut pas l'ouvrir — "Use a Quick Look viewer to let people preview a file even when your app can’t open it." — file-management › Quick Look previews
- **Types personnalisés** : fournir un générateur Quick Look pour les types de fichiers produits par l'app — "A Quick Look generator lets other apps — including the Finder, Files, and Spotlight — display previews of your documents, making it easier for people to find them." — file-management › Quick Look previews

**Spécifique macOS**
- Sur Mac, on parcourt les documents dans le Finder, sans ouvrir d'app de documents — "On a Mac, for example, people use the Finder to access the macOS file system" — file-management › Discussion
- Barre des menus (menu bar) : les commandes de création et d'ouverture se présentent dans le menu Fichier (File menu) — "macOS presents them in the menu bar File menu" — file-management › Creating and opening files
- Action d'ajout : dans une app macOS, elle se place dans le menu Fichier — "In a macOS app, you put the add action in the File menu" — file-management › Creating and opening files
- Navigateur par défaut : l'utiliser, sauf raison importante d'en créer un personnalisé — "Use the default file browser unless you have an important reason to create a custom one." — file-management › Platform considerations › macOS › Custom file management
- Ouverture : prévoir une action de fichiers récents, des critères de filtrage et une ouverture multiple ; le titre du bouton Ouvrir peut refléter la tâche (Insert, par exemple) — "In a macOS open panel, you can customize the title of the Open button to reflect the task" — file-management › Platform considerations › macOS › Custom file management
- Enregistrement : une interface pour changer le nom, le format et l'emplacement ; un nouveau document s'intitule Untitled par défaut — "Provide a save interface to let people change a file’s name, format, or location." — file-management › Platform considerations › macOS › Custom file management
- Boîte d'enregistrement : une vue accessoire peut porter des options utiles (joindre les pièces jointes, par exemple) — "you can add a custom accessory view containing useful settings or options to the Save dialog" — file-management › Platform considerations › macOS › Custom file management
- Synchronisation : une extension Finder Sync (Finder Sync extension) peut afficher des badges d'état, des éléments de menu contextuel et des boutons de barre d'outils dans le Finder — "you can create a Finder Sync app extension to express file synchronization status and control within the Finder" — file-management › Platform considerations › macOS › Finder Sync extensions
- Sans enregistrement automatique (option Ask to keep changes when closing documents des réglages Desktop et Dock) : signaler les modifications non enregistrées et présenter une boîte d'enregistrement à la fermeture du document, à la fermeture de l'app, à la déconnexion ou au redémarrage — "In this scenario, show that a document has unsaved changes and present a save dialog when people choose to close the document, quit your app, log out, or restart." — file-management › Platform considerations › macOS
- Marqueurs : un point sur le bouton de fermeture de la fenêtre et à côté du nom dans le menu Fenêtre (Window menu) — "display a dot on the document window’s close button and next to the document’s name in your app’s Window menu" — file-management › Platform considerations › macOS
- Avec enregistrement automatique, ce point tromperait ; le suffixe Edited peut s'ajouter au titre, à retirer dès l'enregistrement automatique ou manuel — "be sure to remove this suffix as soon as autosave occurs or when people explicitly save their work" — file-management › Platform considerations › macOS

**À transposer dans un shell Linux**
- Lanceur et recherche de hyprdesk : fichiers récents, ouverture de plusieurs éléments, filtres par type ou par date ; libellé de l'action adapté à la tâche.
- Résultats de fichiers dans la recherche : masquer l'extension si le bureau le règle ainsi, et joindre un aperçu (équivalent Quick Look) chaque fois que possible.
- Sélecteur de fichiers du shell (fond d'écran, thème, fichier de calibration) : partir du navigateur de fichiers par défaut du bureau plutôt que d'un navigateur maison.
- Ce que le shell fait éditer (profils de réglages, disposition) se conserve sans action d'enregistrement explicite ; hors de là, la page concerne surtout les apps de documents.

### Offering help (`offering-help`)

**Rôle** : Apporter une aide contextuelle quand l'interface seule ne suffit pas, par des astuces brèves et des infobulles.

**Anatomie et cotes**
- Astuce (tip) : petite vue transitoire qui décrit brièvement l'usage d'une fonctionnalité — "A tip is a small, transient view that briefly describes how to use a feature in your app." — offering-help › Creating tips
- Variantes : astuce en pop-over (popover tip), en ligne (inline tip), en annotation (annotation-style inline tip), en indice (hint-style tip).
- Longueur d'une astuce : une ou deux phrases ; au-delà de trois actions, la fonction est trop complexe pour une astuce — "If a feature requires more than three actions, it’s probably too complicated for a tip." — offering-help › Creating tips
- Cadence de plusieurs astuces : raisonnable, par exemple une fois toutes les 24 heures — "set the display frequency so tips display at a reasonable cadence — for example, once every 24 hours" — offering-help › Creating tips
- Infobulle (tooltip) : petite vue transitoire qui décrit un composant, appelée help tag dans la documentation utilisateur — "A *tooltip* (called a *help tag* in user documentation) displays a small, transient view that briefly describes how to use a component in the interface." — offering-help › Platform considerations › macOS, visionOS
- Longueur d'une infobulle : 60 à 75 caractères au maximum — "As much as possible, limit tooltip content to a maximum of 60 to 75 characters (note that localization often changes the length of text)." — offering-help › Platform considerations › macOS, visionOS
- Délai d'apparition, durée d'affichage, dimensions : non chiffrés dans la HIG.

**Règles**
- **Proportion** : choisir la forme d'aide selon la tâche, vue en ligne pour une ou deux étapes, tutoriel pour une tâche complexe — "Let your app’s tasks inform the types of help people might need." — offering-help › Best practices
- **Rejet facile** : l'aide doit être facile à écarter ou à éviter — "make it easy for people to dismiss or avoid the help if they don’t need it" — offering-help › Best practices
- **Cohérence** : langage et images adaptés au contexte, cohérents avec la plateforme (cliquer sur un Mac, toucher sur un iPhone) et inclusifs — "Use relevant and consistent language and images in your help content." — offering-help › Best practices
- **Concision** : ne pas expliquer le fonctionnement des composants standards ; pour un contrôle inédit, préférer l'animation ou le graphisme à la longue description — "Avoid bloating your help content by explaining how standard components or patterns work." — offering-help › Best practices
- **Type d'astuce** : en pop-over pour préserver le flux, en ligne pour laisser l'entourage visible — "Display a popover tip when you want to preserve the content flow, or an inline tip when you want to ensure that surrounding information is visible." — offering-help › Creating tips
- **Simplicité** : réserver les astuces aux fonctionnalités simples à décrire — "Use tips for simple features." — offering-help › Creating tips
- **Style** : langage direct et orienté action, une ou deux phrases, rien de promotionnel ni d'étranger à la fonction — "Make tips short, actionable, and engaging." — offering-help › Creating tips
- **Audience** : n'afficher une astuce que si la personne peut en bénéficier, selon des règles d'éligibilité par paramètre ou par événement — "Use parameter-based or event-based eligibility rules to control when a tip appears, and only display a tip if someone might benefit from its use." — offering-help › Creating tips
- **Image** : inclure l'image ou le symbole associé à la fonction, dans sa variante pleine — "If there’s an image or symbol that people associate with the feature, consider including it in the tip, and prefer the filled variant." — offering-help › Creating tips
- **Boutons** : des boutons peuvent mener aux réglages de la fonction ou à des ressources complémentaires — "Use buttons to direct people to information or options." — offering-help › Creating tips

**Spécifique macOS**
- Apparition : sur Mac (y compris apps iPhone et iPad), l'infobulle apparaît quand le pointeur stationne sur un élément — "tooltips can appear when a person holds the pointer over an element" — offering-help › Platform considerations › macOS, visionOS
- Périmètre : décrire seulement le contrôle visé, ni les contrôles voisins ni la tâche d'ensemble — "Describe only the control that people indicate interest in." — offering-help › Platform considerations › macOS, visionOS
- Contenu : expliquer l'action ou la tâche déclenchée, en commençant si possible par un verbe — "Explain the action or task the control initiates." — offering-help › Platform considerations › macOS, visionOS
- Nom : en général, ne pas répéter le nom du contrôle — "In general, avoid repeating a control’s name in its tooltip." — offering-help › Platform considerations › macOS, visionOS
- Brièveté : fragment de phrase sans articles si besoin ; beaucoup de texte invite à simplifier l'interface — "If you need a lot of text to describe a control, consider simplifying your interface design." — offering-help › Platform considerations › macOS, visionOS
- Casse : casse de phrase (sentence case) — "Use sentence case." — offering-help › Platform considerations › macOS, visionOS
- Contexte : envisager des textes différents selon l'état du contrôle — "Consider offering context-sensitive tooltips." — offering-help › Platform considerations › macOS, visionOS
- Absent de la page : délai et durée d'affichage, dimensions et position de l'infobulle, aide propre à la barre des menus ou au Dock ; rien à en tirer.

**À transposer dans un shell Linux**
- Infobulles des widgets de la barre, des icônes du dock et des commandes des Réglages : une seule ligne de la longueur cotée plus haut, commençant par un verbe, sans répéter le libellé du contrôle.
- Variantes d'infobulle selon l'état du contrôle (coupé, connecté, muet) plutôt qu'un texte unique.
- Découverte des fonctions du shell : astuces d'une ou deux phrases, en pop-over près du widget concerné, à la cadence cotée plus haut, avec règles d'éligibilité (fonction pas encore utilisée).
- Aide-mémoire des raccourcis du shell : se limiter aux actions propres au shell, sans expliquer les composants standards.
- Les délais d'apparition ne sont pas cotés : les fixer dans la spécification du shell.

### Onboarding (`onboarding`)

**Rôle** : Aider à démarrer rapidement lors de la première utilisation, si l'expérience seule ne suffit pas.

**Anatomie et cotes**
- Éléments cités : parcours d'accueil (onboarding flow), tutoriel, écran de présentation (splash screen), astuces contextuelles, demande d'autorisation (permission request).
- Placement : après la fin du lancement, dont il ne fait pas partie.
- Écran de présentation : affiché juste assez longtemps pour absorber l'information d'un coup d'œil ; aucune durée en secondes, non chiffré dans la HIG.

**Règles**
- **Facultatif** : si l'accueil est nécessaire, un parcours rapide, agréable et facultatif — "if onboarding is necessary, design a flow that’s fast, fun, and optional" — onboarding › Discussion
- **Interactivité** : enseigner par la pratique, en laissant tester une action sans risque — "Teach through interactivity." — onboarding › Best practices
- **Astuces plutôt que parcours** : envisager des astuces contextuelles à la place d'un parcours unique, affichées près de la zone de l'interface qu'elles concernent — "Consider providing a collection of context-specific tips instead of a single onboarding flow." — onboarding › Best practices
- **Peu à mémoriser** : un parcours prérequis reste bref, sans grande quantité d'information à retenir — "If you need to present a prerequisite onboarding flow, design a brief, enjoyable experience that doesn’t require people to memorize a lot of information." — onboarding › Best practices
- **Tutoriel** : le rendre facultatif ; passé une fois, il ne se représente plus aux lancements suivants, mais reste facile à retrouver (aide, compte, réglages) — "If you let people skip the tutorial when they first launch your app or game, don’t present it again on subsequent launches" — onboarding › Best practices
- **Périmètre** : rester centré sur l'expérience offerte ; le système ou l'appareil n'a pas à y être appris — "Keep onboarding content focused on the experience you provide." — onboarding › Best practices
- **Écran de présentation** : l'afficher brièvement, seulement s'il le faut — "Briefly display a splash screen if necessary." — onboarding › Additional content
- **Téléchargements** : ne pas laisser de gros téléchargements entraver l'accueil — "Don’t let large downloads hinder onboarding." — onboarding › Additional content
- **Licences** : éviter d'afficher les conditions de licence dans le parcours — "Avoid displaying licensing details within your onboarding flow." — onboarding › Additional content
- **Configuration** : remettre à plus tard les étapes de configuration non essentielles, des réglages par défaut raisonnables suffisant à démarrer — "Postpone nonessential setup flows or customization steps." — onboarding › Additional requests
- **Autorisations** : si l'accès à des données privées est indispensable, intégrer la demande au parcours pour en montrer l'intérêt ; sinon, la présenter au premier accès à la fonction — "Otherwise, present a permission request when people first access the specific function that relies on private data or resources." — onboarding › Additional requests
- **Évaluations et achats** : laisser vivre l'expérience avant de demander une note ou un achat — "Prefer letting people experience your app or game before prompting them for ratings or purchases." — onboarding › Additional requests

**Spécifique macOS**
- Bloc Platform considerations : aucune considération supplémentaire pour macOS ni pour les autres plateformes — "No additional considerations for iOS, iPadOS, macOS, tvOS, visionOS, or watchOS." — onboarding › Platform considerations
- Aucune mention du Dock, de la barre des menus, des fenêtres ou d'un assistant de configuration du système ; le premier démarrage du bureau lui-même n'est pas traité.

**À transposer dans un shell Linux**
- Premier lancement de hyprdesk : aucune étape de configuration obligatoire, les réglages par défaut suffisent à utiliser le bureau.
- À la place d'un tutoriel général, des astuces contextuelles attachées aux surfaces réelles (barre, dock, lanceur, aperçu des espaces), non réaffichées une fois passées et faciles à retrouver depuis les Réglages.
- Demande d'autorisation (notifications d'une app, accès à un dossier) au premier usage de la fonction, avec explication du bénéfice.
- Lancement de session sans écran de bienvenue ni logo ; tout écran de présentation reste bref.
- Demande de note, de dons ou de contribution seulement après un usage réel.

### Entering data (`entering-data`)

**Rôle** : Recueillir une information en minimisant la saisie et les erreurs.

**Anatomie et cotes**
- Champ sécurisé (secure text-entry field) : l'entrée est masquée, typiquement par un petit cercle plein par caractère — "typically by displaying a small filled circle symbol for each character" — entering-data › Best practices
- Infobulle d'expansion (expansion tooltip) sur macOS : affiche le texte complet d'un champ trop étroit.
- Dimensions des champs, largeurs minimales, délais de validation : non chiffrés dans la HIG.

**Règles**
- **Précollecte** : recueillir à l'avance autant d'information que possible pour réduire ce que la personne doit fournir — "Pre-gathering as much information as possible to minimize the amount of data that people need to supply" — entering-data › Discussion
- **Méthodes d'entrée** : prendre en charge toutes celles qui sont disponibles — "Supporting all available input methods so people can choose the method that works for them" — entering-data › Discussion
- **Système d'abord** : ne pas demander ce qu'on peut obtenir automatiquement ou avec permission — "Get information from the system whenever possible." — entering-data › Best practices
- **Clarté** : indiquer la donnée attendue (invite dans le champ, libellé) et préremplir avec des valeurs par défaut raisonnables — "You can also prefill fields with reasonable default values, which can minimize decision making and speed data entry." — entering-data › Best practices
- **Choix plutôt que texte** : offrir des listes, menus ou sélecteurs quand c'est possible — "When possible, offer choices instead of requiring text entry." — entering-data › Best practices
- **Glisser et coller** : permettre de fournir des données par glisser-déposer ou par collage — "As much as possible, let people provide data by dragging and dropping it or by pasting it." — entering-data › Best practices
- **Validation** : valider les valeurs dès la saisie et signaler un problème dès sa détection — "Dynamically validate field values." — entering-data › Best practices
- **Nombres** : un formateur numérique n'accepte que des valeurs numériques et les affiche selon un format (décimales, pourcentage, devise) — "For numeric data in particular, consider using a number formatter, which automatically configures a text field to accept only numeric values." — entering-data › Best practices
- **Champ obligatoire** : faire comprendre que la donnée requise doit être fournie avant de poursuivre ; le bouton Suivant n'est disponible qu'ensuite — "When data entry is necessary, make sure people understand that they must provide the required data before they can proceed." — entering-data › Best practices
- **Mot de passe** : ne jamais préremplir un champ de mot de passe ; demander le mot de passe ou une authentification biométrique ou par trousseau — "Never prepopulate a password field." — entering-data › Best practices
- **Donnée sensible** : utiliser un champ sécurisé qui masque l'entrée — "Use a secure text-entry field when appropriate." — entering-data › Best practices

**Spécifique macOS**
- Infobulle d'expansion : elle montre la version complète d'un texte rogné ou tronqué dans un champ — "Consider using an expansion tooltip to show the full version of clipped or truncated text in a field." — entering-data › Platform considerations › macOS
- Comportement : comme une infobulle ordinaire, elle apparaît quand le pointeur stationne sur le champ — "An *expansion tooltip* behaves like a regular tooltip, appearing when the pointer rests on top of a field." — entering-data › Platform considerations › macOS
- Elle vaut aussi pour les apps iOS et iPadOS tournant sur Mac.
- Rien d'autre pour macOS : ni menu, ni barre des menus, ni Dock, ni raccourci de saisie.

**À transposer dans un shell Linux**
- Écran de verrouillage : champ de mot de passe jamais prérempli, masqué par un cercle plein par caractère, bouton de validation sans effet tant que le champ est vide.
- Réglages de hyprdesk : listes, menus et sélecteurs de préférence aux champs libres ; champs numériques (tailles, durées) limités aux valeurs valides et corrigés dès la saisie.
- Préremplir ce que le shell connaît déjà (nom d'hôte, écrans, disposition clavier) au lieu de le demander.
- Champ de recherche du lanceur : invite décrivant ce qui est cherchable ; résultats rafraîchis à mesure de la frappe, sans bloquer la saisie.
- Dans une fenêtre étroite, afficher en entier le contenu tronqué d'un champ (infobulle d'expansion).

### Charting data (`charting-data`)

**Rôle** : Présenter des données en graphique pour communiquer clairement, du simple coup d'œil à l'exploration interactive.

**Anatomie et cotes**
- Deux régimes : du graphique simple lisible d'un coup d'œil à l'expérience riche et interactive — "A chart can range from a simple graphic that provides glanceable information to a rich, interactive experience" — charting-data › Discussion
- Taille : à ajuster à la fonctionnalité, au sujet et au niveau de détail ; un petit graphique sert de coup d'œil sur un élément ou d'aperçu d'une version plus grande.
- Dimensions, épaisseurs de trait, corps de texte : non chiffré dans la HIG (renvoi à la page Charts sur les composants).

**Règles**
- **Opportunité** : un graphique met en évidence une information importante d'un jeu de données ; pour de simples données à fournir, une liste ou un tableau suffit — "Use a chart when you want to highlight important information about a dataset." — charting-data › Best practices
- **Simplicité** : garder le graphique simple et laisser choisir quand voir plus de détails — "Keep a chart simple, letting people choose when they want additional details." — charting-data › Best practices
- **Accessibilité** : libellés décrivant valeurs et composants, et éléments d'accessibilité pour l'interaction — "Make every chart in your app accessible." — charting-data › Best practices
- **Types courants** : préférer les types connus (barres, lignes) — "In general, prefer using common chart types." — charting-data › Designing effective charts
- **Innovation** : un graphique d'un genre nouveau s'explique, par exemple par une animation — "If you need to create a chart that presents data in a novel way, help people learn how to interpret the chart." — charting-data › Designing effective charts
- **Niveaux de lecture** : examiner les données à plusieurs niveaux (totaux ou moyennes, sous-ensembles utiles, points singuliers) — "Examine the data from multiple levels or perspectives to find details you can display to enhance the chart." — charting-data › Designing effective charts
- **Texte descriptif** : des titres, sous-titres et annotations mettent en évidence l'essentiel — "Aid comprehension by adding descriptive text to the chart." — charting-data › Designing effective charts
- **Titre résumé** : un bref texte d'ensemble aide à saisir l'essentiel d'un coup d'œil, sans remplacer les libellés d'accessibilité — "Although a descriptive headline or summary can make a chart more accessible, it doesn’t take the place of accessibility labels." — charting-data › Designing effective charts
- **Cohérence** : des graphiques de même finalité suivent la même approche visuelle, sauf différence à souligner — "Prefer consistency across multiple charts, deviating only when you need to highlight differences." — charting-data › Designing effective charts
- **Continuité** : pour un même jeu de données, un seul type et des couleurs, annotations, dispositions et textes constants — "Maintain continuity among multiple charts that use the same data." — charting-data › Designing effective charts
- **Taille** : assez grande pour les détails et l'interaction prévues — "Match the size of a chart to its functionality, topic, and level of detail." — charting-data › Designing effective charts

**Spécifique macOS**
- Bloc Platform considerations : aucune considération supplémentaire pour macOS — "No additional considerations for iOS, iPadOS, macOS, tvOS, visionOS, or watchOS." — charting-data › Platform considerations
- Aucune mention de widgets de bureau, de la barre des menus ou du Dock ; les usages cités (tendances, état courant d'un processus ou d'une quantité qui varie, comparaisons) sont transversaux.

**À transposer dans un shell Linux**
- Widgets de supervision de hyprdesk (processeur, mémoire, réseau, batterie) : petits graphiques de tendance lisibles d'un coup d'œil, avec une vue étendue reprenant le même style.
- Un court texte résumé (valeur courante, sens de la variation) et des libellés accessibles pour chaque graphique.
- Même type, mêmes couleurs et mêmes annotations d'un widget à l'autre pour une même grandeur.
- Une valeur unique sans tendance n'appelle pas un graphique : un texte suffit quand il n'y a rien à analyser.

### Playing audio (`playing-audio`)

**Rôle** : Faire en sorte que le son se comporte comme attendu quand le volume, la sortie ou le contexte change.

**Anatomie et cotes**
- Vue de volume (volume view) fournie par le système : un curseur de niveau et une commande de réacheminement de la sortie — "The volume view includes a volume-level slider and a control for rerouting audio output." — playing-audio › Best practices
- Mode silencieux : seuls jouent les sons explicitement initiés (média, alarmes, messages audio et vidéo) — "When a device is in silent mode, it plays only the audio that people explicitly initiate, like media playback, alarms, and audio/video messaging." — playing-audio › Discussion
- Casque : le son se réachemine sans interruption au branchement ; la lecture se met en pause immédiatement au débranchement — "When connecting headphones, people expect sound to reroute automatically without interruption; when disconnecting headphones, they expect playback to pause immediately." — playing-audio › Discussion
- Le réglage de volume attendu agit sur tous les sons du système ; courbe de volume, paliers, pas de réglage : non chiffrés dans la HIG.

Catégories audio (audio categories) proposées par la page :

| Catégorie | Sens | Comportement |
|---|---|---|
| Solo ambient | Son non essentiel, qui coupe les autres sons (jeu avec bande-son) | Répond à l'interrupteur silencieux ; ne se mélange pas ; ne joue pas en arrière-plan |
| Ambient | Son non essentiel, qui ne coupe pas les autres sons | Répond à l'interrupteur silencieux ; se mélange ; ne joue pas en arrière-plan |
| Playback | Son essentiel, peut se mélanger (livre audio, apprentissage) | Ne répond pas à l'interrupteur ; se mélange ou non ; joue en arrière-plan |
| Record | Son enregistré (prise de notes audio) | Ne répond pas à l'interrupteur ; ne se mélange pas ; enregistre en arrière-plan |
| Play and record | Enregistrement et lecture, éventuellement simultanés (messagerie audio, appel vidéo) | Ne répond pas à l'interrupteur ; se mélange ou non ; enregistre et joue en arrière-plan |

- La catégorie choisie détermine si le son se mélange aux autres et joue en arrière-plan — "your app’s sounds can mix with other audio, play while your app is in the background" — playing-audio › Best practices

**Règles**
- **Volume global** : l'app peut régler des niveaux relatifs, jamais le volume d'ensemble, que le volume du système gouverne toujours — "Adjust levels automatically when necessary — don’t adjust the overall volume." — playing-audio › Best practices
- **Réacheminement** : permettre de choisir une autre sortie audio, sauf raison impérieuse — "Permit rerouting of audio when possible." — playing-audio › Best practices
- **Commande de volume** : utiliser la vue de volume du système — "Use the system-provided volume view to let people make audio adjustments." — playing-audio › Best practices
- **Catégorie** : choisir une catégorie adaptée à l'usage du son ; ne pas couper la musique d'une autre app sans nécessité — "Choose an audio category that fits the way your app or game uses sound." — playing-audio › Best practices
- **Commandes externes** : ne réagir aux commandes de lecture venues de l'extérieur que si l'app joue, se trouve dans un contexte clairement audio ou est reliée à un appareil Bluetooth ou AirPlay — "If your app is actively playing audio, in a clear audio-related context, or connected to a device that uses Bluetooth or AirPlay, it’s fine to respond to audio controls." — playing-audio › Best practices
- **Pas de détournement** : ne pas redéfinir le sens d'une commande audio ; une commande non prise en charge reste sans réponse — "Avoid repurposing audio controls." — playing-audio › Best practices
- **Commandes propres** : n'en créer que pour des commandes absentes du système — "Consider creating custom audio player controls only if you need to offer commands that the system doesn’t support." — playing-audio › Best practices
- **Fin d'un son temporaire** : prévenir les autres apps quand le son qui les interrompait s'achève — "Let other apps know when your app finishes playing temporary audio." — playing-audio › Best practices
- **Interruptions** : décider comment répondre aux interruptions de la session audio — "Determine how to respond to audio-session interruptions." — playing-audio › Handling interruptions
- **Reprise** : à la fin d'une interruption, décider de reprendre ou non la lecture — "When an interruption ends, determine whether to resume audio playback automatically." — playing-audio › Handling interruptions
- **Type d'interruption** : la décision dépend du type, récupérable (resumable, appel entrant) ou non (nouvelle liste de lecture lancée), et du type d'app — "Use the interruption type and your app’s type to decide whether to resume playback automatically." — playing-audio › Handling interruptions

**Spécifique macOS**
- Sons de notification : sur macOS, ils se mélangent aux autres sons par défaut — "In macOS, notification sounds mix with other audio by default." — playing-audio › Platform considerations › macOS
- Le tableau des catégories fait référence à l'interrupteur Ring/Silent de l'iPhone et de l'iPad ; la page ne dit rien de la transposition de ce modèle sur macOS.
- Absent de la page : widget ou commande média du bureau, menu du volume dans la barre des menus, choix de sortie par application ; rien à en tirer.

**À transposer dans un shell Linux**
- Widget média de la barre : n'agir que sur le lecteur courant ; ne pas redéfinir les commandes précédent, lecture et suivant ; n'ajouter de commandes propres que pour ce que le lecteur n'offre pas.
- Curseur de volume du shell : une commande unique de niveau, éventuellement accompagnée du choix de la sortie ; les apps ne modifient pas le volume global à la place de la personne.
- Casque : réacheminer sans coupure au branchement, mettre en pause au débranchement.
- Sons du shell (notifications, alertes) : se mélangent à la lecture en cours, ne la coupent pas.
- Après une interruption (appel, autre lecteur), ne reprendre la lecture que si l'interruption était récupérable et si la lecture avait été lancée explicitement.

### Managing accounts (`managing-accounts`)

**Rôle** : N'imposer un compte que s'il est indispensable, et rendre sa création, son utilisation et sa suppression claires.

**Anatomie et cotes**
- Éléments cités : vue de connexion (sign-in view), bouton de méthode d'authentification, clé d'accès (passkey), authentification biométrique, parcours de suppression de compte.
- Méthodes citées : Sign in with Apple, clé d'accès, mot de passe avec authentification à deux facteurs.
- Aucune dimension ni délai : non chiffré dans la HIG.

**Règles**
- **Nécessité** : ne demander un compte que si la fonctionnalité de base l'exige — "Ask people to create an account only if your core functionality requires it; otherwise, let people enjoy your app or game without one." — managing-accounts › Discussion
- **Bénéfices** : expliquer les raisons et les bénéfices du compte dans la vue de connexion — "Explain the benefits of creating an account and how to sign up." — managing-accounts › Best practices
- **Connexion tardive** : la retarder le plus possible ; on abandonne une app qui force à se connecter avant toute action utile — "People often abandon apps when they’re forced to sign in before they can do anything useful." — managing-accounts › Best practices
- **Méthode** : sans Sign in with Apple, préférer une clé d'accès ; avec des mots de passe, renforcer par une authentification à deux facteurs — "augment security by requiring two-factor authentication" — managing-accounts › Best practices
- **Identification** : toujours nommer la méthode d'authentification proposée — "Always identify the authentication method you offer." — managing-accounts › Best practices
- **Disponibilité** : ne mentionner que les méthodes disponibles dans le contexte courant — "Refer only to authentication methods that are available in the current context." — managing-accounts › Best practices
- **Biométrie** : éviter un réglage propre à l'app pour l'authentification biométrique, qui se règle au niveau du système — "People turn on biometric authentication at the system level, so presenting an in-app setting is redundant and could be confusing." — managing-accounts › Best practices
- **Vocabulaire** : réserver le terme passcode au code de déverrouillage de l'appareil, et non à l'authentification à un compte — "People create a passcode to unlock their device or authenticate for Apple services." — managing-accounts › Best practices
- **Suppression** : qui permet de créer un compte doit permettre de le supprimer, pas seulement de le désactiver — "If you help people create an account within your app or game, you must also help them delete it, not just deactivate it." — managing-accounts › Deleting accounts
- **Accès** : offrir un moyen clair de lancer la suppression dans l'app, ou un lien direct vers la page web — "Provide a clear way to initiate account deletion within your app or game." — managing-accounts › Deleting accounts
- **Cohérence** : même expérience de suppression dans l'app et sur le site — "Provide a consistent account-deletion experience whether people perform it within your app or game or on the website." — managing-accounts › Deleting accounts
- **Programmation** : envisager une suppression différée, avec en plus une option de suppression immédiate — "Consider letting people schedule account deletion to occur in the future." — managing-accounts › Deleting accounts
- **Information** : dire quand la suppression sera achevée et prévenir à la fin — "Tell people when account deletion will complete, and notify them when it’s finished." — managing-accounts › Deleting accounts

**Spécifique macOS**
- La préférence pour la clé d'accès vaut explicitement pour les apps macOS — "If you don’t use Sign in with Apple in your iOS, iPadOS, macOS, or visionOS app, prefer using a passkey." — managing-accounts › Best practices
- Bloc Platform considerations : aucune considération supplémentaire pour macOS — "No additional considerations for iOS, iPadOS, macOS, or visionOS." — managing-accounts › Platform considerations
- La section TV provider accounts concerne tvOS ; rien à en tirer pour un bureau.
- Absent de la page : compte système du bureau, écran de connexion ou de verrouillage, synchronisation des réglages du shell ; non traité.

**À transposer dans un shell Linux**
- Le shell n'a, par défaut, aucune raison d'exiger un compte : aucune connexion avant usage.
- Si un service en ligne est proposé (sauvegarde des réglages, synchronisation), ne le proposer qu'au moment où il devient utile, avec ses bénéfices, sans bloquer le reste.
- Vocabulaire de l'écran de verrouillage : le terme de code sert au déverrouillage de la session, jamais à un éventuel compte du shell.
- Nommer explicitement la méthode d'authentification offerte et n'afficher que celles disponibles sur la machine.
- Si compte il y a : suppression accessible depuis les Réglages, avec information sur son achèvement.

### Valeurs de référence

| Propriété | Valeur | Source (slug › section) |
|---|---|---|
| Seuil d'apparition de l'image de glissement | environ trois points (about three points) | drag-and-drop › Providing feedback |
| Apparence de l'image de glissement | représentation translucide du contenu | drag-and-drop › Providing feedback |
| Touche forçant la copie au dépôt | Option, vérifiée au moment du dépôt | drag-and-drop › Accepting drops |
| Badge de glissement multiple | petit ovale plein contenant un nombre | drag-and-drop › Platform considerations › macOS |
| Longueur maximale d'une infobulle | 60 à 75 caractères | offering-help › Platform considerations › macOS, visionOS |
| Longueur d'une astuce | une ou deux phrases | offering-help › Creating tips |
| Limite de complexité d'une astuce | plus de trois actions : trop compliqué pour une astuce | offering-help › Creating tips |
| Cadence d'affichage des astuces (exemple) | une fois toutes les 24 heures | offering-help › Creating tips |
| Masquage d'un champ sécurisé | petit cercle plein par caractère | entering-data › Best practices |
| Extensions de fichier | masquées par défaut | file-management › Saving work |
| Titre par défaut d'un nouveau document | Untitled | file-management › Platform considerations › macOS › Custom file management |
| Marqueurs de modifications non enregistrées | point sur le bouton de fermeture et dans le menu Window | file-management › Platform considerations › macOS |
| Suffixe de titre d'un document modifié | Edited | file-management › Platform considerations › macOS |
| Vue de volume système | curseur de niveau et commande de réacheminement | playing-audio › Best practices |
| Sons de notification sur macOS | se mélangent aux autres sons par défaut | playing-audio › Platform considerations › macOS |
| Catégories audio | Solo ambient, Ambient, Playback, Record, Play and record | playing-audio › Best practices |

## Seconde partie — hyprdesk aujourd'hui, écarts et actions

### Ce que hyprdesk fait aujourd'hui
| Surface | Ce qui existe | Valeurs clés | Preuves |
| --- | --- | --- | --- |
| Aperçu des espaces : glisser-déposer d'une fenêtre | Une fenêtre est saisie dans sa minimap et déposée dans la carte d'un autre espace (déplacement silencieux) | seuil 6 px ; bordure d'accent de 2 px et ombre sur la carte sous le curseur ; pastille (icône 18 px, titre à 26 caractères) à 14 px du curseur ; Échap ferme l'écran | `src/ui/mod.rs:87` ; `src/ui/overview.rs:487`, `516-531`, `560`, `798-845`, `870-875`, `1022-1072` |
| Modules de barre : réordonnancement | Poignées dans la barre et dans Réglages → Barre, repère d'insertion, boutons fléchés | curseur Grab puis Grabbing ; trait d'insertion de 2 px d'accent ; Échap ou sortie de zone annule | `src/ui/plugins/bar.rs:640`, `662-675`, `691-716`, `726-737` ; `src/ui/mod.rs:5018-5045` ; `DESIGN.md:19` |
| Dock comme cible | Aucun dépôt ; une application s'épingle par un menu, un champ de réglages, et se retire par « Retirer » | — | `src/ui/context.rs:718` ; `src/ui/plugins/dock.rs:192`, `240` ; `src/ui/plugins/dock/surface.rs` |
| Panorama : gestes | Défilement au doigt et appui long ; ce ne sont pas des glisser-déposer au sens de la HIG | seuil 14 px ; curseur Grabbing | `src/ui/plugins/panorama/surface/drag.rs:17`, `185-187` ; `hold.rs:1-5` |
| Recherche de fichiers (`:f`) | Démon d'index, puces de nature (tout, fichiers, dossiers, liens) et de catégorie, ouverture par `xdg-open` | 100 résultats au plus ; nom, taille, dossier parent ; aucune ouverture multiple ni « Ouvrir avec » | `src/fileindex/proto.rs:23`, `32-38` ; `src/ui/search.rs:885-912`, `1198`, `1214-1300`, `1303-1357` ; `src/launch.rs:211-221` |
| Applications par défaut, sélecteur de fichiers | Listes déroulantes appliquées aussi au système ; boîte de dialogue du portail FreeDesktop | — | `src/ui/plugins/apps.rs:139` ; `src/portal.rs:1-3`, `76-77` |
| Assistant de premier démarrage | Dix étapes, chaque choix appliqué aussitôt, tout annulable au récapitulatif | carte 960 × 640 ; calque à clavier exclusif | `src/main.rs:321-323` ; `src/ui/wizard.rs:47-98`, `474`, `550`, `995-1032` |
| Visite des Réglages | Dix blocs de lecture, proposée une fois à la première ouverture, « Plus tard », quittable, relançable | fenêtre 780 × 520 | `src/ui/tour.rs:21-23`, `47-330`, `437-460`, `550` |
| Réglages : saisie | Interrupteurs, curseurs bornés, listes, champs de couleur et de texte à valider par Entrée | erreur rendue sous le champ en couleur d'alerte | `src/ui/hyprset.rs:213-219`, `234-345`, `466-481` ; `src/ui/hyprlists.rs:511-545`, `987-994` |
| Connexion, verrouillage, coffre myMFA | Champs de mot de passe masqués, tampon effacé après envoi ; aucune création de compte | validation du coffre dès la frappe | `src/greeter_ui.rs:758-786`, `962-964` ; `src/lock/mod.rs:642` ; `src/ui/mymfa.rs:125-137`, `451-453`, `575`, `725-741` |
| Graphiques | Widget Système : libellé, valeur, barre de 4 px en option ; Panorama : anneaux et courbes sur historique | seuil d'alerte `monitoring.threshold` ; couleur par grandeur | `src/widgets/system.rs:50-125` ; `src/ui/widget_view.rs:90-117` ; `src/ui/plugins/panorama/surface/instruments.rs:20-22`, `324-327`, `534-547` |
| Audio | Carte Volume : curseur et choix de la sortie ; widget média : précédent, lecture-pause, suivant | niveau par `wpctl` | `src/ui/plugins/bar/popover.rs:1626-1636` ; `src/osd.rs:84-116` ; `src/widgets/media.rs:301-320` ; `src/ui/widget_view.rs:180-186` |
| Aide | 22 infobulles dans 11 fichiers ; une astuce en ligne | délai de 400 ms sur le bouton « Selon le mode » | `src/ui/panel.rs:549`, `913` ; `src/ui/plugins/bar.rs:1213-1233` ; `src/ui/plugins/widgets.rs:385` |

### Écarts
| # | Composant | Nature | Force | Règle HIG (slug › section) | Réel dans hyprdesk (preuve) | Action proposée | Effort | Confiance |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| DAT-01 | Assistant de premier démarrage | ÉCART | Impératif | onboarding › Additional requests : « Postpone nonessential setup flows or customization steps. » ; onboarding › Discussion : « design a flow that's fast, fun, and optional » | L'assistant s'ouvre de lui-même au premier démarrage (`src/main.rs:321-323`), en dix étapes dont cinq de personnalisation : Apparence, Barre, Bureau, Performance, Veille et sécurité (`src/ui/wizard.rs:47-98`). Le calque prend le clavier en exclusif (`wizard.rs:474`). « Passer » avance d'une étape (`wizard.rs:550`, `1026-1032`) ; « Annuler l'assistant » n'existe qu'au récapitulatif (`wizard.rs:995-1021`) ; aucune sortie clavier (zéro occurrence d'`Escape` dans le fichier). | Rendre l'assistant quittable à toute étape (bouton « Passer l'assistant » et Échap) et retirer du premier démarrage les cinq étapes de personnalisation, qui restent accessibles par le bouton « Assistant » des Réglages (`src/ui/panel.rs:943-949`). | S | haute |
| DAT-02 | Champs de couleur et de texte des réglages | ÉCART | Impératif | entering-data › Best practices : « Dynamically validate field values. » | La frappe ne fait que ranger le brouillon et retire l'erreur (`src/ui/hyprset.rs:461-464`). L'analyse n'a lieu qu'à Entrée (`hyprset.rs:466-481`). Seul indice avant cela : les échantillons de couleur disparaissent (`hyprset.rs:309-313`). | Analyser la valeur à chaque frappe, dans `Msg::Draft`, et afficher le message dès qu'elle est invalide, avec le rendu existant de `hyprset.rs:213-219`. | S | haute |
| DAT-03 | Résultats de recherche de fichiers | ÉCART | Impératif | file-management › Quick Look previews : « Use a Quick Look viewer to let people preview a file even when your app can't open it. » | Un résultat se réduit à une icône de type, au nom et à « taille · dossier » (`src/ui/search.rs:1303-1357`) ; aucune vignette ni aucun extrait (recherche d'aperçu vide dans `ui/search.rs`, `fileindex/`, `launch.rs`). L'ouverture lance `xdg-open` sans lire son résultat (`src/launch.rs:211-221`) ; un échec ne produit qu'une ligne de journal (`ui/search.rs:897-908`). | Joindre un aperçu au résultat sélectionné (vignette d'image, premières lignes d'un texte, date de modification sinon) et signaler à l'écran l'échec d'ouverture à sa place. | M | haute |
| DAT-04 | Dock comme cible de dépôt | ABSENT | Impératif atténué | drag-and-drop › Best practices : « As much as possible, support drag and drop throughout your app. » | Le shell n'accepte aucun dépôt : ni `start_drag`, ni `DataDevice`, ni `wl_data` dans `src` ; `text/uri-list` ne sert qu'à copier un fichier dans le presse-papiers (`src/ui/context.rs:85`, `468`). Le dock n'a pas de zone de dépôt (`src/ui/plugins/dock/surface.rs`). | Accepter sur le dock le dépôt d'une application pour l'épingler, la cible étant signalée par la bordure d'accent de l'aperçu. | L | haute |
| DAT-05 | Retrait d'un casque pendant une lecture | ABSENT | Impératif | playing-audio › Discussion : « when disconnecting headphones, they expect playback to pause immediately » ; playing-audio › Handling interruptions : « Determine how to respond to audio-session interruptions. » | Aucune réaction : `headphone`, `casque` et `unplug` ne se rencontrent que dans des tests (`src/sources/bluetooth.rs:243-291`, `src/config/lua.rs:3793`). Le lecteur du shell n'émet que `PlayPause`, `Next`, `Previous` et `Seek` (`src/widgets/media.rs:301-320`). | Mettre le lecteur courant en pause (`Pause` MPRIS) quand un casque, filaire ou Bluetooth, se déconnecte pendant une lecture. | M | moyenne (un réglage de WirePlumber, hors dépôt, pourrait déjà le faire) |
| DAT-06 | Écran de connexion : envoi à vide | ÉCART | Impératif | entering-data › Best practices : « make sure people understand that they must provide the required data before they can proceed » | Entrée dans un mot de passe vide envoie une réponse vide à greetd (`src/greeter_ui.rs:758-786`). Le bouton « Se connecter » reste actif et un identifiant vide est ignoré sans message (`greeter_ui.rs:718-721`, `981-995`). | Ne rien envoyer tant que le champ requis est vide : bouton « Se connecter » inactif sans identifiant, `GMsg::Send` sans effet sans mot de passe, champ gardé au focus. | S | haute |
| DAT-07 | Découverte des fonctions du shell | ABSENT | Recommandation | onboarding › Best practices : « Consider providing a collection of context-specific tips instead of a single onboarding flow. » ; offering-help › Creating tips | Une seule astuce dans tout le shell (`src/ui/plugins/widgets.rs:385`, à l'ouverture des réglages). Ni cadence ni règle d'éligibilité. L'aide de l'aperçu des espaces ne mentionne pas le glisser d'une fenêtre : Naviguer, Ouvrir, Fermer (`src/ui/overview.rs:729-732`). | Des astuces d'une ou deux phrases, en pop-over près de leur surface (barre, dock, lanceur, aperçu des espaces), montrées une fois par fonction, au plus toutes les 24 heures, pour une fonction pas encore utilisée. | M | haute |
| DAT-08 | Aperçu des espaces : cible de dépôt | ÉCART | Impératif | drag-and-drop › Providing feedback : « Display highlighting or other visual cues only while the content is positioned above the destination » | Toute carte sous le curseur prend la bordure d'accent et l'ombre, y compris celle qui contient déjà la fenêtre (`src/ui/overview.rs:870-875`). Y déposer ne fait rien (`overview.rs:524-530`). | Marquer comme cible seulement les cartes autres que celle d'origine de la fenêtre glissée. | S | haute |
| DAT-09 | Champs numériques des listes de règles | ÉCART | Impératif | entering-data › Best practices : « Dynamically validate field values. » | Dans un champ `Field::Num`, une saisie non numérique est rangée comme chaîne (`src/ui/hyprlists.rs:511-545`), puis appliquée sans message (`hyprlists.rs:987-994`). | N'accepter que des nombres dans un champ `Field::Num` et afficher l'erreur sous le champ. | S | moyenne |
| DAT-10 | Extensions dans les résultats de fichiers | ÉCART | Impératif | file-management › Saving work : « Hide file extensions by default, but let people view them if they choose. » | Le nom est servi avec son extension (`src/fileindex/index.rs:576`) et affiché tel quel (`src/ui/search.rs:1333`). Les seuls réglages d'extensions concernent l'indexation (`src/ui/plugins/index.rs:123`, `516-523`). | Afficher le nom sans extension pour les fichiers d'une catégorie connue, avec un réglage « Afficher les extensions » dans Réglages → Recherche de fichiers. | S | moyenne (la transposition suppose un réglage du bureau qui n'existe pas ici) |
| DAT-11 | Visite des Réglages | PISTE | Recommandation | onboarding › Best practices : « Teach through interactivity. » et « doesn't require people to memorize a lot of information » | La visite est un texte à lire : dix blocs, un paragraphe par réglage (`src/ui/tour.rs:47-330`), pendant que le tiroir s'ouvre derrière. À la fin de l'assistant, un bouton l'enchaîne (`src/ui/wizard.rs:2636-2641`). | Remplacer la lecture par de courts essais guidés : chaque bloc demande une action dans la surface réelle (ouvrir l'aperçu, glisser un module). | M | haute |
| DAT-12 | Annulation d'un déplacement de fenêtre | PISTE | Recommandation | drag-and-drop › Best practices : « Prefer letting people undo a drag-and-drop operation. » | Un dépôt déplace la fenêtre sans possibilité de retour (`src/ui/overview.rs:531`). Échap ferme l'écran (`overview.rs:560`, aide « Échap Fermer » en `732`), alors qu'il annule d'abord un renommage (`overview.rs:541-547`). | Une annulation : Échap interrompt le geste sans fermer l'écran, et le déplacement déposé propose « Annuler ». | S | haute |
| DAT-13 | Jauges du widget Système et instruments de Panorama | PISTE | Recommandation | charting-data › Designing effective charts : « Prefer consistency across multiple charts » et « Maintain continuity among multiple charts that use the same data. » | Même grandeur, deux langages : au bureau, barre horizontale de 4 px dans la couleur d'accent (`src/ui/widget_view.rs:90-117`) ; dans Panorama, anneau ou courbe d'une couleur propre à la grandeur (`src/ui/plugins/panorama/surface/instruments.rs:324-327`, `534-547`). | Reprendre au bureau la couleur par grandeur de Panorama (`metric_color`) pour les jauges du widget Système. | S | haute |
| DAT-14 | Tendance dans le widget Système | PISTE | Recommandation | charting-data › Best practices : « Use a chart when you want to highlight important information about a dataset. » | Le widget Système ne montre que des valeurs instantanées (`src/widgets/system.rs:50-125`). L'historique n'existe que dans Panorama (`src/ui/plugins/panorama/surface/instruments.rs:20-22`). | Ajouter au widget Système une courbe de tendance par grandeur, dessinée par le même code que Panorama. | M | haute |
| DAT-15 | Fichiers récents dans la recherche | PISTE | Recommandation | file-management › Platform considerations › macOS › Custom file management : fichiers récents, critères de filtrage, ouverture multiple | La requête au démon ne connaît que le texte, la limite, la nature et la catégorie (`src/fileindex/proto.rs:32-38`). La date de modification est indexée mais jamais servie (`src/fileindex/index.rs:95`, `580`). Un seul résultat s'ouvre à la fois (`src/ui/search.rs:885-912`). | Ajouter une puce « Récents » (modifiés aujourd'hui, cette semaine) fondée sur la date indexée. | S | haute |
| DAT-16 | Image de glissement de l'aperçu | PISTE | Recommandation | drag-and-drop › Providing feedback : « It works well to create a translucent representation of the content people are dragging. » | La pastille est opaque : fond du thème à 0,94 (`src/ui/overview.rs:798-845`). Sa forme est décrite dans `DESIGN.md:21` et se conserve. | Rendre la pastille translucide, avec une opacité propre indépendante du fondu d'apparition. | S | haute |
| DAT-17 | Curseur pendant le glisser d'une fenêtre | PISTE | Recommandation | drag-and-drop › Platform considerations › macOS : « you might want to use the drag link, disappearing item, and operation not allowed pointers » | Aucun curseur n'est défini dans l'aperçu : `mouse::Interaction` n'y apparaît pas et la flèche reste pendant le glisser. La barre, elle, passe de Grab à Grabbing (`src/ui/plugins/bar.rs:711-714`). | Afficher le curseur Grabbing dès que le seuil de glisser est franchi dans l'aperçu. | S | haute |
| DAT-18 | Infobulles d'état des modules et des widgets | PISTE | Recommandation | offering-help › Platform considerations › macOS, visionOS : « Consider offering context-sensitive tooltips. » | 22 appels à `tooltip(` dans tout `src/ui`, sur 11 fichiers ; aucun dans `src/ui/plugins/dock/surface.rs`, `src/ui/widget_view.rs` ni `src/ui/overview.rs`. | Une infobulle d'état pour chaque module de barre et chaque commande de widget (coupé, connecté, muet), d'une ligne de 75 caractères au plus. | S | moyenne |
| DAT-19 | Seuil d'apparition de l'image de glissement | ÉCART | Cote | drag-and-drop › Providing feedback : « Display a drag image as soon as people drag a selection about three points. » | Seuil de 6 px (`src/ui/mod.rs:87`), utilisé par l'aperçu (`src/ui/overview.rs:487`, `684`). | Abaisser à 3 px le seuil qui fait apparaître l'image, sans changer celui qui distingue le clic. | S | haute |
| DAT-20 | Infobulle du bouton « Selon le mode » | ÉCART | Cote | offering-help › Platform considerations › macOS, visionOS : « As much as possible, limit tooltip content to a maximum of 60 to 75 characters » | 86 caractères (`src/ui/plugins/bar.rs:1225`). | Ramener cette infobulle à une phrase de 75 caractères au plus, commençant par un verbe. | S | haute |

### Ce qui est déjà conforme
- drag-and-drop › Providing feedback (cibles signalées pendant le survol seulement) | `src/ui/overview.rs:870-875`, `1022-1027`, `1065-1072` : bordure et ombre tant que le curseur est sur la carte.
- drag-and-drop › Providing feedback (retour continu) | `src/ui/plugins/bar.rs:662-675` : repère d'insertion suivant la position de dépôt.
- drag-and-drop › Platform considerations › macOS (curseur adapté à l'opération) | `src/ui/plugins/bar.rs:711-714` : Grab puis Grabbing ; `src/ui/plugins/panorama/surface/drag.rs:185-187`.
- drag-and-drop › Best practices (annulation) | `src/ui/mod.rs:5029-5039` : Échap et sortie de zone annulent le glisser d'un module ; `DESIGN.md:19`.
- drag-and-drop › Best practices (alternatives) | `src/ui/plugins/bar/window.rs:98` : « Déplacer vers l'espace » ; `src/ui/plugins/bar.rs:726-737` : flèches de déplacement ; `src/ui/context.rs:718` et `src/ui/plugins/dock.rs:240` : épingler.
- file-management › Platform considerations › macOS (navigateur par défaut) | `src/portal.rs:1-3`, `76-77` : portail `org.freedesktop.portal.FileChooser`.
- file-management › Saving work (préservation, pas d'enregistrement explicite) | `src/ui/hyprset.rs:453-460` et `src/ui/hyprlists.rs:987-994` : chaque changement est appliqué.
- entering-data › Best practices (choix plutôt que texte) | `src/ui/hyprset.rs:234-345` : interrupteur, curseur borné, listes ; `src/ui/plugins/apps.rs:139`.
- entering-data › Best practices (système d'abord) | `src/ui/wizard.rs:244-265`, `332-345` : disposition clavier lue ; `src/greeter_ui.rs:64-68` : dernier identifiant rappelé.
- entering-data › Best practices (invite, résultats à la frappe) | `src/ui/search.rs:1162`, `1198` ; résultats interrogés à chaque frappe (`src/ui/search.rs:13-15`, `783`).
- entering-data › Best practices (validation dès la saisie, champ requis) | `src/ui/mymfa.rs:725-741` : reste à saisir ou écart signalé pendant la frappe ; `mymfa.rs:451-453` : rien n'est envoyé à vide.
- entering-data › Best practices (champ sécurisé, mot de passe jamais prérempli) | `src/greeter_ui.rs:962`, `src/lock/mod.rs:642`, `src/ui/mymfa.rs:575` ; tampon effacé (`src/greeter_ui.rs:755`, `781` ; `src/ui/mymfa.rs:116-120`).
- managing-accounts › Discussion (aucun compte exigé) | aucun compte ni connexion au shell ; coffre myMFA local sans identifiant en ligne.
- managing-accounts › Best practices (méthode nommée, méthodes disponibles seulement) | `src/greeter_ui.rs:958-960` : l'invite de greetd nomme la méthode demandée sur la machine.
- playing-audio › Best practices (vue de volume : niveau et réacheminement) | `src/ui/plugins/bar/popover.rs:1626-1636` ; `src/osd.rs:84-116` : le shell n'agit que sur le niveau.
- playing-audio › Best practices (pas de détournement) | `src/widgets/media.rs:301-320`, `src/ui/widget_view.rs:180-186` : précédent, lecture-pause, suivant tels quels.
- playing-audio › Platform considerations › macOS (sons de notification mêlés) | sans objet : le shell ne joue aucun son (`canberra`, `paplay`, `pw-play`, `aplay` absents de `src`).
- onboarding › Best practices (tutoriel facultatif, non representé, retrouvable) | `src/ui/tour.rs:412-415`, `437-460`, `550` ; relance par le bouton de l'en-tête (`tour.rs:5-6`).
- onboarding › Best practices (interactivité, nothing to memorize) | `src/ui/wizard.rs:60` : la disposition clavier s'essaie dans un champ.
- offering-help › Best practices (rejet facile) | `src/ui/tour.rs:458` : « Plus tard ».
- offering-help › Best practices (aide en ligne pour un geste) | `src/ui/plugins/bar.rs:640` : « Faites glisser un module pour le déplacer. »
- offering-help › Platform considerations (brièveté, verbe) | `src/ui/panel.rs:549` : « Remettre cette page à sa configuration par défaut » (49 caractères).
- charting-data › Designing effective charts (texte descriptif, types courants) | `src/ui/widget_view.rs:77-90` : libellé et valeur ; `src/ui/plugins/panorama/surface/instruments.rs:324-400` : titre, valeur, courbe.
- charting-data › Best practices (une valeur sans tendance reste en texte) | `src/widgets/system.rs:50-125` : charge et durée d'allumage en texte.

### À vérifier sur un rendu
- Aperçu des espaces avec beaucoup d'espaces de travail : la grille se resserre sans défilement (`src/ui/overview.rs:228-266`) ; lisibilité des miniatures et accessibilité de la carte cible quand le glisser dure.
- Aperçu sans aucun espace de travail : état vide, titre et aides seuls.
- Pastille de glissement : opacité perçue sur un fond clair et sur un fond chargé.
- Résultats de fichiers : nom long et chemin tronqué en tête de chaîne (`src/ui/search.rs:1304-1322`), sans infobulle d'expansion.
- Widget Système : barre de 4 px à côté de la valeur, seuil d'alerte franchi, sur les deux palettes claire et sombre.
- Courbes lissées de Panorama (`src/ui/plugins/panorama/surface/instruments.rs:549-551`) : le lissage dessine-t-il des points hauts que les relevés n'ont pas atteints ?
- Écran de connexion et verrouillage : un point plein par caractère dans le champ masqué.
- Infobulles du dock : délai et longueur (400 ms dans le style Convex, `DESIGN.md:53`), la HIG ne cotant ni l'un ni l'autre.
