# Composants : expériences système — macOS d'après la HIG, face à hyprdesk

> Proposition de design, rien d'implémenté. Plan d'ensemble et priorités : `plan-d-action.md` ; méthode et périmètre : `README.md`.

> Référentiel : Human Interface Guidelines d'Apple, export Markdown officiel relevé le 23 novembre 2026. Code : commit `00f6d69` (arbre de travail du 23 novembre 2026, avec des modifications non commitées).

## Première partie — Spécification d'après la HIG

### Résumé

- La famille « expériences système » (System experiences) compte onze pages. Quatre décrivent des surfaces utiles à un bureau : notifications, widgets, commandes du Centre de contrôle, barre d'état. Les autres concernent l'iPhone, l'Apple Watch ou l'Apple TV, hormis les raccourcis d'app, les activités en direct et les extraits.
- Trois pages portent du contenu propre à macOS : `widgets` (bureau et Centre de notifications), `live-activities` (barre des menus d'un Mac appairé) et `app-shortcuts` (non pris en charge sur macOS).
- Quatre pages (`notifications`, `controls`, `snippets`, `status-bars`) n'ont pas de bloc macOS : les trois premières déclarent ne rien ajouter, la dernière n'est pas prise en charge. Leurs règles sont communes à toutes les plateformes, sans cote propre au Mac.
- Ce que cette famille ne dit pas : rien sur le Dock, la souris ou les fenêtres ; rien sur la barre des menus hors des activités en direct ; rien sur les styles de remise des notifications sur macOS ni sur leur regroupement (pour personnaliser l'expérience, la HIG renvoie à sa page Managing notifications, hors de ce chapitre) ; aucune cote en points des widgets ni des modules du Centre de contrôle.
- Convention : sauf mention contraire dans « Spécifique macOS », une page ne traite ni du Dock, ni de la barre des menus, ni de la souris, ni des fenêtres, ni des raccourcis clavier. « Non chiffré dans la HIG » signale l'absence de valeur dans l'export textuel.
- Valeurs chiffrées des pages utiles au bureau : marges de 16 et 11 points, texte à partir de 11 points, animations de 2 secondes au plus, 4 actions par notification, 10 raccourcis d'app par app, 8 heures d'activité au plus, 4 heures de persistance au plus, renvoi après 15 à 30 minutes, cercle de 28 pt de diamètre, marge de 14 points, hauteur de 400 points au plus.
- Correspondances pour hyprdesk : barre avec `live-activities` et, par analogie, `status-bars` ; notifications avec `notifications` et `live-activities` ; widgets de bureau avec `widgets` ; centre de contrôle avec `controls` ; lanceur et recherche avec `app-shortcuts` et `snippets`.

### System experiences (`system-experiences`)

Page d'index de la famille : une définition d'une ligne et un renvoi pour chacune des dix expériences, listées deux fois à l'identique (sections « Discussion » et « Topics »). Ni règle, ni cote, ni mention de macOS. Les définitions servent de « Rôle » à chaque page ci-dessous.

### Notifications (`notifications`)

**Rôle** : une notification (notification) donne une information opportune et de valeur, comprise d'un coup d'œil — "A notification gives people timely, high-value information they can understand at a glance." — system-experiences › Discussion.

**Anatomie et cotes**

Trois styles selon la plateforme. Le texte nomme le bureau (« desktop ») pour le premier et le Centre de notifications (Notification Center) pour le troisième.

| Style | Énoncé HIG (recopié) | Cotes |
|---|---|---|
| Bannière ou vue (écrans verrouillé et d'accueil, bureau) | « A banner or view on a Lock Screen, Home Screen, Home View, or desktop » | non chiffré dans la HIG |
| Badge (badge) sur l'icône de l'app | « A badge on an app icon » ; « a small, filled oval containing a number » | non chiffré dans la HIG |
| Élément du Centre de notifications | « An item in Notification Center » | non chiffré dans la HIG |

- Titre : en haut, là où il est le plus visible — "When a notification includes a title, the system displays it at the top where it’s most visible." — notifications › Content.
- Icône de l'app : grande version en tête, ajoutée par le système — "The system automatically displays a large version of your app icon at the leading edge of each notification" — notifications › Content.
- Communication directe (appel, message) : interface distincte des autres notifications — "a notification related to direct communication — like a phone call or message — can provide an interface that’s distinct from noncommunication notifications" — notifications › Anatomy ; l'image du contact remplace l'icône, avec les noms de groupes — "and group names instead of the app icon" — notifications › Anatomy.
- Actions : vue de détail d'au plus quatre boutons, utilisables sans ouvrir l'app — "A notification can present a customizable detail view that contains up to four buttons people use to perform actions without opening your app." — notifications › Notification actions.
- Icône d'action : placée après le libellé — "The system displays your interface icon on the trailing side of the action title." — notifications › Notification actions.
- Son : facultatif. Hauteur, largeur, espacements, rayon, couleurs, durée d'affichage, délai de fermeture : non chiffrés dans la HIG.

**Règles**

- Permission et réglages
  - Aucune notification sans consentement préalable — "Before you can send any notifications to people, you have to get their consent" — notifications › Discussion.
  - Les styles reçus et les horaires de remise selon l'urgence se choisissent dans les réglages — "people generally use settings to specify the styles of notification they want to receive, and to specify delivery times for notifications that have different levels of urgency" — notifications › Discussion.
  - L'affichage des badges s'autorise par app, dans les réglages de notification — "People can choose whether to allow an app to display badges in their notification settings." — notifications › Badging.
- Pertinence et volume
  - Informer en peu de mots — "People turn on notifications to get quick updates, so you want to provide valuable information succinctly." — notifications › Best practices.
  - Ne pas renvoyer une notification pour la même chose, même sans réponse — "If you send multiple notifications for the same thing, you fill up Notification Center, and people may turn off all notifications from your app." — notifications › Best practices.
  - Ne pas enjoindre d'effectuer une tâche dans l'app ; offrir plutôt des actions quand la tâche est simple — "Otherwise, avoid telling people what to do because it’s hard for people to remember such instructions after they dismiss the notification." — notifications › Best practices.
  - Un message d'erreur appelle une alerte (alert), pas une notification — "People are familiar with both alerts and notifications, so you don’t want to cause confusion by using the wrong component." — notifications › Best practices.
- Application au premier plan
  - Rien d'affiché, mais l'information parvient à l'app — "Your app’s notifications don’t appear when your app is in the front, but your app still receives the information." — notifications › Best practices.
  - Présenter l'information sans distraire : badge incrémenté ou donnée insérée dans la vue — "present the information in a way that’s discoverable but not distracting or invasive, such as incrementing a badge or subtly inserting new data into the current view" — notifications › Best practices.
- Confidentialité
  - Les aperçus peuvent être masqués pour toutes les apps — "In Settings, people can choose to hide notification previews for all apps." — notifications › Content.
  - Prévoir alors un texte générique qui décrit sans rien révéler — "write body text that succinctly describes the notification content without revealing too many details" — notifications › Content.
  - Aucune information sensible, personnelle ou confidentielle — "You can’t predict what people will be doing when they receive a notification, so it’s essential to avoid including private information that could be visible to others." — notifications › Best practices.
- Titre et corps
  - Titre court, lisible d'un coup d'œil — "Prefer brief titles that people can read at a glance" — notifications › Content.
  - Titre purement générique : laisser le système afficher le nom de l'app — "If you can only provide a generic title for a noncommunication notification — like New Document — it can be better to let the system display your app name instead." — notifications › Content.
  - Communication directe : le nom de l'expéditeur occupe la zone du titre — "the system automatically displays the sender’s name in the title area" — notifications › Content.
  - Corps en phrases complètes, casse de phrase, ponctuation soignée ; pas de troncature manuelle — "Use complete sentences, sentence case, and proper punctuation, and don’t truncate your message — the system does this automatically when necessary." — notifications › Content.
- Son
  - Son personnalisé court, distinctif et de qualité professionnelle — "If you use a custom sound, make sure it’s short, distinctive, and professionally produced." — notifications › Content.
  - Aucune information importante ne dépend du son — "don’t rely on it to communicate important information, because people may not hear it" — notifications › Content.
- Actions
  - Actions qui économisent l'ouverture de l'app — "Prefer actions that let people perform common, time-saving tasks that eliminate the need to open your app." — notifications › Notification actions.
  - Libellé court, en casse de titre, décrivant le résultat ; ni nom de l'app ni texte superflu — "For each button, use a short, title-case term or phrase that clearly describes the result of the action." — notifications › Notification actions.
  - Jamais d'action qui se contente d'ouvrir l'app — "presenting an action button that does the same thing clutters the detail view and can be confusing" — notifications › Notification actions.
  - Action destructrice : à éviter, ou avec assez de contexte ; le système la distingue visuellement — "The system gives a distinct appearance to the actions you identify as destructive." — notifications › Notification actions.
- Badge
  - Il ne compte que les notifications non lues — "Don’t use a badge to convey numeric information that isn’t related to notifications, such as weather-related data, dates and times, stock prices, or game scores." — notifications › Badging.
  - Il n'est jamais le seul canal d'une information essentielle — "People can turn off badging for your app, so if you rely on it to show people when there’s important information, people can miss the message." — notifications › Badging.
  - Mis à jour dès l'ouverture des notifications — "Update your app’s badge as soon as people open the corresponding notifications." — notifications › Badging.
  - Compteur à zéro : les notifications liées quittent le Centre de notifications — "Note that reducing a badge’s count to zero removes all related notifications from Notification Center." — notifications › Badging.
  - Ne pas imiter le badge par un composant personnalisé — "People can turn off notification badges if they choose, and will become frustrated if they have done so and then see what appears to be a badge." — notifications › Badging.

**Spécifique macOS**

- Bloc Platform considerations : rien de propre à macOS — "No additional considerations for iOS, iPadOS, macOS, tvOS, or visionOS." — notifications › Platform considerations.
- Surfaces nommées pour un bureau : le bureau (« desktop ») pour la bannière, le Centre de notifications pour la liste (tableau ci-dessus). Aucune disposition, aucun comportement de fermeture ou de regroupement n'est décrit pour Mac.
- Badge : la page parle de l'icône de l'app sans nommer le Dock.
- Clic : le texte ne décrit qu'un toucher, qui ouvre l'app sur le contenu lié — "When people tap a notification or its preview, they expect your app to display related content" — notifications › Notification actions ; aucun geste à la souris pour Mac.
- Absent de cette page, donc à chercher ailleurs : les styles de remise bannière et alerte et leur réglage, le regroupement dans le Centre de notifications (seuls les noms de groupe des communications directes sont évoqués), les gestes et délais de fermeture.

**À transposer dans un shell Linux**

- Notifications, autorisation : consentement par app avant tout affichage, puis réglages par app de style et de remise selon l'urgence.
- Notifications, bannière : titre court en haut, corps en phrases complètes, troncature laissée au shell, grande icône de l'app en tête sans répéter son nom, rien de sensible, texte de remplacement générique quand les aperçus sont masqués.
- Notifications, actions : vue de détail d'au plus quatre boutons, libellés courts en casse de titre, icône après le libellé, aspect distinct pour les actions destructrices, jamais un simple « ouvrir l'app ».
- Notifications, centre : liste sans doublons ; compteur de non-lues sur l'icône de l'app dans la barre ou le dock, remis à jour à la lecture, et dont le passage à zéro retire les notifications liées ; rien qui imite le badge lorsqu'il est désactivé.
- Notifications, app au premier plan : pas de bannière, incrément du badge ou insertion discrète dans la vue.
- Notifications, à régler hors HIG : regroupement dans le centre, gestes et délais de fermeture, styles bannière et alerte sur le bureau.

### Widgets (`widgets`)

**Rôle** : un widget (widget) donne un accès rapide à l'information essentielle et à des interactions ciblées d'une app, hors de l'app — "A widget provides quick access to essential information and focused interactions from your app or game in additional contexts." — system-experiences › Discussion.

**Anatomie et cotes**

Deux familles. Seule la famille système (system family) existe sur Mac, avec une taille « extra large » en plus ; les widgets accessoires (accessory : circular, corner, inline, rectangular) tiennent sur l'écran verrouillé de l'iPhone et de l'iPad et dans les complications de l'Apple Watch — "system family widgets that include an extra large size on iPad, Mac, and Apple Vision Pro" — widgets › Anatomy.

| Taille (famille système) | Contextes sur Mac (colonne « Mac », recopiée) |
|---|---|
| System small | Desktop and Notification Center |
| System medium | Desktop and Notification Center |
| System large | Desktop and Notification Center |
| System extra large | Desktop and Notification Center |
| System extra large portrait | Not supported |

Apparences : un widget peut se montrer en couleurs pleines, en monochrome teinté ou en style translucide — "A widget can appear in full-color, in monochrome with a tint color, or in a clear, translucent style." — widgets › Appearances. Occurrences des modes de rendu sur Mac (ligne « Mac » du tableau, recopiée) :

| Mode de rendu | Occurrence sur Mac |
|---|---|
| Full-color | Desktop and Notification Center |
| Accented | Not supported |
| Vibrant | Desktop |

- Dimensions en points de chaque taille : non chiffrées dans la HIG ; la section Specifications se limite à une phrase d'introduction — "As you design your widgets, use the following values for guidance." — widgets › Specifications.
- Marges : 16 points, 11 points en resserré (voir Règles). Rayon du widget : aucune valeur (voir Règles).
- Taille de texte : 11 points ou plus. Durée des animations de mise à jour : 2 secondes au plus.

**Règles**

- Contenu
  - Une idée simple, liée au but principal de l'app — "Include timely content and relevant functionality." — widgets › Best practices.
  - Contenu signifiant, actions utiles et liens profonds ; recopier l'icône de l'app n'apporte rien — "Replicating an app icon offers little additional value, and people may be less likely to keep it on their screens." — widgets › Best practices.
  - Information qui change au fil de la journée — "Although widgets don’t update from minute to minute, it’s important to find ways to keep their content fresh to invite frequent viewing." — widgets › Best practices.
  - Densité équilibrée — "Sparse layouts can make the widget seem unnecessary, while overly dense layouts are less glanceable." — widgets › Best practices.
  - Rien d'autre que l'information liée au but principal — "In larger widgets, you can display more data — or more detailed visualizations of the data — but you don’t want to lose sight of the widget’s primary purpose." — widgets › Best practices.
  - Éléments de marque discrets — "Incorporate brand colors, typefaces, and stylized glyphs to make your widget recognizable but don’t overpower useful information or make your widget look out of place." — widgets › Best practices.
  - Configuration laissée au choix, quand le contenu à afficher en dépend — "In some cases, people need to configure a widget to ensure it displays the information that’s most useful for them." — widgets › Best practices.
  - Ne pas reproduire l'allure du widget dans l'app — "Including an element in your app that looks like your widget but doesn’t behave like it can confuse people." — widgets › Best practices.
- Tailles
  - Petite taille : une seule information ; grandes tailles : davantage de couches — "Small widgets use their limited space to typically show a single piece of information while larger sizes support additional layers of information and actions." — widgets › Best practices.
  - Ne pas étirer le contenu d'une petite taille pour remplir une grande — "Avoid expanding a smaller widget’s content to simply fill a larger area." — widgets › Best practices.
  - Une taille bien choisie vaut mieux que toutes — "It’s more important to create one widget in the size that best represents the content than providing the widget in all sizes." — widgets › Best practices.
- Mise à jour
  - Rafraîchissement périodique seulement — "To remain relevant and useful, widgets periodically refresh their information but don’t support continuous, real-time updates, and the system may adjust the limits for updates depending on various factors." — widgets › Updating widget content.
  - Date de dernière mise à jour affichée si besoin — "If people are likely to check your widget more frequently than you can update it, consider displaying text that describes when the data was last updated." — widgets › Updating widget content.
  - Dates et heures rafraîchies par le système — "Because widget update frequency is limited, let the system automatically refresh date and time information to preserve update opportunities." — widgets › Updating widget content.
  - Transition animée de 2 secondes au plus à chaque mise à jour — "use standard and custom animations with a duration of up to two seconds to let people know when new information is available or when content displays differently" — widgets › Updating widget content.
- Interactivité
  - Cliquer le widget ouvre son app — "People tap or click a widget to launch its corresponding app." — widgets › Adding interactivity.
  - Boutons et interrupteurs admis sans ouvrir l'app — "It can also include buttons and toggles to offer additional functionality without launching the app." — widgets › Adding interactivity.
  - Toute zone qui n'est ni bouton ni interrupteur ouvre l'app — "When people interact with your widget in areas that aren’t buttons or toggles, the interaction launches your app." — widgets › Adding interactivity.
  - Ouverture au bon endroit, sans navigation — "Deep link to details and actions that directly relate to the widget’s content, and don’t make people navigate to the relevant area in the app." — widgets › Adding interactivity.
  - Cibles cliquables sans erreur, pas de mise en page d'app — "Pay attention to the size of targets and make sure people can tap or click them with confidence and without accidentally performing unintended interactions." — widgets › Adding interactivity.
- Marges et coins
  - Marge standard de 16 points — "Use the standard margin width for widgets — 16 points for most widgets — to avoid crowding their edges and creating a cluttered appearance." — widgets › Choosing margins and padding.
  - Marge de 11 points pour grouper graphiques, boutons ou formes de fond — "setting margins of 11 points can work well" — widgets › Choosing margins and padding.
  - Rayon du contenu accordé à celui du widget — "To ensure that your content looks good within a widget’s rounded corners, use a SwiftUI container to apply the correct corner radius." — widgets › Choosing margins and padding.
- Texte
  - Police système, styles de texte et SF Symbols — "Using the system font helps your widget look at home on any platform" — widgets › Displaying text in widgets.
  - Jamais en dessous de 11 points — "In general, display text using fonts at 11 points or larger." — widgets › Displaying text in widgets.
  - Texte jamais rasterisé, lisible par VoiceOver — "Always use text elements and styles to ensure that your text scales well and to allow VoiceOver to speak your content." — widgets › Displaying text in widgets.
- Couleur
  - Elle ne doit pas concurrencer le contenu — "Beautiful colors draw the eye, but they’re best when they don’t prevent people from absorbing a widget’s information at a glance." — widgets › Using color.
  - Sens aussi par le texte et les icônes — "Use text and iconography in addition to color to express meaning." — widgets › Using color.
  - Images en couleurs pleines réservées aux médias — "Consider reserving full-color images to represent media content, such as album art for a music app’s widget" — widgets › Using color.
  - Couleurs sémantiques pour s'adapter au clair et au sombre — "consider using the semantic system colors for text and backgrounds to let the colors dynamically adapt to the current appearance" — widgets › Full-color.
  - Mode vibrant : l'opacité des pixels règle la force de l'effet de matériau — "In the vibrant rendering mode, the opacity of pixels within an image determines the strength of the blurred background material effect." — widgets › Vibrant.
  - Mode vibrant : blanc ou gris clair pour le principal, gris foncé pour le secondaire — "Use white or light gray for the most prominent content and darker grayscale values for secondary elements to establish hierarchy." — widgets › Vibrant ; valeurs de gris opaques, non des opacités de blanc — "use opaque grayscale values, rather than opacities of white, to achieve the best vibrant material effect" — widgets › Vibrant.
- Galerie
  - Tailles regroupées, description unique — "If your widget is available in multiple sizes, group them together so people don’t think each size is a different widget." — widgets › Previews and placeholders.
  - Aperçu réaliste, à défaut des données simulées — "You can display real data in your widget preview, but if the data takes too long to generate or load, display realistic simulated data instead." — widgets › Previews and placeholders.
  - Contenu provisoire : composants statiques et formes semi-opaques — "Create an effective placeholder appearance by combining static interface components with semi-opaque shapes that stand in for dynamic content." — widgets › Previews and placeholders.
  - Description commençant par un verbe d'action — "Begin a description with an action verb" — widgets › Previews and placeholders.

**Spécifique macOS**

- Bloc Platform considerations : rien de propre à macOS — "No additional considerations for macOS." — widgets › Platform considerations.
- Contextes sur Mac : bureau et Centre de notifications — "On the desktop and Notification Center of their Mac" — widgets › Discussion ; tailles et modes de rendu : tableaux ci-dessus.
- Marges plus petites sur le bureau du Mac, sans valeur donnée — "Additionally, note that widgets use smaller margins on the desktop on Mac and on the Lock Screen, including in StandBy." — widgets › Choosing margins and padding.
- Le mode vibrant vise aussi un fond de bureau macOS — "creates a vibrant effect by coloring your content appropriately for the Lock Screen background or a macOS desktop" — widgets › Appearances.
- Point non tranché par la HIG : la prose réserve deux groupes au mode accentué — "The accented rendering mode divides the widget’s view hierarchy into an accent group and a primary group." — widgets › Accented ; elle le teinte en blanc sur Mac — "On iPhone, iPad, and Mac, the system tints primary and accented content white." — widgets › Accented ; le tableau d'occurrences le déclare « Not supported » sur Mac. Les deux énoncés sont recopiés sans être conciliés.

**À transposer dans un shell Linux**

- Widgets de bureau, grille : quatre tailles (small, medium, large, extra large) ; la HIG ne donne aucune cote, le shell fixe donc lui-même la cellule de sa grille.
- Widgets de bureau, marges : 16 points par défaut, 11 points pour grouper, rayon du contenu concentrique à celui du widget.
- Widgets de bureau, texte : police système, jamais sous 11 points ; date et heure rafraîchies par le shell et non par chaque widget.
- Widgets de bureau, rafraîchissement : périodique, jamais continu ; transition de 2 secondes au plus ; mention de la dernière mise à jour quand la donnée change vite.
- Widgets de bureau, interaction : clic sur le fond égale ouverture de l'app au contenu lié ; boutons et interrupteurs en nombre limité, assez grands pour éviter les erreurs.
- Widgets de bureau, thème : couleurs sémantiques pour le clair et le sombre ; le sens ne dépend jamais de la seule couleur.
- Galerie de widgets : aperçus réalistes, contenu provisoire en formes semi-opaques, une description par widget commençant par un verbe d'action, tailles regroupées.

### Controls (`controls`)

**Rôle** : une commande (control), bouton ou interrupteur, donne un accès rapide à une fonctionnalité d'une app depuis le Centre de contrôle (Control Center), l'écran verrouillé ou le bouton Action — "A control provides quick access to a feature of your app from Control Center, the Lock Screen, or the Action button." — system-experiences › Discussion.

**Anatomie et cotes**

| Élément | Contenu | Cotes |
|---|---|---|
| Symbole (symbol image) | Symbole SF Symbols ou personnalisé ; représente l'action | non chiffré dans la HIG |
| Titre (title) | Ce à quoi se rapporte la commande | non chiffré dans la HIG |
| Valeur (value), facultative | État de la commande | non chiffré dans la HIG |

- Les trois éléments — "Controls contain a symbol image, a title, and, optionally, a value." — controls › Anatomy.
- Exemple : nom de la lampe en titre, allumée ou éteinte en valeur — "For example, the title can display the name of a light in a room, while the value can display whether it’s on or off." — controls › Anatomy.
- Deux types : le bouton agit, l'interrupteur (toggle) bascule — "Control buttons perform an action, link to a specific area of your app" — controls › Discussion ; "Control toggles switch between two states, such as on and off." — controls › Discussion.
- Information montrée selon la taille dans le Centre de contrôle — "In Control Center, a control displays its symbol and, at larger sizes, its title and value." — controls › Anatomy.
- Ajout dans le Centre de contrôle — "People can add controls to Control Center by pressing and holding in an empty area of Control Center" — controls › Discussion.
- Dimensions des modules, grille, espacement, rayon, couleurs : non chiffrés dans la HIG. Seule distinction de taille : « at larger sizes ».

**Règles**

- Offre
  - Réserver des commandes aux actions qui dispensent d'ouvrir l'app — "Offer controls for actions that provide the most benefit without having to launch your app." — controls › Best practices.
  - État toujours exact, action en cours visible — "Update the contents of a control to accurately reflect the state and show if an action is still in progress." — controls › Best practices.
- Symbole
  - Le symbole suffit à lui seul, le titre et la valeur pouvant être masqués — "Depending on where a person adds a control, it may not display the title and value, so the symbol needs to convey enough information about the control’s action." — controls › Best practices.
  - Deux symboles pour un interrupteur — "For control toggles, provide a symbol for both the on and off states." — controls › Best practices.
  - Animation à chaque bascule — "For control toggles, animate the transition between both on and off states." — controls › Best practices.
  - Action longue : animation sans fin jusqu'à son terme — "For control buttons with actions that have a duration, animate indefinitely while the action performs and stop animating when the action is complete." — controls › Best practices.
  - Teinte propre à l'app, appliquée au symbole activé — "The system applies this tint color to a control toggle’s symbol in its on state." — controls › Best practices.
- Configuration
  - Configuration demandée dès l'ajout, modifiable ensuite — "If a control requires configuration, prompt people to complete this step when they first add it." — controls › Best practices.
  - Texte indicatif quand titre et valeur varient — "Placeholder information tells people what your control does when the title and value are situational." — controls › Best practices.
- Verrouillage et sécurité
  - Appareil verrouillé : titre et valeur sensibles masqués — "When the device is locked, consider having the system redact the title and value to hide personal or security-related information." — controls › Best practices.
  - Le symbole passe alors à l'état inactif — "If specified, the system redacts the title and value, and displays the symbol in its off state." — controls › Best practices.
  - Action sensible : déverrouillage requis — "For example, require people to unlock their device to access controls to lock or unlock the door to their house or start their car." — controls › Best practices.

**Spécifique macOS**

- Bloc Platform considerations : rien de propre à macOS — "No additional considerations for iOS, iPadOS, or macOS." — controls › Platform considerations.
- Le Centre de contrôle est le seul des trois emplacements à recevoir un énoncé valant pour un Mac ; l'écran verrouillé et le bouton Action n'ont aucun énoncé propre au Mac, le bouton Action étant attribué à l'iPhone — "On iPhone devices with a control assigned to the Action button" — controls › Anatomy.
- La section Camera experiences on a locked device est propre à iOS — "starting with iOS 18 you can create a control that launches directly to your app’s camera experience while the device is locked" — controls › Camera experiences on a locked device.
- La page ne décrit que les commandes apportées par les apps ; les modules système préinstallés du Centre de contrôle ne sont pas décrits. Le geste d'ajout est formulé en vocabulaire tactile (« pressing and holding »), sans équivalent souris.

**À transposer dans un shell Linux**

- Centre de contrôle, modèle de donnée : chaque module égale symbole, titre et valeur d'état ; deux types, action (bouton) ou bascule (interrupteur à deux états) ; configuration demandée à l'ajout, texte indicatif quand le titre dépend du contexte.
- Centre de contrôle, tailles : deux niveaux d'information, symbole seul en petit, symbole, titre et valeur en grand. Les cotes sont à fixer par le shell (non chiffrées).
- Centre de contrôle, symboles : un symbole qui se comprend sans libellé, une paire pour chaque interrupteur, une couleur d'accent à l'état activé.
- Centre de contrôle, retour et édition : animation brève à chaque bascule, animation continue pendant une action longue, état rafraîchi à l'interaction, à la fin de l'action et sur événement distant ; mode d'ajout ouvert depuis une zone vide du panneau.
- Verrouillage : masquer titre et valeur sensibles sur l'écran verrouillé et exiger la session pour les actions sensibles.

### Status bars (`status-bars`)

**Rôle** : une barre d'état (status bar) occupe le bord supérieur de l'écran et montre l'état courant de l'appareil — "A status bar appears along the upper edge of the screen and displays information about the device’s current state, like the time, cellular carrier, and battery level." — system-experiences › Discussion.

**Anatomie et cotes**

- Position : bord supérieur de l'écran. Contenu cité à titre d'exemple : heure, opérateur cellulaire, niveau de batterie (citation ci-dessus).
- Fond transparent par défaut, le contenu situé dessous reste visible — "By default, the background of the status bar is transparent, allowing content beneath to show through." — status-bars › Best practices.
- Hauteur, espacements, taille de texte, couleurs : non chiffrés dans la HIG. La section Discussion est vide dans l'export.

**Règles**

- Lisibilité
  - Obscurcir le contenu qui passe sous la barre — "Obscure content under the status bar." — status-bars › Best practices.
  - Raison : une commande visible derrière la barre tente l'utilisateur, qui ne peut pas l'actionner — "If controls are visible behind the status bar, people may attempt to interact with them and be unable to do so." — status-bars › Best practices.
  - Préférer un effet de bord de défilement (scroll edge effect), c'est-à-dire une vue floue derrière la barre — "Prefer using a scroll edge effect to place a blurred view behind the status bar." — status-bars › Best practices.
- Masquage
  - Masquage temporaire devant un média plein écran — "A status bar can be distracting when people are paying attention to media." — status-bars › Best practices.
  - Jamais de masquage permanent — "Without a status bar, people have to leave your app to check the time or see if they have a Wi-Fi connection." — status-bars › Best practices.
  - Réaffichage par un geste simple et repérable — "Let people redisplay a hidden status bar with a simple, discoverable gesture." — status-bars › Best practices.

**Spécifique macOS**

- Composant non pris en charge sur macOS — "Not supported in macOS, tvOS, visionOS, or watchOS." — status-bars › Platform considerations ; la page ne vaut que pour iOS et iPadOS — "No additional considerations for iOS or iPadOS." — status-bars › Platform considerations.
- Rapport avec macOS : aucun. La barre des menus du Mac n'est pas décrite ici ; dans cette famille, elle n'apparaît que dans la page `live-activities`.
- Les règles ci-dessus sont donc des règles iOS, transposables par analogie à une barre de bureau, et non des prescriptions pour macOS.

**À transposer dans un shell Linux**

- Barre, statut de la transposition : par analogie seulement, la HIG ne prescrit rien pour la barre d'un bureau ; contenu minimal cité, l'heure et l'état de l'appareil.
- Barre, lisibilité : aucun contenu de fenêtre lisible à travers les glyphes ; derrière la barre, un flou d'effet de bord plutôt qu'un fond plein, et rien qui paraisse cliquable dessous.
- Barre, plein écran : masquer temporairement la barre devant un média plein écran, la réafficher par un geste simple, ne jamais la supprimer pour de bon.

### App Shortcuts (`app-shortcuts`)

**Rôle** : un raccourci d'app (App Shortcut) donne accès aux fonctions ou contenus clés d'une app depuis tout le système — "An App Shortcut gives people access to your app’s key functions or content throughout the system." — system-experiences › Discussion.

**Anatomie et cotes** : aucun élément graphique décrit, non chiffré dans la HIG. Une seule limite — "Each app can include up to 10 App Shortcuts." — app-shortcuts › Discussion.

**Règles**

- Déclenchement par Siri, Spotlight ou l'app Raccourcis — "People can initiate App Shortcuts using features like Siri, Spotlight, and the Shortcuts app" — app-shortcuts › Discussion.
- Disponibles dès la fin de l'installation, sans première ouverture de l'app — "Because App Shortcuts are part of your app, they are available immediately when installation finishes." — app-shortcuts › Discussion.
- Tâches courantes et importantes, réalisables sans quitter le contexte — "Straightforward tasks that people can complete without leaving their current context work best" — app-shortcuts › Best practices.
- Un paramètre optionnel unique, à valeurs prévisibles — "Include predictable and familiar values as options, because people won’t have the list in front of them for reference." — app-shortcuts › Best practices.
- Valeur la plus probable proposée par défaut — "If one option is most likely, consider presenting it as the default" — app-shortcuts › Best practices.
- Phrases d'activation brèves et mémorables — "Provide brief, memorable activation phrases and natural variants." — app-shortcuts › Editorial guidelines.
- Phrases simples à dire — "If your phrase feels too complicated when you say it aloud, it’s probably too difficult to remember or say correctly." — app-shortcuts › Best practices.
- En réponse : un extrait pour une vue statique ou un choix, une activité en direct pour un suivi dans le temps — "are great for custom views that display static information or dialog options" — app-shortcuts › Responding to App Shortcuts.

**Spécifique macOS**

- Non pris en charge — "App Shortcuts aren’t supported in macOS." — app-shortcuts › macOS.
- Mais les actions déclarées servent à bâtir des raccourcis personnalisés sur Mac — "actions you create for your app using App Intents are supported, and people can build custom shortcuts using them with the Shortcuts app on Mac" — app-shortcuts › macOS.
- Spotlight n'est cité que dans la section générale, comme déclencheur possible. Rien sur les raccourcis clavier : la page ne parle que de raccourcis d'app et de l'app Raccourcis.

**À transposer dans un shell Linux**

- Lanceur et recherche : indexer des actions d'app en plus des apps, disponibles dès l'installation, au plus dix par app.
- Lanceur, libellés : tâches courantes réalisables sans quitter le contexte courant ; phrases brèves et mémorables ; un paramètre optionnel unique, à valeurs prévisibles, la valeur la plus probable étant proposée par défaut.
- Lanceur, résultat : afficher le résultat dans le lanceur sous forme d'extrait (voir Snippets) plutôt que d'ouvrir l'app.

### Live Activities (`live-activities`)

**Rôle** : une activité en direct (Live Activity) permet de suivre d'un coup d'œil la progression d'une activité, d'un événement ou d'une tâche — "A Live Activity lets people track the progress of an activity, event, or task at a glance." — system-experiences › Discussion. C'est le seul composant de cette famille que la HIG place dans la barre des menus du Mac.

**Anatomie et cotes**

Quatre présentations à prendre en charge — "As a result, your Live Activity must support:" — live-activities › Anatomy.

- Compact : une seule activité, en deux éléments séparés — "The presentation consists of two separate elements: one on the leading side of the TrueDepth camera and one on the trailing side." — live-activities › Compact.
- Minimal : plusieurs activités, jusqu'à trois — "When multiple Live Activities are active, the system uses the minimal presentation to display up to three of them in the Dynamic Island." — live-activities › Minimal.
- Expanded : après appui long sur une présentation compacte ou minimale — "When people touch and hold a Live Activity in compact or minimal presentation, the system displays the expanded presentation." — live-activities › Expanded.
- Lock Screen : bannière en bas de l'écran verrouillé — "The system uses the Lock Screen presentation to display a banner at the bottom of the Lock Screen." — live-activities › Lock Screen.
- Unité : toutes les valeurs des tableaux sont en points — "All values listed in the tables below are in points." — live-activities › Specifications.
- Cotes pour macOS — "Use the provided iOS dimensions." — live-activities › macOS dimensions. Les tableaux iOS ne figurent pas dans l'export : aucune dimension en points n'est disponible.
- Largeur limitée : tenir dans un cercle de 28 pt de diamètre — "adapt your compact and minimal presentations to fit inside a 28pt-diameter circle" — live-activities › Creating Live Activity layouts.
- Marge de la présentation Lock Screen : 14 points, pour s'aligner sur les notifications — "The standard layout margin for Live Activities on the Lock Screen is 14 points." — live-activities › Lock Screen presentation.
- Animations : 2 secondes au plus — "In addition to extending and contracting transitions, Live Activities use system and custom animations with a maximum duration of two seconds." — live-activities › Adding transitions and animating content updates.
- Texte : graisse medium ou plus — "Use large, heavier-weight text — a medium weight or higher." — live-activities › Best practices.
- Rayon, espacements et couleurs des présentations compacte, minimale et étendue : non chiffrés dans la HIG.

**Règles**

- Périmètre
  - Tâche à début et à fin définis, huit heures au plus — "Live Activities work best for tracking short to medium duration activities that don’t exceed eight hours." — live-activities › Best practices.
  - Seule l'information utile d'un coup d'œil, le détail dans l'app — "Think about what information people find most useful and prioritize sharing it in a concise way." — live-activities › Best practices.
  - Pas de publicité ni de promotion — "Live Activities help people stay informed about ongoing events and tasks, so it’s important to display only information that’s related to those events and tasks." — live-activities › Best practices.
  - Logo sans conteneur, jamais l'icône complète de l'app — "Don’t use the entire app icon." — live-activities › Best practices.
- Confidentialité
  - Contenu sensible : résumé anodin, détail dans l'app — "For content people might consider sensitive or private, display an innocuous summary and let people tap the Live Activity to view the sensitive information in your app." — live-activities › Best practices.
- Cycle de vie
  - Démarrage au moment attendu, arrêt facile depuis l'app — "Live Activities that appear unexpectedly can be surprising or even unwanted." — live-activities › Starting, updating, and ending a Live Activity.
  - Mise à jour seulement quand le contenu change — "If the underlying content or status remains the same, maintain the same display until the underlying content or status changes." — live-activities › Starting, updating, and ending a Live Activity.
  - Alerte uniquement pour l'essentiel, jamais doublée par une notification — "avoid alerting people too often or with updates that aren’t crucial, and don’t use push notifications alongside Live Activities for the same updates" — live-activities › Starting, updating, and ending a Live Activity.
  - Fin immédiate à la fin de la tâche — "Always end a Live Activity immediately when the task or event ends, and consider setting a custom dismissal time." — live-activities › Starting, updating, and ending a Live Activity.
  - Délai de renvoi proportionnel à la durée — "Consider choosing a custom dismissal time that’s proportional to the duration of your Live Activity." — live-activities › Starting, updating, and ending a Live Activity.
  - Le plus souvent 15 à 30 minutes — "In most cases, 15 to 30 minutes is adequate." — live-activities › Starting, updating, and ending a Live Activity.
- Mise en page
  - Marges égales, placement concentrique — "Use even, matching margins between rounded shapes and the edges of the Live Activity to ensure a harmonious fit." — live-activities › Creating Live Activity layouts.
  - Aucun rayon codé en dur — "to match the corner radius of the Live Activity instead of hard-coding a value for it" — live-activities › Creating Live Activity layouts.
  - Changer de mise en page en animant les éléments existants — "preserve as much of the existing layout as possible by animating existing elements to their new positions" — live-activities › Adding transitions and animating content updates.
- Couleur
  - Pas de fond personnalisé pour les présentations compacte, minimale et étendue — "You can’t customize background colors for compact, minimal, and expanded presentations." — live-activities › Choosing colors.
  - Couleurs franches pour le texte et les objets — "Consider using bold colors for text and objects to convey the personality and brand of your app." — live-activities › Choosing colors.
- Interactivité
  - L'ouverture mène directement au détail — "Take people directly to related details and actions" — live-activities › Offering interactivity.
  - Un seul élément interactif de préférence — "If you offer interactivity, prefer limiting it to a single element to help people avoid accidentally tapping the wrong control." — live-activities › Offering interactivity.

**Spécifique macOS**

- Lieu d'affichage sur Mac : la barre des menus — "The menu bar" — live-activities › Discussion.
- L'activité démarre sur iPad ou iPhone et se propage sur les autres appareils — "A Live Activity from your app starts on iPad or iPhone and automatically appears in system locations across a person’s devices" — live-activities › Discussion.
- Bloc Platform considerations › macOS : affichage automatique dans la barre des menus d'un Mac appairé — "Active Live Activities automatically appear in the Menu bar of a paired Mac using the compact, minimal, and expanded presentations." — live-activities › macOS.
- Le clic lance l'app via iPhone Mirroring — "Clicking the Live Activity launches iPhone Mirroring to display your app." — live-activities › macOS.
- Après la fin, l'activité reste jusqu'à quatre heures dans la barre des menus du Mac — "On the Lock Screen, in the Mac menu bar, and the watchOS Smart Stack, it remains for up to four hours." — live-activities › Starting, updating, and ending a Live Activity.
- La présentation Lock Screen n'est pas listée pour Mac. La HIG ne décrit ni activité d'une autre source qu'un iPhone ou un iPad, ni déclencheur à la souris de la présentation étendue, ni emplacement, menu ou survol dans la barre.

**À transposer dans un shell Linux**

- Barre, zone d'activités en cours : trois niveaux, compact (deux éléments, tête et queue), minimal (jusqu'à trois activités) et étendu ; sur iPhone l'étendu s'ouvre par appui long, le déclencheur à la souris étant laissé au shell (survol ou clic long) ; le clic ouvre l'app au contenu lié à l'activité.
- Barre, durée de vie : fin immédiate à la fin de la tâche, résumé conservé 15 à 30 minutes (4 heures au plus), aucune activité au-delà de 8 heures.
- Barre, fraîcheur : mise à jour seulement quand le contenu change ; aucune alerte en doublon avec une notification.
- Barre, style : marges égales, coins concentriques sans rayon codé en dur, texte de graisse medium ou plus, animations de 2 secondes au plus ; pas de fond personnalisé.
- Notifications : confidentialité, résumé anodin dans la barre, détail dans l'app.

### Snippets (`snippets`)

**Rôle** : un extrait (snippet) montre le résultat d'une tâche ou demande une confirmation quand cette tâche est déclenchée avec Siri ou un raccourci d'app — "When someone performs a task with Siri or an App Shortcut, a snippet shows the result or asks for confirmation." — system-experiences › Discussion.

**Anatomie et cotes**

- Deux types — "There are two snippet types: confirmation and result." — snippets › Discussion. Une intention qui affiche un extrait montre toujours un résultat, la confirmation étant facultative — "An app intent that displays a snippet always shows a result, while the confirmation step is optional." — snippets › Discussion.
- Trois éléments : dialogue (texte parlé, au-dessus de la vue), vue personnalisée, boutons système — "The system includes the dialogue text by default and places it above the custom view." — snippets › Anatomy.
- Boutons d'une confirmation — "A confirmation snippet includes two system-provided buttons under the custom view: a secondary Cancel button and a primary button with a customizable label." — snippets › Anatomy.
- Bouton d'un résultat — "A result snippet includes a single Done button that dismisses the view." — snippets › Anatomy.
- Hauteur maximale de la vue personnalisée : 400 points. Largeur, marges, rayon, couleurs : non chiffrés dans la HIG.

**Règles**

- Vue de 400 points de haut au plus — "create custom views that are no taller than the 400-point maximum height" — snippets › Best practices.
- Bouton primaire nommé par ce qu'il fait — "labeling the primary button Order is clearer than labeling it OK or Proceed" — snippets › Best practices.
- Libellé par défaut « Continue » — "If you don’t specify a label, the system default is Continue." — snippets › Best practices.
- Détail trop long : lien profond vers l'app — "For a result snippet, if you need to provide more detail, deep-link to the content in your app instead of including it in the custom view." — snippets › Best practices.
- Le sens passe par la vue, pas par le texte du dialogue — "prefer to omit it from a snippet’s visual representation and use the custom view to convey its information instead" — snippets › Best practices.
- Marges cohérentes dans la vue — "keep consistent margins for the content within the view" — snippets › Best practices.

**Spécifique macOS**

- Bloc Platform considerations : rien de propre à macOS — "No additional considerations for iOS, iPadOS, or macOS." — snippets › Platform considerations.
- Spotlight est cité parmi les déclencheurs, avec Siri et l'app Raccourcis — "Spotlight, or the Shortcuts app." — snippets › Discussion.
- Aucun emplacement, aucune taille ni aucun comportement à la souris n'est décrit pour Mac.

**À transposer dans un shell Linux**

- Lanceur et recherche : deux familles de résultats, confirmation (Annuler et un bouton primaire nommé par un verbe) et résultat (bouton Terminé) ; libellé « Continue » si rien n'est précisé.
- Lanceur, taille : vue personnalisée de 400 points de haut au plus ; le détail s'obtient par lien profond vers l'app.
- Lanceur, contenu : sens porté par la vue, texte court, marges cohérentes, contraste vérifié en clair et en sombre.

### Complications (`complications`)

Une complication (complication) affiche une information opportune sur le cadran de l'Apple Watch — "A complication displays timely, relevant information on the watch face, where people can view it each time they raise their wrist." — system-experiences › Discussion. watchOS seulement — "Not supported in iOS, iPadOS, macOS, tvOS, or visionOS." — complications › Platform considerations. Les tableaux de cotes (familles circular, corner, inline, rectangular, par taille de boîtier) concernent l'Apple Watch et ne sont pas recopiés.

### Top Shelf (`top-shelf`)

Zone de l'écran d'accueil de l'Apple TV, propre à tvOS, qui met en avant les contenus — "showcases your content in a rich, engaging way while also giving people access to their favorite apps in the Dock" — system-experiences › Discussion. Le « Dock » cité est celui de l'Apple TV, non celui du Mac. Non pris en charge sur macOS — "Not supported in iOS, iPadOS, macOS, visionOS, or watchOS." — top-shelf › Platform considerations. Les cotes d'images (affiches, bannières) sont propres à tvOS et ne sont pas recopiées.

### Watch faces (`watch-faces`)

Un cadran de montre (watch face) est la vue principale choisie dans watchOS — "A watch face is a view that people choose as their primary view in watchOS." — system-experiences › Discussion. Non pris en charge sur macOS — "Not supported in iOS, iPadOS, macOS, tvOS, or visionOS." — watch-faces › Platform considerations. Sans rapport avec un bureau.

### Valeurs de référence

Valeurs recopiées telles quelles, sans arrondi ni conversion ; les libellés entre guillemets français sont extraits mot pour mot de la HIG.

| Propriété | Valeur | Source (slug › section) |
|---|---|---|
| Styles d'une notification | « A banner or view on a Lock Screen, Home Screen, Home View, or desktop » ; « A badge on an app icon » ; « An item in Notification Center » | notifications › Anatomy |
| Boutons d'action par notification | « up to four buttons » | notifications › Notification actions |
| Forme du badge | « a small, filled oval containing a number » | notifications › Badging |
| Tailles de widgets système sur Mac | System small, System medium, System large, System extra large : « Desktop and Notification Center » | widgets › System family widgets |
| System extra large portrait sur Mac | « Not supported » | widgets › System family widgets |
| Marge standard des widgets | « 16 points for most widgets » | widgets › Choosing margins and padding |
| Marge resserrée des widgets | « margins of 11 points » | widgets › Choosing margins and padding |
| Marge sur le bureau du Mac | « smaller margins » (aucune valeur) | widgets › Choosing margins and padding |
| Taille de texte minimale des widgets | « 11 points or larger » | widgets › Displaying text in widgets |
| Durée maximale des animations de widget | « up to two seconds » | widgets › Updating widget content |
| Fréquence de mise à jour des widgets | « periodically refresh » ; pas de mise à jour « continuous, real-time » | widgets › Updating widget content |
| Mode de rendu Full-color sur Mac | « Desktop and Notification Center » | widgets › Appearances |
| Mode de rendu Accented sur Mac | « Not supported » | widgets › Appearances |
| Mode de rendu Vibrant sur Mac | « Desktop » | widgets › Appearances |
| Éléments d'une commande | « a symbol image, a title, and, optionally, a value » | controls › Anatomy |
| Information affichée dans le Centre de contrôle | symbole ; « at larger sizes », titre et valeur (aucune cote) | controls › Anatomy |
| Raccourcis d'app par app | « up to 10 App Shortcuts » | app-shortcuts › Discussion |
| Présentations obligatoires d'une activité en direct | Compact, Minimal, Expanded, Lock Screen | live-activities › Anatomy |
| Lieu sur Mac | « The menu bar » | live-activities › Discussion |
| Présentations sur Mac | « compact, minimal, and expanded presentations » | live-activities › macOS |
| Présentations minimales simultanées | « up to three » | live-activities › Minimal |
| Conteneur en largeur limitée | « a 28pt-diameter circle » | live-activities › Creating Live Activity layouts |
| Marge de la présentation Lock Screen | « 14 points » | live-activities › Lock Screen presentation |
| Graisse du texte | « a medium weight or higher » | live-activities › Best practices |
| Durée maximale des animations | « a maximum duration of two seconds » | live-activities › Adding transitions and animating content updates |
| Durée d'une activité | « don’t exceed eight hours » | live-activities › Best practices |
| Persistance après la fin dans la barre des menus du Mac | « up to four hours » | live-activities › Starting, updating, and ending a Live Activity |
| Délai de renvoi conseillé | « 15 to 30 minutes » | live-activities › Starting, updating, and ending a Live Activity |
| Cotes pour macOS | « Use the provided iOS dimensions. » | live-activities › macOS dimensions |
| Hauteur maximale de la vue personnalisée d'un extrait | « the 400-point maximum height » | snippets › Best practices |
| Boutons d'un extrait de confirmation | « a secondary Cancel button and a primary button with a customizable label » | snippets › Anatomy |
| Bouton d'un extrait de résultat | « a single Done button » | snippets › Anatomy |
| Libellé par défaut du bouton primaire | « Continue » | snippets › Best practices |

## Seconde partie — hyprdesk aujourd'hui, écarts et actions

### Ce que hyprdesk fait aujourd'hui
| Surface | Ce qui existe | Valeurs clés | Preuves |
|---|---|---|---|
| Bulle | Une couche `Overlay` sur l'écran actif ; une carte par notification : icône générique, nom de l'app, ✕, titre, corps, boutons d'action ; un clic lance l'action `default` | largeur 360 (260..560) ; carte de 104 px de haut, fixe ; marge interne 10/14 ; rayon 14 ; 4 bulles visibles (1..8) ; 5 coins ; titre 14, corps 12, app 11, actions 11 ; 3 boutons hors `default` | `src/ui/toasts.rs:28-36`, `:210-231`, `:421-486`, `:488-607` |
| Durée et urgence | Sans demande de l'app : 6 s (2..60) ; sinon le délai de l'app. Critique : jamais d'expiration, bord d'accent, passe le mode Silence. L'urgence basse est traitée comme la normale | 6 s | `src/ui/toasts.rs:122-137`, `:498-502`, `:259-262` ; `src/design_options.rs:166-172` |
| Mode Silence | Persistant ; basculé par la pilule « Silence » du centre de contrôle, par le menu du widget Lua `notifications`, par la CLI. Aucun signe dans la barre tant que le centre n'est pas ouvert | — | `src/ui/toasts.rs:159-160`, `:178-198` ; `src/ui/plugins/bar/popover/continuity.rs:568-589` ; `assets/widgets/integres/notifications.lua` |
| Centre de notifications | Historique de 50, en liste plate, sans horodatage ni regroupement. Trois rendus : carte Continuité (12 lignes, icône générique, bouton « Effacer »), carte Convex (30 lignes, texte seul), menu du widget Lua (6 lignes, Silence, Tout effacer) | 50 gardées | `src/ui/toasts.rs:157-158`, `:609-646` ; `src/ui/plugins/bar/popover/continuity.rs:1079-1152` ; `src/ui/plugins/bar/popover.rs:1212` ; `src/ui/mod.rs:4752-4760` |
| Réglages des notifications | Aucun paquet Réglages. Cinq options globales : coin, largeur, nombre visible, durée, empilement | — | `src/design_options.rs:141-179` ; `src/ui/plugins/mod.rs:72` |
| Widget `controls` (bureau ou barre) | Tuiles Wi-Fi et Bluetooth (pastille, titre, valeur, bouton « › »), luminosité, son, mode Bureau en trois segments, batterie | titre 12,5 medium ; valeur 11 ; 1 ou 2 colonnes | `src/ui/controls.rs:20-191`, `:193-372` ; `src/widgets/controls.rs:5-13` |
| Centre de contrôle Continuité | Panneau à 1, 2 ou 3 colonnes : raccourcis (Apparence, Performance, Écrans, Espaces de travail, Réglages), Wi-Fi, Bluetooth, Silence, Verrouiller, son, luminosité, micro, média, calendrier, notifications, système, réseau | pas 12 ; pilules 39 px ; rayons 12 à 26 | `src/ui/plugins/bar/popover/continuity.rs:162-179`, `:275-370`, `:459-643` |
| Widgets de bureau | Huit types intégrés plus des plugins Lua ; carte de verre à 16 px de marge interne, rayon 20 ; réglés par ancrage, décalage, taille du texte, fond en carte | rafraîchissement 1 à 60 s ; corps 14 par défaut ; marge à l'écran 32 | `src/widgets/mod.rs:70-141` ; `src/ui/widget_view.rs:32-319` ; `src/ui/plugins/widgets.rs:61-200`, `:341-355` |
| Barre : modules d'état | Espaces, fenêtre active, bouton bureau, volume, zone de notification, luminosité, widgets compacts. Zone réservée aux fenêtres, flou des couches de la barre | révélation 120 ms, masquage 600 ms, zone 4 px | `src/config/model.rs:923-947`, `:1071-1092`, `:1108-1113`, `:677-683` |
| Barre : activité en cours | Module média (icône lecture ou pause, « titre — artiste », clic, molette). En style Island, la bulle vivante remplace l'horloge de la pilule centrale. L'espace de l'app émettrice passe en couleur d'alerte | libellé coupé à 40 signes (média) ou 26 (Island) | `src/ui/plugins/bar/surface.rs:2224-2264`, `:2307-2356`, `:1392-1397`, `:1579-1592` |
| Zone de notification | Icônes StatusNotifierItem et menus dbusmenu ; icône masquée quand l'app demande l'état `Passive` | — | `src/tray.rs:124-125`, `:323` |

### Écarts
| # | Composant | Nature | Force | Règle HIG (slug › section) | Réel dans hyprdesk (preuve) | Action proposée | Effort | Confiance |
|---|---|---|---|---|---|---|---|---|
| SYS-01 | Icône de l'app dans la bulle et le centre | ABSENT | Descriptif (anatomie) | notifications › Content — « The system automatically displays a large version of your app icon at the leading edge of each notification » | La bulle montre une icône générique de 18 px, en gris, puis le nom de l'app (`src/ui/toasts.rs:36`, `:504-517`). `t.n.icon` n'est dessiné que dans la pilule Island (`src/ui/plugins/bar/surface.rs:2233`). Les cartes du centre font pareil (`src/ui/toasts.rs:625-644` ; `src/ui/plugins/bar/popover/continuity.rs:1105-1110`). | Dessiner l'icône de la notification (déjà résolue par `icon_named`) en grand, à gauche de la bulle et de chaque carte du centre. | S | haute |
| SYS-02 | Compteur de non-lues | ABSENT | Impératif et cote | notifications › Badging — « Update your app's badge as soon as people open the corresponding notifications » ; notifications › Anatomy — « a small, filled oval containing a number » | Aucun état de non-lue : l'historique est une liste simple (`src/ui/toasts.rs:157-158`). `count` vaut la longueur de l'historique, donc jusqu'à 50, et ne retombe qu'à « Tout effacer » (`src/notifications.rs:93-95`). Aucun badge sur l'horloge (`src/ui/plugins/bar/surface.rs:2241-2264`) ; au dock, `badge` compte les fenêtres ouvertes (`src/ui/plugins/dock/surface.rs:1067`, `:1095-1156`). Graphite prévoyait « un badge ne déplace pas l'heure » (`design-plans/barre-graphite.md:168`). | Ajouter un état « non lue » par notification et un compteur ovale sur l'horloge de la barre, remis à zéro à l'ouverture de la liste. | M | haute |
| SYS-03 | Réglages des notifications | ABSENT | Impératif | notifications › Discussion — « Before you can send any notifications to people, you have to get their consent » ; « people generally use settings to specify the styles of notification they want to receive, and to specify delivery times » | Toute notification est acceptée, sans filtre par application (`src/notifications.rs:228-258`). Le registre des réglages n'a pas de paquet Notifications (`src/ui/plugins/mod.rs:72`). Le style de remise dépend du seul code : critique ou bannière de 6 s (`src/ui/toasts.rs:122-137`). | Un paquet de réglages Notifications : par application (`desktop-entry`), l'autorisation d'afficher et le style de remise selon l'urgence. | L | haute |
| SYS-04 | Clic sur une notification du centre | ABSENT | Descriptif | notifications › Notification actions — « When people tap a notification or its preview, they expect your app to display related content » | Les cartes ne sont pas cliquables et ne reprennent aucune action : colonnes de texte (`src/ui/toasts.rs:625-644`), conteneur sans clic (`src/ui/plugins/bar/popover/continuity.rs:1095-1123`). | Rendre chaque carte cliquable : action `default` si l'app en propose une, sinon ouverture de l'application. | S | haute |
| SYS-05 | Aperçus masquables | ABSENT | Descriptif | notifications › Content — « In Settings, people can choose to hide notification previews for all apps. » | Le corps s'affiche partout, sans réglage pour le masquer : bulle (`src/ui/toasts.rs:539-549`), carte (`src/ui/toasts.rs:636-640`), centre (`src/ui/plugins/bar/popover/continuity.rs:1097-1101`). Les cinq options de `src/design_options.rs:141-179` n'en parlent pas. | Un réglage global « Masquer les aperçus » qui remplace le corps par un texte générique. | S | haute |
| SYS-06 | Centre de contrôle : bascule | ÉCART | Impératif | controls › Best practices — « For control toggles, animate the transition between both on and off states. » | Le fond passe du gris à l'accent d'un seul trait : style du bouton (`src/ui/plugins/bar/popover/continuity.rs:219-239`), pilule (`src/ui/plugins/bar/popover/continuity.rs:617-643`), pastille (`src/ui/controls.rs:70-90`). Le seul mouvement est un retour de pression, au style Convex seulement (`src/ui/controls.rs:186-190` ; `src/ui/kinetic.rs:234-248`). | Animer le passage d'un état à l'autre (fond et encre) par une transition brève, nulle sous `transition = "none"`. | S | haute |
| SYS-07 | Centre de contrôle : action en cours | ÉCART | Impératif | controls › Best practices — « For control buttons with actions that have a duration, animate indefinitely while the action performs and stop animating when the action is complete. » | Pendant la bascule, un texte immobile « En cours… » et un bouton désactivé (`src/ui/controls.rs:44`, `:54-55`, `:146`), « Wi-Fi … » dans le centre (`src/ui/plugins/bar/popover/continuity.rs:690-700`). L'attente est posée sans lancer d'animation (`src/ui/mod.rs:4998`). | Remplacer le libellé d'attente par une animation continue, arrêtée à la réception de `ControlToggled`. | S | haute |
| SYS-08 | Centre de contrôle : symboles du Wi-Fi | ÉCART | Impératif | controls › Best practices — « For control toggles, provide a symbol for both the on and off states. » | La tuile « Wi-Fi » prend le symbole de la connexion (filaire, signal, hors ligne), jamais celui de la radio : en filaire, le même symbole dans les deux états (`assets/widgets/integres/network.lua:144-155` ; `src/ui/controls.rs:62-68`, `:240`). Bluetooth est conforme (`assets/widgets/integres/bluetooth.lua:55`, `:67`). | Donner à la tuile Wi-Fi un symbole par état de la radio, l'état de la connexion passant dans la valeur. | S | haute |
| SYS-09 | Galerie de widgets : descriptions | ÉCART | Impératif | widgets › Previews and placeholders — « Begin a description with an action verb » | Aucune description ne commence par un verbe d'action : « Heure (ou date) au format strftime. », « Le mois en cours… », « CPU, mémoire… », « Titre en cours… », « Sortie d'une commande shell… », « Texte rendu par une fonction Lua… » (`src/widgets/mod.rs:86`, `:94`, `:102`, `:110`, `:118`, `:126`), « Wi-Fi, Bluetooth, son… » (`src/widgets/controls.rs:9`), plugins livrés (`assets/widgets/integres/network.lua:123`, `bluetooth.lua:34`, `notifications.lua:56`). Elles s'affichent telles quelles (`src/ui/plugins/widgets.rs:116-121`). | Réécrire chaque description en la commençant par un verbe d'action. | S | haute |
| SYS-10 | Widgets de bureau : clic sur le fond | ABSENT | Descriptif | widgets › Adding interactivity — « People tap or click a widget to launch its corresponding app. » ; « When people interact with your widget in areas that aren't buttons or toggles, the interaction launches your app. » | Hors réglages ouverts, le fond d'un widget ne reçoit aucun clic : la zone de clic n'existe que quand le panneau est ouvert (`src/ui/widget_view.rs:265-288`). Seuls les boutons du lecteur média agissent (`src/ui/widget_view.rs:142-151`). | Rendre le fond de chaque widget intégré cliquable et l'ouvrir sur son contenu lié : horloge et calendrier sur la carte Calendrier de la barre, système sur Réglages › Performance, média sur le lecteur. | M | haute |
| SYS-11 | Galerie de widgets : aperçus | ABSENT | Impératif | widgets › Previews and placeholders — « display realistic simulated data instead » ; « Create an effective placeholder appearance » | Chaque rangée du catalogue tient en nom, origine, description et interrupteur ; ni aperçu ni forme provisoire (`src/ui/plugins/widgets.rs:75-176`). | Ajouter à chaque rangée un aperçu en données simulées. | M | haute |
| SYS-12 | Centre de contrôle : tailles | ABSENT | Descriptif (anatomie) | controls › Anatomy — « In Control Center, a control displays its symbol and, at larger sizes, its title and value. » | Une seule taille, toujours avec titre et valeur : tuile (`src/ui/controls.rs:121-162`), pilule de 39 px (`src/ui/plugins/bar/popover/continuity.rs:617-643`). Seules options : largeur et colonnes (`src/design_options.rs:125-139`). | Ajouter une taille compacte, symbole seul, à côté de la taille actuelle. | M | haute |
| SYS-13 | Barre Island : activité en cours | ABSENT | Impératif | live-activities › Anatomy — « your Live Activity must support: » (compact, minimal, expanded) ; live-activities › Offering interactivity — « Take people directly to related details and actions » | Une seule activité, la dernière bulle, et aucune vue étendue : le clic ouvre le calendrier (`src/ui/plugins/bar/surface.rs:2228-2260`). Le plan prévoit une carte ouverte après 250 ms de survol, « reste à écrire » (`design-plans/barre-island.md:139-149`). | Ouvrir au clic sur la pilule une vue étendue de la notification : corps, actions, accès à la carte des notifications. | M | haute |
| SYS-14 | Barre Island : icône de l'app | ÉCART | Interdiction | live-activities › Best practices — « Don't use the entire app icon. » | La pilule affiche l'icône de l'application telle que la notification la fournit (`src/ui/plugins/bar/surface.rs:2232-2236`). | Remplacer l'icône de l'app par un glyphe monochrome de l'activité. | S | haute |
| SYS-15 | Widgets de bureau : tailles | ABSENT | Tableau et impératif | widgets › System family widgets (quatre tailles sur Mac) ; widgets › Best practices — « Avoid expanding a smaller widget's content to simply fill a larger area. » | Un widget n'a qu'une taille. « Taille du texte » agrandit toute la mise en page d'un même facteur (`src/widgets/mod.rs:70-83` ; `src/ui/plugins/widgets.rs:341-349` ; `src/ui/widget_view.rs:84`, `:194`, `:392`). | Définir par widget des tailles qui ajoutent des couches d'information au lieu d'agrandir le contenu. | L | haute |
| SYS-16 | Widget Système : alerte | ÉCART | Impératif | widgets › Using color — « Use text and iconography in addition to color to express meaning. » | Au-delà du seuil, seule la couleur de la jauge change ; libellé et valeur restent identiques (`src/ui/widget_view.rs:92-117`). | Ajouter à la ligne en alerte un signe non coloré : icône ou libellé. | S | moyenne |
| SYS-17 | Actions destructrices | ABSENT | Descriptif | notifications › Notification actions — « The system gives a distinct appearance to the actions you identify as destructive. » | Une action est une paire clé et libellé, sans marque (`src/notifications.rs:137-143`) ; tous les boutons ont le même style (`src/ui/toasts.rs:558-568`). | Ajouter au modèle un attribut « destructive » (convention de clé à documenter : la spécification freedesktop 1.2 n'en définit pas) et un style d'alerte pour ces boutons. | S | moyenne |
| SYS-18 | Icônes des actions | ABSENT | Descriptif | notifications › Notification actions — « The system displays your interface icon on the trailing side of the action title. » | Les boutons sont de texte seul (`src/ui/toasts.rs:560-567`) ; la capacité `action-icons` n'est pas déclarée (`src/notifications.rs:272-276`). | Déclarer `action-icons`, lire l'icône de chaque action et la dessiner après le libellé. | S | moyenne |
| SYS-19 | Bulle : application au premier plan | PISTE | Descriptif | notifications › Best practices — « Your app's notifications don't appear when your app is in the front, but your app still receives the information. » | Aucun filtre selon la fenêtre active (`src/ui/toasts.rs:252-272`). `desktop_entry` ne sert qu'à l'indicateur d'espace (`src/ui/plugins/bar/surface.rs:1579-1592`). | Ne pas afficher de bulle quand une fenêtre de l'application émettrice a le focus ; la notification reste à l'historique. | M | moyenne |
| SYS-20 | Centre de contrôle : ajout | PISTE | Descriptif | controls › Discussion — « People can add controls to Control Center by pressing and holding in an empty area of Control Center » | L'ajout et le retrait passent par Réglages → Widgets (`src/ui/plugins/widgets.rs:165-176`) ; le panneau n'a ni appui long ni glisser. `DESIGN.md:45` veut que toute fonction passe par le catalogue. | Ouvrir un mode d'édition par appui long sur une zone vide du panneau, en réutilisant le catalogue. | M | moyenne |
| SYS-21 | Widgets de bureau : taille du texte | PISTE | Cote atténuée | widgets › Displaying text in widgets — « In general, display text using fonts at 11 points or larger. » | Par défaut conforme (corps 14, `src/config/model.rs:1745` ; initiales du calendrier à 11,9). Mais le curseur descend à 0 px (`src/ui/plugins/widgets.rs:341-349`), les initiales valent `size × 0,85` sans plancher, soit 10,2 px à 12 (`src/ui/widget_view.rs:414`), et `font_size` dès 6 (`src/config/lua.rs:1179`). | Borner le curseur et les tailles dérivées à 11 points. | S | haute |
| SYS-22 | Widgets de bureau : police | IDENTITÉ | Recommandation | widgets › Displaying text in widgets — « Using the system font helps your widget look at home on any platform » | Mindlog dans tous les widgets (`src/ui/widget_view.rs:30`). | Aucune : choix documenté (orientation, section 4). | — | — |
| SYS-23 | Barre Island : une seule activité | IDENTITÉ | Impératif | live-activities › Anatomy — compact en deux éléments séparés, minimal jusqu'à trois | Une capsule unique, une notification à la fois (`src/ui/plugins/bar/surface.rs:2228-2237`). | Aucune : choix documenté (`DESIGN.md:19` ; `design-plans/barre-island.md:139-143`). | — | — |

Bilan : 6 ÉCART, 12 ABSENT, 3 PISTE, 2 IDENTITÉ.

### Ce qui est déjà conforme
- Titre au-dessus du corps — notifications › Content : titre 14 en police de titre, puis corps 12 (`src/ui/toasts.rs:530-549`).
- Au plus quatre actions — notifications › Notification actions « up to four buttons » : trois boutons hors `default`, plus le clic sur la carte (`src/ui/toasts.rs:550-555`, `:600-606`).
- Bannière ou alerte selon l'urgence — notifications › Anatomy : la critique n'expire pas et garde un bord d'accent (`src/ui/toasts.rs:128-131`, `:498-502`).
- Son facultatif, aucune information n'en dépend — notifications › Content : aucun son n'est joué (`src/notifications.rs:147-176` ne lit aucun indice sonore).
- Fermeture — ✕ sur la bulle, expiration signalée à l'app (`src/ui/toasts.rs:518-526`, `:295-315`).
- Modèle symbole, titre, valeur — controls › Anatomy (`src/ui/controls.rs:70-90`, `:124-137`).
- État exact, action en cours visible — controls › Best practices (`src/ui/controls.rs:44-61`, `:178-184`) ; la bascule relit l'état et signale l'échec (`assets/widgets/integres/network.lua:127-131`).
- Deux symboles, Bluetooth et Silence — controls › Best practices (`assets/widgets/integres/bluetooth.lua:55`, `:67` ; `src/ui/plugins/bar/popover/continuity.rs:568-573`).
- Teinte à l'état activé — controls › Best practices (`src/ui/controls.rs:76-87` ; `src/ui/plugins/bar/popover/continuity.rs:219-229`).
- Marge de 16 points — widgets › Choosing margins and padding (`src/ui/widget_view.rs:249`).
- Texte à 11 points ou plus, par défaut — widgets › Displaying text in widgets (`src/config/model.rs:1745` ; `src/ui/widget_view.rs:81-88`, `:414`).
- Date et heure rafraîchies par le shell, mise à jour périodique — widgets › Updating widget content (`src/ui/widget_view.rs:46-57` ; intervalles en `src/widgets/mod.rs:88-128`).
- Animations de 2 secondes au plus — widgets › Updating widget content ; live-activities › Adding transitions : aucun changement de contenu de widget n'est animé (`src/ui/widget_view.rs:32-213`).
- Graisse medium ou plus — live-activities › Best practices (`src/ui/plugins/bar/surface.rs:1848-1871`).
- Activité terminée avec la tâche — live-activities › Starting, updating, and ending : sans lecteur, le module média disparaît (`src/ui/plugins/bar/surface.rs:2307-2356`).
- Barre au bord supérieur, heure et état de l'appareil — status-bars › Best practices (`src/config/model.rs:923-937` ; `src/ui/plugins/bar/surface.rs:2241-2264`).
- Contenu obscurci sous la barre — status-bars › Best practices : zone réservée aux fenêtres par défaut (`src/config/model.rs:1112`), flou des couches de la barre (`src/config/model.rs:677-683`, `:749-753`).
- Barre masquée sous un média plein écran, jamais pour de bon — status-bars › Best practices : couche `Top` par défaut (`src/config/model.rs:1057-1066`, `:1113`), masquage automatique en option (`src/config/model.rs:1108-1111`).

### À vérifier sur un rendu
- Bulle à corps de deux lignes et actions : la carte fait 104 px de haut, fixe, avec troncature (`src/ui/toasts.rs:571-575`). La rangée d'actions reste-t-elle entière ?
- Titre long coupé sans points de suspension (`src/ui/toasts.rs:536`, `:575`) ; corps limité à deux lignes sources, une ligne vide coupe la suite (`src/ui/toasts.rs:541`).
- Lisibilité des glyphes de la barre quand une fenêtre passe dessous (`reserve_space = false` ou couche `overlay`, `src/config/model.rs:1089-1091`) ; flou réel fourni par Hyprland (`DESIGN.md:33`).
- Zone de révélation de 4 px en masquage automatique (`src/config/model.rs:1111`) : le geste est-il « simple et repérable » ?
- Écran verrouillé : le verrou est un processus distinct sous ext-session-lock-v1 (`src/lock/mod.rs:3-5`). Rien dans le code ne masque la barre, le centre de contrôle (nom du réseau, appareils Bluetooth) ni les bulles ; vérifier en session ce que le compositeur laisse visible ou actif.
- Densité du centre de contrôle à une, deux et trois colonnes (`src/ui/plugins/bar/popover/continuity.rs:162-179`) : « overly dense layouts are less glanceable ».
- Cibles du centre : pilule de 39 px, bouton « › » de 29 px collé à la bascule (`src/ui/plugins/bar/popover/continuity.rs:641`, `:690-733`).
- Mode Silence sans signe dans la barre tant que le centre de contrôle n'est pas ouvert : s'en rend-on compte ?
