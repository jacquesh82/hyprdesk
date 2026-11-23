# Entrées : clavier, pointeur, gestes et focus — macOS d'après la HIG, face à hyprdesk

> Proposition de design, rien d'implémenté. Plan d'ensemble et priorités : `plan-d-action.md` ; méthode et périmètre : `README.md`.

> Référentiel : Human Interface Guidelines d'Apple, export Markdown officiel relevé le 23 novembre 2026. Code : commit `00f6d69` (arbre de travail du 23 novembre 2026, avec des modifications non commitées).

## Première partie — Spécification d'après la HIG

### Résumé

- Ces quatre pages fixent presque aucune cote : ni taille de cible de clic, ni délai de survol ou de double-clic, ni vitesse de défilement, ni seuil de geste. Les seules valeurs fermes sont des combinaisons de touches, un ordre de modificatrices, un nombre de doigts pour quelques gestes système et deux rendus de focus.
- Clavier : un tableau de 116 lignes de raccourcis standard que chacun attend identiques dans tout le système, l'interdiction de les détourner, Control réservée au système et la demande de prendre en charge l'accès complet au clavier (Full Keyboard Access).
- Pointeur : sur Mac, souris ou pavé tactile se combinent au clavier ; sept interactions sont communes aux deux dispositifs, neuf sont propres au pavé tactile ; tout est personnalisable ; un jeu de pointeurs standard exprime l'état interactif et le résultat d'un déplacement.
- Gestes : le toucher est un mode secondaire ; aucun geste ne doit être le seul chemin vers une action importante ; les gestes système ne se redéfinissent pas ; un geste indisponible se signale.
- Focus : anneau pour les champs, surbrillance pour les listes ; le focus ne se déplace pas sans interaction ; en mode accès clavier, seuls les éléments de contenu demandent un focus propre à l'app.
- Lacunes : rien ici sur les menus, les claviers virtuels ou le texte prédictif ; la page Focus n'a pas de bloc macOS dédié ; les glyphes des symboles de modificateurs sont des images dans la source.

### Keyboards (`keyboards`)

**Rôle** — Définir ce qu'une personne attend d'un clavier physique : des raccourcis standard identiques dans tout le système, des raccourcis personnalisés réservés aux commandes fréquentes, un emploi prévisible des touches modificatrices.

**Anatomie et cotes** — Tailles et durées : non chiffré dans la HIG. Seules valeurs formelles : quatre modificatrices (Control, Option, Shift, Command) et leur ordre d'affichage — "always list them in this order: Control, Option, Shift, Command." — keyboards › Custom keyboard shortcuts.

**Règles**

- Définition : un raccourci clavier (keyboard shortcut) est l'association d'une commande à une touche principale et à une ou plusieurs modificatrices — "a combination of a primary key and one or more modifier keys (Control, Option, Shift, and Command) that map to a specific command" — keyboards › Discussion ; dans un jeu, une association de touches (key binding) se réduit souvent à une seule touche — "often consists of a single key" — keyboards › Discussion.
- Accès clavier : prendre en charge l'accès complet au clavier (Full Keyboard Access) quand c'est possible, qui permet de naviguer et d'activer fenêtres, menus, contrôles et fonctions système au seul clavier — "Full Keyboard Access lets people navigate and activate windows, menus, controls, and system features using only the keyboard." — keyboards › Best practices ; pour le tester, l'activer dans Réglages, zone Accessibilité — "turn it on in the Accessibility area of the system-supplied Settings app" — keyboards › Best practices.
- Raccourcis standard : Apple les définit pour qu'ils fonctionnent de façon cohérente dans le système et la plupart des apps et que les connaissances se transfèrent — "Apple defines standard keyboard shortcuts to work consistently across the system and most apps, helping people transfer their knowledge to new experiences." — keyboards › Discussion ; dans la plupart des apps, on compte sur eux — "While using most apps, people generally expect to rely on the standard keyboard shortcuts that work in other apps and throughout the system." — keyboards › Best practices.
- Raccourcis standard : pour une action propre à l'app que l'on effectue souvent, créer un raccourci personnalisé plutôt que de détourner un raccourci standard — "instead of repurposing a standard one that people associate with a different action" — keyboards › Best practices ; dans un jeu, chacun attend en plus de pouvoir modifier les associations de touches — "they also expect to be able to modify each game’s key bindings to fit their personal play style" — keyboards › Best practices.
- Raccourcis standard : en général, ne pas réaffecter un raccourci standard à une action personnalisée — "In general, don’t repurpose standard keyboard shortcuts for custom actions." — keyboards › Standard keyboard shortcuts ; un raccourci connu qui se comporte autrement désoriente — "People can get confused when the shortcuts they know work differently in your app or game." — keyboards › Standard keyboard shortcuts.
- Raccourcis standard : ne redéfinir un raccourci standard que si son action n'a pas de sens dans l'expérience (exemple de la HIG : une app sans édition de texte peut réaffecter à Get Info la combinaison de la mise en italique) — "Only consider redefining a standard shortcut if its action doesn’t make sense in your experience." — keyboards › Standard keyboard shortcuts.
- Raccourcis personnalisés : n'en définir que pour les commandes propres à l'app les plus utilisées — "Define custom keyboard shortcuts for only the most frequently used app-specific commands." — keyboards › Custom keyboard shortcuts ; en définir trop rend l'app difficile à apprendre — "defining too many new shortcuts can make your app seem difficult to learn" — keyboards › Custom keyboard shortcuts.
- Modificatrices, emploi attendu : Command pendant un déplacement déplace les éléments en groupe, Shift pendant un redimensionnement contraint au rapport hauteur/largeur — "pressing Command while dragging moves items as a group, and pressing Shift while drag-resizing constrains resizing to the item’s aspect ratio" — keyboards › Custom keyboard shortcuts.
- Modificatrices, déplacement au clavier : une flèche maintenue déplace l'élément sélectionné de la plus petite unité de distance définie par l'app — "holding an arrow key moves the selected item by the smallest app-defined unit of distance until people release the key" — keyboards › Custom keyboard shortcuts.
- Modificatrices, ordre : un raccourci qui en emploie plusieurs les liste toujours dans cet ordre : Control, Option, Shift, Command — "If you use more than one modifier key in a custom shortcut, always list them in this order: Control, Option, Shift, Command." — keyboards › Custom keyboard shortcuts.
- Modificatrices, touches à deux caractères : ne pas ajouter Shift quand le raccourci utilise le caractère supérieur de la touche et écrire ce caractère — "Avoid adding Shift to a shortcut that uses the upper character of a two-character key." — keyboards › Custom keyboard shortcuts ; la HIG donne Command-Slash pour Hide Status Bar et Command-Question mark, et non Shift-Command-Slash, pour Help — "the keyboard shortcut for Hide Status Bar is Command-Slash, whereas the keyboard shortcut for Help is Command-Question mark, not Shift-Command-Slash" — keyboards › Custom keyboard shortcuts.
- Modificatrices, claviers locaux : certaines langues exigent une modificatrice pour produire des caractères (la HIG cite le clavier français, où Option-5 produit une accolade ouvrante) ; Command reste sûre — "It’s usually safe to use the Command key as a modifier, but avoid using an additional modifier with characters that aren’t available on all keyboards." — keyboards › Custom keyboard shortcuts ; sinon, limiter les autres modificatrices aux lettres — "If you must use a modifier other than Command, prefer using it only with the alphabetic characters." — keyboards › Custom keyboard shortcuts.
- Dérivation : ne pas créer de raccourci en ajoutant une modificatrice à un raccourci existant pour une commande sans rapport (exemple de la HIG : Shift-Command-Z pour autre chose qu'annuler ou rétablir) — "Avoid creating a new shortcut by adding a modifier to an existing shortcut for an unrelated command." — keyboards › Custom keyboard shortcuts.
- Localisation : laisser le système adapter les touches du raccourci au clavier connecté — "The system automatically localizes a shortcut’s primary and modifier keys to support the currently connected keyboard" — keyboards › Custom keyboard shortcuts ; et le refléter dans une mise en page droite-à-gauche — "if your app or game switches to a right-to-left layout, the system automatically mirrors the shortcut" — keyboards › Custom keyboard shortcuts.

**Spécifique macOS**

- Usage : sur Mac, le clavier physique est utilisé en permanence — "Mac users tend to use a physical keyboard all the time" — keyboards › Discussion.
- Bloc Platform considerations : aucune considération additionnelle pour macOS — "No additional considerations for iOS, iPadOS, macOS, or tvOS. Not supported in watchOS." — keyboards › Platform considerations.
- Symboles de modificateurs : la HIG les donne sous forme d'images ; seuls leurs textes alternatifs, cités ci-dessous, sont repris, les glyphes eux-mêmes ne figurent pas dans le texte de la page.

| Modificatrice | Symbole | Emploi recommandé |
|---|---|---|
| Command | "Outline of a stylized clover shape." — keyboards › Custom keyboard shortcuts (image `Keyboard_Command.svg`) | En faire la modificatrice principale — "Prefer the Command key as the main modifier key in a custom keyboard shortcut." — keyboards › Custom keyboard shortcuts |
| Shift | "Outline of an upward-pointing arrow." — keyboards › Custom keyboard shortcuts (image `Keyboard_Shift.svg`) | En faire la modificatrice secondaire d'un raccourci lié — "Prefer the Shift key as a secondary modifier that complements a related shortcut." — keyboards › Custom keyboard shortcuts |
| Option | "Line segments that suggest a horizontally transformed Z shape combined with a short horizontal segment aligned with the top of the Z." — keyboards › Custom keyboard shortcuts (image `Keyboard_Option.svg`) | L'employer avec parcimonie, pour les commandes moins courantes ou les fonctions avancées — "Use the Option modifier sparingly for less-common commands or power features." — keyboards › Custom keyboard shortcuts |
| Control | "A shallow, upside-down V shape." — keyboards › Custom keyboard shortcuts (image `Keyboard_Control.svg`) | Éviter de l'employer : le système l'utilise dans beaucoup de fonctions et raccourcis système, comme déplacer le focus ou capturer l'écran — "Avoid using the Control key as a modifier. The system uses Control in many systemwide features and shortcuts, like moving focus or capturing screenshots." — keyboards › Custom keyboard shortcuts |

- Raccourcis standard : chacun attend du tableau suivant qu'il produise l'action indiquée — "People expect each of the following standard keyboard shortcuts to perform the action listed in the table below." — keyboards › Standard keyboard shortcuts. Les 116 lignes sont recopiées ; seules les actions sont traduites, les combinaisons et la colonne de regroupement Touche sont telles quelles.

| Touche | Raccourci | Action |
|---|---|---|
| Space | Command-Space | Afficher ou masquer le champ de recherche Spotlight. |
|  | Shift-Command-Space | Variable. |
|  | Option-Command-Space | Afficher la fenêtre de résultats de recherche Spotlight. |
|  | Control-Command-Space | Afficher la fenêtre des caractères spéciaux. |
| Tab | Shift-Tab | Parcourir les contrôles en sens inverse. |
|  | Command-Tab | Avancer vers l'app suivante la plus récemment utilisée dans une liste d'apps ouvertes. |
|  | Shift-Command-Tab | Reculer dans une liste d'apps ouvertes (triées par utilisation récente). |
|  | Control-Tab | Déplacer le focus sur le groupe de contrôles suivant d'une boîte de dialogue ou le tableau suivant (quand Tab passe à la cellule suivante). |
|  | Control-Shift-Tab | Déplacer le focus sur le groupe de contrôles précédent. |
| Esc | Esc | Annuler l'action ou le processus en cours. |
| Esc | Option-Command-Esc | Ouvrir la boîte de dialogue Forcer à quitter. |
| Eject | Control-Command-Eject | Quitter toutes les apps (après enregistrement des modifications des documents ouverts) et redémarrer l'ordinateur. |
|  | Control-Option-Command-Eject | Quitter toutes les apps (après enregistrement des modifications des documents ouverts) et éteindre l'ordinateur. |
| F1 | Control-F1 | Activer ou désactiver l'accès complet au clavier. |
| F2 | Control-F2 | Déplacer le focus sur la barre des menus. |
| F3 | Control- F3 | Déplacer le focus sur le Dock. |
| F4 | Control-F4 | Déplacer le focus sur la fenêtre active (ou suivante). |
|  | Control-Shift-F4 | Déplacer le focus sur la fenêtre active précédente. |
| F5 | Control-F5 | Déplacer le focus sur la barre d'outils. |
|  | Command-F5 | Activer ou désactiver VoiceOver. |
| F6 | Control-F6 | Déplacer le focus sur le premier panneau (ou le suivant). |
|  | Control-Shift-F6 | Déplacer le focus sur le panneau précédent. |
| F7 | Control-F7 | Outrepasser temporairement le mode d'accès clavier en cours dans les fenêtres et les boîtes de dialogue. |
| F8 |  | Variable. |
| F9 |  | Variable. |
| F10 |  | Variable. |
| F11 |  | Afficher le bureau. |
| F12 |  | Masquer ou afficher Dashboard. |
| Grave accent (\`) | Command-Grave accent | Activer la fenêtre ouverte suivante de l'app au premier plan. |
|  | Shift-Command-Grave accent | Activer la fenêtre ouverte précédente de l'app au premier plan. |
|  | Option-Command-Grave accent | Déplacer le focus sur le tiroir de la fenêtre. |
| Hyphen (-) | Command-Hyphen | Réduire la taille de la sélection. |
|  | Option-Command-Hyphen | Dézoomer quand le zoom d'écran est activé. |
| Left bracket ({) | Command-Left bracket | Aligner une sélection à gauche. |
| Right bracket (}) | Command-Right bracket | Aligner une sélection à droite. |
| Pipe (\|) | Command-Pipe | Centrer une sélection. |
| Colon (:) | Command-Colon | Afficher la fenêtre d'orthographe. |
| Semicolon (;) | Command-Semicolon | Rechercher les mots mal orthographiés du document. |
| Comma (,) | Command-Comma | Ouvrir la fenêtre des réglages de l'app. |
|  | Control-Option-Command-Comma | Réduire le contraste de l'écran. |
| Period (.) | Command-Period | Annuler une opération. |
|  | Control-Option-Command-Period | Augmenter le contraste de l'écran. |
| Question mark (?) | Command-Question mark | Ouvrir le menu Aide de l'app. |
| Forward slash (/) | Option-Command-Forward slash | Activer ou désactiver le lissage des polices. |
| Equal sign (=) | Shift-Command-Equal sign | Augmenter la taille de la sélection. |
|  | Option-Command-Equal sign | Zoomer quand le zoom d'écran est activé. |
| 3 | Shift-Command-3 | Capturer l'écran dans un fichier. |
|  | Control-Shift-Command-3 | Capturer l'écran dans le presse-papiers. |
| 4 | Shift-Command-4 | Capturer une sélection dans un fichier. |
|  | Control-Shift-Command-4 | Capturer une sélection dans le presse-papiers. |
| 8 | Option-Command-8 | Activer ou désactiver le zoom d'écran. |
|  | Control-Option-Command-8 | Inverser les couleurs de l'écran. |
| A | Command-A | Sélectionner tous les éléments d'un document ou d'une fenêtre, ou tous les caractères d'un champ de texte. |
|  | Shift-Command-A | Désélectionner toutes les sélections ou tous les caractères. |
| B | Command-B | Mettre le texte sélectionné en gras, ou activer et désactiver le gras. |
| C | Command-C | Copier la sélection dans le presse-papiers. |
|  | Shift-Command-C | Afficher la fenêtre Couleurs. |
|  | Option-Command-C | Copier le style du texte sélectionné. |
|  | Control-Command-C | Copier les réglages de mise en forme de la sélection et les stocker dans le presse-papiers. |
| D | Option-Command-D | Afficher ou masquer le Dock. |
|  | Control-Command-D | Afficher la définition du mot sélectionné dans l'app Dictionnaire. |
| E | Command-E | Utiliser la sélection pour une opération de recherche. |
| F | Command-F | Ouvrir une fenêtre de recherche. |
|  | Option-Command-F | Atteindre le champ de recherche. |
|  | Control-Command-F | Passer en plein écran. |
| G | Command-G | Rechercher l'occurrence suivante de la sélection. |
|  | Shift-Command-G | Rechercher l'occurrence précédente de la sélection. |
| H | Command-H | Masquer les fenêtres de l'app en cours d'utilisation. |
|  | Option-Command-H | Masquer les fenêtres de toutes les autres apps en cours d'exécution. |
| I | Command-I | Mettre en italique le texte sélectionné, ou activer et désactiver l'italique. |
|  | Command-I | Afficher une fenêtre d'informations. |
|  | Option-Command-I | Afficher une fenêtre d'inspection. |
| J | Command-J | Faire défiler jusqu'à une sélection. |
| M | Command-M | Réduire la fenêtre active dans le Dock. |
|  | Option-Command-M | Réduire toutes les fenêtres de l'app active dans le Dock. |
| N | Command-N | Ouvrir un nouveau document. |
| O | Command-O | Afficher une boîte de dialogue pour choisir un document à ouvrir. |
| P | Command-P | Afficher la boîte de dialogue d'impression. |
|  | Shift-Command-P | Afficher la boîte de dialogue de mise en page. |
| Q | Command-Q | Quitter l'app. |
|  | Shift-Command-Q | Déconnecter la personne actuellement connectée. |
|  | Option-Shift-Command-Q | Déconnecter la personne actuellement connectée sans confirmation. |
| S | Command-S | Enregistrer un nouveau document ou enregistrer une version d'un document. |
|  | Shift-Command-S | Dupliquer le document actif ou lancer un Enregistrer sous. |
| T | Command-T | Afficher la fenêtre Polices. |
|  | Option-Command-T | Afficher ou masquer une barre d'outils. |
| U | Command-U | Souligner le texte sélectionné, ou activer et désactiver le soulignement. |
| V | Command-V | Coller le contenu du presse-papiers au point d'insertion. |
|  | Shift-Command-V | Coller sous une autre forme (par exemple Coller comme citation). |
|  | Option-Command-V | Appliquer le style d'un objet à la sélection. |
|  | Option-Shift-Command-V | Coller le contenu du presse-papiers au point d'insertion et appliquer au contenu inséré le style du texte environnant. |
|  | Control-Command-V | Appliquer des réglages de mise en forme à la sélection. |
| W | Command-W | Fermer la fenêtre active. |
|  | Shift-Command-W | Fermer un fichier et ses fenêtres associées. |
|  | Option-Command-W | Fermer toutes les fenêtres de l'app. |
| X | Command-X | Retirer la sélection et la stocker dans le presse-papiers. |
| Z | Command-Z | Annuler l'opération précédente. |
|  | Shift-Command-Z | Rétablir (quand Annuler et Rétablir sont deux commandes distinctes au lieu d'être basculées avec Command-Z). |
| Right arrow | Command-Right arrow | Passer la disposition clavier à la disposition courante de l'écriture latine (Roman script). |
|  | Shift-Command-Right arrow | Étendre la sélection à l'unité sémantique suivante, généralement la fin de la ligne en cours. |
|  | Shift-Right arrow | Étendre la sélection d'un caractère vers la droite. |
|  | Option-Shift-Right arrow | Étendre la sélection jusqu'à la fin du mot en cours, puis jusqu'à la fin du mot suivant. |
|  | Control-Right arrow | Déplacer le focus vers une autre valeur ou cellule d'une vue, comme un tableau. |
| Left arrow | Command-Left arrow | Passer la disposition clavier à la disposition courante de l'écriture du système (system script). |
|  | Shift-Command-Left arrow | Étendre la sélection à l'unité sémantique précédente, généralement le début de la ligne en cours. |
|  | Shift-Left arrow | Étendre la sélection d'un caractère vers la gauche. |
|  | Option-Shift-Left arrow | Étendre la sélection jusqu'au début du mot en cours, puis jusqu'au début du mot précédent. |
|  | Control-Left arrow | Déplacer le focus vers une autre valeur ou cellule d'une vue, comme un tableau. |
| Up arrow | Shift-Command-Up arrow | Étendre la sélection vers le haut à l'unité sémantique suivante, généralement le début du document. |
|  | Shift-Up arrow | Étendre la sélection à la ligne du dessus, jusqu'à la limite de caractère la plus proche à la même position horizontale. |
|  | Option-Shift-Up arrow | Étendre la sélection jusqu'au début du paragraphe en cours, puis jusqu'au début du paragraphe suivant. |
|  | Control-Up arrow | Déplacer le focus vers une autre valeur ou cellule d'une vue, comme un tableau. |
| Down arrow | Shift-Command-Down arrow | Étendre la sélection vers le bas à l'unité sémantique suivante, généralement la fin du document. |
|  | Shift-Down arrow | Étendre la sélection à la ligne du dessous, jusqu'à la limite de caractère la plus proche à la même position horizontale. |
|  | Option-Shift-Down arrow | Étendre la sélection jusqu'à la fin du paragraphe en cours, puis jusqu'à la fin du paragraphe suivant (inclure le terminateur de paragraphe, comme Retour, dans les opérations couper, copier et coller). |
|  | Control-Down arrow | Déplacer le focus vers une autre valeur ou cellule d'une vue, comme un tableau. |

- Raccourcis de localisation : le système en définit aussi pour les versions localisées, claviers, dispositions et méthodes de saisie localisés — "The system also defines several keyboard shortcuts for use with localized versions of the system, localized keyboards, keyboard layouts, and input methods." — keyboards › Standard keyboard shortcuts ; ils ne correspondent pas directement à des commandes de menu — "These shortcuts don’t correspond directly to menu commands." — keyboards › Standard keyboard shortcuts.

| Raccourci | Action |
|---|---|
| Control-Space | Basculer entre la source de saisie en cours et la précédente. |
| Control-Option-Space | Passer à la source de saisie suivante de la liste. |
| [Modifier key]-Command-Space | Variable. |
| Command-Right arrow | Passer la disposition clavier à la disposition courante de l'écriture latine. |
| Command-Left arrow | Passer la disposition clavier à la disposition courante de l'écriture du système. |

- Menus : ces pages ne contiennent aucune règle sur la conception des menus ni sur l'affichage des raccourcis dans les menus.
- Claviers virtuels et texte prédictif : la page ne fait que renvoyer à la page Virtual keyboards, absente du condensé, et ne dit rien sur le texte prédictif.
- Bizarreries conservées de la source : `Control- F3` comporte un espace ; la ligne Shift-Tab n'a pas de pendant pour Tab seul ; Command-I apparaît deux fois (italique, fenêtre d'informations) ; Option-Shift-Up arrow est décrit comme allant au début du paragraphe en cours, puis du paragraphe suivant.

**À transposer dans un shell Linux**

- Raccourcis globaux : constituer pour le shell (barre, dock, lanceur, espaces, notifications, verrouillage, réglages) un jeu de raccourcis système stable, listé dans les Réglages, jamais réaffecté par une mise à jour ; n'y rien emprunter à ce que les apps utilisent déjà.
- Entrée clavier dans le shell : chaque surface (barre, dock, lanceur, centre de notifications, fenêtre de réglages) reçoit un raccourci qui pose le focus dessus, sur le modèle de Control-F2 (barre des menus), Control-F3 (Dock), Control-F4 (fenêtre), Control-F5 (barre d'outils), Control-F6 (panneau), plus une bascule globale d'accès complet au clavier (modèle : Control-F1) et une bascule temporaire (modèle : Control-F7).
- Lanceur et recherche : un raccourci global affiche ou masque le champ de recherche (modèle : Command-Space), une variante ouvre la fenêtre de résultats (modèle : Option-Command-Space), Échap annule et referme (modèle : Esc) ; Command-Period sert de modèle d'annulation d'une opération.
- Bascule entre applications : un sélecteur des apps ouvertes ordonnées par usage récent, avec sens avant et arrière (modèle : Command-Tab et Shift-Command-Tab).
- Fenêtres et visibilité : raccourcis globaux pour masquer l'app courante, masquer les autres apps, réduire la fenêtre active et toutes les fenêtres de l'app, fermer la fenêtre active, afficher ou masquer la barre ou le dock, afficher le bureau, passer en plein écran (modèles : Command-H, Option-Command-H, Command-M, Option-Command-M, Command-W, Option-Command-D, F11, Control-Command-F).
- Réglages et blocage : ouvrir les réglages du shell avec la combinaison des réglages des apps (modèle : Command-Comma) ; offrir une page de personnalisation des raccourcis qui affiche les modificatrices dans un ordre constant et signale les conflits ; une boîte de dialogue pour forcer la fermeture d'une app bloquée (modèle : Option-Command-Esc) et une déconnexion avec ou sans confirmation (modèles : Shift-Command-Q, Option-Shift-Command-Q).

### Pointing devices (`pointing-devices`)

**Rôle** — Définir ce qu'une personne attend d'un dispositif de pointage (pointing device), souris ou pavé tactile : précision, souplesse, cohérence des clics et des gestes dans tout le système.

**Anatomie et cotes** — Tailles et durées : non chiffré dans la HIG (aucune taille de cible de clic ou tactile, aucun délai de double-clic, de survol ou de défilement, aucune vitesse, aucun seuil de geste). La seule durée évoquée est le remasquage d'une commande cachée après quelques secondes sans usage (voir Règles), sans valeur précise.

**Règles**

- Cohérence : réagir de la même façon aux gestes de souris et de pavé tactile — "Be consistent when responding to mouse and trackpad gestures." — pointing-devices › Best practices ; la plupart des gestes doivent fonctionner pareil dans tout le système — "People expect most gestures to work the same throughout the system, regardless of the app or game they’re using." — pointing-devices › Best practices.
- Cohérence : ne pas redéfinir les gestes système du pavé tactile ; même dans un jeu qui emploie des gestes propres, ils doivent rester disponibles — "Even in a game that uses app-specific gestures in a custom way, people expect systemwide gestures to be available" — pointing-devices › Best practices ; on fait en effet les gestes familiers pour afficher le Dock ou Mission Control — "people expect to make familiar gestures to reveal the Dock or Mission Control in macOS" — pointing-devices › Best practices.
- Cohérence : offrir la même expérience que l'on utilise gestes, regard, pointeur ou clavier — "Provide a consistent experience in your app, whether people are using gestures, eyes, a pointing device, or a keyboard." — pointing-devices › Best practices.
- Modificatrices : appuyer et maintenir une modificatrice pendant l'interaction doit produire le même résultat au toucher et au pointeur — "Provide a consistent experience when people press and hold a modifier key while interacting with objects in your app." — pointing-devices › Best practices.
- Survol : laisser le pointeur révéler et masquer les commandes qui se réduisent ou s'estompent automatiquement (l'exemple de la HIG, la barre d'outils réduite de Safari, est donné pour iPadOS) — "Let people use the pointer to reveal and hide controls that automatically minimize or fade out." — pointing-devices › Best practices.
- Commandes masquées, tactile : décider lesquelles ont besoin d'une prise en charge tactile et la réserver aux commandes essentielles — "Reserve touch support for essential controls." — pointing-devices › Touch support for hidden controls ; ni les informations statiques comme les infobulles (tooltips), ni ce qui est déjà accessible au toucher autrement — "In general, don’t add touch support for features that display static information, like tooltips, or for features people can already use another way with touch." — pointing-devices › Touch support for hidden controls ; et ne pas les laisser visibles en permanence pour accommoder le toucher — "Avoid keeping hidden controls visible all the time just to accommodate touch." — pointing-devices › Touch support for hidden controls.
- Commandes masquées, révélation : au toucher, taper l'emplacement général d'une commande cachée la révèle — "Reveal a hidden control when someone taps its general location on a touch display." — pointing-devices › Touch support for hidden controls ; seulement celles utiles au contenu en cours — "Reveal only controls relevant to the content someone is working with; too many at once can overwhelm and distract people." — pointing-devices › Touch support for hidden controls ; la masquer de nouveau après usage, ou après quelques secondes si elle n'est pas utilisée — "Hide a control again when the person finishes with it, or after a few seconds if they don’t use it." — pointing-devices › Touch support for hidden controls.
- Commandes masquées, sélection : conserver la sélection courante quand on révèle une commande cachée puis qu'on la touche — "keep the current selection when someone taps the area of a hidden control to reveal it" — pointing-devices › Touch support for hidden controls.
- Commandes masquées, menu contextuel (context menu) : rendre les commandes cachées essentielles disponibles dans un menu contextuel, qui dispense de trouver l'endroit exact où elles apparaissent — "A context menu gives people a dependable way to reach a control without finding the spot where it appears." — pointing-devices › Touch support for hidden controls ; l'inclure court — "Include the controls people need most, and keep the menu short." — pointing-devices › Touch support for hidden controls.

**Spécifique macOS**

- Association : sur Mac, on attend de combiner dispositif de pointage et clavier pour naviguer dans les apps et le système — "On a Mac, people typically expect to combine a pointing device with a keyboard as they navigate apps and the system." — pointing-devices › Discussion.
- Réglages : macOS prend en charge un large éventail d'interactions standard de souris et de pavé tactile que l'on peut personnaliser — "macOS supports a wide range of standard mouse and trackpad interactions that people can customize." — pointing-devices › macOS ; un clic ou un geste qui n'est pas un moyen principal d'interaction peut souvent être activé ou désactivé — "when a click or gesture isn’t a primary way to interact with content, people can often turn it on or off based on their current workflow" — pointing-devices › macOS ; on choisit des zones de la souris ou du pavé pour le clic secondaire et des combinaisons de doigts pour certains gestes — "People can also choose specific regions of a mouse or trackpad to invoke secondary clicks, and select specific finger combinations and movements for certain gestures." — pointing-devices › macOS. La HIG ne nomme aucun panneau de réglages ni ne dresse la liste des options.
- Rappel : les gestes des actions système sont personnalisables — "Remember that Mac users can customize the gestures for performing systemwide actions." — pointing-devices › Best practices.
- Bloc Platform considerations : aucune considération additionnelle pour iOS ; non pris en charge sur tvOS et watchOS — "No additional considerations for iOS. Not supported in tvOS or watchOS." — pointing-devices › Platform considerations.
- Interactions standard (source : pointing-devices › macOS) ; disponible dans la source = coché.

| Clic ou geste (nom HIG) | Comportement attendu | Souris | Pavé tactile |
|---|---|---|---|
| Clic principal (Primary click) | Sélectionner ou activer un élément, comme un fichier ou un bouton. | oui | oui |
| Clic secondaire (Secondary click) | Révéler les menus contextuels. | oui | oui |
| Défilement (Scrolling) | Déplacer le contenu haut, bas, gauche ou droite dans une vue. | oui | oui |
| Zoom intelligent (Smart zoom) | Zoomer ou dézoomer le contenu, comme une page web ou un PDF. | oui | oui |
| Balayer entre les pages (Swipe between pages) | Naviguer en avant ou en arrière entre des pages affichées une à une. | oui | oui |
| Balayer entre les apps en plein écran (Swipe between full-screen apps) | Naviguer en avant ou en arrière entre les apps en plein écran et les espaces. | oui | oui |
| Mission Control (double-tap de la souris à deux doigts ou balayage vers le haut du pavé tactile à trois ou quatre doigts) | Activer Mission Control. | oui | oui |
| Recherche et détecteurs de données (Lookup and data detectors ; appui forcé à un doigt ou toucher à trois doigts) | Afficher une fenêtre de recherche au-dessus du contenu sélectionné. | — | oui |
| Toucher pour cliquer (Tap to click) | Effectuer l'action du clic principal par un toucher plutôt qu'un clic. | — | oui |
| Appui forcé (Force click) | Cliquer puis appuyer fermement pour afficher une fenêtre Coup d'œil (Quick Look) ou de recherche au-dessus du contenu sélectionné ; une pression variable agit sur les commandes sensibles à la pression, comme des commandes multimédia à vitesse variable. | — | oui |
| Zoom avant ou arrière (pincer à deux doigts) | Zoomer ou dézoomer. | — | oui |
| Rotation (déplacer deux doigts en mouvement circulaire) | Tourner le contenu, comme une image. | — | oui |
| Centre de notifications (balayer depuis le bord du pavé tactile) | Afficher le Centre de notifications. | — | oui |
| App Exposé (balayer vers le bas à trois ou quatre doigts) | Afficher les fenêtres de l'app courante dans Exposé. | — | oui |
| Apps (pincer avec le pouce et trois doigts) | Afficher la liste des apps. | — | oui |
| Afficher le bureau (Show Desktop ; écarter le pouce et trois doigts) | Écarter toutes les fenêtres pour révéler le bureau. | — | oui |

- Pointeurs : macOS offre un éventail de styles de pointeur standard — "macOS offers a variety of standard pointer styles" — pointing-devices › Pointers ; ils communiquent l'état interactif d'un élément ou le résultat d'un déplacement — "to communicate the interactive state of an interface element or the result of a drag operation" — pointing-devices › Pointers. La HIG donne pour chacun l'API AppKit `NSCursor` et SwiftUI `PointerStyle`, non reprise ici.

| Pointeur (nom HIG) | Signification |
|---|---|
| Arrow (flèche) | Pointeur standard pour sélectionner et interagir avec le contenu et les éléments d'interface. |
| Open hand (main ouverte) | Le contenu peut être repositionné par déplacement dans la vue. |
| Closed hand (main fermée) | Déplacement en cours, pour repositionner le contenu dans la vue. |
| Context menu | Un menu contextuel est disponible pour le contenu sous le pointeur. |
| Pointing hand (main index) | Le contenu sous le pointeur est un lien URL vers une page web, un document ou un autre élément. |
| Cross (croix) | Sélection rectangulaire précise possible, comme une image dans Prévisualisation. |
| Horizontal I beam / Vertical I beam | Sélection et insertion de texte possibles dans une disposition horizontale / verticale. |
| Disappearing item | L'élément déplacé disparaîtra au relâchement ; l'original n'est pas affecté s'il existe. |
| Drag copy | Duplique, au lieu de déplacer, l'élément relâché dans la destination — "Appears when pressing the Option key during a drag operation." — pointing-devices › Pointers |
| Drag link | Crée un alias du fichier sélectionné relâché ; l'original reste en place — "Appears when pressing the Option and Command keys during a drag operation" — pointing-devices › Pointers |
| Operation not allowed | L'élément déplacé ne peut pas être relâché à cet endroit. |
| Zoom in / Zoom out (loupe avec signe + ou −) | Agrandir (zoom avant) un canevas ou un objet / dézoomer. |
| Resize down, east, left, north, northeast, northwest, right, south, southeast, southwest, up, west (12 variantes à un seul sens) | Un séparateur (down, up : horizontal ; left, right : vertical) ou un bord (east, west, north, south) ou un coin du cadre rectangulaire d'une fenêtre ne se déplace que dans ce sens ; pour un cadre, typiquement quand sa taille minimale ou maximale est atteinte. |
| Resize east west, left right, north south, northeast southwest, northwest southeast, up down (6 variantes bidirectionnelles) | Redimensionner une ligne (up down) ou une colonne (left right) en déplaçant un séparateur, ou un cadre rectangulaire comme une fenêtre en déplaçant un bord ou un coin, dans les deux sens. |

- Pointeur Context menu : généralement affiché seulement quand la touche Control est enfoncée — "This pointer is generally shown only when the Control key is pressed." — pointing-devices › Pointers.
- Commandes masquées, barre latérale (sidebar) : un toucher sur l'en-tête de section ou la ligne révèle les commandes cachées correspondantes, qui restent visibles jusqu'à ce que le toucher quitte la barre latérale ou qu'un dispositif de pointage soit de nouveau utilisé — "Reveal only the controls for the tapped item, and keep them visible until touch moves outside the sidebar or someone uses a pointing device again." — pointing-devices › Touch support for hidden controls.
- Commandes masquées, barre latérale : taper la zone d'une commande cachée la révèle sans l'activer ; elle n'agit qu'au toucher suivant — "If someone taps the area of a hidden control, reveal the control without activating it." — pointing-devices › Touch support for hidden controls.
- Commandes masquées, fenêtre à onglets (tabbed window) : toucher n'importe où dans la fenêtre révèle le bouton de fermeture de l'onglet sélectionné — "In a tabbed window, reveal the selected tab’s close button when someone taps anywhere in the window." — pointing-devices › Touch support for hidden controls ; il reste visible jusqu'à la reprise du dispositif de pointage — "Keep the button visible until they resume using a pointing device." — pointing-devices › Touch support for hidden controls.
- Menu contextuel au toucher : toucher et maintenir — "Someone who uses touch can open a context menu with a touch and hold." — pointing-devices › Touch support for hidden controls.

**À transposer dans un shell Linux**

- Survol : toute commande de la barre ou du dock qui s'estompe ou se réduit automatiquement se révèle au passage du pointeur et se masque quand il s'éloigne, sans déplacer les autres éléments ; les infobulles ne comptent pas comme commandes à rendre tactiles.
- Cibles de clic : la HIG ne fixe aucune taille ; retenir le principe d'agrandir la zone tactile sans agrandir le contrôle visible, pour les widgets de la barre, les icônes du dock et les tuiles de l'aperçu des espaces.
- Clic secondaire et molette : chaque élément du shell (icône du dock, widget, notification, espace) ouvre un menu contextuel court ; les zones défilantes (résultats du lanceur, notifications, pages des réglages, aperçu des espaces) répondent au défilement haut, bas, gauche et droite.
- Pointeurs : reprendre le jeu standard — flèche, main ouverte puis main fermée en glisser-déposer (réorganisation du dock, des widgets), main index pour les liens, opération non autorisée, variantes de redimensionnement à un seul sens quand un minimum ou un maximum est atteint.
- Gestes système : le shell reste propriétaire d'un jeu de gestes configurables (vue d'ensemble, espaces, centre de notifications, lanceur, bureau), jamais intercepté par une app, avec le même effet à la souris et au pavé tactile là où la HIG le prévoit (Mission Control).
- Écrans tactiles : les commandes cachées se révèlent aussi au toucher (tape sur l'emplacement, menu contextuel), la sélection courante est conservée, et rien n'est accessible uniquement au toucher.

### Gestures (`gestures`)

**Rôle** — Définir ce qu'est un geste (gesture), fixer les gestes standard que l'on attend partout, encadrer les gestes personnalisés et décrire les gestes du pavé tactile et de l'écran tactile sur Mac.

**Anatomie et cotes** — Tailles et durées : non chiffré dans la HIG (aucun seuil de distance, de vitesse ou de durée, aucune taille de cible tactile : le système qualifie d'optimale celle de ses contrôles standard sans la donner). Les seules cotes sont le nombre de doigts et le sens de quelques gestes système, donnés dans le tableau Interactions standard de la section Pointing devices.

**Règles**

- Définition : un geste est un mouvement physique qui agit directement sur un objet — "A gesture is a physical motion that a person uses to directly affect an object in an app or game on their device." — gestures › Discussion ; les gestes de base (toucher, balayer, glisser) varient dans leur forme exacte selon la plate-forme et le dispositif, mais leur fonction est connue et attendue partout — "people are familiar with the underlying functionality of these gestures and expect to use them everywhere" — gestures › Discussion.
- Autres entrées : offrir plus d'une façon d'interagir (voix, clavier, Switch Control) — "Give people more than one way to interact with your app." — gestures › Best practices ; ne pas présumer qu'une personne peut faire un geste donné — "Don’t assume that people can use a specific gesture to perform a given task." — gestures › Best practices.
- Attentes : la plupart des gestes fonctionnent pareil quel que soit le contexte — "People expect most gestures to work the same regardless of their current context." — gestures › Best practices ; ne pas employer un geste familier pour une action propre à l'app — "Avoid using a familiar gesture like tap or swipe to perform an action that’s unique to your app" — gestures › Best practices ; ni créer un geste inédit pour une action standard — "avoid creating a unique gesture to perform a standard action like activating a button or scrolling a long view" — gestures › Best practices.
- Réactivité : traiter les gestes le plus réactivement possible, avec un retour immédiat — "Useful gestures enhance the experience of direct manipulation and provide immediate feedback." — gestures › Best practices ; ce retour aide à prédire le résultat et, si besoin, communique l'amplitude et le type de mouvement requis — "provide feedback that helps them predict its results and, if necessary, communicates the extent and type of movement required to complete the action" — gestures › Best practices.
- Indisponibilité : signaler quand un geste n'est pas disponible — "Indicate when a gesture isn’t available." — gestures › Best practices ; sinon on croit que l'app est figée ou que le geste est mal fait (objet verrouillé qu'on tente de déplacer, bouton indisponible mal distingué) — "If you don’t clearly communicate why a gesture doesn’t work, people might think your app has frozen" — gestures › Best practices.
- Gestes personnalisés : ne les ajouter que si nécessaire, pour des tâches spécialisées, fréquentes et non couvertes par les gestes existants (jeu, app de dessin) — "Custom gestures work best when you design them for specialized tasks that people perform frequently and that aren’t covered by existing gestures" — gestures › Custom gestures.
- Gestes personnalisés, qualités : un geste personnalisé doit être — "Discoverable" — gestures › Custom gestures ; "Straightforward to perform" — gestures › Custom gestures ; "Distinct from other gestures" — gestures › Custom gestures ; et ne pas être le seul moyen d'accomplir une action importante — "Not the only way to perform an important action in your app or game" — gestures › Custom gestures.
- Gestes personnalisés, apprentissage : offrir des moments pour les apprendre et les tester en usage réel ; si l'on peine à décrire un geste en mots simples et en images, il sera difficile à apprendre — "If you’re finding it difficult to use simple language and graphics to describe a gesture, it may mean people will find the gesture difficult to learn and perform." — gestures › Custom gestures.
- Gestes raccourcis : ils complètent les gestes standard, ils ne les remplacent pas — "Use shortcut gestures to supplement standard gestures, not replace them." — gestures › Custom gestures ; garder les moyens simples et familiers, même s'il faut un ou deux touchers de plus — "people also need simple, familiar ways to navigate and perform actions, even if it means an extra tap or two" — gestures › Custom gestures.
- Interface système : éviter de définir des gestes personnalisés qui pourraient entrer en conflit avec ceux qui accèdent à l'interface système — "It’s important to avoid defining custom gestures that might conflict with these interactions, as people expect these controls to work consistently." — gestures › Custom gestures.

**Spécifique macOS**

- Entrées principales : sur macOS, on interagit principalement au clavier et au dispositif de pointage ; des gestes standard se font aussi — "on a Magic Trackpad, Magic Mouse, or a game controller with a touch surface" — gestures › macOS.
- Toucher secondaire : sur un Mac à écran tactile, ou un iPad connecté en Sidecar, le toucher est un mode d'entrée secondaire — "On a Mac with a touch display, or on an iPad connected with Sidecar, people can also interact directly using touch gestures as a secondary input mode." — gestures › macOS.
- Stabilité : garder la même disposition et les mêmes tailles quand on prend en charge le toucher — "Keep the layout and size of interface elements the same when you support touch." — gestures › macOS ; car le toucher est secondaire — "In macOS, touch is secondary to keyboard and pointing device input." — gestures › macOS.
- Cibles tactiles : agrandir les cibles, pas les contrôles — "Make tap targets bigger, not the controls themselves." — gestures › macOS ; l'interaction peut commencer légèrement hors des limites visuelles — "enlarging their tap targets to anticipate that interaction may start slightly outside of their visual bounds" — gestures › macOS ; les contrôles standard du système reçoivent automatiquement une taille de cible optimale, non chiffrée — "use standard framework controls, which the system configures with an optimal tap target size automatically" — gestures › macOS.
- Contrôles dilatables : envisager, pour les petits contrôles ou les contrôles serrés, une variante temporaire agrandie pour le toucher — "Consider providing expandable controls." — gestures › macOS ; elle reste en place un court instant après le relèvement du doigt — "leave it in place for a short time after you lift your finger to keep the control accessible to touch" — gestures › macOS.
- Jamais seul : ne pas faire du toucher le seul accès aux commandes — "Don’t use touch as the only way to access controls or features." — gestures › macOS ; toutes les fonctions restent accessibles au clavier et au pointeur — "Make sure all of your app’s functions are available through input from a keyboard and pointing device." — gestures › macOS ; réciproquement, donner un équivalent tactile aux actions réservées au pointeur — "aim to provide touch equivalents for actions otherwise performed exclusively with a pointing device" — gestures › macOS.
- Gestes connus : conserver les gestes déjà présents sur iPhone, iPad ou Mac — "Preserve gestures that may already exist for your app on iPhone or iPad, or on Mac with a trackpad or Magic Mouse." — gestures › macOS ; vérifier l'absence de conflit avec les gestes système — "Test to ensure they don’t conflict with macOS system gestures." — gestures › macOS.
- Gestes personnalisés : les faire fonctionner pour le toucher et le pavé tactile, à l'aide de modificatrices si besoin — "Supporting both direct and indirect touch input makes the gesture available to the widest audience." — gestures › macOS.
- Gestes système du bureau : balayage entre les apps en plein écran et les espaces, Mission Control, Centre de notifications, App Exposé, liste des apps, affichage du bureau, zoom par pincement, rotation, appui forcé, toucher pour cliquer ; la page `gestures` ne les décrit pas, ils figurent dans le tableau Interactions standard de la section Pointing devices (source pointing-devices › macOS). La HIG nomme Apps le geste qui affiche la liste des apps ; le terme Launchpad n'apparaît pas dans le condensé.
- Gestes standard (source : gestures › Standard gestures) ; les sept sont pris en charge sur macOS.

| Geste | Pris en charge sur | Action courante |
|---|---|---|
| Tap | iOS, iPadOS, macOS, tvOS, visionOS, watchOS | Activer un contrôle ; sélectionner un élément. |
| Swipe | iOS, iPadOS, macOS, tvOS, visionOS, watchOS | Révéler des actions et des commandes ; masquer des vues ; faire défiler. |
| Drag | iOS, iPadOS, macOS, tvOS, visionOS, watchOS | Déplacer un élément d'interface. |
| Touch (or pinch) and hold | iOS, iPadOS, macOS, tvOS, visionOS, watchOS | Révéler des commandes ou des fonctionnalités supplémentaires. |
| Double tap | iOS, iPadOS, macOS, tvOS, visionOS, watchOS | Zoomer ; dézoomer si déjà agrandi ; ouvrir un élément sur un Mac à écran tactile ; effectuer une action principale sur Apple Watch Series 9 et ultérieur et Apple Watch Ultra 2 et ultérieur. |
| Zoom | iOS, iPadOS, macOS, tvOS, visionOS | Agrandir une vue ; magnifier le contenu. |
| Rotate | iOS, iPadOS, macOS, tvOS, visionOS | Tourner un élément sélectionné. |

- Gestes du pavé tactile et de l'écran tactile (source : gestures › macOS trackpad and touch display gestures) : les actions suivantes sont prises en charge par des gestes standard sur les deux dispositifs.

| Action | Pavé tactile | Écran tactile |
|---|---|---|
| Sélectionner | Clic principal | Toucher |
| Ouvrir | Double clic | Double toucher |
| Faire défiler | Glisser deux doigts vers le haut ou le bas | Glisser un doigt vers le haut ou le bas |
| Zoomer | Pincer | Pincer |
| Tourner | Déplacer deux doigts l'un autour de l'autre | Déplacer deux doigts l'un autour de l'autre |
| Ouvrir un menu contextuel | Clic secondaire | Toucher et maintenir |
| Glisser | Cliquer et glisser | Toucher et maintenir, puis glisser |

**À transposer dans un shell Linux**

- Aperçu des espaces : un geste système ouvre la vue d'ensemble (modèle : balayage vers le haut à trois ou quatre doigts au pavé tactile, double-clic à deux doigts à la souris) ; un autre la limite aux fenêtres de l'app courante (modèle : balayage vers le bas à trois ou quatre doigts).
- Espaces et pages : un balayage navigue en avant et en arrière entre espaces et apps en plein écran (la HIG ne précise pas le nombre de doigts) ; le même balayage tourne les pages du lanceur et des réglages.
- Lanceur, notifications, bureau : pincer pouce et trois doigts pour la liste des apps, balayer depuis le bord du pavé pour le centre de notifications, écarter pouce et trois doigts pour le bureau.
- Équivalents : chaque geste système garde un raccourci clavier et un moyen au pointeur, et produit le même effet dans tout le shell ; les gestes configurables par l'utilisateur sont listés dans les Réglages.
- Retour : le geste donne un retour immédiat qui suit le doigt et annonce son résultat ; quand il ne s'applique pas (dernier espace, bord d'une liste), le shell le signale au lieu de ne rien faire.
- Écrans tactiles : disposition et tailles inchangées, cibles tactiles plus grandes que les contrôles, variantes dilatées pour les petits contrôles, gestes identiques au pavé tactile (toucher et maintenir pour un menu contextuel ou un glissement).

### Focus and selection (`focus-and-selection`)

**Rôle** — Définir comment le focus (focus) confirme visuellement la cible de l'interaction, comment on l'amène sur les composants au clavier ou avec un autre dispositif directionnel, et quand il sélectionne.

**Anatomie et cotes** — Tailles et durées : non chiffré dans la HIG (aucune épaisseur, couleur ou taille d'anneau, aucun délai). Les seules valeurs visuelles sont les deux rendus de liste donnés dans Spécifique macOS.

**Règles**

- Définition : le focus aide à confirmer visuellement l'objet que l'interaction vise — "Focus helps people visually confirm the object that their interaction targets." — focus-and-selection › Discussion ; il permet une navigation par composants — "Focus supports simplified, component-based navigation." — focus-and-selection › Discussion.
- Plate-forme : aider à amener le focus comme la plate-forme le fait — "Be consistent with the platform as you help people bring focus to items in your app." — focus-and-selection › Best practices ; l'indiquer par des apparences cohérentes avec la plate-forme — "Indicate focus using visual appearances that are consistent with the platform." — focus-and-selection › Best practices.
- Effets : s'appuyer sur les effets de focus du système — "Rely on system-provided focus effects." — focus-and-selection › Best practices ; ne créer des effets personnalisés qu'en cas d'absolue nécessité — "Consider creating custom focus effects only if it’s absolutely necessary." — focus-and-selection › Best practices.
- Stabilité : ne pas changer le focus sans interaction de la personne — "Avoid changing focus without people’s interaction." — focus-and-selection › Best practices ; on s'appuie sur le focus pour savoir où l'on est — "People rely on the focus system to help them know where they are in your app." — focus-and-selection › Best practices ; sinon il faut chercher le nouvel élément, ce qui retarde la tâche en cours — "If you change focus without their interaction, people have to spend time finding the newly focused item, delaying their current task." — focus-and-selection › Best practices.
- Stabilité, exception : quand on déplace le focus avec un dispositif à mouvements discrets et directionnels (clavier, télécommande, manette) et que l'élément qui a le focus disparaît, amener le focus sur l'un des éléments restants — "moving focus to one of these remaining items ensures that the focus indicator is in a location people can easily find" — focus-and-selection › Best practices.
- Stabilité, hors navigation : quand la personne ne déplace pas le focus avec un tel dispositif, masquer simplement l'indicateur — "it’s generally best to simply hide the focus indicator when the focused object disappears" — focus-and-selection › Best practices.
- Anneau ou surbrillance : en général, un anneau de focus (focus ring) pour un champ de texte ou de recherche, une surbrillance dans une liste ou une collection — "In general, use a focus ring for a text or search field, but use a highlight in a list or collection." — focus-and-selection › Best practices.
- Surbrillance : un anneau peut signaler un élément qui remplit une cellule, comme une photo, mais la ligne entière en surbrillance est plus lisible — "it’s usually easier for people to view lists and collections when an entire row is highlighted" — focus-and-selection › Best practices.
- Focus et sélection : dans beaucoup de cas, amener le focus sélectionne l'élément — "In many cases, focusing an item also selects it." — focus-and-selection › Discussion ; sauf si la sélection automatique provoquerait un changement de contexte distrayant, comme l'ouverture d'une vue — "The exception is when automatic selection might cause a distracting context shift, like opening a new view." — focus-and-selection › Discussion.

**Spécifique macOS**

- Apparence : sur iPadOS et macOS, le focus se montre par un anneau autour de l'élément ou par sa surbrillance — "For example, iPadOS and macOS show focus by drawing a ring around an item or highlighting it" — focus-and-selection › Discussion.
- Accès complet au clavier : en iPadOS et macOS, un mode d'accès complet au clavier permet d'atteindre tout contrôle — "a full keyboard access mode helps people use the keyboard to reach every control" — focus-and-selection › Best practices ; une app ne doit donc prendre en charge le focus que pour les éléments de contenu — "you only need to support focus for content elements like list items, text fields, and search fields" — focus-and-selection › Best practices ; et non pour les commandes comme les boutons, curseurs et interrupteurs.
- Listes : élément ayant le focus en texte blanc sur fond de la couleur d'accentuation de l'app — "the system draws focused list items using white text and a background highlight that matches the app’s accent color" — focus-and-selection › Best practices ; élément sans focus en couleur de texte standard sur fond gris — "drawing unfocused items using the standard text color and a gray background highlight" — focus-and-selection › Best practices.
- Bloc Platform considerations : la page n'a pas de bloc macOS ; ses considérations se limitent à — "Not supported in iOS or watchOS." — focus-and-selection › Platform considerations.
- Navigation Tab (d'après `keyboards`) : Control-Tab amène le focus sur le groupe de contrôles suivant — "Move focus to the next group of controls in a dialog or the next table (when Tab moves to the next cell)." — keyboards › Standard keyboard shortcuts ; Shift-Tab parcourt les contrôles en sens inverse, Control-Shift-Tab revient au groupe précédent, Control-F1 active ou désactive l'accès complet au clavier, Control-F7 outrepasse temporairement le mode d'accès clavier en cours.
- Touches directionnelles (d'après `keyboards`) : Control-flèche déplace le focus vers une autre valeur ou cellule d'une vue comme un tableau — "Move focus to another value or cell within a view, such as a table." — keyboards › Standard keyboard shortcuts ; Shift-flèche étend la sélection — "Extend selection one character to the right." — keyboards › Standard keyboard shortcuts.
- Sélection de texte (d'après `keyboards`) : Option-Shift-flèche étend la sélection au mot ou au paragraphe — "Extend selection to the end of the current word, then to the end of the next word." — keyboards › Standard keyboard shortcuts ; Shift-Command-flèche à l'unité sémantique suivante — "Extend selection to the next semantic unit, typically the end of the current line." — keyboards › Standard keyboard shortcuts ; Command-A sélectionne tout — "Select every item in a document or window, or all characters in a text field." — keyboards › Standard keyboard shortcuts ; Shift-Command-A désélectionne. La HIG ne dit rien ici sur les poignées de sélection ni sur la sélection par double-clic.

**À transposer dans un shell Linux**

- Anneau ou surbrillance : anneau de focus sur le champ de recherche du lanceur et sur les champs des réglages ; surbrillance pleine ligne ou pleine tuile dans les résultats du lanceur, la liste des espaces, les rangées de réglages et le dock vu comme une collection.
- Couleurs : élément ayant le focus clavier en texte blanc sur la couleur d'accentuation ; élément sans focus en couleur de texte standard sur gris ; le focus ne modifie ni taille ni disposition.
- Accès complet au clavier : une bascule globale depuis laquelle le clavier atteint tout dans le shell (widgets de la barre, dock, menus, actions des notifications, aperçu des espaces) ; hors de ce mode, Tab ne s'arrête que sur les champs et éléments de contenu.
- Focus stable : le shell ne déplace jamais le focus de sa propre initiative ; si l'élément qui a le focus disparaît pendant une navigation clavier (fenêtre fermée, notification expirée, résultat filtré), le focus passe à un élément voisin à un pas ; sinon l'indicateur se masque.
- Tabulation : groupes nets dans les fenêtres du shell (réglages : recherche, barre latérale, contenu ; lanceur : champ, résultats, actions), Shift-Tab en sens inverse, Control-Tab pour le groupe suivant.
- Focus et sélection : dans l'aperçu des espaces, amener le focus sur une tuile la sélectionne ; dans la barre latérale des réglages, amener le focus ne change pas la vue ; les raccourcis d'extension de sélection s'appliquent au champ de recherche.

### Valeurs de référence

| Propriété | Valeur | Source (slug › section) |
|---|---|---|
| Modificatrices | Control, Option, Shift, Command | keyboards › Discussion |
| Ordre de liste des modificatrices | Control, Option, Shift, Command | keyboards › Custom keyboard shortcuts |
| Symboles | Command, Shift, Option, Control : images `Keyboard_Command.svg`, `Keyboard_Shift.svg`, `Keyboard_Option.svg`, `Keyboard_Control.svg` ; glyphes non donnés en texte | keyboards › Custom keyboard shortcuts |
| Recherche | Command-Space (afficher ou masquer le champ) ; Option-Command-Space (fenêtre de résultats) | keyboards › Standard keyboard shortcuts |
| Bascule d'applications | Command-Tab (suivante, la plus récemment utilisée) ; Shift-Command-Tab (précédente) | keyboards › Standard keyboard shortcuts |
| Accès complet au clavier | Control-F1 (bascule) ; Control-F7 (outrepassement temporaire) | keyboards › Standard keyboard shortcuts |
| Focus clavier | Control-F2 (barre des menus) ; Control- F3 (Dock, espace dans la source) ; Control-F4 (fenêtre active ou suivante) ; Control-Shift-F4 (fenêtre précédente) ; Control-F5 (barre d'outils) ; Control-F6 (premier ou suivant panneau) ; Control-Shift-F6 (panneau précédent) | keyboards › Standard keyboard shortcuts |
| Navigation dans les contrôles | Shift-Tab (sens inverse) ; Control-Tab (groupe suivant) ; Control-Shift-Tab (groupe précédent) | keyboards › Standard keyboard shortcuts |
| Annulation, fermeture | Esc (annuler) ; Option-Command-Esc (forcer à quitter) ; Command-W (fermer la fenêtre active) ; Option-Command-W (toutes les fenêtres de l'app) | keyboards › Standard keyboard shortcuts |
| Masquer, réduire | Command-H (app courante) ; Option-Command-H (les autres apps) ; Command-M (fenêtre active) ; Option-Command-M (toutes les fenêtres de l'app) | keyboards › Standard keyboard shortcuts |
| Bureau, Dock, plein écran, réglages | F11 (afficher le bureau) ; Option-Command-D (afficher ou masquer le Dock) ; Control-Command-F (plein écran) ; Command-Comma (réglages de l'app) | keyboards › Standard keyboard shortcuts |
| Interactions disponibles | 7 communes à la souris et au pavé tactile ; 9 propres au pavé tactile (décompte des lignes du tableau) | pointing-devices › macOS |
| Mission Control | Pavé tactile : balayer vers le haut à trois ou quatre doigts ; souris : double-tap à deux doigts | pointing-devices › macOS |
| Espaces, pages | Balayer en avant ou en arrière entre les apps en plein écran et les espaces (nombre de doigts non précisé) ; entre les pages affichées une à une | pointing-devices › macOS |
| Centre de notifications, App Exposé | Balayer depuis le bord du pavé tactile ; balayer vers le bas à trois ou quatre doigts | pointing-devices › macOS |
| Liste des apps, bureau | Pincer avec le pouce et trois doigts ; écarter avec le pouce et trois doigts | pointing-devices › macOS |
| Défilement, menu contextuel | Pavé tactile : glisser deux doigts, clic secondaire ; écran tactile : glisser un doigt, toucher et maintenir | gestures › macOS trackpad and touch display gestures |
| Liste avec focus | Texte blanc, fond de la couleur d'accentuation de l'app | focus-and-selection › Best practices |
| Liste sans focus | Couleur de texte standard, fond gris | focus-and-selection › Best practices |
| Champ de texte ou de recherche, liste ou collection | Anneau de focus ; surbrillance | focus-and-selection › Best practices |
| Tailles de cible, délais, vitesses, seuils de geste, épaisseur et couleur de l'anneau | non chiffré dans la HIG | les quatre pages |

## Seconde partie — hyprdesk aujourd'hui, écarts et actions

### Ce que hyprdesk fait aujourd'hui

| Surface | Ce qui existe | Valeurs clés | Preuves |
| --- | --- | --- | --- |
| Raccourcis du bureau | 11 clés `keys.*`, réglables (chaîne vide = aucun) | SUPER+D mode suivant ; SUPER+ALT+D lanceur seul ; ALT+Tab commutateur (MAJ : arrière) ; SUPER+R recherche ; SUPER+Tab vue d'ensemble ; SUPER+MAJ+D réglages ; SUPER+CTRL+D masquer ; SUPER+Échap secours ; SUPER+MAJ+L assistante ; verrouillage vide (laissé à hyprland.lua) | `src/config/model.rs:1467-1525` ; `src/ui/plugins/keys.rs:104-119` |
| Essentiels de fenêtres, espaces, écrans | 21 raccourcis posés par hyprdesk là où hyprland.lua n'en a pas, coupables un à un | SUPER+Q fermer ; SUPER+flèches fenêtre voisine ; SUPER+MAJ+flèches déplacer ; SUPER+CTRL+flèches échanger ; SUPER+F plein écran ; SUPER+MAJ+F maximiser ; SUPER+1 à 10 espaces ; SUPER+MAJ+1 à 10 envoyer ; SUPER+CTRL+Tab onglet de groupe | `src/essentials.rs:71-169`, `352-470` |
| Page Réglages → Raccourcis | Vue d'ensemble avec origine, activation, capture d'une combinaison, retour au défaut, conflits (prendre ou rendre) | « Ajoutez SUPER, ALT ou CTRL… » si la touche servirait à écrire | `src/ui/plugins/keys.rs:568-575`, `679-707`, `790-814` ; `src/ui/key_conflict.rs:80-145` |
| Libellés des combinaisons | `pretty` : touches en majuscules, codes physiques des chiffres rendus en chiffres ; modificatrices dans l'ordre stocké | SUPER, ALT, CTRL, SHIFT | `src/essentials.rs:956-1014` ; `src/ui/plugins/keys.rs:371-438` ; `src/hypr.rs:572`, `1003` |
| Lanceur et recherche | Champ focalisé à l'ouverture, flèches (par rangée dans la grille), Entrée lance, Échap referme | — | `src/ui/mod.rs:5357-5376` ; `src/ui/search.rs:426`, `545-562` |
| Vue d'ensemble des espaces | Clavier exclusif, flèches et Tab, Entrée ouvre l'espace, Échap referme ou annule le renommage | — | `src/ui/overview.rs:158`, `539-591` |
| Commutateur de fenêtres | Ordre du dernier focus, Tab et flèche bas en avant, flèche haut en arrière, Échap annule | — | `src/switcher.rs:2`, `45-49` ; `src/ui/mod.rs:5344-5355` |
| Menus (composant unique) | Flèches, Tab, Début, Fin, Page préc. et suiv., Entrée ou Espace, Échap | — | `src/ui/context.rs:1172-1200` ; `src/ui/menu.rs:198-227` |
| Dock au clavier | Tab depuis le plein bureau, ou raccourci du mode lanceur ; flèches, Tab, MAJ+Tab, Début, Fin, Entrée, Espace, Échap ; anneau de 2 px | anneau `fg`, 2 px | `src/ui/mod.rs:5423-5427` ; `src/ui/plugins/dock/keyboard.rs:236-259` ; `src/ui/plugins/dock/surface.rs:576-577`, `737-741` |
| Centre de contrôle (Continuité) | Tab et MAJ+Tab dans le volet, flèches sur les curseurs, Entrée ou Espace, Page préc. et suiv. | ±0,05 par flèche ; 260 px par page ; anneau 2 px | `src/ui/plugins/bar/popover/continuity/keyboard.rs:13-36`, `161-196`, `226-235` |
| Réglages (tiroir) | Clavier à la demande, Échap ferme, Tab et MAJ+Tab d'un champ de saisie à l'autre | — | `src/ui/panel.rs:243` ; `src/ui/mod.rs:5393-5394`, `5410-5421` |
| Barre | Aucun clavier ; clic gauche, droit et milieu et molette sur les modules ; info-bulle ; révélation au survol en masquage automatique | bulle 500 ms | `src/ui/plugins/bar/surface.rs:666`, `957-958`, `450-455`, `1769-1840` |
| Notifications | Bulles sans clavier et sans clic droit ; cartes du centre défilantes | — | `src/ui/toasts.rs:414`, `477` ; `src/ui/plugins/bar/popover.rs:1318`, `1377` |
| Dock au pointeur | Survol continu, grossissement et levée, bulle au survol, clic droit = menu de l'application | ≤ 140 ms ; ≥ 9 % ; 2,5 px ; bulle 180 ms (400 ms en Convex) | `src/ui/plugins/dock/surface.rs:591-628`, `749-757`, `1286-1289` ; `src/ui/plugins/dock/lifecycle.rs:13` |
| Pointeur et pavé tactile | Vitesse, 3 profils d'accélération, défilement naturel, gaucher, fenêtre active ; toucher pour cliquer, clic droit à deux doigts, désactivation à la frappe, facteur de défilement ; thème du curseur ; réglages par périphérique | — | `src/ui/plugins/input.rs:172-332` ; `src/ui/hyprlists.rs:738-790` |
| Gestes du pavé | Liste « N doigts · sens → action », ajout et suppression, écrite dans inputs.lua ; balayement d'espaces de Hyprland | 8 actions ; distance, inversion, boucle, ratio d'annulation | `src/ui/hyprlists.rs:182-191`, `673-734` ; `src/ui/hyprset.rs:635-653` |
| Panorama (écran tactile) | Glisser de page au doigt ou à la souris, geste de bord, retour amorti ; appui long ; défilement au doigt | seuil 28 px ; 18 % de page ; 0,6 px/ms ; appui long 650 ms | `src/ui/plugins/panorama/surface/swipe.rs:1-36`, `433-439` ; `hold.rs:18-20` ; `drag.rs:1-17` |

### Écarts

| # | Composant | Nature | Force | Règle HIG (slug › section) | Réel dans hyprdesk (preuve) | Action proposée | Effort | Confiance |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| ENT-01 | Accès complet au clavier | ABSENT | Recommandation + tableau standard | « Support Full Keyboard Access, when possible. » — keyboards › Best practices ; « a full keyboard access mode helps people use the keyboard to reach every control » — focus-and-selection › Best practices ; bascule Control-F1 — keyboards › Standard keyboard shortcuts | Aucun mode d'accès clavier (aucune occurrence dans `src`, les 11 clés de `src/ui/plugins/keys.rs:104-119`). Dans les réglages, Tab ne circule que entre champs de saisie : `src/ui/mod.rs:5410-5421` ; les boutons et curseurs natifs ne prennent pas le focus (`continuity/keyboard.rs:1`), seul le centre de contrôle les enveloppe (`continuity.rs`, 9 appels). Barre : `KeyboardInteractivity::None` (`bar/surface.rs:666`). Bulles : `None` (`toasts.rs:414`, `477`). | Introduire un mode d'accès clavier du shell, basculé par un raccourci, dans lequel chaque commande du tiroir de réglages, de la barre et des notifications prend le focus par Tab (généraliser l'enveloppe de `continuity/keyboard.rs`). | L | haute |
| ENT-02 | Focus clavier par surface (barre, dock, notifications) | ABSENT | Tableau standard | « Control-F2 — Move focus to the menu bar », Control-F3 (Dock), Control-F6 (panneau) — keyboards › Standard keyboard shortcuts ; « People expect each of the following standard keyboard shortcuts to perform the action listed » | `Keys` (`src/config/model.rs:1467-1489`) ne contient aucun raccourci de focus. La barre n'a jamais le clavier (`bar/surface.rs:666`). Le dock ne le prend qu'en changeant le mode du bureau : `ToggleLauncher`, `LauncherOnly` et `Cycle` appellent `show_with_windows()` puis `launcher_grab_focused()` (`src/ui/mod.rs:3866-3892`). Le passage par Tab n'existe que sur le plein bureau (`mod.rs:5423-5427`) ; hors de lui, la surface du bureau est sans clavier (`mod.rs:2244-2258`), donc aussi dans le mode `no-desktop` de la session. | Ajouter à `Keys` un raccourci par surface qui pose le focus sans changer le mode du bureau : barre (modèle Control-F2), dock (Control-F3), notifications, volet des réglages (Control-F6). | M | haute |
| ENT-03 | Gestes système (vue d'ensemble, liste des apps, notifications, bureau) | ABSENT | Meilleure pratique + tableau des interactions | Mission Control, App Exposé, Apps, Show Desktop, Centre de notifications — pointing-devices › macOS (tableau Interactions standard) ; « people expect systemwide gestures to be available » — pointing-devices › Best practices | La table des actions d'un geste compte 8 entrées, toutes fenêtre ou espace : workspace, special, move, resize, close, fullscreen, float, cursorZoom (`src/ui/hyprlists.rs:182-191`). Aucune ne rejoint `Overview`, `Apps` ou `Notifications` (`src/control.rs:60-64`). Seul le balayement d'espaces de Hyprland est réglable (`src/ui/hyprset.rs:635-653`). | Ajouter à `ACTIONS` les actions `overview`, `apps`, `notifications` et `show-desktop`, liées par défaut aux gestes de la HIG (3 doigts vers le haut et vers le bas, pincer, écarter, depuis le bord), dans la limite de ce qu'accepte `hl.gesture`. | L | haute |
| ENT-04 | Ordre des modificatrices affichées | ÉCART | Always | « If you use more than one modifier key in a custom shortcut, always list them in this order: Control, Option, Shift, Command. » — keyboards › Custom keyboard shortcuts | La capture produit SUPER, ALT, CTRL, SHIFT (`src/ui/plugins/keys.rs:417-429`, `437-438`). `pretty` garde l'ordre stocké (`src/essentials.rs:1012-1014`) et sert l'affichage (`keys.rs:696`, `816`, `926`). Deux autres chemins se contredisent : SUPER, CTRL, ALT, SHIFT (`src/hypr.rs:572`) et SUPER, ALT, CTRL, SHIFT (`src/hypr.rs:1003`). Attendu par transposition : CTRL, ALT, SHIFT, SUPER. | Ordonner CTRL, ALT, SHIFT, SUPER à l'affichage, dans `pretty` et dans les trois fonctions qui construisent une combinaison. | S | haute |
| ENT-05 | MAJ+Tab dans les menus et la vue d'ensemble | ÉCART | Attente du tableau standard | « People expect each of the following standard keyboard shortcuts to perform the action listed in the table below. » ; « Shift-Tab — Parcourir les contrôles en sens inverse. » — keyboards › Standard keyboard shortcuts | Les menus reçoivent la touche sans ses modificateurs : Tab et MAJ+Tab avancent (`src/ui/mod.rs:5291-5295`, `src/ui/context.rs:1182`). Même choix dans la vue d'ensemble (`src/ui/overview.rs:567`). Le dock, lui, traite MAJ+Tab en sens inverse (`src/ui/plugins/dock/keyboard.rs:253`), de même que le tiroir (`mod.rs:5413-5418`) et le centre de contrôle (`continuity/keyboard.rs:15-22`). | Dans `icon_menu_key` et `overview_key`, transmettre l'état de Maj et faire reculer MAJ+Tab (entrée précédente, flèche haut). | S | haute |
| ENT-06 | SUPER+Tab | PISTE | « In general, don't » + tableau | « In general, don't repurpose standard keyboard shortcuts for custom actions. » — keyboards › Standard keyboard shortcuts ; « Command-Tab — Avancer vers l'app suivante la plus récemment utilisée » | Par défaut, SUPER+Tab ouvre la vue d'ensemble (`src/config/model.rs:1515`) et le commutateur est sur ALT+Tab (`model.rs:1511`). SUPER+Tab fait donc autre chose que la bascule entre apps attendue. | Rattacher le commutateur de fenêtres à SUPER+Tab, en plus d'ALT+Tab, et reloger la vue d'ensemble sur une autre combinaison. | S | haute |
| ENT-07 | Menus contextuels (modules de barre, notifications, espaces de l'aperçu) | PISTE | Tableau + bonne pratique | « Secondary click — Révéler les menus contextuels » — pointing-devices › macOS ; « A context menu gives people a dependable way to reach a control… keep the menu short. » — pointing-devices › Touch support for hidden controls | Sur les modules de barre, l'argument clic droit vaut `None` : débordement (`src/ui/plugins/bar/surface.rs:1371`), titre de fenêtre (`1945`), volume (`2081`), luminosité (`2111`) ; seule la zone de notification en a un (`2166-2184`). Un clic droit ferme la carte ouverte (`src/ui/plugins/bar/popover.rs:1201`). Les bulles n'ont pas de `on_right_press` (`src/ui/toasts.rs`), ni les cartes d'espace (`src/ui/overview.rs`). Le dock est conforme (`dock/surface.rs:757`). | Donner un menu contextuel court aux espaces de la vue d'ensemble, aux notifications et aux modules de barre. | M | haute |
| ENT-08 | Réduire la fenêtre active au clavier | ABSENT | Tableau standard | « Command-M — Réduire la fenêtre active dans le Dock » ; « Option-Command-M » — keyboards › Standard keyboard shortcuts | La fonction existe : `hyprdesk window minimize` range la fenêtre dans l'espace spécial (`src/hypr.rs:200`), lancé par le bouton de la barre de titre (`src/hyprbars.rs:104-116`). Aucun raccourci : ni dans le catalogue des essentiels (`src/essentials.rs:71-169`), ni dans `Keys` (`src/config/model.rs:1467-1489`). La réduction n'est donc accessible qu'au pointeur. | Ajouter un essentiel « Réduire la fenêtre active » lié à `hyprdesk window minimize`. | S | haute |
| ENT-09 | Raccourci du lanceur | PISTE | Tableau standard | « Command-Space — Afficher ou masquer le champ de recherche Spotlight. » — keyboards › Standard keyboard shortcuts | Le lanceur est sur SUPER+R par défaut (`src/config/model.rs:1513`). SUPER+Espace n'est lié nulle part dans le code (seul un banc d'essai le mentionne, `src/keymap.rs:810`). | Prendre SUPER+Espace comme raccourci par défaut de `keys.search`. | S | haute |
| ENT-10 | Forcer une application à quitter | ABSENT | Tableau standard | « Option-Command-Esc — Ouvrir la boîte de dialogue Forcer à quitter. » — keyboards › Standard keyboard shortcuts | Aucune boîte ni commande : `forcer`, `force_quit`, `kill_window`, `force_close` sont absents de `src/ui`, `src/control.rs` et `src/hypr.rs`. L'essentiel « Fermer la fenêtre » ne connaît que `window.close()` (`src/essentials.rs:367-373`). | Ajouter une boîte « Forcer à quitter » (liste des fenêtres, forcer la fermeture) et son raccourci. | M | haute |
| ENT-11 | Ctrl+Tab entre les groupes du centre de contrôle | ABSENT | Tableau standard | « Control-Tab — Déplacer le focus sur le groupe de contrôles suivant » ; « Control-Shift-Tab » — keyboards › Standard keyboard shortcuts | Le volet est un seul périmètre de focus (`continuity/keyboard.rs:10`, `16-21`) et ne traite que Tab et Page précédente ou suivante (`13-36`). Les quatre groupes y sont empilés (`continuity.rs:356`). Aucun `mods.control()` ne sert à la navigation (`src/ui/mod.rs:5403-5409` ne transmet que Maj). | Dans `handle`, faire sauter Ctrl+Tab et Ctrl+MAJ+Tab au premier contrôle du groupe suivant ou précédent. | S | haute |
| ENT-12 | Pointeurs de glisser-déposer dans la vue d'ensemble | PISTE | Tableau des pointeurs | « Open hand », « Closed hand », « Operation not allowed » — pointing-devices › Pointers | Les tuiles de fenêtre et les cartes utilisent `mouse_area` sans interaction (`src/ui/overview.rs:913`, `1019`) ; aucun `mouse::Interaction` dans ce fichier. Déposer une fenêtre sur sa propre carte ne fait rien (`overview.rs:527-535`) sans le signaler. Aucun pointeur « non autorisé » dans tout `src` (seuls Grab, Grabbing, Pointer, Crosshair sont employés). | Afficher la main fermée pendant le glissement d'une fenêtre et le pointeur « opération non autorisée » quand la cible de dépôt est invalide. | S | haute |
| ENT-13 | Apparence du focus clavier selon la surface | PISTE | Impératif de bonne pratique | « Indicate focus using visual appearances that are consistent with the platform. » — focus-and-selection › Best practices | Les listes prennent le jeton `focus` du pack (`src/ui/tokens.rs:413-420`, `494-528`). Le dock dessine son anneau dans la couleur du texte (`dock/surface.rs:737-741`) et le centre de contrôle reçoit `pal.fg` ou `pal.ink` (`continuity.rs:423`, `642` ; `continuity/keyboard.rs:226-235`). Deux apparences pour un même état. | Passer `tokens::focus(accent)` comme couleur d'anneau au dock et au centre de contrôle. | S | moyenne |
| ENT-14 | Focus de la vue d'ensemble quand la tuile disparaît | PISTE | Bonne pratique | « when moving focus with a discrete and directional device and the focused item disappears, move focus to one of the remaining items » — focus-and-selection › Best practices | L'index choisi n'est jamais recalé : il n'est écrit qu'à l'ouverture (`src/ui/overview.rs:112`) et au déplacement (`591`). Si la carte disparaît, Entrée ne fait rien (`overview.rs:550-566`) et aucune carte n'est surlignée (`869`). | Au recalcul des cartes, ramener l'index sur la carte voisine la plus proche. | S | moyenne |

Décompte : 14 écarts — ABSENT 6 (ENT-01, 02, 03, 08, 10, 11), ÉCART 2 (ENT-04, 05), PISTE 6 (ENT-06, 07, 09, 12, 13, 14).

### Ce qui est déjà conforme

- Échap annule et referme partout : recherche (`src/ui/mod.rs:5360`), vue d'ensemble (`src/ui/overview.rs:560`), commutateur (`mod.rs:5347`), cartes et menus de la barre (`mod.rs:5389-5391`), réglages (`mod.rs:5393-5394`), assistante (`mod.rs:5396-5398`), menus (`src/ui/context.rs:1181`), dock (`dock/keyboard.rs:237-246`), capture de raccourci (`keys.rs:1894-1895`), glisser-déposer de la barre (`mod.rs:5296-5301`) — keyboards › Standard keyboard shortcuts, « Esc — Cancel the current action or process ».
- Raccourci global du lanceur, flèches, Entrée : `src/config/model.rs:1513`, `src/ui/mod.rs:5357-5376` — modèle Command-Space.
- Bascule entre apps ordonnée par usage récent, avec sens avant et arrière : `src/switcher.rs:2`, `45-49` ; `src/ui/mod.rs:5349-5352` — modèle Command-Tab et Shift-Command-Tab.
- Menus au clavier, entrées grisées sautées : `src/ui/context.rs:1172-1200`, `src/ui/menu.rs:198-227`.
- Dock au clavier complet, anneau visible, Échap : `dock/keyboard.rs:236-259`, `dock/surface.rs:576-577` et `737-741` — focus-and-selection › Discussion, « drawing a ring around an item or highlighting it ».
- Anneau pour les champs (style iced sur le thème personnalisé, `src/ui/mod.rs:6319-6332`), surbrillance de la ligne entière pour les listes : `src/ui/tokens.rs:494-528`, `src/ui/search.rs:1350-1355` — focus-and-selection › Best practices.
- Le focus ne change ni taille ni disposition, et choisir n'ouvre rien sans Entrée : `src/ui/tokens.rs:514-528`, `src/ui/overview.rs:559-566` — « focusing an item also selects it », exception de changement de contexte respectée.
- Les commandes masquées se révèlent au survol : barre en masquage automatique (`bar/surface.rs:957-958`, `1191`, `1558-1559`) — pointing-devices › Best practices, « Let people use the pointer to reveal and hide controls ».
- Molette sur les modules de barre (volume et luminosité par pas de 5) : `bar/surface.rs:450-455`, `1805`, `2083`, `2113`.
- Poignées de réorganisation : main ouverte puis main fermée : `bar/surface.rs:1737`, `1657-1659` — pointing-devices › Pointers.
- Pavé tactile réglable, gestes listés et modifiables dans les Réglages : `src/ui/plugins/input.rs:285-310`, `src/ui/hyprlists.rs:673-734` — pointing-devices › macOS, « select specific finger combinations and movements for certain gestures ».
- Panorama : le contenu suit le doigt, retour amorti d'un geste abandonné, appui long, même geste au doigt et à la souris : `swipe.rs:1-12`, `433-439` ; `hold.rs:1-5` — gestures › Best practices, « provide immediate feedback ».
- Menu de l'application au clic droit sur une icône du dock : `dock/surface.rs:757` ; menu d'édition (Couper, Copier, Coller, Tout sélectionner) au clic droit dans un champ : `src/ui/edit.rs:1-11`.
- Aucun geste n'est le seul chemin : chaque action de la table a son équivalent par raccourci ou par la barre — gestures › Custom gestures, « Not the only way to perform an important action ».
- Pages d'aide à la frappe : défilement du contenu des réglages et des notifications (`src/ui/panel.rs:893`, `1025` ; `popover.rs:1318`, `1377`).
- Hors périmètre du shell (SANS OBJET) : zoom intelligent, pincement, rotation, recherche par appui forcé, qui appartiennent aux applications ; disposition et tailles propres de Panorama, documentées (`DESIGN.md:25`, `29`).
- Adaptations assumées : Super pour Command ; essentiels à la place de Command-W, Control-Command-F et Control-n. Les valeurs de survol du dock (140 ms, 9 %, 2,5 px) et les délais de bulle (180, 400, 500 ms) ne sont pas chiffrés par la HIG : pas d'écart.

### À vérifier sur un rendu

- Lisibilité réelle de l'anneau de focus : listes (`src/ui/tokens.rs:524-528`, largeur `focus_width`, opacité 0,6) et dock (2 px, couleur du texte) sur le verre, pack par pack.
- Accord entre l'anneau des champs de saisie (primaire du thème iced, `src/ui/mod.rs:6325-6332`) et le jeton `focus` des listes.
- Échap dans un champ de saisie des réglages : ferme le tiroir entier (`src/ui/mod.rs:5385-5395`) ou annule la saisie en cours ?
- Centre de contrôle : Tab atteint-il une commande située hors de la partie visible sans faire défiler (`continuity/keyboard.rs:13-22`, un seul volet défilant en `continuity.rs:356`) ?
- Vue d'ensemble : haut et bas suivent-ils les colonnes réelles des cartes ? Le code suppose ⌈√n⌉ colonnes (`src/ui/overview.rs:565-575`).
- Pointeur affiché sur une icône du dock (bouton Iced par défaut, non relevé dans le code).
- Panorama : cibles tactiles égales aux contrôles visuels (`DESIGN.md:25`), à mesurer sur l'écran ; gestures › macOS, « Make tap targets bigger, not the controls themselves ».
- Plein bureau : les raccourcis personnels de hyprland.lua sont suspendus sauf `submap_universal` (`src/hypr.rs:526-534`, `744-760`). Essai en session : rien de nécessaire manque-t-il ?
- Balayement au dernier espace : Hyprland signale-t-il l'indisponibilité ? hyprdesk ne le fait pas (`src/ui/hyprset.rs:635-653`, quatre options, aucun signal) ; gestures › Best practices, « Indicate when a gesture isn't available ».
