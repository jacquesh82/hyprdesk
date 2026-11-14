# Graphite — barre des tâches inspirée de Windows 11

## Mise en application native — 14 novembre 2026

La correction demandée après la capture de référence est implémentée dans le
code Iced : widget `launcher` enregistré au catalogue et hébergé au centre de
Graphite, autres widgets à droite par défaut. Les trois zones restent
personnalisables par ajout ou glisser-déposer, dans la barre et les réglages. La surface
de référence mesure 48 pixels logiques, sans marge ni arrondi extérieur ; les
applications portent des marques distinctes et l’horloge affiche heure/date.
L’ancien préréglage Graphite titre/workspaces/40/10 est migré. Continuité garde
son dock autonome et son rendu existant.

Le lanceur réutilise épingles, regroupement, lancement et activation existants.
Il n’ajoute aucune surface ni zone exclusive en doublon sur les écrans Graphite.
Les menus et le clavier utilisent les coordonnées de la barre, même bureau
masqué ; le retour à Continuité restitue le dock autonome. Sur petite largeur,
les applications défilent horizontalement dans le budget central ; la sélection
clavier se rend visible. Le débordement commun conserve les widgets écartés.

Validation : 388 tests réussis, 6 ignorés, clippy sans avertissement et compilation.
Le DnD a été exercé avec des événements de souris natifs dans l’aperçu Iced :
dépôt depuis la liste des réglages vers le centre et déplacement direct de
l’horloge depuis la barre vers la gauche. Les tests couvrent aussi le
rechargement des zones, les options des widgets, Échap, la sortie de zone
et les configurations portrait.
Captures natives contrôlées en sombre/clair et aux largeurs 1440/900/640.
Les miniatures photographiques de fenêtres de l’étude ne sont pas implémentées :
les bulles conservent actuellement titre et liste des fenêtres. Le menu de
débordement des applications décrit plus bas appartient à la proposition ; le
rendu natif utilise pour le moment le défilement horizontal.

Les sections suivantes conservent l’analyse et la proposition précédant cette
implémentation ; leurs constats décrivent l’ancien code.

---

Révision du 14 novembre 2026. **Statut : analyse, spécification et maquette ;
application native non modifiée dans cette étude.** Cette révision remplace la
direction « lame usinée » conservée en archive à la fin du document.

Written against: `1940626d18fd0ba0bc00a843ccc4bbcdd5fe95df` avec les modifications
locales présentes le 14 novembre, notamment les fichiers de styles/layout non
suivis. Les constats portent sur ces sources locales. Aucune capture de la
session utilisateur n’a été utilisée. La maquette est une proposition.

## Design language

- Audited surface: barre native et dock lanceur des écrans sélectionnés.
- Design sources: demande utilisateur actuelle ; `DESIGN.md` pour les thèmes,
  icônes, plugins/widgets et mouvement désactivable. Sa définition Graphite
  décrit encore l’ancienne intention ; la nouvelle demande prévaut.
- Documented decisions: deux styles existants (`continuite`, `graphite`).
  Continuité garde son identité Unix ; Graphite devient une bande rectangulaire
  inférieure avec commandes/applications centrées et zone système à droite.
- Governing owners and consumers: `BarStyle::preset()` dans `config/model.rs`
  → Lua/réglages → `App::view_bar()` dans `ui/bar.rs` → `bar_layout::strip()`
  et dispatch `bar_style` → `bar_style/graphite.rs`. Chemin distinct du dock :
  `sync_launchers()` → `view_launcher()`/`dock_slots()` dans
  `ui/plugins/launcher.rs`, matière fournie par `ui/design.rs`.
- Explicit exceptions: les zones/couleurs/opacités Lua explicitement réglées
  sont des personnalisations ; elles ne définissent pas le préréglage cible.

## Findings

| # | Problem | Evidence | Proposed change | Scope | Confidence |
| --- | --- | --- | --- | --- | --- |
| 1 | La silhouette Graphite flotte et reste arrondie. | Contrat demandé : bande rectangulaire collée en bas. `BarStyle::default_height/default_margin` donnent 40/10, transmis au layer-shell par `sync_bars()`. `view_bar()` réserve `SLAB_INSET=6` ; `graphite::surface()` utilise `SLAB_RADIUS=14`, relief et ombre. | Préréglage 48/bas/0 ; surface rayon 0, filet supérieur 1 ; supprimer jour interne et ombre de lame. | `config/model.rs`, `ui/bar.rs`, `ui/bar_style/graphite.rs`. | Haute : valeurs et chemin de rendu explicites. |
| 2 | Le centre expose les workspaces ; les applications restent dans un dock distinct. | Contrat demandé : Démarrer, recherche, tâches, applications au centre ; gauche libre ; système et heure/date à droite. `BarStyle::Graphite.preset()` donne `window` à gauche, `workspaces` au centre, horloge `%H:%M` à droite. `view_bar()` consomme ces zones ; `view_launcher()` ne lit pas `BarStyle`. | Intégrer le contenu du plugin Lanceur dans le centre Graphite avec les commandes demandées ; horloge deux lignes et groupe `controls` à droite. | Préréglage Graphite et présentation en barre du plugin Lanceur. | Haute : composition native explicite. |
| 3 | L’activité Graphite repose sur le relief monochrome, et le dock conserve son grossissement Unix. | Contrat demandé : icônes stables, marque ouverte courte, active allongée accentuée. `bar_style::ink(Graphite)` renvoie `fg` ; `graphite::module_button()` dessine un siège permanent, ses workspaces montent. `view_launcher()` applique +9 % et une levée de 2,5. | Réutiliser les états des applications avec une présentation Graphite fixe, un survol plat et des indicateurs neutre/accent 6×3 et 16×3. | Style Graphite et présentation du lanceur. | Haute : traitements visuels explicitement implémentés. |

## Improve first

La composition (#2) : les applications et commandes donnent son identité à la
barre des tâches. Les trois constats relèvent de la même refonte Graphite
demandée et sont donc spécifiés ensemble ci-dessous.

## Evidence chain

- Surface: `App::view_bar()` et `App::view_launcher()`.
- Problem: préréglage et matière reflètent l’ancienne direction « lame ».
- Design evidence: description utilisateur ; sources précitées ; maquette
  `design-plans/graphite-windows11.html`.
- Owner: plugin Barre pour la surface ; plugin Lanceur pour les applications ;
  widgets enregistrés pour commandes et système.
- Scope and affected surfaces: barre, présentation intégrée du lanceur,
  panneaux, réglages, scènes d’aperçu Graphite.
- Uncertainty: le lanceur inspecté affiche une liste de fenêtres, sans images.
  Aucun fournisseur de capture par fenêtre n’est démontré dans ce chemin.
  Les miniatures nécessitent donc un travail fonctionnel distinct du style.

## Design decision

Un choix de style gouverne la présentation barre + lanceur. Graphite compose
les applications du plugin Lanceur dans une surface unique ; le dock flottant
n’est pas affiché en doublon sur cet écran. Épingles, fenêtres et actions restent
une source commune. Continuité conserve barre et dock actuels.

Respecter le catalogue et le placement/cycle de vie existants : les conteneurs
hébergent les fonctions enregistrées. Ne pas coder de recherche ou de lanceur
spécial dans `bar.rs`. Désactiver un plugin doit rester effectif.

## Composition et dimensions

Pixels logiques, multipliés par le facteur de l’écran. Ce sont des **jetons
proposés**, pas des mesures officielles de Windows.

| Élément | Règle cible |
| --- | --- |
| Barre | Pleine largeur, hauteur 48, bas, marge 0, rayon 0 ; zone exclusive 48 réservée une seule fois. |
| Filet | 1 sur le bord supérieur, sans contour latéral ni ombre sous la barre. |
| Commande/application | Cible fixe 44×44, rayon 4, intervalle 4 ; pas horizontal 48. |
| Icône | Boîte 24×24 ; taille optique homogène ; système 16×16. |
| Recherche | Capsule visible 160×32 dans une cible haute de 44 ; loupe 18, intervalle 10, retrait 12. Puis bouton icône 44×44. |
| Groupe central témoin | 44 + 160 + 44 + trois applications de 44 + cinq intervalles de 4 = 400 ; origine `(largeur écran − 400)/2`. |
| Zone système | Chevron 32×44 (44 au tactile), groupe réseau/son/batterie 88×44, horloge 84×44, intervalles 2, retrait droit 8. |
| Horloge | Deux lignes heure/date, alignées à droite, chiffres tabulaires 11–12 ; agrandir la largeur à la mesure des textes localisés. |
| Indicateur ouvert | 6×3, arrondi, centré sous l’icône, retrait inférieur 3. |
| Indicateur actif | 16×3, accent du thème, même emplacement vertical. |
| Panneau | 12 au-dessus de la barre, rayon 8, contour 1, une ombre douce ; borné à l’écran. |
| Aperçu unique | Largeur 264, retrait 10, image 242×132 dans la maquette ; titre et fermeture au-dessus. Respecter le rapport de la vraie capture avec letterboxing. |

La gauche reste libre. L’ordre au centre est Démarrer, Recherche, Vue des
tâches, épingles puis applications ouvertes non épinglées. Les épingles gardent
leur rang à l’ouverture. Centrer ce groupe sur **l’écran**, indépendamment de
la largeur du système. À droite : chevron, réseau, volume, batterie éventuelle,
heure/date. Réserver la place du groupe rapide lorsque la batterie est absente.
Les bureaux virtuels sont accessibles dans la vue des tâches.

## Matière et thèmes

| Jeton proposé | Sombre | Clair |
| --- | --- | --- |
| Barre | `#202020` à 94 % | `#f3f3f3` à 94 % |
| Panneau | `#292929`, opaque | `#fafafa`, opaque |
| Texte | `#f5f5f5` | `#202020` |
| Secondaire / marque ouverte | `#c4c4c4` | `#606060` |
| Filet | Blanc à 9 % | Noir à 8 % |
| Survol | Blanc à 7 % | Noir à 4 % |
| Pression | Blanc à 4 % | Noir à 6 % |
| Accent de démonstration | `#9db9ff` | `#345bbb` |

L’accent réel vient du thème Lua avec contraste approprié. Retirer le plafond
Graphite de 55 % et sa neutralisation de l’accent. Les couleurs personnalisées
restent prioritaires. Le flou complète le fond uniforme, qui doit rester lisible
sans flou sur un fond d’écran clair ou sombre. Les applications conservent les
icônes colorées du thème système. La police native reste celle du bureau
(Mindlog par défaut) ; Segoe UI dans la maquette est une référence de métrique.

## États et interactions

| État / déclencheur | Présentation et action cible |
| --- | --- |
| Épinglée fermée | Icône seule, aucune marque. |
| Ouverte en arrière-plan | Icône et marque neutre 6×3. Plusieurs fenêtres regroupées par application. |
| Active | Marque d’accent 16×3 et fond discret ; état confirmé par Hyprland. |
| Survol / pression | Fond arrondi 4 ; icône stable au survol, compression optique 0,94 à la pression, cible fixe. |
| Focus clavier | Contour 2 visible, distinct de la marque active ; nom et état accessibles. |
| Démarrer | Bascule le panneau d’applications au-dessus du groupe de cet écran. |
| Recherche | Ouvre la recherche existante avec saisie focalisée. |
| Vue des tâches | Ouvre fenêtres et bureaux virtuels existants (`ui/overview.rs`). |
| Clic application fermée | Lance ; attente jusqu’à la fenêtre confirmée, l’échec ou le délai déjà géré. |
| Clic application ouverte | Active sa fenêtre récente ; plusieurs fenêtres accessibles dans les aperçus. |
| Clic application active | Réduit/masque seulement si une action native cohérente existe ; sinon conserve le focus et propose ses fenêtres. Ne pas simuler un état minimisé. |
| Survol prolongé | Après 450 ms, vraie vignette par fenêtre avec titre, au-dessus du bouton ; rien pour une épingle fermée. Traversée vers la carte possible ; fermeture après 250 ms de sortie. Clic vignette = focus ; × = ferme cette fenêtre. |
| Clic droit | Nouvelle fenêtre si disponible, épingler/détacher, fenêtres et fermeture ; extérieur/Échap ferme et rend le focus au déclencheur. |
| Réseau/volume/batterie | Une cible groupée ouvre le centre de contrôle commun ; sources et actions existantes. |
| Heure/date | Calendrier et notifications au-dessus à droite ; un badge ne déplace pas l’heure. |
| Chevron | Icônes masquées ; menu distinct du débordement des applications. |
| Clavier | Entrée par raccourci configurable, flèches, Début/Fin, Entrée/Espace, Menu ou Maj+F10, Échap. Reprendre `launcher/keyboard.rs` ; le survol ne prend pas le clavier. |

La maquette simule localement ces panneaux et les états des applications. Elle
ne pilote pas la session. Ses vignettes sont illustrées, sans capture native.

## Adaptation et plusieurs écrans

Mesurer écran `W`, groupe central `C`, emprises latérales `L/R`. Le centrage
exact reste possible si `C + 2 × max(L,R) ≤ W`, retraits compris.

1. Réduire d’abord la recherche en icône.
2. Retirer les applications excédentaires en réservant immédiatement les 44 du
   bouton de débordement. Garder l’active prioritairement, puis l’ordre des
   épingles. Toutes les entrées retirées restent accessibles avec leur état.
3. Garder système et horloge stables ; ne pas déplacer le groupe central à
   chaque relevé batterie/réseau.
4. Aux largeurs extrêmes où même commandes et système ne tiennent plus centrés,
   borner le groupe à l’espace libre et placer la vue des tâches dans le menu.
   La maquette illustre ce repli à 320 ; les aperçus natifs du dépôt commencent
   à 640. Ne pas réduire les cibles sous 44.

La maquette utilise des paliers de démonstration. Le produit doit conserver la
mesure intrinsèque, tenant compte des textes et échelles réels.

Une barre par écran sélectionné, avec sa propre géométrie, son budget et sa
zone exclusive. Épingles communes dans le même ordre ; applications ouvertes
et focus reflètent l’écran concerné. Pour une épingle ouverte ailleurs,
l’aperçu indique l’écran/espace de chaque fenêtre et l’activation focalise sa
destination sans déplacer silencieusement la fenêtre. Pas de fausse marque
active sur un écran non focalisé. Débranchement : fermer les panneaux de
l’écran disparu. Portrait : budget indépendant. Convertir une seule fois les
coordonnées physiques/logiques pour les ancres et images.

## Animations et mouvement réduit

Survol 120–140 ms, pression 90, indicateur 140, ouverture 160–180, fermeture
120. Ease-out, sans rebond ; reprendre l’état affiché lors d’une interruption.
Pas de grossissement des voisines, ni redessin permanent.
`transition = "none"` coupe fondu, translation, compression et pulsation, en
conservant fond, marques et focus. Le délai anti-ouverture accidentelle de
450 ms reste actif.

## Reuse

- `BarStyle`, `BAR_STYLES`, `continuite` et `graphite` existants.
- `bar_layout::strip()` pour mesure/centrage ; politique de budget Graphite
  distincte des priorités Continuité.
- Plugin Lanceur : `dock_slots()`, épingles, résolution `.desktop`, suivi du
  lancement, focus, ancres de `launcher/lifecycle.rs`, `launcher/keyboard.rs`.
- Exemplar: `view_launcher()` distingue déjà fermé/ouvert/actif par marque.
- `ui/search.rs` (`toggle_apps()`), `ui/apps_grid.rs`, `ui/overview.rs` : modèles
  des panneaux.
- Widget enregistré `controls` (`widgets/controls.rs`), ses sources partagées
  et `ui/controls.rs` ; popovers pour ancrage et limites.

Le lanceur expose aujourd’hui une surface indépendante : ajouter un adaptateur
de présentation en barre via le mécanisme de widgets/catalogue existant,
partageant état et actions. Les commandes doivent être exposées par leurs
plugins/widgets. Un fournisseur de miniatures appartient au lanceur, pas au
conteneur de barre. Ne pas dupliquer modèles ou surveillance des fenêtres.

## Changes

1. `config/model.rs`, `config/lua.rs`, `settings.rs`, `cli_config.rs` : préréglage
   48/bas/0 et composition ; réconcilier les tests 40/10/workspaces. Préserver
   identifiants et valeurs explicites. Vérifier par Lua, CLI et réglages.
2. `ui/bar_style/graphite.rs`, `mod.rs`, `ui/bar.rs` : surface plate, retrait nul,
   accent et boutons transparents au repos. Préserver Continuité, y compris son
   sélecteur relief qui réutilise actuellement le dessin Graphite.
3. `ui/plugins/launcher.rs` et modules : présentation intégrée et états fixes,
   menu contextuel, débordement et aperçus ; garder une source unique des
   épingles, lancements et actions. Aucun dock doublé ni réservation doublée.
4. `ui/bar_layout.rs`, fournisseurs de commandes et popovers : budget système,
   réduction recherche puis applications, ancrage aux limites rendues.
   Préserver le comportement Continuité.
5. `ui/plugins/bar.rs`, `ui/preview.rs`, `scripts/verify.sh`, traductions : nouvelle
   description Graphite, scènes clair/sombre, trois états, survol/pression,
   clavier, aperçu et compacité. Garder toutes les scènes Continuité.

## Scope

- Inherit: préréglage Graphite, présentation du lanceur, widgets et panneaux.
- Verify: activation/désactivation des plugins, mode `launcher-only`, reload,
  écrans et retour à Continuité ; la barre ne dépend pas du bureau « raised ».
- Exclude: refonte Continuité, thème des applications, police globale,
  remplacement du compositeur ou du système d’icônes.

## Validation

- Product: sélection Graphite → une barre ; fermé/ouvert/actif côte à côte puis
  lancement/focus/fermeture confirmés par les vrais états.
- Interface: 1440/900/640, clair/sombre, échelles 1/1,25/1,5/2, batterie absente,
  nombreuses applications, titres longs, portrait, deux écrans dont un non
  focalisé, clavier, mouvement désactivé.
- System: widgets enregistrés, états partagés, réservations et cycle de vie.
- Repository **après implémentation** : `bash scripts/verify.sh --visual
  /tmp/hyprdesk-graphite-validation` ; refaire les captures avec
  `HYPRDESK_PREVIEW_THEME=light` et `HYPRDESK_PREVIEW_SCALE` approprié. Ces
  contrôles natifs ne sont pas annoncés exécutés dans cette étude.
- Maquette de l’étude : vérifier rendu et interactions locales à 1024/736/640/320,
  présence des deux thèmes, vue cotée et aperçu de fenêtre.

Contrôles effectués le 14 novembre sur la maquette avec Chromium : quatre
largeurs sans collision ni débordement horizontal ; recherche filtrée,
activation, survol différé et clic sur l’aperçu, menu contextuel/fermeture,
réglages rapides, calendrier, Échap/retour de focus et mouvement réduit.
Aucune erreur JavaScript observée. Vérification des icônes d’applications à
24 pixels. Ces résultats concernent la maquette, pas les fonctionnalités natives.

## Stop conditions

- Capture native nécessitant une permission/protocole nouveau : isoler cette
  dépendance, prévoir titre/icône en secours, ne pas annoncer de vraie miniature.
- Placement nécessitant une extension du catalogue : expliciter le contrat
  commun avant de modifier le conteneur.
- Modifications concurrentes : réconcilier le plan sans écraser le travail local.

## Design documentation

Après implémentation et validation, remplacer dans `DESIGN.md` l’ancienne
définition « lame 40/10, monochrome et relief » ; documenter présentation du
lanceur, jetons, débordement et synchroniser aide Lua/README/traductions.
Cette étude modifie uniquement `design-plans/`.

---

<details>
<summary>Archive : ancienne direction du 13 novembre, remplacée par la révision ci-dessus</summary>

## Graphite — ancienne direction « lame usinée »

Direction UI/UX retenue et implémentée dans l'application native le 13
novembre 2026. Spécification écrite contre la branche de travail qui suit
`1940626d` (« Continuité »). Elle étend la barre à **plusieurs styles** :
`hd.bar({ style = "continuite" | "graphite" })`, un module de rendu par
style dans `src/ui/bar_style/`.

## Intention

Une barre compacte, monochrome et originale, dont le relief aide à comprendre
quel espace de travail est actif. Imagine une **lame horizontale en
graphite** : le gris du thème **sans sa teinte**, un acrylique neutre très
transparent — la même matière que le menu Démarrer de Windows 11, jamais
teintée par le fond d'écran — en bloc arrondi flottant au-dessus du bord,
avec une ombre douce généreuse. Les espaces de travail sont gravés dans la
surface ; l'espace actif devient une petite touche surélevée. Lors d'un
changement, l'ancienne touche redescend et la nouvelle se soulève — la barre
reste immobile et les numéros gardent leur place.

Le rendu évoque un objet usiné : contours nets, coins légèrement adoucis,
ombre courte et reflet fin sur l'arête. L'alternance entre gravure et relief
suffit à identifier la barre **en noir et blanc** ; les effets lumineux
restent réservés à la lisibilité de l'arête et au retour d'interaction.
Aucune couleur d'accent dans le style : tout se dérive de la luminance de
`fg`/`bg` du thème (les états `warning`/`critical` restent sémantiques). Les
couleurs posées explicitement pour la barre (`colors.text`,
`colors.background`) restent celles de l'utilisateur.

## Composition

**La lame est un bloc à part entière, pas une bande collée au bord** : coins
arrondis (rayon 14), contour d'usinage d'un pixel, reflet fin sur l'arête
supérieure, tranche inférieure plus sombre et ombre douce portée sous le
bloc — une bande transparente de 6 px au bas de la fenêtre la laisse tomber.
Matière acrylique : l'opacité du thème est plafonnée à 55 % (flou Hyprland
actif sur la barre) ; `bar.opacity` posé explicitement reste maître.

**La composition fait partie du style** : titre de l'application au premier
plan à gauche, espaces de travail **centrés**, et à droite les indicateurs
du quotidien (réseau, batterie, volume), l'horloge à chiffres courts (`%H:%M`)
et le seul panneau de réglages rapides. Ce préréglage s'applique partout :
Réglages (choisir le style pose modules, pose, hauteur, marge), CLI
(`bar.style`), et `hd.bar` quand aucune zone n'y est écrite. Ce qui est
ajusté ensuite — zones, champs — reste prioritaire.
Posée **en bas de l'écran par défaut** : choisir le style pose toute la
géométrie (bas, 40 px, marge au bord 10 px réglable par `bar.margin` ; la
zone réservée aux fenêtres compte la hauteur et la marge) — en Réglages,
changer de style applique aussitôt cette pose, les champs Position/Hauteur/
Marge restent ajustables ensuite. Hauteur cible 38–42 px logiques. Trois
zones d'une même surface :

- **Gauche** : bouton Hyprdesk et nom court de l'application au premier plan,
  tronqué si nécessaire, nom complet en info-bulle ; le bouton ouvre le menu
  Hyprdesk (comportements existants, inchangés).
- **Centre** : les espaces de travail, centrés sur l'écran indépendamment de
  la longueur du titre à gauche et de l'horloge à droite (mesure intrinsèque
  et centrage géométrique de la barre « Continuité », réutilisés tels quels).
- **Droite** : icônes monochromes (réseau, volume, batterie) puis horloge à
  chiffres de largeur fixe ; les réglages rapides restent groupés derrière le
  panneau unique (`controls`), qui s'ouvre au-dessus de la barre.

## États

| État | Rendu |
|---|---|
| Module au repos | Siège gravé permanent : chaque module est une pièce posée dans la lame (creux discret, contour à peine visible). |
| Espace vide | Numéro lisible et discret (fg 55 %), gravé au niveau de la surface : creux plus sombre en haut, la lumière tombe dedans. |
| Espace occupé | Numéro plus présent (fg 78 %) et petit point plein sous le numéro ; sa place est réservée pour que les touches gardent leur hauteur. |
| Espace actif | Touche surélevée de 4 px : fond dégradé vers l'arête claire, contour d'usinage 1 px, ombre courte portée entre la touche et le creux qu'elle laisse. |
| Module survolé | L'arête du siège s'éclaircit ; l'espace survolé monte d'un pixel (¼ de la montée). |
| Appui | Le siège s'enfonce ; la touche aussi, un pixel brièvement — le changement est confirmé par l'état remonté par Hyprland, jamais par l'appui. |
| Espace urgent | Petit losange plein fixe à côté du numéro (dessiné par hyprdesk), libellé accessible en info-bulle (« Fenêtre nécessitant votre attention »). |
| Focus clavier | Suivant séparé : la surface layer-shell reste sans clavier dans cette passe ; le contour distinct du relief est à venir avec la navigation clavier. |

## Mouvement

- Montée/redescente des touches : 120–180 ms (170 ms livré), ease-out cubique,
  **toujours à partir de la hauteur affichée** — des changements rapides
  convergent vers le dernier espace actif sans rejouer les transitions
  intermédiaires.
- Survol des modules ordinaires : contour clair (arête éclaircie) plutôt que
  capsule remplie ; durée 140 ms comme la pose des cartes.
- Boîtes de clic immobiles : la translation ne touche que le dessin.
- Redessins uniquement pendant le geste ; la barre est au repos entre deux
  changements.
- `transition = "none"` (mouvement réduit) : les différences de relief et de
  contraste restent, les translations sautent à leur cible.

## Adaptation multi-écrans

Chaque barre reflète l'état de son écran (zones portrait, compacité
automatique, débordement « Autres commandes », workspace actif par écran) :
tous les mécanismes de « Continuité » sont réutilisés, le style ne décide que
du dessin. Sur un écran étroit, le titre de fenêtre part d'abord, puis les
indicateurs débordent dans le menu ; les espaces et l'heure restent
accessibles.

**Un style est une barre complète** : Graphite impose ses touches en relief —
le sélecteur de workspaces (`workspace_selector`) ne s'applique qu'au style
Continuité, pour ne jamais composer de barre hybride.

## Implémentation

- `src/ui/bar_style/` — façade (`mod.rs`) : jetons de durées, dispatch
  surface/module/touches, contexte des espaces, losange d'urgence partagé.
  La recette pour ajouter un style y est documentée.
- `bar_style/continuite.rs` — le rendu d'origine, extrait sans changement de
  comportement.
- `bar_style/graphite.rs` — surface, chrome des modules, touches gravées.
- `src/ui/bar_motion.rs` — `lift(content, raised, ink, durée)` : montée
  animée, creux au sol, ombre courte (quad + blur du renderer), reflet 1 px
  sur l'arête ; et `RailLook::{Fill,Edge}` pour la capsule de survol.
- Urgence : `hypr.rs` relaie `urgent>>` (et relit les fenêtres) ; l'état est
  élagué quand la fenêtre ferme ou quand son espace redevient actif sur son
  écran. Icône `hyprdesk-urgent-symbolic` dans `src/ui/icons.rs`.
- Géométrie : `BarSurface` porte la marge ; `sync_bars` la pose dans
  `NewLayerShellSettings.margin` (ordre top/right/bottom/left) sans changer la
  zone exclusive.

## Vérification

Scènes d'aperçu jumelles : `bar-graphite-idle`, `bar-graphite-pressed`,
`bar-graphite-switch`, `bar-graphite-bottom` (bas, 40 px, marge 10 — le jour
au-dessus du bord doit se voir) et `bar-graphite-urgent`. Les contrôler à
1440/900/640 px, en thème clair et sombre (`HYPRDESK_PREVIEW_THEME=light`) et
à l'échelle 1,5 (`HYPRDESK_PREVIEW_SCALE=1.5`) : le groupe central reste
immobile quand un titre s'allonge ou qu'une icône apparaît, le relief reste
lisible en niveaux de gris, le losange d'urgence se voit sans couleur.

Le résultat est réussi si, au premier coup d'œil, on comprend quel espace est
actif et si le geste distinctif de la barre — **une touche s'enfonce pendant
qu'une autre se soulève** — reste lisible sans couleur et sans animation
continue.
</details>
