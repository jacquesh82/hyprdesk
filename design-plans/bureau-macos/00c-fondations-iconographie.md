# Fondations : icônes et images — macOS d'après la HIG, face à hyprdesk

> Proposition de design, rien d'implémenté. Plan d'ensemble et priorités : `plan-d-action.md` ; méthode et périmètre : `README.md`.

> Référentiel : Human Interface Guidelines d'Apple, export Markdown officiel relevé le 23 novembre 2026. Code : commit `00f6d69` (arbre de travail du 23 novembre 2026, avec des modifications non commitées).

## Première partie — Spécification d'après la HIG

### Résumé

La famille couvre quatre pages : SF Symbols (système de symboles à couches), Icons (icônes d'interface, ou interface icons), App icons (icônes d'application, ou app icons) et Images (points, facteurs d'échelle, formats).
Un symbole se définit par des paramètres plus que par des dimensions : neuf graisses, trois échelles relatives à la hauteur des capitales, quatre modes de rendu, des variantes (contour, plein, barré, encadré) et des animations.
La HIG ne donne aucune taille en points pour les symboles ni pour les icônes d'interface ; les seules cotes absolues sont la toile des icônes d'app (1024x1024 px), les icônes de document macOS (de 16x16 px à 512x512 px @1x) et les facteurs d'échelle (@1x and @2x sur macOS).
Trois principes reviennent sur les quatre pages : simplicité et reconnaissance immédiate, cohérence du jeu d'icônes (taille, niveau de détail, épaisseur de trait, perspective) et confiance au système pour les effets, les masques et les états.
Seule la page Icons comporte un bloc macOS, consacré aux icônes de document. Les trois autres pages déclarent ne rien ajouter pour macOS, et aucune ne mentionne le Dock ni la barre des menus (menu bar).
Pour un shell Linux, le transfert porte donc sur le vocabulaire et la discipline : un jeu de symboles à couches, des glyphes vectoriels cohérents, un masque unique pour les icônes d'app, la lisibilité aux petites tailles.

### SF Symbols (`sf-symbols`)

**Rôle** : Fournir un jeu de symboles (symbols) cohérents et configurables, intégrés à la police système San Francisco et alignés sur le texte en toute graisse et taille, partout où une icône d'interface peut apparaître — "You can use a symbol to convey an object or concept wherever interface icons can appear, such as in toolbars, tab bars, context menus, and within text." — sf-symbols › Discussion

**Anatomie et cotes**
- Couches (layers) : les chemins d'un symbole sont regroupés en couches distinctes — "To support the rendering modes, SF Symbols organizes a symbol’s paths into distinct layers." — sf-symbols › Rendering modes
- Exemple `cloud.sun.rain.fill` : primaire le nuage, secondaire le soleil, tertiaire la pluie — "the primary layer contains the cloud paths, the secondary layer contains the paths that define the sun and its rays, and the tertiary layer contains the raindrop paths." — sf-symbols › Rendering modes
- Modes de rendu (rendering modes) : quatre, soit monochrome, hiérarchique (hierarchical), palette et multicolore (multicolor) — "SF Symbols provides four rendering modes — monochrome, hierarchical, palette, and multicolor — that give you multiple options when applying color to symbols." — sf-symbols › Rendering modes
- Graisses (weights) : neuf, d'ultralight à black, chacune reliée à une graisse de la police San Francisco — "Each of the nine symbol weights — from ultralight to black — corresponds to a weight of the San Francisco system font" — sf-symbols › Weights and scales
- Échelles (scales) : trois, medium par défaut — "Each symbol is also available in three scales: small, medium (the default), and large." — sf-symbols › Weights and scales
- Unité des échelles : la hauteur des capitales de San Francisco ; aucune taille de symbole en points n'est donnée — "The scales are defined relative to the cap height of the San Francisco system font." — sf-symbols › Weights and scales
- Variantes (design variants) : contour (outline), plein (fill), barré (slash) et encadré (enclosed) dans un cercle, un carré ou un rectangle — "In addition to outline and fill, SF Symbols also defines variants that include a slash or enclose a symbol within a shape like a circle, square, or rectangle." — sf-symbols › Design variants
- Couleur variable (variable color) : la valeur franchit des seuils, entre zéro et cent pour cent, qui colorent différentes couches — "variable color applies color to different layers of a symbol as a value reaches different thresholds between zero and 100 percent." — sf-symbols › Variable color
- Dégradés (gradients) : à partir de SF Symbols 7, un dégradé linéaire issu d'une seule couleur source — "In SF Symbols 7 and later, gradient rendering generates a smooth linear gradient from a single source color." — sf-symbols › Gradients
- Animations : Appear, Disappear, Bounce, Scale, Pulse, Variable color, Replace (down-up, up-up, off-up), Magic Replace, Wiggle, Breathe, Rotate, Draw On / Draw Off ; aucune durée n'est chiffrée dans la HIG.
- Grille de dessin et tailles en points : non chiffré dans la HIG.

**Règles**

#### Modes de rendu et couleur
- Monochrome : une seule couleur sur toutes les couches — "Applies one color to all layers in a symbol." — sf-symbols › Rendering modes
- Hiérarchique : une seule couleur, d'opacité variable selon le niveau de la couche, pour donner du relief — "For example, Hierarchical rendering mode assigns a different opacity of a single color to each layer, creating a visual hierarchy that gives depth to the symbol." — sf-symbols › Rendering modes
- Palette : une couleur par couche ; avec deux couleurs sur trois niveaux, les couches secondaire et tertiaire sont de la même couleur — "Specifying only two colors for a symbol that defines three levels of hierarchy means the secondary and tertiary layers use the same color." — sf-symbols › Rendering modes
- Multicolore : couleurs propres à certains symboles, pour renforcer le sens — "Applies intrinsic colors to some symbols to enhance meaning." — sf-symbols › Rendering modes
- Couleurs : privilégier les couleurs système, qui suivent les réglages d'accessibilité et les apparences — "Regardless of rendering mode, using system-provided colors ensures that symbols automatically adapt to accessibility accommodations and appearance modes like vibrancy and Dark Mode." — sf-symbols › Rendering modes
- Contrôle : vérifier le mode de rendu dans chaque contexte, même avec le réglage automatique — "Confirm that a symbol’s rendering mode works well in every context." — sf-symbols › Rendering modes
- Dégradés : rendus à toute taille, meilleurs en grand — "Gradients render for symbols of any size, but look best at larger sizes." — sf-symbols › Gradients
- Couleur variable : elle signifie un changement, jamais la profondeur — "Use variable color to communicate change — don’t use it to communicate depth." — sf-symbols › Variable color
- Couleur variable : seules certaines couches y participent, comme les ondes du haut-parleur et non son corps — "A symbol can support variable color in any number of layers." — sf-symbols › Variable color

#### Graisses, échelles et variantes
- Graisse : accorder celle du symbole à celle du texte voisin — "helping you achieve precise weight matching between symbols and adjacent text" — sf-symbols › Weights and scales
- Échelle : règle l'accentuation du symbole sans rompre l'accord de graisse avec le texte de même corps — "Specifying a scale lets you adjust a symbol’s emphasis compared to adjacent text, without disrupting the weight matching with text that uses the same point size." — sf-symbols › Weights and scales
- Sens des variantes : barré pour l'indisponible, plein pour la sélection — "you could use the slash variant of a symbol to show that an item or action is unavailable, or use the fill variant to indicate selection." — sf-symbols › Design variants
- Contour : la variante la plus courante, adaptée aux barres d'outils et aux listes — "The outline variant works well in toolbars, lists, and other places where you display a symbol alongside text." — sf-symbols › Design variants
- Encadré : la forme englobante améliore la lisibilité en petite taille — "Symbols that use an enclosing shape — like a square or circle — can improve legibility at small sizes." — sf-symbols › Design variants
- Plein : davantage d'emphase visuelle, adapté à une sélection marquée par la couleur d'accent — "The solid areas in a fill variant tend to give a symbol more visual emphasis" — sf-symbols › Design variants
- Choix de la variante : le plus souvent la vue qui affiche le symbole tranche ; une barre d'outils (toolbar) prend le contour — "In many cases, the view that displays a symbol determines whether to use outline or fill, so you don’t have to specify a variant." — sf-symbols › Design variants
- Langues : les variantes propres à une langue ou à un système d'écriture suivent la langue de l'appareil — "Language- and script-specific variants adapt automatically when the device language changes." — sf-symbols › Design variants

#### Animations
- Finalité : exprimer une idée, confirmer une action, signaler un changement d'état ou une activité — "Symbol animations help communicate ideas, provide feedback in response to people’s actions, and signal changes in status or ongoing activities." — sf-symbols › Animations
- Portée : tous les symboles, dans tous les modes de rendu, graisses et échelles — "Animations work on all SF Symbols in the library, in all rendering modes, weights, and scales, and on custom symbols." — sf-symbols › Animations
- Parcimonie : trop d'animations submergent l'interface et distraient — "While there’s no limit to how many animations you can add to a view, too many animations can overwhelm an interface and distract people." — sf-symbols › Animations
- Intention : chaque animation sert un propos clair, sans ambiguïté d'interprétation — "Make sure that animations serve a clear purpose in communicating a symbol’s intent." — sf-symbols › Animations
- Bounce : joué une seule fois par défaut, pour signifier qu'une action a eu lieu ou doit avoir lieu — "The bounce animation plays once by default and can help communicate that an action occurred or needs to take place." — sf-symbols › Animations
- Scale : persiste jusqu'à nouvelle échelle ou retrait de l'effet, contrairement à Bounce qui revient à l'état initial — "the scale animation persists until you set a new scale or remove the effect." — sf-symbols › Animations
- Pulse : opacité variable pour une activité en cours, jusqu'à ce qu'une condition soit remplie — "You might use the pulse animation to communicate ongoing activity, playing it continuously until a condition is met." — sf-symbols › Animations
- Variable color : progression ou activité (lecture, connexion, diffusion) — "You might use variable color to communicate progress or ongoing activity, such as playback, connecting, or broadcasting." — sf-symbols › Animations
- Breathe : comme Pulse, mais en jouant sur l'opacité et la taille — "Breathe is similar to pulse; however pulse animates by changing opacity alone, while breathe changes both opacity and size to convey ongoing activity." — sf-symbols › Animations
- Rotate : la rotation confirme qu'une tâche avance — "when a task is in progress, rotation confirms that it’s working as expected." — sf-symbols › Animations
- Replace : Magic Replace est la transition par défaut entre formes apparentées, down-up sinon — "Magic Replace is the new default replace animation, but doesn’t occur between unrelated symbols; the default down-up animation occurs instead." — sf-symbols › Animations
- Replace, down-up : l'ancien symbole rétrécit, le nouveau grandit, pour un changement d'état — "Down-up, where the outgoing symbol scales down and the incoming symbol scales up, communicating a change in state." — sf-symbols › Animations

#### Symboles personnalisés et disponibilité
- Procédé : exporter le gabarit d'un symbole proche, puis le modifier dans un outil vectoriel — "To create a custom symbol, first export the template for a symbol that’s similar to the design you want, then use a vector-editing tool to modify it." — sf-symbols › Custom symbols
- Cohérence : même niveau de détail, graisse optique, alignement, position et perspective que les symboles système — "Create a custom symbol that’s consistent with the ones the system provides in level of detail, optical weight, alignment, position, and perspective." — sf-symbols › Custom symbols
- Annotation : attribuer à chaque couche une couleur ou un niveau hiérarchique (primaire, secondaire, tertiaire) — "you can assign a specific color — or a specific hierarchical level, such as primary, secondary, or tertiary — to each layer in a custom symbol." — sf-symbols › Custom symbols
- Marges : des marges latérales négatives compensent, à des fins d'alignement optique, un badge qui élargit le symbole — "SF Symbols supports negative side margins to aid optical horizontal alignment when a symbol contains a badge or other elements that increase its width." — sf-symbols › Custom symbols
- Variantes : ne pas intégrer d'encadré ni de badge dans un symbole personnalisé — "Avoid making custom symbols that include common variants, such as enclosures or badges." — sf-symbols › Custom symbols
- Animations : tester chaque symbole personnalisé avec tous les préréglages — "Test animations for custom symbols." — sf-symbols › Custom symbols
- Accessibilité : fournir un libellé de texte alternatif (alternative text labels) — "Provide alternative text labels for custom symbols." — sf-symbols › Custom symbols
- Produits Apple : pas de réplique, et pas de personnalisation d'un symbole qui représente un produit ou une fonction Apple — "Don’t design replicas of Apple products." — sf-symbols › Custom symbols
- Conditions d'utilisation : les symboles, ou des images qui s'y confondent, sont interdits dans les icônes d'app, les logos et tout usage de marque — "including the prohibition against using symbols — or images that are confusingly similar — in app icons, logos, or any other trademarked use." — sf-symbols › Discussion
- Disponibilité : les symboles et leurs fonctions dépendent de la version du système visé — "Availability of individual symbols and features varies based on the version of the system you’re targeting." — sf-symbols › Discussion

**Spécifique macOS**
- Aucune considération propre à macOS : la page le déclare explicitement — "No additional considerations for iOS, iPadOS, macOS, tvOS, visionOS, or watchOS." — sf-symbols › Platform considerations
- Barres d'outils (toolbars) : elles prennent la variante contour ; l'exemple opposé cité est la barre d'onglets d'iOS — "For example, an iOS tab bar prefers the fill variant, whereas a toolbar takes the outline variant." — sf-symbols › Design variants
- Apparences : la vibrance (vibrancy) et le Dark Mode sont cités parmi les apparences que suivent les couleurs système — "appearance modes like vibrancy and Dark Mode" — sf-symbols › Rendering modes
- API : la page renvoie à la référence développeur AppKit pour régler l'échelle (sf-symbols › Weights and scales, lien non cité).
- Non traités dans la page : barre des menus (menu bar), Dock, tailles des symboles sur macOS.

**À transposer dans un shell Linux**
- Barre, lanceur, réglages, widgets : un jeu de symboles unique et organisé en couches, dont la graisse suit celle du texte voisin et dont l'échelle (petite, moyenne, grande) est relative à la hauteur des capitales de la police de l'interface.
- Toutes les surfaces : contour à côté d'un texte, plein pour la sélection, barré pour l'indisponible, forme englobante en petite taille ; le composant choisit la variante, jamais l'auteur de l'écran.
- Réglages rapides de la barre : couleur variable par seuils pour le volume, le réseau ou la batterie ; réserver l'opacité hiérarchique au rendu, jamais à la valeur.
- Thème : les quatre modes de rendu pilotés par la couleur d'accentuation et les couleurs sémantiques, de façon à suivre le mode sombre et les réglages d'accessibilité.
- Notifications et barre : animations fonctionnelles seulement (pulse ou breathe pour une activité en cours, bounce pour confirmer une action, replace pour un changement d'état), en nombre limité.
- Symboles maison : partir d'un symbole existant, en garder les marges et la graisse optiques, et joindre un libellé alternatif ; ne pas reprendre de symbole SF dans une icône d'app ni dans un logo.

### Icons (`icons`)

**Rôle** : Définir les icônes d'interface (interface icons), aussi appelées glyphes (glyphs), qui traduisent les éléments, actions et modes par des formes épurées et quelques touches de couleur, à la différence des icônes d'app riches en détails — "typically uses streamlined shapes and touches of color to communicate a straightforward idea." — icons › Discussion

**Anatomie et cotes**
- Taille en points : non chiffré dans la HIG ; seule borne chiffrée, le plus petit affichage des icônes de document macOS, 16x16 px (voir Spécifique macOS).
- Vecteurs de cohérence : taille, niveau de détail, épaisseur du trait et perspective — "all interface icons in your app need to use a consistent size, level of detail, stroke thickness (or weight), and perspective." — icons › Best practices
- Couleurs sources : noir et transparent, le système teinte les zones noires — "Both interface icons and symbols use black and clear colors to define their shapes; the system can apply other colors to the black areas in each image." — icons › Discussion
- Format : vectoriel (PDF ou SVG) pour toute icône personnalisée — "If you create a custom interface icon, use a vector format like PDF or SVG." — icons › Best practices
- Décalage d'alignement optique : très faible, non chiffré — "Adjustments for optical centering are typically very small, but they can have a big impact on your app’s appearance." — icons › Best practices

**Icônes standard (Standard icons)** : colonnes Action et Symbol name recopiées ; les lignes qui partagent un même symbole dans la HIG sont regroupées.
- **Editing** (source : icons › Editing) : Cut `scissors` ; Copy `document.on.document` ; Paste `document.on.clipboard` ; Done et Save `checkmark` ; Cancel et Close `xmark` ; Delete `trash` ; Undo `arrow.uturn.backward` ; Redo `arrow.uturn.forward` ; Compose `square.and.pencil` ; Duplicate `plus.square.on.square` ; Rename `pencil` ; Move to Folder `folder` ; Attach `paperclip` ; Add `plus` ; More `ellipsis`
- **Selection** (source : icons › Selection) : Select `checkmark.circle` ; Deselect et Close `xmark` ; Delete `trash`
- **Search** (source : icons › Search) : Search `magnifyingglass` ; Find, Find and Replace, Find Next, Find Previous et Use Selection for Find `text.page.badge.magnifyingglass` ; Filter `line.3.horizontal.decrease`
- **Sharing and exporting** (source : icons › Sharing and exporting) : Share et Export `square.and.arrow.up` ; Print `printer`
- **Users and accounts** (source : icons › Users and accounts) : Account, User et Profile `person.crop.circle`
- **Text formatting** (source : icons › Text formatting) : Superscript `textformat.superscript` ; Subscript `textformat.subscript` ; Bold `bold` ; Italic `italic` ; Underline `underline` ; Align Left `text.alignleft` ; Center `text.aligncenter` ; Justified `text.justify` ; Align Right `text.alignright`
- **Ratings** (source : icons › Ratings) : Dislike `hand.thumbsdown` ; Like `hand.thumbsup`
- **Layer ordering** (source : icons › Layer ordering) : Bring to Front `square.3.layers.3d.top.filled` ; Send to Back `square.3.layers.3d.bottom.filled` ; Bring Forward `square.2.layers.3d.top.filled` ; Send Backward `square.2.layers.3d.bottom.filled`
- **Other** (source : icons › Other) : Alarm `alarm` ; Archive `archivebox` ; Calendar `calendar`

**Règles**

#### Simplicité et reconnaissance
- Détails : trop de détails rendent une icône confuse ou illisible — "Too many details can make an interface icon confusing or unreadable." — icons › Best practices
- Métaphores : visuelles, familières, en lien direct avec l'action déclenchée ou le contenu représenté — "In general, icons work best when they use familiar visual metaphors that are directly related to the actions they initiate or content they represent." — icons › Best practices
- Inclusion : figures humaines neutres de genre, pas d'image difficile à reconnaître selon la culture ou la langue — "Prefer depicting gender-neutral human figures and avoid images that might be hard to recognize across different cultures or languages." — icons › Best practices

#### Cohérence du jeu
- Uniformité : la règle vaut qu'on n'emploie que des icônes maison ou un mélange d'icônes maison et système — "Whether you use only custom icons or mix custom and system-provided ones" — icons › Best practices
- Poids visuel : ajuster les dimensions d'une icône pour qu'elle paraisse de même taille que ses voisines — "Depending on the visual weight of an icon, you may need to adjust its dimensions to ensure that it appears visually consistent with other icons." — icons › Best practices
- Graisse : identique à celle du texte adjacent, sauf volonté d'insister sur l'un ou l'autre — "In general, match the weights of interface icons and adjacent text." — icons › Best practices

#### Alignement optique
- Centrage : un symbole asymétrique se centre optiquement, pas géométriquement — "Some icons — especially asymmetric ones — can look unbalanced when you center them geometrically instead of optically." — icons › Best practices
- Compensation : la correction est intégrée comme remplissage (padding) dans la ressource, que l'on centre ensuite géométriquement — "you can optically center the icon by geometrically centering the asset." — icons › Best practices

#### États, textes, formats
- État sélectionné (selected state) : pas de version dédiée dans les composants standard (barres d'outils, barres d'onglets, boutons) — "You don’t need to provide selected and unselected appearances for an icon that’s used in standard system components such as toolbars, tab bars, and buttons." — icons › Best practices
- Texte : seulement s'il est essentiel au sens — "Include text in your design only when it’s essential for conveying meaning." — icons › Best practices
- Texte : tout caractère affiché dans une icône est localisé — "If you need to display individual characters in your icon, be sure to localize them." — icons › Best practices
- Sens de lecture : pour suggérer un passage de texte, prévoir la version miroir de l'icône quand le contexte se lit de droite à gauche — "include a flipped version of the icon to use when the context is right-to-left." — icons › Best practices
- Vectoriel : mis à l'échelle automatiquement pour les écrans haute résolution — "The system automatically scales a vector-based interface icon for high-resolution displays, so you don’t need to provide high-resolution versions of it." — icons › Best practices
- PNG : une version par définition — "doesn’t support scaling, so you have to supply multiple versions for each PNG-based interface icon." — icons › Best practices
- Accessibilité : libellé de texte alternatif pour toute icône personnalisée — "Provide alternative text labels for custom interface icons." — icons › Best practices
- Matériel Apple : pas de réplique — "Avoid using replicas of Apple hardware products." — icons › Best practices

**Spécifique macOS**
- Seul bloc macOS de la famille : les icônes de document (document icons), pour un type de document personnalisé — "If your macOS app can use a custom document type, you can create a document icon to represent it." — icons › Document icons
- Forme : une feuille de papier au coin supérieur droit rabattu — "Traditionally, a document icon looks like a piece of paper with its top-right corner folded down." — icons › Document icons
- Icône par défaut : sans icône fournie pour un type de fichier, macOS compose l'icône de l'app et l'extension du fichier sur la toile — "macOS creates one for you by compositing your app icon and the file’s extension onto the canvas." — icons › Document icons
- Jeux d'icônes : un jeu peut distinguer plusieurs types de fichiers gérés par une app — "In some cases, it can make sense to create a set of document icons to represent a range of file types your app handles." — icons › Document icons
- Composition : toute combinaison de fond (background fill), d'image centrale (center image) et de texte — "To create a custom document icon, you can supply any combination of background fill, center image, and text." — icons › Document icons
- Assemblage : le système superpose, positionne et masque ces éléments, puis les compose sur la forme à coin rabattu — "The system layers, positions, and masks these elements as needed and composites them onto the familiar folded-corner icon shape." — icons › Document icons
- Simplicité : formes non compliquées et palette réduite de couleurs distinctes — "prefer uncomplicated shapes and a reduced palette of distinct colors." — icons › Document icons
- Tailles du fond (background fill) : 512x512 px @1x, 1024x1024 px @2x ; 256x256 px @1x, 512x512 px @2x ; 128x128 px @1x, 256x256 px @2x ; 32x32 px @1x, 64x64 px @2x ; 16x16 px @1x, 32x32 px @2x — "Create a set of background images in the sizes listed below." — icons › Document icons
- Tailles de l'image centrale (center image) : 256x256 px @1x, 512x512 px @2x ; 128x128 px @1x, 256x256 px @2x ; 32x32 px @1x, 64x64 px @2x ; 16x16 px @1x, 32x32 px @2x — "You can provide center images in the following sizes:" — icons › Document icons
- Proportion de l'image centrale : la moitié de la toile de l'icône, soit 16x16 px pour une icône de 32x32 px (exemple de la HIG) — "The center image measures half the size of the overall document icon canvas." — icons › Document icons
- Marge : environ 10 % de la toile, l'essentiel de l'image restant à l'intérieur — "Define a margin that measures about 10% of the image canvas and keep most of the image within it." — icons › Document icons
- Occupation : environ 80 % de la toile — "it’s best when the image occupies about 80% of the image canvas" — icons › Document icons
- Exemple : 205x205 px dans une toile de 256x256 px — "For example, most of the center image in a 256x256 px canvas would fit in an area that measures 205x205 px." — icons › Document icons
- Petit format : affichage possible à 16x16 px, donc reconnaissable à toute taille — "Your document icon can display as small as 16x16 px, so you want to create designs that remain recognizable at every size." — icons › Document icons
- Réduction de complexité dans les petites versions, jusqu'à retirer des détails à 16x16 px — "In the 16x16 px size, you might remove the lines altogether." — icons › Document icons
- Coin supérieur droit du fond (background fill) : ne pas y placer de contenu important — "Avoid placing important content in the top-right corner of your background fill." — icons › Document icons
- Texte : l'extension s'affiche par défaut en bas de l'icône — "By default, the system displays a document’s extension at the bottom edge of the document icon" — icons › Document icons
- Texte : si l'extension n'est pas familière, un terme plus descriptif peut la remplacer — "but if the extension is unfamiliar you can supply a more descriptive term." — icons › Document icons
- Texte : tout en capitales par défaut — "By default, the system capitalizes every letter in the text." — icons › Document icons
- Texte : mis à l'échelle automatiquement, donc assez court pour rester lisible en petit — "so be sure to use a term that’s short enough to be legible at small sizes." — icons › Document icons
- Hors icônes de document : la page ne déclare rien d'autre, et macOS n'a pas d'autre bloc ; barre des menus et Dock ne sont pas abordés — "No additional considerations for iOS, iPadOS, tvOS, visionOS, or watchOS." — icons › Platform considerations

**À transposer dans un shell Linux**
- Barre, dock, lanceur, réglages, verrouillage : un seul jeu de glyphes d'interface, de même taille, même épaisseur de trait, même perspective et même niveau de détail ; les icônes d'app restent une famille à part.
- Lanceur, recherche, menus : reprendre les icônes standard comme table de correspondance entre action et glyphe (loupe pour la recherche, points de suspension pour plus, croix pour fermer, coche pour terminer).
- Toutes les surfaces : graisse du glyphe égale à celle du texte voisin ; dimensions ajustées d'après le poids visuel ; correction d'alignement optique intégrée dans la ressource.
- Composants standard (éléments de barre, commandes du verrouillage, lignes de réglages) : l'état sélectionné est rendu par le composant, sans seconde ressource.
- Toutes les surfaces : glyphes plats en SVG ; libellé alternatif (description accessible) pour chaque icône personnalisée ; pas de texte dans une icône sauf nécessité.
- Icônes de fichiers (bureau, lanceur) : à défaut d'icône de type, composer l'icône de l'application et l'extension du fichier ; garder une marge d'environ 10 %, l'essentiel dans 80 % de la toile, et simplifier jusqu'à 16x16 px.

### App icons (`app-icons`)

**Rôle** : Exprimer l'identité, le rôle et la personnalité d'une app par une icône unique et mémorable, affichée dans les emplacements clés du système — "A well-designed app icon conveys your app’s or game’s identity clearly and consistently across all Apple platforms." — app-icons › Discussion

**Anatomie et cotes**
- Emplacements cités : écran d'accueil, résultats de recherche, notifications, réglages système, feuilles de partage — "including search results, notifications, system settings, and share sheets." — app-icons › Discussion
- Couches : un fond (background layer) et une ou plusieurs couches de premier plan (foreground layers) — "iOS, iPadOS, macOS, and watchOS app icons include a background layer and one or more foreground layers that coalesce to create dimensionality." — app-icons › Layer design
- Effets : attributs Liquid Glass (reflets spéculaires, réfraction, transparence) — "These icons take on Liquid Glass attributes like specular highlights, refraction, and translucency." — app-icons › Layer design
- Variantes réduites : mises à l'échelle par le système pour certains emplacements ; leurs tailles ne sont pas chiffrées — "The system automatically scales your icon to produce smaller variants that appear in certain locations, such as Settings and notifications." — app-icons › Specifications
- Espaces colorimétriques : sRGB (color), Gray Gamma 2.2 (grayscale), Display P3 (wide-gamut color in iOS, iPadOS, macOS, tvOS, and watchOS only) (app-icons › Specifications).
- Grille de placement : renvoyée aux modèles de production, cotes non chiffrées dans la HIG — "use the grids in the app icon production templates" — app-icons › Icon shape
- Rayon des coins arrondis et tailles d'affichage sur macOS : non chiffré dans la HIG.

Tableau Specifications, recopié (source : app-icons › Specifications) :

| Platform | Layout shape | Icon shape after system masking | Layout size | Style | Appearances |
|---|---|---|---|---|---|
| iOS, iPadOS, macOS | Square | Rounded rectangle (square) | 1024x1024 px | Layered | Default, dark, clear light, clear dark, tinted light, tinted dark |
| tvOS | Rectangle (landscape) | Rounded rectangle (rectangular) | 800x480 px | Layered (Parallax) | N/A |
| visionOS | Square | Circular | 1024x1024 px | Layered (3D) | N/A |
| watchOS | Square | Circular | 1088x1088 px | Layered | N/A |

**Règles**

#### Couches (Layer design)
- Bords nets dans les couches de premier plan, pour que reflets et ombres du système réussissent — "To ensure system-drawn highlights and shadows look best, avoid soft and feathered edges on foreground layer shapes." — app-icons › Layer design
- Opacité variable dans les couches de premier plan, pour la profondeur — "Vary opacity in foreground layers to increase the sense of depth and liveliness." — app-icons › Layer design
- Fond : se détacher tout en mettant en valeur le contenu du premier plan — "Design a background that both stands out and emphasizes foreground content." — app-icons › Layer design
- Fond importé : plein cadre et opaque — "If you do import a background layer, make sure it’s full-bleed and opaque." — app-icons › Layer design
- Effets : ils s'adaptent à la taille de l'icône, de façon cohérente entre plateformes, et varient selon la version du système — "These effects automatically adapt with the size of your icon, apply consistently across platforms, and can appear differently between system versions." — app-icons › Layer design
- Vecteur de préférence (SVG ou PDF) — "Unlike raster images, vector graphics (such as SVG or PDF) scale gracefully and appear crisp at any size." — app-icons › Layer design
- Textes : convertis en contours dans le dessin — "Outline artwork and convert text to outline in your design." — app-icons › Layer design
- PNG pour les dégradés de maillage et le travail raster — "For mesh gradients and raster artwork, prefer PNG format because it’s a lossless image format." — app-icons › Layer design

#### Formats (Icon formats)
- Deux formats : bitmap non en couches (PNG) ou fichier Icon Composer en couches — "In iOS, iPadOS, macOS, and watchOS, you can provide icons in one of two formats: nonlayered bitmap images (such as PNG) or layered Icon Composer files." — app-icons › Icon formats
- Préférer Icon Composer, pour la maîtrise des effets et des couleurs par apparence — "Provide your icon in the Icon Composer format for more flexibility and control." — app-icons › Icon formats
- Bitmap : le système segmente l'icône en deux couches (premier plan et fond) — "the system attempts to segment your icon, isolating the foreground elements from the background to create an icon with two layers." — app-icons › Icon formats
- Bitmap : les effets Liquid Glass ne s'appliquent pas automatiquement aux petits éléments complexes (texte, traits fins) — "It doesn’t automatically apply Liquid Glass effects to smaller, more complex elements in an icon like text and thin lines." — app-icons › Icon formats

#### Forme (Icon shape)
- Couches carrées, coins arrondis appliqués par le système — "For iOS, iPadOS, and macOS icons, provide square layers so the system can apply rounded corners." — app-icons › Icon shape
- Jamais de masque pré-découpé — "Providing layers with pre-defined masking negatively impacts specular highlight effects and makes edges look jagged." — app-icons › Icon shape
- Courbure des coins identique à celle des autres éléments arrondis du système — "the system applies masking to produce rounded corners that precisely match the curvature of other rounded interface elements throughout the system" — app-icons › Icon shape
- Contenu principal centré, pour éviter la troncature — "Keep primary content centered to avoid truncation when the system adjusts corners or applies masking." — app-icons › Icon shape

#### Conception (Design)
- Idée : exprimer le motif de l'icône de façon simple et unique, avec un nombre minimal de formes — "express it in a simple, unique way with a minimal number of shapes." — app-icons › Design
- Fond : simple (couleur unie ou dégradé), qui met le motif principal en valeur — "Prefer a simple background, such as a solid color or gradient, that puts the emphasis on your primary design" — app-icons › Design
- Plateformes : une icône visuellement cohérente partout — "Provide a visually consistent icon design across all the platforms your app supports." — app-icons › Design
- Formes : s'appuyer sur des formes pleines qui se chevauchent — "Consider basing your icon design around filled, overlapping shapes." — app-icons › Design
- Texte : seulement s'il est essentiel — "Text in icons doesn’t support accessibility or localization, is often too small to read easily, and can make an icon appear cluttered." — app-icons › Design
- Images : illustrations plutôt que photos, pas de composant d'interface recopié — "Prefer illustrations to photos and avoid replicating UI components." — app-icons › Design
- Tracé : ni trait très fin ni angle vif — "Make sure to avoid extremely thin line weights and sharp corners, because they tend to lose detail and crispness in smaller icon sizes at lower resolutions." — app-icons › Design
- Produits Apple : pas de réplique — "Apple products are copyrighted and can’t be reproduced in your app icons." — app-icons › Design

#### Effets visuels (Visual effects)
- Le système applique les effets (flou, reflets, ombres, lueurs) : ne pas les inclure — "Let the system handle blurring and other visual effects." — app-icons › Visual effects
- Un effet maison est statique, un effet système dynamique — "custom effects are static, whereas the system supplies dynamic ones" — app-icons › Visual effects
- Grouper des couches pour appliquer un effet à plusieurs à la fois — "Create layer groupings to apply effects to multiple layers at once." — app-icons › Visual effects

#### Apparences (Appearances)
- Quatre apparences au choix : par défaut, sombre (dark), claire (clear), teintée (tinted) — "people can choose whether their Home Screen app icons are default, dark, clear, or tinted in appearance." — app-icons › Appearances
- Variantes non fournies : générées par le système — "the system automatically generates variants you don’t provide." — app-icons › Appearances
- Mêmes caractéristiques visuelles dans toutes les apparences — "Keep your icon’s features consistent across appearances." — app-icons › Appearances
- Variantes sombres, claires et teintées : plus effacées que l'icône par défaut — "dark icons are more subdued, and clear and tinted icons are even more so" — app-icons › Appearances
- Icône sombre : dérivée de l'icône claire — "Use your light app icon as the basis for your dark icon." — app-icons › Appearances
- Contraste : les fonds colorés donnent le meilleur contraste en sombre — "Color backgrounds generally offer the greatest contrast in dark icons." — app-icons › Appearances

**Spécifique macOS**
- Aucune considération propre à macOS : la page le déclare explicitement — "No additional considerations for iOS, iPadOS, or macOS." — app-icons › Platform considerations
- Sections concernées : macOS est cité dans Layer design et Icon formats (avec iOS, iPadOS et watchOS), ainsi que dans Icon shape et Appearances (avec iOS et iPadOS) ; les règles ci-dessus s'y appliquent sans réserve.
- Forme : les coins arrondis épousent la courbure des autres éléments arrondis du système et la bordure physique de l'appareil — "and the bezel of the physical device itself" — app-icons › Icon shape
- Outil : Icon Composer reçoit les couches des icônes macOS — "For iOS, iPadOS, macOS, and watchOS icons, you then import your icon layers into Icon Composer" — app-icons › Layer design
- Dans Icon Composer : annoter les variantes par défaut, sombre et mono — "annotate for default, dark, and mono appearance variants" — app-icons › Layer design
- Spécification : Square ; Rounded rectangle (square) ; 1024x1024 px ; Layered ; Default, dark, clear light, clear dark, tinted light, tinted dark (tableau Specifications).
- Espaces colorimétriques : Display P3 pris en charge sur macOS, en plus de sRGB et Gray Gamma 2.2 (app-icons › Specifications).
- Icônes alternatives : non proposées pour macOS, seulement pour iOS, iPadOS, tvOS et visionOS — "In iOS, iPadOS, tvOS, and compatible apps running in visionOS, it’s possible to let people visit your app’s settings to choose an alternate version of your app icon." — app-icons › Appearances
- Non traités dans la page : Dock, barre des menus, tailles d'affichage des icônes sur macOS, rayon des coins arrondis.

**À transposer dans un shell Linux**
- Dock, lanceur, recherche, notifications, aperçu des espaces : une seule icône par app, identique sur toutes les surfaces, mise à l'échelle par le shell et jamais redessinée.
- Dock et lanceur : un masque unique en rectangle arrondi, appliqué par le shell, dont la courbure est celle des autres éléments arrondis de l'interface (fenêtres, widgets).
- Dock : reflets, ombres et translucidence générés dynamiquement selon l'état (lancement, application active, survol), jamais cuits dans la ressource ; les effets suivent la taille de l'icône.
- Barre et notifications : l'icône y est réduite à une taille non chiffrée par la HIG ; contrôler la lisibilité des icônes tierces en petit, sans trait fin ni texte.
- Réglages et thème : variantes par défaut, sombre et teintée qui conservent les mêmes caractéristiques, la variante sombre dérivant de la claire.
- Icônes tierces livrées en bitmap : le shell peut isoler le premier plan du fond pour produire les variantes, en sachant que textes et traits fins s'y prêtent mal.

### Images (`images`)

**Rôle** : Garantir le rendu des ressources graphiques sur tout appareil en livrant chaque image au facteur d'échelle (scale factor) voulu ; la page traite la résolution et les formats, mais ne comporte ni section sur le recadrage (cropping) ni développement sur les images vectorielles hors du cas des icônes plates.

**Anatomie et cotes**
- Point (point) : unité abstraite, indépendante de l'affichage — "an abstract unit of measurement that helps visual content remain consistent regardless of how it’s displayed." — images › Resolution
- Correspondance : en 2D, un point vaut un nombre de pixels qui varie avec la résolution de l'écran — "In 2D platforms, a point maps to a number of pixels that can vary according to the resolution of the display" — images › Resolution
- Facteur d'échelle (scale factor) @1x : un pixel pour un point — "For example, a scale factor of 1 (also called @1x) describes a 1:1 pixel density, where one pixel is equal to one point." — images › Resolution
- @2x et @3x : densités 2:1 et 3:1 — "A 2:1 density (called @2x) has a scale factor of 2, and a 3:1 density (called @3x) has a scale factor of 3." — images › Resolution

Tableau des facteurs d'échelle, recopié (source : images › Resolution ; renvoi interne de la HIG omis pour visionOS) :

| Platform | Scale factors |
|---|---|
| iPadOS, watchOS | @2x |
| iOS | @2x and @3x |
| visionOS | @2x or higher |
| macOS, tvOS | @1x and @2x |

Tableau des formats, recopié (source : images › Formats) :

| Image type | Format |
|---|---|
| Bitmap or raster work | De-interlaced PNG files |
| PNG graphics that don’t require full 24-bit color | An 8-bit color palette |
| Photos | JPEG files, optimized as necessary, or HEIC files |
| Stereo or spatial photos | Stereo HEIC |
| Flat icons, interface icons, and other flat artwork that requires high-resolution scaling | PDF or SVG files |

- Recadrage, redimensionnement et marges des images : non chiffré dans la HIG, la page ne comporte aucune section correspondante.

**Règles**

#### Résolution
- Ressources : fournir une version haute résolution de chaque image bitmap, pour chaque appareil — "Provide high-resolution assets for all bitmap images in your app, for every device you support." — images › Resolution
- Nommage : dans le catalogue de ressources, marquer le facteur d'échelle par le suffixe du nom de fichier (@1x, @2x, @3x) — "As you add each image to your project’s asset catalog, identify its scale factor by appending" — images › Resolution
- Méthode : dessiner à la plus basse résolution, puis monter en définition — "In general, design images at the lowest resolution and scale them up to create high-resolution assets." — images › Resolution
- Formes vectorielles redimensionnables : points de contrôle en valeurs entières, alignés à 1x — "position control points at whole values so that they’re cleanly aligned at 1x." — images › Resolution

#### Bonnes pratiques
- Couleur : joindre un profil colorimétrique à chaque image — "Include a color profile with each image." — images › Best practices
- Vérification : tester sur de vrais appareils, car une image peut ressortir pixélisée, étirée ou compressée — "An image that looks great at design time may appear pixelated, stretched, or compressed when viewed on various devices." — images › Best practices

**Spécifique macOS**
- Aucune considération propre à macOS : la page le déclare explicitement — "No additional considerations for iOS, iPadOS, or macOS." — images › Platform considerations
- Facteurs d'échelle de macOS : @1x and @2x (tableau de la section Resolution).
- La section Discussion de la page est vide dans l'export ; aucun format ni aucune taille propre à macOS n'est cité ailleurs.

**À transposer dans un shell Linux**
- Barre, dock, widgets, verrouillage : fournir chaque image bitmap en @1x et @2x au minimum, et sélectionner la version selon l'échelle de chaque écran.
- Lanceur, réglages, notifications : artwork plat en SVG ; bitmaps en PNG non entrelacé ; photos en JPEG ou HEIC.
- Tout le shell : cotes logiques en points, jamais en pixels ; le facteur d'échelle de l'écran assure la conversion.
- Pipeline : dessiner à @1x avec des points de contrôle en valeurs entières, puis décliner les autres définitions.
- Contrôle : profil colorimétrique inclus ; test sur des écrans 1x et 2x réels.

### Valeurs de référence

| Propriété | Valeur | Source (slug › section) |
|---|---|---|
| Modes de rendu d'un symbole | four rendering modes — monochrome, hierarchical, palette, and multicolor | sf-symbols › Rendering modes |
| Nombre de graisses de symboles | nine symbol weights — from ultralight to black | sf-symbols › Weights and scales |
| Échelles de symboles | three scales: small, medium (the default), and large | sf-symbols › Weights and scales |
| Référence des échelles | relative to the cap height of the San Francisco system font | sf-symbols › Weights and scales |
| Plage de la couleur variable | between zero and 100 percent | sf-symbols › Variable color |
| Version des dégradés | SF Symbols 7 and later | sf-symbols › Gradients |
| Version de Draw On / Draw Off | SF Symbols 7 and later | sf-symbols › Animations |
| Plus petite taille d'affichage d'une icône de document | 16x16 px | icons › Document icons |
| Fond d'icône de document (background fill) | 512x512 px @1x, 1024x1024 px @2x ; 256x256 px @1x, 512x512 px @2x ; 128x128 px @1x, 256x256 px @2x ; 32x32 px @1x, 64x64 px @2x ; 16x16 px @1x, 32x32 px @2x | icons › Document icons |
| Image centrale (center image) | 256x256 px @1x, 512x512 px @2x ; 128x128 px @1x, 256x256 px @2x ; 32x32 px @1x, 64x64 px @2x ; 16x16 px @1x, 32x32 px @2x | icons › Document icons |
| Taille de l'image centrale | half the size of the overall document icon canvas | icons › Document icons |
| Marge de l'image centrale | about 10% of the image canvas | icons › Document icons |
| Part occupée de l'image centrale | about 80% of the image canvas | icons › Document icons |
| Zone utile dans une toile de 256x256 px | 205x205 px | icons › Document icons |
| Couches d'une icône d'app | a background layer and one or more foreground layers | app-icons › Layer design |
| Icône d'app : forme de départ (macOS) | Square | app-icons › Specifications |
| Icône d'app : forme après masque du système (macOS) | Rounded rectangle (square) | app-icons › Specifications |
| Icône d'app : taille de la toile (macOS) | 1024x1024 px | app-icons › Specifications |
| Icône d'app : style (macOS) | Layered | app-icons › Specifications |
| Icône d'app : apparences (macOS) | Default, dark, clear light, clear dark, tinted light, tinted dark | app-icons › Specifications |
| Espaces colorimétriques des icônes d'app | sRGB (color) ; Gray Gamma 2.2 (grayscale) ; Display P3 (wide-gamut color in iOS, iPadOS, macOS, tvOS, and watchOS only) | app-icons › Specifications |
| Facteurs d'échelle : macOS, tvOS | @1x and @2x | images › Resolution |
| Facteurs d'échelle : iOS | @2x and @3x | images › Resolution |
| Facteurs d'échelle : iPadOS, watchOS | @2x | images › Resolution |
| Facteurs d'échelle : visionOS | @2x or higher | images › Resolution |
| Bitmap ou travail raster | De-interlaced PNG files | images › Formats |
| PNG sans pleine couleur 24 bits | An 8-bit color palette | images › Formats |
| Photos | JPEG files, optimized as necessary, or HEIC files | images › Formats |
| Photos stéréo ou spatiales | Stereo HEIC | images › Formats |
| Icônes plates, icônes d'interface et artwork plat à mettre à l'échelle | PDF or SVG files | images › Formats |

## Seconde partie — hyprdesk aujourd'hui, écarts et actions

### Ce que hyprdesk fait aujourd'hui
| Surface | Ce qui existe | Valeurs clés | Preuves |
|---|---|---|---|
| Jeu de glyphes d'interface | SVG écrit en mémoire, une seule couleur teintée ensuite ; variantes barrées ; batterie et Wi-Fi gradués | 111 noms, grille 24, trait 1,7, bouts et angles arrondis | `src/ui/icons.rs:1`, `:12`, `:15-55`, `:57-338` |
| Résolution et rendu d'une icône nommée | Jeu de hyprdesk d'abord si le nom finit par `-symbolic` ou commence par `hyprdesk-`, sinon thème puis Adwaita ; teinte des symboliques seulement | recherche à 32 | `src/ui/mod.rs:3439-3466`, `:3469-3503`, `:3505-3523` |
| Recherche dans le thème | SmartUI devant, chemin absolu sinon `freedesktop-icons` (taille, cache, thème) | `with_size` sans échelle ; classe à 48, bureau à `grid.icon_size` (24 à 128, 48 par défaut) | `src/launch.rs:493-507`, `:405-415`, `:533-578` ; `src/ui/mod.rs:2470`, `:2532-2535` ; `src/config/model.rs:797-806` |
| Taille des icônes de la barre | Liée à la taille du texte, ou `icon_size` en px | 1,2 × texte (texte = hauteur × 0,38, borné 11 à 18) ; sélecteur d'activité 1,25 × | `src/ui/plugins/bar/surface.rs:1904-1907` ; `src/ui/plugins/bar/style/selector.rs:153-154` |
| Dock | Icône brute, voile d'accent au survol, grossissement, repère d'activité qui pulse au lancement | `launcher_size` 24 à 96 (42 par défaut) ; voile 7 % | `src/ui/plugins/dock.rs:266-271` ; `src/ui/plugins/dock/surface.rs:458-465`, `:474-504`, `:582-590`, `:629-641`, `:667` |
| Lanceur d'applications | Tuile avec icône et nom, anneau de sélection | tuile 128 ; icône 60 (suggestions 64, carte centrée 46) | `src/ui/plugins/launcher/grid.rs:58-59`, `:76-77`, `:222-305`, `:338`, `:421-428` |
| Icônes du bureau | Fichiers et dossiers en icônes de type générique, corbeille | coupé par défaut ; repli : initiale sur accent | `src/desktop_files.rs:1-3`, `:83-91`, `:126-146` ; `src/ui/mod.rs:6066-6105` |
| Notifications | Bulle à cloche unique ; icône de l'application seulement dans la prise de main Island | cloche 18 | `src/ui/toasts.rs:36`, `:504-506` ; `src/ui/plugins/bar/surface.rs:2230-2234` ; `src/notifications.rs:150-173` |
| Aperçu des espaces, commutateur | Icône de classe dans les puces et les cartes | 18, 13, 26 | `src/ui/overview.rs:802-809`, `:979` ; `src/ui/switcher.rs:198` |
| Menus | Tête de ligne à 16 (coche à 14 si coché) ; glyphe du jeu ou du thème selon le nom | 16 / 14 | `src/ui/menu.rs:273-290` ; `src/ui/context.rs:501-554`, `:724-742`, `:1026` ; `src/ui/plugins/bar/window.rs:52-88` |
| Widget média, Réglages du dock | Icônes du thème en couleur, repli ASCII | recherche à 24, affichage 1,3 × (média), 0,6 × (Réglages) | `src/ui/mod.rs:91-103`, `:2532-2535`, `:4496-4502` ; `src/ui/widget_view.rs:129-139`, `:183-189` ; `src/ui/plugins/dock/surface.rs:530-546` |
| Verrouillage | Lecture et pause en caractère de police ; avatar ; fond flouté ou plein | `⏸` / `▶` 16 ; avatar 192 ; fond 480 ou 3840 | `src/lock/mod.rs:118-124`, `:144`, `:728`, `:744` |
| SmartUI design | Option : thème d'icônes freedesktop généré par IA, devant le thème sur toutes les surfaces | SVG validé ; squircle 22,37 %, dégradés, reflet, ombre | `src/smartui.rs:1-13`, `:54-73`, `:176-195`, `:219-226` ; `src/ui/plugins/dock.rs:290-311` |
| Fonds d'écran | Quatre ajustements, zoom, point focal, flou, clair/sombre, vidéo, scène étalée ; vignettes | rééchantillonné à la taille du moniteur ; vignettes 320×180 pour 160×90 | `src/ui/mod.rs:779-838`, `:3041-3049`, `:6030-6059` ; `src/ui/plugins/wallpaper.rs:127`, `:192-206`, `:496-522` |
| Pochette, portrait | Pochette cadrée et arrondie ; portrait LYRA masqué en cercle | rayon 8 ; portrait 423×305 vers 256×256 | `src/ui/widget_view.rs:155-165` ; `src/ui/chat/appearance.rs:21-31`, `:248-258` |
| Images embarquées | Logo, fonds du gestionnaire de connexion, mire, portrait, illustrations | 3 PNG non entrelacés, 6 JPEG ; aucun profil | `assets/greeter/`, `assets/colors/mire.png`, `assets/lyra/portrait.png`, `assets/wizard/` |

### Écarts
Classés du plus important au moins important. Effort : S (moins d'un jour), M (un à trois jours), L (plus de trois jours).

| # | Composant | Nature | Force | Règle HIG (slug › section) | Réel dans hyprdesk (preuve) | Action proposée | Effort | Confiance |
|---|---|---|---|---|---|---|---|---|
| ICO-01 | Icônes d'application (lanceur, dock, aperçu, commutateur, sélecteur d'activité, menu de fenêtre) | ÉCART | Impératif : « Provide high-resolution assets for all bitmap images in your app, for every device you support. » ; tableau @1x and @2x | images › Resolution | Recherche dans le thème à taille fixe, sans échelle d'écran. Toute icône nommée est cherchée à 32 (`src/ui/mod.rs:3493-3494`) alors que la grille du lanceur l'affiche à 60 (`src/ui/plugins/launcher/grid.rs:59`, `:421-428`) et les suggestions à 64 (`:338`). Les icônes de classe sont cherchées à 48 (`src/ui/mod.rs:2470`) pour un dock réglable de 24 à 96 (`src/ui/plugins/dock.rs:267`). `lookup_icon` appelle `with_size` sans `with_scale` (`src/launch.rs:493-507`) ; `Monitor.scale` (`src/hypr.rs:36`) ne sert qu'aux captures et à l'avis de résolution (`src/ui/context.rs:1718-1720`, `src/ui/plugins/wallpaper.rs:500-514`). La crate rend le PNG de la taille demandée quand il existe (`freedesktop-icons-0.4.0/src/theme/mod.rs:36-37`) ; sur cette machine 47 `.desktop` n'ont que des bitmaps (OpenJDK : 48 px au plus) et l'écran HDMI-A-1 est à l'échelle 1,6. | Chercher chaque icône dans le thème à la taille affichée multipliée par l'échelle de l'écran (`with_size`, `with_scale`), par surface. | M | haute |
| ICO-02 | Commandes média et volume, bouton Réglages du dock | ÉCART | Impératif : « Maintain visual consistency across all interface icons in your app. » | icons › Best practices | Les quatre commandes média, le volume et `preferences-system` sont pris dans le thème d'icônes, en couleur, avant le jeu de hyprdesk : `UI_ICONS` sans suffixe `-symbolic` (`src/ui/mod.rs:91-103`), recherche directe à 24 (`:2532-2535`), thème d'abord et jeu de secours seulement si le thème manque (`:4496-4502`), rendu sans teinte (`:3505-3523`). Le widget média les affiche à 1,3 × la taille (`src/ui/widget_view.rs:130-134`, `:183-189`). Le dock autonome affiche le Réglages du thème à 0,6 × (`src/ui/plugins/dock/surface.rs:537-545`) alors que le dock intégré Graphite prend le glyphe de hyprdesk à 0,85 (`:530-536`). Le jeu définit pourtant ces commandes (`src/ui/icons.rs:101-103`, `:118-135`) et l'OSD les utilise (`src/ui/osd.rs:51-66`). DESIGN.md:8 : « icônes symboliques pour les commandes ». | Passer `UI_ICONS` par `icons::get` d'abord, comme `want_icon` le fait pour les noms `-symbolic`. | S | haute |
| ICO-03 | Bulle de notification | ABSENT | Rôle de l'icône d'app : « including search results, notifications, system settings, and share sheets. » | app-icons › Discussion | Toute bulle porte la même cloche à 18 (`src/ui/toasts.rs:36`, `:504-506`). Le serveur relève pourtant l'icône envoyée et le `desktop-entry` (`src/notifications.rs:150-173`), et `t.n.icon` n'est dessiné que dans la prise de main Island (`src/ui/plugins/bar/surface.rs:2230-2234`). Aucun chemin de `toasts.rs` ne lit l'icône ni l'image jointe. | Remplacer la cloche par l'icône de l'application émettrice (`n.icon`, repli sur l'icône de sa classe). | S | haute |
| ICO-04 | Icônes d'application (dock, lanceur, aperçu) | ABSENT | Impératif : « provide square layers so the system can apply rounded corners » ; « the system applies masking to produce rounded corners that precisely match the curvature of other rounded interface elements throughout the system » | app-icons › Icon shape | Aucun masque ni découpe. `img_element` et `icon_named` posent l'image brute (`src/ui/mod.rs:3458-3462`, `:3507-3512`) ; le dock ne l'entoure que d'un voile d'accent à 7 % au survol (`src/ui/plugins/dock/surface.rs:629-641`). La forme vient donc du thème installé (cercles avec Tela-circle sur la session) ou, avec SmartUI, de la ressource (ICO-05). | Appliquer aux icônes d'application un masque unique en rectangle arrondi, piloté par le thème, de même famille de courbure que les cartes de l'interface. | M | moyenne |
| ICO-05 | SmartUI design | ÉCART | Impératif : « Let the system handle blurring and other visual effects. » ; « Providing layers with pre-defined masking negatively impacts specular highlight effects and makes edges look jagged. » | app-icons › Visual effects ; app-icons › Icon shape | Le style par défaut demande un squircle de 22,37 % déjà découpé, un dégradé de fond, un reflet interne, une ombre portée et un reflet spéculaire (`src/smartui.rs:54-73`, lignes 57 et 63-65). L'aperçu livré confirme : `clipPath` arrondi et dégradés `spec`, `hl`, `glow` dans la ressource (`assets/preview/smartui/firefox.svg:1`). Ces icônes remplacent celles du thème partout (`src/launch.rs:495-497`). | Faire générer un carré plein cadre, sans découpe, reflet ni ombre, et confier la forme au masque d'ICO-04. | S | haute |
| ICO-06 | Menus (bureau, fenêtre, widget) et Panorama | ÉCART | Impératif : « Maintain visual consistency across all interface icons in your app. » ; « Create a custom symbol that's consistent with the ones the system provides in level of detail, optical weight, alignment, position, and perspective. » | icons › Best practices ; sf-symbols › Custom symbols | `want_icon` ne prend le jeu de hyprdesk que pour les noms qu'il contient (`src/ui/mod.rs:3475-3479`) ; les autres viennent du thème (`:3493-3494`). Au moins 18 noms de commande sont hors jeu, dont 16 dans des menus. Un même menu du bureau mêle `preferences-system` (jeu) et `document-properties`, `list-remove`, `preferences-desktop-wallpaper` (thème) (`src/ui/context.rs:513-554`, résolution `:1026`) ; y manquent aussi copier, couper, tout sélectionner (`:595-618`) et réduire, agrandir, plein écran, flottante, épingler (`src/ui/plugins/bar/window.rs:55-84`). Panorama ajoute `go-up`, `go-down`, `media-seek-*` (`src/panorama/design.rs:97-99`, `:173-183`). Le test `interface_entierement_dessinee` ne couvre que quatre fichiers (`src/ui/icons.rs:349-353`). | Dessiner dans `src/ui/icons.rs` les glyphes manquants et étendre le test à `context.rs`, `bar/window.rs` et Panorama. | M | haute |
| ICO-07 | Fermeture, lecture et pause | ÉCART | Impératif : « Maintain visual consistency across all interface icons in your app. » | icons › Best practices | La fermeture est un caractère de police `✕` ou `×` en onze endroits (`src/ui/toasts.rs:519`, `src/ui/controls.rs:218`, `src/ui/convex.rs:356`, `src/ui/curves.rs:268`, `src/ui/hyprset.rs:628`, `src/ui/hyprlists.rs:390`, `:931`, `src/ui/chat.rs:1183`, `:1661`, `src/ui/plugins/panorama/surface/panes.rs:1667`, `src/ui/plugins/panorama/surface/dashboard.rs:180`) alors que les menus prennent le glyphe vectoriel `window-close` (`src/ui/context.rs:814`, `:823` ; `src/ui/icons.rs:137`). Le verrouillage n'utilise jamais le jeu : lecture et pause en `⏸` / `▶`, police par défaut (`src/lock/mod.rs:744`), `⚡` dans la batterie (`:728`). | Remplacer ces caractères par les glyphes de `src/ui/icons.rs` (`window-close`, `media-playback-start`, `media-playback-pause`). | S | haute |
| ICO-08 | Variantes « indisponible » | ÉCART | Impératif : « Maintain visual consistency across all interface icons in your app. » ; « the slash variant of a symbol to show that an item or action is unavailable » | icons › Best practices ; sf-symbols › Design variants | La barre de refus `M4 4l16 16` est celle de cinq glyphes (`src/ui/icons.rs:91`, `:128`, `:237`, `:243`, `:333`). Trois s'en écartent : volume coupé en croix (`:130`), réseau hors ligne en croix courte sous des arcs différents de ceux du Wi-Fi (`:230-232`), bureau masqué en barre plus inclinée (`:96`). Volume bas et volume moyen partagent un dessin (`:121-123`). | Reprendre la barre `M4 4l16 16` et les arcs du Wi-Fi sur ces trois glyphes. | S | haute |
| ICO-09 | Icône de remplacement (application sans icône) | ÉCART | Impératif : « Provide a visually consistent icon design across all the platforms your app supports. » | app-icons › Design | Trois dessins pour la même application. Dock : initiale à 0,5 ×, police du corps, texte au premier plan, accent à 16 %, rayon 0,24 × (`src/ui/plugins/dock/surface.rs:476-504`). Lanceur : initiale à 0,45 ×, police des titres, texte `on_button`, accent plein, rayon 0,28 × (`src/ui/plugins/launcher/grid.rs:228-251`). Bureau : initiale à 0,5 ×, police du corps, accent plein, rayon 0,25 × (`src/ui/mod.rs:6087-6105`). | Une seule fonction d'icône de remplacement (initiale, police, couleur, rayon), appelée par les trois surfaces. | S | haute |
| ICO-10 | Apparence des icônes d'application | ABSENT | Impératif : « Keep your icon's features consistent across appearances. » ; tableau Default, dark, clear light, clear dark, tinted light, tinted dark | app-icons › Appearances ; app-icons › Specifications | Le moteur de thème ne choisit que le nom du thème d'icônes (`src/theme_engine/resolve.rs:48-49`, `:170`, `:212`). Aucune variante sombre, claire ou teintée, aucun réglage d'apparence : aucune clé `icon` dans `src/design_options.rs`, aucune teinte d'icône dans `src/ui/mod.rs`. | Ajouter une apparence des icônes (par défaut, sombre, teintée) pilotée par le mode du thème, la variante sombre dérivée de l'icône claire. | L | moyenne |
| ICO-11 | Icônes de fichiers du bureau | ABSENT | Description du comportement système : « macOS creates one for you by compositing your app icon and the file's extension onto the canvas. » | icons › Document icons | Un fichier reçoit une icône de type générique d'après quelques familles d'extension, sinon `text-x-generic` (`src/launch.rs:510-529`, `:570-578` ; `src/desktop_files.rs:83-91`, `icon: None`). Aucune icône de l'application par défaut, aucune extension composée. Réglage coupé par défaut (`src/desktop_files.rs:1-3`). | Composer l'icône du fichier d'après son type MIME : icône de type du thème, sinon icône de l'application par défaut avec l'extension. | M | moyenne |
| ICO-12 | Graisse des glyphes | PISTE | Recommandation : « In general, match the weights of interface icons and adjacent text. » | icons › Best practices ; sf-symbols › Weights and scales | Un seul trait, `stroke-width="1.7"`, pour les 111 noms (`src/ui/icons.rs:337`), alors que l'interface emploie plusieurs graisses de texte (`src/ui/mod.rs:3426-3436`). | Paramétrer la graisse du trait (au moins normal et semibold) et l'accorder à celle du texte voisin. | M | haute |
| ICO-13 | Taille des glyphes hors barre | PISTE | Recommandation : « The scales are defined relative to the cap height of the San Francisco system font. » | sf-symbols › Weights and scales | Les tailles sont des constantes : 16 (coche 14) dans les menus (`src/ui/menu.rs:287`), 18 dans les bulles (`src/ui/toasts.rs:505`), 16 dans les popovers (`src/ui/plugins/bar/popover.rs:1746`, `src/ui/plugins/bar/popover/bar_menus.rs:121`). Le texte passe par `tokens::ts`, qui suit le thème, l'écran et l'utilisateur (`src/ui/tokens.rs:231-242`) ; aucun appel `icon_named` ne prend `ts(`. Seule la barre lie icône et texte (`src/ui/plugins/bar/surface.rs:1905-1907`). | Dériver la taille des glyphes de `tokens::ts` appliqué à la taille du texte voisin, comme le fait la barre. | S | haute |
| ICO-14 | Changement d'état d'un symbole | PISTE | Description : « Down-up, where the outgoing symbol scales down and the incoming symbol scales up, communicating a change in state. » | sf-symbols › Animations | Lecture et pause se remplacent sans transition, dans le widget média (`src/ui/widget_view.rs:183-189`) et sur le verrouillage (`src/lock/mod.rs:744`). Seule l'activité de lancement est animée (`src/ui/plugins/dock/surface.rs:582-590`, `:667`). | Jouer un remplacement (down-up) au changement lecture et pause, coupé par `transition = "none"`. | M | moyenne |
| ICO-15 | Images embarquées | ÉCART | Impératif : « Include a color profile with each image. » | images › Best practices | Les trois PNG livrés n'ont ni `iCCP` ni `sRGB` (`assets/greeter/logo.png`, `assets/colors/mire.png`, `assets/lyra/portrait.png`) ; les six JPEG n'ont pas de profil ICC (`assets/greeter/fond.jpg`, `assets/greeter/fond-vertical.jpg`, `assets/wizard/bar-*.jpg`). `mire.png` sert de mire de couleurs (`src/colors.rs:308`). | Incorporer un profil sRGB à chaque image embarquée. | S | haute |

Choix d'identité signalé, jamais à corriger :

| # | Composant | Nature | Force | Règle HIG | Réel dans hyprdesk | Action | Effort | Confiance |
|---|---|---|---|---|---|---|---|---|
| ICO-16 | Thèmes d'icônes système pour les applications | IDENTITÉ | — | — | DESIGN.md:8 : « thèmes système pour les applications ; icônes symboliques pour les commandes » ; `design-plans/barre-continuite.md:31`. ICO-01, ICO-04 et ICO-10 portent sur la façon de les afficher, pas sur ce choix. | Aucune. | — | — |

Falsification : deux écarts cherchés n'ont pas résisté et ne figurent pas ci-dessus. Le fond d'écran rééchantillonné à la taille du moniteur serait dégradé à l'échelle 2 si cette taille était logique, ce que le code commente (`src/ui/plugins/dock/contrast.rs:164`) ; sur Hyprland 0.57.2, `hyprctl monitors -j` rend les pixels du mode (HDMI-A-1 : 2560×720 à l'échelle 1,6), donc pleine résolution (voir À vérifier 5). L'étirement des icônes non carrées n'existe pas : l'ajustement par défaut de l'image est `Contain` (`iced_core-0.14.0/src/content_fit.rs:28-29`).

### Ce qui est déjà conforme
- Icônes d'interface vectorielles (icons › Best practices : « use a vector format like PDF or SVG ») | SVG écrit en mémoire, `src/ui/icons.rs:337` ; SVG du thème rendus en tant que tels, `src/ui/mod.rs:3412-3416`.
- Un jeu de même taille, trait et niveau de détail (icons › Best practices) | grille 24, trait 1,7, bouts et angles arrondis pour les 111 noms, `src/ui/icons.rs:1`, `:337`.
- Barré pour l'indisponible (sf-symbols › Design variants) | Wi-Fi, micro, Bluetooth, éclairage nocturne, notifications : `src/ui/icons.rs:91`, `:128`, `:237`, `:243`, `:333`.
- Forme englobante en petite taille (sf-symbols › Design variants) | cercles de `help-about` et de `preferences-desktop-theme`, `src/ui/icons.rs:154-161`.
- Couleur variable par couches, jamais pour la profondeur (sf-symbols › Variable color) | arcs Wi-Fi estompés au-delà du signal, `src/ui/icons.rs:11-12`, `:43-55` ; batterie remplie selon le niveau, `:15-40` ; volume à trois seuils, `:118-130`.
- Rendu monochrome aux couleurs du thème (sf-symbols › Rendering modes) | teinte au premier plan ou au gris du texte, `src/ui/mod.rs:3447-3453`.
- Échelle liée au texte dans la barre (sf-symbols › Weights and scales) | `src/ui/plugins/bar/surface.rs:1905-1907`, `src/ui/plugins/bar/style/selector.rs:153-154`.
- État sélectionné rendu par le composant, sans seconde ressource (icons › Best practices) | anneau de la tuile, `src/ui/plugins/launcher/grid.rs:294-299` ; coche de tête, `src/ui/menu.rs:273-290`.
- Animation fonctionnelle et sobre (sf-symbols › Animations) | pulsation du seul repère de lancement, `src/ui/plugins/dock/surface.rs:582-590`, `:667` ; `transition = "none"` la coupe (`:583-585`).
- Icônes standard (icons › Editing, Search, Other) | loupe `src/ui/icons.rs:174-176`, points de suspension `:111-113`, croix `:137`, corbeille `:214-216`, plus `:211`, crayon `:267-269`, partage `:162-164`, calendrier `:263-265`.
- Une icône par application sur toutes les surfaces (app-icons › Discussion) | un seul chemin de recherche, `src/launch.rs:493-507` ; SmartUI devant, `src/smartui.rs:219-226`.
- SmartUI : vecteur, sans texte, glyphe centré et margé, pas de logo Apple (app-icons › Design, Icon shape) | `src/smartui.rs:12-13`, `:58-62`, `:69-73`.
- Formats et définitions (images › Formats, Resolution) | PNG non entrelacés dans `assets/` ; vignettes de fonds décodées en 320×180 pour 160×90 affichés, `src/ui/plugins/wallpaper.rs:127`, `:197-202` ; cotes en points logiques, Iced appliquant l'échelle.
- Sens de lecture droite à gauche (icons › Best practices) | sans objet : interface livrée en français et en anglais, `src/i18n.rs:236`.

### À vérifier sur un rendu
1. Lisibilité des icônes tierces en petit : 13 dans la carte de fenêtre de l'aperçu (`src/ui/overview.rs:979`), 18 dans les puces (`:809`) et les bulles, 23 au plus dans le sélecteur d'activité (`src/ui/plugins/bar/style/selector.rs:153-154`).
2. Poids visuel interne du jeu : pause en deux traits de 1,7 face au triangle plein de lecture (`src/ui/icons.rs:131-132`), points de suspension de rayon 1 (`:111-113`), trait ramené à 1,1 pour une icône de 16.
3. Écart visible entre les glyphes de hyprdesk et ceux du thème actif (Tela-circle-dracula-pro-dark) dans un même menu, pour ICO-06 : capture du menu du bureau.
4. Netteté réelle des icônes bitmap dans le lanceur et le dock sur HDMI-A-1 (échelle 1,6) et au réglage de 96, pour ICO-01.
5. Unité de la taille du moniteur. Le fond est rééchantillonné à `width × height` (`src/ui/mod.rs:807-825`). Le code tient ces valeurs pour logiques (`src/ui/plugins/dock/contrast.rs:164`) et les multiplie par `scale` ailleurs (`src/ui/context.rs:1718-1720`, `src/ui/plugins/wallpaper.rs:500-514`), alors que Hyprland 0.57.2 rend ici les pixels du mode. Contrôler la netteté du fond sur une session où la taille reçue serait logique.
6. Réduction des fonds avec le filtre `Triangle` pour de forts rapports (`src/ui/mod.rs:807-820`), par exemple un 4K affiché en 1080p.
7. Ajustement « fill » du fond, qui étire l'image (`src/ui/mod.rs:807`, choix proposé dans `src/ui/plugins/wallpaper.rs:526-536`).
8. Icônes non carrées : ajustement `Contain`, donc un vide autour de l'icône ; vérifier qu'aucune icône du thème actif n'est concernée.
9. Effets du dock : aucun reflet ni ombre propre aux icônes, un voile d'accent à 7 % au survol (`src/ui/plugins/dock/surface.rs:629-641`). La HIG ne tranche pas ; à juger sur un rendu.
