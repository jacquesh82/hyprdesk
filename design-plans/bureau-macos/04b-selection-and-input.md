# Composants : sélection et saisie — macOS d'après la HIG, face à hyprdesk

> Proposition de design, rien d'implémenté. Plan d'ensemble et priorités : `plan-d-action.md` ; méthode et périmètre : `README.md`.

> Référentiel : Human Interface Guidelines d'Apple, export Markdown officiel relevé le 23 novembre 2026. Code : commit `00f6d69` (arbre de travail du 23 novembre 2026, avec des modifications non commitées).

## Première partie — Spécification d'après la HIG

### Résumé

- Périmètre : la page d'index `selection-and-input` ne contient que deux listes identiques de définitions, pour onze composants : puits de couleur (color well), combo box, vue de saisie de chiffres (digit entry view), puits d'image (image well), sélecteur (picker), contrôle segmenté (segmented control), curseur (slider), stepper, champ texte (text field), interrupteur (toggle) et clavier virtuel (virtual keyboard) — "A picker displays one or more scrollable lists of distinct values that people can choose from." — selection-and-input › Discussion
- Exclusifs à macOS : combo box et puits d'image — "Not supported in iOS, iPadOS, tvOS, visionOS, or watchOS." — combo-boxes › Platform considerations ; absents de macOS : la vue de saisie de chiffres et le clavier virtuel — "Not supported in macOS." — virtual-keyboards › Platform considerations
- Interrupteurs : trois styles sur macOS, à loger dans le corps de la fenêtre — "Use switches, checkboxes, and radio buttons in the window body, not the window frame." — toggles › Platform considerations › macOS
- Tailles de contrôle : la seule nommée dans ces pages est celle du switch, mini et regular ; aucune hauteur en points — "The height of a mini switch is similar to the height of buttons and other controls" — toggles › Platform considerations › macOS › Switches
- Contrôle segmenté : environ cinq à sept segments au plus — "Aim for no more than about five to seven segments in a wide interface" — segmented-controls › Best practices
- Curseur : minimum du côté d'en-tête, maximum du côté terminal — "with minimum values on the leading side and maximum values on the trailing side (for horizontal sliders)" — sliders › Best practices
- Champ texte : champ sécurisé pour toute donnée sensible — "Always use a secure text field when your app asks for sensitive data, such as a password." — text-fields › Best practices
- Absences : aucune de ces douze pages ne donne de hauteur, rayon, durée ou couleur.

### Combo boxes (`combo-boxes`)

**Rôle** : une combo box réunit, dans un seul contrôle, un champ texte et un bouton déroulant (pull-down button).
**Anatomie et cotes**
- Deux usages : saisir une valeur libre, ou cliquer le bouton pour choisir dans une liste prédéfinie — "People can enter a custom value into the field or click the button to choose from a list of predefined values." — combo-boxes › Discussion
- Libellé introductif, en capitales de titre, terminé par deux-points — "Generally, use title-style capitalization for labels and end them with a colon." — combo-boxes › Best practices
- Largeur : aucun élément de liste ne dépasse le champ — "Make sure list items aren’t wider than the text field." — combo-boxes › Best practices ; motif — "If an item is too wide, the text field might truncate it, which is hard for people to read." — combo-boxes › Best practices
- Hauteur, largeur, rayon, taille de police, nombre de lignes visibles, taille de contrôle (mini, small, regular) : non chiffrés dans la HIG.
**Règles**
- **Valeur par défaut** : une valeur significative prise dans la liste — "Populate the field with a meaningful default value from the list." — combo-boxes › Best practices ; le champ peut rester vide, mais mieux vaut une valeur évoquant les choix cachés — "Although the field can be empty by default, it’s best when the default value refers to the hidden choices." — combo-boxes › Best practices ; elle n'a pas à être la première — "The default value doesn’t have to be the first item in the list." — combo-boxes › Best practices
- **Contenu de la liste** : les choix les plus probables — "Provide relevant choices." — combo-boxes › Best practices
- **Valeur libre** : elle n'entre pas dans la liste — "When people enter a custom value, it’s not added to the list of choices." — combo-boxes › Discussion
**Spécifique macOS**
- Composant exclusif à macOS — "Not supported in iOS, iPadOS, tvOS, visionOS, or watchOS." — combo-boxes › Platform considerations
- La page Text fields y renvoie pour associer saisie et liste de choix — "Consider using a combo box if you need to pair text input with a list of choices." — text-fields › Platform considerations › macOS
- Clavier : aucun raccourci (flèches, Échap) n'est décrit. Page liée non couverte ici : Pull-down buttons.
**À transposer dans un shell Linux**
- Écran de réglages : toute valeur à la fois libre et choisissable dans une liste (délai, format, commande) associe champ et bouton déroulant dans un même contrôle.
- Préremplir avec la valeur de la liste qui correspond au réglage courant ; n'ajouter aucune valeur libre à la liste.
- Libellé en capitales de titre, terminé par deux-points ; liste dimensionnée au plus large élément.

### Color wells (`color-wells`)

**Rôle** : un puits de couleur (color well) règle la couleur d'éléments à l'écran (texte, formes, repères).
**Anatomie et cotes**
- Au clic ou au toucher, il ouvre un sélecteur de couleur (color picker) — "A color well displays a color picker when people tap or click it." — color-wells › Discussion
- Sélecteur du système ou interface personnalisée — "This color picker can be the system-provided one or a custom interface that you design." — color-wells › Discussion
- Forme du puits, taille, rayon, bordure : non chiffrés dans la HIG ; le contenu du sélecteur n'est pas décrit.
**Règles**
- **Sélecteur** : préférer celui du système, pour une expérience familière — "Consider the system-provided color picker for a familiar experience." — color-wells › Best practices ; il mémorise des couleurs accessibles depuis toute app — "in addition to letting people save a set of colors they can access from any app" — color-wells › Best practices
**Spécifique macOS**
- Clic : le puits est surligné pour confirmer qu'il est actif — "When people click a color well, it receives a highlight to provide visual confirmation that it’s active." — color-wells › Platform considerations › macOS
- Il ouvre alors le sélecteur — "It then opens a color picker so people can choose a color." — color-wells › Platform considerations › macOS ; puis il affiche la nouvelle couleur — "After they make a selection, the color well updates to show the new color." — color-wells › Platform considerations › macOS
- Glisser-déposer : d'un puits à l'autre, et du sélecteur vers un puits — "Color wells also support drag and drop, so people can drag colors from one color well to another, and from the color picker to a color well." — color-wells › Platform considerations › macOS. Clavier : rien dans la HIG.
**À transposer dans un shell Linux**
- Écran de réglages (thème, couleur d'accent, couleur d'un espace, fond d'écran) : pastille montrant la couleur courante, qui ouvre un sélecteur unique pour tout le shell.
- État actif visible dès le clic ; pastille mise à jour aussitôt après le choix.
- Palette de couleurs enregistrées commune à tous les écrans de réglage ; glisser-déposer de couleur en option.

### Digit entry views (`digit-entry-views`)

- Hors périmètre : la vue de saisie de chiffres (digit entry view), plein écran avec clavier numérique pour un code PIN, n'existe pas sur macOS — "Not supported in iOS, iPadOS, macOS, visionOS, or watchOS." — digit-entry-views › Platform considerations ; seul principe transférable au verrouillage : un champ de code secret affiche des astérisques — "Secure digit fields display asterisks instead of the entered digit onscreen." — digit-entry-views › Best practices

### Image wells (`image-wells`)

**Rôle** : un puits d'image (image well) est une vue d'image (image view) éditable.
**Anatomie et cotes**
- Une fois sélectionné : copier-coller ou supprimer l'image — "After selecting an image well, people can copy and paste its image or delete it." — image-wells › Discussion
- On peut aussi y glisser une nouvelle image sans le sélectionner d'abord — "People can also drag a new image into an image well without selecting it first." — image-wells › Discussion
- Dimensions, rayon, bordure, image par défaut : non chiffrés dans la HIG.
**Règles**
- **Vidage** : revenir à l'image par défaut — "Revert to a default image when necessary." — image-wells › Best practices ; obligatoire si le puits exige une image — "If your image well requires an image, display the default image again if people clear the content of the image well." — image-wells › Best practices
- **Copier-coller** : proposer les commandes standard du menu — "If your image well supports copy and paste, make sure the standard copy and paste menu items are available." — image-wells › Best practices
**Spécifique macOS**
- Composant exclusif à macOS — "Not supported in iOS, iPadOS, tvOS, visionOS, or watchOS." — image-wells › Platform considerations
- Commandes du menu Édition ou raccourcis standard attendus — "People generally expect to choose these menu items — or use the standard keyboard shortcuts — to interact with an image well." — image-wells › Best practices
- Souris : glisser-déposer sans sélection préalable. Page liée non couverte ici : Image views.
**À transposer dans un shell Linux**
- Réglages du bureau (fond d'écran, image d'avatar ou d'espace) : zone de dépôt acceptant un glisser-déposer sans clic préalable, avec retour à l'image par défaut au vidage.
- Raccourcis de copier-coller standard quand la zone est sélectionnée ; aucune cote à reprendre.

### Pickers (`pickers`)

**Rôle** : un sélecteur (picker) affiche une ou plusieurs listes défilantes de valeurs distinctes parmi lesquelles choisir.
**Anatomie et cotes**
- Plusieurs styles selon le type de valeur ; valeurs et ordre dépendent de la langue de l'appareil — "The exact values shown in a picker, and their order, depend on the device language." — pickers › Discussion
- Le sélecteur de date (date picker) offre un calendrier ou une saisie au clavier numérique — "Date pickers specifically offer additional ways to choose values, like selecting a day in a calendar view or entering dates and times using a numeric keypad." — pickers › Discussion
- Liste des minutes : 60 valeurs (0 à 59) par défaut — "By default, a minute list includes 60 values (0 to 59)." — pickers › Best practices
- Hauteur, nombre de rangées visibles, largeur, rayon : non chiffrés dans la HIG.
**Règles**
- **Choix du composant** : liste moyenne à longue — "Consider using a picker to offer medium-to-long lists of items." — pickers › Best practices ; liste courte : un pull-down button, le sélecteur alourdissant — "it may add too much visual weight to a short list of items" — pickers › Best practices ; très grand ensemble : liste ou tableau — "Lists and tables can adjust in height, and tables can include an index" — pickers › Best practices
- **Valeurs** : prévisibles et ordonnées logiquement — "Use predictable and logically ordered values." — pickers › Best practices
- **Présentation** : en contexte, sans changer de vue — "Avoid switching views to show a picker." — pickers › Best practices ; en bas d'une fenêtre ou dans un popover — "A picker typically appears at the bottom of a window or in a popover." — pickers › Best practices
- **Minutes** : intervalle plus grossier permis s'il divise 60 — "You can optionally increase the minute interval as long as it divides evenly into 60." — pickers › Best practices ; exemple — "For example, you might want quarter-hour intervals (0, 15, 30, and 45)." — pickers › Best practices
**Spécifique macOS**
- Deux styles de sélecteur de date : textuel et graphique — "There are two styles of date pickers in macOS: textual and graphical." — pickers › Platform considerations › macOS
- Textuel : espace limité, choix précis — "The textual style is useful when you’re working with limited space and you expect people to make specific date and time selections." — pickers › Platform considerations › macOS
- Graphique — "useful when you want to give people the option of browsing through days in a calendar or selecting a range of dates" — pickers › Platform considerations › macOS ; ou pour un cadran d'horloge. Référence développeur citée sans cote : NSDatePicker (AppKit). Clavier : rien dans la HIG.
**À transposer dans un shell Linux**
- Écran de réglages : choix unique dans une liste moyenne ou longue → sélecteur ; liste très courte → bouton déroulant ; très longue liste → liste défilante avec index.
- Valeurs triées de façon prévisible et localisées ; sélecteur ouvert près du champ, en popover, sans changer d'écran.
- Réglages horaires : intervalle des minutes divisible par 60 (0, 15, 30, 45 pour le quart d'heure).

### Segmented controls (`segmented-controls`)

**Rôle** : un contrôle segmenté (segmented control) est une suite linéaire d'au moins deux segments, chacun fonctionnant comme un bouton.
**Anatomie et cotes**
- Segments généralement de largeur égale — "Within a segmented control, all segments are usually equal in width." — segmented-controls › Discussion
- Contenu : texte ou images — "segments can contain text or images" — segmented-controls › Discussion ; libellés possibles sous chaque segment ou sous le contrôle entier — "Segments can also have text labels beneath them (or beneath the control as a whole)." — segmented-controls › Discussion
- Nombre : environ cinq à sept au plus dans une interface large — "Aim for no more than about five to seven segments in a wide interface" — segmented-controls › Best practices
- Hauteur, largeur, rayon des segments, taille de contrôle (mini, small, regular), apparence du segment sélectionné : non chiffrés dans la HIG.
**Règles**
- **Usage** : choix étroitement liés portant sur un objet, un état ou une vue — "Use a segmented control to provide closely related choices that affect an object, state, or view." — segmented-controls › Best practices
  - Pour regrouper des fonctions ou rendre la sélection lisible — "Consider a segmented control when it’s important to group functions together, or to clearly show their selection state." — segmented-controls › Best practices ; le groupe subsiste quelle que soit la taille de la vue — "segmented controls preserve their grouping regardless of the view size or where they appear" — segmented-controls › Best practices
  - Un seul type par contrôle : sélection ou action, jamais les deux — "Keep control types consistent within a single segmented control." — segmented-controls › Best practices
- **Nombre et taille** : limiter le nombre de segments — "Limit the number of segments in a control." — segmented-controls › Best practices ; taille constante — "In general, keep segment size consistent." — segmented-controls › Best practices ; largeurs d'icône et de titre constantes aussi — "To the extent possible, it’s best to keep icon and title widths consistent too." — segmented-controls › Best practices
- **Contenu** : texte ou images, sans mélange — "Prefer using either text or images — not a mix of both — in a single segmented control." — segmented-controls › Content
  - Taille de contenu semblable partout — "As much as possible, use content with a similar size in each segment." — segmented-controls › Content
  - Noms ou groupes nominaux — "Use nouns or noun phrases for segment labels." — segmented-controls › Content ; pas de texte introductif si les segments portent des libellés — "A segmented control that displays text labels doesn’t need introductory text." — segmented-controls › Content
**Spécifique macOS**
- Choix unique ou choix multiple selon le contrôle (Keynote : alignement à choix unique, attributs de police à choix multiple) — "A segmented control offers a single choice from among a set of options, or in macOS, either a single choice or multiple choices." — segmented-controls › Discussion
- Ou jeu de boutons à action, sans état de sélection (Répondre, Répondre à tous, Transférer dans Mail) — "a segmented control can function as a set of buttons that perform actions without showing a selection state" — segmented-controls › Discussion
- Avec des symboles ou des icônes : texte introductif possible, libellé sous chaque segment, et une infobulle par segment — "If your app includes tooltips, provide one for each segment in a segmented control." — segmented-controls › Platform considerations › macOS
- Changement de vue : un tab view dans la zone principale de la fenêtre, et non un contrôle segmenté — "Use a tab view in the main window area — instead of a segmented control — for view switching." — segmented-controls › Platform considerations › macOS
  - Le contrôle segmenté reste indiqué dans une barre d'outils ou un inspecteur — "Consider using a segmented control to help people switch views in a toolbar or inspector pane." — segmented-controls › Platform considerations › macOS
- Spring loading, avec un Magic Trackpad — "On a Mac equipped with a Magic Trackpad, spring loading lets people activate a segment by dragging selected items over it and force clicking without dropping the selected items." — segmented-controls › Platform considerations › macOS
**À transposer dans un shell Linux**
- Réglages rapides et menus de la barre : choix étroitement liés (mode d'affichage, tri, position de la barre) dans un contrôle segmenté à segments de même largeur.
- Cinq à sept segments au plus ; un seul type par contrôle ; pas de mélange icônes et texte ; infobulle par segment si icônes.
- Sur macOS, le changement de vue de la zone principale revient au tab view : réserver le contrôle segmenté aux barres d'outils, inspecteurs et panneaux de réglages.
- Spring loading : propre au Magic Trackpad, non transposable en l'état ; aucune cote de hauteur ou de rayon à reprendre.

### Sliders (`sliders`)

**Rôle** : un curseur (slider) est une piste horizontale portant une poignée (thumb), déplacée entre une valeur minimale et une valeur maximale.
**Anatomie et cotes**
- Piste et poignée ; la part de piste entre le minimum et la poignée se remplit de couleur — "As a slider’s value changes, the portion of track between the minimum value and the thumb fills with color." — sliders › Discussion
- Icônes optionnelles à gauche et à droite, illustrant minimum et maximum — "A slider can optionally display left and right icons that illustrate the meaning of the minimum and maximum values." — sliders › Discussion
- Longueur de piste, hauteur, taille de poignée, rayon : non chiffrés dans la HIG.
**Règles**
- **Direction** : minimum côté leading, maximum côté trailing — "with minimum values on the leading side and maximum values on the trailing side (for horizontal sliders)" — sliders › Best practices ; en vertical, minimum en bas et maximum en haut — "minimum values at the bottom and maximum values at the top (for vertical sliders)" — sliders › Best practices
- **Aspect** : personnalisation possible (couleur de piste, image et teinte de la poignée, icônes) — "Customize a slider’s appearance if it adds value." — sliders › Best practices
- **Saisie exacte** : adjoindre un champ texte et un stepper — "Consider supplementing a slider with a corresponding text field and stepper." — sliders › Best practices ; le stepper incrémente par valeurs entières — "Adding a stepper provides a convenient way for people to increment in whole values." — sliders › Best practices
**Spécifique macOS**
- Curseur linéaire : poignée en losange étroit, avec ou sans graduations (tick marks) — "In a linear slider either with or without tick marks, the thumb is a narrow lozenge shape" — sliders › Platform considerations › macOS
- Curseur circulaire — "In a circular slider, the thumb appears as a small circle." — sliders › Platform considerations › macOS ; graduations en points régulièrement espacés — "Tick marks, when present, appear as evenly spaced dots around the circumference of the slider." — sliders › Platform considerations › macOS
- Style : l'horizontal convient entre un début et une fin fixes ; le circulaire à des valeurs qui se répètent — "Use circular sliders when values repeat or continue indefinitely." — sliders › Platform considerations › macOS
  - Exemples chiffrés : opacité de 0 à 100 pour cent, rotation de 0 à 360 degrés, quatre tours — "four complete rotations equals four spins, or 1440 degrees of rotation" — sliders › Platform considerations › macOS
- Retour en direct : la taille du Dock change pendant le réglage — "For example, your Dock icons are dynamically scaled when adjusting the Size slider in Dock settings." — sliders › Platform considerations › macOS
- Libellé : capitales de phrase, deux-points finals — "and end with a colon" — sliders › Platform considerations › macOS
- Graduations : elles clarifient l'échelle — "Use tick marks to increase clarity and accuracy." — sliders › Platform considerations › macOS ; étiqueter le minimum et le maximum suffit souvent — "In many cases, labeling only the minimum and maximum values is sufficient." — sliders › Platform considerations › macOS
  - Valeurs non linéaires : étiquettes périodiques (volet Energy Saver) — "When the values of the slider are nonlinear, like in the Energy Saver settings pane, periodic labels provide context." — sliders › Platform considerations › macOS
  - Infobulle de valeur au survol de la poignée — "that displays the value of the thumb when people hold their pointer over it" — sliders › Platform considerations › macOS. Clavier : aucun pas de flèche n'est donné.
**À transposer dans un shell Linux**
- Réglages rapides et écran de réglages : volume, luminosité, taille, opacité → piste remplie jusqu'à la poignée, minimum à gauche, icônes petite et grande aux extrémités.
- Effet immédiat pendant le glissement ; plage large → champ texte numérique et stepper accolés.
- Graduations avec étiquettes minimum et maximum ; infobulle de valeur au survol ; libellé en capitales de phrase terminé par deux-points.
- Seule indication de forme : poignée en losange étroit ; aucune dimension à reprendre.

### Steppers (`steppers`)

**Rôle** : un stepper (stepper) est un contrôle à deux segments qui augmente ou diminue une valeur d'un incrément.
**Anatomie et cotes**
- Il n'affiche aucune valeur et se place à côté du champ qui la montre — "A stepper sits next to a field that displays its current value, because the stepper itself doesn’t display a value." — steppers › Discussion
- Taille des segments, espacement, rayon, incrément par défaut : non chiffrés dans la HIG.
**Règles**
- **Valeur visée** : elle doit être évidente — "Make the value that a stepper affects obvious." — steppers › Best practices
- **Petits écarts** : le stepper suffit — "Steppers work well by themselves for making small changes that require a few taps or clicks." — steppers › Best practices ; **grands écarts** : l'associer à un champ texte — "Consider pairing a stepper with a text field when large value changes are likely." — steppers › Best practices
**Spécifique macOS**
- Plage large : Maj-clic (Shift-click) pour changer la valeur plus vite — "For large value ranges, consider supporting Shift-click to change the value quickly." — steppers › Platform considerations › macOS ; par exemple dix fois l'incrément — "to change the value by more than the default increment (by 10 times the default, for example)" — steppers › Platform considerations › macOS. Clavier : aucun pas de flèche ni répétition n'est décrit.
**À transposer dans un shell Linux**
- Réglages rapides et écran de réglages : stepper accolé à un champ numérique qui affiche la valeur.
- Maj-clic égale incrément multiplié par dix sur les grandes plages ; champ et stepper réunis quand les valeurs varient beaucoup.

### Text fields (`text-fields`)

**Rôle** : un champ texte (text field) est une zone rectangulaire où l'on saisit ou modifie de petits morceaux de texte précis.
**Anatomie et cotes**
- Texte de substitution (placeholder) quand le champ est vide — "Because placeholder text disappears when people start typing" — text-fields › Best practices
- Taille proportionnelle au texte attendu — "To the extent possible, match the size of a text field to the quantity of anticipated text." — text-fields › Best practices
- Débordement : coupé par défaut — "By default, the system clips any text extending beyond the bounds of a text field." — text-fields › Best practices ; ou retour à la ligne, ou troncature — "truncate (indicated by an ellipsis) at the beginning, middle, or end" — text-fields › Best practices
- Hauteur, largeur, rayon, taille de police, taille de contrôle (mini, small, regular), espacement : non chiffrés dans la HIG.
**Règles**
- **Usage** : une petite quantité d'information (nom, adresse e-mail) — "Use a text field to request a small amount of information, such as a name or an email address." — text-fields › Best practices ; au-delà, une vue de texte (text view).
- **Libellés** : texte de substitution, plus un libellé séparé — "Because placeholder text disappears when people start typing, it can also be useful to include a separate label describing the field to remind people of its purpose." — text-fields › Best practices
- **Confidentialité** : champ sécurisé pour toute donnée sensible — "Always use a secure text field when your app asks for sensitive data, such as a password." — text-fields › Best practices
- **Disposition** : espacement régulier — "Evenly space multiple text fields." — text-fields › Best practices
  - Chaque champ se rattache visiblement à son libellé — "leave enough space between them so people can easily see which input field belongs with each introductory label" — text-fields › Best practices ; empilement vertical et largeurs cohérentes — "Stack multiple text fields vertically when possible, and use consistent widths to create a more organized layout." — text-fields › Best practices ; **clavier** : tabulation dans un ordre logique — "Ensure that tabbing between multiple fields flows as people expect." — text-fields › Best practices
- **Validation** : au moment pertinent — "Validate fields when it makes sense." — text-fields › Best practices
  - Adresse e-mail : en quittant le champ — "when entering an email address, it’s best to validate when people switch to another field" — text-fields › Best practices ; nom d'utilisateur ou mot de passe : avant de le quitter — "when creating a user name or password, validation needs to happen before people switch to another field" — text-fields › Best practices
- **Nombres** : format variable selon la région — "Don’t assume the actual presentation of data, however, as formatting can vary significantly based on people’s locale." — text-fields › Best practices
- **Texte tronqué** : infobulle d'expansion — "Consider using an expansion tooltip to show the full version of clipped or truncated text." — text-fields › Best practices
**Spécifique macOS**
- Saisie et liste de choix réunies : la combo box — "Consider using a combo box if you need to pair text input with a list of choices." — text-fields › Platform considerations › macOS
- Souris : l'infobulle d'expansion apparaît quand le pointeur est posé sur le champ — "appears when someone places the pointer over the field" — text-fields › Best practices
- Le choix du type de clavier ne vaut que pour iOS, iPadOS, tvOS et visionOS ; aucune taille de champ n'est nommée pour macOS.
**À transposer dans un shell Linux**
- Verrouillage : champ de mot de passe sécurisé, validé avant de quitter le champ, avec libellé permanent en plus du texte de substitution.
- Écran de réglages : champs empilés, largeurs cohérentes, espacement suffisant pour rattacher chaque champ à son libellé ; tabulation de haut en bas ; nombres formatés selon la locale.
- Lanceur : champ unique, débordement tronqué avec ellipse, infobulle au survol ; texte long (notes) dans une vue de texte.

### Toggles (`toggles`)

**Rôle** : un interrupteur (toggle) fait choisir entre deux états opposés (marche et arrêt), chacun reconnaissable à son apparence.
**Anatomie et cotes**
- Styles : switch (interrupteur à bascule) et checkbox (case à cocher), employés différemment selon la plateforme — "A toggle can have various styles, such as switch and checkbox, and different platforms can use these styles in different ways." — toggles › Discussion
- Case à cocher — "A checkbox is a small, square button that’s empty when the button is off, contains a checkmark when the button is on" — toggles › Platform considerations › macOS › Checkboxes ; tiret pour l'état mixte, titre côté trailing — "Typically, a checkbox includes a title on its trailing side." — toggles › Platform considerations › macOS › Checkboxes
- Bouton radio (radio button) — "A radio button is a small, circular button followed by a label." — toggles › Platform considerations › macOS › Radio buttons ; plein quand il est sélectionné, vide sinon — "A radio button’s state is either selected (a filled circle) or deselected (an empty circle)." — toggles › Platform considerations › macOS › Radio buttons ; par groupes de deux à cinq — "Typically displayed in groups of two to five, radio buttons present a set of mutually exclusive choices." — toggles › Platform considerations › macOS › Radio buttons
- Switch : deux tailles nommées, mini et regular — "you can use a regular switch for the primary setting and mini switches for the subordinate settings" — toggles › Platform considerations › macOS › Switches ; la hauteur du mini rejoint celle des boutons — "The height of a mini switch is similar to the height of buttons and other controls, resulting in rows that have a consistent height." — toggles › Platform considerations › macOS › Switches. Aucune hauteur, largeur, rayon, taille de coche ni couleur d'état n'est donné.
**Règles**
- **Usage** : deux valeurs opposées affectant l'état d'un contenu ou d'une vue — "Use a toggle to help people choose between two opposing values that affect the state of content or a view." — toggles › Best practices ; pour choisir dans une liste, un autre composant (pop-up button) — "A toggle always lets people manage the state of something" — toggles › Best practices ; nommer clairement ce qui est affecté — "Clearly identify the setting, view, or content the toggle affects." — toggles › Best practices ; sur macOS, un libellé peut décrire l'état — "In some cases, often in macOS apps, you can also supply a label to describe the state the toggle controls." — toggles › Best practices
- **Lisibilité de l'état** : différences visuelles évidentes — "Make sure the visual differences in a toggle’s state are obvious." — toggles › Best practices ; jamais la seule couleur — "Avoid relying solely on different colors to communicate state, because not everyone can perceive the differences." — toggles › Best practices
- **Switch ou case** : le switch met en valeur — "Prefer a switch for settings that you want to emphasize." — toggles › Platform considerations › macOS › Switches ; il pèse plus qu'une case — "A switch has more visual weight than a checkbox" — toggles › Platform considerations › macOS › Switches ; la case pour une hiérarchie de réglages — "Use a checkbox instead of a switch if you need to present a hierarchy of settings." — toggles › Platform considerations › macOS › Checkboxes ; ne pas remplacer une case existante — "In general, don’t replace a checkbox with a switch." — toggles › Platform considerations › macOS › Switches
- **Case ou radio** : plus de deux options exclusives → boutons radio — "Consider using radio buttons if you need to present a set of more than two mutually exclusive options." — toggles › Platform considerations › macOS › Checkboxes ; un seul réglage marche ou arrêt → case — "To present a single setting that can be on or off, prefer a checkbox." — toggles › Platform considerations › macOS › Radio buttons ; choix multiples → cases — "If you need to let people choose multiple options in a set, use checkboxes instead." — toggles › Platform considerations › macOS › Radio buttons
- **Groupes** : libellé de groupe de cases aligné sur la première — "Describe the set of options, and align the label’s baseline with the first checkbox in the group." — toggles › Platform considerations › macOS › Checkboxes ; radios en ligne : espacement calé sur le libellé le plus long — "Measure the space needed to accommodate the longest button label, and use that measurement consistently." — toggles › Platform considerations › macOS › Radio buttons
**Spécifique macOS**
- Trois styles : switch, checkbox et boutons radio — "In addition to the switch toggle style, macOS supports the checkbox style and also defines radio buttons that can provide similar behaviors." — toggles › Platform considerations › macOS
- **Placement** : dans le corps de la fenêtre — "Use switches, checkboxes, and radio buttons in the window body, not the window frame." — toggles › Platform considerations › macOS ; pas dans une barre d'outils ni une barre d'état — "In particular, avoid using these components in a toolbar or status bar." — toggles › Platform considerations › macOS
- **Switches** : dans un formulaire groupé, un mini switch par rangée — "Within a grouped form, consider using a mini switch to control the setting in a single row." — toggles › Platform considerations › macOS › Switches ; switch regular pour le réglage principal, mini pour les subordonnés (citation en Anatomie).
- **Checkboxes** : état fidèle, on, off ou mixed — "Accurately reflect a checkbox’s state in its appearance." — toggles › Platform considerations › macOS › Checkboxes ; case mère en état mixte quand les subordonnées diffèrent — "show a mixed state when the subordinate checkboxes have different states" — toggles › Platform considerations › macOS › Checkboxes
  - Dépendances montrées par alignement au bord leading et indentation — "By using alignment — generally along the leading edge of the checkboxes — and indentation, you can show dependencies" — toggles › Platform considerations › macOS › Checkboxes ; sans titre dans une liste de tâches éditable — "In an editable checklist, a checkbox can appear without a title or any additional content." — toggles › Platform considerations › macOS › Checkboxes
- **Radio buttons** : état mixte possible mais peu utile — "this state is rarely useful because you can communicate multiple states by using additional radio buttons" — toggles › Platform considerations › macOS › Radio buttons ; une paire de radios pour un seul réglage si la case ne suffit pas — "In rare cases where a single checkbox doesn’t clearly communicate the opposing states, you can use a pair of radio buttons" — toggles › Platform considerations › macOS › Radio buttons ; au-delà d'environ cinq options, un pop-up button — "If you need to present more than about five options, consider using a component like a" — toggles › Platform considerations › macOS › Radio buttons
- **Boutons à bascule** (buttons that behave like toggles), admis sur toutes les plateformes — "In addition to toggles, all platforms also support buttons that behave like toggles by using a different appearance for each state." — toggles › Discussion ; à icône et apparence variant selon l'état — "you generally use an interface icon that communicates its purpose, and you update its appearance" — toggles › Best practices. Souris et clavier : aucun clic ni aucune touche (Espace, flèches) n'est décrit.
**À transposer dans un shell Linux**
- Écran de réglages : switch pour les réglages principaux, cases alignées au bord leading et indentées pour les groupes (case mère en état mixte), boutons radio pour deux à cinq options exclusives, bouton déroulant au-delà de cinq.
- Corps du panneau de réglages seulement : la HIG écarte ces contrôles des barres d'outils et d'état ; dans la barre du bureau, employer des boutons à bascule (icône, apparence changée selon l'état).
- Mini switch sur les rangées de formulaire, regular pour le réglage principal ; aucune hauteur en points n'est fournie.
- Ne jamais coder l'état par la seule couleur ; libeller chaque interrupteur.

### Virtual keyboards (`virtual-keyboards`)

- Hors périmètre : les claviers virtuels (virtual keyboards) n'existent pas sur macOS — "Not supported in macOS." — virtual-keyboards › Platform considerations ; la page ne fournit rien pour un bureau à clavier physique, sinon qu'un clavier virtuel — "doesn’t support keyboard shortcuts" — virtual-keyboards › Discussion

### Valeurs de référence

| Propriété | Valeur | Source (slug › section) |
|---|---|---|
| Nombre de segments | au moins deux ; environ 5 à 7 au plus dans une interface large ; environ 5 au plus sur iPhone | selection-and-input › Discussion ; segmented-controls › Best practices |
| Liste des minutes d'un sélecteur de date | 60 valeurs (0 à 59) par défaut ; intervalle divisible par 60 ; exemple 0, 15, 30, 45 | pickers › Best practices |
| Styles de sélecteur de date (macOS) | textuel ; graphique | pickers › Platform considerations › macOS |
| Curseur horizontal | minimum côté leading, maximum côté trailing ; exemple de 0 percent à 100 percent | sliders › Best practices |
| Curseur vertical | minimum en bas, maximum en haut | sliders › Best practices |
| Exemples de plage (macOS) | opacité 0 à 100 pour cent ; rotation 0 à 360 degrés ; quatre tours, soit 1440 degrés | sliders › Platform considerations › macOS |
| Poignée de curseur (macOS) | linéaire : losange étroit ; circulaire : petit cercle ; graduations en points espacés régulièrement | sliders › Platform considerations › macOS |
| Étiquetage des graduations | minimum et maximum souvent suffisants | sliders › Platform considerations › macOS |
| Stepper, Maj-clic (macOS) | plus que l'incrément par défaut ; exemple : 10 fois | steppers › Platform considerations › macOS |
| Tailles de switch (macOS) | mini, regular ; hauteur du mini proche de celle des boutons et autres contrôles | toggles › Platform considerations › macOS › Switches |
| Case à cocher | petit bouton carré ; vide (off), coche (on), tiret (mixed) ; titre côté trailing | toggles › Platform considerations › macOS › Checkboxes |
| Boutons radio | petit bouton circulaire ; groupes de deux à cinq ; plein (selected) ou vide (deselected) | toggles › Platform considerations › macOS › Radio buttons |
| Seuil de passage des boutons radio au pop-up button | plus d'environ cinq options | toggles › Platform considerations › macOS › Radio buttons |
| Placement des switchs, cases et boutons radio (macOS) | corps de la fenêtre ; ni barre d'outils ni barre d'état | toggles › Platform considerations › macOS |
| Prise en charge sur macOS | digit entry view : non ; virtual keyboard : non | digit-entry-views › Platform considerations ; virtual-keyboards › Platform considerations |
| Hauteurs, largeurs, rayons, durées, couleurs | non chiffrés dans la HIG | les douze pages |

## Seconde partie — hyprdesk aujourd'hui, écarts et actions

### Ce que hyprdesk fait aujourd'hui

| Surface | Ce qui existe | Valeurs clés | Preuves |
|---|---|---|---|
| Interrupteurs des Réglages | Commutateur sous un libellé de champ, style d'Iced ; avertissement quand `hyprdesk.lua` surcharge la clé | — | `src/ui/hyprset.rs:221`, `239` ; `src/ui/panel.rs:1093-1111` |
| Interrupteurs hors des Réglages | Centre de contrôle Continuité : pastilles Wi-Fi, Bluetooth, Silence ; menu Son : commutateur « Muet » ; en-tête des Réglages : commutateur « Configuration avancée » | — | `src/ui/plugins/bar/popover/continuity.rs:597`, `617-643`, `219-239` ; `src/ui/plugins/bar/popover.rs:1766-1778` ; `src/ui/panel.rs:949-953` |
| Tuiles Wi-Fi et Bluetooth du widget `controls` | Pastille ronde, libellé, ligne de détail ; attente (« En cours … », bouton inactif), erreur (« Indisponible » et message en couleur d'avertissement), chargement | pastille 32 px ; texte 12,5 et 11 | `src/ui/controls.rs:44-61`, `70-90`, `143-146`, `178-185` |
| Curseurs | Piste remplie depuis le minimum, poignée ronde, valeur en texte ; dans les Réglages, application à chaud puis enregistrement au relâchement | piste 10 px (barre et menus), 5 px (Continuité) ; volume 0 à 150 % par pas de 0,01 ; curseur 16 à 96 par pas de 2 | `src/ui/design.rs:58-77` ; `src/ui/plugins/bar/popover/continuity.rs:248-257` ; `src/ui/hyprset.rs:252-257`, `686-690` ; `src/ui/controls.rs:274-279` |
| Compteurs | « − valeur + » sous un libellé ; bornes inactives ; pour les couleurs d'écran, valeur en couleur d'accent si elle diffère de la valeur native | workspaces de 1 à `MAX_WORKSPACES` | `src/ui/plugins/workspaces.rs:305-331`, `528` ; `src/ui/plugins/colors.rs:866-878` |
| Segments | « Affichage du bureau » : Masqué, Lanceur, Complet ; dans le widget `controls` et dans le centre de contrôle | 3 segments | `src/ui/controls.rs:305-359` ; `src/ui/plugins/bar/popover/continuity.rs:1307-1328` |
| Listes déroulantes | Boutons à liste pour les choix, les entiers et le choix à trois états « Comme partout, Oui, Non » | largeurs de 180 à 320 px | `src/ui/hyprset.rs:352-370` ; `src/ui/hyprlists.rs:448-515` |
| Champs texte | Libellé au-dessus (`field`) ; validation à Entrée pour les options et couleurs de Hyprland et pour les couleurs du thème ; champs secrets pour les données sensibles | — | `src/ui/panel.rs:1093-1111` ; `src/ui/hyprset.rs:213`, `318-345`, `465-482` ; `src/ui/plugins/theme.rs:1015-1024` ; `src/ui/plugins/bar/popover.rs:905-963` |
| Couleur | Sélecteur propre (teinte × saturation, luminosité, avant et après, hex, Appliquer et Annuler), ouvert par « Choisir… » pour les couleurs de module de barre ; ailleurs pastilles prédéfinies et champ hex | zone 264 × 132 | `src/ui/color_picker.rs:18-19`, `123-280` ; `src/ui/plugins/bar.rs:958-966`, `1268-1282` ; `src/ui/plugins/theme.rs:564-587` |

### Écarts

Classés du plus important au moins important. Les chemins sont relatifs à la racine du dépôt.

| # | Composant | Nature | Force | Règle HIG (slug › section) | Réel dans hyprdesk (preuve) | Action proposée | Effort | Confiance |
|---|---|---|---|---|---|---|---|---|
| SEL-01 | Champ numérique des listes de règles Hyprland | ÉCART | « your app needs to alert people » ; « Use a number formatter » | « Validate fields when it makes sense. For example, if the only legitimate value for a field is a string of digits, your app needs to alert people if they've entered characters other than digits. » ; « A number formatter automatically configures the text field to accept only numeric values. » — text-fields › Best practices | Un champ `Field::Num` accepte tout texte : un nombre est lu, sinon la chaîne est gardée telle quelle et posée dans la règle (`src/ui/hyprlists.rs:532`), puis appliquée à Hyprland sans message (`src/ui/hyprlists.rs:987-994`). Le champ n'a pas de texte de substitution (`src/ui/hyprlists.rs:517`). | Refuser une valeur non numérique dans `Field::Num` et afficher l'erreur à côté du champ, comme les options de Hyprland (`src/ui/hyprset.rs:213`). | S | haute |
| SEL-02 | Puits de couleur | ABSENT | Composant macOS sans équivalent | « A color well displays a color picker when people tap or click it. » — color-wells › Discussion ; « When people click a color well, it receives a highlight to provide visual confirmation that it's active. It then opens a color picker… After they make a selection, the color well updates to show the new color. » — color-wells › Platform considerations › macOS | La couleur en cours d'un emplacement du thème est une pastille qui n'ouvre rien (`dot(value, 22.0, false)`, `src/ui/plugins/theme.rs:578`) ; on la règle par pastilles prédéfinies et champ hex (`src/ui/plugins/theme.rs:564-587`). Le sélecteur n'est relié qu'aux couleurs de module de barre, par un bouton « Choisir… » (`src/ui/plugins/bar.rs:958-966`). Les couleurs d'options Hyprland n'ont qu'un champ hex et des aperçus (`src/ui/hyprset.rs:318-345`). | Faire de la pastille de la couleur en cours un puits qui ouvre le sélecteur existant, partout où une couleur se règle. | M | haute |
| SEL-03 | Commutateur « Configuration avancée » | ÉCART | « Use… not » ; « avoid using » | « Use switches, checkboxes, and radio buttons in the window body, not the window frame. In particular, avoid using these components in a toolbar or status bar. » — toggles › Platform considerations › macOS | Le commutateur siège dans la ligne d'actions de l'en-tête des Réglages, avec la recherche, « Visite guidée », « Assistant » et la fermeture (`src/ui/panel.rs:919-955`, commutateur lignes 949-953). Il n'a pas d'autre emplacement dans le corps : l'autre occurrence du libellé est la visite guidée (`src/ui/tour.rs:309`). | Déplacer le commutateur dans le corps des Réglages, sur une ligne propre de la vue de détail. | S | moyenne |
| SEL-04 | Pastilles Wi-Fi, Bluetooth, Silence du centre de contrôle | ÉCART | « Make sure » ; « Avoid relying solely » | « Make sure the visual differences in a toggle's state are obvious… Avoid relying solely on different colors to communicate state, because not everyone can perceive the differences. » — toggles › Best practices | Même icône et même libellé dans les deux états ; seuls changent le fond (accent, ou surface en creux) et la couleur du texte (`src/ui/plugins/bar/popover/continuity.rs:617-643`, `219-239`). | Ajouter dans la pastille un indice d'état qui ne tient pas à la couleur (mot d'état, ou nom du réseau quand il est connecté). | S | moyenne |
| SEL-05 | Segments sans info-bulle | ÉCART | « provide one for each segment », condition remplie | « If your app includes tooltips, provide one for each segment in a segmented control. » — segmented-controls › Platform considerations › macOS | Les segments « Masqué, Lanceur, Complet » (`src/ui/controls.rs:326-353` ; `src/ui/plugins/bar/popover/continuity.rs:1314-1326`) et les cellules d'espaces (`src/ui/plugins/bar/popover/continuity.rs:1273-1294`) n'ont pas d'info-bulle : `dash_button` n'en met pas (`src/ui/plugins/bar/popover/continuity.rs:384-398`) alors que `dash_icon_button` en met une (`src/ui/plugins/bar/popover/continuity.rs:402-428`). Les noms d'espaces y sont coupés à 4 caractères (ligne 1278). | Donner une info-bulle à chaque segment : son libellé, et le nom complet de l'espace. | S | haute |
| SEL-06 | Libellés des segments « Affichage du bureau » | ÉCART | « Use » (impératif, sans mot-clé de force) | « Use nouns or noun phrases for segment labels. » — segmented-controls › Content | « Masqué », « Lanceur », « Complet » : deux adjectifs et un nom (`src/ui/controls.rs:310-323` ; `src/ui/plugins/bar/popover/continuity.rs:1311-1313`). | Libeller chaque segment par un groupe nominal qui nomme le mode d'affichage. | S | haute |
| SEL-07 | Champ hexadécimal du sélecteur de couleur | ÉCART | « your app needs to alert people » | « if the only legitimate value for a field is a string of digits, your app needs to alert people if they've entered characters other than digits » — text-fields › Best practices | Une valeur illisible rendue par Entrée est effacée sans message : le champ reprend la couleur en cours (`src/ui/color_picker.rs:112-118`). Le champ hex du thème, lui, signale « couleur illisible » (`src/ui/plugins/theme.rs:1015-1024`). | Garder le texte saisi et afficher l'erreur sous le champ. | S | haute |
| SEL-08 | Valeur des curseurs en lecture seule | PISTE | « Consider » | « Consider supplementing a slider with a corresponding text field and stepper. » — sliders › Best practices | La valeur est un texte non modifiable : options numériques (`src/ui/hyprset.rs:252-262`), taille du curseur (`src/ui/hyprset.rs:686-696`), volume et luminosité des menus (`src/ui/plugins/bar/popover.rs:1698-1721`, `1754-1759`). | Rendre la valeur des curseurs des Réglages saisissable, validée à Entrée. | M | haute |
| SEL-09 | Graduations des curseurs à pas discret | PISTE | « Use tick marks… » (sous Platform considerations, sans mot-clé de force) | « Use tick marks to increase clarity and accuracy. » ; « In many cases, labeling only the minimum and maximum values is sufficient. » — sliders › Platform considerations › macOS | Aucune graduation ni valeur de bornes sur les curseurs à pas : options (`step`, `src/ui/hyprset.rs:252-257`), taille du curseur (`src/ui/hyprset.rs:686-690`), luminosité du sélecteur (`src/ui/color_picker.rs:186-190`). | Graduer les curseurs à pas discret et étiqueter leur minimum et leur maximum. | M | moyenne |
| SEL-10 | Libellés et icônes des curseurs | PISTE | « Consider » | « Consider using a label to introduce a slider. Labels generally use sentence-style capitalization and end with a colon. » ; « A linear slider often includes supplementary icons that illustrate the meaning of the minimum and maximum values. » — sliders › Platform considerations › macOS | Libellés sans deux-points : « Luminosité » (`src/ui/plugins/bar/popover/continuity.rs:791`), « Son · 45 % » (`src/ui/plugins/bar/popover/continuity.rs:852-856`), « Sortie » et « Micro » (`src/ui/plugins/bar/popover.rs:1628-1636`, `src/ui/menu.rs:241-257`). Une seule icône pour la luminosité (`src/ui/plugins/bar/popover/continuity.rs:762`), aucune dans les menus (`src/ui/plugins/bar/popover.rs:1698-1721`, `src/ui/controls.rs:250-254`). | Une seule ligne curseur : libellé terminé par deux points, icônes du minimum et du maximum de part et d'autre. | S | haute |
| SEL-11 | Textes coupés sans info-bulle d'expansion | PISTE | « Consider » | « Consider using an expansion tooltip to show the full version of clipped or truncated text. » — text-fields › Best practices | Noms de périphériques audio coupés à 28 caractères (`src/ui/plugins/bar/popover.rs:1817`), noms d'espaces à 4 (`src/ui/plugins/bar/popover/continuity.rs:1278`), dossier parent d'un fichier à 34 (`src/ui/search.rs:1309-1324`). | Afficher le texte entier en info-bulle au survol de ces lignes. | S | haute |
| SEL-12 | Sélecteur de couleur : couleurs gardées | PISTE | « Consider » | « Consider the system-provided color picker for a familiar experience… letting people save a set of colors they can access from any app. » — color-wells › Best practices | Sélecteur propre, sans palette de couleurs enregistrées (`src/ui/color_picker.rs:123-280`). | Ajouter au sélecteur une rangée de couleurs gardées, commune à tous les réglages de couleur. | S | haute |
| SEL-13 | Largeur des segments « Affichage du bureau » | PISTE | « In general, keep… » | « In general, keep segment size consistent. When all segments have equal width, a segmented control feels balanced. » — segmented-controls › Best practices | Segments larges comme leur contenu dans le centre de contrôle (`src/ui/plugins/bar/popover/continuity.rs:1314-1320`), à largeur égale dans le widget `controls` (`src/ui/controls.rs:334`). | Même rendu aux deux endroits : segments de largeur égale. | S | haute |
| SEL-14 | Listes déroulantes sans valeur posée | PISTE | « Populate the field… » (combo boxes, pris pour la moitié liste) | « Populate the field with a meaningful default value from the list. » — combo-boxes › Best practices | Aucun texte de substitution n'est fixé sur les listes d'options (`src/ui/hyprset.rs:363-368`) ni sur celles des champs de règles (`src/ui/hyprlists.rs:472-515`) ; une seule en a un (`src/ui/hyprlists.rs:781`). Sans valeur posée, aucune indication n'est prévue. | Donner à toute liste déroulante un texte de substitution quand aucune valeur n'est posée. | S | moyenne |

Hors périmètre, relevé au passage : plusieurs libellés ne passent pas par `tr()` et resteront en français quelle que soit la langue — `src/ui/controls.rs:208`, `296`, `310-320` et les titres du centre de contrôle (`src/ui/plugins/bar/popover/continuity.rs:543`, `1082`, `1160`, `1221`, `1329`).

### Ce qui est déjà conforme

- Interrupteurs nommés : « Clearly identify the setting, view, or content the toggle affects. » (toggles › Best practices) | chaque commutateur des Réglages est sous un libellé de champ — `src/ui/hyprset.rs:221`, `239`, `src/ui/panel.rs:1093-1111` ; « Muet », « Nombre fixe » — `src/ui/plugins/bar/popover.rs:1766-1771`, `src/ui/plugins/workspaces.rs:310-313`.
- État d'un commutateur lisible sans la couleur : « Avoid relying solely on different colors to communicate state » (toggles › Best practices) | commutateur d'Iced, la poignée change de côté ; aucun style propre de commutateur dans le projet — `src/ui/plugins/bar/popover.rs:1772-1778`.
- Bouton à deux états changeant de fond : « you generally use an interface icon that communicates its purpose, and you update its appearance — typically by changing the background — based on the current state » (toggles › Best practices) | pastille ronde des tuiles Wi-Fi et Bluetooth, fond d'accent quand actives — `src/ui/controls.rs:70-90`.
- Retour à chaud : « Consider giving live feedback as the value of a slider changes. » (sliders › Platform considerations › macOS) | curseur d'option appliqué pendant le glissé, enregistré au relâchement — `src/ui/hyprset.rs:252-257`.
- Sens des curseurs : « Use familiar slider directions. » (sliders › Best practices) | minimum à gauche — `src/ui/design.rs:58-77`, `src/ui/controls.rs:274` ; minimum en bas pour le curseur vertical de luminosité — `src/ui/plugins/bar/popover/continuity.rs:764-770` (sens d'`iced_widget` 0.14.0, `src/vertical_slider.rs:282-283` du crate).
- Piste remplie depuis le minimum : « the portion of track between the minimum value and the thumb fills with color » (sliders › Discussion) | deux fonds de piste — `src/ui/design.rs:64-71`, `src/ui/plugins/bar/popover/continuity.rs:251`.
- Valeur visible en texte : « It's also a good idea to provide a tooltip that displays the value of the thumb » (sliders › Platform considerations › macOS) | pourcentage à droite du curseur — `src/ui/plugins/bar/popover.rs:1754-1759`, `src/ui/controls.rs:280-289`.
- Compteur : « Make the value that a stepper affects obvious. » (steppers › Best practices) | « − N + » sous le libellé « Workspaces permanents », bornes inactives — `src/ui/plugins/workspaces.rs:305-331`, `528`.
- Segments : nombre limité, un seul type de contenu, choix qui touchent un état : « Aim for no more than about five to seven segments » ; « Prefer using either text or images — not a mix of both » (segmented-controls › Best practices ; › Content) ; « Use a segmented control to provide closely related choices that affect an object, state, or view. » | trois segments de texte pour le mode d'affichage du bureau — `src/ui/controls.rs:305-359`.
- Indice dans un champ texte : « Show a hint in a text field to help communicate its purpose. » (text-fields › Best practices) | textes de substitution (« #rrggbb ou #rrggbbaa », « Choisir un périphérique… ») et libellé séparé — `src/ui/hyprset.rs:299-307`, `src/ui/hyprlists.rs:781`, `src/ui/panel.rs:1093-1111`.
- Validation là où elle a du sens : « Validate fields when it makes sense. » (text-fields › Best practices) | options de Hyprland : erreur affichée, valeur non posée — `src/ui/hyprset.rs:465-482`, `213` ; couleurs du thème — `src/ui/plugins/theme.rs:1015-1024`.
- Champs secrets : « Always use a secure text field when your app asks for sensitive data » (text-fields › Best practices) | `.secure(` dans `src/lock/mod.rs`, `src/greeter_ui.rs`, `src/ui/polkit.rs`, `src/ui/mymfa.rs`, `src/ui/mymfa_fenetre.rs`, `src/ui/plugins/assistant/providers.rs` et dans les menus de la barre (`src/ui/plugins/bar/popover.rs:942`).
- Champs alignés : « use consistent widths to create a more organized layout » (text-fields › Best practices) | variables d'environnement : nom de 260 px, valeur en pleine largeur, écart de 8 — `src/ui/hyprset.rs:614-631`.
- Ouverture du sélecteur signalée : « it receives a highlight to provide visual confirmation that it's active » (color-wells › Platform considerations › macOS) | bouton « Choisir… » en style primaire tant que le sélecteur est ouvert — `src/ui/plugins/bar.rs:1268-1282`.
- Choix exclusif marqué d'une coche : « contains a checkmark when the button is on » (toggles › Platform considerations › macOS › Checkboxes) | périphérique audio par défaut — `src/ui/plugins/bar/popover.rs:1804-1818`.
- Sans objet : claviers virtuels, vues de saisie de chiffres et puits d'image — « Not supported in… macOS » (virtual-keyboards, digit-entry-views, image-wells › Platform considerations) ; aucun clavier virtuel dans l'interface (seule mention, `src/doctor.rs`). Maj-clic à ×10 sur un compteur — « For large value ranges » (steppers › Platform considerations › macOS), plages courtes (`src/ui/plugins/workspaces.rs:317-331`).

### À vérifier sur un rendu

- État choisi des segments « Masqué, Lanceur, Complet » : fond à 13 % du texte dans le widget `controls` (`src/ui/controls.rs:339-352`), accent plein dans le centre de contrôle (`src/ui/plugins/bar/popover/continuity.rs:219-224`) ; la différence se voit-elle sur le verre des menus (opacité 0,94) ?
- Pastilles Wi-Fi et Bluetooth : accent iris et surface en creux se distinguent-ils sur un fond clair (pack « clair ») — `src/ui/plugins/bar/popover/continuity.rs:219-239`.
- Commutateurs sur le verre du menu Son — `src/ui/plugins/bar/popover.rs:1772-1778`.
- Taille perçue des petites cibles : croix de fermeture du centre de contrôle (`src/ui/controls.rs:217-229`), chevron « › » des tuiles (`src/ui/controls.rs:163-174`), cellules d'espaces de 43 px (`src/ui/plugins/bar/popover/continuity.rs:1286-1288`). La HIG de cette famille ne fixe aucune cote.
- Champs de règles à 320 px pour des chemins et des expressions (`src/ui/hyprlists.rs:542`) : place suffisante ?
- Messages d'erreur des options de Hyprland : lisibilité sous le champ (`src/ui/hyprset.rs:181-220`).
