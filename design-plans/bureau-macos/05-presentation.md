# Composants : présentation — macOS d'après la HIG, face à hyprdesk

> Proposition de design, rien d'implémenté. Plan d'ensemble et priorités : `plan-d-action.md` ; méthode et périmètre : `README.md`.

> Référentiel : Human Interface Guidelines d'Apple, export Markdown officiel relevé le 23 novembre 2026. Code : commit `00f6d69` (arbre de travail du 23 novembre 2026, avec des modifications non commitées).

## Première partie — Spécification d'après la HIG

### Résumé

- La famille Presentation (présentation) compte huit composants qui affichent du contenu au-dessus ou à côté du contexte courant : fenêtres (windows), panneaux (panels), popovers, feuilles (sheets), alertes (alerts), feuilles d'action (action sheets), vues défilantes (scroll views) et contrôles de pages (page controls).
- Disponibilité sur macOS : les panneaux n'existent que sur macOS ; fenêtres, popovers, feuilles, alertes et vues défilantes y ont un bloc de considérations propres ; les feuilles d'action n'y ont aucune considération supplémentaire ; les contrôles de pages n'y sont pas pris en charge.
- Modalité : la feuille (toujours modale sur macOS), l'alerte et la feuille d'action sont modales ; le popover est transitoire et non modal ; le panneau flotte au-dessus des fenêtres et disparaît quand son app devient inactive.
- La HIG ne publie aucune cote pour ces composants : ni hauteur de barre de titre, ni taille des boutons de fenêtre, ni rayon de coin, ni largeur de barre de défilement, ni durée d'animation.
- Les rares valeurs sont des dénombrements et des positions : trois états de fenêtre, une seule fenêtre key à l'écran, une feuille ou un popover à la fois, trois boutons au plus par alerte, un titre d'alerte de deux lignes au plus, environ dix points par contrôle de pages.
- Le plein écran n'est pas traité dans ces pages, hormis deux renvois (feuilles, panneaux) ; la barre des menus et le Dock n'y apparaissent que par le menu Window, le menu File et l'icône du Dock.
- Chaque puce issue de la HIG porte sa preuve : citation anglaise mot pour mot, puis page (slug) et section. Les puces « À transposer » sont des propositions pour un shell Linux, qui ne viennent pas de la HIG.

### Présentation — Presentation (`presentation`)

- **Rôle** — Page d'index de la famille : elle définit en une phrase chacun des huit composants et renvoie à leur page.
- **Anatomie et cotes** — Non chiffré dans la HIG : l'index se réduit à deux listes de définitions (Discussion et Topics), sans illustration ni valeur.
- **Règles** — Aucune recommandation normative sur cette page ; les définitions qu'elle fixe sont citées sur la puce Rôle de chaque section ci-dessous.
- **Spécifique macOS** — Rien : l'index ne mentionne ni macOS, ni le Dock, ni la barre des menus, ni la souris, ni de raccourcis. La disponibilité de chaque composant figure dans sa propre section.
- **À transposer dans un shell Linux** — **Vocabulaire** : garder la correspondance fenêtre (window), panneau (panel), popover, feuille (sheet), alerte (alert), feuille d'action (action sheet), vue défilante (scroll view), contrôle de pages (page control) pour nommer sans ambiguïté les surfaces de hyprdesk.
  - **Classement** : rattacher chaque surface du shell à l'un de ces huit composants avant de la comparer (un menu de barre à un popover, un dialogue de confirmation à une alerte), pour savoir quelles règles lui sont applicables.
  - **Composant sans modèle** : les contrôles de pages n'ont pas d'équivalent macOS dans la HIG ; une surface qui s'en inspire ne peut pas se réclamer d'une conformité au bureau macOS.

### Fenêtres — Windows (`windows`)

- **Rôle** — Une fenêtre (window) présente les vues et les composants d'une app ou d'un jeu — "A window presents UI views and components in your app or game." — presentation › Discussion
- **Anatomie et cotes** — Sur macOS, les fenêtres délimitent visuellement le contenu de l'app et rendent possible le multitâche dans l'app et entre apps — "windows help define the visual boundaries of app content and separate it from other areas of the system, and enable multitasking workflows both within and between apps" — windows › Discussion
  - Deux types conceptuels : la fenêtre principale (primary window) porte la navigation et le contenu principaux de l'app — "window presents the main navigation and content of an app, and actions associated with them" — windows › Discussion
  - La fenêtre auxiliaire (auxiliary window) est consacrée à une tâche ou à une zone unique, sans navigation vers d'autres zones de l'app, avec un bouton de fermeture — "an auxiliary window doesn’t allow navigation to other app areas, and it typically includes a button people use to close it after completing the task" — windows › Discussion
  - Le système fournit le cadre (frame) et les contrôles de fenêtre (window controls), qui servent à ouvrir, fermer, redimensionner et déplacer — "Windows include system-provided interface elements such as frames and window controls that let people open, close, resize, and relocate them." — windows › Discussion
  - Non chiffré dans la HIG : hauteur du cadre et de la barre de titre, taille et espacement des boutons, rayon des coins, largeur et hauteur minimales ou maximales, épaisseur de la bordure de redimensionnement, couleurs exactes, durées d'animation. Non traités sur cette page : plein écran, réduction dans le Dock, aperçu des fenêtres, espaces.
- **Règles** — *Taille* : les fenêtres doivent s'adapter souplement à toutes les tailles, pour le multitâche et le multi-fenêtrage — "Make sure that your windows adapt fluidly to different sizes to support multitasking and multiwindow workflows." — windows › Best practices
  - *Ouverture* : ouvrir du contenu dans une fenêtre séparée aide au multitâche et préserve le contexte — "Opening content in a separate window is great for helping people multitask or preserve context." — windows › Best practices
  - N'ouvrir une nouvelle fenêtre par défaut que si cela a un sens pour l'app — "Avoid opening new windows as default behavior unless it makes sense for your app." — windows › Best practices
  - L'excès de fenêtres encombre et rend la navigation dans l'app plus confuse — "opening new windows excessively creates clutter and can make navigating your app more confusing" — windows › Best practices
  - *Apparence système* : ne pas fabriquer de cadre ou de contrôle de fenêtre personnalisé, ni chercher à reproduire l'apparence du système — "Avoid making custom window frames or controls, and don’t try to replicate the system-provided appearance." — windows › Best practices
  - Une imitation imparfaite de l'apparence et du comportement du système donne à l'app un air cassé — "Doing so without perfectly matching the system’s look and behavior can make your app feel broken." — windows › Best practices
  - *Vocabulaire* : employer le terme window dans les textes adressés à l'utilisateur — "Use the term *window* in user-facing content." — windows › Best practices
- **Spécifique macOS** — la page n'existe ni sur iOS, ni sur tvOS, ni sur watchOS — "Not supported in iOS, tvOS, or watchOS." — windows › Platform considerations
  - *Contexte* : sur macOS, plusieurs apps tournent à la fois et l'on passe souvent d'une fenêtre à l'autre sur un même bureau — "In macOS, people typically run several apps at the same time, often viewing windows from multiple apps on one desktop and switching frequently between different windows" — windows › macOS
  - Les manipulations attendues sont le déplacement, le redimensionnement, la réduction et la mise en évidence des fenêtres — "moving, resizing, minimizing, and revealing the windows to suit their work style" — windows › macOS
  - *Anatomie* : une fenêtre macOS se compose d'un cadre (frame) et d'une zone de corps (body area) — "A macOS window consists of a frame and a body area." — windows › macOS window anatomy
  - Elle se déplace en faisant glisser le cadre et, le plus souvent, se redimensionne en faisant glisser ses bords — "People can move a window by dragging the frame and can often resize the window by dragging its edges." — windows › macOS window anatomy
  - Le cadre se trouve au-dessus de la zone de corps et peut contenir les contrôles de fenêtre et une barre d'outils (toolbar) — "appears above the body area and can include window controls" — windows › macOS window anatomy
  - Une barre inférieure (bottom bar), partie du cadre située sous le contenu, est possible mais rare — "In rare cases, a window can also display a bottom bar, which is a part of the frame that appears below body content." — windows › macOS window anatomy
  - *États* : une fenêtre macOS a l'un de trois états, main, key ou inactive — "A macOS window can have one of three states:" — windows › macOS window states
  - Main : la fenêtre de premier plan que l'on consulte ; une seule par app — "The frontmost window that people view is an app’s main window. There can be only one main window per app." — windows › macOS window states
  - Key : celle qui reçoit les saisies ; une seule à l'écran — "the key window accepts people’s input. There can be only one key window onscreen at a time." — windows › macOS window states
  - La fenêtre main de l'app de premier plan est en général la fenêtre key, mais une autre fenêtre, par exemple un panneau flottant au-dessus de la fenêtre main, peut être key à sa place — "Although the front app’s main window is usually the key window" — windows › macOS window states
  - Inactive : toute fenêtre qui n'est pas au premier plan — "A window that’s not in the foreground is an inactive window." — windows › macOS window states
  - Un clic sur une fenêtre la rend key — "People typically click a window to make it key" — windows › macOS window states
  - Dock : un clic sur l'icône d'une app amène toutes ses fenêtres au premier plan et seule la plus récemment consultée devient key — "when people click an app’s Dock icon to bring all of that app’s windows forward, only the most recently accessed window becomes key" — windows › macOS window states
  - *Apparence selon l'état* : le système donne aux trois états des apparences différentes — "The system gives main, key, and inactive windows different appearances to help people visually identify them." — windows › macOS window states
  - Barre de titre : les options de fermeture, de réduction et de zoom (closing, minimizing, zooming) sont en couleur sur la fenêtre key, en gris sur les fenêtres inactives et sur les fenêtres main qui ne sont pas key — "the key window uses color in the title bar options for closing, minimizing, and zooming; inactive windows and main windows that aren’t key use gray in these options" — windows › macOS window states
  - Les fenêtres inactives n'utilisent pas la vibrance (vibrancy), effet qui tire la couleur du contenu situé dessous ; elles paraissent plus effacées et plus lointaines que les fenêtres main et key — "which makes them appear subdued and seem visually farther away than the main and key windows" — windows › macOS window states
  - Certains panneaux, comme Colors ou Fonts, ne deviennent key que lorsqu'on clique leur barre de titre ou un composant qui exige une saisie clavier — "become the key window only when people click the window’s title bar or a component that requires keyboard input, such as a text field" — windows › macOS window states
  - *Fenêtres personnalisées* : toute fenêtre personnalisée doit reprendre les apparences définies par le système — "Make sure custom windows use the system-defined appearances." — windows › macOS window states
  - Avec les composants système, fond et boutons se mettent à jour automatiquement au changement d'état ; une implémentation personnalisée doit le faire elle-même — "a window’s background and button appearances update automatically when the window changes state; if you use custom implementations, you need to do this work yourself" — windows › macOS window states
  - *Barre inférieure* : ne pas y mettre d'information ni d'action critiques, car le bord inférieur d'une fenêtre est souvent masqué quand on la déplace — "Avoid putting critical information or actions in a bottom bar, because people often relocate a window in a way that hides its bottom edge." — windows › macOS window states
  - Au-delà d'une petite quantité d'information, préférer un inspecteur, généralement placé côté trailing (côté de fin) d'une vue partagée — "consider using an inspector, which typically presents information on the trailing side of a split view" — windows › macOS window states
  - *Barre des menus* : la commande qui ouvre un contenu dans une nouvelle fenêtre peut vivre dans un menu contextuel ou dans le menu File — "Consider letting people view content in a new window using a command in a" — windows › Best practices
- **À transposer dans un shell Linux** — **Barre de titre** : zone de déplacement en haut de la fenêtre, boutons fermer, réduire et zoomer dans cette zone, bords redimensionnables ; tailles et couleurs libres, la HIG n'en fixe aucune.
  - **Focus** : distinguer visuellement la fenêtre key (boutons en couleur) des fenêtres inactives (boutons gris, pas de vibrance), un seul focus à la fois, y compris dans une barre de titre dessinée par le shell.
  - **Dock** : un clic sur une icône amène toutes les fenêtres de l'app au premier plan et donne le focus à la plus récemment utilisée.
  - **Zoom et plein écran** : la HIG nomme le zoom sans le définir et ne traite pas le plein écran dans ces pages ; ne pas présumer que l'un vaut l'autre.
  - **Ouverture de fenêtres** : ne pas multiplier les fenêtres des outils du shell (réglages, recherche) quand une seule suffit.
  - **Barre d'état** : n'y placer ni action ni information critiques.

### Panneaux — Panels (`panels`)

- **Rôle** — Dans une app macOS, un panneau (panel) flotte au-dessus des autres fenêtres ouvertes et fournit des commandes, options ou informations complémentaires liées à la fenêtre active ou à la sélection en cours — "In a macOS app, a panel typically floats above other open windows providing supplementary controls, options, or information related to the active window or current selection." — presentation › Discussion
- **Anatomie et cotes** — Le panneau a une apparence moins saillante que la fenêtre main (main window) de l'app — "In general, a panel has a less prominent appearance than an app’s" — panels › Discussion
  - Il comporte une barre de titre, qui permet de le positionner à volonté — "Because a panel often floats above other open windows in your app, it needs a title bar so people can position it where they want." — panels › Best practices
  - Variante d'affichage tête haute (HUD) : même fonction, apparence plus sombre et translucide — "A HUD-style panel serves the same function as a standard panel, but its appearance is darker and translucent." — panels › HUD-style panels
  - Non chiffré dans la HIG : dimensions, rayon des coins, hauteur de la barre de titre, opacité et couleur de fond du HUD. La page n'emploie pas le mot non modal ; le caractère non bloquant du panneau ressort des règles de visibilité ci-dessous et de la section Feuilles.
- **Règles** — *Objet* : réserver le panneau à un accès rapide à des commandes ou informations importantes sur le contenu en cours de travail — "Use a panel to give people quick access to important controls or information related to the content they’re working with." — panels › Best practices
  - Sur les autres plateformes, la HIG oriente vers une vue modale pour le contenu complémentaire — "When your app runs in other platforms, consider using a modal view to present supplementary content that’s relevant to the current task or selection." — panels › Discussion
  - *Inspecteur* : un inspecteur (inspector) affiche les détails de l'élément sélectionné et se met à jour à chaque changement — "displays the details of the currently selected item, automatically updating its contents when the item changes or when people select a new item" — panels › Best practices
  - Une fenêtre Info, au contenu fixe, est une fenêtre ordinaire et non un panneau — "use a regular window, not a panel" — panels › Best practices
  - *Commandes* : éviter celles qui exigent de saisir du texte ou de sélectionner des éléments ; préférer curseurs (sliders) et compteurs (steppers) — "Instead, consider using controls like sliders and steppers because these components can give people more direct control." — panels › Best practices
  - *Visibilité* : quand l'app devient active, amener au premier plan tous ses panneaux ouverts, quelle que soit la fenêtre active au moment de leur ouverture — "When your app becomes active, bring all of its open panels to the front, regardless of which window was active when the panel opened." — panels › Best practices
  - Quand l'app devient inactive, masquer tous ses panneaux — "When your app is inactive, hide all of its panels." — panels › Best practices
  - *HUD* : préférer le panneau standard ; le HUD se justifie dans trois cas seulement (app multimédia, panneau standard qui masquerait un contenu essentiel, absence de commandes) — "In general, use a HUD only:" — panels › HUD-style panels
  - Raison du troisième cas : la plupart des commandes système ne s'accordent pas avec l'apparence d'un HUD — "most system-provided controls don’t match a HUD’s appearance" — panels › HUD-style panels
  - Un HUD sans raison d'être distrait, et il peut ne pas s'accorder avec le réglage d'apparence en cours — "a HUD might not match the current appearance setting" — panels › HUD-style panels
  - Garder un seul style de panneau quand l'app change de mode — "Maintain one panel style when your app switches modes." — panels › HUD-style panels
  - Couleur avec parcimonie : de petites touches très contrastées suffisent à mettre en évidence l'important — "you need only small amounts of high-contrast color to highlight important information in a HUD" — panels › HUD-style panels
  - HUD petits, qui ne masquent ni ne concurrencent le contenu qu'ils règlent — "Don’t let a HUD obscure the content it adjusts, and make sure it doesn’t compete with the content for people’s attention." — panels › HUD-style panels
- **Spécifique macOS** — composant propre à macOS — "Not supported in iOS, iPadOS, tvOS, visionOS, or watchOS." — panels › Platform considerations
  - *Titre* : bref, décrivant l'usage du panneau — "Write a brief title that describes the panel’s purpose." — panels › Best practices
  - Panneaux familiers de macOS : Fonts et Colors ; Inspector est un titre courant dans les apps — "macOS provides familiar panels titled “Fonts” and “Colors,” and many apps use the title “Inspector.”" — panels › Best practices
  - *Désignation* : dans les menus, employer le titre du panneau sans le mot panel (exemples de la HIG : Show Fonts, Show Colors, Show Inspector) — "In menus, use the panel’s title without including the term *panel*" — panels › Best practices
  - *Bouton de réduction* : en général, ne pas le rendre disponible — "In general, avoid making a panel’s minimize button available." — panels › Best practices
  - Raison : le panneau ne s'affiche que lorsque nécessaire et disparaît quand l'app est inactive — "People don’t usually need to minimize a panel, because it displays only when needed and disappears when the app is inactive." — panels › Best practices
  - *Menu Window (barre des menus)* : ne pas faire figurer les panneaux dans la liste de documents — "Avoid including panels in the Window menu’s documents list." — panels › Best practices
  - Ce ne sont ni des documents ni des fenêtres d'app standard — "panels aren’t documents or standard app windows, and they don’t belong in the Window menu’s list" — panels › Best practices
  - Des commandes d'affichage ou de masquage des panneaux peuvent y figurer — "It’s fine to include commands for showing or hiding panels in the" — panels › Best practices
  - *Plein écran et HUD* : le HUD convient aux apps à contenu très visuel ou immersif, comme le montage ou un diaporama plein écran — "HUDs work well in apps that present highly visual content or that provide an immersive experience, such as media editing or a full-screen slide show." — panels › HUD-style panels
  - Si l'app emploie un HUD en plein écran, conserver ce style en sortie de plein écran — "if you use a HUD when your app is in full-screen mode, prefer maintaining the HUD style when people take your app out of full-screen mode" — panels › HUD-style panels
- **À transposer dans un shell Linux** — **Rétroactions à l'écran (volume, luminosité, lecture)** : modèle HUD, à savoir petit, sombre, translucide, peu coloré, sans commandes, de style constant quel que soit le mode.
  - **Réglages rapides** : modèle panneau, à savoir titre court, curseurs et compteurs plutôt que saisie, pas de bouton réduire, masqué quand l'app n'est plus active.
  - **Inspecteur** : une surface qui suit une sélection (fenêtre ou espace choisi dans l'aperçu) se met à jour à chaque changement de sélection.
  - **Menus** : intituler les commandes d'affichage par le nom de la surface, sans son type technique, et ne pas mélanger ces surfaces avec la liste des fenêtres.
  - **Cotes** : aucune valeur à reprendre ; dimensions, opacité et rayon restent libres.

### Popovers — Popovers (`popovers`)

- **Rôle** — Un popover est une vue transitoire qui apparaît au-dessus du reste quand on clique sur un contrôle ou une zone interactive — "A popover is a transient view that appears above other content when people click or tap a control or interactive area." — presentation › Discussion
- **Anatomie et cotes** — Une flèche (arrow) pointe vers l'élément qui a ouvert le popover — "Make sure a popover’s arrow points as directly as possible to the element that revealed it." — popovers › Best practices
  - Taille : juste assez pour le contenu et la flèche ; le système peut la corriger — "If necessary, the system can adjust the size of a popover to ensure it fits well in the interface." — popovers › Best practices
  - Bouton de fermeture facultatif (Close, Cancel ou Done) — "A Close button, including Cancel or Done, is worth including if it provides clarity, like exiting with or without saving changes." — popovers › Best practices
  - Non chiffré dans la HIG : largeur et hauteur maximales, longueur de la flèche, rayon des coins, marges, durées d'apparition et de disparition.
- **Règles** — *Modalité et fermeture* : le popover est non modal ; il se ferme en général quand on clique en dehors ou quand on choisit un élément — "a popover generally closes when people click or tap outside its bounds or select an item in the popover" — popovers › Best practices
  - Si plusieurs sélections sont possibles, il reste ouvert jusqu'à fermeture explicite ou clic à l'extérieur — "If multiple selections are possible, make sure the popover remains open until people explicitly dismiss it or they click or tap outside its bounds." — popovers › Best practices
  - Enregistrer toujours le travail à la fermeture automatique d'un popover non modal — "Always save work when automatically closing a nonmodal popover." — popovers › Best practices
  - N'abandonner le travail que sur un bouton Cancel explicite — "Discard people’s work only when they click or tap an explicit Cancel button." — popovers › Best practices
  - Le bouton de fermeture ne sert qu'à confirmer ou guider — "Use a Close button for confirmation and guidance only." — popovers › Best practices
  - *Contenu* : exposer peu d'information ou de fonctionnalité — "Use a popover to expose a small amount of information or functionality." — popovers › Best practices
  - *Position* : ne pas recouvrir l'élément déclencheur ni un contenu essentiel — "Ideally, a popover doesn’t cover the element that revealed it or any essential content people may need to see while using it." — popovers › Best practices
  - *Unité et empilement* : un seul popover à la fois — "Show one popover at a time." — popovers › Best practices
  - Jamais de cascade ni de hiérarchie de popovers — "Never show a cascade or hierarchy of popovers, in which one emerges from another." — popovers › Best practices
  - Rien ne s'affiche au-dessus d'un popover, sauf une alerte — "Make sure nothing displays on top of a popover, except for an alert." — popovers › Best practices
  - Permettre, quand c'est possible, de fermer l'un et d'ouvrir l'autre d'un seul clic — "When possible, let people close one popover and open another with a single click or tap." — popovers › Best practices
  - C'est particulièrement souhaitable quand plusieurs boutons d'une barre ouvrent chacun un popover — "Avoiding extra gestures is especially desirable when several different bar buttons each open a popover." — popovers › Best practices
  - *Changement de taille* : animer le passage entre vue condensée et vue étendue — "Provide a smooth transition when changing the size of a popover." — popovers › Best practices
  - *Avertissements* : ne pas utiliser de popover, car on peut le manquer ou le fermer par accident — "Avoid using a popover to show a warning." — popovers › Best practices
- **Spécifique macOS** — aucune considération supplémentaire pour visionOS ; non pris en charge sur tvOS et watchOS — "No additional considerations for visionOS. Not supported in tvOS or watchOS." — popovers › Platform considerations
  - *Popover détachable* : sur macOS, un popover peut être détachable et devient un panneau séparé quand on le fait glisser — "You can make a popover detachable in macOS, which becomes a separate panel when people drag it." — popovers › macOS
  - Ce panneau reste visible pendant qu'on interagit avec d'autres contenus — "The panel remains visible onscreen while people interact with other content." — popovers › macOS
  - Envisager de laisser détacher le popover — "Consider letting people detach a popover." — popovers › macOS
  - Modifier au minimum l'apparence du popover détaché — "Make minimal appearance changes to a detached popover." — popovers › macOS
  - Un panneau ressemblant au popover d'origine aide à conserver le contexte — "A panel that looks similar to the original popover helps people maintain context." — popovers › macOS
  - La HIG ne dit rien, pour macOS, sur un ancrage à la barre des menus ou au Dock, ni sur une touche clavier de fermeture.
- **À transposer dans un shell Linux** — **Menus de la barre (réseau, son, batterie, calendrier)** : une seule popover ouverte à la fois, fermeture au clic extérieur ou après un choix, flèche pointant vers l'icône, icône non recouverte.
  - **Bascule entre menus** : un clic sur une autre icône de la barre ferme la popover ouverte et ouvre la suivante.
  - **Réglages dans une popover** : appliquer et mémoriser chaque changement à la volée ; n'abandonner que sur un bouton Annuler explicite.
  - **Empilement et avertissements** : rien au-dessus d'une popover sauf une alerte, jamais de popover dans une popover, pas d'avertissement dans une popover.
  - **Épinglage** : à l'image du popover détachable, une popover glissée hors de son ancre peut devenir un panneau flottant d'apparence quasi identique.
  - **Animations** : animer tout changement de taille (vue condensée à vue étendue).

### Feuilles — Sheets (`sheets`)

- **Rôle** — Une feuille (sheet) aide à accomplir une tâche délimitée, étroitement liée au contexte en cours — "A sheet helps people perform a scoped task that’s closely related to their current context." — presentation › Discussion
- **Anatomie et cotes** — Usage : demander une information précise ou proposer une tâche simple à terminer avant de revenir à la vue parente — "A sheet is useful for requesting specific information from people or presenting a simple task that they can complete before returning to the parent view." — sheets › Discussion
  - Cancel (ou Close) ferme sans rien enregistrer ; ce bouton est présent dans la plupart des feuilles — "button dismisses a sheet without saving any changes. This type of button is common in most sheets." — sheets › Anatomy
  - Done ferme après accomplissement de la tâche ou enregistrement explicite — "button dismisses a sheet after completing a task or explicitly saving changes." — sheets › Anatomy
  - Back mène à l'étape précédente d'un flux en plusieurs étapes ou à la vue parente d'une hiérarchie et ne sert pas à fermer — "lets people navigate to a previous step in a multi-step flow or to a parent view in a hierarchy. It isn’t intended to dismiss a sheet." — sheets › Anatomy
  - Non chiffré dans la HIG : dimensions par défaut, rayon des coins, opacité de l'assombrissement, écart avec le cadre de la fenêtre parente, durées.
- **Règles** — *Flux complexes* : pour un flux complexe ou long, envisager autre chose qu'une feuille — "For complex or prolonged user flows, consider alternatives to sheets." — sheets › Best practices
  - *Unité* : n'afficher qu'une feuille à la fois depuis l'interface principale — "Display only one sheet at a time from the main interface." — sheets › Best practices
  - Si une action dans une feuille doit en ouvrir une autre, fermer la première avant d'afficher la nouvelle — "If something people do within a sheet results in another sheet appearing, close the first sheet before displaying the new one." — sheets › Best practices
  - *Alternative non modale* : pour des éléments complémentaires qui agissent sur la tâche principale, utiliser une vue non modale — "Use a nonmodal view when you want to present supplementary items that affect the main task in the parent view." — sheets › Best practices
  - *Boutons* : un bouton Done va toujours de pair avec un bouton Cancel (ou un bouton Back) — "If you provide a Done button, always pair it with a Cancel button" — sheets › Best practices
  - Ne jamais afficher Cancel, Done et Back ensemble — "Avoid showing all three buttons — Cancel, Done, and Back — together." — sheets › Best practices
- **Spécifique macOS** — sur macOS, une feuille est toujours modale — "In macOS, tvOS, visionOS, and watchOS, a sheet is always *modal*." — sheets › Anatomy
  - Elle empêche d'interagir avec la vue parente jusqu'à sa fermeture — "A modal sheet presents a targeted experience that prevents people from interacting with the parent view until they dismiss the sheet" — sheets › Anatomy
  - *Apparence* : vue en forme de carte, à coins arrondis, qui flotte au-dessus de sa fenêtre parente — "In macOS, a sheet is a cardlike view with rounded corners that floats on top of its parent window." — sheets › macOS
  - La fenêtre parente est assombrie tant que la feuille est affichée, signe qu'on ne peut pas interagir avec elle — "The parent window is dimmed while the sheet is onscreen, signaling that people can’t interact with it until they dismiss the sheet." — sheets › macOS
  - *Rapport aux autres fenêtres* : on doit pouvoir interagir avec les autres fenêtres de l'app sans d'abord fermer la feuille — "Let people interact with other app windows without first dismissing a sheet." — sheets › macOS
  - À l'ouverture de la feuille, sa fenêtre parente passe au premier plan — "When a sheet opens, you bring its parent window to the front" — sheets › macOS
  - Si la parente est une fenêtre de document, ses panneaux non modaux liés au document passent aussi au premier plan — "you also bring forward its modeless document-related panels" — sheets › macOS
  - *Taille* : taille par défaut raisonnable, adaptée au contenu, car on ne s'attend pas à redimensionner une feuille — "People don’t generally expect to resize sheets, so it’s important to use a size that’s appropriate for the content you display." — sheets › macOS
  - Soutenir tout de même le redimensionnement — "so it’s a good idea to support resizing" — sheets › macOS
  - *Panneau plutôt que feuille* : si l'on doit fournir plusieurs fois une entrée et observer le résultat — "Use a panel instead of a sheet if people need to repeatedly provide input and observe results." — sheets › macOS
  - Exemple : un panneau de recherche et remplacement qui laisse lancer chaque remplacement un par un — "A find and replace panel, for example, might let people initiate replacements individually, so they can observe the result of each search for correctness." — sheets › macOS
  - Pour des informations et actions à utiliser en continuant d'interagir avec la fenêtre main, macOS oriente vers un panneau — "To give people access to information and actions they need while continuing to interact with the main window" — sheets › Best practices
  - *Fenêtre ou plein écran plutôt que feuille* — "In a macOS experience, you might want to open a new window or let people enter full-screen mode instead of using a sheet." — sheets › Best practices
  - *Non précisé* : la position des boutons Cancel, Done et Back sur macOS, le bloc macOS de la page ne la donnant pas — "The placement of these buttons varies between platforms" — sheets › Anatomy
- **À transposer dans un shell Linux** — **Dialogues rattachés à une fenêtre** (enregistrement, propriétés d'un élément) : feuille solidaire de sa fenêtre parente, seule la parente est assombrie, les autres fenêtres de l'app restent accessibles.
  - **Boutons** : toute feuille fermée par Done offre aussi Cancel ou Back ; jamais les trois ; Back ne ferme jamais.
  - **Unité** : une seule feuille à la fois ; fermer la première avant d'ouvrir la suivante.
  - **Tâches longues** : une fenêtre distincte plutôt qu'une feuille.
  - **Réglages itératifs** : pour un usage répété entrée puis résultat (recherche, remplacement), un panneau non modal.
  - **Position des boutons** : non fixée par la HIG pour macOS ; choisir une position et l'appliquer aussi aux alertes.

### Alertes — Alerts (`alerts`)

- **Rôle** — Une alerte (alert) donne une information critique dont la personne a besoin immédiatement — "An alert gives people critical information they need right away." — presentation › Discussion
- **Anatomie et cotes** — Vue modale, d'aspect variable selon les plateformes et les appareils — "An alert is a modal view that can look different in different platforms and devices." — alerts › Anatomy
  - Contenu sur toutes les plateformes : un titre, un texte informatif facultatif, trois boutons au plus — "In all platforms, alerts display a title, optional informative text, and up to three buttons." — alerts › Content
  - Usages : confirmer un achat ou une autre action importante engagée — "give them an opportunity to confirm a purchase or another important action they initiated" — alerts › Discussion
  - Non chiffré dans la HIG : largeur, hauteur, rayon des coins, taille de l'icône, espacements, durée d'affichage ; la position de l'alerte à l'écran ou par rapport à sa fenêtre n'est pas donnée.
- **Règles** — *Quand alerter* : utiliser les alertes avec parcimonie, car elles interrompent la tâche en cours — "Use alerts sparingly." — alerts › Best practices
  - Ne pas utiliser une alerte pour simplement informer — "Avoid using an alert merely to provide information." — alerts › Best practices
  - Pour informer seulement, préférer un autre moyen dans le contexte pertinent — "If you need to provide only information, prefer finding an alternative way to communicate it within the relevant context." — alerts › Best practices
  - Exemple : serveur indisponible, Mail affiche un indicateur sur lequel on peut en savoir plus — "when a server connection is unavailable, Mail displays an indicator that people can choose to learn more" — alerts › Best practices
  - Pas d'alerte pour des actions courantes et annulables, même destructrices — "Avoid displaying alerts for common, undoable actions, even when they’re destructive." — alerts › Best practices
  - Alerter en revanche pour une action destructive rare et irréversible — "when people take an uncommon destructive action that they can’t undo, it’s important to display an alert in case they initiated the action accidentally" — alerts › Best practices
  - Pas d'alerte au démarrage de l'app — "Avoid showing an alert when your app starts." — alerts › Best practices
  - Plutôt des données en cache ou fictives et une étiquette non intrusive — "you could show cached or placeholder data and a nonintrusive label that describes the problem" — alerts › Best practices
  - *Texte* : ton direct, neutre et avenant — "In all alert copy, be direct, and use a neutral, approachable tone." — alerts › Content
  - Titre décrivant la situation : ce qui s'est passé, dans quel contexte, pourquoi — "describe what happened, the context in which it happened, and why" — alerts › Content
  - Titre de deux lignes au maximum — "avoid overly long titles that wrap to more than two lines" — alerts › Content
  - Texte informatif seulement s'il apporte quelque chose — "Include informative text only if it adds value." — alerts › Content
  - Ne pas expliquer les boutons — "Avoid explaining alert buttons." — alerts › Content
  - Champ de texte seulement si la résolution exige une saisie — "include a text field only if you need people’s input to resolve the situation" — alerts › Content
  - *Titres des boutons* : un ou deux mots décrivant le résultat — "Aim for a one- or two-word title that describes the result of selecting the button." — alerts › Buttons
  - Préférer des verbes liés directement au texte de l'alerte — "Prefer verbs and verb phrases that relate directly to the alert text" — alerts › Buttons
  - Le bouton d'annulation s'intitule toujours Cancel — "Always use “Cancel” to title a button that cancels the alert’s action." — alerts › Buttons
  - OK seulement dans les alertes purement informatives, où l'on évite Yes et No — "In informational alerts only, you can use “OK” for acceptance, avoiding “Yes” and “No.”" — alerts › Buttons
  - *Placement* : le bouton le plus probable côté trailing (côté de fin) d'une rangée ou en haut d'une pile — "In general, place the button people are most likely to choose on the trailing side in a row of buttons or at the top in a stack of buttons." — alerts › Buttons
  - Le bouton par défaut toujours côté trailing d'une rangée ou en haut d'une pile — "Always place the default button on the trailing side of a row or at the top of a stack." — alerts › Buttons
  - Cancel en général côté leading (côté de début) d'une rangée ou en bas d'une pile — "Cancel buttons are typically on the leading side of a row or at the bottom of a stack." — alerts › Buttons
  - *Destruction* : style destructeur pour l'action destructive que la personne n'a pas choisie délibérément — "Use the destructive style to identify a button that performs a destructive action people didn’t deliberately choose." — alerts › Buttons
  - Action choisie délibérément (vider la corbeille) : pas de style destructeur — "the resulting alert doesn’t apply the destructive style to the Empty Trash button because the button performs the person’s original intent" — alerts › Buttons
  - En pareil cas, la commodité de confirmer par Return l'emporte — "the convenience of pressing Return to confirm the deliberately chosen Empty Trash action outweighs the benefit of reaffirming that the button is destructive" — alerts › Buttons
  - S'il y a une action destructive, offrir un bouton Cancel — "If there’s a destructive action, include a Cancel button to give people a clear, safe way to avoid the action." — alerts › Buttons
  - Jamais Cancel comme bouton par défaut — "Note that you don’t want to make a Cancel button the default button." — alerts › Buttons
  - Pour forcer la lecture, aucun bouton par défaut — "If you want to encourage people to read an alert and not just automatically press Return to dismiss it, avoid making any button the default button." — alerts › Buttons
  - OK comme titre du bouton par défaut seulement si l'alerte est purement informative — "Avoid using OK as the default button title unless the alert is purely informational." — alerts › Buttons
  - Alerte à un seul bouton, qui est aussi le bouton par défaut : un bouton Done, et non Cancel — "if you must display an alert with a single button that’s also the default, use a Done button, not a Cancel button" — alerts › Buttons
  - *Annulation* : prévoir d'autres façons d'annuler quand cela a du sens — "Provide alternative ways to cancel an alert when it makes sense." — alerts › Buttons
- **Spécifique macOS** — le bloc macOS de la page porte sur le contenu et le symbole de prudence — "No additional considerations for tvOS or watchOS." — alerts › Platform considerations
  - *Contenu propre à macOS* : icône de l'app affichée automatiquement, remplaçable par une autre icône ou un symbole — "macOS automatically displays your app icon in an alert, but you can supply an alternative icon or symbol." — alerts › macOS
  - Vue personnalisée (accessory view) ajoutable pour donner plus d'information — "Append a custom view if it’s necessary to provide additional information" — alerts › macOS
  - Champ de texte possible — "In iOS, iPadOS, macOS, and visionOS, an alert can include a text field." — alerts › Content
  - Alerte répétitive : possibilité de supprimer les occurrences suivantes de la même alerte — "Configure repeating alerts to let people suppress subsequent occurrences of the same alert." — alerts › macOS
  - Bouton Aide (Help button) ouvrant la documentation — "Include a Help button that opens your help documentation" — alerts › macOS
  - *Symbole de prudence* : à utiliser avec parcimonie — "Use a caution symbol sparingly." — alerts › macOS
  - Réservé aux cas demandant vraiment plus d'attention, comme confirmer une action pouvant entraîner une perte de données inattendue — "Use the symbol only when extra attention is really needed, as when confirming an action that might result in unexpected loss of data." — alerts › macOS
  - Pas pour des tâches dont le seul but est d'écraser ou de retirer des données — "Don’t use the symbol for tasks whose only purpose is to overwrite or remove data, such as a save or empty trash." — alerts › macOS
  - *Clavier* : sur un clavier connecté, Escape (Esc) ou Command-Period annule une alerte sur iOS, iPadOS, macOS et visionOS — "Pressing Escape (Esc) or Command-Period (.) on an attached keyboard" — alerts › Buttons
- **À transposer dans un shell Linux** — **Confirmations du shell (suppression, déconnexion, arrêt)** : alerte seulement pour une action destructive rare et irréversible ; jamais pour informer ni au lancement de la session.
  - **Information non urgente** : indicateur discret dans le contexte, à la façon de Mail, plutôt qu'une alerte ; c'est le rôle des notifications.
  - **Boutons** : trois au plus, verbes d'un ou deux mots, Cancel pour annuler, pas de OK par défaut, Cancel jamais par défaut ; bouton par défaut côté trailing (fin de rangée), Cancel côté leading (début de rangée) dans une interface qui se lit de gauche à droite.
  - **Style destructeur** : seulement lorsque l'action destructive n'a pas été choisie délibérément.
  - **Extras** : icône de l'app, option de non-réaffichage pour les alertes répétées, bouton d'aide ; symbole de prudence réservé aux pertes de données inattendues ; touche clavier d'annulation (la HIG cite Escape).
  - **Position à l'écran** : non précisée par la HIG ; choix libre pour le centre de notifications et les dialogues.

### Feuilles d'action — Action sheets (`action-sheets`)

- **Rôle** — Une feuille d'action (action sheet) est une vue modale qui propose des choix liés à une action engagée par la personne — "An action sheet is a modal view that presents choices related to an action people initiate." — presentation › Discussion
- **Anatomie et cotes** — Non chiffré dans la HIG : la page ne donne ni nombre de choix, ni hauteur, ni rayon, ni durée, ni position à l'écran.
- **Règles** — *Action ou alerte* : pour proposer des choix liés à une action délibérée, utiliser une feuille d'action et non une alerte — "Use an action sheet — not an alert — to offer choices related to an intentional action." — action-sheets › Best practices
  - L'alerte, elle, survient en général à l'improviste — "an alert is usually unexpected, generally telling people about a problem or a change in the current situation that might require them to act" — action-sheets › Best practices
  - *Parsimonie* : utiliser les feuilles d'action avec parcimonie — "Use action sheets sparingly." — action-sheets › Best practices
  - *Texte* : titre assez court pour tenir sur une ligne — "Aim to keep titles short enough to display on a single line." — action-sheets › Best practices
  - Message seulement si nécessaire — "Provide a message only if necessary." — action-sheets › Best practices
  - *Annulation et destruction* : placer le bouton Cancel en bas de la feuille d'action — "Place the Cancel button at the bottom of the action sheet" — action-sheets › Best practices
  - Rendre les choix destructeurs visuellement proéminents — "Make destructive choices visually prominent." — action-sheets › Best practices
  - Les placer en haut de la feuille, où ils sont le plus visibles — "place these buttons at the top of the action sheet where they tend to be most noticeable" — action-sheets › Best practices
- **Spécifique macOS** — aucune considération supplémentaire pour macOS ; non pris en charge sur visionOS — "No additional considerations for macOS or tvOS. Not supported in visionOS." — action-sheets › Platform considerations
  - Avec SwiftUI, la fonctionnalité de feuille d'action est offerte sur toutes les plateformes, donc macOS — "you can offer action sheet functionality in all platforms" — action-sheets › Discussion
  - Sous la forme d'un modificateur de présentation propre à un dialogue de confirmation (confirmation dialog) — "for a confirmation dialog" — action-sheets › Discussion
  - La variante UIKit ne concerne qu'iOS, iPadOS et tvOS — "to display an action sheet in iOS, iPadOS, and tvOS" — action-sheets › Discussion
  - La HIG ne donne rien, pour macOS, sur l'apparence, la taille, la position à l'écran, l'ancre, la barre des menus, le Dock ou la souris ; le placement de Cancel en haut à gauche mentionné sur la page est propre à watchOS.
- **À transposer dans un shell Linux** — **Menu contextuel d'une icône du Dock ou d'un élément** : liste de choix liés à l'action volontairement engagée, avec une entrée Annuler.
  - **Hiérarchie** : choix destructeur en style destructeur et en tête de liste, annulation à l'opposé, en bas.
  - **Feuille d'action ou alerte** : deux options ou plus liées à une action choisie, feuille d'action ; avertissement ou simple confirmation, alerte.
  - **Texte** : titre sur une ligne, message seulement si nécessaire.
  - **Apparence** : libre, la HIG ne décrivant rien pour macOS.

### Vues défilantes — Scroll views (`scroll-views`)

- **Rôle** — Une vue défilante (scroll view) permet de parcourir un contenu plus grand que ses limites en le déplaçant verticalement ou horizontalement — "A scroll view lets people view content that’s larger than the view’s boundaries by moving the content vertically or horizontally." — presentation › Discussion
- **Anatomie et cotes** — Pas d'apparence propre ; un indicateur de défilement (scroll indicator) translucide peut s'afficher, en général après le début du défilement — "but it can display a translucent *scroll indicator* that typically appears after people begin scrolling the view’s content" — scroll-views › Discussion
  - L'indicateur situe le contenu visible : début, milieu ou fin — "the indicator shows whether the currently visible content is near the beginning, middle, or end of the view" — scroll-views › Discussion
  - Non chiffré dans la HIG : largeur de la barre, longueur du curseur, durée d'affichage après l'arrêt du défilement, valeurs des tailles small et mini.
- **Règles** — *Comportement* : prendre en charge les gestes de défilement et les raccourcis clavier par défaut — "Support default scrolling gestures and keyboard shortcuts." — scroll-views › Best practices
  - Pour un défilement personnalisé, des indicateurs au comportement élastique attendu — "If you build custom scrolling for a view, make sure your scroll indicators use the elastic behavior that people expect." — scroll-views › Best practices
  - Rendre évident que le contenu défile : un contenu partiel au bord de la vue signale une suite — "displaying partial content at the edge of a view indicates that there’s more content in that direction" — scroll-views › Best practices
  - Pas de vue défilante dans une vue défilante de même orientation — "Avoid putting a scroll view inside another scroll view with the same orientation." — scroll-views › Best practices
  - *Page par page* : à envisager si le contenu s'y prête — "Consider supporting page-by-page scrolling if it makes sense for your content." — scroll-views › Best practices
  - Une page vaut en général la hauteur ou la largeur courante de la vue — "typically the current height or width of the view" — scroll-views › Best practices
  - Retrancher de la page une unité de chevauchement pour garder le contexte — "you can define a unit of overlap, such as a line of text, a row of glyphs, or part of a picture" — scroll-views › Best practices
  - *Défilement automatique* : dans certains cas, pour aider à retrouver sa place — "In some cases, scroll automatically to help people find their place." — scroll-views › Best practices
  - Quand une opération sélectionne du contenu hors de vue, comme un résultat de recherche — "when your app locates text that people are searching for, scroll the content to bring the new selection into view" — scroll-views › Best practices
  - Quand le pointeur franchit le bord de la vue pendant une sélection, suivre le pointeur — "follow the pointer by scrolling in the direction it moves" — scroll-views › Best practices
  - Défiler au minimum, pour préserver le contexte — "In all cases, automatically scroll the content only as much as necessary to help people retain context." — scroll-views › Best practices
  - *Zoom* : des échelles minimale et maximale appropriées — "If you support zoom, set appropriate maximum and minimum scale values." — scroll-views › Best practices
  - *Effet de bord (scroll edge effect)* : préférer le style automatique — "Prefer the automatic scroll edge effect style." — scroll-views › Scroll edge effects
  - Avec le style soft, vérifier la lisibilité des commandes en tout contexte — "If you use the soft scroll edge effect style instead, thoroughly test your interface to ensure your controls maintain legibility in a variety of contexts." — scroll-views › Scroll edge effects
  - Un effet seulement si une vue défilante passe derrière des éléments flottants — "Only use a scroll edge effect when a scroll view is behind floating interface elements." — scroll-views › Scroll edge effects
  - Pas un ornement, ni un calque qui assombrit — "Scroll edge effects aren’t decorative. They don’t block or darken like overlays; they exist to ensure controls stay visually distinct." — scroll-views › Scroll edge effects
  - Un effet par vue — "Apply one scroll edge effect per view." — scroll-views › Scroll edge effects
  - Dans une vue partagée, un effet par volet, tous de même hauteur — "each pane can have its own scroll edge effect; in this case, keep them consistent in height to maintain alignment" — scroll-views › Scroll edge effects
- **Spécifique macOS** — sur macOS, l'indicateur s'appelle usuellement barre de défilement (scroll bar) — "In macOS, a *scroll indicator* is commonly called a *scroll bar*." — scroll-views › macOS
  - Au besoin, de petites (small) ou mini barres dans un panneau — "If necessary, use small or mini scroll bars in a panel." — scroll-views › macOS
  - Quand la place manque, des barres plus petites pour les panneaux qui coexistent avec d'autres fenêtres — "When space is tight, you can use smaller scroll bars in panels that need to coexist with other windows." — scroll-views › macOS
  - Même taille pour toutes les commandes d'un tel panneau — "Be sure to use the same size for all controls in such a panel." — scroll-views › macOS
  - *Effet de bord* : il existe aussi sur macOS — "In iOS, iPadOS, and macOS, a *scroll edge effect* provides a visual separation between certain interface elements" — scroll-views › Scroll edge effects
  - Avec des barres personnalisées, ajouter l'effet à la main — "If you use custom bars, you might want to add this effect manually" — scroll-views › Scroll edge effects
  - Ou passer le style d'automatique à hard ou soft — "adjust its style from automatic to the hard or soft style" — scroll-views › Scroll edge effects
  - *Visibilité* : les indicateurs ne sont pas toujours visibles ; la HIG ne décrit ni barre permanente avec piste, ni réglage de visibilité, ni comportement au survol de la souris — "Because scroll indicators aren’t always visible, it can be helpful to make it obvious when content extends beyond the view." — scroll-views › Best practices
  - Rien sur le Dock, la barre des menus ou la molette.
- **À transposer dans un shell Linux** — **Listes du lanceur et de la recherche, pages de réglages** : barre fine translucide liée au contenu, qui apparaît au défilement, sans cote imposée.
  - **Signe de débordement** : laisser affleurer un contenu partiel au bord de la vue quand il y a une suite.
  - **Défilement automatique** : amener au minimum l'élément sélectionné (résultat de recherche, réglage ciblé) dans la vue.
  - **Barres flottantes** : un effet de bord sous toute barre ou tout en-tête flottant au-dessus d'une liste, un par vue, de même hauteur entre volets ; ni calque assombri ni ornement.
  - **Imbrication** : pas de liste défilante dans une liste de même orientation.
  - **Panneaux compacts** : barre plus fine, de taille uniforme dans tout le panneau.

### Contrôles de pages — Page controls (`page-controls`)

- **Rôle** — Un contrôle de pages (page control) affiche une rangée d'images indicatrices, chacune représentant une page d'une liste plate — "A page control displays a row of indicator images, each of which represents a page in a flat list." — presentation › Discussion
- **Anatomie et cotes** — Par défaut, une série de petits points indicateurs ; un point plein marque la page en cours — "Page controls appear as a series of small indicator dots by default, representing the available pages. A solid dot denotes the current page." — page-controls › Discussion
  - Points toujours équidistants, tronqués s'ils ne tiennent pas dans la fenêtre — "Visually, these dots are always equidistant, and are clipped if there are too many to fit in the window." — page-controls › Discussion
  - La rangée d'indicateurs défile pour atteindre la page voulue — "The scrolling row of indicators helps people navigate the list to find the page they want." — page-controls › Discussion
  - Nombre de pages quelconque, d'où l'intérêt pour les listes personnalisées — "Page controls can handle an arbitrary number of pages, making them particularly useful in situations where people can create custom lists." — page-controls › Discussion
  - Environ dix points au-delà desquels le comptage à vue devient difficile — "More than about 10 dots are hard to count at a glance." — page-controls › Best practices
  - Non chiffré dans la HIG : diamètre et espacement des points, couleurs.
- **Règles** — *Usage* : représenter un déplacement dans une liste ordonnée de pages — "Use page controls to represent movement between an ordered list of pages." — page-controls › Best practices
  - Pas de relations hiérarchiques ou non séquentielles — "Page controls don’t represent hierarchical or nonsequential page relationships." — page-controls › Best practices
  - Pour une navigation complexe, barre latérale ou vue partagée — "For more complex navigation, consider using a sidebar or split view instead." — page-controls › Best practices
  - *Position* : centré en bas de la vue ou de la fenêtre — "Center a page control at the bottom of the view or window." — page-controls › Best practices
  - Centré horizontalement, près du bas — "center it horizontally and position it near the bottom of the view" — page-controls › Best practices
  - *Limite* : plus de dix pages de même rang, une autre disposition comme une grille — "If your app needs to display more than 10 pages as peers, consider using a different arrangement" — page-controls › Best practices
  - *Images indicatrices personnalisées* : ne remplacer l'image par défaut que si cela renforce le sens — "Customize the default indicator image only when it enhances the page control’s overall meaning." — page-controls › Customizing indicators
  - Images simples et claires — "Make sure custom indicator images are simple and clear." — page-controls › Customizing indicators
  - Pas de forme complexe, d'espace négatif, de texte ni de trait interne — "Avoid complex shapes, and don’t include negative space, text, or inner lines" — page-controls › Customizing indicators
  - Deux images indicatrices différentes au plus — "Avoid using more than two different indicator images in a page control." — page-controls › Customizing indicators
  - Ne pas colorer les images — "Avoid coloring indicator images." — page-controls › Customizing indicators
  - Une couleur personnalisée réduit le contraste qui distingue la page en cours — "Custom colors can reduce the contrast that differentiates the current-page indicator and makes the page control visible on the screen." — page-controls › Customizing indicators
- **Spécifique macOS** — composant non pris en charge sur macOS ; la page ne contient rien d'autre à son sujet, ni Dock, ni barre des menus, ni souris, ni fenêtre — "Not supported in macOS." — page-controls › Platform considerations
- **À transposer dans un shell Linux** — **Aperçu des espaces, pagination du bureau** : aucun modèle macOS dans la HIG ; une pagination de surfaces ne peut pas se réclamer du bureau macOS sur ce point.
  - **Si un indicateur de pages est retenu** : points équidistants, point plein pour la page en cours, centré en bas, environ dix au plus, sans couleur personnalisée.
  - **Au-delà** : une grille navigable dans n'importe quel ordre.
  - **Relations non séquentielles (espaces indépendants)** : barre latérale ou vue partagée plutôt qu'un contrôle de pages.

### Valeurs de référence

| Propriété | Valeur | Source (slug › section) |
|---|---|---|
| Types conceptuels de fenêtre | 2 : primary, auxiliary | windows › Discussion |
| États d'une fenêtre macOS | 3 : main, key, inactive | windows › macOS window states |
| Fenêtres main par app | une seule | windows › macOS window states |
| Fenêtres key à l'écran | une seule | windows › macOS window states |
| Partie d'une fenêtre macOS | 2 : frame (au-dessus), body area | windows › macOS window anatomy |
| Disponibilité des panels | macOS uniquement | panels › Platform considerations |
| Popovers affichés à la fois | 1 | popovers › Best practices |
| Feuilles affichées à la fois depuis l'interface principale | 1 | sheets › Best practices |
| Boutons d'une feuille | 3 nommés : Cancel (ou Close), Done, Back ; jamais les trois ensemble | sheets › Anatomy |
| Modalité d'une feuille sur macOS | toujours modale | sheets › Anatomy |
| Boutons d'une alerte | 3 au plus | alerts › Content |
| Longueur du titre d'une alerte | pas plus de 2 lignes | alerts › Content |
| Titre d'un bouton d'alerte | 1 ou 2 mots | alerts › Buttons |
| Bouton par défaut d'une alerte | côté trailing d'une rangée, ou haut d'une pile | alerts › Buttons |
| Bouton Cancel d'une alerte | côté leading d'une rangée, ou bas d'une pile | alerts › Buttons |
| Annulation d'une alerte au clavier | Escape (Esc) ou Command-Period (.) | alerts › Buttons |
| Feuilles d'action sur macOS | aucune considération supplémentaire | action-sheets › Platform considerations |
| Position du Cancel d'une feuille d'action | en bas de la feuille (en haut à gauche sur watchOS) | action-sheets › Best practices |
| Taille d'une page (défilement page par page) | en général la hauteur ou la largeur courante de la vue, moins une unité de chevauchement | scroll-views › Best practices |
| Taille des barres de défilement sur macOS | small ou mini, dans un panneau ; aucune valeur numérique | scroll-views › macOS |
| Effet de bord par vue | 1 ; styles automatic, hard, soft | scroll-views › Scroll edge effects |
| Contrôles de pages sur macOS | non pris en charge | page-controls › Platform considerations |
| Points d'un contrôle de pages | environ 10 au-delà desquels le comptage devient difficile | page-controls › Best practices |
| Images indicatrices différentes | 2 au plus | page-controls › Customizing indicators |

## Seconde partie — hyprdesk aujourd'hui, écarts et actions

### Ce que hyprdesk fait aujourd'hui
| Surface | Ce qui existe | Valeurs clés | Preuves |
| --- | --- | --- | --- |
| Cartes et menus de la barre | Une carte à la fois (surface unique réutilisée), ancrée aux limites de la commande, sans flèche ; capteur plein écran hors barre ; clavier exclusif ; Échap ferme ; confirmation « Annuler / Confirmer » dans la carte | largeur 328 px, écart 10 px, ombre 18 px, ouverture 180 ms, contenu 160 ms, fermeture 120 ms, hauteur bornée à écran − barre − 20 px | `src/ui/plugins/bar/popover.rs:26,30,342-345,405-411,632-690,771-795,816-831,850-854,962-990,1154-1192,1194-1202,1549-1606` ; `src/ui/mod.rs:5385-5392` ; `DESIGN.md` § Barre « Continuité » |
| Centre de contrôle (Continuité) | Carte large à colonnes, curseurs et boutons, bouton Fermer, clavier (Tab, flèches) | largeur de 80 à 1600 px selon l'écran, écart 12 px, même borne de hauteur | `src/ui/plugins/bar/popover.rs:413-418,668-677` ; `src/ui/plugins/bar/popover/continuity.rs:23-24,248-257,341-362,537` ; `continuity/keyboard.rs:13-38` |
| Bulles du dock | Surface `Overlay` propre, ouverte au survol, à 8 px de l'icône ; nom ou liste des fenêtres de l'application | délai 180 ms (400 ms en Convex), fermeture 250 ms après sortie | `src/ui/plugins/dock/surface.rs:142,1286-1291,1396` ; `src/ui/plugins/dock/lifecycle.rs:13,126-163` |
| Tiroir des Réglages | Tiroir latéral non modal (clavier à la demande), glisse de 28 px, Échap ferme | largeur 936 px, écran entier en dessous de 1200 px, écart 12 px, barre de navigation de 4 px | `src/ui/panel.rs:126,130,234-250,639-645,893-900,1025-1031,1083` ; `src/ui/mod.rs:5393-5395` |
| Tiroir de l'assistant | Même tiroir, non modal ; un seul tiroir à la fois avec les Réglages | largeur 780 px | `src/ui/panel.rs:129,199-201` ; `src/ui/chat.rs:458,475-480` |
| Assistant de configuration | Plein écran, bureau assombri, carte centrée, clavier exclusif, dix étapes ; choix appliqués aussitôt, Échap ferme en conservant | pastilles numérotées cliquables | `src/ui/wizard.rs:468-476,876-933,956-964,980-1043` ; `src/ui/mod.rs:5311-5316` |
| Visite guidée | Carte `Overlay` au-dessus des Réglages ; points d'avancement ; Précédent, Suivant, Terminer, Quitter la visite | 780 × 520 px, dix étapes | `src/ui/tour.rs:21-23,373-378,532-582` |
| Aperçu des espaces, commutateur de fenêtres | Plein écran `Overlay`, clavier exclusif, Échap ferme ; aperçu sur fond assombri | — | `src/ui/overview.rs:155-160,560,777` ; `src/ui/switcher.rs:74-79` ; `src/ui/mod.rs:5344-5353` |
| Alerte problème | Fenêtre `Overlay` (clavier à la demande) : titre, texte, journal brut, au plus trois boutons, fermeture par bouton | 620 × 440 px | `src/ui/problem.rs:18-19,58-63,71-86,168-207` |
| Authentification polkit | Fenêtre `Overlay` : « Authentification requise », champ, Détails, Annuler, Authentifier | 460 × 268 px | `src/ui/polkit.rs:37-38,282,353,369,385,511-517` |
| Vues défilantes | Menus longs bornés à l'écran et défilants ; défilement dans tiroir, recherche, assistant, visite, alerte ; fondu de bord sur la seule grille du lanceur | fondu 14 px ; barre de 4 px dans la navigation des Réglages | `src/ui/plugins/bar/popover.rs:1318,1377` ; `src/ui/tokens.rs:454-491` ; `src/ui/plugins/launcher/grid.rs:446-447` ; `src/ui/panel.rs:893-900,1025-1031` ; `src/ui/search.rs:1552` ; `src/ui/chat.rs:1610` ; `src/ui/wizard.rs:1048` |
| Fenêtres (côté Hyprland) | Bordure d'accent pour la fenêtre active, de fond pour les autres ; barres de titre hyprbars seulement en plein bureau, option coupée par défaut : fermer, agrandir, réduire | barre 34 px, texte 11 | `src/ui/hyprset.rs:578-589` ; `src/hyprbars.rs:1-12,95-141` |
| Lanceur (feuille) | Grande feuille avec voile réglable, recherche et grille | réglages de forme | `src/ui/plugins/launcher.rs:115-117` ; `src/ui/plugins/launcher/grid.rs:603-605` |

### Écarts
| # | Composant | Nature | Force | Règle HIG (slug › section) | Réel dans hyprdesk (preuve) | Action proposée | Effort | Confiance |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| PRES-01 | Alertes problème et polkit | ÉCART | tableau de valeurs | alerts › Buttons — « Pressing Escape (Esc) or Command-Period (.) on an attached keyboard » (annulation d'une alerte) | `src/ui/mod.rs:5385-5401` : Échap traite le menu d'icône, la carte, le tiroir, l'assistant, puis désélectionne ; aucune branche pour l'alerte problème, dont `PrMsg::Close` ne vient que du bouton (`src/ui/problem.rs:190-193`). `src/ui/polkit.rs` : aucune touche Échap dans le fichier, seul « Annuler » ferme (`:385-391`) | Fermer l'alerte problème et annuler polkit sur Échap (`PrMsg::Close`, `PMsg::Cancel`) | S | haute |
| PRES-02 | Cartes de la barre : changement de taille | PISTE | « Provide » (impératif, non suivi sur le cadre seul) | popovers › Best practices — « Provide a smooth transition when changing the size of a popover. » | D'une carte à l'autre, le contenu se fond en 160 ms (`src/ui/plugins/bar/popover.rs:771`) mais la surface prend sa taille finale d'un bloc (`:792`). Ouverture d'une confirmation : 40 px ajoutés sans transition (`:525`, `:1154-1192`). Seuls les panneaux rattachés du style Convex interpolent la hauteur (`:1176-1177`) | Animer la hauteur de la carte hors style Convex comme celle des panneaux rattachés | M | moyenne |
| PRES-03 | Barres de titre hyprbars | PISTE | transposition ; l'impératif « Make sure » porte sur la fenêtre entière, distinguée par sa bordure | windows › macOS window states — « the key window uses color in the title bar options for closing, minimizing, and zooming; inactive windows and main windows that aren't key use gray in these options » | `src/hyprbars.rs:121` : un seul `fg_color` pour fermer, agrandir et réduire ; `bar_color` unique (`:131`). La fenêtre active ne se distingue que par sa bordure (`src/ui/hyprset.rs:578-589`). Option coupée par défaut (`src/hyprbars.rs:4`) | Quand hyprbars est activée, dessiner la barre de titre et ses boutons en teinte atténuée sur les fenêtres sans focus | M | moyenne |
| PRES-04 | Tiroir des Réglages : barres de défilement | ÉCART | impératif (« Be sure ») | scroll-views › macOS — « Be sure to use the same size for all controls in such a panel. » | `src/ui/panel.rs:893-900` : barre de 4 px dans la navigation ; `:1025-1031` : zone de contenu sans barre explicite, à la taille par défaut d'Iced. Aucun style global de défilement ne les rapproche (seul `continuity.rs:241-245` définit un style) | Donner la même barre de 4 px à la zone de contenu | S | moyenne |
| PRES-05 | Alerte problème : titre | PISTE | « describe what happened, the context in which it happened, and why » | alerts › Content | `src/ui/problem.rs:169-172` : titre fixe « Un problème est survenu » ; l'origine (« Mises à jour », `:42`) ne sert que de titre de conversation (`:67`) ; la cause est dans le journal brut (`:152-160`) | Composer le titre avec l'origine et la nature du problème (par exemple « La mise à jour a échoué ») | S | haute |
| PRES-06 | Alerte problème : texte d'accompagnement | ÉCART | impératif (« Avoid ») | alerts › Content — « Avoid explaining alert buttons. » | `src/ui/problem.rs:173-175` : « Le détail ci-dessous peut être copié pour le chercher ou le confier à un assistant. » explique les boutons « Copier » et « Demander à LYRA » (`:194-203`) | Retirer cette phrase | S | moyenne |
| PRES-07 | Alerte problème : informe seulement | PISTE | « Avoid » et « prefer » | alerts › Best practices — « Avoid using an alert merely to provide information. » | `src/ui/problem.rs:168-203` : aucune décision à prendre, deux actions d'assistance ; ouverte par une commande extérieure (`src/ui/mod.rs:3931-3932,3955`) | Remplacer la fenêtre par un indicateur discret dans la page d'origine, qui ouvre le détail | M | moyenne |
| PRES-08 | Dock : clic sur une application groupée | PISTE | transposition | windows › macOS window states — « only the most recently accessed window becomes key » | `src/ui/plugins/dock/surface.rs:1684-1700` : un clic donne le focus à « sa fenêtre suivante, à tour de rôle » | Donner le focus à la fenêtre la plus récemment utilisée de l'application | S | moyenne |
| PRES-09 | Confirmation dans les menus de la barre | PISTE | « Aim for », « Prefer » | alerts › Buttons — « Aim for a one- or two-word title that describes the result of selecting the button. » | `src/ui/plugins/bar/popover.rs:1587` : bouton « Confirmer » quelle que soit l'entrée ; question « Confirmer ? » par défaut (`:1551`) | Intituler le bouton avec le verbe de l'entrée validée | S | haute |
| PRES-10 | Assistant de configuration, étape Récapitulatif | ÉCART | impératif (« Avoid ») | sheets › Best practices — « Avoid showing all three buttons — Cancel, Done, and Back — together. » | `src/ui/wizard.rs:980-1043` : à cette étape, « Retour » (`:982-990`), « Annuler l'assistant » (`:994-1022`) et « Terminer » (`:1035-1042`) sont affichés ensemble | Masquer « Retour » à cette étape (les pastilles numérotées de l'en-tête, `:876-929`, permettent déjà de revenir) | S | haute |
| PRES-11 | Visite guidée : points d'avancement | ÉCART | impératif (« Center ») | page-controls › Best practices — « Center a page control at the bottom of the view or window. » | `src/ui/tour.rs:543` : `row![dots, Space, boutons]` — les dix points sont en tête de rangée, à gauche des boutons | Placer les points sur leur propre rangée, centrée sous celle des boutons | S | haute |
| PRES-12 | Fondu de bord de la grille du lanceur | PISTE | impératif (« Only use »), atténué : le fondu sert de signe de débordement | scroll-views › Scroll edge effects — « Only use a scroll edge effect when a scroll view is behind floating interface elements. » | `src/ui/tokens.rs:454-491` : dégradé de 14 px posé sur la liste, en haut et en bas. Unique emploi : `src/ui/plugins/launcher/grid.rs:446-447`, grille empilée dans une colonne avec le champ de recherche (`:603-605`), sans élément flottant au-dessus | Ne plus appliquer `scroll_edges` à la grille du lanceur | S | moyenne |
| PRES-13 | Alerte problème répétée | ABSENT | « Configure repeating alerts » | alerts › macOS — « Configure repeating alerts to let people suppress subsequent occurrences of the same alert. » | `src/ui/problem.rs:58-63` : une seule fenêtre, son texte est remplacé à chaque signalement ; aucune option de non-réaffichage (recherche « ne plus afficher », « show again » dans `src/ui` : rien) | Ajouter à l'alerte problème une option « Ne plus signaler ce problème » | S | moyenne |
| PRES-14 | Alerte problème : image | ABSENT | « macOS automatically displays your app icon in an alert » | alerts › macOS | `src/ui/problem.rs:168-207` : titre, texte, détail, boutons ; aucune image | Afficher l'icône de hyprdesk (ou un symbole) à gauche du titre | S | haute |
| PRES-15 | Cartes de la barre : épinglage | PISTE | « Consider » | popovers › macOS — « Consider letting people detach a popover. » | Aucune fonction d'épinglage ou de détachement dans `src/ui/plugins/bar/popover.rs` ni `continuity.rs` (recherche « detach », « épingl », « pin » : rien) ; tout clic hors carte ferme (`popover.rs:1194-1202`) | Laisser une carte se détacher en panneau flottant d'aspect identique | L | moyenne |
| PRES-16 | Cartes de la barre et bulles du dock : pas de flèche | IDENTITÉ | impératif (« Make sure »), non suivi par choix documenté | popovers › Best practices — « Make sure a popover's arrow points as directly as possible to the element that revealed it. » | `src/ui/plugins/bar/popover.rs:405-411` : carte centrée sur la boîte rendue de la commande, à 10 px de la barre, sans pointeur (test « anchored without a pointer », `:158`) ; bulles à 8 px de l'icône (`src/ui/plugins/dock/lifecycle.rs:139-142`). `DESIGN.md` § Barre « Continuité » : « Cartes ancrées aux limites mesurées des commandes » | Aucune — choix documenté, à signaler seulement | — | haute |

### Ce qui est déjà conforme
- Un seul popover à la fois, passage de l'un à l'autre en un clic | `self.popover: Option<Popover>` (`src/ui/plugins/bar/popover.rs:342`), surface réutilisée (`:756-795`), capteur qui épargne la barre (`:816-831`).
- Fermeture au clic extérieur ou après un choix | `popover.rs:1194-1202` ; seuls `Spawn`, `Submit` et `Confirm` ferment (`:1079`, `:1112`, `:1144`).
- Popover non modal, ouvert tant qu'on règle | `PMsg` n'a aucun message de fermeture pour les curseurs (`popover.rs:320-331`).
- L'icône qui ouvre la carte n'est pas recouverte | carte à 10 px de la barre (`popover.rs:371-380,405-411`) ; bulle à 8 px (`src/ui/plugins/dock/lifecycle.rs:139-142`).
- Taille bornée à l'écran, menus longs défilants | `popover.rs:679-690`, `:1163-1171`, `:1318`, `:1377`.
- Une seule feuille ou un seul tiroir à la fois | `src/ui/panel.rs:199-201` ; `src/ui/chat.rs:458` ; `src/ui/wizard.rs:455`.
- Feuille modale, parent assombri | `src/ui/wizard.rs:468-476` ; `src/ui/overview.rs:155-160,777`.
- Flux long dans une surface distincte | assistant de configuration plein écran (`src/ui/wizard.rs:2,865`).
- « Terminer » va de pair avec « Annuler » ; Échap ne discarde rien | `src/ui/wizard.rs:994-1042` ; Échap et croix ferment en conservant les choix (`src/ui/mod.rs:5311-5316` ; `src/ui/wizard.rs:930-933`). Visite : « Terminer » à la dernière étape seulement, où « Quitter la visite » disparaît (`src/ui/tour.rs:546-582`).
- Alerte : trois boutons au plus, titre court | `src/ui/problem.rs:169,190-203` ; `src/ui/polkit.rs:282,353-391`.
- Alerte : annulation côté début, action principale côté fin | `src/ui/problem.rs:190-203` ; « Annuler » puis « Confirmer » (`popover.rs:1562-1606`).
- Alerte : champ de texte seulement quand la résolution l'exige | `src/ui/polkit.rs:282-360`.
- Alerte : parcimonie | une entrée de menu ne demande confirmation que si son auteur le demande (`popover.rs:1094-1101`).
- Symbole de prudence : aucun employé | `src/ui/problem.rs:168-207`.
- Pas de vue défilante dans une vue de même orientation, dans les surfaces parcourues | `src/ui/panel.rs:893-900,1025-1031` (volets côte à côte) ; `continuity.rs:341-362`.
- Fenêtres : apparences différentes selon l'état | `src/ui/hyprset.rs:578-589`.
- Fenêtres : fermer, agrandir, réduire dans la barre de titre | `src/hyprbars.rs:108-131`.

### À vérifier sur un rendu
- Barres de défilement : visibles en permanence avec une piste dès que le contenu déborde (taille et style par défaut d'Iced, hors centre de contrôle sans piste) ; le tiroir, la recherche et l'alerte ont-ils le même indicateur ?
- OSD (calque `Overlay`, `src/ui/osd.rs:90-93`) affiché par-dessus le centre de contrôle ouvert (calque `Overlay`, `popover.rs:850`) quand on touche au volume.
- Menu de fenêtre ouvert depuis la bulle du dock : la bulle reste-t-elle affichée (cascade de popovers) ?
- Changement de carte : le saut du cadre sous le fondu de contenu est-il perceptible (PRES-02) ?
- Pied de l'assistant de configuration sur carte étroite (moins de 700 px, `src/ui/wizard.rs:872`) : densité des boutons.
- Tiroir des Réglages sur écran de moins de 1200 px (pleine largeur, `src/ui/panel.rs:639-645`) : voit-on qu'une fenêtre reste active dessous ?
- Lisibilité réelle des cartes à opacité 0,94 et du tiroir en verre.
- Dock : application dont les fenêtres sont réparties sur plusieurs espaces, après un clic sur son icône groupée.
