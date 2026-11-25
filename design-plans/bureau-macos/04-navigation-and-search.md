# Composants : navigation et recherche — macOS d'après la HIG, face à hyprdesk

> Proposition de design, rien d'implémenté. Plan d'ensemble et priorités : `plan-d-action.md` ; méthode et périmètre : `README.md`.

> Référentiel : Human Interface Guidelines d'Apple, export Markdown officiel relevé le 23 novembre 2026. Code : commit `00f6d69` (arbre de travail du 23 novembre 2026, avec des modifications non commitées).

## Première partie — Spécification d'après la HIG

### Résumé

- Périmètre : la page d'index `navigation-and-search` ne contient que deux listes identiques de définitions, pour cinq composants : contrôle de chemin (path control), champ de recherche (search field), barre latérale (sidebar), barre d'onglets (tab bar) et champ à jetons (token field) — "A search field lets people search a collection of content for specific terms they enter." — navigation-and-search › Discussion
- Champ de recherche : icône Search, bouton Clear et texte de substitution ; recherche dès la frappe si possible — "If possible, start search immediately when a person types." — search-fields › Best practices
- Résultats : les plus pertinents en premier, catégorisés, filtrables par une barre de portée ; portée large par défaut — "Default to a broader scope and let people refine it as they need." — search-fields › Scope bars and tokens
- Placement sur macOS et iPadOS : bord terminal (trailing) de la barre d'outils, haut de la barre latérale (cas de Réglages) ou zone dédiée — "Put a search field at the trailing side of the toolbar for many common uses." — search-fields › Platform considerations
- Barre latérale : masquable à la demande mais jamais par défaut, hiérarchie limitée, rien d'essentiel en bas — "In general, show no more than two levels of hierarchy in a sidebar." — sidebars › Best practices
- Barre d'onglets : elle sert à naviguer, ses boutons ne se désactivent ni ne se masquent, ses badges sont réservés au critique ; aucune considération propre à macOS — "Use a tab bar to support navigation, not to provide actions." — tab-bars › Best practices
- Champ à jetons : conversion à la virgule par défaut, suggestions immédiates par défaut — "By default, text people enter turns into a token whenever they type a comma." — token-fields › Best practices
- Absences : aucune de ces six pages ne donne de hauteur, largeur, rayon, durée ou valeur de couleur ; les dimensions d'icônes de barre d'onglets sont renvoyées à Apple Design Resources.

### Path controls (`path-controls`)

**Rôle** : un contrôle de chemin (path control) affiche le chemin, dans le système de fichiers, du fichier ou du dossier sélectionné.
**Anatomie et cotes**
- Deux styles, Standard et Pop up — "There are two styles of path control." — path-controls › Discussion
- Standard : liste linéaire à trois sortes d'éléments — "A linear list that includes the root disk, parent folders, and selected item." — path-controls › Discussion
- Chaque élément du Standard porte une icône et un nom — "Each item appears with an icon and a name." — path-controls › Discussion
- Débordement du Standard : les noms du milieu sont masqués — "If the list is too long to fit within the control, it hides names between the first and last items." — path-controls › Discussion
- Pop up : contrôle apparenté au pop-up button, qui ne montre que l'icône et le nom de l'élément sélectionné — "People can click the item to open a menu containing the root disk, parent folders, and selected item." — path-controls › Discussion
- Hauteur, largeur, espacement, taille d'icône, rayon : non chiffrés dans la HIG.
**Règles**
- **Emplacement**
  - Dans le corps de la fenêtre, jamais dans son cadre — "Use a path control in the window body, not the window frame." — path-controls › Best practices
  - Ni dans une barre d'outils ni dans une barre d'état — "Path controls aren’t intended for use in toolbars or status bars." — path-controls › Best practices
- **Mode éditable** (facultatif, dans les deux styles)
  - Glisser un élément sur le contrôle le sélectionne et affiche son chemin — "If you make the control editable, people can drag an item onto the control to select the item and display its path in the control." — path-controls › Discussion
  - En style Pop up, le menu gagne une commande Choose — "If you make the control editable, the menu contains an additional Choose command that people can use to select an item and display it in the control." — path-controls › Discussion
**Spécifique macOS**
- Composant exclusif à macOS — "Not supported in iOS, iPadOS, tvOS, visionOS, or watchOS." — path-controls › Platform considerations
- Cas de référence : la barre de chemin du Finder, en bas de la fenêtre — "For example, choosing View > Show Path Bar in the Finder displays a path bar at the bottom of the window." — path-controls › Discussion
- Elle suit la sélection — "It shows the path of the selected item, or the path of the window’s folder if nothing is selected." — path-controls › Discussion
- Elle appartient au corps de la fenêtre, pas à la barre d'état — "Note that the path control in the Finder appears at the bottom of the window body, not in the status bar." — path-controls › Best practices
- Souris : clic pour ouvrir le menu (Pop up), glisser-déposer (mode éditable). Clavier : aucun raccourci ni comportement de focus n'est décrit.
**À transposer dans un shell Linux**
- Résultats de recherche de fichiers du lanceur : afficher le chemin du résultat sélectionné en liste linéaire icône et nom (racine, dossiers parents, élément), en masquant les noms intermédiaires quand la largeur manque.
- Par analogie avec la barre d'état, ne pas loger ce chemin dans la barre du bureau : le placer dans le corps du panneau de résultats.
- Prévoir la variante Pop up (bouton ouvrant le menu des ancêtres) quand la largeur est comptée.
- Si le contrôle est éditable, accepter le glisser-déposer d'un élément dessus ; c'est la seule interaction à la souris décrite.
- La HIG ne fixant aucune cote, choisir une hauteur et des espacements propres et les réutiliser partout dans le shell.

### Search fields (`search-fields`)

**Rôle** : un champ de recherche (search field) permet de chercher, dans une collection de contenus, les termes que la personne saisit.
**Anatomie et cotes**
- Trois éléments : icône Search, bouton Clear, texte de substitution — "A search field is an editable text field that displays a Search icon, a Clear button, and placeholder text where people can enter what they are searching for." — search-fields › Discussion
- Deux composants annexes, avant ou après la recherche : barre de portée (scope bar) et jetons (tokens) — "Scope bars and tokens are components you can use to let someone narrow the parameters of a search either before or after they make it." — search-fields › Scope bars and tokens
- Barre de portée — "is a control for filtering and adjusting the scope of a search" — search-fields › Scope bars and tokens
- Jeton : représentation visuelle d'un terme, sélectionnable et modifiable — "acts as a filter for any additional terms in the search" — search-fields › Scope bars and tokens
- Effacement : la HIG ne décrit que le bouton Clear ; son apparence, ses conditions d'apparition et son raccourci clavier ne sont pas précisés.
- Annulation : aucun bouton Annuler ni fermeture par Échap n'est mentionné dans cette page.
- Cotes : hauteur, largeur, rayon, taille du texte de substitution, délai avant recherche : non chiffrés dans la HIG.
**Règles**
- **Texte de substitution**
  - Indiquer ce qui est cherchable — "Use placeholder text to help people know what they can search for." — search-fields › Best practices
  - Rappeler aussi la portée — "Placeholder text can be helpful when you need to reinforce the scope of your search" — search-fields › Best practices
- **Déclenchement**
  - Lancer la recherche dès la frappe si possible — "If possible, start search immediately when a person types." — search-fields › Best practices
  - Les résultats s'affinent avec la saisie — "it provides results that are continuously refined as the text becomes more specific" — search-fields › Best practices
- **Suggestions**
  - Envisager des termes suggérés — "Consider showing suggested search terms." — search-fields › Best practices
  - Recherches récentes avant la saisie, suggestions prédictives pendant — "For example, you can display recent searches before search begins, or predictive search suggestions as a person types." — search-fields › Best practices
- **Résultats**
  - Les plus pertinents en premier — "Provide the most relevant search results first to minimize the need for someone to scroll to find what they’re looking for." — search-fields › Best practices
  - Envisager des catégories — "In addition to prioritizing the most likely results, consider categorizing them to help people find what they want." — search-fields › Best practices
  - Envisager un filtrage, par exemple par une barre de portée dans la zone des résultats — "For example, you can include a scope bar in the search results content area to help people quickly and easily filter search results." — search-fields › Best practices
- **Portée**
  - Barre de portée réservée à des catégories clairement définies — "Use a scope bar to filter among clearly defined search categories." — search-fields › Scope bars and tokens
  - Portée large par défaut, affinée par la personne — "Default to a broader scope and let people refine it as they need." — search-fields › Scope bars and tokens
- **Jetons**
  - Filtrer par des termes ou éléments courants — "Use tokens to filter by common search terms or items." — search-fields › Scope bars and tokens
  - Les associer à des suggestions — "Consider pairing tokens with search suggestions." — search-fields › Scope bars and tokens
**Spécifique macOS**
- Placement et comportement communs à iPadOS et macOS — "The placement and behavior of the search field in iPadOS and macOS is similar." — search-fields › Platform considerations
- Trois emplacements décrits :
  - Bord terminal de la barre d'outils pour la plupart des usages — "Put a search field at the trailing side of the toolbar for many common uses." — search-fields › Platform considerations
  - La sélection reste visible dans la vue de détail pendant qu'on parcourt les résultats — "This placement makes great use of space because it lets people navigate results while keeping their selection visible in the detail view." — search-fields › Platform considerations
  - En haut de la barre latérale quand on filtre son contenu — "Include search at the top of the sidebar when filtering content or navigation there." — search-fields › Platform considerations
  - Cas de Réglages : la recherche fait apparaître des rubriques enfouies — "Apps such as Settings take advantage of search to quickly filter the sidebar and expose sections that may be multiple levels deep" — search-fields › Platform considerations
  - Comme élément de la barre latérale ou de la barre d'onglets, pour une zone dédiée à la découverte — "Include search as an item in the sidebar or tab bar when you want an area dedicated to discovery." — search-fields › Platform considerations
- Focus : dans une zone dédiée, focaliser le champ dès l'arrivée — "consider immediately focusing the field when a person navigates to the area to help them search faster and locate the field more easily" — search-fields › Platform considerations
  - L'exception citée vise l'iPad sans clavier physique — "it’s better to leave the field unfocused to prevent the keyboard from unexpectedly covering the view" — search-fields › Platform considerations
- Redimensionnement : prévoir le comportement du champ quand la fenêtre change de taille — "Account for window resizing with the placement of the search field." — search-fields › Platform considerations
  - Sur Mac, le champ suit la largeur de la fenêtre — "On iPad, the search field fluidly resizes with the app window like it does on Mac." — search-fields › Platform considerations
  - En vue compacte, Notes et Mail placent la recherche au-dessus de la liste — "For example, Notes and Mail place search above the column for the content list when they resize down to a compact view." — search-fields › Platform considerations
- Clavier : aucun raccourci d'ouverture, d'effacement ni de fermeture n'est donné ; la recherche à l'échelle du système est renvoyée à la page Searching, non couverte ici.
**À transposer dans un shell Linux**
- Lanceur d'applications : champ à loupe (icône Search), bouton Clear et texte de substitution ; recherche dès la première frappe, sans attendre Entrée.
- Résultats du lanceur et de la recherche de fichiers : ordre de pertinence, regroupement par catégorie (applications, fichiers, réglages, actions), sans faire défiler pour l'essentiel.
- Suggestions : recherches récentes quand le champ est vide, suggestions prédictives pendant la frappe.
- Portée : filtres par catégorie dans la zone de résultats, portée large par défaut.
- Écran de réglages : champ de recherche en haut de la barre latérale, qui filtre la liste et révèle les rubriques de niveau profond, la rubrique restant visible dans la vue de détail.
- Écrans tactiles : ne pas focaliser le champ si un clavier virtuel recouvrirait les résultats.
- Clavier : la HIG ne donnant ni Échap ni raccourci, définir et documenter les vôtres sans les attribuer à macOS.

### Sidebars (`sidebars`)

**Rôle** : une barre latérale (sidebar) occupe le bord d'en-tête (leading) d'une vue et fait naviguer entre les zones de l'app ou des collections de premier niveau (dossiers, playlists).
**Anatomie et cotes**
- Position : bord leading de la vue — "A sidebar appears on the leading side of a view and lets people navigate between areas of your app or top-level collections of content, like folders and playlists." — navigation-and-search › Discussion
- Encombrement : beaucoup d'espace vertical et horizontal — "A sidebar requires a large amount of vertical and horizontal space." — sidebars › Discussion
- Tailles sur macOS : small, medium, large ; hauteur de ligne, texte et glyphe en dépendent — "A sidebar’s row height, text, and glyph size depend on its overall size, which can be small, medium, or large." — sidebars › Platform considerations › macOS
- Valeurs des trois tailles, largeurs minimale et maximale, rayon, aspect de la ligne sélectionnée : non chiffrés dans la HIG.
- Transparence : aucun taux d'opacité, ni matériau, ni flou n'est donné (voir Règles › Arrière-plan).
**Règles**
- **Contenu et hiérarchie**
  - Au plus deux niveaux — "In general, show no more than two levels of hierarchy in a sidebar." — sidebars › Best practices
  - Au-delà, insérer une liste de contenu entre barre latérale et vue de détail — "When a data hierarchy is deeper than two levels, consider using a split view interface that includes a content list between the sidebar items and detail view." — sidebars › Best practices
  - Regrouper avec des disclosure controls — "Group hierarchy with disclosure controls if your app has a lot of content." — sidebars › Best practices
  - Titres de groupes courts et descriptifs — "use succinct, descriptive labels to title each group" — sidebars › Best practices
  - Laisser personnaliser le contenu — "When possible, let people customize the contents of a sidebar." — sidebars › Best practices
- **Icônes**
  - Symboles familiers (SF Symbols) — "Consider using familiar symbols to represent items in the sidebar." — sidebars › Best practices
  - Symbole personnalisé de préférence à une image bitmap — "rather than using a bitmap image" — sidebars › Best practices
  - Suivre la couleur d'accent choisie par la personne — "make sure your sidebar icons display the color people choose" — sidebars › Best practices
  - Couleur fixe seulement avec parcimonie — "However, if you use them sparingly, fixed colors can help clarify the meaning of an icon or draw attention to it." — sidebars › Best practices
  - Exemple : icône jaune pour se distinguer — "the VIP icon in Mail uses a yellow color to set it apart from other sidebar icons" — sidebars › Best practices
- **Masquage**
  - Envisager de laisser masquer — "Consider letting people hide the sidebar." — sidebars › Best practices
  - Jamais par défaut — "Avoid hiding the sidebar by default to ensure that it remains discoverable." — sidebars › Best practices
- **Arrière-plan**
  - Prolonger le contenu riche sous la barre — "Extend visually rich content beneath the sidebar." — sidebars › Best practices
  - Soit par défilement horizontal, soit par un effet d'extension d'arrière-plan (background extension effect) — "A background extension effect mirrors adjacent content to give the impression of stretching it under the sidebar." — sidebars › Best practices
- **Alternative** : quand l'espace manque — "a more compact control such as a tab bar may provide a better navigation experience" — sidebars › Discussion
**Spécifique macOS**
- Taille fixée par programmation, ou changée par la personne dans Réglages généraux — "You can set the size programmatically, but people can also change it by selecting a different sidebar icon size in General settings." — sidebars › Platform considerations › macOS
- Affichage et masquage — "in macOS, you can include a show/hide button or add Show Sidebar and Hide Sidebar commands to your app’s View menu" — sidebars › Best practices
- Couleur d'accent modifiable par la personne pour toutes les apps — "In macOS, people can change the system accent color, which applies to all apps." — sidebars › Best practices
- La barre flotte dans la couche Liquid Glass, comme les barres d'outils et d'onglets (règle Arrière-plan ci-dessus), avec un choix ouvert entre barre d'onglets et barre latérale — "For many apps, you don’t need to choose between a tab bar or sidebar for navigation; instead, you can adopt a style of tab bar that provides both." — sidebars › Discussion
- Repli automatique quand la fenêtre se redimensionne — "Consider automatically hiding and revealing a sidebar when its container window resizes." — sidebars › Platform considerations › macOS
  - Exemple : une fenêtre Mail rétrécie replie sa barre latérale — "For example, reducing the size of a Mail viewer window can automatically collapse its sidebar, making more room for message content." — sidebars › Platform considerations › macOS
- Rien de critique en bas — "Avoid putting critical information or actions at the bottom of a sidebar." — sidebars › Platform considerations › macOS
  - Motif : le bas de la fenêtre passe souvent hors écran — "People often relocate a window in a way that hides its bottom edge." — sidebars › Platform considerations › macOS
- Recherche placée en haut de la barre latérale : voir Search fields.
- Clavier : aucun raccourci n'est donné ; largeur par défaut et durée d'animation absentes. Non pris en charge sur watchOS.
**À transposer dans un shell Linux**
- Écran de réglages : colonne leading avec icône et libellé par ligne, en trois tailles de rangée (small, medium, large) pilotées par un réglage de taille d'icône ; valeurs à définir, la HIG ne les donne pas.
- Limiter la barre latérale à deux niveaux ; au-delà, insérer une liste intermédiaire.
- Jamais masquée par défaut ; bouton d'affichage et commande de menu ; repli automatique quand la fenêtre rétrécit.
- Couleur des icônes = couleur d'accent du thème ; une couleur fixe seulement pour marquer un élément important.
- Ne rien placer d'essentiel en bas de la colonne.
- Fond translucide : prolonger le contenu dessous ; la HIG ne donne ni flou ni opacité, à régler localement.
- Aperçu des espaces et listes de fenêtres : appliquer les mêmes règles de liste et de hiérarchie.

### Tab bars (`tab-bars`)

**Rôle** : une barre d'onglets (tab bar) fait naviguer entre les sections de premier niveau d'une app.
**Anatomie et cotes**
- Chaque onglet porte une icône et un libellé — "A tab label appears beneath or beside a tab bar icon" — tab-bars › Best practices
- Vue compacte : icônes au-dessus des libellés ; vue régulière : côte à côte — "Tab bar icons appear above tab labels in compact views, whereas in regular views, the icons and labels appear side by side." — tab-bars › Best practices
- La forme dépend de l'appareil et de l'orientation — "the tab bar can be regular or compact, depending on the device and orientation" — tab-bars › Best practices
- Badge (insigne) — "a red oval containing white text and either a number or an exclamation point" — tab-bars › Best practices
- Dimensions des icônes : renvoyées à Apple Design Resources — "for tab bar icon dimensions" — tab-bars › Best practices
- Hauteur, largeur d'onglet, espacement, rayon, taille de police : non chiffrés dans la HIG.
**Règles**
- **Usage**
  - Navigation, pas actions — "Use a tab bar to support navigation, not to provide actions." — tab-bars › Best practices
  - Pour agir sur les éléments de la vue, employer une barre d'outils — "If you need to provide controls that act on elements in the current view" — tab-bars › Best practices
  - L'état de navigation de chaque section est conservé — "while preserving the current navigation state within each section" — tab-bars › Discussion
- **Stabilité**
  - Toujours visible pendant la navigation — "Make sure the tab bar is visible when people navigate to different sections of your app." — tab-bars › Best practices
  - Seule exception : une vue modale — "The exception is when a modal view covers the tab bar, because a modal is temporary and self-contained." — tab-bars › Best practices
  - Jamais désactiver ni masquer un onglet — "Don’t disable or hide tab bar buttons, even when their content is unavailable." — tab-bars › Best practices
  - Section vide : expliquer pourquoi — "If a section is empty, explain why its content is unavailable." — tab-bars › Best practices
- **Nombre**
  - Moins d'onglets, navigation plus facile — "keep in mind that it’s generally easier to navigate among fewer tabs" — tab-bars › Best practices
  - Éviter les onglets de débordement — "Avoid overflow tabs." — tab-bars › Best practices
  - Hiérarchie complexe : barre latérale, ou barre d'onglets qui s'adapte en barre latérale — "Where available, consider a sidebar or a tab bar that adapts to a sidebar as an alternative for an app with a complex information structure." — tab-bars › Best practices
- **Libellés et icônes**
  - Un seul mot si possible — "Use single words whenever possible." — tab-bars › Best practices
  - Symboles pleins privilégiés — "Prefer filled symbols or icons for consistency with the platform." — tab-bars › Best practices
- **Badges**
  - Pour signaler une information critique — "Use a badge to indicate that critical information is available." — tab-bars › Best practices
  - Réservés à l'essentiel — "Reserve badges for critical information so you don’t dilute their impact and meaning." — tab-bars › Best practices
- **Couleur**
  - Éviter une teinte proche du contenu — "Avoid applying a similar color to tab labels and content layer backgrounds." — tab-bars › Best practices
  - Contenu très coloré : monochrome, ou accent nettement distinct — "prefer a monochromatic appearance for tab bars, or choose an accent color with sufficient visual differentiation" — tab-bars › Best practices
**Spécifique macOS**
- Le bloc Platform considerations ne dit rien de particulier — "No additional considerations for macOS." — tab-bars › Platform considerations
- Ce qui vaut donc pour macOS : les règles générales ci-dessus, la page ne distinguant pas les plateformes dans ses Best practices.
- L'onglet More, qui regroupe les onglets cachés, est décrit pour iOS et iPadOS seulement — "If horizontal space limits the number of visible tabs, the trailing tab becomes a More tab in iOS and iPadOS, revealing the remaining items in a separate list." — tab-bars › Best practices
  - Il nuit à la découvrabilité — "The More tab makes it harder for people to reach and notice content on tabs that are hidden, so limit scenarios in your app where this can happen." — tab-bars › Best practices
- La page ne dit ni quelle forme (régulière ou compacte) s'applique au Mac, ni combien d'onglets tiennent dans une fenêtre ; aucune touche ni interaction souris n'est décrite.
**À transposer dans un shell Linux**
- Barre du bureau et aperçu des espaces : les sélecteurs d'espaces ou de modes relèvent de la navigation ; les actions (verrouiller, réglages rapides) vont dans des boutons ou des menus.
- Ne jamais retirer ni griser un onglet quand son contenu est vide : afficher un état vide expliqué.
- Un mot par libellé, icône pleine, état de chaque section conservé quand on change d'onglet.
- Badges : nombre dans un ovale rouge à texte blanc, réservé à l'essentiel ; la HIG renvoie à la page Notifications, non couverte ici.
- Pas d'onglet More documenté pour macOS : limiter le nombre d'onglets pour qu'ils tiennent tous.
- Apparence monochrome sur un contenu très coloré.

### Token fields (`token-fields`)

**Rôle** : un champ à jetons (token field) est un champ texte qui convertit le texte saisi en jetons (tokens), faciles à sélectionner et à manipuler.
**Anatomie et cotes**
- Le jeton se sélectionne comme un seul élément — "People can select these recipient tokens and drag to reorder them or move them into a different field." — token-fields › Discussion
- Liste de suggestions pendant la saisie — "You can configure a token field to present people with a list of suggestions as they enter text into the field." — token-fields › Discussion
- Menu contextuel propre à chaque jeton — "An individual token can also include a contextual menu that offers information about the token or editing options." — token-fields › Discussion
- Forme, hauteur, rayon et espacement des jetons : non chiffrés dans la HIG ; la page Search fields ne dit qu'ils reçoivent un traitement visuel qui les encapsule.
**Règles**
- **Conversion**
  - Virgule par défaut — "By default, text people enter turns into a token whenever they type a comma." — token-fields › Best practices
  - Raccourcis supplémentaires possibles, dont Return — "You can specify additional shortcuts, such as pressing Return, that also invoke this action." — token-fields › Best practices
- **Suggestions**
  - Immédiates par défaut — "By default, suggestions appear immediately." — token-fields › Best practices
  - Délai réglable — "Consider customizing the delay the system uses before showing suggested tokens." — token-fields › Best practices
  - Trop tôt, elles distraient — "However, suggestions that appear too quickly may distract people while they’re typing." — token-fields › Best practices
- **Menu contextuel**
  - Apporter une valeur ajoutée — "Add value with a context menu." — token-fields › Best practices
**Spécifique macOS**
- Composant exclusif à macOS — "Not supported in iOS, iPadOS, tvOS, visionOS, and watchOS." — token-fields › Platform considerations
- Cas de référence : les champs d'adresse à l'écriture d'un message dans Mail — "For example, Mail uses token fields for the address fields in the compose window." — token-fields › Discussion
- Le nom saisi devient jeton — "As people enter recipients, Mail converts the text that represents each recipient’s name into a token." — token-fields › Discussion
- Souris : glisser pour réordonner ou déplacer un jeton vers un autre champ ; clic droit pour le menu contextuel (exemple de Mail : modifier le nom, marquer VIP, voir la fiche contact).
- Clavier : virgule par défaut, Return en option ; aucune valeur de délai n'est donnée.
**À transposer dans un shell Linux**
- Recherche du lanceur : convertir les filtres choisis (catégorie, dossier, contact) en jetons sélectionnables et effaçables un à un dans le champ.
- Déclencheur de jeton : virgule par défaut, Return en option.
- Suggestions affichées pendant la frappe ; en choisir une insère un jeton.
- Menu contextuel au clic droit sur un jeton (supprimer, modifier).
- La HIG ne donne ni rayon ni hauteur : ne pas inventer de cotes macOS.

### Valeurs de référence

| Propriété | Valeur | Source (slug › section) |
|---|---|---|
| Styles de contrôle de chemin | 2 : Standard, Pop up | path-controls › Discussion |
| Éléments d'un contrôle de chemin | disque racine, dossiers parents, élément sélectionné | path-controls › Discussion |
| Placement d'un contrôle de chemin | corps de la fenêtre ; pas de barre d'outils ni de barre d'état | path-controls › Best practices |
| Barre de chemin du Finder | bas du corps de la fenêtre ; View > Show Path Bar | path-controls › Best practices |
| Éléments d'un champ de recherche | icône Search, bouton Clear, texte de substitution | search-fields › Discussion |
| Portée par défaut | la plus large possible | search-fields › Scope bars and tokens |
| Emplacements du champ de recherche (macOS, iPadOS) | bord trailing de la barre d'outils ; haut de la barre latérale ; élément de la barre latérale ou de la barre d'onglets | search-fields › Platform considerations |
| Tailles de barre latérale (macOS) | small, medium, large ; réglées via la taille d'icône de barre latérale dans General settings | sidebars › Platform considerations › macOS |
| Profondeur de hiérarchie d'une barre latérale | au plus 2 niveaux | sidebars › Best practices |
| Couleur des icônes de barre latérale | couleur d'accent (accent color) de l'app par défaut ; couleur d'accent système sur macOS | sidebars › Best practices |
| Masquage de la barre latérale | à la demande (bouton ou commandes Show Sidebar et Hide Sidebar du menu View) ; jamais par défaut | sidebars › Best practices |
| Disposition d'un onglet | vue compacte : icône au-dessus du libellé ; vue régulière : icône et libellé côte à côte | tab-bars › Best practices |
| Badge d'onglet | ovale rouge, texte blanc, nombre ou point d'exclamation | tab-bars › Best practices |
| Considérations propres à macOS pour la barre d'onglets | aucune | tab-bars › Platform considerations |
| Conversion en jeton | virgule par défaut ; Return en option | token-fields › Best practices |
| Délai des suggestions de jetons | immédiat par défaut ; réglable, valeur non donnée | token-fields › Best practices |
| Hauteurs, largeurs, rayons, durées, couleurs | non chiffrés dans la HIG | les six pages |

## Seconde partie — hyprdesk aujourd'hui, écarts et actions

### Ce que hyprdesk fait aujourd'hui

| Surface | Ce qui existe | Valeurs clés | Preuves |
|---|---|---|---|
| Champ de recherche du lanceur, mode applications | Pastille avec loupe à gauche, texte de substitution, anneau de focus de 2 px ; la frappe reclasse sans attendre Entrée | loupe 18 px (`edit-find`, `system-search`, secours `⌕`) ; marge gauche 44 ; texte 20 | `src/ui/plugins/launcher/grid.rs:145-223` |
| Champ de recherche, modes `:f`, `:clip`, `:2fa` et `:mdp` | Même carte, champ nu ; le mode vient d'un préfixe tapé | substitution « Rechercher un fichier ou un dossier… » en mode fichiers | `src/ui/search.rs:103-113`, `1126-1136`, `1161-1171`, `1370-1380` |
| Résultats | 8 visibles sans défilement ; classement flou bonifié par la fréquence de lancement ; première ligne choisie d'office ; sélection clavier en boucle ; un clic lance | `SHOWN = 8` ; rangée 46 px | `src/ui/search.rs:32`, `208-245`, `555-564`, `1299-1362` |
| Portée | Barre de portée dans la zone de résultats : catégories d'applications et « Toutes » ; en mode fichiers, natures (Tout, Fichiers, Dossiers, Liens) puis catégories et « Toutes » ; filtre appliqué côté démon d'index | portée large à l'ouverture (`category: None`) | `src/ui/plugins/launcher/grid.rs:103-129`, `385-410` ; `src/ui/search.rs:404`, `1216-1297` |
| États | Avant la frappe : applications les plus lancées, ou épinglées, ou rien (réglage) ; vide expliqué ; index en démarrage, en chargement, en cours (nombre), indisponible (erreur) | `EmptyView` : `Suggestions`, `Favorites`, `Field` | `src/ui/search.rs:264-314`, `1189-1211` ; `src/ui/plugins/launcher/grid.rs:411-413`, `545-547` |
| Clavier du lanceur | Échap ferme ; flèches haut et bas déplacent, gauche et droite en grille ; Entrée lance la sélection | — | `src/ui/mod.rs:5356-5374` |
| Barre latérale des Réglages | Colonne à gauche : titres de groupes, paquets, onglets du seul paquet choisi ; paquets coupés absents ; défilement vertical | 166 px avec libellés, 46 px en icônes seules sous 640 px ; tiroir 936 px, pleine largeur d'écran sous 1200 px | `src/ui/panel.rs:126`, `639-645`, `712`, `815-901` ; `src/ui/plugins/mod.rs:122-175` |
| Recherche des Réglages | Champ dans la ligne d'actions de l'en-tête ; résultats par paquet, avec le bloc de la section ; Entrée ouvre le premier ; sur la page ouverte, bandeau « Résultats » et « Effacer », réglages surlignés | 230 px, puis pleine largeur sous 880 px ; 120 résultats affichés au plus | `src/ui/panel.rs:918-935`, `994-1001`, `1205-1250`, `1252-1428` ; `src/ui/settings_search.rs:363-387` |
| Segments d'espaces (centre de contrôle Continuité) | Cinq cellules à largeur égale prises autour de l'espace actif, puis un bouton « … » vers la liste complète dès plus de cinq espaces ; dans la barre, losange d'urgence à côté du numéro d'espace | fenêtre de 5 ; cellule de 43 px de haut | `src/ui/plugins/bar/popover/continuity.rs:1249-1305` ; `src/ui/plugins/bar/surface.rs:38-39` |

### Écarts

Classés du plus important au moins important. Les chemins sont relatifs à la racine du dépôt.

| # | Composant | Nature | Force | Règle HIG (slug › section) | Réel dans hyprdesk (preuve) | Action proposée | Effort | Confiance |
|---|---|---|---|---|---|---|---|---|
| NAV-01 | Champ de recherche du lanceur | ÉCART | Définition du composant et tableau de référence | « A search field is an editable text field that displays a Search icon, a Clear button, and placeholder text » — search-fields › Discussion ; « Éléments d'un champ de recherche : icône Search, bouton Clear, texte de substitution » — search-fields › Valeurs de référence | Aucun bouton Clear dans les quatre modes. En mode applications, `launcher_field` ne pose que la loupe à gauche (`src/ui/plugins/launcher/grid.rs:162-223`) ; dans les trois autres modes le champ est un `text_input` nu (`src/ui/search.rs:1126-1136`, `1161-1171`, `1370-1380`). Effacer exige de vider le champ à la main. | Ajouter un bouton Clear à droite du champ, affiché dès que la requête n'est pas vide, qui la vide et rend le focus. | S | haute |
| NAV-02 | Champ de recherche des Réglages | ÉCART | Définition du composant et tableau de référence | Mêmes citations — search-fields › Discussion ; search-fields › Valeurs de référence | Champ nu dans l'en-tête (`src/ui/panel.rs:920-935`). Le seul effacement est le bouton texte « Effacer » du bandeau qui suit l'ouverture d'un résultat (`src/ui/panel.rs:1229-1235`), absent de la liste des résultats (`src/ui/panel.rs:1252-1428`). | Ajouter un bouton Clear dans le champ, affiché dès que la requête n'est pas vide. | S | haute |
| NAV-03 | Icône Search | ÉCART | Définition du composant et tableau de référence | Mêmes citations — search-fields › Discussion ; search-fields › Valeurs de référence | La loupe n'est dessinée que par `launcher_field`, en mode applications (`src/ui/plugins/launcher/grid.rs:193-212`). Dans les modes `:f`, `:clip`, `:2fa` et `:mdp`, la carte prend un champ sans icône (`src/ui/search.rs:1126-1136`, `1161-1171`, `1370-1380`). | Faire passer les quatre modes par `launcher_field`, pour garder la loupe. | S | haute |
| NAV-04 | Portée de la recherche du lanceur | PISTE | « Default to… » (forme impérative) ; « consider categorizing » | « Default to a broader scope and let people refine it as they need. » — search-fields › Scope bars and tokens ; « consider categorizing them to help people find what they want » — search-fields › Best practices | La barre de portée ne filtre qu'à l'intérieur d'un mode. Le mode lui-même vient d'un préfixe tapé (`src/ui/search.rs:103-113`), annoncé par le seul texte de substitution (`src/ui/plugins/launcher/grid.rs:163`). Sans préfixe, une requête ne cherche que les applications et répond « Aucune application ne correspond. » (`src/ui/plugins/launcher/grid.rs:411-413`). | Ouvrir la recherche sur applications et fichiers ensemble, les puces de portée servant à resserrer. | L | moyenne |
| NAV-05 | Jetons de recherche | ABSENT | « Use tokens… » ; « Consider pairing tokens… » ; « By default… » | « Use tokens to filter by common search terms or items. » ; « Consider pairing tokens with search suggestions. » — search-fields › Scope bars and tokens ; « By default, text people enter turns into a token whenever they type a comma. » — token-fields › Best practices | Aucun jeton. Le filtre de mode est un préfixe brut dans le champ (`src/ui/search.rs:103-113`) ; nature et catégorie sont des puces hors du champ (`src/ui/search.rs:1216-1297`, `src/ui/plugins/launcher/grid.rs:385-410`). Dans le lanceur, le mot `token` ne désigne que des jetons de design. | Représenter le filtre en cours (mode, nature, catégorie) par un jeton dans le champ, sélectionnable et effaçable d'un clic. | M | haute |
| NAV-06 | Chemin d'un résultat de fichier | ABSENT | Composant macOS sans équivalent | « A linear list that includes the root disk, parent folders, and selected item. » ; « Each item appears with an icon and a name. » ; « If the list is too long to fit within the control, it hides names between the first and last items. » — path-controls › Discussion | Chaque résultat n'affiche que « taille · dossier parent », le parent coupé à 34 caractères par la gauche (`src/ui/search.rs:1304-1323`). Aucun chemin en liste nulle part ailleurs dans le lanceur. | Afficher, pour le résultat choisi, le chemin en liste icône et nom (racine, dossiers parents, élément), noms du milieu masqués quand la largeur manque. | M | haute |
| NAV-07 | Recherche des Réglages : colonne de navigation | PISTE | « take advantage of search to quickly filter the sidebar » | « Apps such as Settings take advantage of search to quickly filter the sidebar and expose sections that may be multiple levels deep » ; « Include search at the top of the sidebar when filtering content or navigation there. » — search-fields › Platform considerations | Pendant la recherche, les paquets sans correspondance restent affichés, estompés à 35 % (`src/ui/panel.rs:728-732`) ; les autres portent un compte (`src/ui/panel.rs:778-791`). Le champ siège dans l'en-tête (`src/ui/panel.rs:920-935`), pas en tête de la colonne (`src/ui/panel.rs:893-901`). | Masquer les paquets sans correspondance pendant la recherche. | S | moyenne |
| NAV-08 | Recherches récentes et termes suggérés | PISTE | « Consider » | « Consider showing suggested search terms. For example, you can display recent searches before search begins, or predictive search suggestions as a person types. » — search-fields › Best practices | Avant la frappe, seules des applications sont proposées : les plus lancées, puis les épinglées, ou rien selon le réglage (`src/ui/search.rs:264-314`). Le lanceur ne garde que des lancements, pas de requêtes (`src/search.rs:97-139`). Aucune recherche récente affichée, aucun terme suggéré pendant la frappe. | Afficher les dernières recherches et les derniers fichiers ouverts quand le champ est vide. | M | haute |
| NAV-09 | Masquage de la colonne de navigation | PISTE | « Consider » | « Consider letting people hide the sidebar. » ; « in macOS, you can include a show/hide button or add Show Sidebar and Hide Sidebar commands to your app's View menu » — sidebars › Best practices | La colonne est toujours affichée : 166 px, ou 46 px en icônes seules (`src/ui/panel.rs:893-901`). L'en-tête n'offre que la fermeture du tiroir (`src/ui/panel.rs:903-915`). | Ajouter une commande d'affichage et de masquage de la colonne de navigation. | S | haute |
| NAV-10 | Taille des rangées de la colonne | ABSENT | Possibilité de macOS (description de plateforme) | « A sidebar's row height, text, and glyph size depend on its overall size, which can be small, medium, or large. » ; « people can also change it by selecting a different sidebar icon size in General settings » — sidebars › Platform considerations › macOS | La taille ne dépend que de la largeur de l'écran : libellés ou icônes seules, sans réglage (`src/ui/panel.rs:639-645`, `712`, `895`). Aucun réglage de taille de navigation dans `src/ui`, `src/config.rs` ni `src/theme_engine`. | Offrir trois tailles de rangée (petite, moyenne, grande) dans les Réglages. | M | moyenne |
| NAV-11 | Débordement des segments d'espaces | PISTE | « Avoid » | « Avoid overflow tabs. » ; « The More tab makes it harder for people to reach and notice content on tabs that are hidden » — tab-bars › Best practices | Le centre de contrôle montre cinq espaces pris autour de l'actif, puis un bouton « … » vers la liste complète dès qu'il y en a plus de cinq (`src/ui/plugins/bar/popover/continuity.rs:1273-1305`). | Montrer tous les espaces permanents dans la carte (rangée élargie ou grille), sans bouton de débordement. | S | moyenne |
| NAV-12 | Rail d'icônes seules | PISTE | Analogie (« A tab label appears beneath or beside a tab bar icon ») | tab-bars › Best practices, étendue par analogie à une navigation sans libellel | Sous 640 px, chaque entrée est une icône de 18 px, sans libellé ni info-bulle (`src/ui/panel.rs:733-743`, `792-808`). | Donner une info-bulle (libellé du paquet) à chaque entrée du rail. | S | faible |
| NAV-13 | Couleur des icônes de la colonne | IDENTITÉ | « make sure your sidebar icons display the color people choose », au sein d'un « Consider » | sidebars › Best practices | Icône dans la couleur d'accent si le paquet est choisi, dans celle du texte sinon (`src/ui/panel.rs:739`). Documenté : « L'accent sert à la sélection et aux actions » (`DESIGN.md:5`). | Aucune : à signaler seulement. | — | haute |
| NAV-14 | Réduction aux icônes sur petit écran | IDENTITÉ | — | « Consider automatically hiding and revealing a sidebar when its container window resizes. » — sidebars › Platform considerations › macOS | Sous 640 px, la colonne passe en icônes seules (`src/ui/panel.rs:712`, `895`). Documenté (`DESIGN.md:16`). Suit la recommandation, sans masquer la navigation. | Aucune : à signaler seulement. | — | haute |
| NAV-15 | Champ de recherche sous les résultats (style Convex) | IDENTITÉ | — | search-fields › Discussion (aucune règle de placement dans une carte) | En style Convex, les résultats passent au-dessus du champ (`src/ui/search.rs:1552`). Documenté (`DESIGN.md:19`). | Aucune : à signaler seulement. | — | haute |

### Ce qui est déjà conforme

- Recherche dès la frappe : « If possible, start search immediately when a person types. » (search-fields › Best practices) | `on_input` puis reclassement à chaque `SMsg::Query` — `src/ui/search.rs:982-1016`, `src/ui/plugins/launcher/grid.rs:167`.
- Résultats les plus pertinents en premier, sans défilement : « Provide the most relevant search results first to minimize the need for someone to scroll » (search-fields › Best practices) | classement flou bonifié par l'usage, 8 résultats — `src/ui/search.rs:32`, `208-245`.
- Portée large par défaut, catégories clairement définies, barre de portée dans la zone des résultats : « Use a scope bar to filter among clearly defined search categories. » ; « you can include a scope bar in the search results content area » (search-fields › Scope bars and tokens ; › Best practices) | puces « Tout » et « Toutes » choisies d'office — `src/ui/search.rs:1216-1297`, `src/ui/plugins/launcher/grid.rs:385-410`.
- Texte de substitution qui dit ce qui est cherchable : « Use placeholder text to help people know what they can search for. » (search-fields › Best practices) | « Rechercher une application… (:f, :clip, :2fa, :mdp) » — `src/ui/plugins/launcher/grid.rs:163` ; aide de recherche de fichiers — `src/ui/search.rs:1198`.
- Champ focalisé à l'arrivée : « consider immediately focusing the field when a person navigates to the area » (search-fields › Platform considerations) | `SMsg::Shown` donne le focus — `src/ui/search.rs:970-981` ; focus rendu au changement de vue — `src/ui/search.rs:1008`.
- Redimensionnement du champ : « Account for window resizing with the placement of the search field. » (search-fields › Platform considerations) | 230 px sur une ligne, pleine largeur sous 880 px, au-dessus de la colonne et de la vue — `src/ui/panel.rs:918-935`, `958-980`, `1037`.
- Colonne jamais masquée par défaut : « Avoid hiding the sidebar by default to ensure that it remains discoverable. » (sidebars › Best practices) | colonne toujours rendue — `src/ui/panel.rs:893-901`.
- Deux niveaux au plus, groupes titrés : « show no more than two levels of hierarchy in a sidebar » ; « use succinct, descriptive labels to title each group » (sidebars › Best practices) | paquets, puis onglets du seul paquet choisi ; dix titres de groupes — `src/ui/panel.rs:820-890`, `src/ui/plugins/mod.rs:122-175`.
- Rien d'essentiel en bas : « Avoid putting critical information or actions at the bottom of a sidebar. » (sidebars › Platform considerations › macOS) | la colonne ne contient que la navigation ; fermeture en en-tête — `src/ui/panel.rs:893-915`.
- Contenu personnalisable : « When possible, let people customize the contents of a sidebar. » (sidebars › Best practices) | paquets coupés retirés de la colonne — `src/ui/panel.rs:820-826`.
- La sélection reste visible pendant qu'on parcourt les résultats : « keeping their selection visible in the detail view » (search-fields › Platform considerations) | page ouverte avec bandeau et réglages surlignés — `src/ui/panel.rs:994-1000`, `1205-1250`.
- Segments d'espaces : navigation et non action, largeur égale, libellés d'un mot : « Use a tab bar to support navigation, not to provide actions. » ; « In general, keep segment size consistent » ; « Use single words whenever possible. » (tab-bars › Best practices) | `src/ui/plugins/bar/popover/continuity.rs:1273-1294`.
- Information critique signalée par une forme : « Use a badge to indicate that critical information is available. » (tab-bars › Best practices) | losange d'urgence à côté du numéro d'espace — `src/ui/plugins/bar/surface.rs:38-39`.
- Symboles vectoriels : « Consider using familiar symbols… rather than using a bitmap image » (sidebars › Best practices) | icônes Freedesktop et glyphes de 18 px — `src/ui/panel.rs:733-743`.
- Couleur fixe parcimonieuse : « fixed colors can help clarify the meaning of an icon » (sidebars › Best practices) | fiole cyan de la configuration avancée — `src/ui/plugins/mod.rs:222`, `src/ui/panel.rs:746-748`.
- Clavier : la HIG ne donne ni Échap ni raccourci ; ceux de hyprdesk sont définis et montrés (« Fermer · Échap ») — `src/ui/mod.rs:5356-5374`, `src/ui/panel.rs:903-915`.

### À vérifier sur un rendu

- Lisibilité des noms d'espaces coupés à 4 caractères dans les cellules du centre de contrôle (`src/ui/plugins/bar/popover/continuity.rs:1278`), avec des noms longs.
- Densité des deux rangées de puces (natures, puis catégories) au-dessus des résultats de fichiers, dans une carte de 680 px (`src/ui/search.rs:1216-1297`).
- Entrées estompées à 35 % pendant la recherche des Réglages : se lisent-elles comme désactivées plutôt que comme sans objet (`src/ui/panel.rs:728-732`) ?
- Rail d'icônes seules : reconnaissance des icônes à 18 px sur 46 px de large (`src/ui/panel.rs:733-743`).
- Verre du tiroir : le fond se prolonge-t-il sous la colonne (« Extend visually rich content beneath the sidebar ») (`src/ui/panel.rs:1059-1076`) ?
- Souris et sélection : la sélection suit le clavier seulement (`src/ui/search.rs:1328`, `1348-1356`) ; Entrée lance la ligne choisie au clavier même si le pointeur en survole une autre. Aucune règle HIG ; à essayer en session.
