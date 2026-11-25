# Composants : disposition et organisation — macOS d'après la HIG, face à hyprdesk

> Proposition de design, rien d'implémenté. Plan d'ensemble et priorités : `plan-d-action.md` ; méthode et périmètre : `README.md`.

> Référentiel : Human Interface Guidelines d'Apple, export Markdown officiel relevé le 23 novembre 2026. Code : commit `00f6d69` (arbre de travail du 23 novembre 2026, avec des modifications non commitées).

## Première partie — Spécification d'après la HIG

Convention de preuve : chaque règle chiffrée ou normative porte, sur sa puce, une citation anglaise mot pour mot (30 mots au plus) suivie de sa source `slug › section` ; la section est le titre exact du sous-titre HIG le plus proche, les rubriques `macOS` étant les sous-sections de « Platform considerations ». La mention « non chiffré dans la HIG » signale une absence dans les pages sources : aucune valeur ne doit en être déduite, ni complétée d'après une connaissance personnelle de macOS. Les rubriques « À transposer dans un shell Linux » sont des propositions dérivées des règles citées, non des énoncés de la HIG.

### Résumé
- Dix composants organisent le contenu : groupes (boxes), ensembles visuels (collections), hiérarchies (column views, outline views), rangées (lists and tables), volets adjacents (split views) ou exclusifs (tab views), plus deux éléments transversaux, les libellés (labels) et les contrôles de divulgation (disclosure controls).
- La HIG ne donne ici presque aucune cote : ni hauteur de rangée, ni taille de cellule, ni rayon, ni espacement. Une seule dimension, le séparateur fin d'une vue scindée, large d'un point.
- Les autres limites chiffrées sont des nombres : au plus six onglets, au plus un bouton de divulgation par vue, quatre couleurs de libellé, quatre types de lockups.
- Deux composants n'existent que sur macOS (column views, outline views) ; lockups n'existe que sur tvOS et n'apporte donc aucune règle macOS.
- Pour un shell, l'essentiel vient des lists and tables (notifications, réglages), des split views et tab views (réglages), des collections (grilles du lanceur, aperçu des espaces), des labels et, pour les écrans de réglages, des disclosure controls et des boxes.

### Layout and organization (`layout-and-organization`)
**Rôle** — Page d'index : définir en une phrase chacun des dix composants qui organisent la disposition du contenu.
**Anatomie et cotes** — non chiffré dans la HIG : la page ne contient que des définitions.

**Règles** — définitions des dix composants (noms anglais gardés) :
- *Boxes* : groupe visuellement distinct d'informations et de composants logiquement liés — "A box creates a visually distinct group of logically related information and components." — layout-and-organization › Discussion
- *Collections* : ensemble ordonné de contenu présenté dans une disposition personnalisable et très visuelle — "A collection manages an ordered set of content and presents it in a customizable and highly visual layout." — layout-and-organization › Discussion
- *Column views* : parcours d'une hiérarchie de données par une série de colonnes verticales — "lets people view and navigate a data hierarchy using a series of vertical columns" — layout-and-organization › Discussion
- *Disclosure controls* : révèlent et masquent informations et fonctions liées à des contrôles ou des vues — "Disclosure controls reveal and hide information and functionality related to specific controls or views." — layout-and-organization › Discussion
- *Labels* : texte statique, lisible et souvent copiable, mais non modifiable — "A label is a static piece of text that people can read and often copy, but not edit." — layout-and-organization › Discussion
- *Lists and tables* : données en une ou plusieurs colonnes de rangées — "Lists and tables present data in one or more columns of rows." — layout-and-organization › Discussion
- *Lockups* : plusieurs vues distinctes réunies en une seule unité interactive — "Lockups combine multiple separate views into a single, interactive unit." — layout-and-organization › Discussion
- *Outline views* : données hiérarchiques en liste défilante de cellules organisées en colonnes et rangées — "An outline view presents hierarchical data in a scrolling list of cells that are organized into columns and rows." — layout-and-organization › Discussion
- *Split views* : plusieurs volets de contenu adjacents, pouvant contenir tables, collections, images et vues personnalisées — "A split view manages the presentation of multiple adjacent panes of content" — layout-and-organization › Discussion
- *Tab views* : plusieurs volets de contenu mutuellement exclusifs dans une même zone, changés par un contrôle à onglets — "A tab view presents multiple mutually exclusive panes of content in the same area, which people can switch between using a tabbed control." — layout-and-organization › Discussion

**Spécifique macOS** — aucun bloc de plateforme sur cette page d'index ; la répartition par plateforme figure dans chaque page de composant (rubrique « Spécifique macOS » ci-dessous).

**À transposer dans un shell Linux**
- Écran de réglages : split view (catégories d'un côté, détail de l'autre) ou tab view ; groupes en boxes ; options avancées derrière des disclosure controls ; texte en labels et lists and tables.
- Notifications et résultats de recherche : lists and tables, jamais de collections, car il s'agit de texte.
- Grilles d'applications du lanceur et aperçu des espaces : collections (grille), et non des lockups, absents de macOS.

### Boxes (`boxes`)
**Rôle** — Créer, par une bordure ou un fond, un groupe visuellement distinct d'informations et de composants logiquement liés, avec titre facultatif.
**Anatomie et cotes** — non chiffré dans la HIG : ni rayon, ni épaisseur de bordure, ni marge interne. Éléments nommés : bordure visible ou couleur de fond, titre facultatif.

**Règles**
- *Apparence* : par défaut, une boîte sépare son contenu du reste de l'interface par une bordure visible ou une couleur de fond ; elle peut aussi comporter un titre — "By default, a box uses a visible border or background color to separate its contents from the rest of the interface." — boxes › Discussion
- *Taille* : la garder relativement petite par rapport à sa vue conteneur — "Prefer keeping a box relatively small in comparison with its containing view." — boxes › Best practices
- *Taille* : approchant la taille de la fenêtre ou de l'écran, elle sépare moins bien les groupes et encombre le reste — "As a box’s size gets close to the size of the containing window or screen, it becomes less effective at communicating the separation of grouped content" — boxes › Best practices
- *Sous-groupes* : exprimer les groupes internes par la marge interne et l'alignement — "Consider using padding and alignment to communicate additional grouping within a box." — boxes › Best practices
- *Sous-groupes* : les boîtes imbriquées donnent une interface chargée et contrainte — "adding nested boxes to define subgroups can make your interface feel busy and constrained" — boxes › Best practices
- *Titre* : un titre introductif succinct s'il éclaire le contenu, sous la forme d'une brève phrase qui le décrit — "Provide a succinct introductory title if it helps clarify the box’s contents." — boxes › Content
- *Titre* : majuscules de phrase et pas de ponctuation finale, sauf dans un volet de réglages, où le titre se termine par deux-points — "Use sentence-style capitalization. Avoid ending punctuation unless you use a box in a settings pane, where you append a colon to the title." — boxes › Content

**Spécifique macOS**
- Par défaut, macOS affiche le titre de la boîte au-dessus d'elle — "By default, macOS displays a box’s title above it." — boxes › macOS

**À transposer dans un shell Linux**
- Panneaux de la barre (réglages rapides) et écran de réglages : chaque groupe forme une boîte petite relativement au panneau, sans boîtes imbriquées ; les sous-groupes passent par l'espacement et l'alignement.
- Titres de groupes dans un volet de réglages : phrase courte en majuscules de phrase, terminée par deux-points et placée au-dessus du groupe.
- Cartes de notification et widgets : un seul niveau de cadre.

### Collections (`collections`)
**Rôle** — Gérer un ensemble ordonné de contenu, surtout graphique, et le présenter dans une disposition personnalisable et très visuelle.
**Anatomie et cotes** — non chiffré dans la HIG : ni taille de cellule, ni espacement, ni rayon. Dispositions par défaut : rangée horizontale ou grille.

**Règles**
- *Usage* : idéales pour le contenu graphique — "Generally speaking, collections are ideal for showing image-based content." — collections › Discussion
- *Usage* : pour du texte, préférer une table, une liste défilante rendant l'information textuelle plus simple à parcourir — "Consider using a table instead of a collection for text." — collections › Best practices
- *Disposition* : employer la disposition standard en rangée ou en grille, simple, efficace et attendue — "Use the standard row or grid layout whenever possible." — collections › Best practices
- *Disposition* : éviter une disposition personnalisée qui déroute ou attire l'attention sur elle-même — "Avoid creating a custom layout that might confuse people or draw undue attention to itself." — collections › Best practices
- *Sélection* : faciliter le choix d'un élément — "Make it easy to choose an item." — collections › Best practices
- *Sélection* : marge interne suffisante autour des images pour que le focus ou le survol reste visible et que le contenu ne se chevauche pas — "Use adequate padding around images to keep focus or hover effects easy to see and prevent content from overlapping." — collections › Best practices
- *Interactions et retour* : des interactions personnalisées peuvent s'ajouter au besoin ; animer l'insertion, la suppression et la réorganisation des éléments — "Consider using animations to provide feedback when people insert, delete, or reorder items." — collections › Best practices

**Spécifique macOS** — aucune considération supplémentaire pour macOS ; composant non pris en charge sur watchOS.

**À transposer dans un shell Linux**
- Grilles d'applications (lanceur, aperçu des espaces) : grille standard de cellules de taille identique, avec assez de marge autour de chaque vignette pour que le survol et la sélection restent visibles.
- Réorganisation du dock ou de la grille : retour animé à l'insertion, à la suppression et au déplacement.
- Listes de notifications et résultats de recherche : ce sont des textes, donc des lists and tables, pas des collections.

### Column views (`column-views`)
**Rôle** — Voir et parcourir une hiérarchie de données dans une série de colonnes verticales, chaque colonne représentant un niveau ; composant aussi appelé *browser*.
**Anatomie et cotes** — non chiffré dans la HIG : ni largeur de colonne par défaut, ni hauteur de rangée, ni taille de triangle. Éléments : colonnes de rangées horizontales, triangle sur les éléments parents, colonnes redimensionnables.

**Règles**
- *Fonctionnement* : tout élément parent contenant des enfants est marqué d'une icône triangle ; sélectionner un parent affiche ses enfants dans la colonne suivante — "When people select a parent, the next column displays its children." — column-views › Discussion
- *Fonctionnement* : on peut remonter la hiérarchie pour explorer d'autres branches — "can also navigate back up the hierarchy to explore other branches of data" — column-views › Discussion
- *Quand* : hiérarchie profonde, allers-retours fréquents entre niveaux, pas besoin de tri — "Consider using a column view when you have a deep data hierarchy in which people tend to navigate back and forth frequently between levels" — column-views › Best practices
- *Racine* : montrer la racine de la hiérarchie dans la première colonne, où l'on peut revenir vite pour repartir du sommet — "Show the root level of your data hierarchy in the first column." — column-views › Best practices
- *Feuille* : quand l'élément sélectionné n'a pas d'enfants, envisager d'afficher une information le concernant (le Finder y montre un aperçu et des dates, type et taille) — "Consider showing information about the selected item when there are no nested items to display." — column-views › Best practices
- *Largeur* : laisser redimensionner les colonnes, surtout si des noms dépassent la largeur par défaut — "Let people resize columns." — column-views › Best practices

**Spécifique macOS** — composant propre à macOS.
- Il n'est pris en charge par aucune des autres plateformes citées — "Not supported in iOS, iPadOS, tvOS, visionOS, or watchOS." — column-views › Platform considerations
- Le Finder propose la vue en colonnes, en plus des vues icônes, liste et galerie, pour parcourir les répertoires — "Finder offers a column view (in addition to icon, list, and gallery views) for navigating directory structures." — column-views › Best practices

**À transposer dans un shell Linux**
- Peu de surfaces d'un shell ont une hiérarchie profonde ; à réserver à un navigateur de fichiers intégré (lanceur, choix d'un fond d'écran par dossiers).
- Marquer d'un triangle tout élément ayant des enfants ; la sélection met à jour la colonne suivante ; la racine reste en première colonne.
- Quand l'élément n'a pas d'enfants, la colonne suivante affiche un aperçu ou des informations.

### Disclosure controls (`disclosure-controls`)
**Rôle** — Révéler et masquer des informations ou des fonctions liées à une vue ou à une liste (triangle de divulgation, disclosure triangle) ou à un contrôle précis (bouton de divulgation, disclosure button).
**Anatomie et cotes** — non chiffré dans la HIG : ni taille, ni espacement, ni durée d'animation. Deux états, **Collapsed** et **Expanded**. Limite : au plus un bouton de divulgation par vue.

**Règles**
- *Principe* : masquer les détails jusqu'à ce qu'ils soient pertinents — "Use a disclosure control to hide details until they’re relevant." — disclosure-controls › Best practices
- *Principe* : placer en haut de la hiérarchie de divulgation les contrôles les plus utilisés, toujours visibles, et masquer par défaut les fonctions avancées — "Place controls that people are most likely to use at the top of the disclosure hierarchy so they’re always visible, with more advanced functionality hidden by default." — disclosure-controls › Best practices
- *Triangle* : il pointe vers l'intérieur depuis le bord d'origine quand le contenu est caché, vers le bas quand il est visible — "A disclosure triangle points inward from the leading edge when its content is hidden and down when its content is visible." — disclosure-controls › Disclosure triangles
- *Triangle* : un clic ou un toucher bascule entre les deux états, la vue s'étendant ou se repliant en conséquence — "Clicking or tapping the disclosure triangle switches between these two states, and the view expands or collapses accordingly to accommodate the content." — disclosure-controls › Disclosure triangles
- *Triangle* : fournir un libellé descriptif indiquant ce qui est révélé ou masqué — "Make sure your labels indicate what is disclosed or hidden" — disclosure-controls › Disclosure triangles
- *Bouton* : il montre et cache des fonctions liées à un contrôle précis — "A disclosure button shows and hides functionality associated with a specific control." — disclosure-controls › Disclosure buttons
- *Bouton* : il pointe vers le bas quand le contenu est caché, vers le haut quand il est visible — "A disclosure button points down when its content is hidden and up when its content is visible." — disclosure-controls › Disclosure buttons
- *Bouton* : le placer près du contenu qu'il montre et cache, pour une relation claire — "Place a disclosure button near the content that it shows and hides." — disclosure-controls › Disclosure buttons
- *Bouton* : au plus un par vue, plusieurs ajoutant de la complexité et prêtant à confusion — "Use no more than one disclosure button in a single view." — disclosure-controls › Disclosure buttons

**Spécifique macOS** — aucune considération supplémentaire pour macOS.
- Exemple de bouton : la feuille d'enregistrement de macOS place un bouton de divulgation à côté du champ Save As — "the macOS Save sheet shows a disclosure button next to the Save As text field" — disclosure-controls › Disclosure buttons
- Exemples de triangles : Keynote révèle des options avancées à l'export et le Finder révèle progressivement une arborescence en vue liste — "Keynote uses a disclosure triangle to show advanced options when exporting a presentation, and the Finder uses disclosure triangles" — disclosure-controls › Disclosure triangles

**À transposer dans un shell Linux**
- Écran de réglages : réglages courants visibles d'emblée ; réglages avancés derrière un triangle flanqué d'un libellé (par exemple « Options avancées »), qui déploie le contenu dans la même vue.
- Sens des chevrons : triangle pointant vers l'intérieur = replié, vers le bas = déplié ; bouton pointant vers le bas = caché, vers le haut = visible.
- Volets de la barre et du lanceur : au plus un bouton de divulgation par vue (filtres, options de recherche), placé près de ce qu'il révèle.

### Labels (`labels`)
**Rôle** — Afficher un texte statique, lisible et souvent copiable mais non modifiable, dans les boutons, les éléments de menu et les vues, pour faire comprendre le contexte et la suite possible.
**Anatomie et cotes** — non chiffré dans la HIG : ni corps de police, ni interligne, ni espacement. Le système définit quatre couleurs de libellé, dont les valeurs ne sont pas données sur cette page. La police système et Dynamic Type (où disponible) sont pris en charge par défaut.

**Règles**
- *Usage* : dans un bouton, le libellé dit ce que fait le bouton (Edit, Cancel, Send) — "Within a button, a label generally conveys what the button does, such as Edit, Cancel, or Send." — labels › Discussion
- *Usage* : dans une liste, il décrit chaque élément, souvent avec un symbole ou une image ; dans une vue, il introduit un contrôle ou décrit une action — "Within many lists, a label can describe each item, often accompanied by a symbol or an image." — labels › Discussion
- *Composant* : un libellé pour une petite quantité de texte non modifiable ; un champ de texte pour du texte à modifier ; une vue de texte pour de grandes quantités — "Use a label to display a small amount of text that people don’t need to edit." — labels › Best practices
- *Typographie* : préférer les polices système — "Prefer system fonts." — labels › Best practices
- *Typographie* : en cas de style ajusté ou de police personnalisée, veiller à la lisibilité — "If you adjust the style of a label or use custom fonts, make sure the text remains legible." — labels › Best practices
- *Couleur* : employer les couleurs de libellé système pour communiquer l'importance relative — "Use system-provided label colors to communicate relative importance." — labels › Best practices
- *Sélection* : rendre sélectionnable le texte utile (message d'erreur, position, adresse IP), pour le copier et le coller ailleurs — "Make useful label text selectable." — labels › Best practices

Les quatre couleurs de libellé (tableau recopié, colonne iOS, iPadOS, tvOS, visionOS omise) :
| System color | Example usage | macOS |
|---|---|---|
| Label | Primary information | `AppKit/NSColor/labelColor` |
| Secondary label | A subheading or supplemental text | `AppKit/NSColor/secondaryLabelColor` |
| Tertiary label | Text that describes an unavailable item or behavior | `AppKit/NSColor/tertiaryLabelColor` |
| Quaternary label | Watermark text | `AppKit/NSColor/quaternaryLabelColor` |

**Spécifique macOS**
- Pour afficher du texte non modifiable dans un libellé, utiliser la propriété `isEditable` de `NSTextField` (note développeur ; la fin de la phrase est faite de liens dans la source) — "To display uneditable text in a label, use the" — labels › macOS

**À transposer dans un shell Linux**
- Notifications, réglages, résultats de recherche, infobulles du dock : titre en Label, sous-titre ou détail en Secondary label, élément indisponible en Tertiary label, filigrane en Quaternary label.
- Boutons des panneaux et menus : un libellé qui dit ce que fait le bouton ; police système partout.
- Textes utiles affichés par le shell (chemins, adresses, messages d'erreur des réglages) : sélectionnables et copiables.

### Lists and tables (`lists-and-tables`)
**Rôle** — Présenter des données en une ou plusieurs colonnes de rangées : format de référence pour le texte, la navigation hiérarchique et les données à attributs triables.
**Anatomie et cotes** — non chiffré dans la HIG : ni hauteur de rangée, ni espacement, ni rayon, ni largeur de colonne. Éléments : rangées, colonnes, en-têtes de colonnes ; styles propres à chaque plateforme.

**Règles**
- *Usage* : une liste ou une table peut représenter des données groupées ou hiérarchiques et permettre sélection, ajout, suppression et réorganisation — "A table or list can represent data that’s organized in groups or hierarchies, and it can support user interactions like selecting, adding, deleting, and reordering." — lists-and-tables › Discussion
- *Usage* : préférer une liste ou une table pour le texte ; les réglages d'iOS utilisent ainsi une hiérarchie de listes ; pour des éléments de taille très variable ou un grand nombre d'images, envisager une collection ; les tâches de productivité utilisent une table aux colonnes séparées et triables — "Prefer displaying text in a list or table." — lists-and-tables › Best practices
- *Édition* : permettre l'édition quand cela a du sens ; la seule réorganisation est déjà appréciée — "People appreciate being able to reorder a list, even if they can’t add or remove items." — lists-and-tables › Best practices
- *Sélection* : un retour approprié à la sélection d'une rangée, variable selon qu'elle révèle une vue ou bascule un état — "Provide appropriate feedback when people select a list item." — lists-and-tables › Best practices
- *Sélection* : une table de navigation surligne durablement la rangée sélectionnée — "In general, a table that helps people navigate through a hierarchy persistently highlights the selected row to clarify the path people are taking." — lists-and-tables › Best practices
- *Sélection* : une table d'options ne la surligne que brièvement avant d'afficher une coche — "a table that lists options often highlights a row only briefly before adding an image" — lists-and-tables › Best practices
- *Contenu* : texte des éléments succinct, ce qui limite troncature et retours à la ligne — "Keep item text succinct so row content is comfortable to read." — lists-and-tables › Content
- *Contenu* : en cas de texte abondant, n'afficher que les titres et révéler le contenu dans une vue de détail — "you could list item titles only, letting people choose an item to reveal its content in a detail view" — lists-and-tables › Content
- *Contenu* : une ellipse au milieu du texte tronqué conserve le début et la fin — "Sometimes, an ellipsis in the middle of text can make an item easier to distinguish because it preserves both the beginning and the end of the content." — lists-and-tables › Content
- *En-têtes* : descriptifs dans une table multicolonne (noms ou syntagmes nominaux courts, majuscules de titre, sans ponctuation finale) ; sans en-tête dans une table à une colonne, ajouter un libellé ou un en-tête de contexte — "If you don’t include a column heading in a single-column table view, use a label or a header to help people understand the context." — lists-and-tables › Content
- *Style* : un style de liste ou de table accordé aux données et à la plateforme, et un style de rangée adapté aux informations (petite image en tête puis libellé bref, par exemple) — "Choose a table or list style that coordinates with your data and platform." — lists-and-tables › Style

**Spécifique macOS**
- *Style bordé* : macOS définit un style bordé (bordered) aux arrière-plans de rangées alternés, qui facilite les grandes tables ; envisager aussi des couleurs alternées dans une table multicolonne, surtout large — "macOS defines a bordered style that uses alternating row backgrounds to help make large tables easier to use" — lists-and-tables › Style
- *Tri* : quand cela a de la valeur, un clic sur un en-tête de colonne trie la table ; un clic sur un en-tête déjà trié inverse l'ordre — "If people click the heading of a column that’s already sorted, re-sort the data in the opposite direction." — lists-and-tables › macOS
- *Colonnes* : laisser redimensionner les colonnes, pour se concentrer sur certaines zones ou révéler les données rognées — "Let people resize columns." — lists-and-tables › macOS
- *Hiérarchie* : pour des données hiérarchiques, une vue en plan (outline view), qui ajoute des triangles de divulgation, plutôt qu'une table ; Mail place une table dans une vue scindée — "Use an outline view instead of a table view to present hierarchical data." — lists-and-tables › macOS

**À transposer dans un shell Linux**
- Liste de notifications : rangées à texte succinct (titre, sous-titre), sélection signalée brièvement puis par un marqueur quand il s'agit d'une option, surlignage durable quand la sélection mène à un autre volet.
- Réglages en liste : une colonne avec un libellé ou un en-tête de groupe ; table multicolonne à en-têtes nominaux si plusieurs attributs.
- Troncature dans les listes étroites : ellipse au milieu des noms d'applications, de fenêtres ou de fichiers.
- Souris : clic sur en-tête pour trier, colonnes redimensionnables, rangées alternées dans les tableaux larges.

### Lockups (`lockups`)
**Rôle** — Réunir plusieurs vues distinctes en une seule unité interactive qui s'étend et se contracte au focus ; quatre types : cards, caption buttons, monograms, posters.
**Anatomie et cotes** — non chiffré dans la HIG. Composition : une vue de contenu, un en-tête (header) au-dessus, un pied (footer) en dessous ; les trois se dilatent et se contractent ensemble au focus.

**Règles**
- *Structure* : chaque bloc se compose d'une vue de contenu, d'un en-tête et d'un pied — "Each lockup consists of a content view, a header, and a footer." — lockups › Discussion
- *Disposition* : laisser assez d'espace entre les blocs, car celui qui a le focus s'agrandit et ne doit ni chevaucher ni déplacer ses voisins ; tailles constantes dans une rangée ou un groupe — "Use consistent lockup sizes within a row or group." — lockups › Best practices
- *Cards* : en-tête, pied et vue de contenu présentant notes et avis d'éléments multimédias — "A card combines a header, footer, and content view to present ratings and reviews for media items." — lockups › Cards
- *Caption buttons* : titre et sous-titre sous le bouton, qui contient une image ou du texte — "A caption button can include a title and a subtitle beneath the button." — lockups › Caption buttons
- *Caption buttons* : elles s'inclinent dans le sens du balayement (haut et bas, gauche et droite, les deux en grille) — "When aligned vertically, caption buttons tilt up and down. When aligned horizontally, caption buttons tilt left and right." — lockups › Caption buttons
- *Monograms* : photo circulaire et nom d'une personne ; à défaut d'image, ses initiales — "Each monogram consists of a circular picture of the person and their name." — lockups › Monograms
- *Posters* : image avec titre et sous-titre facultatifs, cachés jusqu'au focus — "Posters consist of an image and an optional title and subtitle, which are hidden until the poster comes into focus." — lockups › Posters

**Spécifique macOS** — non pris en charge sur macOS : composant de tvOS uniquement, donc aucune règle macOS à en tirer.

**À transposer dans un shell Linux**
- Aucune règle macOS à transposer ; à titre d'analogie seulement, une cellule de grille de lanceur (icône et nom) ressemble à une *caption button*.
- Principes transférables : largeurs et hauteurs identiques dans une rangée ou une grille, et assez d'espace pour que l'agrandissement au survol ou au focus ne déplace pas les voisins.
- Ne pas reproduire l'inclinaison au balayement, propre à tvOS.

### Outline views (`outline-views`)
**Rôle** — Présenter des données hiérarchiques dans une liste défilante de cellules organisées en colonnes et rangées, les conteneurs parents portant des triangles de divulgation.
**Anatomie et cotes** — non chiffré dans la HIG. Au moins une colonne de données hiérarchiques principales, des colonnes supplémentaires pour les attributs, des triangles de divulgation sur les conteneurs parents.

**Règles**
- *Structure* : convient au contenu textuel et apparaît souvent du côté d'origine d'une vue scindée, le contenu lié étant de l'autre côté — "Outline views work well to display text-based content" — outline-views › Best practices
- *Structure* : au moins une colonne de données hiérarchiques, les colonnes supplémentaires portant des attributs (taille, date de modification) — "You can add columns, as needed, to display attributes that supplement the primary data" — outline-views › Discussion
- *Structure* : pour des données non hiérarchiques, une table et non une vue en plan — "Use a table instead of an outline view to present data that’s not hierarchical." — outline-views › Best practices
- *Colonnes* : n'exposer la hiérarchie que dans la première colonne — "Expose data hierarchy in the first column only." — outline-views › Best practices
- *Colonnes* : en-têtes descriptifs toujours présents dans une vue multicolonne (noms, majuscules de titre, aucune ponctuation ni deux-points final) — "Always provide column headings in a multi-column outline view." — outline-views › Best practices
- *Tri* : envisager le tri par clic sur un en-tête ; sur la colonne principale, il s'applique à chaque niveau de hiérarchie — "If people click the primary column heading, sorting occurs at each hierarchy level." — outline-views › Best practices
- *Déploiement* : faciliter l'ouverture et la fermeture des conteneurs imbriqués — "Make it easy for people to expand or collapse nested containers." — outline-views › Best practices
- *Mémoire* : conserver les choix de déploiement d'une session à l'autre — "Retain people’s expansion choices." — outline-views › Best practices
- *Édition* : dans une cellule éditable, un clic simple édite ; le double-clic peut faire autre chose — "In an editable outline view cell, people expect to be able to single-click a cell to edit its contents." — outline-views › Best practices
- *Recherche* : envisager un champ de recherche dans une vue en plan longue — "Consider offering a search field to help people find values quickly in a lengthy outline view." — outline-views › Best practices

**Spécifique macOS**
- Composant propre à macOS ; le Finder l'offre pour parcourir le système de fichiers — "Finder windows offer an outline view for navigating the file system." — outline-views › Discussion
- *Souris* : le clic sur un dossier du Finder n'ouvre que ce dossier ; l'Option-clic sur un triangle ouvre tous ses sous-dossiers — "Option-clicking the disclosure triangle expands all of its subfolders" — outline-views › Best practices
- *Souris* : exemple d'édition, un clic simple sur un nom de fichier l'édite, un double-clic l'ouvre — "might let people single-click a file’s name to edit it, but double-click a file’s name to open the file" — outline-views › Best practices

**À transposer dans un shell Linux**
- Réglages en arborescence : hiérarchie dans la première colonne seulement, les colonnes suivantes portant des attributs ; en-têtes toujours présents si plusieurs colonnes.
- Mémoire : l'état déployé ou replié des nœuds survit à la fermeture et à la réouverture du panneau.
- Souris : le clic sur un triangle n'ouvre que ce nœud ; le clic modifié (Alt) ouvre toute la branche ; liste longue : champ de recherche.

### Split views (`split-views`)
**Rôle** — Gérer plusieurs volets de contenu adjacents (tables, collections, images, vues personnalisées), le plus souvent pour montrer plusieurs niveaux de hiérarchie à la fois et naviguer entre eux.
**Anatomie et cotes** — une seule cote : le séparateur fin. Volets : primaire (côté d'origine), secondaire, tertiaire facultatif ; séparateurs redimensionnables. Bornes minimale et maximale des volets : non chiffré dans la HIG.
- *Séparateur* : le séparateur fin (thin divider) mesure un point de largeur, laissant un maximum de place au contenu — "The thin divider measures one point in width, giving you maximum space for content while remaining easy for people to use." — split-views › macOS

**Règles**
- *Structure* : une vue scindée montre plusieurs niveaux de hiérarchie à la fois et permet de naviguer entre eux — "Typically, you use a split view to show multiple levels of your app’s hierarchy at once and support navigation between them." — split-views › Discussion
- *Structure* : sélectionner un élément du volet primaire affiche son contenu dans le volet secondaire ; un volet tertiaire est possible — "a split view can display a tertiary pane if items in the secondary pane contain additional content" — split-views › Discussion
- *Usage* : souvent une barre latérale de navigation, le volet d'origine listant les éléments de premier niveau, les autres volets les collections enfants et le détail — "the secondary and optional tertiary panes can present child collections and item details" — split-views › Discussion
- *Navigation* : surligner durablement la sélection courante dans chaque volet qui mène à la vue de détail — "To support navigation, persistently highlight the current selection in each pane that leads to the detail view." — split-views › Best practices
- *Glisser-déposer* : envisager de laisser déplacer du contenu d'un volet à l'autre — "Consider letting people drag and drop content between panes." — split-views › Best practices

**Spécifique macOS**
- *Disposition* : volets verticaux, horizontaux ou les deux ; séparateurs pouvant être déplaçables pour redimensionner — "In macOS, you can arrange the panes of a split view vertically, horizontally, or both." — split-views › macOS
- *Redimensionnement* : fixer des bornes minimales et maximales raisonnables pour les volets, car un volet trop petit fait disparaître le séparateur en apparence — "Set reasonable defaults for minimum and maximum pane sizes." — split-views › macOS
- *Masquage* : envisager de laisser masquer un volet (Keynote : navigateur et notes du présentateur) — "Consider letting people hide a pane when it makes sense." — split-views › macOS
- *Masquage* : offrir plusieurs moyens de rappeler un volet masqué, par exemple un bouton de barre d'outils ou une commande de menu avec raccourci clavier — "Provide multiple ways to reveal hidden panes." — split-views › macOS
- *Style* : préférer le séparateur fin ; les styles épais se justifient d'un besoin précis (rangées à forts éléments linéaires des deux côtés) — "Prefer the thin divider style." — split-views › macOS

**À transposer dans un shell Linux**
- Écran de réglages : volet primaire (liste des catégories) et volet de détail ; la catégorie courante reste surlignée.
- Bornes de redimensionnement : minimum et maximum par volet, pour que le séparateur reste visible.
- Volet masquable, avec plusieurs moyens de le rappeler (bouton, commande, raccourci clavier) ; séparateur fin d'un point.

### Tab views (`tab-views`)
**Rôle** — Présenter plusieurs volets de contenu mutuellement exclusifs dans une même zone, entre lesquels on bascule par un contrôle à onglets.
**Anatomie et cotes** — le contrôle à onglets se place sur le bord supérieur de la zone de contenu et peut être masqué ; limiter à six onglets. Aucune hauteur, largeur ni espacement n'est chiffré dans la HIG.
- *Position* : le contrôle à onglets (tabbed control) apparaît sur le bord supérieur de la zone de contenu — "The tabbed control appears on the top edge of the content area." — tab-views › Anatomy
- *Contrôle masqué* : la zone de contenu peut alors être sans bordure, avec bezel, ou bordée d'une ligne — "When you hide the tabbed control, the content area can be borderless, bezeled, or bordered with a line." — tab-views › Anatomy
- *Marges* : insérer la vue à onglets en laissant une marge de corps de fenêtre tout autour — "In general, inset a tab view by leaving a margin of window-body area on all sides of a tab view." — tab-views › Anatomy

**Règles**
- *Contenu* : employer une vue à onglets pour des zones de contenu étroitement liées — "Use a tab view to present closely related areas of content." — tab-views › Best practices
- *Contenu* : les contrôles d'un volet n'affectent que le contenu de ce volet, les volets étant auto-suffisants — "Make sure the controls within a pane affect content only in the same pane." — tab-views › Best practices
- *Libellés* : un libellé par onglet, décrivant le contenu de son volet — "Provide a label for each tab that describes the contents of its pane." — tab-views › Best practices
- *Libellés* : en général des noms ou syntagmes nominaux courts, en majuscules de titre (title-style) ; un verbe peut convenir selon le contexte — "In general, use nouns or short noun phrases for tab labels." — tab-views › Best practices
- *Choix du contrôle* : ne pas utiliser un bouton local (pop-up button) pour changer d'onglet — "Avoid using a pop-up button to switch between tabs." — tab-views › Best practices
- *Choix du contrôle* : un onglet demande un seul clic, un bouton local deux — "A tabbed control is efficient because it requires a single click or tap to make a selection, whereas a pop-up button requires two." — tab-views › Best practices
- *Choix du contrôle* : le bouton local reste raisonnable si les volets sont trop nombreux pour des onglets — "a pop-up button can be a reasonable alternative in cases where there are too many panes of content to reasonably display with tabs" — tab-views › Best practices
- *Nombre* : éviter plus de six onglets, qui submergent et posent des problèmes de disposition ; si six onglets ou plus sont nécessaires, envisager une autre interface, par exemple chaque onglet en option de vue d'un menu local — "Avoid providing more than six tabs in a tab view." — tab-views › Best practices

**Spécifique macOS** — composant pris en charge par macOS, absent d'iOS, iPadOS, tvOS et visionOS ; la page ne comporte pas de bloc macOS séparé, les règles ci-dessus sont donc celles de macOS.
- Masquer le contrôle à onglets convient à une app qui change de volet par programme — "You can choose to hide the control, which is appropriate for an app that switches between panes programmatically." — tab-views › Anatomy

**À transposer dans un shell Linux**
- Panneaux à onglets (réglages, centre de notifications) : au plus six onglets, libellés nominaux courts en majuscules de titre, chaque volet autonome ; au-delà, un menu local.
- Ne pas remplacer les onglets par un menu déroulant : un onglet demande un clic et montre tous les choix.
- Marge de corps de fenêtre autour de la vue à onglets, contrôle en haut ; un volet changé par programme n'exige pas de contrôle visible.

### Valeurs de référence
Rares valeurs et limites de ces pages ; tout le reste (hauteurs de rangée, tailles de cellule, rayons, espacements) est non chiffré dans la HIG.
| Propriété | Valeur | Source (slug › section) |
|---|---|---|
| Séparateur de vue scindée à privilégier | fin (thin divider), large d'un point | split-views › macOS |
| Disposition des volets d'une vue scindée (macOS) | verticale, horizontale ou les deux | split-views › macOS |
| Nombre d'onglets | éviter plus de six ; dès six ou plus, envisager une autre interface | tab-views › Best practices |
| Position du contrôle à onglets | bord supérieur de la zone de contenu | tab-views › Anatomy |
| Boutons de divulgation par vue | au plus un | disclosure-controls › Disclosure buttons |
| Triangle de divulgation | vers l'intérieur depuis le bord d'origine : contenu caché ; vers le bas : visible | disclosure-controls › Disclosure triangles |
| Bouton de divulgation | vers le bas : contenu caché ; vers le haut : visible | disclosure-controls › Disclosure buttons |
| Couleurs de libellé système | 4 : Label, Secondary label, Tertiary label, Quaternary label | labels › Best practices |
| Dispositions par défaut des collections | rangée horizontale ou grille | collections › Best practices |
| Titre d'une boîte dans un volet de réglages | majuscules de phrase, terminé par deux-points | boxes › Content |
| Titre d'une boîte (macOS) | affiché au-dessus de la boîte | boxes › macOS |
| Types de lockups | 4 : cards, caption buttons, monograms, posters | lockups › Discussion |
| Composants propres à macOS | column views, outline views | column-views › Platform considerations |

## Seconde partie — hyprdesk aujourd'hui, écarts et actions

### Ce que hyprdesk fait aujourd'hui

| Surface | Ce qui existe | Valeurs clés | Preuves |
|---|---|---|---|
| Écran Réglages — vue scindée | Tiroir latéral en verre : en-tête pleine largeur, puis navigation à gauche et détail à droite, sans séparateur | Tiroir 936 px (pleine largeur sous 1200 px d'écran), marge au bord 12 px, rayon 24, marge interne 24 ; navigation 166 px (46 px sous 640 px), écart de 24 px ; filet de 1 px sous l'en-tête | `panel.rs:126,130` ; `panel.rs:639-645` ; `panel.rs:893-900` ; `panel.rs:1033-1043` |
| Écran Réglages — en-tête | Sur-titre, titre « Réglages », contexte (écran · espace), puis champ de recherche, Visite guidée, Assistant, interrupteur « Configuration avancée » | HYPRDESK 11, Réglages 27, contexte 12 ; recherche large de 230 px ; sous 880 px, la ligne des actions passe sous le titre | `panel.rs:918-993` |
| Navigation latérale | 37 paquets en 10 groupes, défilants ; le paquet choisi déploie ses onglets en sous-entrées ; fiole cyan sur l'avancé ; en recherche, les paquets sans correspondance s'effacent, les autres portent un compte | Titres de groupes en capitales 12 ; entrées : icône 18 + libellé 13 medium, marges 9/12, rayon 11 ; paquet courant : fond accent 16 %, icône accent ; sous-entrées : retrait 40, texte 12, onglet courant en accent seul | `plugins/mod.rs:72-175` ; `panel.rs:724-814` ; `panel.rs:818-892` |
| Onglets des paquets | 19 paquets ont de 1 à 6 onglets ; tiroir large : dans la navigation ; tiroir étroit : pastilles au-dessus du contenu | Pastilles : marges 6/14, rayon 999, retour à la ligne ; actif : fond accent 28 %, bord accent 70 % | `plugins/mod.rs:363-386` ; `panel.rs:648-704` |
| Pages de réglages | Colonne de sections : titre, champs « libellé au-dessus du contrôle », aide ; listes d'éléments en cartes (titre, ↑ ↓ ×, lignes libellé 260 px + contrôle) ; contenu avancé sous interrupteur global, en cadre cyan ou sous pastille « Avancé » | Titre de section 16 semibold accent ; libellé 14 ; aide 12 ; écart libellé-contrôle 6 ; cartes : rayon 10, marge 12, bord accent 35 % ; cadre avancé : rayon 10, cyan | `panel.rs:1093-1117,1134-1141` ; `hyprlists.rs:356-430` ; `hyprset.rs:536-561` ; `plugins/mod.rs:225-248,316-337,441-447,630-653` |
| Recherche de réglages | Résultats groupés par paquet ; rangée « libellé surligné + Paquet › Onglet », ou bloc de contexte ; sur la page ouverte : bandeau, « Résultats », « Effacer » | 120 résultats, 40 blocs de 12 lignes au plus ; bandeau : fond accent 12 %, rayon 10 | `panel.rs:1205-1249` ; `panel.rs:1252-1423` |
| Bulles de notification | Une carte par notification : application, ✕, titre, corps, actions ; un clic lance l'action par défaut | 380 × 104 px fixes, rayon 14, marges 10/14, rognée ; application 11 et corps 12 en secondaire, titre 14 semibold sur une ligne, corps de 2 lignes, 3 actions au plus ; 4 bulles visibles ; bord accent si critique | `toasts.rs:28-33` ; `toasts.rs:488-607` ; `toasts.rs:648-666` |
| Historique de notifications | Trois rendus : carte de la barre ; carte « Notifications » du centre de contrôle Continuité ; volet droit de Panorama | Carte de la barre : 420 px de haut, 30 éléments en texte brut, tous de la même couleur, corps entiers, sans effacement. Continuité : « Effacer l'historique », 12 éléments en encarts, titre 42 et détail 74 caractères, deux couleurs. Panorama : « Tout effacer », 30 cartes teintées | `plugins/bar/popover.rs:486,1212` ; `toasts.rs:609-646` ; `plugins/bar/popover/continuity.rs:1079-1145` ; `plugins/panorama/surface/panes.rs:1619-1725` ; `src/notifications.rs:48-65` |
| Lanceur — feuille en plein bureau | Champ pastille, bandeau « Suggestions », pastilles de catégories, en-tête « Toutes les applications / Résultats », grille de tuiles, consigne de clavier | Tuile 128 px, icône 60, nom 13 coupé à 26 caractères, pas 6 ; colonnes selon réglage (7 en session) | `plugins/launcher/grid.rs:58-59` ; `grid.rs:256-305` ; `grid.rs:318-510` |
| Lanceur — carte centrée, fichiers, presse-papiers | Suggestions en tuiles de 76 px, puis rangées ; fichiers : puces de nature et de catégorie toujours visibles, rangées « icône, nom, taille · dossier » ; presse-papiers : icône + aperçu | Rangées de 46 à 48 px, icône 28, nom 15, indicateur ↵ ; 8 résultats visibles ; dossier parent rogné par le début, avec « … » | `grid.rs:513-590` ; `search.rs:32-33` ; `search.rs:1181-1362` ; `search.rs:1478-1541` |
| Aperçu des espaces de travail | Titre, grille de cartes (une par espace), minimaps des fenêtres, aides au clavier, « Nouvel espace » | Cartes identiques plafonnées à 620 × 380, pas 18, marge 40, bandeau titre 72, aides 64 ; titre de fenêtre 10 rogné à 22 caractères | `overview.rs:25-30` ; `overview.rs:228-262` ; `overview.rs:646-740` ; `overview.rs:850-1000` |
| Jetons de disposition | Rayons, surfaces, états de liste, surface de contenu, estompe de défilement, placement des widgets du bureau | Rayons 10 (commandes) et 20 (cartes), panneau 24, encart 12 ; base 4 px, groupes 8–12, sections 20–24 ; widgets du bureau écartés de 16 px | `design.rs:5-6,139-186` ; `tokens.rs:425-536` ; `desk_layout.rs:22,47-93` ; `DESIGN.md:9` |

### Écarts

| # | Composant | Nature | Force | Règle HIG (slug › section) | Réel dans hyprdesk (preuve) | Action proposée | Effort | Confiance |
|---|---|---|---|---|---|---|---|---|
| LAY-01 | Disclosure controls | ABSENT | Impératif (« Use ») ; composant introuvable | "Use a disclosure control to hide details until they’re relevant." — disclosure-controls › Best practices | Aucune section de réglage ne se replie. Tout ce qui est avancé dépend d'un seul interrupteur global, « Configuration avancée » (`panel.rs:950-954`), qui montre partout à la fois les onglets avancés (`plugins/mod.rs:350-356`), les sections encadrées (`plugins/mod.rs:441-447`) et les groupes d'options brutes (`plugins/mod.rs:652`). Allumé, il allonge d'autant chaque page. Recherché dans `src/ui` (chevron, disclosure, replier, déplier) : seulement la mire des couleurs (`plugins/colors.rs:821-825`), le détail de l'assistante (`chat.rs:1256-1263`) et la demande polkit (`polkit.rs:367`), hors réglages. | Pour chaque section `adv(…)`, remplacer le cadre permanent par un triangle de divulgation intitulé, replié par défaut, qui déploie dans la page. | M | haute |
| LAY-02 | Lists and tables — historique | ABSENT | Composant et comportement introuvables | "A table or list can represent data that’s organized in groups or hierarchies" — lists-and-tables › Discussion | Les trois rendus de l'historique sont des listes à plat, sans groupe par application ni heure : `toasts.rs:625-644`, `plugins/bar/popover/continuity.rs:1095-1125`, `plugins/panorama/surface/panes.rs:1682-1704`. Le modèle ne porte aucune date (`src/notifications.rs:48-65`). | Regrouper l'historique par application : une boîte par application avec son en-tête, comme les résultats de recherche regroupés par paquet (`panel.rs:1299-1302`). | M | haute |
| LAY-03 | Labels — carte Notifications de la barre | ÉCART | Impératif (« Use »), tableau des quatre couleurs | "Use system-provided label colors to communicate relative importance." — labels › Best practices | Dans la carte Notifications de la barre, application (11), titre (14) et corps (12) prennent la même couleur (`toasts.rs:610`, `toasts.rs:628-640`) : la hiérarchie ne repose que sur la taille. La bulle (`toasts.rs:516,547`), le centre de contrôle Continuité (`continuity.rs:1112-1113`) et Panorama (`panes.rs:1684`) utilisent, eux, une couleur secondaire. | Mettre l'application et le corps de cette carte dans la couleur secondaire du thème, comme la bulle. | S | haute |
| LAY-04 | Lists and tables — corps des notifications | ÉCART | Impératif (« Keep ») | "Keep item text succinct so row content is comfortable to read." — lists-and-tables › Content | La carte Notifications de la barre affiche le corps entier de chacune des 30 notifications (`toasts.rs:636-640`) dans 420 px de haut (`popover.rs:486`) ; Panorama fait de même (`panes.rs:1692`). La bulle limite à deux lignes (`toasts.rs:540-541`) et Continuité à 42 et 74 caractères (`continuity.rs:1096-1101`). | Borner titre et corps dans cette carte comme le fait Continuité (42 et 74 caractères). | S | moyenne |
| LAY-05 | Labels — textes utiles des réglages | ÉCART | Impératif (« Make ») | "Make useful label text selectable." — labels › Best practices | Chemins, avertissements et erreurs sont rendus en texte non sélectionnable, sans copie : résultat de la remise par défaut, avec le chemin de la copie (`panel.rs:593-609`, texte construit en `panel.rs:1457-1467`) ; « surchargé dans hyprdesk.lua — sans effet ici » (`panel.rs:1108-1115`) ; lignes restées dans hyprland.lua (`hyprlists.rs:561-570`, `hyprset.rs:653-658`) ; chemin du plugin et URL d'installation (`plugins/desktop.rs:222-228`). `text_editor` ne sert qu'aux citations de l'assistante (`chat.rs:1315`) et à un champ de style (`plugins/assistant/services.rs:754`) ; seul « À propos » offre « Copier les informations » (`plugins/about.rs:322-332`). | Rendre ces textes sélectionnables en lecture seule (`text_editor`, comme `chat.rs:1315`). | M | haute |
| LAY-06 | Boxes — contenu avancé | PISTE | Recommandation (« Consider », « Prefer ») | "Consider using padding and alignment to communicate additional grouping within a box." — boxes › Best practices | Boîtes emboîtées : le cadre cyan enveloppe des cartes qui ont déjà leur bord (`hyprlists.rs:644-666` autour de `hyprlists.rs:419-429`). Boîtes de la taille de la page, sans titre : `self.adv(self.hyprset_groups(key))` encadre tous les groupes d'une clé (`hyprset.rs:544`), `adv_frame(app.hyprset_groups(k))` de même (`plugins/mod.rs:652`), contre "Prefer keeping a box relatively small in comparison with its containing view." | Retirer le cadre autour des listes de cartes et des groupes entiers ; marquer le caractère avancé par la fiole au titre de section (`plugins/mod.rs:316-337`). | S | haute |
| LAY-07 | Split views — onglet courant | PISTE | Impératif de bonnes pratiques (« persistently highlight ») | "To support navigation, persistently highlight the current selection in each pane that leads to the detail view." — split-views › Best practices | Dans le tiroir large, les onglets du paquet choisi sont des sous-entrées de la navigation (`panel.rs:851-890`). L'onglet courant n'est marqué que par la couleur accent et la graisse (`panel.rs:884`), sans fond : le fond n'apparaît qu'au survol (`panel.rs:876-883`). Le paquet courant, lui, est surligné (`panel.rs:800-801`). | Donner à l'onglet courant le même voile de sélection qu'au paquet courant. | S | moyenne |
| LAY-08 | Labels — texte secondaire | PISTE | Impératif (« Use »), tableau des quatre couleurs | "Use system-provided label colors to communicate relative importance." — labels › Best practices | Le rôle « texte secondaire » est redéfini sur place par une transparence du texte : 0,85 (`panel.rs:1103`), 0,65 (`panel.rs:972`), 0,6 (`panel.rs:1016`), 0,35 (`panel.rs:729`), 0,65 (`toasts.rs:492`), 0,62 (`overview.rs:653-656`), 0,6 (`overview.rs:862-865`). Ailleurs, `ink().dim` vient du thème avec 4,5:1 garantis (`panel.rs:1121-1132`, `hyprset.rs:523-529`, `search.rs:1123-1124`). Les aides de la page Bureau restent en couleur pleine (`plugins/desktop.rs:202,222-228`). | Faire passer tout texte secondaire (aides, états, comptes) par la couleur secondaire unique du thème. | S | haute |
| LAY-09 | Collections — tuiles de la grille | PISTE | Principe transposé depuis tvOS | "Use consistent lockup sizes within a row or group." — lockups › Best practices | La tuile a une largeur fixe de 128 px mais pas de hauteur (`grid.rs:274-293`). Le nom est coupé à 26 caractères puis revient à la ligne (iced 0.14 : `Wrapping::Word` par défaut, `iced_core-0.14.0/src/text.rs:186-188`), sur une ou deux lignes. Les rangées n'alignent pas les tuiles en hauteur (`grid.rs:419-432`). | Donner à la tuile une hauteur fixe qui tient deux lignes de nom. | S | moyenne |
| LAY-10 | Lists and tables — textes rognés | PISTE | Recommandation (« Sometimes ») | "Sometimes, an ellipsis in the middle of text can make an item easier to distinguish because it preserves both the beginning and the end of the content." — lists-and-tables › Content | L'ellipse tombe toujours en fin de texte : titres de fenêtres de l'aperçu à 22 ou 26 caractères (`overview.rs:806,981,1082-1088`), noms d'applications à 26 ou 12 (`grid.rs:268-273`), noms de périphériques audio à 28 (`popover.rs:1817`). Le titre d'une bulle est rogné sans ellipse (`toasts.rs:536,575`). Seul le dossier parent d'un fichier garde sa fin (`search.rs:1310-1319`). | Rogner au milieu, début et fin conservés, les titres de fenêtres et les noms de périphériques. | S | haute |
| LAY-11 | Collections — choix d'un style visuel | PISTE | Recommandation (« Generally ») | "Generally speaking, collections are ideal for showing image-based content." — collections › Discussion | Les styles visuels se choisissent parmi des boutons radio textuels, sans aperçu : quatre styles de barre (`plugins/bar.rs:443-477`), six styles de sélecteur d'espaces (`plugins/workspaces.rs:239-259`), deux modes de bureau « Opaque / Verre » (`plugins/desktop.rs:234-235`). | Présenter chaque style par une vignette, le nom dessous. | M | moyenne |
| LAY-12 | Labels — boutons à glyphe seul | PISTE | Recommandation (« generally ») | "Within a button, a label generally conveys what the button does, such as Edit, Cancel, or Send." — labels › Discussion | Les commandes d'une carte de liste sont des glyphes seuls, sans libellé ni infobulle : ↑ ↓ × (`hyprlists.rs:376-394`), × des variables (`hyprset.rs:628-631`), × de suppression (`plugins/mod.rs:456-465`), ✕ d'une bulle (`toasts.rs:518-526`). Le bouton de fermeture du tiroir a, lui, une infobulle (`panel.rs:903-915`). | Donner une infobulle à chaque bouton à glyphe seul, sur le modèle de « Fermer · Échap ». | S | haute |
| LAY-13 | Labels — navigation en icônes seules | PISTE | Recommandation (« can ») | "Within many lists, a label can describe each item, often accompanied by a symbol or an image." — labels › Discussion | Sous 640 px de tiroir, la navigation passe à 46 px et n'affiche plus que l'icône : le libellé n'est ajouté que si le tiroir est large (`panel.rs:750-794`, largeur en `panel.rs:895`). Les entrées n'ont pas d'infobulle (`panel.rs:795-813`). La réduction aux icônes est une décision documentée (`DESIGN.md:16`), l'absence d'infobulle n'en fait pas partie. | Ajouter une infobulle (nom du paquet) aux entrées en icônes seules. | S | haute |
| LAY-14 | Split views — volets des réglages | PISTE | Recommandation (« Consider ») | "Consider letting people hide a pane when it makes sense." — split-views › macOS | La largeur de la navigation est fixée par celle du tiroir, 166 ou 46 px (`panel.rs:620-624`, `panel.rs:893-900`). Aucun volet ne se redimensionne ni ne se masque à la demande : ni `pane_grid` ni `resizable` dans `src/ui`. Le tiroir garde 936 px jusqu'à 1200 px d'écran (`panel.rs:639-645`). | Offrir une commande et son raccourci pour replier la navigation en icônes dans le tiroir large. | M | moyenne |
| LAY-15 | Collections — cartes de l'aperçu | PISTE | Recommandation (« Consider ») | "Consider using animations to provide feedback when people insert, delete, or reorder items." — collections › Best practices | Seul le fondu d'ouverture et de fermeture de l'écran anime l'aperçu : `animating` ne teste que la visibilité (`overview.rs:103-105`) et l'état ne garde rien par carte (`overview.rs:108-120`). « Nouvel espace » (`overview.rs:738`) pose sa carte sans transition. | Animer l'apparition d'une carte. | S | moyenne |
| LAY-16 | Labels — police | IDENTITÉ | Recommandation (« Prefer ») | "Prefer system fonts." — labels › Best practices | Tous les libellés prennent la police du thème, Mindlog par défaut (`panel.rs:752-756` ; `DESIGN.md:7`). | À signaler, sans correction. | — | haute |

### Ce qui est déjà conforme

- tab-views › Best practices, "Avoid providing more than six tabs in a tab view." | 19 paquets à onglets, de 1 à 6 ; maximum Layouts (`plugins/layouts.rs:30-37`) et Clavier et souris (`plugins/input.rs:26-33`).
- tab-views › Best practices, "Provide a label for each tab that describes the contents of its pane." et "In general, use nouns or short noun phrases for tab labels." | « Général, Modules, Couleurs » (`plugins/bar.rs:43-47`), « Clavier, Souris, Pavé tactile… » (`plugins/input.rs:26-33`).
- tab-views › Best practices, "Avoid using a pop-up button to switch between tabs." | Aucun `pick_list` ne change d'onglet ; les 33 des paquets choisissent une valeur (par exemple `plugins/icons.rs:246`, `plugins/workspaces.rs:164`).
- tab-views › Anatomy, "The tabbed control appears on the top edge of the content area." | Tiroir étroit : pastilles d'onglets au-dessus du contenu défilant (`panel.rs:1024-1029`).
- tab-views › Best practices, "Make sure the controls within a pane affect content only in the same pane." | « Par défaut… » agit sur la seule page courante (`panel.rs:536-553`).
- split-views › Best practices, "persistently highlight the current selection" | Paquet courant : fond accent 16 %, icône accent, texte plein (`panel.rs:733-741`, `panel.rs:800-809`) ; écran choisi du panneau Écrans : bordure accent de 2 px (`plugins/screens.rs:323-332`) au-dessus de ses réglages (`plugins/screens.rs:588`).
- split-views › macOS, "Prefer the thin divider style." | Aucun séparateur épais : 24 px de vide entre navigation et détail (`panel.rs:1037`), un filet de 1 px sous l'en-tête (`panel.rs:1036`).
- split-views › macOS, "Set reasonable defaults for minimum and maximum pane sizes." | Largeur de navigation bornée par construction, 166 ou 46 px (`panel.rs:620-624`, `panel.rs:895`).
- disclosure-controls › Best practices, "with more advanced functionality hidden by default" | `advanced_settings: false` par défaut (`src/config/model.rs:1735`) ; onglets, paquets et sections avancés masqués tant qu'il est éteint (`plugins/mod.rs:340-356`, `plugins/mod.rs:441-447`).
- disclosure-controls › Disclosure buttons, "Use no more than one disclosure button in a single view." | Aucune vue des surfaces auditées n'en compte.
- lists-and-tables › Best practices, "People appreciate being able to reorder a list, even if they can’t add or remove items." | ↑ ↓ sur chaque carte de règle et de geste (`hyprlists.rs:374-388`) ; modules de barre en glisser-déposer (`DESIGN.md:19`).
- lists-and-tables › Best practices, "Provide appropriate feedback when people select a list item." | Voile de sélection et anneau de focus sur les résultats du lanceur (`tokens.rs:494-536` ; `grid.rs:581-589` ; `search.rs:1349-1357`).
- lists-and-tables › Content, "If you don’t include a column heading in a single-column table view, use a label or a header to help people understand the context." | Résultats de recherche groupés sous le nom du paquet (`panel.rs:1299-1302`) ; grille précédée de « Toutes les applications » ou « Résultats » (`grid.rs:437-444`).
- lists-and-tables › Best practices, "Prefer displaying text in a list or table." | Résultats, réglages et notifications en listes, jamais en grille (`search.rs:1188` ; `panel.rs:1329-1344` ; `toasts.rs:625-644`).
- collections › Best practices, "Use the standard row or grid layout whenever possible." | Grille de tuiles en rangées (`grid.rs:419-436`) ; aperçu en grille quasi carrée de cartes identiques (`overview.rs:234-240`).
- collections › Best practices, "Use adequate padding around images to keep focus or hover effects easy to see and prevent content from overlapping." | Tuile de 128 px pour une icône de 60, marges 12/10/6/6, voile de sélection tracé dans la tuile (`grid.rs:58-59`, `grid.rs:288-299`).
- collections › Best practices, "Make it easy to choose an item." | La tuile entière est le bouton (`grid.rs:274-300`).
- collections, retour au déplacement | Aperçu : cible de dépôt en bordure d'accent, fenêtre suivie en pastille (`DESIGN.md:21`).
- boxes › macOS, "By default, macOS displays a box’s title above it." | Encadré « Suggestions » du lanceur : titre au-dessus des tuiles (`grid.rs:341-353`).
- boxes › Best practices, "Prefer keeping a box relatively small in comparison with its containing view." | Même encadré : une seule rangée de tuiles (`grid.rs:341-353`).
- boxes, transposition « un seul niveau de cadre » | Bulle de notification : un seul cadre (`toasts.rs:571-598`).
- labels › Discussion, "Within a button, a label generally conveys what the button does" | Par défaut…, Confirmer, Annuler, Résultats, Effacer, Visite guidée (`panel.rs:542`, `panel.rs:578-585`, `panel.rs:935`, `panel.rs:1220`, `panel.rs:1228`).
- labels › Best practices, "Use a label to display a small amount of text that people don’t need to edit." | Textes fixes en `text`, saisies en `text_input` (`hyprset.rs:320`, `hyprlists.rs:517`).
- Sans objet | Column views et outline views : aucune hiérarchie profonde à parcourir ; la navigation des réglages tient lieu de vue en plan à deux niveaux, le paquet choisi déployant ses onglets (`panel.rs:851-890`). Lockups n'a de règle que pour tvOS : seul son principe de taille constante est repris (LAY-09).

### À vérifier sur un rendu

- Libellés longs de la navigation (166 px) : « Configuration de Hyprland », « Démarrage automatique », « Index de fichiers », « Clavier et souris » (`plugins/mod.rs:72-110`). En configuration avancée, la fiole et le compteur retirent encore de la place au libellé (`panel.rs:744-777`) ; le texte revient à la ligne par défaut. Un mot isolé dépasse-t-il ? La hauteur des entrées varie-t-elle ?
- Tiroir étroit : icône de 18 px et fiole de 12 px, espacées de 10 px, dans 46 px moins 24 px de marges internes (`panel.rs:736-748`, `panel.rs:797`). La fiole déborde-t-elle ?
- Bulle de notification : hauteur fixe de 104 px avec rognage (`toasts.rs:32`, `toasts.rs:571-575`). Avec un titre long, un corps de deux lignes et trois actions (`toasts.rs:539-570`), les actions restent-elles visibles ?
- Rangées de résultats : hauteur fixe de 46 à 48 px (`search.rs:33`, `search.rs:1348` ; `grid.rs:580`) alors que le texte revient à la ligne par défaut. Un nom de fichier ou d'application long passe-t-il sur la ligne voisine ou sur le détail ?
- Densité des pages : colonne unique, libellé au-dessus du contrôle (`panel.rs:1116`), sections espacées de 12 à 18 px. La page « Barre › Général » (`plugins/bar.rs:126-1025`) se lit-elle d'un seul défilement ?
- Place du libellé : au-dessus du contrôle dans `field` (`panel.rs:1116`), à gauche sur 260 px dans les cartes et l'éditeur de variables (`hyprlists.rs:406-416`, `hyprlists.rs:766-770` ; `hyprset.rs:615-632`). La cohabitation se lit-elle dans une même page ?
- Deux volets sans séparateur ni fond distinct (`panel.rs:1037`) sur le verre du tiroir (`panel.rs:1048-1076`) : navigation et détail se distinguent-ils ?
- Cartes de règles au bord d'accent à 35 % (`hyprlists.rs:418-428`) alors que l'accent est réservé à la sélection (`DESIGN.md:5`) : une liste de cartes ne se prend-elle pas pour une sélection ?
- Tuiles de la grille : hauteur variable selon une ou deux lignes de nom (LAY-09), à voir sur une grille réelle.
- Carte Notifications de la barre : 420 px de haut (`popover.rs:486`) pour 30 notifications au corps entier ; part prise par une seule notification longue.
