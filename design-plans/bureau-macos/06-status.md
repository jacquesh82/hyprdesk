# Composants : statut — macOS d'après la HIG, face à hyprdesk

> Proposition de design, rien d'implémenté. Plan d'ensemble et priorités : `plan-d-action.md` ; méthode et périmètre : `README.md`.

> Référentiel : Human Interface Guidelines d'Apple, export Markdown officiel relevé le 23 novembre 2026. Code : commit `00f6d69` (arbre de travail du 23 novembre 2026, avec des modifications non commitées).

## Première partie — Spécification d'après la HIG

### Résumé

- La famille Status (statut) compte quatre composants qui rendent visible l'état d'une mesure ou d'un processus : jauges (gauges), indicateurs de progression (progress indicators), anneaux d'activité (activity rings) et indicateurs de notation (rating indicators).
- Deux seulement concernent un bureau macOS : les jauges, que macOS complète par les indicateurs de niveau (level indicators), et les indicateurs de progression, en barre ou en cercle, déterminés ou indéterminés.
- Les anneaux d'activité ne sont pas pris en charge sur macOS et sont réservés aux objectifs Move, Exercise et Stand ; les indicateurs de notation n'ont aucune considération propre à macOS.
- Règle directrice des indicateurs de progression : autant que possible déterminé (durée connue) plutôt qu'indéterminé, exact, en mouvement, à un emplacement constant ; sur macOS, un spinner discret de préférence pour une opération d'arrière-plan.
- La HIG ne publie aucune cote : ni taille de spinner, ni épaisseur de piste, ni vitesse d'animation, ni délai d'affichage. Les valeurs disponibles sont des dénombrements, des directions et une couleur par défaut.

### Statut — Status (`status`)

- **Rôle** — Page d'index de la famille : elle définit en une phrase chacun des quatre composants et renvoie à leur page.
- **Anatomie et cotes** — Non chiffré dans la HIG : l'index se réduit à deux listes de définitions (Discussion et Topics), sans illustration ni valeur.
- **Règles** — Aucune recommandation normative sur cette page ; les définitions qu'elle fixe sont citées sur la puce Rôle de chaque section ci-dessous.
- **Spécifique macOS** — Rien : l'index ne mentionne ni macOS, ni le Dock, ni la barre des menus, ni la souris, ni de raccourcis.
- **À transposer dans un shell Linux** — **Vocabulaire** : garder la correspondance jauge (gauge), indicateur de progression (progress indicator), anneau d'activité (activity ring), indicateur de notation (rating indicator) pour nommer les widgets et les états de la barre de hyprdesk.
  - **Périmètre** : seules les jauges et les indicateurs de progression offrent un modèle macOS ; les deux autres composants n'apportent rien à la comparaison.

### Jauges — Gauges (`gauges`)

- **Rôle** — Une jauge (gauge) affiche une valeur numérique précise dans une plage de valeurs — "A gauge displays a specific numerical value within a range of values." — status › Discussion
- **Anatomie et cotes** — Un trajet circulaire ou linéaire représente la plage de valeurs et la valeur en cours y occupe un point précis — "A gauge uses a circular or linear path to represent a range of values, mapping the current value to a specific point on the path." — gauges › Anatomy
  - Style standard : un index marque la position de la valeur — "A standard gauge displays an indicator that shows the current value’s location" — gauges › Anatomy
  - Style capacity : un remplissage s'arrête à la position de la valeur — "a gauge that uses the capacity style displays a fill that stops at the value’s location on the path" — gauges › Anatomy
  - Variante accessory, d'aspect proche des complications de watchOS, adaptée aux widgets d'écran verrouillé iOS — "works well in iOS Lock Screen widgets and anywhere you want to echo the appearance of complications" — gauges › Anatomy
  - Non chiffré dans la HIG : dimensions, épaisseur du trajet et du remplissage, rayon des extrémités, graduations, couleurs (aucune valeur pour le rouge et le bleu de l'exemple).
- **Règles** — *Libellés* : succincts, décrivant la valeur en cours et les deux extrémités de la plage — "Write succinct labels that describe the current value and both endpoints of the range." — gauges › Best practices
  - Tous les styles n'affichent pas tous les libellés, mais VoiceOver lit les libellés visibles — "VoiceOver reads the visible labels to help people understand the gauge without seeing the screen" — gauges › Best practices
  - *Contexte* : une jauge peut informer aussi sur la plage elle-même — "a gauge can provide more context about the range itself" — gauges › Discussion
  - Exemple : un spectre de couleurs qui renforce visuellement les valeurs changeantes — "display a spectrum of colors that visually reinforce the changing values" — gauges › Discussion
  - *Dégradé* : envisager de remplir le trajet d'un dégradé qui communique l'usage de la jauge — "Consider filling the path with a gradient to help communicate the purpose of the gauge." — gauges › Best practices
  - Exemple : du rouge au bleu pour des températures du chaud au froid — "a temperature gauge might use colors that range from red to blue to represent temperatures that range from hot to cold" — gauges › Best practices
- **Spécifique macOS** — pas de considération supplémentaire pour iOS, iPadOS, visionOS et watchOS ; non prise en charge sur tvOS — "No additional considerations for iOS, iPadOS, visionOS, or watchOS. Not supported in tvOS." — gauges › Platform considerations
  - *Indicateurs de niveau (level indicators)* : macOS définit en outre un indicateur de niveau, qui affiche une valeur numérique dans une plage — "In addition to supporting gauges, macOS also defines a level indicator that displays a specific numerical value within a range." — gauges › macOS
  - Certains de leurs styles ressemblent à des jauges — "some of which have visual styles that are similar to gauges" — gauges › Anatomy
  - Réglable pour exprimer la capacité, la notation ou, rarement, la pertinence — "You can configure a level indicator to convey capacity, rating, or — rarely — relevance." — gauges › macOS
  - *Style capacity* : valeurs discrètes ou continues — "The capacity style can depict discrete or continuous values." — gauges › macOS
  - Continu : piste horizontale translucide que remplit une barre pleine — "A horizontal translucent track that fills with a solid bar to indicate the current value." — gauges › macOS
  - Discret : rangée horizontale de segments rectangulaires séparés et de taille égale — "A horizontal row of separate, equally sized, rectangular segments." — gauges › macOS
  - Le nombre de segments égale la capacité totale et chaque segment se remplit entièrement, jamais partiellement — "The number of segments matches the total capacity, and the segments fill completely — never partially — with color to indicate the current value." — gauges › macOS
  - Continu de préférence pour les grandes plages, qui rendraient les segments trop petits — "A large value range can make the segments of a discrete capacity indicator too small to be useful." — gauges › macOS
  - *Couleur* : verte par défaut pour les deux styles capacity — "By default, the fill color for both capacity indicator styles is green." — gauges › macOS
  - La couleur peut changer quand la valeur atteint certains niveaux (très bas, très haut, juste après le milieu) — "you can change the fill color when the current value reaches certain levels, such as very low, very high, or just past the middle" — gauges › macOS
  - Sur tout l'indicateur, ou par paliers pour montrer plusieurs couleurs — "you can use the tiered state to show a sequence of several colors in one indicator" — gauges › macOS
  - *Style rating* : il renvoie aux indicateurs de notation — "For guidance using the rating style to help people rank something" — gauges › macOS
  - *Style relevance* : rarement utilisé, barre horizontale ombrée — "the relevance style can communicate relevancy using a shaded horizontal bar" — gauges › macOS
  - Par exemple dans une liste de résultats de recherche, pour visualiser la pertinence — "a relevance indicator might appear in a list of search results, helping people visualize the relevancy of the results when sorting or comparing multiple items" — gauges › macOS
  - Non précisé pour macOS : taille, épaisseur, valeur RGB du vert, nombre de segments par défaut ; rien sur le Dock, la barre des menus, la souris ou des raccourcis.
- **À transposer dans un shell Linux** — **Widgets de mesure (batterie, stockage, mémoire)** : modèle capacity, en barre continue pour une grande plage, en segments pour une petite capacité.
  - **Segments** : nombre égal à la capacité, remplissage entier uniquement.
  - **Couleur** : une teinte par défaut, changement de couleur seulement aux niveaux significatifs (très bas, très haut) ; la valeur du vert n'est pas donnée.
  - **Libellés** : valeur en cours et deux extrémités, lisibles par un lecteur d'écran.
  - **Recherche** : une barre de pertinence ombrée dans les résultats est possible, rarement employée sur macOS.
  - **Anneaux** : pour un widget en anneau, le seul modèle macOS est la jauge circulaire (voir Anneaux d'activité).

### Indicateurs de progression — Progress indicators (`progress-indicators`)

- **Rôle** — Un indicateur de progression (progress indicator) fait savoir que l'app n'est pas bloquée pendant qu'elle charge du contenu ou exécute une opération longue — "Progress indicators let people know that your app isn’t stalled while it loads content or performs lengthy operations." — status › Discussion
- **Anatomie et cotes** — Tout indicateur de progression est transitoire : affiché pendant l'opération seulement — "All progress indicators are transient, appearing only while an operation is ongoing and disappearing after it completes." — progress-indicators › Discussion
  - Déterminé (determinate) : tâche à durée bien définie, comme une conversion de fichier — "for a task with a well-defined duration, such as a file conversion" — progress-indicators › Discussion
  - Indéterminé (indeterminate) : tâche non quantifiable, comme le chargement ou la synchronisation de données complexes — "for unquantifiable tasks, such as loading or synchronizing complex data" — progress-indicators › Discussion
  - Déterminé : une piste linéaire ou circulaire se remplit au fil de la tâche — "A determinate progress indicator shows the progress of a task by filling a linear or circular track as the task completes." — progress-indicators › Discussion
  - Barre de progression (progress bar) : remplissage du côté leading (début) vers le côté trailing (fin) — "include a track that fills from the leading side to the trailing side" — progress-indicators › Discussion
  - Indicateur circulaire : remplissage dans le sens horaire — "have a track that fills in a clockwise direction" — progress-indicators › Discussion
  - Indéterminé, ou indicateur d'activité (activity indicator) : image animée — "uses an animated image to indicate progress" — progress-indicators › Discussion
  - Non chiffré dans la HIG : taille, épaisseur, couleur, vitesse d'animation, délai avant affichage ; aucune plage, aucun palier ni fréquence de mise à jour pour les valeurs de progression (seul exemple chiffré : le contre-exemple des Règles).
- **Règles** — *Choix du type* : autant que possible, déterminé — "When possible, use a determinate progress indicator." — progress-indicators › Best practices
  - L'indéterminé montre qu'un processus a lieu, mais n'aide pas à estimer la durée — "An indeterminate progress indicator shows that a process is occurring, but it doesn’t help people estimate how long a task will take." — progress-indicators › Best practices
  - Dès que la durée d'un processus indéterminé devient connue, passer en barre déterminée — "When possible, switch a progress bar from indeterminate to determinate." — progress-indicators › Best practices
  - *Exactitude* : être aussi exact que possible dans un indicateur déterminé — "Be as accurate as possible when reporting advancement in a determinate progress indicator." — progress-indicators › Best practices
  - Régulariser le rythme d'avancement — "Consider evening out the pace of advancement to help people feel confident about the time needed for the task to complete." — progress-indicators › Best practices
  - À éviter : 90 pour cent de complétion en cinq secondes, puis les 10 derniers en 5 minutes — "Showing 90 percent completion in five seconds and the last 10 percent in 5 minutes" — progress-indicators › Best practices
  - *Mouvement* : garder l'indicateur en mouvement — "Keep progress indicators moving so people know something is continuing to happen." — progress-indicators › Best practices
  - Si le processus s'arrête, un retour qui explique le problème et la marche à suivre — "If a process stalls for some reason, provide feedback that helps people understand the problem and what they can do about it." — progress-indicators › Best practices
  - Ne jamais passer du style circulaire au style barre — "Don’t switch from the circular style to the bar style." — progress-indicators › Best practices
  - *Description et emplacement* : un texte d'accompagnement exact et succinct, si utile — "If it’s helpful, display a description that provides additional context for the task." — progress-indicators › Best practices
  - Éviter les termes vagues comme loading ou authenticating — "Avoid vague terms like *loading* or *authenticating* because they seldom add value." — progress-indicators › Best practices
  - Toujours le même emplacement — "Display a progress indicator in a consistent location." — progress-indicators › Best practices
  - *Arrêt* : quand c'est possible, laisser arrêter le traitement — "When it’s feasible, let people halt processing." — progress-indicators › Best practices
  - Sans effet secondaire : un bouton Cancel — "If people can interrupt a process without causing negative side effects, include a Cancel button." — progress-indicators › Best practices
  - Avec effet secondaire (perdre la partie déjà téléchargée d'un fichier) : un bouton Pause en plus de Cancel — "it can be useful to provide a Pause button in addition to a Cancel button" — progress-indicators › Best practices
  - Quand l'arrêt a une conséquence fâcheuse, une alerte qui propose de confirmer ou de reprendre — "that includes an option to confirm the cancellation or resume the process" — progress-indicators › Best practices
- **Spécifique macOS** — aucune considération supplémentaire pour tvOS et visionOS — "No additional considerations for tvOS or visionOS." — progress-indicators › Platform considerations
  - Sur macOS, un indicateur indéterminé a l'apparence d'une barre ou d'un cercle — "In macOS, an indeterminate progress indicator can have a bar or circular appearance." — progress-indicators › macOS
  - Toutes les plateformes ont l'image circulaire qui tourne ; macOS prend de plus en charge une barre de progression indéterminée — "All platforms support a circular image that appears to spin; however, macOS also supports an indeterminate progress bar." — progress-indicators › Discussion
  - *Choix* : préférer l'indicateur d'activité (spinner) pour l'état d'une opération d'arrière-plan ou quand la place manque — "Prefer an activity indicator (spinner) to communicate the status of a background operation or when space is constrained." — progress-indicators › macOS
  - Petits et discrets, les spinners conviennent aux tâches asynchrones d'arrière-plan, comme récupérer des messages d'un serveur — "Spinners are small and unobtrusive, so they’re useful for asynchronous background tasks, like retrieving messages from a server." — progress-indicators › macOS
  - Ils conviennent aussi dans un espace réduit : dans un champ de texte ou à côté d'une commande — "Spinners are also good for communicating progress within a small area, such as within a text field or next to a specific control, such as a button." — progress-indicators › macOS
  - Ne pas étiqueter un spinner — "Avoid labeling a spinning progress indicator." — progress-indicators › macOS
  - Non précisé pour macOS : taille, vitesse et couleur du spinner, position dans la fenêtre ; rien sur le Dock, la barre des menus ou la souris.
- **À transposer dans un shell Linux** — **Lanceur et recherche** : spinner sans libellé pendant l'indexation ou l'interrogation d'une source lente ; barre déterminée dès que la progression est connue.
  - **Barre** : toute opération de durée connue (copie, synchronisation, mise à jour) affiche une barre déterminée, exacte et régulière, jamais figée.
  - **Panneau de réglages** : spinner à côté de la commande concernée plutôt qu'une barre globale.
  - **Arrêt** : bouton Annuler quand l'interruption est sans effet de bord ; Pause et Annuler sinon.
  - **Cohérence** : ne pas passer du cercle à la barre, et garder le même emplacement d'un écran à l'autre.
  - **Non couvert** : la HIG ne dit rien d'un indicateur de progression dans le Dock, la barre des menus ou les notifications.

### Anneaux d'activité — Activity rings (`activity-rings`)

- Progression quotidienne d'une personne vers les objectifs Move, Exercise et Stand — "Activity rings show an individual’s daily progress toward Move, Exercise, and Stand goals." — status › Discussion ; usage réservé à ces trois mesures, sans détournement — "Use Activity rings only to show Move, Exercise, and Stand information." — activity-rings › Best practices ; non pris en charge sur macOS, tvOS et visionOS — "Not supported in macOS, tvOS, or visionOS." — activity-rings › Platform considerations

### Indicateurs de notation — Rating indicators (`rating-indicators`)

- Série de symboles graphiques disposés horizontalement pour communiquer un niveau de classement — "A rating indicator uses a series of horizontally arranged graphical symbols" — status › Discussion ; valeur arrondie, jamais de symbole partiel — "A rating indicator doesn’t display partial symbols; it rounds the value to display complete symbols only." — rating-indicators › Discussion ; aucune considération propre à macOS — "No additional considerations for macOS." — rating-indicators › Platform considerations

### Valeurs de référence

| Propriété | Valeur | Source (slug › section) |
|---|---|---|
| Types d'indicateur de progression | 2 : determinate, indeterminate | progress-indicators › Discussion |
| Sens de remplissage d'une barre de progression | du côté leading vers le côté trailing | progress-indicators › Discussion |
| Sens de remplissage d'un indicateur circulaire déterminé | sens horaire | progress-indicators › Discussion |
| Apparences d'un indicateur indéterminé sur macOS | barre ou circulaire, image animée | progress-indicators › macOS |
| Rythme à éviter (exemple de la HIG) | 90 percent en five seconds, puis les derniers 10 percent en 5 minutes | progress-indicators › Best practices |
| Formes de jauge | circulaire ou linéaire ; styles standard et capacity ; variante accessory | gauges › Anatomy |
| Style capacity continu | piste horizontale translucide remplie d'une barre pleine | gauges › macOS |
| Style capacity discret | segments rectangulaires séparés, de taille égale ; remplissage entier, jamais partiel | gauges › macOS |
| Couleur de remplissage par défaut des indicateurs de capacité | green (aucune valeur RGB donnée) | gauges › macOS |
| Usages d'un indicateur de niveau sur macOS | capacity, rating, relevance (rare) | gauges › macOS |
| Anneaux d'activité sur macOS | non pris en charge | activity-rings › Platform considerations |
| Indicateurs de notation sur macOS | aucune considération supplémentaire | rating-indicators › Platform considerations |

## Seconde partie — hyprdesk aujourd'hui, écarts et actions

### Ce que hyprdesk fait aujourd'hui
| Surface | Ce qui existe | Valeurs clés | Preuves |
| --- | --- | --- | --- |
| Indicateur de lancement (dock) | Marque iris qui « respire » sous l'icône en attendant une fenêtre ; arrêtée à son apparition, à l'échec ou au bout de 5 s | alpha 0,4 à 1,0 sur 1,1 s ; marque 12 × 4 px (16 × 3 en intégré) ; fixe si le mouvement est coupé | `src/ui/plugins/dock/surface.rs:579-589,648-670` ; `src/ui/plugins/dock/lifecycle.rs:14,212-227` ; `DESIGN.md` § Lancement |
| OSD volume et luminosité | Carte HUD centrée en bas, sans clavier : icône, jauge de 6 px, « NN % » | 300 × 64 px, levée de 140 px, affichée 1,5 s, piste au texte à 15 %, remplissage d'accent (atténué et « muet » si coupé) | `src/ui/osd.rs:21-27,88-94,117-199` |
| Jauges du widget système | Barre continue sous chaque ligne (CPU, RAM, disques), en option, couleur d'alerte au-delà d'un seuil | piste de 4 px, longueur 16 × taille de texte | `src/widgets/system.rs:62-109` ; `src/ui/widget_view.rs:90-112` |
| Centre de contrôle | Batterie : titre, pourcentage coloré selon l'état, barre de 8 px ; volume : arc de 57 px autour de l'icône et « Nom · NN % » ; média : arc | — | `src/ui/plugins/bar/popover/continuity.rs:852-866,962-974,1349-1402,1422-1430` |
| Mises à jour | Avancement déterministe par phase et par paquet, agrégé par source ; jauge fine dans le module de barre | de 0 à 99 % pendant l'opération | `src/update_progress.rs:92-136,286-320` ; `src/updates.rs:342-400,502-506` ; `src/ui/plugins/bar/surface.rs:2414-2426` |
| Assistant LYRA | Pastille d'état : phase, puis « phase — NN % » dès que la fraction arrive ; indicateur de réflexion à trois barres | — | `src/ui/chat.rs:974,1579-1591` ; `DESIGN.md` § LYRA |
| Curseurs | Rail de 10 px arrondi, poignée circulaire de 8 px, remplissage depuis le début | — | `src/ui/design.rs:58-75` ; `src/ui/plugins/bar/popover/continuity.rs:248-257` |

### Écarts
| # | Composant | Nature | Force | Règle HIG (slug › section) | Réel dans hyprdesk (preuve) | Action proposée | Effort | Confiance |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| STA-01 | Indicateur de lancement du dock : fin sans retour | ÉCART | impératif (« provide feedback ») | progress-indicators › Best practices — « If a process stalls for some reason, provide feedback that helps people understand the problem and what they can do about it. » | Sans fenêtre correspondante au bout de 5 s, l'entrée de lancement est retirée sans message (`src/ui/plugins/dock/lifecycle.rs:14,212-227`). Échec de résolution ou d'envoi : même retrait muet (`src/ui/mod.rs:4163-4166,4171-4176` ; `lifecycle.rs:264-271`) | Annoncer la fin sans fenêtre par un retour non intrusif proposant de relancer l'application | S | haute |
| STA-02 | Jauges : aplat sans dégradé | PISTE | recommandation (« Consider ») | gauges › Best practices — « Consider filling the path with a gradient to help communicate the purpose of the gauge. » | Remplissage d'un seul ton : accent sur les jauges du widget système (`src/ui/widget_view.rs:98-108`), sur l'OSD (`src/ui/osd.rs:126`), ton du texte sur la batterie du centre de contrôle (`src/ui/plugins/bar/popover/continuity.rs:1384-1402`) | Tracer la jauge de batterie du centre de contrôle en dégradé, de l'alerte vers l'accent | S | haute |
| STA-03 | Indicateur de lancement : marque iris respirante, fixe si mouvement coupé, bornée à 5 s | IDENTITÉ | impératif (« Keep »), non suivi par choix documenté | progress-indicators › Best practices — « Keep progress indicators moving so people know something is continuing to happen. » | `src/ui/plugins/dock/surface.rs:582-589` : `Transition::None` donne une marque fixe. `DESIGN.md` § Lancement : « indicateur iris respirant pendant l'attente d'une nouvelle fenêtre correspondante (…) ou après cinq secondes. Le mouvement désactivé conserve un indicateur fixe. » | Aucune — choix documenté, à signaler seulement | — | haute |
| STA-04 | Assistant LYRA : statut en texte à côté de l'indicateur de réflexion | IDENTITÉ | impératif (« Avoid »), non suivi par choix documenté | progress-indicators › macOS — « Avoid labeling a spinning progress indicator. » | `src/ui/chat.rs:1579-1591` : pastille d'état avec phase et pourcentage. `DESIGN.md` § LYRA : « indicateur de réflexion à trois barres (…) Les statuts restent lisibles en texte. » | Aucune — choix documenté, à signaler seulement | — | haute |

### Ce qui est déjà conforme
- Indicateur déterministe chaque fois que possible | mises à jour par phase et par paquet (`src/update_progress.rs:92-136,286-320`), jauge du module de barre (`src/ui/plugins/bar/surface.rs:2414-2426`).
- Passage d'indéterminé à déterminé | LYRA : phase seule, puis « phase — NN % » (`src/ui/chat.rs:1580-1584`).
- Indicateurs transitoires | marque retirée à l'apparition de la fenêtre (`src/ui/plugins/dock/lifecycle.rs:219-227`) ; jauge de mise à jour seulement pendant l'opération (`src/updates.rs:556`).
- Petit indicateur discret pour une tâche de fond | marque de 12 × 4 px sous l'icône (`src/ui/plugins/dock/surface.rs:648-670`).
- Emplacement unique de la progression des mises à jour | le module de barre, à chaque fois (`src/ui/plugins/bar/surface.rs:2414-2426`).
- Jauge continue pour de grandes plages | CPU, RAM, disques (`src/widgets/system.rs:62-109` ; `src/ui/widget_view.rs:108-112`).
- Couleur de remplissage informative | alerte au-delà du seuil (`src/ui/widget_view.rs:96-105`) ; batterie selon l'état (`src/ui/plugins/bar/popover/continuity.rs:1366`) ; OSD atténué et « muet » (`src/ui/osd.rs:126,159-163`).
- Valeur courante libellée | « 42 % », « 5,4 Gio / 15 Gio » (`src/widgets/system.rs:64-76`) ; OSD « NN % » (`src/ui/osd.rs:159-163`) ; volume « Nom · NN % » (`continuity.rs:852-856`).
- HUD petit, sombre, translucide, peu coloré, écarté du contenu qu'il règle, d'un seul style | OSD (`src/ui/osd.rs:22-27,91-93,126`).
- Sans objet | Activity rings et Rating indicators (aucun anneau ni notation dans un bureau) ; page status (index sans règle) ; curseurs (hors des pages de la famille) ; arrêt d'une mise à jour : « When it's feasible, let people halt processing » — interrompre une transaction de paquets a des effets de bord, aucune commande d'arrêt n'est offerte (`src/updates.rs:342-400`).

### À vérifier sur un rendu
- Salience de la marque de lancement, 12 × 4 px sous une icône de 42 px (`src/ui/plugins/dock/surface.rs:648-670`).
- Distinction entre la marque de lancement (iris qui respire) et le trait de fenêtre active, de même taille (`src/ui/plugins/dock/surface.rs:653-658`).
- Régularité de l'avancement des mises à jour : poids fixes par phase (pacman 0,15 · 0,32 · 0,675 · 0,95 ; `src/update_progress.rs:336-358`) et départ à 0 % (`src/updates.rs:350`).
- Forme de la jauge de mise à jour dans Panorama (`src/ui/plugins/panorama/surface/dashboard.rs:1657`) : barre comme dans la barre, ou anneau.
- Libellés de phase pendant une mise à jour et pendant un travail de LYRA : précis ou vagues.
- Lisibilité de l'OSD en verre sur un contenu clair.
