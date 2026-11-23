# Fondations : typographie, écriture et disposition générale — macOS d'après la HIG, face à hyprdesk

> Proposition de design, rien d'implémenté. Plan d'ensemble et priorités : `plan-d-action.md` ; méthode et périmètre : `README.md`.

> Référentiel : Human Interface Guidelines d'Apple, export Markdown officiel relevé le 23 novembre 2026. Code : commit `00f6d69` (arbre de travail du 23 novembre 2026, avec des modifications non commitées).

## Première partie — Spécification d'après la HIG

### Résumé

- Ces quatre pages chiffrent peu la disposition : marges, grilles et densité n'y ont aucune valeur. Les cotes macOS sont typographiques (onze styles de texte, 13 pt par défaut, 10 pt minimum, tableau de tracking), tactiles (28x28 pt par défaut, 20x20 pt minimum) et contrastives (4.5:1, 3:1).
- La disposition repose sur des principes : ordre de lecture, alignement et indentation comme signaux de hiérarchie, groupement, affichage progressif, contrôles distincts du contenu, adaptation continue à la taille de la fenêtre.
- Deux consignes propres à macOS commandent toute fenêtre, donc aussi la barre, le dock et les panneaux : rien d'essentiel en bas, rien derrière le logement de la caméra en haut.
- La typographie macOS va de Large Title (26 pt, interlignage 32) à Caption (10 pt, interlignage 13), avec un tracking négatif de 13 à 23 pt ; macOS ne prend pas en charge Dynamic Type.
- L'écriture fixe une voix, un ton, des verbes pour les boutons, une casse uniforme par type d'élément, des erreurs sans blâme et des libellés de réglages simples ; elle ne traite ni des éléments de menu, ni de la ponctuation, ni des textes de chargement, ni des dates et heures.
- L'accessibilité exige une interface intuitive, perceptible et adaptable, un contraste minimal, des contrôles assez grands et espacés, la navigation au clavier seul et le respect de Réduire les animations (Reduce Motion).
- Writing et Accessibility n'ont aucun bloc macOS ; Layout en a deux phrases ; Typography fournit les variantes de police propres à macOS, dont celles de la barre des menus et des menus.

### Layout (`layout`)

**Rôle** — Fournir la structure qui rend le contenu compréhensible dès l'ouverture et qui s'adapte aux tailles d'écran, aux orientations et aux configurations multitâches.

**Anatomie et cotes**

- Non chiffré dans la HIG (marges, gouttières, grilles, espacement, densité) ; Apple renvoie à ses modèles — "Apple provides templates and layout guides" — layout › Discussion
- Guides de disposition (layout guides) — "The system includes predefined layout guides that make it easy to apply standard margins around content and restrict the width of text for optimal readability." — layout › Guides and safe areas ; un guide — "defines a rectangular region that helps you position, align, and space your content on the screen" — layout › Guides and safe areas ; des guides personnalisés sont possibles et la page cite `AppKit/NSLayoutGuide` pour Mac.
- Zone sûre (safe area) — "the area within a window that isn’t covered on the edge by a hardware feature or another view within the window" — layout › Guides and safe areas ; vues citées — "like a toolbar, tab bar, or status bar" — layout › Guides and safe areas
- Classes de taille (size classes), d'iOS et d'iPadOS — "In iOS and iPadOS, size classes are an indication of how much horizontal and vertical space is available to an app’s interface." — layout › Size classes ; chaque dimension — "is represented by one of two size classes:" — layout › Size classes ; "when a person runs your app in macOS with iPhone Mirroring, they can freely resize its width and height" — layout › Size classes

**Règles**

*Hiérarchie visuelle*

- Ordre de lecture — "People often start by viewing content in reading order" — layout › Visual hierarchy ; "place the most important items near the top and leading side of the window or display." — layout › Visual hierarchy ; droit-à-gauche — "prefer standard system components that can automatically adapt UI elements to better reflect each language’s natural reading order" — layout › Visual hierarchy
- Alignement et indentation — "Align elements to make them easier to scan, and use indentation to convey hierarchy." — layout › Visual hierarchy ; "People assume that aligned items are related to each other, and conversely, they perceive indented items as subordinate to the item they follow." — layout › Visual hierarchy
- Groupement — "Group related items to clearly express related information or functions." — layout › Visual hierarchy ; "you might use negative space, container shapes, or separator lines to show which elements are related and which are unrelated" — layout › Visual hierarchy
- Affichage progressif (progressive disclosure) — "Use disclosure triangles, menus, or nested views to reduce how much content to initially display" — layout › Visual hierarchy ; "An interface with too much content and too many choices makes it harder to find information quickly, and harder to understand the choices that are available." — layout › Visual hierarchy
- Contrôles distincts du contenu — "Take advantage of the Liquid Glass material on all platforms that support it to provide a distinct appearance for your controls." — layout › Visual hierarchy ; "Instead of applying a solid or semi-opaque background color beneath controls, use a scroll edge effect to visually elevate controls above content." — layout › Visual hierarchy
- Arrière-plan plein écran — "For full-screen background content, be sure to extend it underneath sidebars, toolbars, and tab bars to fit the entire screen or window." — layout › Visual hierarchy ; si des composants masquent l'image — "you can use a background extension effect to flip and blur the image, mirroring it beneath adjacent components" — layout › Visual hierarchy

*Adaptation*

- Principe — "Design a layout that adapts gracefully and consistently." — layout › Adaptability ; "People expect your experience to remain familiar when they rotate their device, resize a window, add another display, or switch to a different device." — layout › Adaptability ; "it’s still important to ensure your interface resizes well to provide the best experience across devices and window sizes" — layout › Adaptability
- Marges et zones sûres — "respecting system-defined safe areas, margins, and guides (where available)" — layout › Adaptability
- Taille du texte — "Be prepared for text-size changes." — layout › Adaptability ; "Apps that don’t respond to this setting can be difficult or impossible to use for people who rely on this feature." — layout › Adaptability ; remèdes — "horizontally adjacent views may need to stack vertically to provide more space for text" — layout › Adaptability ; "table rows or other containers may need to grow in height so that text isn’t cropped or doesn’t overlap other content" — layout › Adaptability
- Vérification — "Preview your app on multiple devices, using different size classes, localizations, and text sizes." — layout › Adaptability ; "You can streamline the testing process by first testing versions of your experience that use the largest and the smallest layouts." — layout › Adaptability
- Images de fond — "don’t change the aspect ratio of the artwork; instead, scale it so that it fills the screen completely" — layout › Adaptability ; "since windows can be very wide and short or tall and narrow" — layout › Adaptability

*Classes de taille*

- Décider d'après l'espace disponible, pas d'après le type ou l'orientation de l'appareil — "Determine layout based on size classes, not device type or orientation." — layout › Size classes ; "Size classes describe the actual space available, regardless of whether an app is in portrait or landscape." — layout › Size classes
- Fonctions constantes, quantité affichée variable — "Don’t change your app’s functionality based on the space it occupies." — layout › Size classes ; "However, you can change the amount of functionality that’s visible onscreen as the amount of space changes." — layout › Size classes ; "expose functionality that might otherwise be grouped into an overflow menu" — layout › Size classes
- Continuité visuelle — "keep the layout recognizable and familiar to the platform even when resizing" — layout › Size classes ; "Consider all possible combinations of size classes." — layout › Size classes

**Spécifique macOS**

- Bas de fenêtre — "Avoid placing controls or critical information at the bottom of a window." — layout › Platform considerations › macOS ; raison — "People often move windows so that the bottom edge is below the bottom of the screen." — layout › Platform considerations › macOS
- Haut de fenêtre — "Avoid displaying content behind the camera housing at the top edge of the window." — layout › Platform considerations › macOS ; le bloc macOS se limite à ces deux consignes sur le contenu des fenêtres : la page ne mentionne ni le Dock ni la barre des menus.
- Fenêtres et écrans — "External display support, Display Zoom, and resizable windows on iPad and Mac" — layout › Adaptability ; iPhone Mirroring — "when a person runs your app in macOS with iPhone Mirroring, they can freely resize its width and height" — layout › Size classes
- Internationalisation à prévoir — "date/time/number formatting, font variation, and text length" — layout › Adaptability

**À transposer dans un shell Linux**

- Barre et dock : les traiter comme la zone que la zone sûre réserve ; ni contrôle ni information critique sous eux, ni dans le bas des surfaces du shell.
- Aperçu des espaces : ordre de lecture du haut vers le bas et du début vers la fin, groupes séparés par l'espace ou des filets, affichage progressif quand le nombre d'espaces ou de fenêtres dépasse ce qui tient à l'écran.
- Lanceur et recherche : mêmes fonctions quelle que soit la taille de la fenêtre ; seule la quantité affichée varie, le surplus passant dans un menu de débordement (overflow menu).
- Widgets : rien d'essentiel collé au bord inférieur ; les rangées s'agrandissent en hauteur quand le texte s'agrandit.
- Fond d'écran : ne jamais changer les proportions ; l'effet d'extension (retournement puis flou) peut habiller le dessous de la barre et du dock.
- Contrôles sur contenu : préférer un effet de bord de défilement (scroll edge effect) ou le matériau translucide du shell à un aplat de couleur sous les contrôles.
- Sens de lecture et locale : composants qui s'inversent en droit-à-gauche, formats de date, d'heure et de nombre de la locale.

### Typography (`typography`)

**Rôle** — Rendre le texte lisible, exprimer la hiérarchie de l'information, mettre en valeur le contenu important et porter le style de la marque.

**Anatomie et cotes**

- Unités : corps (point size) en points, interlignage (leading) en points, approche (tracking) en millièmes d'em (1/1000 em) et en points. Seules les valeurs macOS sont recopiées ; taille conseillée — "Follow the recommended default and minimum text sizes for each platform" — typography › Ensuring legibility

| Platform | Default size | Minimum size |
|---|---|---|
| macOS | 13 pt | 10 pt |

- Polices système — "Apple provides two typeface families that support an extensive range of weights, sizes, styles, and languages." — typography › Using system fonts ; San Francisco (SF) — "is a sans serif typeface family that includes the SF Pro, SF Compact, SF Arabic, SF Armenian, SF Georgian, SF Hebrew, and SF Mono variants" — typography › Using system fonts ; New York (NY) — "is a serif typeface family designed to work well by itself and alongside the SF fonts" — typography › Using system fonts
- Variantes arrondies, graisses et largeurs — "The system also offers SF Pro, SF Compact, SF Arabic, SF Armenian, SF Georgian, and SF Hebrew in rounded variants" — typography › Using system fonts ; "ranging from Ultralight to Black" — typography › Using system fonts ; "several widths, including Condensed and Expanded" — typography › Using system fonts
- Format variable et tailles optiques — "which combines different font styles together in one file, and supports interpolation between styles to create intermediate ones" — typography › Using system fonts ; "which refers to the adjustment of different typographic designs to fit different sizes" — typography › Using system fonts
- Style de texte (text style) — "specifies a combination of font weight, point size, and leading values for each text size" — typography › Using system fonts ; "Taken together, the text styles form a typographic hierarchy you can use to express the different levels of importance in your content." — typography › Using system fonts

Styles de texte intégrés de macOS (macOS built-in text styles), recopiés sans conversion. Source : typography › Specifications › macOS built-in text styles.

| Text style | Weight | Size (points) | Line height (points) | Emphasized weight |
|---|---|---|---|---|
| Large Title | Regular | 26 | 32 | Bold |
| Title 1 | Regular | 22 | 26 | Bold |
| Title 2 | Regular | 17 | 22 | Bold |
| Title 3 | Regular | 15 | 20 | Semibold |
| Headline | Bold | 13 | 16 | Heavy |
| Body | Regular | 13 | 16 | Semibold |
| Callout | Regular | 12 | 15 | Semibold |
| Subheadline | Regular | 11 | 14 | Semibold |
| Footnote | Regular | 10 | 13 | Semibold |
| Caption 1 | Regular | 10 | 13 | Medium |
| Caption 2 | Medium | 10 | 13 | Semibold |

- Variantes mises en valeur (emphasized) — "You can display emphasized variants of system text styles using symbolic traits." — typography › Specifications ; "The emphasized weights can be medium, semibold, bold, or heavy." — typography › Specifications
- Grands titres : la seule valeur macOS pour les grands titres est la ligne Large Title du tableau (26 pt, interlignage 32, Regular, Bold en variante mise en valeur) ; la page ne contient aucun autre tableau de grands titres.
- Approche — "In a running app, the system font dynamically adjusts tracking at every point size." — typography › Using system fonts ; "If necessary, adjust tracking in interface mockups." — typography › Using system fonts

Valeurs de tracking macOS (macOS tracking values), recopiées sans arrondi ni conversion. Source : typography › Specifications › Tracking values › macOS tracking values.

| Size (points) | Tracking (1/1000 em) | Tracking (points) |
|---|---|---|
| 6 | +41 | +0.24 |
| 7 | +34 | +0.23 |
| 8 | +26 | +0.21 |
| 9 | +19 | +0.17 |
| 10 | +12 | +0.12 |
| 11 | +6 | +0.06 |
| 12 | 0 | 0.0 |
| 13 | -6 | -0.08 |
| 14 | -11 | -0.15 |
| 15 | -16 | -0.23 |
| 16 | -20 | -0.31 |
| 17 | -26 | -0.43 |
| 18 | -25 | -0.44 |
| 19 | -24 | -0.45 |
| 20 | -23 | -0.45 |
| 21 | -18 | -0.36 |
| 22 | -12 | -0.26 |
| 23 | -4 | -0.10 |
| 24 | +3 | +0.07 |
| 25 | +6 | +0.15 |
| 26 | +8 | +0.22 |
| 27 | +11 | +0.29 |
| 28 | +14 | +0.38 |
| 29 | +14 | +0.40 |
| 30 | +14 | +0.40 |
| 31 | +13 | +0.39 |
| 32 | +13 | +0.41 |
| 33 | +12 | +0.40 |
| 34 | +12 | +0.40 |
| 35 | +11 | +0.38 |
| 36 | +10 | +0.37 |
| 37 | +10 | +0.36 |
| 38 | +10 | +0.37 |
| 39 | +10 | +0.38 |
| 40 | +10 | +0.37 |
| 41 | +9 | +0.36 |
| 42 | +9 | +0.37 |
| 43 | +9 | +0.38 |
| 44 | +8 | +0.37 |
| 45 | +8 | +0.35 |
| 46 | +8 | +0.36 |
| 47 | +8 | +0.37 |
| 48 | +8 | +0.35 |
| 49 | +7 | +0.33 |
| 50 | +7 | +0.34 |
| 51 | +7 | +0.35 |
| 52 | +6 | +0.31 |
| 53 | +6 | +0.33 |
| 54 | +6 | +0.32 |
| 56 | +6 | +0.30 |
| 58 | +5 | +0.28 |
| 60 | +4 | +0.26 |
| 62 | +4 | +0.24 |
| 64 | +4 | +0.22 |
| 66 | +3 | +0.19 |
| 68 | +2 | +0.17 |
| 70 | +2 | +0.14 |
| 72 | +2 | +0.14 |
| 76 | +1 | +0.07 |
| 80 | 0 | 0 |
| 84 | 0 | 0 |
| 88 | 0 | 0 |
| 92 | 0 | 0 |
| 96 | 0 | 0 |

**Règles**

*Lisibilité*

- Graisses — "In general, avoid light font weights." — typography › Ensuring legibility ; "prefer Regular, Medium, Semibold, or Bold font weights, and avoid Ultralight, Thin, and Light font weights, which can be difficult to see, especially when text is small" — typography › Ensuring legibility ; police fine personnalisée — "If you use a custom font with a thin weight, aim for larger than the recommended sizes to increase legibility." — typography › Ensuring legibility
- Contrôle — "Test legibility in different contexts." — typography › Ensuring legibility ; "consider using a larger type size, increasing contrast by modifying the text or background colors, or using typefaces designed for optimized legibility, like the system fonts" — typography › Ensuring legibility

*Hiérarchie*

- Mise en évidence — "Adjust font weight, size, and color as needed to emphasize important information and help people visualize hierarchy." — typography › Conveying hierarchy ; "Be sure to maintain the relative hierarchy and visual distinction of text elements when people adjust text sizes." — typography › Conveying hierarchy
- Nombre de polices — "Minimize the number of typefaces you use, even in a highly customized interface." — typography › Conveying hierarchy ; "Mixing too many different typefaces can obscure your information hierarchy and hinder readability" — typography › Conveying hierarchy
- Styles intégrés — "Consider using the built-in text styles." — typography › Using system fonts ; "The system-defined text styles give you a convenient and consistent way to convey your information hierarchy through font size and weight." — typography › Using system fonts
- Ajustement des styles — "Modify the built-in text styles if necessary." — typography › Using system fonts ; "the bold trait adds weight to text, letting you create another level of hierarchy" — typography › Using system fonts ; "You can also use symbolic traits to adjust leading if you need to improve readability or conserve space." — typography › Using system fonts
- Interlignage — "If you need to display three or more lines of text, avoid tight leading even in areas where height is limited." — typography › Using system fonts ; "when you display text in wide columns or long passages, more space between lines" — typography › Using system fonts
- Police système et symboles — "don’t embed system fonts in your app or game" — typography › Using system fonts ; "Because SF Symbols use equivalent weights, you can achieve precise weight matching between symbols and adjacent text, regardless of the size or style you choose." — typography › Using system fonts

*Polices personnalisées*

- Lisibilité — "Make sure custom fonts are legible." — typography › Using custom fonts ; "People need to be able to read your custom font easily at various viewing distances and under a variety of conditions." — typography › Using custom fonts
- Accessibilité — "System fonts automatically support Dynamic Type (where available) and respond when people turn on accessibility features, such as Bold Text." — typography › Using custom fonts ; "If you use a custom font, make sure it implements the same behaviors." — typography › Using custom fonts

*Dynamic Type (iOS, iPadOS, tvOS, visionOS, watchOS ; absent de macOS)*

- Définition et principe — "Dynamic Type is a system-level feature in iOS, iPadOS, tvOS, visionOS, and watchOS" — typography › Supporting Dynamic Type ; "Make sure your app’s layout adapts to all font sizes." — typography › Supporting Dynamic Type ; icônes — "Increase the size of meaningful interface icons as font size increases." — typography › Supporting Dynamic Type
- Troncature — "Keep text truncation to a minimum as font size increases." — typography › Supporting Dynamic Type ; "Avoid truncating text in scrollable regions unless people can open a separate view to read the rest of the content." — typography › Supporting Dynamic Type
- Disposition — "Reduce the number of columns when the font size increases to avoid truncation and enhance readability." — typography › Supporting Dynamic Type ; "consider using a stacked layout where text appears above secondary items" — typography › Supporting Dynamic Type
- Priorité au contenu — "Prioritize important content when responding to text-size changes." — typography › Conveying hierarchy ; "when people increase text size to read the content in a tabbed window, they don’t expect the tab titles to increase in size" — typography › Conveying hierarchy
- Hiérarchie stable — "Maintain a consistent information hierarchy regardless of the current font size." — typography › Supporting Dynamic Type ; "keep primary elements toward the top of a view even when the font size is very large" — typography › Supporting Dynamic Type

**Spécifique macOS**

- Police système — "SF Pro is the system font in macOS. NY is available for Mac apps built with Mac Catalyst. macOS doesn’t support Dynamic Type." — typography › Platform considerations › macOS
- Variantes dynamiques — "When necessary, use dynamic system font variants to match the text in standard controls." — typography › Platform considerations › macOS ; "Use the variants listed below to achieve a look that’s consistent with other apps on the platform." — typography › Platform considerations › macOS

| Dynamic font variant | API |
|---|---|
| Control content | AppKit/NSFont/controlContentFont(ofSize:) |
| Label | AppKit/NSFont/labelFont(ofSize:) |
| Menu | AppKit/NSFont/menuFont(ofSize:) |
| Menu bar | AppKit/NSFont/menuBarFont(ofSize:) |
| Message | AppKit/NSFont/messageFont(ofSize:) |
| Palette | AppKit/NSFont/paletteFont(ofSize:) |
| Title | AppKit/NSFont/titleBarFont(ofSize:) |
| Tool tips | AppKit/NSFont/toolTipsFont(ofSize:) |
| Document text (user) | AppKit/NSFont/userFont(ofSize:) |
| Monospaced document text (user fixed pitch) | AppKit/NSFont/userFixedPitchFont(ofSize:) |
| Bold system font | AppKit/NSFont/boldSystemFont(ofSize:) |
| System font | AppKit/NSFont/systemFont(ofSize:) |

- Portée : la HIG ne donne aucune taille, graisse ni interlignage pour ces variantes (non chiffré dans la HIG). Le tableau macOS tracking values est le seul tableau de tracking propre à macOS et la HIG ne nomme pas sa police ; le tableau SF Pro de la même page lui est identique, sauf aux tailles 52 et 53 où les valeurs en points sont inversées (+0.33 à 52 pt, +0.31 à 53 pt).

**À transposer dans un shell Linux**

Propositions dérivées des règles ci-dessus : la HIG ne prescrit aucun style pour les surfaces d'un bureau.

- Toutes les surfaces : une seule famille de police pour l'interface, graisses limitées à Regular, Medium, Semibold et Bold ; rien en Ultralight, Thin ou Light ; rien sous 10 pt.
- Barre et dock : un style unique pour l'horloge et les libellés d'onglets, choisi dans le tableau des styles ; les titres d'onglets ne grossissent pas quand l'utilisateur agrandit le texte ; un style de légende pour les infobulles du dock.
- Lanceur et recherche : résultats en Body (13 pt), titres de groupe en Headline (13 pt Bold) ou Title 3 (15 pt), compléments en Footnote ou Caption 1 (10 pt).
- Notifications : titre en Headline, texte en Body ou Callout ; une seule graisse de mise en valeur par rôle.
- Réglages : titre de page en Title 2 ou Title 3, libellés d'option en Body, explications en Footnote ou Caption 1 ; une graisse (Semibold ou Bold) pour marquer la sélection.
- Verrouillage : l'heure peut prendre Large Title (26 pt) ; appliquer à chaque corps le tracking du tableau, négatif à partir de 13 pt.
- Échelle de texte : offrir un agrandissement d'au moins 200 %, des rangées qui grandissent en hauteur, des éléments qui passent en colonne et des libellés sur plusieurs lignes plutôt que tronqués.

### Writing (`writing`)

**Rôle** — Faire des mots une part essentielle de l'expérience : voix, ton, libellés et messages, pensés aussi pour l'accessibilité et la localisation.

**Anatomie et cotes**

- Non chiffré dans la HIG : aucune longueur de libellé, aucun nombre de caractères ou de lignes ne sont imposés ; les seuls chiffres de la page figurent dans des exemples (mot de passe d'au moins 8 caractères).
- Non traités par la page : libellés des éléments de menu, ponctuation, textes de chargement, dates et heures. Les libellés donnés en exemple (Send, Get Started, Continue, Next, Done) s'écrivent sans point final, sans que la page en fasse une règle ; la description d'une image pour l'accessibilité relève de l'écriture — "describing an image for accessibility" — writing › Discussion

**Règles**

*Voix, ton et clarté*

- Voix — "Create a list of common terms, and reference that list to keep your language consistent." — writing › Getting started ; "Consistent language, along with a voice that reflects your app’s values, helps everything feel more cohesive." — writing › Getting started
- Ton — "Match your tone to the context." — writing › Getting started ; "Once you’ve established your app’s voice, vary your tone based on the situation." — writing › Getting started ; "Situational factors affect both what you say and how you display the text on the screen." — writing › Getting started
- Clarté — "Choose words that are easily understood and convey the right thing." — writing › Getting started ; "If you can use fewer words, do so." — writing › Getting started
- Universel — "Choose simple, plain language and write with accessibility and localization in mind, avoiding jargon and gendered terminology." — writing › Getting started
- Écran — "Pay attention to the order of elements on a screen, and put the most important information first." — writing › Best practices ; "If you’re trying to convey more than one idea, consider breaking up the text onto multiple screens" — writing › Best practices

*Libellés : boutons, liens, parcours, casse*

- Boutons et liens — "When labeling buttons and links, it’s almost always best to use a verb." — writing › Best practices ; "Active voice and clear labels help people navigate through your app from one step to the next, or from one screen to another." — writing › Best practices
- Sobriété — "Prioritize clarity and avoid the temptation to be too cute or clever with your labels." — writing › Best practices ; "For example, just saying “Send” often works better than “Let’s do it!”" — writing › Best practices
- Liens — "For links, avoid using “Click here” in favor of more descriptive words or phrases, such as “Learn more about UX Writing.”" — writing › Best practices ; "This is especially important for people using screen readers to access your app." — writing › Best practices
- Parcours en plusieurs étapes — "Give clear guidance and use consistent language throughout processes with multiple steps." — writing › Best practices ; "Begin with language like “Get Started” to indicate you’re starting a flow." — writing › Best practices ; "Make it clear when a flow is complete by using language like “Done.”" — writing › Best practices
- Suite du parcours — "You can use the button label to hint at the next step, or use terms like “Continue” or “Next,” but be consistent with what you choose." — writing › Best practices ; "Consistency builds familiarity, helping your app feel cohesive, intuitive, and thoughtfully designed." — writing › Best practices
- Casse — "Adopt capitalization rules that align with your app’s style, then apply them consistently." — writing › Best practices ; "Title case is generally considered formal, while sentence case is more casual." — writing › Best practices ; "Choose a style for each UI element type and use it consistently throughout your app" — writing › Best practices
- Composants — "have specific guidelines, how you format text reflects your app’s voice" — writing › Best practices ; la page ne donne aucune règle de ponctuation et ne traite pas de la casse en français.

*Messages : erreurs, champs, états vides*

- Erreurs — "It’s always best to help people avoid errors." — writing › Best practices ; "When an error message is necessary, display it as close to the problem as possible, avoid blame, and be clear about what someone can do to fix it." — writing › Best practices
- Formulation corrective — "For example, “That password is too short” isn’t as helpful as “Choose a password with at least 8 characters.”" — writing › Best practices ; "Interjections like “oops!” or “uh-oh” are typically unnecessary and can sound insincere." — writing › Best practices ; "If you find that language alone can’t address an error that’s likely to affect many people, use that as an opportunity to rethink the interaction." — writing › Best practices
- Pronoms — "Use possessive pronouns sparingly." — writing › Best practices ; "“Favorites” conveys the same message as “Your Favorites,” and is more succinct." — writing › Best practices ; "If you do use possessive pronouns, use them consistently throughout your app, and try not to switch perspectives." — writing › Best practices
- Sans pronom we — "because it may be unclear who the “we” in question refers to" — writing › Best practices ; "This is particularly problematic in error messages like “We’re having trouble loading this content.” Something like “Unable to load content” is much clearer." — writing › Best practices
- Champs — "label all fields clearly, and use hint or placeholder text so people know how to format the information" — writing › Best practices ; "Show errors right next to the field, and instruct people how to enter the information correctly, rather than scolding them for not following the rules." — writing › Best practices ; "“Use only letters for your name” is better than “Don’t use numbers or symbols.”" — writing › Best practices ; "Avoid robotic error messages with no helpful information, like “Invalid name.”" — writing › Best practices
- États vides — "Provide clear next steps on any blank screens." — writing › Best practices ; "An empty screen can be daunting if it isn’t obvious what to do next" — writing › Best practices ; "guide people on actions they can take, and give them a button or link to do so if possible" — writing › Best practices ; "Remember that empty states are usually temporary, so don’t show crucial information that could then disappear." — writing › Best practices

*Réglages, canal, appareil*

- Réglages — "Keep settings labels clear and simple." — writing › Best practices ; "Help people easily find the settings they need by labeling them as practically as possible." — writing › Best practices ; "If the setting label isn’t enough, add an explanation." — writing › Best practices ; "Describe what it does when turned on, and people can infer the opposite." — writing › Best practices
- Accès direct au réglage — "If you need to direct someone to a setting, provide a direct link or button, rather than trying to describe its location." — writing › Best practices
- Canal — "Choose the right delivery method." — writing › Best practices ; "consider the urgency and importance of the message" — writing › Best practices ; "Think about the context in which someone might see the message, whether it requires immediate action, and how much supporting information someone might need." — writing › Best practices
- Appareil — "Make sure you describe gestures correctly on each device" — writing › Best practices ; "not saying “click” for a touch device like iPhone or iPad where you mean “tap.”" — writing › Best practices

**Spécifique macOS**

- Aucune consigne propre à macOS — "No additional considerations for iOS, iPadOS, macOS, tvOS, visionOS, or watchOS." — writing › Platform considerations
- La page ne mentionne ni le Dock, ni la barre des menus, ni les fenêtres.
- Seule piste indirecte : nommer le geste réel de l'appareil, donc cliquer (click) sur un bureau à souris, et non toucher (tap).

**À transposer dans un shell Linux**

- Barre et menus de la barre : libellés courts, verbes pour les commandes, table de termes partagée avec le lanceur et les réglages.
- Dock : infobulles réduites au nom simple, sans pronom possessif.
- Lanceur et recherche : état sans résultat muni d'une prochaine étape et d'une action ; erreur de chargement sans pronom we (*Impossible de charger les applications* plutôt que *Nous n'arrivons pas à charger*).
- Notifications : canal choisi selon l'urgence et l'importance, ton adapté à la situation, aucune interjection.
- Réglages : libellés simples, explication de l'effet à l'activation, bouton direct vers le réglage concerné depuis toute autre surface.
- Verrouillage : erreur de mot de passe affichée près du champ, sans reproche, sous forme corrective (*Choisissez un mot de passe d'au moins 8 caractères*).
- Aperçu des espaces et widgets : noms et légendes sans possessif, casse uniforme ; une casse par type d'élément sur tout le shell, la HIG ne disant rien de la casse française.

### Accessibility (`accessibility`)

**Rôle** — Permettre à tous, quelles que soient leurs capacités et leur façon d'utiliser l'appareil, d'accéder à l'information et aux interactions.

**Anatomie et cotes**

- Tailles de texte macOS (13 pt par défaut, 10 pt minimum, comme sur la page Typography) — "Each platform has different default and minimum sizes for system-defined type styles to promote readability." — accessibility › Vision ; "If you’re using custom type styles, follow the recommended defaults." — accessibility › Vision
- Agrandissement — "Ideally, give people the option to enlarge text by at least 200 percent (or 140 percent in watchOS apps)." — accessibility › Vision
- Contraste minimal (valeurs du WCAG niveau AA retenues par Accessibility Inspector) — "uses the following values from WCAG Level AA as guidance in determining whether your app’s colors have an acceptable contrast" — accessibility › Vision

| Text size | Text weight | Minimum contrast ratio |
|---|---|---|
| Up to 17 pts | All | 4.5:1 |
| 18 pts | All | 3:1 |
| All | Bold | 3:1 |

- Contrôles macOS (28x28 pt par défaut, 20x20 pt minimum) — "Strive to meet the recommended minimum control size for each platform to ensure controls and menus are comfortable for all when tapping and clicking." — accessibility › Mobility

| Platform | Default control size | Minimum control size |
|---|---|---|
| macOS | 28x28 pt | 20x20 pt |

- Espacement — "In general, it works well to add about 12 points of padding around elements that include a bezel." — accessibility › Mobility ; "For elements without a bezel, about 24 points of padding works well around the element’s visible edges." — accessibility › Mobility
- Non chiffré dans la HIG : visibilité du focus clavier, durée des animations et des affichages temporisés. La page n'a pas de section sur les textes alternatifs (alt text) : elle demande de décrire l'interface et le contenu pour VoiceOver et renvoie à la page VoiceOver.

**Règles**

*Principes*

- Définition — "An accessible interface allows people to experience your app or game regardless of their capabilities or how they use their devices." — accessibility › Discussion
- Intuitive — "Your interface uses familiar and consistent interactions that make tasks straightforward to perform." — accessibility › Discussion ; perceptible — "Your interface doesn’t rely on any single method to convey information." — accessibility › Discussion
- Adaptable — "Your interface adapts to how people want to use their device, whether by supporting system accessibility features or letting people personalize settings." — accessibility › Discussion ; audit — "As you design your app, audit the accessibility of your interface." — accessibility › Discussion

*Vision*

- Texte — "Support larger text sizes." — accessibility › Vision ; "Make sure people can adjust the size of your text or icons to make them more legible, visible, and comfortable to read." — accessibility › Vision ; "Your interface can support font size enlargement either through custom UI, or by adopting Dynamic Type." — accessibility › Vision
- Graisse — "Bear in mind that font weight can also impact how easy text is to read." — accessibility › Vision ; "If you’re using a custom font with a thin weight, aim for larger than the recommended sizes to increase legibility." — accessibility › Vision
- Contraste — "Strive to meet color contrast minimum standards." — accessibility › Vision ; "To ensure all information in your app is legible, it’s important that there’s enough contrast between foreground text and icons and background colors." — accessibility › Vision
- Mesure — "Use standard contrast calculators to ensure your UI meets acceptable levels." — accessibility › Vision ; "the Accessible Perceptual Contrast Algorithm (APCA)" — accessibility › Vision
- Augmenter le contraste (Increase Contrast) et Mode sombre (Dark Mode) — "ensure it at least provides a higher contrast color scheme when the system setting Increase Contrast is turned on" — accessibility › Vision ; "make sure to check the minimum contrast in both light and dark appearances" — accessibility › Vision
- Couleurs système — "Prefer system-defined colors." — accessibility › Vision ; "automatically adapt when people adjust their color preferences" — accessibility › Vision
- Pas la couleur seule — "Convey information with more than color alone." — accessibility › Vision ; "Offer visual indicators, like distinct shapes or icons, in addition to color to help people perceive differences in function and changes in state." — accessibility › Vision ; "people who are color blind may have particular difficulty with pairings such as red-green and blue-orange" — accessibility › Vision
- Lecteur d'écran VoiceOver — "Describe your app’s interface and content for VoiceOver." — accessibility › Vision ; "VoiceOver is a screen reader that lets people experience your app’s interface without needing to see the screen." — accessibility › Vision

*Audition*

- Pas le son seul — "It’s important that dialogue and crucial information about your app or game isn’t communicated through audio alone." — accessibility › Hearing
- Index visuels — "Augment audio cues with visual cues." — accessibility › Hearing ; "When using audio to guide people towards a specific action, also add in visual indicators that point to where you want people to interact." — accessibility › Hearing

*Mobilité*

- Taille et espacement des contrôles — "Offer sufficiently sized controls." — accessibility › Mobility ; "Controls that are too small are hard for many people to interact with and select." — accessibility › Mobility ; "Consider spacing between controls as important as size." — accessibility › Mobility ; "Include enough padding between elements to reduce the chance that someone taps the wrong control." — accessibility › Mobility
- Gestes — "Support simple gestures for common interactions." — accessibility › Mobility ; "For interactions people do frequently in your app or game, use the simplest gesture possible" — accessibility › Mobility ; "avoid custom multifinger and multihand gestures" — accessibility › Mobility
- Alternatives aux gestes — "Offer alternatives to gestures." — accessibility › Mobility ; "Make sure your UI’s core functionality is accessible through more than one type of physical interaction." — accessibility › Mobility ; "if you use a swipe gesture to dismiss a view, also make a button available so people can tap or use an assistive device" — accessibility › Mobility
- Contrôle vocal (Voice Control) et technologies d'assistance — "Let people use Voice Control to give guidance and enter information verbally." — accessibility › Mobility ; "To ensure a smooth experience, label interface elements appropriately." — accessibility › Mobility ; "Conduct testing and verify that your app or game supports these technologies, and that your interface elements are appropriately labeled to ensure a great experience." — accessibility › Mobility

*Parole et clavier*

- Clavier seul — "Let people use the keyboard alone to navigate and interact with your app." — accessibility › Speech ; "People can turn on Full Keyboard Access to navigate apps using their physical keyboard." — accessibility › Speech
- Raccourcis système — "Avoid overriding system-defined keyboard shortcuts and evaluate your app to ensure it works well with Full Keyboard Access." — accessibility › Speech
- Contrôle de sélection (Switch Control) — "Support Switch Control." — accessibility › Speech ; "People can perform actions like selecting, tapping, typing, and drawing when your app or game supports the ability to navigate using Switch Control." — accessibility › Speech

*Cognition*

- Simplicité — "Keep actions simple and intuitive." — accessibility › Cognitive ; "Prefer system gestures and behaviors people are already familiar with over creating custom gestures people must learn and retain." — accessibility › Cognitive
- Affichages temporisés — "Minimize use of time-boxed interface elements." — accessibility › Cognitive ; "Views and controls that auto-dismiss on a timer can be problematic for people who need longer to process information" — accessibility › Cognitive ; "Prefer dismissing views with an explicit action." — accessibility › Cognitive
- Lecture et flashes — "Avoid autoplaying audio and video content without also providing controls to start and stop it." — accessibility › Cognitive ; "If your app supports video playback, ensure that it responds appropriately to the Dim Flashing Lights setting." — accessibility › Cognitive
- Réduire les animations (Reduce Motion) — "Be cautious with fast-moving and blinking animations." — accessibility › Cognitive ; "When this setting is active, ensure your app or game responds by reducing automatic and repetitive animations, including zooming, scaling, and peripheral motion." — accessibility › Cognitive
- Bonnes pratiques de mouvement — "Tightening animation springs to reduce bounce effects" — accessibility › Cognitive ; "Tracking animations directly with people’s gestures" — accessibility › Cognitive ; "Avoiding animating depth changes in z-axis layers" — accessibility › Cognitive ; "Replacing transitions in x-, y-, and z-axes with fades to avoid motion" — accessibility › Cognitive ; "Avoiding animating into and out of blurs" — accessibility › Cognitive
- Assistive Access (iOS et iPadOS seulement) — "Always ask for confirmation twice whenever people perform an action that’s difficult to recover from, such a deleting a file." — accessibility › Cognitive

**Spécifique macOS**

- Aucun bloc macOS — "No additional considerations for iOS, iPadOS, macOS, tvOS, or watchOS." — accessibility › Platform considerations
- Conséquence de l'absence de Dynamic Type — "macOS doesn’t support Dynamic Type" — typography › Platform considerations › macOS ; avec "Your interface can support font size enlargement either through custom UI, or by adopting Dynamic Type." — accessibility › Vision, l'agrandissement du texte sur Mac passe donc par une interface propre à l'app.
- Valeurs macOS de la page : 13 pt et 10 pt pour le texte, 28x28 pt et 20x20 pt pour les contrôles. Les réglages cités (Increase Contrast, Dark Mode, Reduce Motion, Bold Text, Dim Flashing Lights, Full Keyboard Access, VoiceOver, Voice Control, Switch Control) ne sont rattachés à aucune plateforme ; la page ne dit pas lesquels existent sur macOS.

**À transposer dans un shell Linux**

- Verrouillage : champ et boutons accessibles au clavier seul, aucune fermeture sur minuteur, cibles d'au moins 20x20 pt, contraste vérifié sur l'image de fond, animations d'entrée réduites quand la réduction du mouvement est active.
- Notifications : signal visuel en plus du son, contenu retrouvable dans un centre persistant plutôt qu'un affichage purement temporisé.
- Barre et dock : cibles de 28x28 pt par défaut et jamais sous 20x20 pt, espacement d'environ 12 points entre éléments à bordure et 24 points entre éléments sans bordure, raccourcis système jamais écrasés, état signalé par une forme ou une icône en plus de la couleur.
- Lanceur et recherche : parcours et validation au clavier seul, libellé accessible pour chaque résultat, contraste vérifié sur le fond translucide.
- Réglages : offrir texte plus grand (au moins 200 %), contraste élevé et réduction du mouvement, car le shell doit fournir lui-même les réglages que macOS assure pour ses apps.
- Widgets : aucune information portée par la seule couleur.
- Aperçu des espaces : remplacer les zooms et mises à l'échelle par des fondus quand la réduction du mouvement est active.

### Valeurs de référence

| Propriété | Valeur | Source (slug › section) |
|---|---|---|
| Taille de texte par défaut, macOS | 13 pt | typography › Ensuring legibility |
| Taille de texte minimale, macOS | 10 pt | typography › Ensuring legibility |
| Styles de texte macOS | 11 styles, de Large Title (26 pt, interlignage 32) à Caption (10 pt, interlignage 13) ; tableau complet plus haut | typography › Specifications › macOS built-in text styles |
| Tracking macOS | +41 (1/1000 em), +0.24 pt à 6 pt ; -6 (1/1000 em), -0.08 pt à 13 pt ; 0 (1/1000 em), 0 de 80 à 96 pt ; tableau complet plus haut | typography › Specifications › Tracking values › macOS tracking values |
| Graisses conseillées | Regular, Medium, Semibold, Bold | typography › Ensuring legibility |
| Contraste minimal (WCAG niveau AA) | 4.5:1 : texte jusqu'à 17 pts, toute graisse ; 3:1 : texte de 18 pts, toute graisse ; 3:1 : texte en gras, toute taille | accessibility › Vision |
| Agrandissement du texte recommandé | au moins 200 percent | accessibility › Vision |
| Taille de contrôle macOS | 28x28 pt par défaut ; 20x20 pt minimum | accessibility › Mobility |
| Espacement autour des éléments | environ 12 points avec bezel ; environ 24 points sans bezel | accessibility › Mobility |
| Contenu de fenêtre à éviter sur macOS (non chiffré) | contrôles et informations critiques en bas ; contenu derrière le logement de la caméra | layout › Platform considerations › macOS |

## Seconde partie — hyprdesk aujourd'hui, écarts et actions

### Ce que hyprdesk fait aujourd'hui
| Surface | Ce qui existe | Valeurs clés | Preuves |
| --- | --- | --- | --- |
| Jetons typographiques | Six rôles (légende, corps, libellé, titre, affichage, mesure) résolus par le pack, l'écran et l'échelle utilisateur ; graisse et interligne de thème | caption 11, body 13, label 12, title 17, display 34, metric 34 ; weight 400, title_weight 700, line_height 1,3, tabular_numbers faux ; aucun des sept packs livrés ne fixe de clé de typographie | `src/ui/tokens.rs:231-269` ; `src/theme_engine/design.rs:101-153` |
| Échelle de texte | Curseur global et curseur par écran | global 75 à 200 % ; par écran 50 à 200 % ; plancher de calcul 6 px | `src/ui/plugins/theme.rs:711` ; `src/ui/plugins/screens.rs:714` ; `src/ui/tokens.rs:130, 242, 263` |
| Barre | Libellés de commande et horloge en Medium et Semibold, titres tronqués au nombre de caractères | libellé 12 px ; troncature 18 ou 36 caractères | `src/ui/plugins/bar/surface.rs:144-151, 868-869, 1855-1857, 1870, 1930` |
| Cartes et menus de la barre | Entrées Normal, valeurs Medium, titres Semibold ; demi-points | en-tête de menu 11,5 px ; entrée 13,5 px ; cartes 12,5 et 13,5 px | `src/ui/menu.rs:241-247, 313-320` ; `src/ui/plugins/bar/popover.rs:1553-1565, 1768-1769, 1860` |
| Bulles de notification | Titre gras, corps sur deux lignes au plus, interligne du thème ; historique application → résumé → corps | titre 14 px Bold ; corps 12 px ; application 11 px | `src/ui/toasts.rs:533-547, 632-646` |
| Lanceur | Grille et carte centrée, colonnes et largeur réglées, hauteur plafonnée ; état sans résultat réduit à une ligne | largeur = réglage (680 px dans la session) ; état vide 14 px estompé | `src/ui/plugins/launcher.rs:185-187` ; `src/ui/plugins/launcher/grid.rs:62-64, 93-97, 411-417, 545-551` |
| Écran Réglages | Tiroir latéral ; navigation 166 px, réduite à 46 px sous 640 px ; en-tête sur deux lignes sous 880 px | titre de page 27 px Bold ; surtitre 11 px Bold ; section 13 px Medium ; description 12 px ; colonnes de 240 à 320 px | `src/ui/panel.rs:622, 754-755, 782-783, 918, 961-971` ; `src/ui/hyprset.rs:259-262, 324-328` |
| Dialogue de problème | Titre, explication, boutons « Fermer », « Copier », « Demander à LYRA » | titre 20 px ; texte 12 px ; boutons 14 px | `src/ui/problem.rs:169-207` |
| Widgets de bureau | Ancrage neuf positions, repoussement avec 16 px d'écart ; horloge de bureau en Light | marge basse fixe 120 px ; horloge 56 px | `src/ui/desk_layout.rs:22-95` ; `src/ui/widget_view.rs:45-48, 302-316` |
| Écriture | Catalogue unique : clé française, valeur anglaise (pas de `fr.toml`) | 2 268 chaînes ; 195 lignes à ellipse « … », 0 à « ... » | `locales/en.toml` (4 840 lignes) |

### Écarts
| # | Composant | Nature | Force | Règle HIG (slug › section) | Réel dans hyprdesk (preuve) | Action proposée | Effort | Confiance |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| TYP-01 | Typographie — plancher de taille de texte | ÉCART | Cote et impératif : « Follow the recommended default and minimum text sizes for each platform » ; macOS 13 pt par défaut, 10 pt minimum | typography › Ensuring legibility | `ts` et `role` bornent le résultat à `.max(6.0)` (`src/ui/tokens.rs:242, 263`). Le curseur global descend à 75 % (`src/ui/plugins/theme.rs:711`, borne `tokens.rs:130`) : `ts(11.0)` rend 8,25 px. Le curseur par écran descend à 50 % (`src/ui/plugins/screens.rs:714`) : `ts(12.0)` rend 6 px. Falsifié : aucun pack livré ni réglage ne relève ce plancher. | Remplacer le plancher 6.0 par 10.0 px dans `ts`, `ts_label` et `ts_metric`. | S | Haute |
| TYP-02 | Écriture — état sans résultat du lanceur | ÉCART | Impératif : « Provide clear next steps on any blank screens. » ; « give them a button or link to do so if possible » | writing › Best practices | Grille et carte centrée n'affichent qu'une ligne estompée, « Aucune application ne correspond. », sans étape ni bouton (`src/ui/plugins/launcher/grid.rs:411-417, 545-551`). Falsifié : les puces de catégorie restent au-dessus, mais rien ne dit quoi faire ; aucune entrée de repli dans `results`. | Faire de cet état un état guidé : une phrase qui dit quoi faire (vérifier la saisie, changer de catégorie) et un bouton qui agit (par exemple « Effacer la recherche »). | S | Haute |
| TYP-03 | Écriture — renommage d'une icône du bureau | ÉCART | Impératif : « Avoid robotic error messages with no helpful information, like “Invalid name.” » ; « be clear about what someone can do to fix it » | writing › Best practices | `rename` renvoie `tr("Nom invalide.")`, soit exactement « Invalid name. » en anglais (`src/ui/context.rs:210, 214` ; `locales/en.toml:1634`), affiché par `menu.error` (`context.rs:1304-1310`). La règle réelle est connue du code : non vide, sans `/`, ni `.` ni `..` (`context.rs:202-205`). | Remplacer par la forme corrective qui reprend la règle : « Choisissez un nom non vide, sans barre oblique (/) » (en : « Choose a name that isn’t empty and contains no slash »). | S | Haute |
| TYP-04 | Typographie — échelle des corps | PISTE | Recommandation : « Consider using the built-in text styles. » ; tableau de 11 styles, de 10 à 26 pt | typography › Using system fonts ; typography › Specifications | 23 corps distincts passent par `ts()` dans `src/ui` : 9, 10, 11, 11,5, 12, 12,5, 13, 13,5, 14, 15, 16, 18, 19, 20, 22, 24, 25, 26, 27, 28, 32, 36, 96. Deux corps de base coexistent (12 px : 625 usages ; 13 px : 343). Demi-points dans `src/ui/menu.rs:245, 318, 326`, `src/ui/plugins/bar/popover.rs:1559-1768`, `src/ui/controls.rs:126`, `src/ui/chat.rs:1488`. Titre de page à 27 px (`src/ui/panel.rs:966`), entre Title 1 (22) et Large Title (26). | Ramener l'échelle aux seuls corps du tableau (11, 12, 13, 15, 17, 22, 26) et supprimer les demi-points. | M | Moyenne |
| TYP-05 | Typographie — graisse des titres | PISTE | Recommandation : « Consider using the built-in text styles. » ; Title 1, 2 et 3 en Regular, Bold en variante mise en valeur ; Bold dès 13 pt seulement (Headline) | typography › Using system fonts ; typography › Specifications | `title_font()` vaut `title_weight`, 700 par défaut (`src/ui/mod.rs:3426-3428` ; `src/theme_engine/design.rs:150`), sans surcharge des packs. Il s'applique à 11, 12, 13, 16 et 27 px dans les réglages (`src/ui/panel.rs:664-665, 782-783, 834-835, 961-967, 1138`), 27 fois dans `wizard.rs`, 9 dans `panel.rs`, 7 dans le centre de contrôle Continuité. `DESIGN.md:7` demande « poids normal, medium et semibold pour la hiérarchie ». Falsifié : la barre proprement dite suit bien cette hiérarchie (`src/ui/menu.rs:247, 313`). | Abaisser `title_weight` par défaut à 600 (semibold) et réserver Bold au titre de page. | S | Moyenne |
| TYP-06 | Écriture — casse des états | ÉCART | Impératif : « Choose a style for each UI element type and use it consistently throughout your app » | writing › Best practices | Le même état s'écrit avec deux casses : pastille « Actif » (`src/ui/overview.rs:928`, en « Active ») ; badges « actif », « coupé », « intégré » en minuscules (`src/ui/plugins/plugins.rs:181-188`, en « on », « off », « built-in » ; `locales/en.toml:2450, 401, 636`) ; « Désactivé » (`src/ui/plugins/keys.rs:1019`, en « Off ») ; « installé » (`src/ui/plugins/apptheme.rs:134`). | Fixer la majuscule initiale pour tout état affiché seul et l'appliquer aux clés minuscules (actif, coupé, intégré, installé, arrêté, connecté). | S | Haute |
| TYP-07 | Typographie — corps sous le minimum | ÉCART | Cote : taille minimale 10 pt | typography › Ensuring legibility | Deux textes à `ts(9.0)` : nom de l'espace au-dessus de la miniature du sélecteur (`src/ui/plugins/bar/style/selector.rs:80-81`) et jours du graphique d'usage myMFA (`src/ui/mymfa_fenetre.rs:2437-2438`). Falsifié : ce sont les deux seules occurrences sous 10 dans tout `src/ui`. | Passer ces deux corps à `ts(10.0)`. | S | Haute |
| TYP-08 | Disposition — texte agrandi dans les réglages | PISTE | Recommandation : « Be prepared for text-size changes. » ; « table rows or other containers may need to grow in height so that text isn’t cropped or doesn’t overlap other content » | layout › Adaptability | Tout le texte tient sur une ligne : 43 `Wrapping::None` explicites et 3 `Wrapping::WordOrGlyph` dans `src/ui` (`chat.rs:1490`, `plugins/keys.rs:1013`, `context.rs:1795`). Les lignes de réglage utilisent des colonnes fixes de 240 à 320 px (`src/ui/hyprset.rs:259, 328, 367, 680, 691`) alors que l'échelle de texte atteint 200 % (`src/ui/tokens.rs:242`). | Faire envelopper libellés et descriptions des réglages (`WordOrGlyph`) pour que les rangées grandissent en hauteur. | M | Moyenne |
| TYP-09 | Typographie — barre et échelle de texte | PISTE | Recommandation : « when people increase text size to read the content in a tabbed window, they don’t expect the tab titles to increase in size » | typography › Conveying hierarchy | `ts_label` et `ts_metric` multiplient par l'échelle utilisateur (`src/ui/tokens.rs:252-264`) ; l'horloge et les libellés de la barre les utilisent (`src/ui/plugins/bar/surface.rs:868, 1855-1857, 1870`). La hauteur de la barre, elle, n'en dépend pas (`surface.rs:967-981`). | Soustraire les rôles `label` et `metric` de la barre à l'échelle utilisateur : seul le contenu grossit. | S | Moyenne |
| TYP-10 | Écriture — boutons du conflit de raccourcis | PISTE | Recommandation : « Prioritize clarity and avoid the temptation to be too cute or clever with your labels. » ; « it’s almost always best to use a verb » | writing › Best practices | « hyprdesk le prend » (en « hyprdesk takes it ») aux côtés de « Rendre » (en « Give back ») et « Désactiver plutôt » (`src/ui/plugins/keys.rs:1600, 1612, 1777, 1186` ; `locales/en.toml:1980, 1979`). | Réécrire ces trois libellés à la forme verbe plus complément, sans pronom (« Donner à hyprdesk », « Rendre à l'application », « Désactiver le raccourci »). | S | Haute |
| TYP-11 | Écriture — pronoms possessifs | PISTE | Recommandation : « Use possessive pronouns sparingly. » ; « “Favorites” conveys the same message as “Your Favorites,” and is more succinct. » | writing › Best practices | « Mes thèmes » / « My themes » (`locales/en.toml:752`, repris lignes 111, 491, 965, 1019) ; « Vos courbes » (3577) ; « Votre assistante du bureau » (2381) ; « Créer votre coffre myMFA » (2242) ; « Un espace à vous » (1052). | Retirer le possessif des libellés de navigation (« Mes thèmes » en « Thèmes personnels », « Vos courbes » en « Courbes personnelles »). | M | Haute |
| TYP-12 | Écriture — fin de parcours | PISTE | Recommandation : « Make it clear when a flow is complete by using language like “Done.” » ; « be consistent with what you choose » | writing › Best practices | Deux libellés de fin : « Terminé » sur le dialogue de conflit de raccourcis (`src/ui/key_conflict.rs:545`), « Terminer » sur la visite (`src/ui/tour.rs:574`) et sur Panorama (`src/ui/plugins/panorama/surface/dashboard.rs:261`). | Un seul libellé de fin de parcours : « Terminé ». | S | Haute |
| TYP-13 | Écriture — échec sans message | ÉCART | Impératif : « be clear about what someone can do to fix it » ; « Avoid robotic error messages with no helpful information » | writing › Best practices | « Échec, sans message. » est renvoyé quand une commande échoue sans stderr (`src/updates.rs:326`, `src/gpu.rs:816`, `src/greeter_setup.rs:741`) et s'affiche tel quel via `opt_error` (`src/ui/plugins/plugins.rs:899-904`). Falsifié : la mise à jour du système, elle, a un rapport complet (`locales/en.toml:1424`) ; le modèle existe donc. | Remplacer par un message qui nomme l'opération, le code de sortie et l'issue (copier le journal, demander à LYRA), sur le modèle du rapport de mise à jour. | S | Moyenne |
| TYP-14 | Écriture — vocabulaire des états | PISTE | Recommandation : « Create a list of common terms, and reference that list to keep your language consistent. » | writing › Getting started | Trois mots pour l'état arrêté : « coupé » (`src/ui/plugins/plugins.rs:184`, en « off »), « Désactivé » (`src/ui/plugins/keys.rs:1019`, en « Off »), « arrêté » (`src/ui/plugins/desktop.rs:530`, en « stopped »). Deux pour l'état en marche : « actif » et « Actif ». | Tenir une table des termes d'état (un mot par état, dans les deux langues) et l'appliquer à ces libellés. | S | Haute |
| TYP-15 | Écriture — ellipse des commandes | PISTE | Recommandation de cohérence : « Create a list of common terms, and reference that list to keep your language consistent. » La HIG est muette sur la ponctuation : à peser | writing › Getting started | La même commande s'écrit de deux façons : « Créer le coffre… » (`src/ui/plugins/passwords.rs:174, 310`) et « Créer le coffre » (`src/ui/mymfa.rs:776` ; `locales/en.toml:2141` et `2240`). « Choisir les fonds d'écran » ouvre un choix sans ellipse (`src/ui/plugins/theme.rs:807`). Falsifié : le glyphe, lui, est uniforme (195 lignes à « … », 0 à « ... »). | Retenir une règle (ellipse quand la commande ouvre une fenêtre ou un choix) et l'appliquer aux commandes qui y échappent. | S | Moyenne |
| TYP-16 | Disposition — largeur du lanceur | PISTE | Recommandation : « Design a layout that adapts gracefully and consistently. » ; « Determine layout based on size classes, not device type or orientation. » | layout › Adaptability ; layout › Size classes | La carte centrée prend la largeur réglée telle quelle (`src/ui/plugins/launcher/grid.rs:62-64`, appliquée `610-617`), sans borne à la largeur de l'écran ; le nombre de colonnes est un réglage (`src/ui/plugins/launcher.rs:185-187`). Seule la hauteur est plafonnée (`grid.rs:93-97`). | Borner la largeur de la carte, et le nombre de colonnes qui en découle, à l'espace disponible sur l'écran. | M | Moyenne |
| TYP-17 | Disposition — marge basse des widgets | PISTE | Recommandation : « Design a layout that adapts gracefully and consistently », « respecting system-defined safe areas, margins, and guides (where available) » | layout › Adaptability | Tout widget ancré en bas reçoit une marge inférieure constante de 120 px (`src/ui/widget_view.rs:310`). Le dock ne réserve aucune zone (`src/ui/plugins/dock/surface.rs:1364`, `exclusive_zone: Some(-1)`) alors que la barre réserve la sienne (`src/ui/plugins/bar/surface.rs:655`). Falsifié : `DESIGN.md:16` impose « garder les widgets bas au-dessus du dock », sans mesurer le dock. | Calculer cette marge à partir de la hauteur rendue du dock de l'écran. | S | Moyenne |
| TYP-18 | Disposition — ajustement du fond d'écran | PISTE | Impératif formulé pour l'œuvre affichée : « don’t change the aspect ratio of the artwork; instead, scale it so that it fills the screen completely » ; réglage explicite de l'utilisateur, à peser | layout › Adaptability | Le réglage des fonds propose « Étirer » (`src/ui/plugins/wallpaper.rs:526-537`), rendu par `ContentFit::Fill` (`src/ui/tokens.rs:615`). Falsifié : le défaut est bien `Cover` (`tokens.rs:610, 613`). | Retirer « Étirer » des ajustements proposés et garder couvrir, ajuster, centrer. | S | Moyenne |
| TYP-19 | Typographie — rôle infobulle | ABSENT | Recommandation : « When necessary, use dynamic system font variants to match the text in standard controls. » ; tableau des variantes : Menu bar, Menu, Tool tips, Title | typography › Platform considerations › macOS | Les infobulles du dock prennent le corps du texte courant, 13 px (`src/ui/plugins/dock/surface.rs:526, 549`) ou 12 px (`:778`). Falsifié : `Typography` n'a que six rôles, sans infobulle ni barre de menus (`src/theme_engine/design.rs:101-118`). | Ajouter un rôle `tooltip` aux jetons, sur le corps de légende (11 px), et l'utiliser pour les infobulles du dock. | S | Moyenne |

IDENTITÉ — à signaler, jamais à corriger :
- Police Mindlog 0.7 plutôt que SF Pro (`DESIGN.md:7`, `design-plans/barre-continuite.md:31`).
- Espacement base 4 px et rayons 10, 20, 24 (`DESIGN.md:9`) : la HIG ne chiffre aucune marge ni grille (layout › Discussion).
- Barre Continuité « fond bord à bord » et barre Graphite de 48 px à boutons de rayon 4 (`DESIGN.md:17, 19`) ; lanceur de Convex sortant du bas, recherche sous les résultats (`DESIGN.md:19`).

Sans objet : les deux consignes macOS « Avoid placing controls or critical information at the bottom of a window » et « Avoid displaying content behind the camera housing » (layout › Platform considerations › macOS) ne s'appliquent pas à des surfaces de calque ancrées à l'écran, qui ne sortent jamais de l'écran et n'ont pas de caméra.

### Ce qui est déjà conforme
- Une seule famille pour toute l'interface (« Minimize the number of typefaces you use ») | `src/ui/mod.rs:3430-3436`, `DESIGN.md:7`.
- Graisses Regular, Medium, Semibold dominent (« prefer Regular, Medium, Semibold, or Bold ») | 21 `Weight::Semibold`, 9 `Medium`, 5 `Normal`, 3 `Bold` explicites dans `src/ui` ; Light seulement sur l'horloge de bureau à 56 px (`src/ui/widget_view.rs:47`), ce que permet « aim for larger than the recommended sizes ».
- Corps par défaut 13 (« Default size 13 pt ») | `src/theme_engine/design.rs:124, 144`.
- Agrandissement du texte jusqu'à 200 %, global et par écran | `src/ui/plugins/theme.rs:711`, `src/ui/plugins/screens.rs:714`.
- Interligne du thème sur les corps de notification (« avoid tight leading ») | `src/ui/toasts.rs:546, 639`, jeton 1,3 (`src/theme_engine/design.rs:151`).
- Casse de phrase uniforme dans les deux langues (« Choose a style for each UI element type ») | catalogue : 1 704 chaînes françaises en casse de phrase, les 23 « casse de titre » sont des noms propres ; 380 anglaises, 1 (« Wi-Fi »).
- Boutons à l'infinitif en français, à la base verbale en anglais (« almost always best to use a verb ») | Fermer, Copier, Annuler, Enregistrer, Appliquer, Ouvrir le dossier (`src/ui/problem.rs:190-201` ; `locales/en.toml:1520, 499, 633, 845`).
- Erreurs déjà correctives, sans blâme ni interjection | « Le lecteur est plein : supprimez une empreinte d'abord. » (`locales/en.toml:1717`) ; 0 interjection dans le catalogue ; 0 « we » dans une erreur.
- Réglages : libellé court et explication sous le contrôle (« If the setting label isn’t enough, add an explanation ») | `src/ui/plugins/bar.rs:1372-1377`, `src/ui/plugins/desktop.rs:203`.
- Ellipse sur les commandes qui ouvrent une fenêtre | « Réglages Bluetooth… », « Renommer… », « Redémarrer… » (`locales/en.toml:938, 1639, 908`).
- Anglais sans marque typographique française | 0 guillemet « » dans les valeurs de `locales/en.toml`.
- Réglages : quantité affichée variable, fonctions constantes (« Don’t change your app’s functionality based on the space it occupies ») | navigation réduite aux icônes sous 640 px (`src/ui/panel.rs:622`), en-tête sur deux lignes sous 880 px (`:918`), seuils fondés sur la largeur du panneau.
- Barre : compacité automatique et débordement (« expose functionality that might otherwise be grouped into an overflow menu ») | `src/ui/plugins/bar/surface.rs:1603-1606`, « Autres commandes » (`DESIGN.md:18`).
- Menus et lanceur bornés à la hauteur de l'écran | `src/ui/plugins/launcher/grid.rs:93-97`, `DESIGN.md:16, 43`.
- Zones sûres : les notifications s'écartent de la barre de sa hauteur ; la barre réserve sa zone | `src/ui/toasts.rs:448-469`, `src/ui/plugins/bar/surface.rs:655`.
- Fond d'écran : proportions conservées par défaut | `src/ui/tokens.rs:610, 613`.
- Lecture : titre puis corps dans les notifications | `src/ui/toasts.rs:533-547`.

### À vérifier sur un rendu
- Barre à 200 % d'échelle de texte : horloge et libellés dans 36 px (`src/ui/plugins/bar/surface.rs:967-981`).
- Réglages à 200 % et au gabarit le plus étroit : colonnes fixes de `src/ui/hyprset.rs`, descriptions sur une ligne.
- Hiérarchie perçue du centre de contrôle : titre et corps à 1 px d'écart (13,5 contre 12,5 ; `src/ui/plugins/bar/popover.rs:1768, 1559`).
- Lisibilité de Mindlog en Bold à 11 px (`src/ui/panel.rs:782-783, 961-963`).
- Approche de Mindlog aux grands corps (22 à 36 px) : aucun réglage d'approche dans le code (0 `letter_spacing` dans `src/ui`), alors que la HIG donne un tableau (négatif de 13 à 23 pt).
- Chiffres de l'horloge : `tabular_numbers` est faux par défaut (`src/theme_engine/design.rs:152`) et aucun pack livré ne l'active, alors que `design-plans/barre-continuite.md:38` demande une « heure tabulaire » ; vérifier si les chiffres de Mindlog sont déjà à chasse fixe, sinon l'horloge tremble au changement de minute.
- Corps de notification longs : deux lignes gardées, sans retournement à la largeur (`src/ui/toasts.rs:541`) ; historique sans retournement (`:632-640`).
- Notifications dans les coins bas : la marge ne compte que la barre, pas le dock (`src/ui/toasts.rs:453-461`), dont la zone n'est pas réservée.
- Réglages : contenu défilant dans la même surface presque opaque que la navigation (`src/ui/panel.rs:893-897, 1025`), sans fondu de bord ; la HIG préfère un « scroll edge effect » à « a solid or semi-opaque background color beneath controls » (layout › Visual hierarchy), la matière opaque étant une identité (`DESIGN.md:6`).
- Troncature au nombre de caractères, indépendante de la largeur : barre 18 ou 36 caractères (`src/ui/plugins/bar/surface.rs:144-151, 1930`), aperçu 22 caractères (`src/ui/overview.rs:981`).
