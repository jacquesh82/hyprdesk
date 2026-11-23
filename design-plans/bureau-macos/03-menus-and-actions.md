# Composants : menus et actions — macOS d'après la HIG, face à hyprdesk

> Proposition de design, rien d'implémenté. Plan d'ensemble et priorités : `plan-d-action.md` ; méthode et périmètre : `README.md`.

> Référentiel : Human Interface Guidelines d'Apple, export Markdown officiel relevé le 23 novembre 2026. Code : commit `00f6d69` (arbre de travail du 23 novembre 2026, avec des modifications non commitées).

## Première partie — Spécification d'après la HIG

Lecture : chaque puce chiffrée ou normative porte sa preuve sur sa propre ligne — citation anglaise mot pour mot entre guillemets doubles, puis son origine sous la forme « slug › titre de section exact » (les blocs imbriqués sont enchaînés, par exemple « Platform considerations › macOS › Menu bar extras »). Les puces « À transposer dans un shell Linux » sont des déductions pour hyprdesk : elles reformulent, pour une surface donnée (barre, dock, lanceur, menus contextuels, réglages), une règle déjà prouvée plus haut dans la même section. « Non spécifié » signale un silence de cet export : ne pas le combler par une valeur supposée de macOS.

### Résumé

- Périmètre : sur macOS s'appliquent la barre des menus (menu bar), les menus, les menus du Dock, les menus d'édition, les menus contextuels, les boutons localisés, les boutons à menu déroulant, les boutons et les barres d'outils ; macOS ne fournit pas de vue d'activité — "Even though macOS doesn’t provide an activity view" — activity-views › Discussion
- Les actions rapides de l'écran d'accueil (Home Screen quick actions) et les ornements (ornaments) n'existent pas sur Mac — "Not supported in macOS, tvOS, visionOS, or watchOS." — home-screen-quick-actions › Platform considerations
- La barre des menus est l'inventaire des commandes d'une app, dans un ordre fixe : nom de l'app, File, Edit, Format, View, menus propres à l'app, Window, Help — "When present in the menu bar, the following menus appear in the order listed below." — the-menu-bar › Anatomy
- Une commande reste visible dans un menu, désactivée plutôt que masquée ; seul le menu contextuel fait l'inverse — "If a menu bar item isn’t actionable, disable the action instead of hiding it from the menu." — the-menu-bar › Best practices ; "Hide unavailable menu items, don’t dim them." — context-menus › Best practices
- Les variantes d'une commande (Minimize All, Zoom All…) sont des éléments dynamiques révélés par une seule touche modificatrice — "Require only a single modifier key to reveal a dynamic menu item." — the-menu-bar › Dynamic menu items ; jamais le seul moyen d'agir — "Avoid making a dynamic menu item the only way to accomplish a task." — the-menu-bar › Dynamic menu items
- Les menus obéissent à des seuils : sous-menus d'un seul niveau, scission au-delà d'environ cinq éléments — "if a submenu contains more than about five items, consider creating a new menu" — menus › Submenus ; ellipse quand une saisie supplémentaire est requise — "Append an ellipsis to a menu item’s label when the action requires more information before it can complete." — menus › Labels
- Les boutons se distinguent par le style et le rôle (primaire, annuler, destructif), non par la taille — "Use style — not size — to visually distinguish the preferred choice among multiple options." — buttons › Style ; une barre d'outils compte trois emplacements — "You can position toolbar items in three locations" — toolbars › Item groupings ; et trois groupes au plus — "In general, aim for a maximum of three." — toolbars › Item groupings
- Toute commande de barre d'outils est aussi une commande de la barre des menus, puisque la barre d'outils peut être masquée ou personnalisée — "Make every toolbar item available as a command in the menu bar." — toolbars › Platform considerations › macOS
- Cotes : la famille ne fournit qu'une vingtaine de valeurs (hauteur de barre, région de touche de bouton, seuils, longueurs de titres) ; rayons, polices, hauteurs d'éléments et durées ne sont pas chiffrés — "The menu bar’s height is 24 pt." — the-menu-bar › Platform considerations › macOS › Menu bar extras ; "a button needs a hit region of at least 44x44 pt" — buttons › Best practices

### Menus and actions (`menus-and-actions`)

**Rôle**
Page d'index de la famille : une phrase de définition par composant, sans recommandation propre.

**Anatomie et cotes**
- Non chiffré dans la HIG : cette page ne contient que des définitions et des renvois.

**Règles**
- Barre des menus — "On a Mac or an iPad, the menu bar at the top of the screen displays the top-level menus in your app or game." — menus-and-actions › Discussion ; menu — "A menu reveals its options when people interact with it, making it a space-efficient way to present commands in your app or game." — menus-and-actions › Discussion
- Menu du Dock — "On a Mac, people can secondary click an app’s or game’s icon in the Dock to reveal a Dock menu, which presents both system-provided and custom items." — menus-and-actions › Discussion ; menu d'édition — "An edit menu lets people make changes to selected content in the current view, in addition to offering related commands like Copy, Select, Translate, and Look Up." — menus-and-actions › Discussion
- Menu contextuel — "A context menu provides access to functionality that’s directly related to an item, without cluttering the interface." — menus-and-actions › Discussion ; bouton — "A button initiates an instantaneous action." — menus-and-actions › Discussion
- Bouton localisé — "A pop-up button displays a menu of mutually exclusive options." — menus-and-actions › Discussion ; bouton à menu déroulant — "A pull-down button displays a menu of items or actions that directly relate to the button’s purpose." — menus-and-actions › Discussion
- Barre d'outils — "A toolbar provides convenient access to frequently used commands, controls, navigation, and search." — menus-and-actions › Discussion ; vue d'activité — "presents a range of tasks that people can perform in the current context" — menus-and-actions › Discussion
- Actions rapides de l'écran d'accueil — "Home Screen quick actions give people a way to perform app-specific actions from the Home Screen." — menus-and-actions › Discussion ; ornement — "In visionOS, an ornament presents controls and information related to a window, without crowding or obscuring the window’s contents." — menus-and-actions › Discussion

**Spécifique macOS**
- L'index n'attribue explicitement au Mac que deux composants : la barre des menus (partagée avec l'iPad) et le menu du Dock ; l'ornement est réservé à visionOS — "In visionOS, an ornament presents controls and information related to a window" — menus-and-actions › Discussion
- Les composants propres au Mac (bouton poussoir, bouton carré, bouton d'aide, bouton image, éléments de barre de menus) ne sont définis que dans les pages détaillées.

**À transposer dans un shell Linux**
- Modèle commun : traiter la barre, le dock, les menus contextuels et le lanceur comme des instances d'un même composant « menu », puisque les mêmes règles s'appliquent à tous les types de menus — "The guidance for labeling and organizing menu items applies to all types of menus in all experiences." — menus › Discussion
- Surfaces concernées chez hyprdesk : barre (menus système et d'app, applets), dock (menus du Dock), menus contextuels du bureau et des fenêtres, réglages (boutons et menus déroulants).
- Sans objet pour un bureau Linux : actions rapides d'écran d'accueil, ornements, vue d'activité.

### The menu bar (`the-menu-bar`)

**Rôle**
Sur Mac comme sur iPad, la barre des menus (menu bar) au sommet de l'écran affiche les menus de premier niveau de l'app, et les utilisateurs de Mac s'y fient pour découvrir ce que fait une app et trouver ses commandes.

**Anatomie et cotes**
- Hauteur de la barre des menus : 24 pt — "The menu bar’s height is 24 pt." — the-menu-bar › Platform considerations › macOS › Menu bar extras
- Ordre des menus, lorsqu'ils sont présents : NomApp, File, Edit, Format, View, menus propres à l'app, Window, Help — "When present in the menu bar, the following menus appear in the order listed below." — the-menu-bar › Anatomy
- Le titre du premier menu est une version abrégée du nom de l'app — "you supply a short version of your app’s name for this menu’s title" — the-menu-bar › Anatomy
- Extrémité amont (leading side) : menu Apple ; extrémité aval (trailing side) : éléments de barre de menus (menu bar extras) — "the macOS menu bar includes the Apple menu on the leading side and menu bar extras on the trailing side" — the-menu-bar › Anatomy
- Nom abrégé de l'app : 16 caractères au plus, sans numéro de version — "Prefer a short name of 16 characters or fewer. Don’t include a version number." — the-menu-bar › App menu ; le même dans About, Hide et Quit — "Use the same short app name you supply for the About item." — the-menu-bar › App menu
- Largeur d'un menu : fixée par le système d'après l'élément le plus large, éléments dynamiques compris — "macOS automatically sets the width of a menu to hold the widest item, including dynamic menu items." — the-menu-bar › Dynamic menu items
- Place contrainte : espacement des titres réduit, puis troncature — "the system may decrease the space between the titles, truncating them if necessary" — the-menu-bar › Platform considerations › macOS
- Icônes des extras : formes définies en noir et en transparent, recolorées par le système — "Both interface icons and symbols use black and clear colors to define their shapes" — the-menu-bar › Platform considerations › macOS › Menu bar extras
- Menu Aide : à l'extrémité aval, avec un champ de recherche en tête quand la documentation est au format Help Book — "macOS automatically includes a search field at the top of the Help menu" — the-menu-bar › Help menu
- Non chiffré dans la HIG : hauteur et largeur des éléments de menu, taille de police, rayon des coins, espacements internes, largeur de la colonne des raccourcis, couleurs de surbrillance, durées d'ouverture et de fermeture.

**Règles**

*Ordre et cohérence*
- Reprendre les menus définis par le système et leur ordre — "Support the default system-defined menus and their ordering." — the-menu-bar › Best practices ; le système implémente souvent la fonction des éléments standard — "In many cases, the system implements the functionality of standard menu items so you don’t have to." — the-menu-bar › Best practices
- Afficher toujours le même ensemble d'éléments — "Always show the same set of menu items." — the-menu-bar › Best practices ; une commande sans effet dans le contexte courant est désactivée, jamais retirée — "If a menu bar item isn’t actionable, disable the action instead of hiding it from the menu." — the-menu-bar › Best practices
- Représenter les actions par des icônes familières, identiques à celles du système — "Use the same icons as the system to represent actions such as Copy, Share, and Delete, wherever they appear." — the-menu-bar › Best practices
- Prendre en charge les raccourcis des éléments standard — "Support the keyboard shortcuts defined for the standard menu items you include." — the-menu-bar › Best practices ; n'en définir de personnalisés qu'en cas de nécessité — "Define custom keyboard shortcuts only when necessary." — the-menu-bar › Best practices
- Titres de menu courts, d'un mot de préférence — "Prefer short, one-word menu titles." — the-menu-bar › Best practices ; au-delà, majuscules de titre (title-style capitalization) — "If you need to use more than one word in the menu title, use title-style capitalization." — the-menu-bar › Best practices

*Menu de l'application (app menu)*
- Portée : l'app entière, non une tâche, un document ou une fenêtre — "The app menu lists items that apply to your app or game as a whole, rather than to a specific task, document, or window." — the-menu-bar › App menu
- Le nom de l'app est affiché en gras pour identifier l'app active — "To help people quickly identify the active app, the menu bar displays your app name in bold." — the-menu-bar › App menu
- Contenu et ordre usuels : About NomApp, Settings…, éléments propres facultatifs, Services, Hide NomApp, Hide Others, Show All, Quit NomApp — "The app menu typically contains the following menu items listed in the following order." — the-menu-bar › App menu
- About NomApp : fenêtre « À propos » avec copyright et version — "Displays the About window for your app, which includes copyright and version information." — the-menu-bar › App menu ; toujours en premier — "Display the About menu item first." — the-menu-bar › App menu ; suivi d'un séparateur, seul dans son groupe — "Include a separator after the About menu item so that it appears by itself in a group." — the-menu-bar › App menu
- Settings… : uniquement les réglages de niveau app — "Use only for app-level settings." — the-menu-bar › App menu ; les réglages propres à un document vont dans le menu File — "If you also offer document-specific settings, put them in the File menu." — the-menu-bar › App menu
- Éléments propres de configuration : après Settings, dans le même groupe — "List custom app-configuration items after the Settings item and within the same group." — the-menu-bar › App menu
- Services : sous-menu de services du système et des autres apps qui s'appliquent au contexte courant — "Displays a submenu of services from the system and other apps that apply to the current context." — the-menu-bar › App menu
- Hide NomApp : masque l'app et toutes ses fenêtres, puis active l'app la plus récemment utilisée — "Hides your app and all of its windows, and then activates the most recently used app." — the-menu-bar › App menu ; Hide Others — "Hides all other open apps and their windows." — the-menu-bar › App menu ; Show All — "Shows all other open apps and their windows behind your app’s windows." — the-menu-bar › App menu
- Quit NomApp quitte l'app, la touche Option transformant l'intitulé en « Quit and Keep Windows » — "Quits your app." — the-menu-bar › App menu

*Menu Fichier (File menu)*
- Contenu : gestion des fichiers ou documents — "The File menu contains commands that help people manage the files or documents an app supports." — the-menu-bar › File menu ; sans fichier à gérer, le menu est renommé ou supprimé — "If your app doesn’t handle any types of files, you can rename or eliminate this menu." — the-menu-bar › File menu
- Contenu et ordre usuels : New Élément, Open, Open Recent, Close, Close Tab, Close File, Save, Save All, Duplicate, Rename…, Move To…, Export As…, Revert To, Page Setup…, Print… — "The File menu typically contains the following menu items listed in the following order." — the-menu-bar › File menu
- New Élément : le terme nomme le type d'objet créé — "use a term that names the type of item your app creates" — the-menu-bar › File menu
- Open : ellipse quand la sélection se fait dans une interface séparée — "If people need to select an item in a separate interface, an ellipsis follows the command to indicate that more input is required." — the-menu-bar › File menu
- Open Recent : des noms de fichiers reconnaissables, jamais des chemins — "List document and filenames that people recognize in the submenu; don’t display file paths." — the-menu-bar › File menu ; du plus récemment ouvert au plus ancien — "List the documents in the order people last opened them, with the most recently opened document first." — the-menu-bar › File menu
- Close : Option donne Close All — "Pressing Option changes Close to Close All." — the-menu-bar › File menu ; dans une fenêtre à onglets, Close Tab remplace Close — "For a tab-based window, Close Tab replaces Close." — the-menu-bar › File menu ; Close Tab : Option donne Close Other Tabs — "Pressing Option changes Close Tab to Close Other Tabs." — the-menu-bar › File menu ; Close File ferme le fichier et toutes ses fenêtres associées — "Consider supporting this menu item if your app can open multiple views of the same file." — the-menu-bar › File menu
- Save : enregistrement automatique périodique — "Automatically save changes periodically as people work so they don’t need to keep choosing File > Save." — the-menu-bar › File menu ; pour un nouveau document, demande d'un nom et d'un emplacement — "For a new document, prompt people for a name and location." — the-menu-bar › File menu
- Duplicate, à préférer à Save As, Export, Copy To et Save To — "Prefer Duplicate to menu items like Save As, Export, Copy To, and Save To" — the-menu-bar › File menu ; la touche Option le transforme en Save As — "Pressing Option changes Duplicate to Save As." — the-menu-bar › File menu
- Export As… : réservé aux formats que l'app ne traite pas habituellement — "Reserve the Export As item for when you need to let people export content in a format your app doesn’t typically handle." — the-menu-bar › File menu
- Page Setup… : paramètres d'impression propres au document — "Include the Page Setup item if you need to support printing parameters that apply to a specific document." — the-menu-bar › File menu ; Print… : panneau d'impression standard — "Opens the standard Print panel, which lets people print to a printer, send a fax, or save as a PDF." — the-menu-bar › File menu

*Menu Édition (Edit menu)*
- Modifications du contenu du document ou du conteneur de texte courant, et commandes du presse-papiers — "The Edit menu lets people make changes to content in the current document or text container, and provides commands for interacting with the Clipboard." — the-menu-bar › Edit menu ; utile même hors des apps à documents — "Because many editing commands apply to any editable content, the Edit menu is useful even in apps that aren’t document-based." — the-menu-bar › Edit menu
- Contenu et ordre usuels : Undo, Redo, Cut, Copy, Paste, Paste and Match Style, Delete, Select All, Find, Spelling and Grammar, Substitutions, Transformations, Speech, Start Dictation, Emoji & Symbols — "The Edit menu typically contains the following top-level menu items, listed in the following order." — the-menu-bar › Edit menu
- Find : à garder dans Edit seulement si la recherche porte sur le contenu courant — "Determine whether Find menu items belong in the Edit menu." — the-menu-bar › Edit menu ; pour chercher des fichiers ou des objets, plutôt dans File — "Find menu items might be more appropriate in the File menu." — the-menu-bar › Edit menu ; son sous-menu standard : Find, Find and Replace, Find Next, Find Previous, Use Selection for Find, Jump to Selection — "Standard submenus include: Find, Find and Replace, Find Next, Find Previous, Use Selection for Find, and Jump to Selection." — the-menu-bar › Edit menu
- Undo et Redo précisent leur cible — "Clarify the target of the undo." — the-menu-bar › Edit menu ; "Clarify the target of the redo." — the-menu-bar › Edit menu ; par exemple "such as Undo Paste and Match Style" — the-menu-bar › Edit menu
- Cut remplace le contenu du presse-papiers — "Removes the selected data and stores it on the Clipboard, replacing the previous contents of the Clipboard." — the-menu-bar › Edit menu ; Copy en place une copie des données sélectionnées — "Duplicates the selected data and stores it on the Clipboard." — the-menu-bar › Edit menu ; Paste laisse le presse-papiers intact — "The Clipboard contents remain unchanged, permitting people to choose Paste multiple times." — the-menu-bar › Edit menu
- Delete plutôt que Erase ou Clear — "Provide a Delete menu item instead of an Erase or Clear menu item." — the-menu-bar › Edit menu ; cohérence avec la touche Suppr — "Choosing Delete is the equivalent of pressing the Delete key, so it’s important for the naming to be consistent." — the-menu-bar › Edit menu
- Start Dictation et Emoji & Symbols ajoutés par le système en bas du menu — "The system automatically adds the Start Dictation menu item at the bottom of the Edit menu." — the-menu-bar › Edit menu ; "The system automatically adds the Emoji & Symbols menu item at the bottom of the Edit menu." — the-menu-bar › Edit menu

*Menu Format (Format menu)*
- Attributs de mise en forme du texte courant, à omettre sans texte formaté — "You can exclude this menu if your app doesn’t support formatted text editing." — the-menu-bar › Format menu
- Contenu et ordre usuels : Font (Show Fonts, Bold, Italic, Underline, Bigger, Smaller, Show Colors, Copy Style, Paste Style), puis Text (Align Left, Align Center, Justify, Align Right, Writing Direction, Show Ruler, Copy Ruler, Paste Ruler) — "The Format menu typically contains the following top-level menu items, listed in the following order." — the-menu-bar › Format menu

*Menu Présentation (View menu)*
- Apparence de toutes les fenêtres de l'app, quel que soit leur type — "The View menu lets people customize the appearance of all an app’s windows, regardless of type." — the-menu-bar › View menu
- Ni navigation entre fenêtres ni gestion des fenêtres, qui relèvent du menu Window — "The View menu doesn’t include items for navigating between or managing specific windows" — the-menu-bar › View menu
- À fournir même partiel — "Provide a View menu even if your app supports only a subset of the standard view functions." — the-menu-bar › View menu
- Intitulé d'affichage ou de masquage reflétant l'état courant — "Ensure that each show/hide item title reflects the current state of the corresponding view." — the-menu-bar › View menu ; "when the toolbar is hidden, provide a Show Toolbar menu item; when the toolbar is visible, provide a Hide Toolbar menu item" — the-menu-bar › View menu
- Contenu et ordre usuels : Show/Hide Tab Bar, Show All Tabs/Exit Tab Overview, Show/Hide Toolbar, Customize Toolbar, Show/Hide Sidebar, Enter/Exit Full Screen — "The View menu typically contains the following top-level menu items, listed in the following order." — the-menu-bar › View menu

*Menus propres à l'app (app-specific menus)*
- Placés entre View et Window — "Your app’s custom menus appear in the menu bar between the View menu and the Window menu." — the-menu-bar › App-specific menus
- Toutes les commandes propres figurent dans la barre des menus, même disponibles ailleurs — "Even when commands are available elsewhere in your app, it’s important to list them in the menu bar." — the-menu-bar › App-specific menus ; bénéfices : raccourcis assignables et accès complet au clavier (Full Keyboard Access) — "lets you assign keyboard shortcuts to them, and makes them more accessible to people using Full Keyboard Access" — the-menu-bar › App-specific menus
- Même les commandes rares ou avancées y restent — "Excluding commands from the menu bar — even infrequently used or advanced commands — risks making them difficult for everyone to find." — the-menu-bar › App-specific menus
- Refléter la hiérarchie de l'app — "As much as possible, reflect your app’s hierarchy in app-specific menus." — the-menu-bar › App-specific menus ; du plus général ou courant au moins — "Aim to list app-specific menus in order from most to least general or commonly used." — the-menu-bar › App-specific menus

*Menu Fenêtre (Window menu)*
- Navigation, organisation et gestion des fenêtres de l'app ; ni l'apparence des fenêtres (menu View) ni leur fermeture (Close, menu File) — "The Window menu doesn’t help people customize the appearance of windows or close them." — the-menu-bar › Window menu
- À fournir même à une seule fenêtre — "Provide a Window menu even if your app has only one window." — the-menu-bar › Window menu ; avec Minimize et Zoom pour l'accès complet au clavier — "Include the Minimize and Zoom menu items so people using Full Keyboard Access can use the keyboard to invoke these functions." — the-menu-bar › Window menu
- Contenu et ordre usuels : Minimize, Zoom, Show Previous Tab, Show Next Tab, Move Tab to New Window, Merge All Windows, Enter/Exit Full Screen, Bring All to Front, nom de chaque fenêtre ouverte — "The Window menu typically contains the following top-level menu items, listed in the following order." — the-menu-bar › Window menu
- Minimize : réduction de la fenêtre active dans le Dock ; Option donne Minimize All — "Minimizes the active window to the Dock. Pressing the Option key changes this item to Minimize All." — the-menu-bar › Window menu
- Zoom : bascule entre une taille prédéfinie adaptée au contenu et la taille choisie — "Toggles between a predefined size appropriate to the window’s content and the window size people set." — the-menu-bar › Window menu ; ne sert pas au plein écran — "Avoid using Zoom to enter or exit full-screen mode." — the-menu-bar › Window menu
- Enter/Exit Full Screen ne figure dans Window qu'en l'absence de menu View — "Include this item in the Window menu only if your app doesn’t have a View menu." — the-menu-bar › Window menu
- Bring All to Front : toutes les fenêtres au premier plan, en conservant position, taille et ordre — "Brings all an app’s open windows to the front, maintaining their onscreen location, size, and layering order." — the-menu-bar › Window menu ; même effet qu'un clic sur l'icône du Dock — "Clicking the app icon in the Dock has the same effect." — the-menu-bar › Window menu ; Option donne Arrange in Front, en mosaïque ordonnée — "brings an app’s windows to the front in a neatly tiled arrangement" — the-menu-bar › Window menu
- Fenêtres ouvertes listées par ordre alphabétique, sans les panneaux ni les vues modales — "List the currently open windows in alphabetical order for easy scanning. Avoid listing panels or other modal views." — the-menu-bar › Window menu

*Menu Aide (Help menu)*
- Dernier menu de la barre — "The Help menu — located at the trailing end of the menu bar — provides access to an app’s help documentation." — the-menu-bar › Help menu ; champ de recherche en tête avec le format Help Book — "When you use the Help Book format for this documentation, macOS automatically includes a search field at the top of the Help menu." — the-menu-bar › Help menu
- Contenu : Send NomApp Feedback to Apple, NomApp Help, éléments supplémentaires (inscription, notes de version) — "Use a separator between your primary help documentation and additional items, which might include registration information or release notes." — the-menu-bar › Help menu
- Peu d'éléments — "Keep the total the number of items you list in the Help menu small to avoid overwhelming people with too many choices when they need help." — the-menu-bar › Help menu

*Éléments dynamiques (dynamic menu items)*
- Définition : élément dont le comportement change quand on le choisit avec une touche modificatrice — "which is a menu item that changes its behavior when people choose it while pressing a modifier key (Control, Option, Shift, or Command)" — the-menu-bar › Dynamic menu items
- Cachés par défaut, donc raccourcis vers des actions avancées accessibles autrement — "Dynamic menu items are hidden by default, so they’re best suited to offer shortcuts to advanced actions that people can accomplish in other ways." — the-menu-bar › Dynamic menu items ; jamais le seul moyen d'accomplir une tâche — "Avoid making a dynamic menu item the only way to accomplish a task." — the-menu-bar › Dynamic menu items
- Surtout dans les menus de la barre des menus — "Use dynamic menu items primarily in menu bar menus." — the-menu-bar › Dynamic menu items ; à éviter dans les menus contextuels et du Dock — "Adding a dynamic menu item to contextual or Dock menus can make the item even harder for people to discover." — the-menu-bar › Dynamic menu items
- Un seul modificateur pour révéler la variante — "Require only a single modifier key to reveal a dynamic menu item." — the-menu-bar › Dynamic menu items

**Spécifique macOS**
- Plateformes : la barre des menus n'existe pas sur iOS, tvOS, visionOS ni watchOS — "Not supported in iOS, tvOS, visionOS, or watchOS." — the-menu-bar › Platform considerations
- Menu Apple, dont la HIG ne détaille pas le contenu dans cette page : toujours en premier, côté amont — "The menu bar in macOS includes the Apple menu, which is always the first item on the leading side of the menu bar." — the-menu-bar › Platform considerations › macOS ; éléments système toujours disponibles, ni modifiables ni retirables — "The Apple menu includes system-defined menu items that are always available, and you can’t modify or remove it." — the-menu-bar › Platform considerations › macOS
- Éléments de barre de menus affichés côté aval si la place le permet — "Space permitting, the system can also display menu bar extras in the trailing end of the menu bar." — the-menu-bar › Platform considerations › macOS ; en cas de place contrainte, priorité aux menus et aux extras essentiels — "the system prioritizes the display of menus and essential menu bar extras" — the-menu-bar › Platform considerations › macOS
- Plein écran : la barre se masque jusqu'à ce que le pointeur atteigne le haut de l'écran — "When people enter full-screen mode, the menu bar typically hides until they reveal it by moving the pointer to the top of the screen." — the-menu-bar › Platform considerations › macOS
- Éléments marqués « macOS only » dans les tableaux du menu de l'app : Services, Hide NomApp, Hide Others, Show All.
- Élément de barre de menus : fonctionnalité propre à une app, sous forme d'icône présente tant que l'app tourne, même non active — "A menu bar extra exposes app-specific functionality using an icon that appears in the menu bar when your app is running, even when it’s not the frontmost app." — the-menu-bar › Platform considerations › macOS › Menu bar extras ; du côté opposé aux menus de l'app — "Menu bar extras are on the opposite side of the menu bar from your app’s menus." — the-menu-bar › Platform considerations › macOS › Menu bar extras
- Masqués par le système pour laisser la place aux menus de l'app — "When necessary, the system hides menu bar extras to make room for app menus." — the-menu-bar › Platform considerations › macOS › Menu bar extras ; aucune dépendance à leur présence — "Avoid relying on the presence of menu bar extras." — the-menu-bar › Platform considerations › macOS › Menu bar extras
- Représentation : une icône ou un SF Symbol — "Consider using a symbol to represent your menu bar extra." — the-menu-bar › Platform considerations › macOS › Menu bar extras ; formes recolorées par le système selon le thème et l'état sélectionné — "so it looks good on both dark and light menu bars, and when your menu bar extra is selected" — the-menu-bar › Platform considerations › macOS › Menu bar extras
- Clic : ouvrir un menu, pas un popover, sauf fonctionnalité trop complexe pour un menu — "Display a menu — not a popover — when people click your menu bar extra." — the-menu-bar › Platform considerations › macOS › Menu bar extras
- Présence décidée par l'utilisateur et non par l'app — "Let people — not your app — decide whether to put your menu bar extra in the menu bar." — the-menu-bar › Platform considerations › macOS › Menu bar extras ; en règle générale par un réglage de l'app — "Typically, people add a menu bar extra to the menu bar by changing a setting in an app’s settings window." — the-menu-bar › Platform considerations › macOS › Menu bar extras ; éventuellement proposée à l'installation — "consider giving people the option of doing so during setup" — the-menu-bar › Platform considerations › macOS › Menu bar extras
- Autre voie d'accès, toujours disponible quand l'app tourne : le menu du Dock (faute « aways » dans la source) — "People can hide or choose not to use your menu bar extra, but a Dock menu is aways available when your app is running." — the-menu-bar › Platform considerations › macOS › Menu bar extras
- Ouverture, survol, fermeture : non spécifié dans cet export — ni l'ouverture au clic simple ou par appui maintenu, ni le passage d'un titre à l'autre par survol, ni les délais, ni la fermeture (clic hors menu, touche Échap). Sont seulement décrits le clic sur un extra, la révélation en plein écran et la largeur automatique (voir ci-dessus et Anatomie et cotes).
- Raccourcis : reprendre ceux que les gens connaissent déjà — "People expect to use the keyboard shortcuts they already know for standard menu items, like Copy, Cut, Paste, Save, and Print." — the-menu-bar › Best practices ; la liste des raccourcis standard et les glyphes des modificateurs ne figurent pas dans cette famille de pages (la HIG renvoie à sa page `keyboards`, hors de ce chapitre), les modificateurs y étant seulement nommés : Control, Option, Shift, Command.
- États des éléments recensés : disponible ; indisponible, estompé et sans réponse — "An unavailable menu item often appears dimmed and doesn’t respond to interactions." — menus › Labels ; coché (voir `menus`) ; à intitulé changeant (Show/Hide) ; dynamique (révélé par modificateur) ; titre d'app en gras (app active) ; extra sélectionné (recoloré par le système).

**À transposer dans un shell Linux**
- Barre (ou équivalent `hyprbars`) : ordre fixe, avec en tête côté amont un menu « système » jouant le rôle du menu Apple (toujours premier, jamais modifié), puis les menus de l'app ou de l'espace actif, puis la zone aval des applets.
- Composant « menu » unique pour la barre, le dock, le clic droit et le lanceur : libellés verbaux, ellipse, états, séparateurs, variantes dynamiques à un seul modificateur.
- Barre : aucune commande retirée selon le contexte, seulement désactivée ; le shell expose la totalité de ses commandes dans au moins un menu, pas seulement dans un raccourci clavier ou un bouton.
- Applets (réseau, volume, batterie, horloge) : icône monochrome, menu au clic plutôt qu'un popover sauf besoin de complexité, présence commandée par un réglage utilisateur, aucune fonction critique disponible uniquement par un applet.
- Dock : clic secondaire sur une icône ouvre le menu du Dock, avec la liste des fenêtres de l'app ; il est le doublon toujours disponible des applets.
- Réglages : entrée « Réglages… » réservée aux réglages globaux du shell ; en plein écran, la barre se masque et réapparaît quand le pointeur atteint le bord supérieur.
- Fenêtres : un menu Fenêtre distinct du menu Présentation (tout au premier plan, fenêtres listées par ordre alphabétique, minimiser d'un côté ; afficher ou masquer barre, dock et panneaux de l'autre).
- Lanceur : un champ de recherche en tête du menu d'aide, dans l'esprit du menu Aide au format Help Book.

### Menus (`menus`)

**Rôle**
Un menu (menu) ne révèle ses options qu'au moment de l'interaction : c'est un moyen économe en espace de présenter des commandes.

**Anatomie et cotes**
- Élément de menu (menu item) : un intitulé, un symbole facultatif et, dans une app, la commande clavier associée — "A menu item’s label describes what it does and may include a symbol if it helps to clarify meaning." — menus › Labels ; "In an app, a menu item can also display the associated keyboard command, if there is one" — menus › Labels
- Séparateur (separator) : trait horizontal ou court intervalle dans le fond, selon la plateforme et le type de menu — "appears between groups of items as a horizontal line or a short gap in the menu’s background appearance" — menus › Organization
- Sous-menu (submenu) : signalé par un symbole, un chevron par exemple, après l'intitulé — "A menu item indicates the presence of a submenu by displaying a symbol — like a chevron — after its label." — menus › Submenus
- Seuils : sous-menus limités à un niveau — "it’s generally best to restrict them to a single level" — menus › Submenus ; plus d'environ cinq éléments dans un sous-menu invite à créer un menu — "if a submenu contains more than about five items, consider creating a new menu" — menus › Submenus
- Ellipse : le caractère « … » — "The ellipsis character (…) signals that people need to input information or make additional choices, typically within another view." — menus › Labels
- Élément indisponible : estompé (dimmed), sans réponse — "An unavailable menu item often appears dimmed and doesn’t respond to interactions." — menus › Labels
- Non chiffré dans la HIG : hauteur des éléments, taille de police, largeur de menu, rayon des coins, espacements, durées, couleur d'accentuation.

**Règles**

*Libellés (Labels)*
- Un intitulé clair et concis par élément — "For each menu item, write a label that clearly and succinctly describes it." — menus › Labels ; une action se nomme par un verbe ou un syntagme verbal — "label a menu item that initiates an action using a verb or verb phrase that describes the action, such as View, Close, or Select" — menus › Labels
- Majuscules de titre (title-style capitalization) — "To be consistent with platform experiences, use title-style capitalization." — menus › Labels ; règle anglaise : "which capitalizes every word except articles, coordinating conjunctions, and short prepositions, and capitalizes the last word in the label, regardless of the part of speech" — menus › Labels
- Retirer les articles des intitulés — "In English, articles always lengthen labels, but rarely enhance understanding." — menus › Labels
- Signaler l'indisponibilité sans retirer l'élément — "Show people when a menu item is unavailable." — menus › Labels ; un menu dont tous les éléments sont indisponibles reste ouvrable — "If all of a menu’s items are unavailable, the menu itself needs to remain available so people can open it and learn about the commands it contains." — menus › Labels
- Ellipse quand l'action exige plus d'informations — "Append an ellipsis to a menu item’s label when the action requires more information before it can complete." — menus › Labels

*Icônes (Icons)*
- Actions courantes représentées comme dans le système — "Represent common actions consistently." — menus › Icons ; "The system provides standard icons to represent common actions like Share, Print, and Search." — menus › Icons
- Icônes parcimonieuses et ciblées — "Use menu item icons sparingly and with purpose." — menus › Icons ; sans icône claire, pas d'icône — "Don’t display an icon if you can’t find one that clearly represents the menu item." — menus › Icons
- Tout ou rien dans un groupe — "provide icons for all menu items in a group, or none of them" — menus › Icons

*Organisation (Organization)*
- Éléments importants ou fréquents en premier — "Prefer listing important or frequently used menu items first." — menus › Organization
- Regrouper ce qui se rattache logiquement et séparer les groupes — "Consider grouping logically related items." — menus › Organization ; "To help people visually distinguish such groups, use a separator." — menus › Organization
- Commandes liées dans un même groupe, même d'importance inégale — "Prefer keeping all logically related commands in the same group, even if the commands don’t all have the same importance." — menus › Organization
- Menu trop long : le scinder — "If a menu is too long, consider dividing it into separate menus." — menus › Organization ; exception, un contenu défini ou généré par l'utilisateur (Historique, marque-pages), où le défilement est admis — "People expect such a menu to accommodate all the items they add to it, so a long menu is fine, and scrolling is acceptable." — menus › Organization

*Sous-menus (Submenus)*
- Avec parcimonie, quand un terme se répète dans plus de deux éléments d'un même groupe — "You might consider creating a submenu when a term appears in more than two menu items in the same group." — menus › Submenus
- Reprendre le terme répété dans l'intitulé du parent — "in the menu item’s label to help people predict the contents of the submenu" — menus › Submenus
- Sous-menu ouvrable même si ses éléments sont indisponibles — "Make sure a submenu remains available even when its nested menu items are unavailable." — menus › Submenus
- Un sous-menu plutôt qu'une indentation — "Prefer using a submenu to indenting menu items." — menus › Submenus

*Éléments à bascule (Toggled items)*
- Un seul élément à intitulé changeant plutôt que deux — "Consider using a changeable label that describes an item’s current state." — menus › Toggled items
- Un verbe quand l'état n'est pas clair — "Include a verb if a changeable label isn’t clear enough." — menus › Toggled items ; ou les deux éléments visibles à la fois — "If necessary, display both menu items instead of one toggled item." — menus › Toggled items
- Coche pour l'attribut en vigueur — "Consider using a checkmark to show that an attribute is currently in effect." — menus › Toggled items
- Élément de remise à zéro de plusieurs attributs — "Consider offering a menu item that makes it easy to remove multiple toggled attributes." — menus › Toggled items

**Spécifique macOS**
- Aucune considération supplémentaire pour macOS — "No additional considerations for macOS, tvOS, or watchOS." — menus › Platform considerations
- Sur macOS et iPadOS, les menus de la barre des menus contiennent toutes les commandes exécutables — "menus contain all the commands people can perform in the app or game" — menus › Discussion
- Les règles de libellé et d'organisation valent pour tous les types de menus — "The guidance for labeling and organizing menu items applies to all types of menus in all experiences." — menus › Discussion ; un menu contextuel, lui, donne un petit nombre d'actions fréquentes — "lets people access a small number of frequently used actions relevant to their current view or task" — menus › Discussion
- Éléments réduits à une icône, sans intitulé : possible seulement sur iOS, iPadOS et visionOS — "an iOS, iPadOS, or visionOS app can display a few unlabeled menu items that use only symbols or icons to identify them" — menus › Discussion

**À transposer dans un shell Linux**
- Composant « menu » unique pour la barre, le dock, le clic droit et le lanceur : intitulé à verbe, raccourci affiché à droite, coche, état estompé, séparateurs, ellipse.
- Barre des menus : un élément sans effet dans le contexte courant est estompé, jamais retiré.
- Menus contextuels du bureau, du dock et des fenêtres : mêmes intitulés et mêmes icônes que dans la barre.
- Réglages : les options exclusives (mode d'affichage, disposition, thème) sont des éléments à coche, avec une entrée de réinitialisation.
- Sous-menus : un seul niveau ; au-delà du seuil d'Anatomie et cotes (tri, disposition des espaces, taille des icônes), créer un menu distinct.

### Dock menus (`dock-menus`)

**Rôle**
Sur Mac, un clic secondaire sur l'icône d'une app ou d'un jeu dans le Dock révèle un menu du Dock (Dock menu), qui présente des éléments fournis par le système et des éléments personnalisés.

**Anatomie et cotes**
- Composition : éléments système et éléments personnalisés, les premiers variant selon que l'app est ouverte ou non — "The system-provided Dock menu items can vary depending on whether the app is open." — dock-menus › Discussion
- Exemple cité, Safari — "actions like viewing a current window or creating a new window" — dock-menus › Discussion
- Non chiffré dans la HIG : dimensions, icônes, nombre d'éléments, position d'ouverture, durées.

**Règles**
- Libellés concis et organisation logique, comme pour tout menu — "As with all menus, you need to label Dock menu items succinctly and organize them logically." — dock-menus › Best practices
- Les éléments personnalisés existent aussi ailleurs — "Make custom Dock menu items available in other places, too." — dock-menus › Best practices ; dans les menus de la barre des menus ou dans l'interface — "offer the same commands elsewhere, like in your menu bar menus or within your interface" — dock-menus › Best practices
- Privilégier les éléments personnalisés à forte valeur — "Prefer high-value custom items for your Dock menu." — dock-menus › Best practices ; en premier lieu la liste des fenêtres ouvertes ou récentes — "a Dock menu can list all currently or recently open windows, making it a convenient way to jump to the window people want" — dock-menus › Best practices
- Quelques actions utiles quand l'app n'est pas au premier plan ou n'a aucune fenêtre — "Also consider listing a few of the actions that are most likely to be useful when your app isn’t frontmost or when there are no open windows." — dock-menus › Best practices

**Spécifique macOS**
- Plateformes : Mac uniquement — "Not supported in iOS, iPadOS, tvOS, visionOS, or watchOS." — dock-menus › Platform considerations
- Déclencheur : le clic secondaire sur l'icône du Dock — "people can secondary click an app’s or game’s icon in the Dock to reveal a Dock menu" — dock-menus › Dock menus ; la page de la barre des menus parle d'un Contrôle-clic — "that appears when people Control-click your app’s Dock icon" — the-menu-bar › Platform considerations › macOS › Menu bar extras
- Toujours disponible quand l'app tourne, à la différence d'un élément de barre de menus — "a Dock menu is aways available when your app is running" — the-menu-bar › Platform considerations › macOS › Menu bar extras
- Éléments dynamiques à éviter, encore moins découvrables dans un menu du Dock — "Adding a dynamic menu item to contextual or Dock menus can make the item even harder for people to discover." — the-menu-bar › Dynamic menu items
- Équivalent mobile : les actions rapides de l'écran d'accueil, obtenues par appui long sur une icône — "people can reveal a similar menu of system-provided and custom items" — dock-menus › Discussion

**À transposer dans un shell Linux**
- Dock : clic droit sur une icône ouvre le menu du Dock, avec la liste des fenêtres de l'app suivie de ses actions propres (nouvelle fenêtre, épingler ou détacher, quitter).
- Chaque action du menu du Dock également proposée dans la barre des menus ou dans un menu contextuel.
- Aperçu des espaces et sélecteur de fenêtres : réutiliser la liste des fenêtres du menu du Dock.
- Aucune variante à modificateur dans les menus du Dock.

### Edit menus (`edit-menus`)

**Rôle**
Un menu d'édition (edit menu) permet de modifier le contenu sélectionné dans la vue courante et réunit les commandes associées (Copier, Sélectionner, Traduire, Regarder).

**Anatomie et cotes**
- Contenus visés, au-delà du texte — "such as images, files, and objects like contact cards, charts, or map locations" — edit-menus › Discussion
- Non chiffré dans la HIG : taille, disposition, hauteur des éléments ; sur macOS, pas de présentation propre : un menu contextuel et le menu Edit de la barre des menus (voir Spécifique macOS).

**Règles**
- Préférer le menu d'édition du système — "Prefer the system-provided edit menu." — edit-menus › Best practices ; un menu personnalisé reprenant les mêmes commandes — "is redundant and likely to be confusing" — edit-menus › Best practices
- Révélation par les interactions déjà connues — "Let people reveal an edit menu using the system-defined interactions they already know." — edit-menus › Best practices ; sur Mac, le clic secondaire avec un trackpad ou un clavier — "or use a secondary click with an attached trackpad or keyboard" — edit-menus › Best practices
- Commandes pertinentes seulement, les autres retirées ou estompées — "Offer commands that are relevant in the current context, removing or dimming commands that don’t apply." — edit-menus › Best practices ; sans sélection — "if nothing is selected, avoid showing options that require a selection, such as Copy or Cut" — edit-menus › Best practices ; presse-papiers vide — "avoid showing a Paste option when there’s nothing to paste" — edit-menus › Best practices
- Commandes personnalisées placées près des commandes système correspondantes — "List custom commands near relevant system-provided ones." — edit-menus › Best practices ; sans excès — "Avoid overwhelming people with too many custom commands." — edit-menus › Best practices
- Texte non modifiable néanmoins sélectionnable et copiable — "In general, let people copy content text, but not control labels." — edit-menus › Best practices
- Annuler et rétablir — "Support undo and redo when possible." — edit-menus › Best practices ; un menu d'édition n'exige pas de confirmation — "an edit menu doesn’t require confirmation before performing its actions, so people can easily use undo and redo to recover a previous state" — edit-menus › Best practices
- Pas d'autre contrôle reprenant les mêmes fonctions — "In general, avoid implementing other controls that perform the same functions as edit menu items." — edit-menus › Best practices
- Delete et Cut distingués — "Differentiate different types of deletion commands when necessary." — edit-menus › Best practices ; "a Delete menu item behaves the same as pressing a Delete key, but a Cut menu item copies the selected content to the system pasteboard before deleting it" — edit-menus › Best practices
- Libellés courts pour les commandes personnalisées — "Use verbs or short verb phrases that succinctly describe the action your command performs." — edit-menus › Content

**Spécifique macOS**
- Deux voies d'accès sur Mac : un menu contextuel ouvert pendant une tâche d'édition, et le menu Edit de la barre des menus — "people can access editing commands in a context menu they can reveal while in an editing task" — edit-menus › Discussion
- L'ordre des éléments du menu Edit renvoie à la page de la barre des menus — "To learn about the order of items in a macOS app’s Edit menu, see" — edit-menus › Platform considerations › macOS
- Détection automatique du type de données sélectionnées, avec ajout d'une action liée (un itinéraire pour une adresse) : iOS, iPadOS et visionOS seulement — "In iOS, iPadOS, and visionOS, the system automatically detects the data type of a selected item" — edit-menus › Discussion
- Pas de menu d'édition en tvOS ni watchOS — "No additional considerations for visionOS. Not supported in tvOS or watchOS." — edit-menus › Platform considerations

**À transposer dans un shell Linux**
- Menus contextuels du bureau, du dock et des champs de saisie du lanceur : commandes Couper, Copier et Coller du presse-papiers du système, estompées hors sélection.
- Même composant de menu d'édition partout, plutôt qu'un menu propre à chaque widget.
- Aucun bouton « Copier » redondant à côté d'une zone où le menu d'édition suffit.
- Libellé « Supprimer » aligné sur la touche Suppr, et distinct de « Couper ».

### Context menus (`context-menus`)

**Rôle**
Un menu contextuel (context menu) donne accès aux fonctions directement liées à un élément, sans encombrer l'interface.

**Anatomie et cotes**
- Caché par défaut — "it’s hidden by default, so people might not know it’s there" — context-menus › Discussion ; révélé après le choix d'une vue ou une sélection — "people generally choose a view or select some content and then perform an action, using the input modes their current configuration supports" — context-menus › Discussion
- Révélation sur Mac : touche Control enfoncée pendant le clic — "Pressing the Control key while clicking a pointing device in macOS and iPadOS" — context-menus › Discussion ; clic secondaire sur un Magic Trackpad — "Using a secondary click on a Magic Trackpad in macOS or iPadOS" — context-menus › Discussion
- Titre rare — "A context menu seldom displays a title." — context-menus › Content
- Groupes : environ trois au plus — "In general, you don’t want more than about three groups in a context menu." — context-menus › Best practices
- Ouverture au-dessus ou au-dessous de la cible — "a context menu might open above or below it" — context-menus › Best practices
- Non chiffré dans la HIG : largeur, hauteur des éléments, rayon des coins, délai, largeur maximale.

**Règles**
- Pertinence avant tout — "Prioritize relevancy when choosing items to include in a context menu." — context-menus › Best practices ; "A context menu isn’t for providing advanced or rarely used items" — context-menus › Best practices
- Petit nombre d'éléments — "Aim for a small number of menu items." — context-menus › Best practices ; "A context menu that’s too long can be difficult to scan and scroll." — context-menus › Best practices
- Cohérence partout — "Support context menus consistently throughout your app." — context-menus › Best practices ; sinon — "people won’t know where they can use the feature and may think there’s a problem" — context-menus › Best practices
- Éléments aussi disponibles dans l'interface principale — "Always make context menu items available in the main interface, too." — context-menus › Best practices
- Sous-menus limités à un niveau — "If you need to use submenus to manage a menu’s complexity, keep them to one level." — context-menus › Best practices ; avec un titre intuitif — "give it an intuitive title that helps people predict its contents without opening it" — context-menus › Best practices
- Éléments sans objet masqués, non estompés — "Hide unavailable menu items, don’t dim them." — context-menus › Best practices ; "a context menu displays only the actions that are relevant to the currently selected view or content" — context-menus › Best practices
- Éléments les plus fréquents du côté où le menu s'ouvre — "people often read it starting from the part that’s closest to where their finger or pointer revealed it" — context-menus › Best practices ; ordre inversé si le menu s'ouvre vers le haut — "so you might also need to reverse the order of items to match the position of the menu" — context-menus › Best practices
- Raccourcis clavier affichés dans les menus principaux, pas ici — "Show keyboard shortcuts in your app’s main menus, not in context menus." — context-menus › Best practices ; "Context menus already provide a shortcut to task-specific commands, so it’s redundant to display keyboard shortcuts too." — context-menus › Best practices
- Séparateurs pour grouper — "you can use separators to group items in a context menu and help people scan the menu more quickly" — context-menus › Best practices
- Titre seulement s'il clarifie l'effet du menu — "Include a title in a context menu only if doing so clarifies the menu’s effect." — context-menus › Content ; sinon intitulé court par élément — "each item in a context menu needs to display a short label that clearly describes what it does" — context-menus › Content ; icônes familières — "Represent menu item actions with familiar icons." — context-menus › Content

**Spécifique macOS**
- Voies de révélation sur Mac : Contrôle-clic ou clic secondaire (voir Anatomie et cotes) ; le menu y est aussi appelé « contextual menu » (source : `context-menus` › Platform considerations › macOS, sans citation).
- Exception au masquage : Cut, Copy et Paste peuvent apparaître indisponibles — "In macOS, the exceptions are the Cut, Copy, and Paste menu items, which may appear unavailable if they don’t apply to the current context." — context-menus › Best practices
- Les menus de la barre des menus listent toutes les commandes, y compris celles des menus contextuels — "In macOS, an app’s menu bar menus list all the app’s commands, including those in various context menus." — context-menus › Best practices
- Éléments dynamiques déconseillés dans les menus contextuels — "Adding a dynamic menu item to contextual or Dock menus can make the item even harder for people to discover." — the-menu-bar › Dynamic menu items
- Partage : une entrée Share du menu contextuel donne accès aux extensions de partage — "choosing Share in a context menu" — activity-views › Share and action extensions
- Éléments destructeurs (Delete, Remove) en fin de menu et identifiés comme tels, en texte rouge : règle donnée pour iOS, iPadOS et visionOS, rien de spécifique à macOS — "In iOS, iPadOS, and visionOS, warn people about context menu items that can destroy data." — context-menus › Best practices

**À transposer dans un shell Linux**
- Même composant de menu contextuel pour le bureau, les icônes du dock, les applets de la barre et les fenêtres.
- Ne montrer que les actions applicables ; griser seulement Couper, Copier et Coller.
- Nombre de groupes limité au seuil d'Anatomie et cotes, un seul niveau de sous-menu, aucune colonne de raccourcis clavier.
- Actions les plus fréquentes du côté du pointeur ; inverser l'ordre quand le menu s'ouvre vers le haut (bord inférieur, dock en bas).
- Toute action de menu contextuel également proposée dans la barre des menus ou dans les réglages.

### Pop-up buttons (`pop-up-buttons`)

**Rôle**
Un bouton localisé (pop-up button) affiche un menu d'options mutuellement exclusives.

**Anatomie et cotes**
- Liste à plat d'options ou d'états exclusifs — "Use a pop-up button to present a flat list of mutually exclusive options or states." — pop-up-buttons › Best practices
- Après choix : le menu se ferme et le bouton peut afficher la sélection courante — "After people choose an item from a pop-up button’s menu, the menu closes, and the button can update its content to indicate the current selection." — pop-up-buttons › Discussion
- Non chiffré dans la HIG : taille du bouton, largeur et hauteur du menu, symbole d'indication.

**Règles**
- Utiliser un bouton à menu déroulant à la place pour — "Offer a list of actions" — pop-up-buttons › Best practices ; "Let people select multiple items" — pop-up-buttons › Best practices ; "Include a submenu" — pop-up-buttons › Best practices
- Sélection par défaut utile — "Provide a useful default selection." — pop-up-buttons › Best practices ; "When possible, make the default selection an item that most people are likely to want." — pop-up-buttons › Best practices
- Options prévisibles sans ouvrir le menu — "Give people a way to predict a pop-up button’s options without opening it." — pop-up-buttons › Best practices ; par un libellé d'introduction ou un intitulé décrivant l'effet — "you can use an introductory label or a button label that describes the button’s effect, giving context to the options" — pop-up-buttons › Best practices
- Quand la place manque — "Consider using a pop-up button when space is limited and you don’t need to display all options all the time." — pop-up-buttons › Best practices
- Option Custom pour les cas occasionnels — "If necessary, include a Custom option in a pop-up button’s menu to provide additional items that are useful in some situations." — pop-up-buttons › Best practices ; texte explicatif possible sous la liste — "You can also display explanatory text below the list to help people understand how the options work." — pop-up-buttons › Best practices

**Spécifique macOS**
- Aucune considération supplémentaire — "No additional considerations for iOS, macOS, or visionOS." — pop-up-buttons › Platform considerations
- Comportement de bouton localisé possible pour deux types propres au Mac : le bouton carré — "you can configure them to behave like push buttons, toggles, or pop-up buttons" — buttons › Platform considerations › macOS › Square buttons ; le bouton image — "You can configure an image button to behave like a push button, toggle, or pop-up button." — buttons › Platform considerations › macOS › Image buttons

**À transposer dans un shell Linux**
- Réglages : tout choix exclusif (thème, police, position de la barre, écran) présenté comme un bouton localisé affichant le choix courant, à liste plate et à valeur par défaut raisonnable.
- Réglages : libellé à gauche (« Position : ») rendant les options prévisibles sans ouvrir le menu.
- Pas de bouton localisé pour des actions, une sélection multiple ou un sous-menu : bouton à menu déroulant dans ces cas.

### Pull-down buttons (`pull-down-buttons`)

**Rôle**
Un bouton à menu déroulant (pull-down button) affiche un menu d'éléments ou d'actions directement liés à sa finalité.

**Anatomie et cotes**
- Après choix : le menu se ferme et l'action choisie est exécutée — "After people choose an item in a pull-down button’s menu, the menu closes, and the app performs the chosen action." — pull-down-buttons › Discussion
- Longueur : au moins trois éléments — "listing a minimum of three items can help the interaction feel worthwhile" — pull-down-buttons › Best practices
- Action destructrice en texte rouge — "Menus use red text to highlight actions that you identify as potentially destructive." — pull-down-buttons › Best practices
- Icône d'interface après l'intitulé, quand elle apporte de la valeur — "Include an interface icon with a menu item when it provides value." — pull-down-buttons › Best practices ; "or image after its label" — pull-down-buttons › Best practices
- Non chiffré dans la HIG : taille du bouton, largeur du menu, hauteur des éléments.

**Règles**
- Usage : commandes liées à l'action du bouton, pour en préciser la cible ou le comportement — "Use a pull-down button to present commands or items that are directly related to the button’s action." — pull-down-buttons › Best practices ; exemples : Add (quoi ajouter), Sort (attribut de tri), Back (lieu à revisiter).
- Choix exclusifs qui ne sont pas des commandes : bouton localisé — "a list of mutually exclusive choices that aren’t commands" — pull-down-buttons › Best practices
- Toutes les actions d'une vue dans un seul bouton : à éviter — "Avoid putting all of a view’s actions in one pull-down button." — pull-down-buttons › Best practices ; "A view’s primary actions need to be easily discoverable" — pull-down-buttons › Best practices
- Une ou deux entrées seulement : un autre composant (boutons, toggles, interrupteurs) — "If you need to list only one or two items, consider using alternative components to present them" — pull-down-buttons › Best practices ; trop d'entrées ralentissent — "listing too many items in a pull-down button’s menu can slow people down because it takes longer to find a specific item" — pull-down-buttons › Best practices
- Titre de menu concis, seulement s'il ajoute du sens — "Display a succinct menu title only if it adds meaning." — pull-down-buttons › Best practices
- Action destructrice signalée et confirmée — "Let people know when a pull-down button’s menu item is destructive, and ask them to confirm their intent." — pull-down-buttons › Best practices

**Spécifique macOS**
- Aucune considération supplémentaire — "No additional considerations for macOS or visionOS." — pull-down-buttons › Platform considerations
- La confirmation d'une action destructrice n'est décrite que pour iOS (action sheet) et iPadOS (popover) ; la HIG ne dit rien de macOS sur ce point.
- Aucune taille de bouton macOS n'est donnée dans cet export (voir la section `buttons`).

**À transposer dans un shell Linux**
- Barre et dock : boutons à menu (ajout de widget, tri, options d'une applet) comptant au moins trois entrées (seuil d'Anatomie et cotes).
- Actions principales jamais enfermées dans un tel menu.
- Élément destructeur (réinitialiser, supprimer un espace) en rouge, avec confirmation.
- Icône à droite de l'intitulé quand elle éclaire l'entrée.

### Activity views (`activity-views`)

**Rôle**
Une vue d'activité (activity view), souvent appelée feuille de partage (share sheet), présente un ensemble de tâches réalisables dans le contexte courant.

**Anatomie et cotes**
- Contenu : activités de partage, actions comme Copy et Print, accès rapide aux apps fréquentes — "Activity views present sharing activities like messaging and actions like Copy and Print, in addition to quick access to frequently used apps." — activity-views › Discussion
- Forme : feuille (sheet) ou popover selon l'appareil et l'orientation — "An activity view can appear as a sheet or a popover, depending on the device and orientation." — activity-views › Discussion
- Icône d'interface personnalisée : centrée dans une zone d'environ 70x70 pixels — "center it in an area measuring about 70x70 pixels" — activity-views › Best practices
- Non chiffré dans la HIG : dimensions de la feuille, nombre d'activités par rangée.

**Règles**
- Pas de doublon d'une action courante déjà proposée — "Avoid creating duplicate versions of common actions that are already available in the activity view." — activity-views › Best practices ; fonction proche d'une action existante : titre personnalisé — "If you need to provide app-specific functionality that’s similar to an existing action, give it a custom title." — activity-views › Best practices
- Un symbole et un titre court et descriptif par activité personnalisée — "Consider using a symbol to represent your custom activity." — activity-views › Best practices ; "Write a succinct, descriptive title for each custom action you provide." — activity-views › Best practices ; un verbe ou un bref syntagme verbal — "Prefer a single verb or a brief verb phrase that clearly communicates what the action does." — activity-views › Best practices ; sans nom d'entreprise ni de produit — "Avoid including your company or product name in an action title." — activity-views › Best practices
- Activités pertinentes pour le contexte, les autres exclues — "Make sure activities are appropriate for the current context." — activity-views › Best practices ; "you can exclude tasks that aren’t applicable to your app" — activity-views › Best practices
- Ouverture par le bouton Partager uniquement — "Use the Share button to display an activity view." — activity-views › Best practices
- Ordre par défaut : actions propres à l'app avant les actions disponibles dans plusieurs apps ou dans tout le système — "By default, the system lists app-specific actions before actions" — activity-views › Discussion
- Tâche longue : poursuivie en arrière-plan, état consultable dans l'app principale — "If a task is time-consuming, continue it in the background, and give people a way to check the status in your main app." — activity-views › Share and action extensions ; notification seulement pour un problème, jamais pour le simple succès — "Although you can use a notification to tell people about a problem, don’t notify them simply because the task completes." — activity-views › Share and action extensions
- Pas de vue modale au-dessus de l'extension — "Avoid placing a modal view above your extension." — activity-views › Share and action extensions ; interaction réduite à quelques étapes — "Streamline and limit interaction." — activity-views › Share and action extensions

**Spécifique macOS**
- Pas de vue d'activité sur macOS ; seules les extensions de partage et d'action y sont utilisables — "Even though macOS doesn’t provide an activity view, you can create share and action app extensions that people can use on a Mac." — activity-views › Discussion ; "Not supported in macOS, tvOS, or watchOS." — activity-views › Platform considerations
- Accès au partage : bouton Share de la barre d'outils ou entrée Share d'un menu contextuel — "In macOS, people access share extensions by clicking a Share button in the toolbar or choosing Share in a context menu." — activity-views › Share and action extensions
- Accès à une action : survol d'un contenu incorporé, bouton de la barre d'outils, action rapide du Finder — "clicking a toolbar button, or choosing a quick action in a Finder window" — activity-views › Share and action extensions

**À transposer dans un shell Linux**
- Menus contextuels : entrée « Partager… » seulement quand une cible de partage existe.
- Notifications : rien en cas de simple succès ; un message en cas de problème ; la progression visible dans l'interface du shell.
- Lanceur et menus d'action : actions propres au shell avant les actions génériques, titres verbaux courts, sans nom de produit.

### Home Screen quick actions (`home-screen-quick-actions`)

Sans objet sur Mac : les actions rapides de l'écran d'accueil (Home Screen quick actions), affichées par appui long sur une icône, sont l'équivalent mobile du menu du Dock — "people can reveal a similar menu of system-provided and custom items" — dock-menus › Discussion ; non prises en charge — "Not supported in macOS, tvOS, visionOS, or watchOS." — home-screen-quick-actions › Platform considerations ; rien à transposer au-delà de la section `dock-menus`.

### Ornaments (`ornaments`)

Sans objet sur Mac, l'ornement (ornament) étant un conteneur flottant propre à visionOS — "Not supported in iOS, iPadOS, macOS, tvOS, or watchOS." — ornaments › Platform considerations ; rien à transposer.

### Buttons (`buttons`)

**Rôle**
Un bouton (button) déclenche une action instantanée.

**Anatomie et cotes**
- Trois attributs : style (taille, couleur, forme), contenu (symbole, texte ou les deux) et rôle — "In general, a button combines three attributes to clearly communicate its function" — buttons › Discussion
- Région de touche (hit region) : 44x44 pt au moins, 60x60 pt en visionOS — "a button needs a hit region of at least 44x44 pt — in visionOS, 60x60 pt" — buttons › Best practices
- Boutons proéminents : un ou deux par vue — "Keep the number of prominent buttons to one or two per view." — buttons › Style
- Couleur selon le rôle : primaire, couleur d'accentuation de l'app ; destructeur, rouge du système — "a primary button uses an app’s accent color, whereas a destructive button uses the system red color" — buttons › Role
- Quatre types propres à macOS : bouton poussoir (push button), bouton carré (square button), bouton d'aide (help button), bouton image (image button) — "Several specific button types are unique to macOS." — buttons › Platform considerations › macOS
- Bouton image : environ 10 pixels de marge entre les bords de l'image et ceux du bouton — "Include about 10 pixels of padding between the edges of the image and the button edges." — buttons › Platform considerations › macOS › Image buttons
- Bouton d'aide : circulaire, de taille constante, avec un point d'interrogation — "Help buttons are circular, consistently sized buttons that contain a question mark." — buttons › Platform considerations › macOS › Help buttons
- Non chiffré dans la HIG : cet export ne donne aucune taille de bouton pour macOS (pas d'échelle mini, small, regular), ni hauteur, rayon des coins, espacement interne ou taille de symbole ; le bouton à hauteur flexible reprend le rayon et le remplissage du bouton standard, sans valeur — "they use the same corner radius and content padding" — buttons › Platform considerations › macOS › Push buttons

**Règles**

*Style*
- Style proéminent pour l'action la plus probable — "In general, use a button that has a prominent visual style for the most likely action in a view." — buttons › Style ; la couleur d'accentuation s'applique alors au fond — "use a prominent button style so the system can apply an accent color to the button’s background" — buttons › Style
- Distinguer l'option préférée par le style, non par la taille — "Use style — not size — to visually distinguish the preferred choice among multiple options." — buttons › Style ; des tailles identiques signalent un ensemble cohérent de choix — "you signal that the options form a coherent set of choices" — buttons › Style
- Éviter une couleur de libellé proche de celle du fond — "Avoid applying a similar color to button labels and content layer backgrounds." — buttons › Style
- Espace suffisant autour du bouton et région de touche minimale — "Make buttons easy for people to use." — buttons › Best practices ; "It’s essential to include enough space around a button so that people can visually distinguish it from surrounding components and content." — buttons › Best practices

*Contenu (Content)*
- But clairement communiqué — "Ensure that each button clearly communicates its purpose." — buttons › Content
- Actions familières, icônes familières — "Try to associate familiar actions with familiar icons." — buttons › Content
- Texte quand un libellé court est plus clair qu'une icône — "Consider using text when a short label communicates more clearly than an icon." — buttons › Content ; "write a few words that succinctly describe what the button does" — buttons › Content

*Rôle (Role)*
- Quatre rôles système : Normal, Primary (bouton par défaut, le plus probable), Cancel, Destructive — "A system button can have one of the following roles" — buttons › Role ; "The button is the default button — the button people are most likely to choose." — buttons › Role ; "The button performs an action that can result in data destruction." — buttons › Role
- Rôle primaire pour le bouton le plus probable — "Assign the primary role to the button people are most likely to choose." — buttons › Role ; il répond à la touche Retour — "When a primary button responds to the Return key, it makes it easy for people to quickly confirm their choice." — buttons › Role ; dans une vue temporaire, Retour ferme la vue — "the view can automatically close when people press Return" — buttons › Role
- Jamais de rôle primaire pour une action destructrice — "Don’t assign the primary role to a button that performs a destructive action, even if that action is the most likely choice." — buttons › Role
- État enfoncé obligatoire pour un bouton personnalisé — "Always include a press state for a custom button." — buttons › Best practices ; sans lui — "Without a press state, a button can feel unresponsive, making people wonder if it’s accepting their input." — buttons › Best practices

**Spécifique macOS**
- Infobulle (tooltip) affichée après un court survol du bouton — "In macOS and visionOS, the system displays a tooltip after people hover over a button for a moment." — buttons › Content
- Bouton poussoir (push button), type standard : texte, symbole, icône, image, ou texte et image — "You can configure a push button to display text, a symbol, an icon, or an image, or a combination of text and image content." — buttons › Platform considerations › macOS › Push buttons ; bouton par défaut et teinte possibles — "Push buttons can act as the default button in a view and you can tint them." — buttons › Platform considerations › macOS › Push buttons
- Bouton à hauteur flexible (flexible-height push button) réservé au contenu haut ou de hauteur variable — "Use a flexible-height push button only when you need to display tall or variable height content." — buttons › Platform considerations › macOS › Push buttons ; "If you need to present a button that contains two lines of text or a tall icon, use a flexible-height button; otherwise, use a standard push button." — buttons › Platform considerations › macOS › Push buttons
- Ellipse finale quand le bouton ouvre une autre fenêtre, vue ou app — "Append a trailing ellipsis to the title when a push button opens another window, view, or app." — buttons › Platform considerations › macOS › Push buttons ; "an ellipsis in a control title signals that people can provide additional input" — buttons › Platform considerations › macOS › Push buttons
- Spring loading (Magic Trackpad) : activation en glissant des éléments sur le bouton et en appuyant plus fort — "Consider supporting spring loading." — buttons › Platform considerations › macOS › Push buttons ; "lets people activate a button by dragging selected items over it and force clicking — that is, pressing harder — without dropping the selected items" — buttons › Platform considerations › macOS › Push buttons
- Bouton carré (square button, dit gradient button) : action liée à une vue, comme ajouter ou retirer des rangées d'un tableau — "initiates an action related to a view, like adding or removing rows in a table" — buttons › Platform considerations › macOS › Square buttons ; symboles ou icônes, jamais de texte — "Square buttons contain symbols or icons — not text — and you can configure them to behave like push buttons, toggles, or pop-up buttons." — buttons › Platform considerations › macOS › Square buttons ; placé contre la vue, dedans ou dessous — "usually within or beneath it" — buttons › Platform considerations › macOS › Square buttons
- Bouton carré dans la vue, jamais dans le cadre de la fenêtre — "Use square buttons in a view, not in the window frame." — buttons › Platform considerations › macOS › Square buttons ; ni barre d'outils ni barre d'état — "Square buttons aren’t intended for use in toolbars or status bars." — buttons › Platform considerations › macOS › Square buttons ; sans libellé d'introduction — "Avoid using labels to introduce square buttons." — buttons › Platform considerations › macOS › Square buttons
- Bouton d'aide (help button) : fourni par le système, ouvre l'aide de l'app — "Use the system-provided help button to display your help documentation." — buttons › Platform considerations › macOS › Help buttons ; sur le sujet lié au contexte courant — "When possible, open the help topic that’s related to the current context." — buttons › Platform considerations › macOS › Help buttons
- Un seul bouton d'aide par fenêtre — "Include no more than one help button per window." — buttons › Platform considerations › macOS › Help buttons ; dans un dialogue avec boutons de fermeture — "Lower corner, opposite to the dismissal buttons and vertically aligned with them" — buttons › Platform considerations › macOS › Help buttons ; dans un dialogue sans bouton de fermeture, une fenêtre ou un panneau de réglages — "Lower-left or lower-right corner" — buttons › Platform considerations › macOS › Help buttons
- Bouton d'aide dans la vue, jamais dans le cadre de la fenêtre — "Use a help button within a view, not in the window frame." — buttons › Platform considerations › macOS › Help buttons ; sans texte d'introduction — "Avoid displaying text that introduces a help button." — buttons › Platform considerations › macOS › Help buttons
- Bouton image (image button) : image, symbole ou icône dans une vue — "You can configure an image button to behave like a push button, toggle, or pop-up button." — buttons › Platform considerations › macOS › Image buttons ; dans la barre d'outils, un élément de barre d'outils à la place — "If you need to use an image as a button in a toolbar, use a toolbar item." — buttons › Platform considerations › macOS › Image buttons
- Bouton image sans bordure système — "In general, avoid including a system-provided border in an image button" — buttons › Platform considerations › macOS › Image buttons ; libellé éventuel placé sous l'image — "If you need to include a label, position it below the image button." — buttons › Platform considerations › macOS › Image buttons

**À transposer dans un shell Linux**
- Toutes surfaces (barre, dock, lanceur, réglages, verrouillage, aperçu des espaces) : état enfoncé sur chaque bouton et région de touche au moins égale au minimum d'Anatomie et cotes, mise à l'échelle comprise.
- Réglages et verrouillage : un seul bouton proéminent par vue (Valider, Déverrouiller), répondant à Retour ; jamais sur une action destructrice.
- Barre et dock : infobulle au survol des boutons à icône seule.
- Intitulé : ellipse finale quand le bouton ouvre une autre vue (par exemple un panneau de réglages).
- Réglages : boutons carrés à symbole, sans texte, placés contre la liste qu'ils modifient (ajouter ou retirer un widget) ; un seul bouton d'aide par panneau, dans un coin bas.
- Pas de différence de taille pour distinguer deux boutons d'un même choix : jouer sur le style.

### Toolbars (`toolbars`)

**Rôle**
Une barre d'outils (toolbar) donne un accès pratique aux commandes, contrôles et fonctions de navigation et de recherche les plus utilisés.

**Anatomie et cotes**
- Un ou plusieurs ensembles de contrôles horizontaux, en haut ou en bas de la vue, en sections logiques — "A toolbar consists of one or more sets of controls arranged horizontally along the top or bottom edge of the view, grouped into logical sections." — toolbars › Discussion
- Trois types de contenu : titre de la vue courante, contrôles de navigation (Retour, Avancer, champ de recherche), actions — "They include three types of content" — toolbars › Discussion
- Trois emplacements : bord amont (leading edge), zone centrale (center area), bord aval (trailing edge) — "the leading edge, center area, and trailing edge of the toolbar" — toolbars › Item groupings
- Groupes : trois au maximum — "In general, aim for a maximum of three." — toolbars › Item groupings
- Titre : moins de 15 caractères — "keep the title under 15 characters long so you leave enough room for other controls" — toolbars › Titles
- Action principale : une seule, côté aval — "Only specify one primary action, and put it on the trailing side of the toolbar." — toolbars › Actions
- Sur macOS : dans le cadre en haut de la fenêtre, sous la barre de titre ou intégré à elle — "In a macOS app, the toolbar resides in the frame at the top of a window, either below or integrated with the title bar." — toolbars › Platform considerations › macOS
- Non chiffré dans la HIG : hauteur, espacement, tailles ; les variantes « Standard » et « Compact » ne sont représentées que par des légendes d'images, sans cote. Seul principe géométrique : des rayons de coins concentriques à ceux de la barre — "ensure that its corner radius is also concentric with the bar’s corners" — toolbars › Best practices

**Règles**

*Choix des éléments (Best practices)*
- Éviter la surcharge et décider à l'avance ce qui passe en menu de débordement (overflow menu) — "Choose items deliberately to avoid overcrowding." — toolbars › Best practices ; "define which items move to the overflow menu as the toolbar becomes narrower" — toolbars › Best practices
- Menu Plus (More menu) pour les actions secondaires, seulement si nécessaire — "Add a More menu to contain additional actions." — toolbars › Best practices ; "Try to include all actions in the toolbar if possible, and only add this menu if you really need it." — toolbars › Best practices
- Arrière-plans et commandes teintées réduits — "Reduce the use of toolbar backgrounds and tinted controls." — toolbars › Best practices ; apparence dictée par la couche de contenu — "Instead, use the content layer to inform the color and appearance of the toolbar" — toolbars › Best practices ; "prefer using the default monochromatic appearance of toolbars" — toolbars › Best practices
- Masquage temporaire possible, avec retour fiable — "Consider temporarily hiding toolbars for a distraction-free experience." — toolbars › Best practices ; "offer ways to reliably restore hidden interface elements" — toolbars › Best practices

*Titres (Titles)*
- Un titre utile par fenêtre — "Provide a useful title for each window." — toolbars › Titles ; zone de titre vide si redondant — "If titling a toolbar seems redundant, you can leave the title area empty." — toolbars › Titles
- Jamais le nom de l'app — "Don’t title windows with your app name." — toolbars › Titles
- Un mot ou une courte locution, sous la limite de caractères d'Anatomie et cotes — "Write a concise title." — toolbars › Titles

*Navigation (Navigation)*
- Barre de navigation en haut de la fenêtre — "A toolbar with navigation controls appears at the top of a window, helping people move through a hierarchy of content." — toolbars › Navigation
- Boutons Retour et Fermer standard, par symbole et sans libellé textuel — "Use the standard Back and Close buttons." — toolbars › Navigation ; "Prefer the standard symbols for each" — toolbars › Navigation

*Actions (Actions)*
- Actions des tâches principales — "Provide actions that support the main tasks people perform." — toolbars › Actions ; "prioritize the commands that people are most likely to want" — toolbars › Actions
- Sens de chaque contrôle évident — "Make sure the meaning of each control is clear." — toolbars › Actions ; "Don’t make people guess or experiment to figure out what a toolbar item does." — toolbars › Actions
- Symboles du système sans bordure — "Prefer system-provided symbols without borders." — toolbars › Actions ; états de survol et de sélection définis par le système — "the system defines hover and selection state appearances automatically" — toolbars › Actions
- Style proéminent pour l'action clé (Done, Submit) — "This separates and tints the action so there’s a clear focal point." — toolbars › Actions

*Regroupements (Item groupings)*
- Bord amont : retour au document précédent, affichage de la barre latérale, puis titre, puis un menu du document ; rien n'y est personnalisable — "items on the toolbar’s leading edge aren’t customizable" — toolbars › Item groupings ; "affect the document as a whole, such as Duplicate, Rename, Move, and Export" — toolbars › Item groupings

- Zone centrale : personnalisable, se replie dans le menu de débordement géré par le système quand la fenêtre rétrécit — "items in this section automatically collapse into the system-managed overflow menu when the window shrinks enough in size" — toolbars › Item groupings
- Bord aval : éléments importants, inspecteurs, recherche facultative, menu Plus, action principale ; toujours visible — "Items on the trailing edge remain visible at all window sizes." — toolbars › Item groupings
- Regrouper par fonction et fréquence d'usage, avec le même placement d'une plateforme à l'autre — "Group toolbar items logically by function and frequency of use." — toolbars › Item groupings ; "Keep consistent groupings and placement across platforms." — toolbars › Item groupings
- Navigation et actions critiques (Done, Close, Save) dans des sections distinctes — "Group navigation controls and critical actions like Done, Close, or Save in dedicated, familiar, and visually distinct sections." — toolbars › Item groupings
- Nombre de groupes réduit — "Minimize the number of groups." — toolbars › Item groupings ; "Too many groups of controls can make a toolbar feel cluttered and confusing, even with the added space on iPad and Mac." — toolbars › Item groupings
- Actions à libellé textuel séparées des actions à symbole — "Keep actions with text labels separate." — toolbars › Item groupings ; risque : "can create the illusion of a single action with a combined text and symbol, leading to confusion and misinterpretation" — toolbars › Item groupings ; "Add separation by inserting fixed space between the buttons." — toolbars › Item groupings
- Épinglage à un bord et espacement pour positionner — "insert space between buttons or other items where appropriate" — toolbars › Item groupings

**Spécifique macOS**
- Emplacement : cadre supérieur de la fenêtre, sous la barre de titre ou intégré à elle (voir Anatomie et cotes) ; titre aligné avec les contrôles, éléments sans bezel — "window titles can display inline with controls, and toolbar items don’t include a bezel" — toolbars › Platform considerations › macOS
- Chaque élément de barre d'outils est aussi une commande de la barre des menus — "Make every toolbar item available as a command in the menu bar." — toolbars › Platform considerations › macOS ; "Because people can customize the toolbar or hide it, it can’t be the only place that presents a command." — toolbars › Platform considerations › macOS ; la réciproque est fausse — "it doesn’t make sense to provide a toolbar item for every menu item" — toolbars › Platform considerations › macOS
- Menu de débordement ajouté automatiquement quand les éléments ne tiennent plus — "The system automatically adds an overflow menu in macOS or iPadOS when items no longer fit." — toolbars › Best practices ; jamais ajouté à la main — "Don’t add an overflow menu manually, and avoid layouts that cause toolbar items to overflow by default." — toolbars › Best practices
- Personnalisation envisageable — "In iPadOS and macOS apps, consider letting people customize the toolbar to include their most common items." — toolbars › Best practices ; ajout, retrait et réordonnancement dans la zone centrale — "In macOS and iPadOS, people can add, remove, and rearrange items here if you let them customize the toolbar" — toolbars › Item groupings
- Affichage de la barre commandé par le menu View — "when the toolbar is hidden, provide a Show Toolbar menu item; when the toolbar is visible, provide a Hide Toolbar menu item" — the-menu-bar › View menu
- Boutons dans la barre d'outils : des éléments de barre d'outils, et non des boutons carrés, image ou d'aide (voir la section `buttons`).
- Partage : bouton Share dans la barre d'outils — "clicking a Share button in the toolbar" — activity-views › Share and action extensions

**À transposer dans un shell Linux**
- Barre : les zones amont (menu système, lanceur, espaces : navigation), centrale (widgets) et aval (recherche, applets, action principale) d'Anatomie et cotes ; la zone centrale seule se replie dans un menu de débordement automatique quand la place manque.
- Réglages : personnalisation des widgets de la zone centrale (ajouter, retirer, réordonner) ; la navigation jamais personnalisable.
- Toute commande présente dans la barre l'est aussi dans un menu.
- Titre des panneaux (aperçu des espaces, réglages, verrouillage) court, et jamais le nom du shell.
- Nombre de groupes limité au seuil d'Anatomie et cotes, visuellement distincts ; boutons à texte séparés par un espace fixe.
- Barre masquable (plein écran, mode sans distraction), avec un moyen fiable de la faire réapparaître.

### Valeurs de référence

| Propriété | Valeur | Source (slug › section) |
|---|---|---|
| Hauteur de la barre des menus | 24 pt | the-menu-bar › Platform considerations › macOS › Menu bar extras |
| Ordre des menus de la barre | NomApp, File, Edit, Format, View, menus propres à l'app, Window, Help | the-menu-bar › Anatomy |
| Menu Apple | Toujours premier, côté amont (leading) ; ni modifiable ni retirable | the-menu-bar › Platform considerations › macOS |
| Éléments de barre de menus (menu bar extras) | Côté aval (trailing), si la place le permet ; masqués par le système au besoin | the-menu-bar › Platform considerations › macOS |
| Nom abrégé de l'app (About, Hide, Quit) | 16 caractères au plus, sans numéro de version | the-menu-bar › App menu |
| Largeur d'un menu | Élément le plus large, éléments dynamiques compris | the-menu-bar › Dynamic menu items |
| Élément dynamique | Une seule touche modificatrice : Control, Option, Shift ou Command | the-menu-bar › Dynamic menu items |
| Menu Help | Dernier de la barre | the-menu-bar › Help menu |
| Profondeur des sous-menus | Un niveau | menus › Submenus |
| Longueur d'un sous-menu | Plus d'environ cinq éléments : créer un menu | menus › Submenus |
| Groupes d'un menu contextuel | Environ trois au plus | context-menus › Best practices |
| Éléments d'un bouton à menu déroulant | Trois au minimum | pull-down-buttons › Best practices |
| Action destructrice dans un menu | Texte rouge | pull-down-buttons › Best practices |
| Région de touche d'un bouton | 44x44 pt au moins (60x60 pt en visionOS) | buttons › Best practices |
| Boutons proéminents | Un ou deux par vue | buttons › Style |
| Couleur d'un bouton selon son rôle | Primaire : couleur d'accentuation de l'app ; destructeur : rouge système | buttons › Role |
| Marge d'un bouton image | Environ 10 pixels | buttons › Platform considerations › macOS › Image buttons |
| Boutons d'aide | Un au plus par fenêtre ; coin bas | buttons › Platform considerations › macOS › Help buttons |
| Titre d'une barre d'outils | Moins de 15 caractères | toolbars › Titles |
| Emplacements dans une barre d'outils | Bord amont, zone centrale, bord aval | toolbars › Item groupings |
| Groupes d'une barre d'outils | Trois au maximum | toolbars › Item groupings |
| Action principale d'une barre d'outils | Une seule, côté aval | toolbars › Actions |
| Icône d'activité personnalisée (vue d'activité, absente de macOS) | Environ 70x70 pixels | activity-views › Best practices |

Valeurs absentes de cet export, à ne pas déduire : hauteurs et largeurs des éléments de menu, tailles de police, rayons des coins, espacements internes, durées et délais d'ouverture, de survol et de fermeture, glyphes des touches modificatrices et liste des raccourcis standard (page `keyboards`, non couverte), contenu du menu Apple, tailles macOS des boutons (mini, small, regular), dimensions des variantes standard et compacte des barres d'outils, hauteur et espacement de la barre d'outils.

## Seconde partie — hyprdesk aujourd'hui, écarts et actions

**Ce qui tient lieu de barre des menus.** Rien au sens de la HIG : hyprdesk n'a ni menus de premier niveau, ni menu système, ni menu Aide. La barre est une barre de modules en trois zones (amont, centre, aval). L'élément le plus proche d'un titre de la barre des menus est le **titre de la fenêtre active**, qui ouvre au clic le **menu de la fenêtre** (réduire, agrandir, plein écran, flottante, sur tous les espaces, déplacer, fermer). Les autres commandes du shell sont réparties entre la carte des espaces, le centre de contrôle, le bouton Réglages du dock et les menus contextuels. L'absence d'un menu global pour les applications tierces est un choix argumenté dans le code (`src/ui/plugins/bar/window.rs:1-6` : GTK4 et libadwaita ne publient plus leurs menus) ; cet argument vaut pour les applications tierces (SANS OBJET), pas pour les commandes propres au shell.

### Ce que hyprdesk fait aujourd'hui
| Surface | Ce qui existe | Valeurs clés | Preuves |
| --- | --- | --- | --- |
| Rôle de la barre des menus | Aucun menu de premier niveau. La barre porte des modules : espaces, titre de la fenêtre active, bouton bureau, volume, zone de notification, luminosité, widgets | Trois zones ; largeur minimale 95 px (espaces), 60 px (volume) | `src/config/model.rs:952-957` ; `src/ui/plugins/bar/surface.rs:517-519` ; `src/ui/plugins/bar/window.rs:1-6` |
| Menu de la fenêtre active | Clic sur le titre (écran d'au moins 1000 px) : Réduire (désactivé si réduite), Agrandir, Plein écran, Flottante, Sur tous les espaces (désactivé si non flottante), coches d'état ; séparateur ; en-tête « Déplacer vers l'espace » et jusqu'à 10 espaces ; séparateur ; Fermer | Carte de 328 px ; lignes 30 px, en-têtes 26 px, séparateurs 11 px | `src/ui/plugins/bar/window.rs:30-122` ; `src/ui/plugins/bar/surface.rs:1916-1951` ; `src/ui/plugins/bar/popover.rs:26`, `1470-1624` |
| Menus de la zone de notification (dbusmenu) | Arbre aplani : chaque sous-menu devient un en-tête de section portant son chemin (« Fichier › Récents »), à toute profondeur ; coches et états repris ; `AboutToShow` avant lecture | Au plus 256 entrées, profondeur 8 ; défilement borné à l'écran | `src/tray.rs:444-450`, `485-526`, `550-575` ; `src/ui/plugins/bar/popover.rs:679-690`, `1154-1171`, `1318`, `1377` |
| Menus des plugins Lua | Mêmes entrées : élément (libellé, détail, icône, coche, état, confirmation), séparateur, en-tête, image, champ de saisie | — | `src/config/model.rs:854-880` ; `src/ui/plugins/bar/popover.rs:1085-1145` |
| Confirmation dans les menus de la barre | Premier clic : rangée ajoutée sous l'entrée (question, Annuler, Confirmer à fond rouge plein) ; second clic : action | Rangée de 40 px | `src/ui/plugins/bar/popover.rs:1085-1102`, `1550-1616` |
| Carte des espaces et débordement | Liste des espaces de l'écran, coche sur l'actif, puis « Organiser les fenêtres… » ; les modules qui ne tiennent plus passent dans « Autres commandes » | Carte de 300 px | `src/ui/plugins/bar/popover/bar_menus.rs:10-66`, `68-152` ; `src/ui/plugins/bar/layout.rs:105-118` |
| Composant de menu unique | Ligne (icône ou coche, libellé, détail à droite), séparateur, en-tête ; survol d'accent et appui distincts ; entrée indisponible grisée et sans survol ; action destructrice en rouge ; flèches, Début, Fin, Entrée | Lignes 30 px (44 si détail long) ; survol 0,12, appui 0,24 ; désactivé à 0,35 ; icône 16 px, coche 14 px | `src/ui/menu.rs:19-23`, `68-99`, `108-177`, `198-222`, `282-362` ; `docs/PLUGINS.md:52-59` |
| Menus du dock et de la grille du Lanceur | Clic droit : fenêtres ouvertes (la plus récente d'abord, coche sur l'active, mention « réduite » ou « espace N ») ; Ouvrir ou Nouvelle fenêtre, actions `.desktop` ; Épingler au dock ou Retirer du dock, Fermer la fenêtre ou les N fenêtres (rouge). Pas de « Quitter » | Largeur 300 px | `src/ui/context.rs:724-831`, `1094-1117`, `1121-1167` ; `src/ui/plugins/dock/surface.rs:756-757` |
| Menus contextuels du bureau | Icône : Ouvrir, Ouvrir le dossier, actions `.desktop` ; Renommer…, Partager…, Copier le chemin, Définir comme fond d'écran, Épingler, Propriétés, Réglages des icônes… ; Retirer du bureau, Mettre à la corbeille (rouge, second clic pour confirmer). Bureau : Aligner les icônes ; Ajouter un widget…, Changer de fond d'écran… ; Réglages du bureau…, Propriétés. Widget : Réglages du widget…, Propriétés, Retirer du bureau. Page « Partager… » | Largeurs 260 / 300 / 220 / 400 px, marge 6 px ; non défilants | `src/ui/context.rs:501-709`, `834-852`, `917-927`, `998-1057`, `1865-1923` |
| Menu d'édition des champs | Couper, Copier, Coller (grisés hors selection ou hors champ modifiable), Tout sélectionner | Largeur 220 px | `src/ui/edit.rs:26-31` ; `src/ui/context.rs:591-623`, `1206-1224` |
| Surfaces sans menu contextuel | Aperçu des espaces, notifications, modules de barre (le clic droit y est une action directe : sourdine sur le volume), boutons de commande du dock | — | `src/ui/overview.rs` (aucun clic droit) ; `src/ui/plugins/dock/surface.rs:756` ; `src/ui/plugins/bar/surface.rs:1797-1800`, `2082` |
| Applets de la barre | Zone de notification : menu au clic. Volume et luminosité : carte à curseurs. Horloge : calendrier. Volume : clic droit pour la sourdine, molette par pas de 5 % | — | `src/ui/plugins/bar/popover.rs:491-507`, `1232-1234` ; `src/ui/plugins/bar/surface.rs:2080-2083` |
| Boutons | Module de barre : hauteur de la barre moins 8 (28 px à 36), padding horizontal 9, info-bulle. Boutons du centre de contrôle 38 à 48 px. Confirmations des menus environ 24 px. « Par défaut… » des Réglages environ 22 px | — | `src/ui/plugins/bar/surface.rs:1769-1807` ; `src/ui/plugins/bar/popover/continuity.rs:490-504`, `590-596` ; `src/ui/plugins/bar/popover.rs:1562-1609` ; `src/ui/panel.rs:536-553` |
| Écran Réglages (rôle de barre d'outils) | En-tête : fermeture avec info-bulle, champ « Rechercher un réglage… », « Par défaut… » suivi d'une confirmation en ligne ; navigation par liste des paquets (icône 18 px, libellé medium 13 px, compteur de résultats) | Titre « Réglages » | `src/ui/panel.rs:536-609`, `724-800` (fermeture et recherche : `:903-920`, repérés par grep) |
| Raccourcis affichés | Noms Hyprland en toutes lettres (SUPER, CTRL, ALT, SHIFT), aucun glyphe de modificateur. Le composant réserve une colonne de détail à droite, vide dans le menu de la fenêtre et les menus dbusmenu | — | `src/hypr.rs:572`, `1001-1003` ; `src/ui/plugins/keys.rs:578-600` ; `src/ui/menu.rs:333-342` |
| Commandes de session | Centre de contrôle : Espaces de travail, Réglages, Verrouiller. Ni mise en veille, ni redémarrage, ni extinction, ni déconnexion dans la barre ou ses cartes | — | `src/ui/plugins/bar/popover/continuity.rs:464-596` ; écran de connexion et Panorama seuls (`src/greeter_ui.rs:1067-1069`, `src/panorama/design.rs:266-276`, repérés par grep) |

### Écarts
Classés du plus important au moins important. Les mentions « repéré par grep » signalent une ligne située par recherche, dont le contexte n'a pas été lu.

| # | Composant | Nature | Force | Règle HIG (slug › section) | Réel dans hyprdesk (preuve) | Action proposée | Effort | Confiance |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| MENU-01 | Barre des menus : inventaire des commandes du shell | ABSENT | fait de plateforme, impératif ("it's important to list them in the menu bar") | the-menu-bar › Platform considerations › macOS ("The menu bar in macOS includes the Apple menu, which is always the first item on the leading side of the menu bar.") ; the-menu-bar › App-specific menus ("Even when commands are available elsewhere in your app, it's important to list them in the menu bar.") ; the-menu-bar › Anatomie et cotes (ordre fixe) | Aucun menu de premier niveau : la barre ne porte que des modules (`src/config/model.rs:952-957`). Les commandes du shell sont dispersées : clic sur le titre (`src/ui/plugins/bar/window.rs:30-122`), carte des espaces (`src/ui/plugins/bar/popover/bar_menus.rs:68-152`), centre de contrôle limité à Espaces de travail, Réglages et Verrouiller (`src/ui/plugins/bar/popover/continuity.rs:490-504`, `590-596`), bouton Réglages du dock (`src/ui/plugins/dock/surface.rs:547`, repéré par grep), clics droits. Mise en veille, redémarrage et extinction n'existent que sur l'écran de connexion et dans Panorama (`src/greeter_ui.rs:1067-1069`, repéré par grep). L'absence d'un menu global est argumentée dans le code (`src/ui/plugins/bar/window.rs:1-6`) pour les applications tierces ; ni `DESIGN.md` ni `design-plans/` ne la consignent. | Ajouter en tête de la zone amont un menu « hyprdesk », toujours premier et non retirable, qui rassemble les commandes du shell (Réglages…, Verrouiller, Espaces de travail, Organiser les fenêtres…, mise en veille, redémarrer, éteindre) et reçoit le menu de la fenêtre et celui des espaces comme menus titrés à sa suite. | L | haute |
| MENU-02 | Menus contextuels (bureau, icônes, widgets, dock, grille, édition) | ÉCART | impératif ("Aim for"), règle propre au projet | context-menus › Best practices ("Aim for a small number of menu items.", "A context menu that's too long can be difficult to scan and scroll.") ; menus › Organization ("a long menu is fine, and scrolling is acceptable") ; `DESIGN.md:16` | Ces menus sont une surface layer-shell taillée au contenu, sans défilement ni plafond de hauteur : `src/ui/context.rs:917-927` (hauteur égale à la somme des lignes), `:1007-1017` (seuls x et y sont bornés), `:1043` (taille de la surface), `:1865-1922` (aucun `scrollable`). Or le menu du dock liste toutes les fenêtres de l'application (`:769-789`) et toutes ses actions `.desktop` (`:724-742`) : au-delà d'une vingtaine de fenêtres, les dernières lignes (épingler, fermer) sortent de l'écran. Les menus de la barre, eux, défilent et sont plafonnés (`src/ui/plugins/bar/popover.rs:679-690`, `1154-1171`, `1318`). `DESIGN.md:16` promet « Menus longs défilants et bornés à la hauteur de l'écran ». | Envelopper la carte du clic droit dans le même défilement plafonné à l'écran que les menus de la barre. | S | haute |
| MENU-03 | Sous-menus d'un niveau | ABSENT | fait de plateforme, recommandation ("generally best") | menus › Anatomie et cotes ("A menu item indicates the presence of a submenu by displaying a symbol — like a chevron — after its label.") ; menus › Submenus ("it's generally best to restrict them to a single level") ; menus › Organization ("If a menu is too long, consider dividing it into separate menus.") | Ni `Item` (`src/ui/menu.rs:108-118`, `156-159`) ni `MenuEntry` (`src/config/model.rs:854-880`) n'ont de sous-menu. L'arbre dbusmenu est donc aplani en une seule liste, à en-têtes de chemin, pouvant atteindre 256 entrées (`src/tray.rs:444-450`, `485-511`) ; « Déplacer vers l'espace » est un groupe en ligne arrêté à 10 espaces (`src/ui/plugins/bar/window.rs:96-112`). | Ajouter au composant un sous-menu d'un niveau (chevron après le libellé, ouvert au survol) et l'employer pour l'arbre dbusmenu et pour « Déplacer vers l'espace ». | M | haute |
| MENU-04 | Commandes accessibles seulement par clic droit | ÉCART | "Always" | context-menus › Best practices ("Always make context menu items available in the main interface, too.") | « Mettre à la corbeille » (`src/ui/context.rs:691-704` ; seul appel de `trash()` : `:1326`) et « Fermer les N fenêtres » (`:820-828`, `:1545-1561` ; les deux autres appels de `close_window` visent la fenêtre active ou une fenêtre désignée : `src/ui/plugins/bar/window.rs:132`, `src/ui/plugins/assistant.rs:190`) ne s'atteignent que par le menu contextuel. | Recenser ces deux commandes dans le menu du shell (MENU-01). | S, une fois MENU-01 en place | haute |
| MENU-05 | Menu Aide | ABSENT | fait de plateforme ("located at the trailing end of the menu bar") | the-menu-bar › Help menu ("The Help menu — located at the trailing end of the menu bar — provides access to an app's help documentation.") ; the-menu-bar › Anatomie et cotes (champ de recherche en tête) | Aucune entrée d'aide dans la barre ni ses cartes : ni le catalogue des modules (`src/config/model.rs:952-957`) ni le centre de contrôle (`src/ui/plugins/bar/popover/continuity.rs:464-596`) n'en comptent. Les seuls libellés d'aide repérés sont « Visite guidée » dans l'en-tête des Réglages (`src/ui/panel.rs:935`) et « Documentation » dans la grille du Lanceur (`src/ui/plugins/launcher/grid.rs:105`). | Ajouter un menu « Aide » en dernier de la barre (visite guidée, documentation, à propos), avec un champ de recherche en tête. | M | haute |
| MENU-06 | Menus contextuels d'une surface à l'autre | PISTE | impératif nu ("Support … consistently") | context-menus › Best practices ("Support context menus consistently throughout your app.") | Présents sur les icônes (`src/ui/mod.rs:6161`), le fond (`:5964`), les widgets (`src/ui/widget_view.rs:283`), les applications du dock (`src/ui/plugins/dock/surface.rs:757`), la grille du Lanceur (`src/ui/context.rs:1094-1117`) et les champs. Absents sur les fenêtres et les cartes de l'aperçu des espaces (`src/ui/overview.rs`), les notifications et les modules de barre, où le clic droit est une action directe (`src/ui/plugins/bar/surface.rs:1797-1800`, `2082`). | Offrir le menu de la fenêtre au clic droit sur une fenêtre de l'aperçu des espaces. | S | moyenne |
| MENU-07 | Titre de la fenêtre active dans la barre | PISTE | fait de plateforme ("displays your app name in bold") | the-menu-bar › App menu ("To help people quickly identify the active app, the menu bar displays your app name in bold.") ; the-menu-bar › App-specific menus ("Excluding commands from the menu bar — even infrequently used or advanced commands — risks making them difficult for everyone to find.") | Seule porte du menu de la fenêtre : un titre tronqué à 18 ou 36 caractères, en graisse Normal, atténué quand le menu est fermé, sans info-bulle (`src/ui/plugins/bar/surface.rs:1930-1950`), sur une zone de touche de 28 px de haut (`:1784`). Il disparaît sous 1000 px de large d'écran (`:1917-1924`), ce qui retire tout accès au menu. | Marquer ce titre comme un menu : graisse semibold et chevron. | S | haute |
| MENU-08 | « Fermer les N fenêtres » | PISTE | impératif, portée incertaine (règle donnée pour les boutons à menu déroulant) | pull-down-buttons › Best practices ("Let people know when a pull-down button's menu item is destructive, and ask them to confirm their intent.") | L'entrée est identifiée comme destructrice, en rouge (`src/ui/context.rs:817`, `826`), mais ferme toutes les fenêtres dès le premier clic (`:1545-1561`). « Mettre à la corbeille », réversible, demande un second clic (`:1315-1319`). | Demander confirmation avant de fermer plusieurs fenêtres. | S | moyenne |
| MENU-09 | Ordre des entrées du menu du dock | PISTE | "might also need" | context-menus › Best practices ("people often read it starting from the part that's closest to where their finger or pointer revealed it", "so you might also need to reverse the order of items to match the position of the menu") | L'ordre des groupes est fixe : fenêtres, ouvrir et actions, épingler et fermer (`src/ui/context.rs:830`). La carte se pose contre le bouton (`src/ui/plugins/dock/lifecycle.rs:77-98`). Avec le dock en bas (valeur de la session), la liste des fenêtres est à l'extrémité la plus loin du pointeur et l'action destructrice la plus près. | Inverser l'ordre des groupes quand le menu s'ouvre vers le haut. | S | moyenne |
| MENU-10 | Menu d'édition des champs | PISTE | impératif conditionnel ("when possible") | edit-menus › Best practices ("Support undo and redo when possible.") ; the-menu-bar › Edit menu (Undo et Redo en tête) | Quatre commandes seulement : Couper, Copier, Coller, Tout sélectionner (`src/ui/edit.rs:26-31`, `src/ui/context.rs:591-623`). | Ajouter Annuler et Rétablir en tête du menu. | M | moyenne |
| MENU-11 | Titre des menus contextuels | PISTE | conditionnel ("only if doing so clarifies") | context-menus › Content ("Include a title in a context menu only if doing so clarifies the menu's effect.") ; context-menus › Anatomie et cotes ("A context menu seldom displays a title.") | Tous portent un titre sauf le menu d'édition : « Bureau », nom de l'application, type du widget (parfois suivi de « — déclaré dans hyprdesk.lua »), nom de l'icône (`src/ui/context.rs:855-892`, rendu en `:1881-1883`). Seuls le nom de l'icône et la page « Partager… » désignent la cible de l'action. | Ne garder le titre que sur le menu d'une icône et sur la page de partage. | S | haute |
| MENU-12 | Menu contextuel d'une icône du bureau | PISTE | "Aim for", "Consider" | context-menus › Best practices ("Aim for a small number of menu items.") ; menus › Organization ("Consider grouping logically related items.", "use a separator") | Pour un fichier qui est aussi une application : jusqu'à 13 entrées en trois groupes, dont un groupe central de sept commandes hétérogères sans séparateur (`src/ui/context.rs:638-679`, `706`). | Rattacher « Propriétés » et « Réglages des icônes… » au dernier groupe, pour ramener le groupe central à cinq commandes. | S | haute |
| MENU-13 | Libellés anglais des menus | ÉCART | impératif nu ("use") | menus › Labels ("To be consistent with platform experiences, use title-style capitalization.", "In English, articles always lengthen labels, but rarely enhance understanding.") | En anglais, les libellés sont en cas de phrase : "New window" (`locales/en.toml:800`), "Nothing to show" (`:925`), "Move to trash" (`:1641`), "Pin to the dock" (`:2482`), "Close the window" (`:2880`, avec article), "More controls" (`:2978`), "On all workspaces" (`:4835`), "Select all" (`:4836`), "Move to workspace" (`:4826`). Les libellés français ne sont pas en cause : la règle est propre à l'anglais. | Réécrire les libellés anglais des menus en cas de titre, sans article. | S | moyenne |
| MENU-14 | Boutons de confirmation et « Par défaut… » | ÉCART | cote | buttons › Best practices ("a button needs a hit region of at least 44x44 pt") | « Annuler » et « Confirmer » des menus de la barre : corps de 12,5 px et padding de 4, soit environ 24 px de haut (`src/ui/plugins/bar/popover.rs:1562-1609`). « Par défaut… » de l'en-tête des Réglages : corps de 12 px et padding de 3, soit environ 22 px (`src/ui/panel.rs:540-551`). | Donner à ces trois boutons une région de touche d'au moins 44 × 44 px, à taille visuelle inchangée. | S | moyenne |
| MENU-15 | Icônes des actions `.desktop` | ÉCART | "Don't" | menus › Icons ("Don't display an icon if you can't find one that clearly represents the menu item.", "provide icons for all menu items in a group, or none of them") | Une action sans icône reçoit une icône générique, `application-x-executable-symbolic`, à côté d'actions qui ont la leur (`src/ui/context.rs:729-739`). | Ne pas fabriquer d'icône de secours : sans icône claire pour une action, afficher le groupe des actions sans icônes. | S | moyenne |
| MENU-16 | « Confirmer » d'une action destructive | PISTE | "Don't" (rôle) | buttons › Role ("Don't assign the primary role to a button that performs a destructive action, even if that action is the most likely choice.") ; buttons › Style ("use a prominent button style for the most likely action in a view") | Dans les menus de la barre, « Confirmer » est le bouton proéminent, à fond rouge plein, et « Annuler » le bouton neutre (`src/ui/plugins/bar/popover.rs:1586-1609`). Les Réglages rendent la même confirmation avec le style `danger` (`src/ui/panel.rs:578-585`). | Retirer à « Confirmer » son fond proéminent (texte rouge), « Annuler » restant le bouton simple. | S | moyenne |
| MENU-17 | Libellés du menu de la fenêtre | PISTE | impératif ("using a verb or verb phrase"), "Include a verb" | menus › Labels ("label a menu item that initiates an action using a verb or verb phrase that describes the action") ; menus › Toggled items ("Include a verb if a changeable label isn't clear enough.") | Trois entrées d'état sont un nom ou un adjectif : « Flottante », « Sur tous les espaces », « Plein écran » (`src/ui/plugins/bar/window.rs:67-88`), à la différence de « Réduire » et « Agrandir ». | Les rédiger avec un verbe, comme « Réduire » et « Agrandir ». | S | moyenne |
| MENU-18 | Raccourcis du menu de la fenêtre | PISTE | facultatif ("can also display") | menus › Anatomie et cotes ("In an app, a menu item can also display the associated keyboard command, if there is one") | Le composant réserve une colonne de détail à droite du libellé (`src/ui/menu.rs:333-342`). Le menu de la fenêtre et les menus dbusmenu la laissent vide (`src/ui/plugins/bar/window.rs:44`, `src/tray.rs:514`), alors que le shell lit les raccourcis de `hyprland.lua` (`src/ui/plugins/keys.rs:578-600`). | Afficher dans le menu de la fenêtre le raccourci de chaque commande quand l'utilisateur en a défini un. | S | faible |
| MENU-19 | Rangée « Organiser les fenêtres… » | PISTE | impératif nu ("use a separator") | menus › Organization ("To help people visually distinguish such groups, use a separator.") | La carte des espaces ajoute cette commande sous la liste sans séparateur (`src/ui/plugins/bar/popover/bar_menus.rs:117-150`), avec son propre dessin : survol neutre à 0,10 et rayon de 6 (`:134-148`), contre survol d'accent à 0,12 et rayon de 10 dans le composant commun (`src/ui/menu.rs:76-85`, `358`). | Dessiner cette rangée avec le composant commun, après un séparateur. | S | haute |
| MENU-20 | Hauteur et styles de barre | IDENTITÉ | — | the-menu-bar › Platform considerations › macOS › Menu bar extras ("The menu bar's height is 24 pt.") | Barre de 36 px dans la session, 48 px pour Graphite, quatre styles documentés (`DESIGN.md:17-19`). | Aucune : à signaler seulement. | — | haute |
| MENU-21 | Menu du dock sans « Quitter » | IDENTITÉ | — | dock-menus › Anatomie et cotes ("The system-provided Dock menu items can vary depending on whether the app is open." ; cet export n'en liste pas le contenu) | Le menu ferme les fenêtres de l'application, il ne la quitte pas (`src/ui/context.rs:809-829`). Ce contenu est arrêté dans `design-plans/barre-graphite.md:166`. Une application qui tourne sans fenêtre ne s'arrête donc pas depuis le dock. | Aucune : à signaler seulement. | — | haute |

Bilan : 3 ABSENT (MENU-01, 03, 05), 5 ÉCART (MENU-02, 04, 13, 14, 15), 11 PISTE (MENU-06 à 12 et 16 à 19), 2 IDENTITÉ (MENU-20, 21).

### Ce qui est déjà conforme
(règle | preuve)

- Un seul composant de menu pour la barre et le clic droit ("The guidance for labeling and organizing menu items applies to all types of menus in all experiences.", menus › Discussion) | `src/ui/menu.rs:1-10`, `303-396` ; `src/ui/plugins/bar/popover.rs:1452-1468` ; `src/ui/context.rs:1884`
- Une commande sans effet est désactivée, jamais retirée ("If a menu bar item isn't actionable, disable the action instead of hiding it from the menu.", the-menu-bar › Best practices) | `src/ui/plugins/bar/window.rs:57`, `86` ; `src/ui/plugins/bar/popover.rs:1541-1549`
- Entrée indisponible estompée et sans réponse ("An unavailable menu item often appears dimmed and doesn't respond to interactions.", menus › Anatomie et cotes) | `src/ui/menu.rs:73-75`, `96`, `354`
- Entrées sans objet masquées dans les menus contextuels ("Hide unavailable menu items, don't dim them.", context-menus › Best practices) | `src/ui/context.rs:627-635`, `644-660`, `681-691`, `833-852` ; `src/tray.rs:492-494`
- Couper, Copier et Coller grisés hors contexte, seuls tolérés ("the exceptions are the Cut, Copy, and Paste menu items", context-menus › Best practices) | `src/ui/context.rs:593-613`
- Ellipse quand l'action demande une suite ("Append an ellipsis to a menu item's label when the action requires more information before it can complete.", menus › Labels) | `src/ui/context.rs:512`, `543`, `553`, `576`, `641`, `645`, `674`, `843` ; `src/ui/plugins/bar/popover/bar_menus.rs:123` ; `src/ui/panel.rs:542`
- Trois groupes au plus, séparés par un trait ("In general, you don't want more than about three groups in a context menu.", context-menus › Anatomie et cotes) | `src/ui/context.rs:560-588`, `706`, `830` ; `src/ui/menu.rs:168-177`
- Entrées importantes ou fréquentes en premier ("Prefer listing important or frequently used menu items first.", menus › Organization) | `src/ui/context.rs:626`, `769`, `797` ; `src/ui/plugins/bar/window.rs:52-59`
- Action destructrice en texte rouge ("Menus use red text to highlight actions that you identify as potentially destructive.", pull-down-buttons › Best practices) | `src/ui/menu.rs:90-92` ; `src/ui/context.rs:702`, `817`, `826`
- Confirmation d'une action destructrice (corbeille, entrées Lua `confirm`, remise à défaut des Réglages) | `src/ui/context.rs:694-698`, `1315-1319` ; `src/ui/plugins/bar/popover.rs:1085-1102`, `1550-1616` ; `src/ui/panel.rs:557-592`
- Intitulé changeant plutôt que deux entrées ("Consider using a changeable label that describes an item's current state.", menus › Toggled items) | `src/ui/context.rs:712-721`, `795-803`
- Coche pour l'attribut en vigueur ("Consider using a checkmark to show that an attribute is currently in effect.", menus › Toggled items) | `src/ui/menu.rs:282-287` ; `src/ui/plugins/bar/window.rs:63`, `70`, `77`, `85` ; `src/ui/plugins/bar/popover/bar_menus.rs:96-104`
- État enfoncé distinct du survol ("Always include a press state for a custom button.", buttons › Best practices) | `src/ui/menu.rs:76-81`
- Menus de la barre défilants et bornés à l'écran | `src/ui/plugins/bar/popover.rs:679-690`, `1154-1171`, `1318`, `1377`
- Pas de raccourci clavier dans les menus contextuels ("Show keyboard shortcuts in your app's main menus, not in context menus.", context-menus › Best practices) | `src/ui/context.rs:786`, seul détail affiché : « réduite » ou « espace N »
- Icônes de toutes les entrées d'un groupe, ou d'aucune ("provide icons for all menu items in a group, or none of them", menus › Icons) | `src/ui/context.rs:509-536`, `560-588`, `624-705`, `790-829`
- Menu, et non une vue complexe, au clic sur l'élément de notification ("Display a menu — not a popover — when people click your menu bar extra.", the-menu-bar › Platform considerations › macOS › Menu bar extras) | `src/ui/plugins/bar/popover.rs:1234`, `1470-1624` ; `src/tray.rs:550-575`
- Liste des fenêtres en tête du menu du Dock, avec les actions de l'application ("Prefer high-value custom items for your Dock menu.", dock-menus › Best practices) | `src/ui/context.rs:769-831`
- Aucune variante à modificateur dans les menus contextuels ou du Dock ("Adding a dynamic menu item to contextual or Dock menus can make the item even harder for people to discover.", the-menu-bar › Dynamic menu items) | `src/ui/context.rs:759-831`
- Titre court du panneau, sans le nom du shell ("Don't title windows with your app name.", toolbars › Titles) | `src/ui/panel.rs:964`, « Réglages » (repéré par grep)
- Recherche en tête des Réglages, dans l'esprit du champ du menu Aide | `src/ui/panel.rs:920` (repéré par grep)
- Débordement automatique de la barre ("define which items move to the overflow menu as the toolbar becomes narrower", toolbars › Best practices) | `src/ui/plugins/bar/layout.rs:105-118`, `277-299` ; `src/ui/plugins/bar/popover/bar_menus.rs:10-66`
- Info-bulle sur un bouton à texte réduit ("the system displays a tooltip after people hover over a button for a moment", buttons › Content) | `src/ui/panel.rs:540-551`
- Un menu sans entrée reste ouvrable ("If all of a menu's items are unavailable, the menu itself needs to remain available", menus › Labels) | `src/ui/plugins/bar/popover.rs:1486-1496`

### À vérifier sur un rendu
- Longs libellés (dbusmenu, Lua) : largeur fixe de 328 px et libellés sans retour à la ligne (`src/ui/plugins/bar/popover.rs:26` ; `src/ui/menu.rs:320`, `329`). Troncature, débordement, ou ajustement par `fit_popover` (`src/ui/plugins/bar/popover.rs:1150`) ?
- Coche de secours « • » quand l'icône symbolique manque : se voit-elle avec le thème d'icônes installé (`src/ui/menu.rs:287-294` ; `src/ui/plugins/bar/popover.rs:1528-1535`) ?
- Lisibilité d'une entrée grisée à 0,35 sur le verre (`src/ui/menu.rs:96`).
- Région de touche des modules de barre, 28 px de haut (`src/ui/plugins/bar/surface.rs:1784`). La HIG fixe sa propre barre des menus à 24 pt : la cote de 44 × 44 pt ne s'y applique pas d'évidence ; à juger au trackpad. Aucun écart revendiqué.
- Densité perçue du menu du dock au-delà de huit fenêtres (`src/ui/context.rs:769-789`).
- Barre masquée en plein écran puis révélée au bord supérieur ("the menu bar typically hides until they reveal it by moving the pointer to the top of the screen", the-menu-bar › Platform considerations › macOS). Seule l'option `convex.auto_hide` a été repérée (`src/ui/plugins/bar/surface.rs:986`, `src/ui/mod.rs:4124`, par grep) ; la session la déclare « toujours visible ».
- Deux mises en page de confirmation : rangée dans le menu, « Annuler » puis « Confirmer » (`src/ui/plugins/bar/popover.rs:1556-1610`) ; zone sous l'en-tête des Réglages, « Confirmer » puis « Annuler » (`src/ui/panel.rs:577-586`). Cet export ne donne pas de règle d'ordre ; à unifier ou à assumer.
- « Confirmer » répond-il à la touche Entrée dans un menu de la barre ? À l'essai, pour trancher MENU-16.
- Glyphes des modificateurs : la HIG ne les donne pas dans cet export. hyprdesk affiche SUPER, CTRL, ALT, SHIFT (`src/hypr.rs:572`, `1001-1003`). Aucun écart revendiqué.
- Hors HIG, pour mémoire : « Confirmer ? », « Annuler » et « Confirmer » des menus de la barre sont figrés en français (`src/ui/plugins/bar/popover.rs:1551`, `1563`, `1587`), de même que « Par défaut… » et sa confirmation (`src/ui/panel.rs:542`, `549`, `570`, `578`, `582`), bien que `locales/en.toml` porte les traductions (`:372`, `:4065`).
