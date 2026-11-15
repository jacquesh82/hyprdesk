# Island — étude initiale (remplacée par la référence utilisateur)

**Mise à jour du 15 novembre 2026 :** la référence fournie par l’utilisateur
montre une seule capsule noire centrée, vide au repos (environ 112 × 28 px).
Cette référence remplace la proposition de trois pilules de verre ci-dessous.
L’implémentation révèle les modules dans une capsule centrale au survol,
conserve les notifications dans son centre et laisse les côtés traversants.
Les décisions actuelles sont dans `DESIGN.md` et `src/ui/bar_style/island.rs`.
Le reste de ce document est conservé comme étude historique.

Révision du 15 novembre 2026. **Statut : étude et proposition ; l'application
native n'est pas modifiée dans ce document.** Troisième style de barre, au
côté de `continuite` et `graphite`, porté par la recette documentée dans
`src/ui/bar_style/mod.rs`.

Written against: `21d3214` (« arch : sha256 du tarball v0.1.3 »). Les constats
portent sur ces sources locales. Les specs proviennent de la lecture du dépôt
minflair (GPL-3.0, licence désormais compatible) : `Modules/Bar/BarStyleConfig.qml`,
`Modules/Bar/Styles/Island/*.qml` — les chiffres ci-dessous transposent cette
géométrie, le code reste à écrire from scratch en Rust/Iced.

## Design language

- Audited surface : barre native (layer-shell, `ui/bar.rs`), conducteur des
  styles (`ui/bar_style/`), choix du style (config Lua, CLI, Réglages).
- Design sources : demande utilisateur (« Island Style » de minflair) ;
  `DESIGN.md` pour la matière, le mouvement et la règle du catalogue.
- Documented decisions : trois styles après cette étude (`continuite`,
  `graphite`, `island`). Un choix de style pose toute la barre : rendu,
  géométrie par défaut et composition des modules.
- Governing owners and consumers : `BarStyle`/`BAR_STYLES` dans
  `config/model.rs` → Lua/CLI/Réglages → `App::view_bar()` dans `ui/bar.rs` →
  dispatch `bar_style` → `bar_style/island.rs`. Panneaux : popovers existants.
- Explicit exceptions : zones, hauteur, marge et opacité posées explicitement
  en Lua restent prioritaires sur les défauts du style.

## Intention

Trois **pilules flottantes** en verre fumé — gauche, centre, droite — posées
sous le bord haut de l'écran, séparées par des jours transparents. La pilule
centrale (horloge) est l'hôte naturel des événements : une notification peut
y « prendre la main », puis lui restituer l'heure. L'espace libre de l'écran
reste entièrement visible entre les pilules ; la barre se lit comme trois
objets posés, pas comme une lame.

Le rendu reprend la charte du bureau : verre fumé dérivé du thème (comme
Continuité), filet d'un pixel, une seule ombre douce, accent réservé à la
sélection. **Ligne de conduite : même look et même fluidité que minflair.**
Les durées et courbes sont reprises telles quelles — expansion 250 ms en
`OutQuint` à l'ouverture, `OutCubic` à la fermeture, survol 150 ms
(`animFast`) — et la levée de pilule au survol fait partie du look.
`DESIGN.md` reste respecté : ease-out, reprise depuis la valeur affichée à
l'interruption, coupure `transition = "none"`.

## Findings

| # | Opportunity | Evidence | Proposed change | Scope | Confidence |
| --- | --- | --- | --- | --- | --- |
| 1 | Le conducteur ne sait peindre qu'une lame pleine largeur ; l'îlot veut trois pilules avec jours. | `surface(style, pal, top)` renvoie un `container::Style` unique, appelé en un seul site (`ui/bar.rs:833`). | Hook optionnel `zone_surface(style, pal) -> Option<container::Style>` appliqué **par zone** (gauche/centre/droite) ; les styles existants renvoient `None` et ne changent pas. | `ui/bar.rs`, `ui/bar_style/mod.rs`. | Haute : un seul site d'appel, extension additive. |
| 2 | Les défauts de géométrie suffisent déjà à la pose flottante. | `default_height/default_position/default_margin` existent par style (`config/model.rs:900-920`) ; la marge passe au layer-shell (`ui/bar.rs:463-467`). | `island` : haut, 36 px, marge 8. Aucun changement du conducteur géométrique. | `config/model.rs`. | Haute : mécanisme livré. |
| 3 | Les zones du conducteur correspondent déjà aux trois pilules de minflair. | Preset `[gauche, centre, droite]` (`BarStyle::preset()`) ; minflair répartit logo+workspaces à gauche, horloge au centre, media/tray à droite. | Préset island : `workspaces`+`window` à gauche, `clock` au centre, la droite de Continuité. Modules inchangés, partagés entre styles. | `config/model.rs`. | Haute : composition explicite. |

## Improve first

Le hook par zone (#1) : c'est la seule extension du conducteur, et elle sert
aussi à Convex (voir `barre-convexe.md`). Island la met en œuvre le premier.

## Design decision

**Island est une peau, pas un nouveau système.** Les widgets, panneaux,
débordement, compacité et multi-écrans restent ceux du conducteur `ui::bar` ;
le style décide de la silhouette (trois pilules), du chrome des modules et
des durées. Le dock autonome reste affiché (comme en Continuité) ; son
intégration type Graphite n'appartient pas à ce style.

Le sélecteur d'espaces suit la règle commune : **Rail** (défaut) dessine la
capsule glissante de Continuité à l'intérieur de la pilule gauche —
cohérent avec l'esthétique pilule ; **Miniatures** et **Icônes d'activité**
sont déjà indépendants du style. Le mode Relief reste celui de Graphite.
Aucun rendu d'espace spécifique à Island en phase 1.

Respecter le catalogue : aucun module spécial codé dans `bar.rs` ; les
modules de la pilule droite restent des widgets enregistrés.

**L'expansion « vraie » dans la fenêtre de barre n'est pas retenue** :
animer la géométrie du layer-shell à chaque image n'est pas le modèle
d'hyprdesk, et une fenêtre haute au repos bloquerait les clics sous les
jours. La fluidité minflair est rendue par une surface ancrée qui **naît de
la pilule** (échelle, translation, rayon animés) — même rendu perçu, clics
libres au repos.

## Composition et dimensions

Pixels logiques, multipliés par le facteur de l'écran. **Jetons proposés**,
issus de la transposition de minflair (`idleHeight` 36, `idleRadius` h/2,
`pillGap` 10, `edgeMargin` 16, `barMarginTop` 8) arrondis aux pas de 4 du
bureau.

| Élément | Règle cible |
| --- | --- |
| Fenêtre | Pleine largeur, haut, hauteur 36, marge 8 ; zone exclusive = 44 (hauteur + marge, mécanisme livré). Fond transparent hors pilules. |
| Pilule | Rayon 18 (= hauteur/2) ; verre fumé `bg` à l'opacité du thème (référence 94 %) ; filet 1 px (`fg` à 10 %) ; ombre douce courte, une seule. |
| Pilule gauche | Mesure intrinsèque de ses modules + 12 de retrait horizontal ; commence à 12 px du bord gauche. |
| Pilule centrale | Horloge `%a %d %b  %H:%M`, centrée géométriquement sur l'écran (mesure intrinsèque de la barre Continuité, réutilisée). |
| Pilule droite | Mesure intrinsèque + 12 de retrait ; ordre du préset Continuité (tray, media, réseau…, `desktop`, `session`, `controls`). |
| Jour entre pilules | 10 px minimum ; les pilules ne fusionnent jamais, même si les zones se rapprochent. |
| Module dans une pilule | Capsule pleine (rayon hauteur de ligne − 4) : survol `fg` 8 %, actif 12 %, appui 18 % — mêmes alphas que Continuité. |
| Hauteur de ligne | Héritée du conducteur (36) ; les modules gardent leur taille d'icônes actuelle. |

## Matière et thèmes

| Jeton proposé | Sombre | Clair |
| --- | --- | --- |
| Pilule | `bg` du thème à l'opacité réglée (94 % défaut) | idem |
| Filet de pilule | `fg` à 10 % | `fg` à 12 % |
| Texte | `fg` du thème | idem |
| Capsule de survol | `fg` à 8 % | `fg` à 6 % |
| Accent | accent du thème, sélection uniquement | idem |

Pas de neutralisation Graphite ici : la pilule est teintée par le thème, comme
le verre Continuité. Le flou Hyprland s'applique aux pilules comme aux autres
surfaces de barre ; lisible sans flou. Couleurs Lua explicites prioritaires.

## États et interactions

| État / déclencheur | Présentation et action cible |
| --- | --- |
| Module au repos | Transparent dans la pilule ; la pilule porte la matière. |
| Survol d'une pilule | La pilule se lève de 4 px, 150 ms — translation du **dessin seul** (comme `bar_motion::lift`), boîtes de clic immobiles. |
| Survol module | Capsule `fg` 8 %, 150 ms, boîte de clic immobile. |
| Appui | Capsule 18 %, compression dessinée à 0,97 en 90 ms, sans déplacer la boîte. |
| Module actif | Capsule 12 % permanente (comme Continuité). |
| Espace actif | Capsule glissante du mode Rail, confinée à la pilule gauche ; 180 ms. |
| Espace urgent | Losange partagé existant (`bar_style::urgent_mark`). |
| Compacité | Le repli « actif + … » du conducteur reste dans la pilule gauche ; le débordement « Autres commandes » s'ouvre en carte ancrée à la pilule droite. |
| Panneau (horloge, tray, controls…) | La carte **naît de la pilule** : échelle 0,97 → 1, translation 6 px depuis le module, rayon 18 → 20, 250 ms `OutQuint`, fermeture `OutCubic` réversible 120–150 ms ; 10 px sous la pilule, ancrée aux limites rendues, opacité 0,94 — le vocabulaire des bulles du dock. |
| Focus clavier | Navigation du conducteur ; contour visible distinct du filet de pilule. |

**Phase 2 — prise de main de la pilule centrale (signature minflair)** : la
dernière notification vivante remplace temporairement l'horloge dans la
pilule centrale (largeur mesurée, fondu 160 ms, retour à l'horloge après
fermeture ou 250 ms de survol qui ouvre la carte des notifications). Specs
relevées dans le code minflair, à réutiliser le moment venu : relais de
contenu après 60 ms (timer de handover), expansion à 250 ms en `OutQuint`
ouverture / `OutCubic` fermeture, contenus en fondu sur les deux tiers du
progrès, rayon qui passe de h/2 à la valeur de carte pendant l'ouverture,
padding intérieur de l'hôte étendu 20 px. La prise de main fait partie de la
**phase 1** — c'est la fluidité signature de l'îlot ; le branchement
notifications → widget reste à écrire (voir Stop conditions).

## Adaptation et plusieurs écrans

Une barre par écran sélectionné, pilules indépendantes : mesure, portrait,
compact et débordement sont ceux du conducteur. Sur écran étroit, l'ordre de
repli est : la fenêtre (`window`) part de la pilule gauche, puis les modules
de droite migrent dans le débordement ; les pilules restent séparées par
10 px et jamais collées aux bords (retrait 12 réservé). La pilule centrale
rester centrée tant que `C + 2 × max(G, D) ≤ W − 24`.

## Animations et mouvement réduit

`TOKENS` : survol 150, appui 90, commutation 180. Ouverture de panneau et
prise de main : **250 ms `OutQuint` / `OutCubic`, valeurs exactes de
minflair**. Levée de pilule : +4 px au survol, dessin seul, boîtes de clic
stables. Apparition d'une pilule à la pose du style : fondu 180 ms.
Interruptions toujours reparties de la valeur affichée ; `transition =
"none"` rend tout immédiat en conservant états et contrastes. Redessins
locaux cessent au repos.

## Reuse

- Recette de `bar_style/mod.rs` (variante config, module, dispatch, scènes).
- `design::bar_surface` pour la matière de pilule (même recette que la lame
  Continuité, rayon porté à 18) ; `bar_motion` pour la capsule glissante.
- Widgets, panneaux/popovers, débordement, compacité : conducteur inchangé.
- `default_margin` livré pour la pose flottante.

## Changes

1. `config/model.rs` : variante `BarStyle::Island` (id Lua `"island"`,
   libellé « Island »), défauts 36/haut/8, préset (gauche `workspaces`+
   `window`, centre `clock`, droite = Continuité) ; réconcilier les tests.
2. `config/lua.rs`, `settings.rs`, `cli_config.rs` : le choix island suit
   `BAR_STYLES` ; vérifier Lua, CLI et Réglages.
3. `ui/bar_style/mod.rs` : dispatch island (tokens, surface, module_button,
   rail_look, ink) ; nouveau dispatch `zone_surface` par zone.
4. `ui/bar.rs` : appliquer `zone_surface` aux trois conteneurs de zones
   (`None` = comportement actuel pour Continuité et Graphite).
5. `ui/bar_style/island.rs` : matière de pilule (rayon 18, filet, jour),
   chrome capsules, durées ; pas de rendu d'espaces propre (Rail).
6. `locales/en.toml` : « Island » = "Island" ; chaînes d'aide du choix de
   style ; crédit « inspiré de minflair » dans le CHANGELOG.
7. `ui/plugins/bar.rs`, `ui/preview.rs`, `scripts/verify.sh` : description du
   style et scènes jumelles (ci-dessous).

## Scope

- Inherit : modules, panneaux, débordement, multi-écrans, sélecteur Rail,
  dock autonome.
- Verify : aller-retour entre les trois styles sans résidu (zone exclusive,
  marge, dock), bureau masqué, mode `launcher-only`, reload Lua, portrait,
  deux écrans avec styles différents.
- Exclude : prise de main des notifications (phase 2), option « pilule
  unique » de minflair, intégration du lanceur, refonte des autres styles.

## Validation

- Interface : scènes `bar-island-idle`, `bar-island-pressed`,
  `bar-island-switch`, `bar-island-urgent`, `bar-island-margin` (la marge de
  8 et le jour sous la barre doivent se voir) ; largeurs 1440/900/640, thème
  clair (`HYPRDESK_PREVIEW_THEME=light`) et échelle 1,5 ; compacité avec « … ».
- System : `bash scripts/verify.sh --visual /tmp/hyprdesk-island-validation`
  après implémentation ; tests unitaires des défauts de géométrie et du
  préset ; clavier (flèches, Échap) sur les cartes ancrées aux pilules.
- Session : en Hyprland réel, la zone exclusive place les fenêtres sous
  hauteur + marge ; les clics dans les jours entre pilules sont sans effet.

Le résultat est réussi si, au premier coup d'œil, on lit **trois objets
posés** et non une lame trouée, et si la barre reste parfaitement lisible
sans flou ni animation continue.

## Stop conditions

- Le hook par zone demande plus qu'un style de conteneur (mesure, ancrage
  des cartes contre les jours) : expliciter le contrat dans `mod.rs` avant de
  toucher le conducteur, et le documenter pour Convex.
- La prise de main des notifications nécessite un branchement
  notifications → widget barre qui n'existe pas : la traiter comme un
  fournisseur du service Notifications, pas comme un module spécial.
- Modifications concurrentes du conducteur : réconcilier le plan sans écraser.

## Design documentation

Après implémentation : ajouter la définition Island dans `DESIGN.md`
(§ Styles de la barre), README (gallery), aide Lua et traductions ; noter le
crédit d'inspiration minflair (GPL-3.0). Cette étude modifie uniquement
`design-plans/`.
