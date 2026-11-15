> Mise à jour après comparaison à l'animation utilisateur : la cible comprend désormais le cadre complet, les tiroirs raccordés et le lanceur au bord inférieur. Les exclusions « cadre plein écran » et « hosts bas » de cette étude initiale sont remplacées par l'implémentation et les réglages décrits dans `DESIGN.md`. Ce document conserve l'historique de la première proposition, pas la cible actuelle.

# Convex — cadre organique sculpté dans le bord, inspiré de minflair

Révision du 15 novembre 2026. **Statut : étude et proposition ; l'application
native n'est pas modifiée dans ce document.** Quatrième style de barre, au
côté de `continuite`, `graphite` et de l'îlot proposé dans
`barre-island.md` — dont il réutilise le hook de surface par zone.

Written against: `21d3214`. Les specs proviennent de la lecture du dépôt
minflair (GPL-3.0, licence désormais compatible) : `Modules/Bar/Styles/Convex/`
— notamment `ConvexShape.qml` dont le `getPath()` bezier est transposé
chiffre pour chiffre ; le code Rust reste à écrire from scratch.

## Design language

- Audited surface : barre native (layer-shell), conducteur des styles,
  hook de surface par zone introduit par Island.
- Design sources : demande utilisateur (« Convex Style » de minflair) ;
  `DESIGN.md` (matière, mouvement, catalogue).
- Documented decisions : un choix de style pose toute la barre ; Convex est
  une **peau sculptée** sur la barre existante, pas un cadre plein écran : le
  dock autonome reste l'équivalent des « outils du bas » de minflair.
- Governing owners and consumers : `BarStyle`/`BAR_STYLES` (`config/model.rs`)
  → Lua/CLI/Réglages → `view_bar()` → dispatch `bar_style` →
  `bar_style/convexe.rs`. Le dessin du path appartient au style, pas au
  conducteur.
- Explicit exceptions : hauteur, marge, opacité et zones posées explicitement
  en Lua restent prioritaires.

## Intention

Une lame pleine largeur, collée au bord haut, dans laquelle **trois niches
sont creusées** pour accueillir les groupes gauche/centre/droite — les
modules descendent dans leur niche, tandis qu'entre les niches la lame
s'amincit en **ponts** fins, reliés par des **évasements** courbes. La
silhouette évoque un objet usiné dont la matière aurait été enlevée au
fraisage : contours nets, un seul filet suivant le path, ombre douce sous la
lame. Au premier coup d'œil on comprend que la barre est **sculptée dans le
bord de l'écran**, pas posée dessus.

## Findings

| # | Opportunity | Evidence | Proposed change | Scope | Confidence |
| --- | --- | --- | --- | --- | --- |
| 1 | La surface d'un style est un rectangle (`container::Style`) ; Convex demande un path de Bézier avec creux et ponts. | `surface()` appelé en un site unique (`ui/bar.rs:833`) ; le canvas Iced est déjà idiomatoire (`ui/icons.rs`, `ui/plugins/*`). | Phase 1 : la lame reste un rectangle plein ; les **niches** sont peintes par un canvas du style posé par le hook par zone (cf. Island) — le path dessine creux, ponts et évasements dans la fenêtre existante. | `ui/bar_style/convexe.rs`. | Haute : dessin local, conducteur inchangé. |
| 2 | La géométrie de minflair tient dans une fenêtre de hauteur uniforme. | `ConvexShape.qml` : `baseHeight` 44, `topBezel` ≈ 4, flares 26×20, rayon 20 — tout le path vit sous `y = 44`. | Fenêtre 48 (44 de path + 4 d'air pour l'ombre), niches uniformes ; **phase 2** seulement : niche centrale plus profonde (fenêtre 64, zone exclusive 64). | `config/model.rs` (défauts), convexe.rs. | Haute : chiffres explicites. |
| 3 | Le cadre minflair couvre aussi le bas de l'écran (hosts bas). | `ConvexBottomHost.qml`, `ConvexExclusions.qml` gèrent un cadre plein écran. | Hors scope : les outils du bas restent le **dock** autonome (modèle Continuité). Aucune seconde surface de barre, aucune exclusion plein écran. | — | Haute : périmètre tranché. |

## Improve first

Le path (#1) : c'est l'identité entière du style. Les niches suivent les
**limites mesurées** des trois zones par le conducteur — le style lit le
layout, il ne le décide pas.

## Design decision

**Ligne de conduite : même look et même fluidité que minflair** — le path
reprend les valeurs d'origine (niches 44, évasements 26×20, rayons 20,
**ponts à 4**), aucune adaptation de charte à la place d'un chiffre mesuré.
**Convex sculpte la barre, il ne la réinvente pas.** Modules, panneaux,
débordement, compacité, multi-écrans : conducteur inchangé. Dock autonome
conservé. Sélecteur d'espaces : **Rail** (défaut) — la capsule glissante de
Continuité vit dans la niche qui héberge les espaces ; Relief reste Graphite,
Miniatures/Icônes inchangés. Des touches segmentées **natives** (segments
séparés par le filet de niche, comme minflair) sont une phase 2 optionnelle.

Respecter le catalogue : aucun module spécial ; le canvas ne dessine que la
matière (fond, filet, ombre), jamais le contenu des modules.

## Composition et dimensions

Pixels logiques. **Jetons proposés**, transposés de `ConvexShape.qml`
(`baseHeight` 44, `flareWidth` = `sizeLg`+2 ≈ 26, `flareHeight` ≈ 20,
`bottomRadius` ≈ 20, `topBezel` ≈ 4) arrondis aux pas du bureau.

| Élément | Règle cible |
| --- | --- |
| Fenêtre | Pleine largeur, haut, hauteur 48, marge 0 ; zone exclusive 48. |
| Profondeur de lame/niches | `bh` = 44 uniforme en phase 1 ; 4 px d'air en bas de fenêtre pour l'ombre. |
| Ponts entre niches | Épaisseur `tb` = 4, valeur exacte de minflair : le pont est un filet vivant entre deux niches, jamais une zone de contenu. |
| Évasements | Courbes cubiques : largeur `fw` = 26, hauteur `fh` = 20, rayon d'angle de niche `br` = 20 ; tangentes verticales/horizontales comme le `getPath()` d'origine. |
| Ancrage des niches | Droite des zones mesurées ± 8 de padding ; la niche centrale est centrée géométriquement (mesure intrinsèque existante). |
| Module dans une niche | Bouton plat type Graphite (transparent au repos, survol 9 %, appui 4 %, rayon 4) ; actif = fond 7 %. |
| Filet | 1 px suivant tout le path : arête claire (`fg` à 18 %) côté haut des ponts, neutre (`fg` à 10 %) ailleurs. |
| Ombre | Une seule, douce, sous la lame et sous les niches (ne pas doubler dans les angles concaves). |

L'ordre de lecture : niches = contenu, ponts = respiration. Si une zone
déborde (compacité), sa niche suit la mesure réelle et les ponts se
déplacent avec elle — jamais l'inverse (les modules ne se plient pas au
path).

## Matière et thèmes

| Jeton proposé | Sombre | Clair |
| --- | --- | --- |
| Lame et niches | `bg` du thème à l'opacité réglée (94 % défaut) | idem |
| Arête (filet haut) | `fg` à 18 % | `fg` à 22 % |
| Filet courant | `fg` à 10 % | `fg` à 12 % |
| Texte / accent | thème | idem |

Matière teintée par le thème (verre Continuité), pas neutralisée — la
signature Graphite « gris sans teinte » n'est pas reprise. Flou Hyprland sur
la surface ; lisible sans flou. Couleurs Lua explicites prioritaires. Le
canvas se redessine sur changement de thème, de layout de zones ou de taille
d'écran, **pas à chaque image**.

## États et interactions

| État / déclencheur | Présentation et action cible |
| --- | --- |
| Module au repos | Transparent ; la niche porte la matière. |
| Survol / appui | Chrome plat type Graphite, 140/90 ms, compression 0,97 sans déplacer les boîtes. |
| Espace actif | Capsule glissante Rail dans sa niche ; 180 ms ; jamais hors de la niche. |
| Espace urgent | Losange partagé existant. |
| Panneaux | Cartes ancrées existantes sous la barre, ancrées aux limites rendues ; inchangées. |
| Clic dans un creux (pont/air sous les niches) | Sans effet — la fenêtre capte le clic mais aucun widget ne s'y trouve ; comportement acceptable, à vérifier en session. |
| Compacité | Repli conducteur : la niche se rétracte, le débordement s'ancre à la niche droite. |

**Phase 2 — profondeur centrale** : niche centrale à 60 (`fenêtre` 64, zone
exclusive 64), horloge sur deux lignes dans la niche profonde ; touches
d'espaces natives segmentées. Requiert que le path dépasse la lame — même
canvas, fenêtre plus haute ; aucun changement du conducteur (la hauteur est
déjà un défaut par style). Chez minflair, la niche centrale sert aussi
d'hôte étendu (mini lecteur, `ConvexCenterPanel.qml`) et les cartes reprennent
l'encoche du bord (`ConvexDrawerShape.qml`) : deux évolutions candidates,
à ne traiter qu'après la phase 1 validée.

## Adaptation et plusieurs écrans

Une barre par écran sélectionné : niches mesurées localement, portrait et
compact indépendants. Sur écran étroit, l'ordre de repli du conducteur
s'applique ; quand deux niches se touchent (`G + C + D + padding > W`), le
pont entre elles disparaît et les niches **fusionnent** (le path relie deux
angles d'évasement sans pont) — une niche fusionnée ne se re-divise pas au
survol. Multi-écrans : chaque barre sculpte son propre layout ; aucune
coordination de ponts entre écrans.

## Animations et mouvement réduit

Survol 140, pression 90, commutation 180 (`TOKENS`). **Le path ne s'anime
pas** : les niches se déplacent quand la mesure change, par transition de
layout courte (160 ms) ou immédiatement en `transition = "none"` — jamais en
respiration continue. Redessins locaux uniquement pendant les gestes et les
changements de layout ; au repos, aucune image produite.

## Reuse

- Hook de surface par zone (introduit par Island, `bar_style/mod.rs`) : le
  canvas Convex se pose par lui.
- `bar_motion` (capsule glissante Rail), `urgent_mark`, mesure intrinsèque
  et centrage du conducteur.
- Chrome plat de `graphite::module_button` (réutilisé tel quel).
- Défauts de géométrie par style (44→48, haut, 0).

## Changes

1. `config/model.rs` : variante `BarStyle::Convexe` (id Lua `"convexe"`,
   libellé « Convex »), défauts 48/haut/0, préset (mêmes zones qu'Island :
   `workspaces`+`window` à gauche, `clock` au centre, droite Continuité) ;
   réconcilier les tests.
2. `config/lua.rs`, `settings.rs`, `cli_config.rs` : choix Convex via
   `BAR_STYLES` ; vérifier Lua, CLI et Réglages.
3. `ui/bar_style/mod.rs` : dispatch Convexe (tokens, module_button → chrome
   Graphite, rail_look, ink, zone_surface → canvas).
4. `ui/bar_style/convexe.rs` : le path (jetons ci-dessus), filet, ombre,
   cache de rendu invalidé sur thème/layout/taille ; fusion de niches.
5. `locales/en.toml` : « Convex » = "Convex" ; chaînes d'aide ; crédit
   « inspiré de minflair » dans le CHANGELOG.
6. `ui/plugins/bar.rs`, `ui/preview.rs`, `scripts/verify.sh` : description et
   scènes jumelles (ci-dessous).

## Scope

- Inherit : conducteur, modules, panneaux, débordement, Rail, dock autonome,
  hook par zone d'Island.
- Verify : aller-retour entre les quatre styles sans résidu (zone exclusive,
  dock, canvas libéré), reload, bureau masqué, `launcher-only`, portrait,
  deux écrans à styles différents.
- Exclude : cadre plein écran et hosts bas de minflair, exclusions plein
  écran, niche centrale profonde et touches natives segmentées (phase 2),
  intégration du lanceur.

## Validation

- Interface : scènes `bar-convex-idle`, `bar-convex-pressed`,
  `bar-convex-switch`, `bar-convex-urgent`, `bar-convex-merge` (deux niches
  fusionnées, écran étroit) ; 1440/900/640, thème clair, échelle 1,5.
  **Contrôle dédié** : à 640, vérifier que les ponts de 4 px restent
  lisibles ; si un pont se perd visuellement, remonter à 8 — ne pas
  épaissir les filets.
- System : `bash scripts/verify.sh --visual /tmp/hyprdesk-convex-validation`
  après implémentation ; test du cache canvas (aucun redraw au repos, traces
  `HYPRDESK_LOG=warn,hyprdesk::ui=trace`) ; tests des défauts de géométrie.
- Session : fenêtres sous 48 ; clics dans les ponts sans effet ni artefact ;
  flou Hyprland sur les niches.

Le résultat est réussi si, au premier coup d'œil, la barre se lit comme
**taillée dans le bord** — la silhouette organique suffit, sans couleur ni
animation continue, et les niches suivent le contenu réel plutôt que l'inverse.

## Stop conditions

- Le canvas par zone dépasse les capacités du hook (clipping, ombre aux
  angles concaves) : étendre le contrat du hook **une fois**, documenté pour
  les deux styles, plutôt que de spécialiser `view_bar()`.
- Le path à petite largeur produit des ponts instables (< 8 px entre deux
  évasements) : fusionner les niches plutôt que compresser les courbes.
- Les ombres concaves demandent un rendu non disponible dans le renderer :
  livrer sans ombre dans les creux (filet seul) avant d'ajouter un passe-rendu.
- Modifications concurrentes du hook Island : réconcilier les deux plans.

## Design documentation

Après implémentation : définition Convex dans `DESIGN.md` (§ Styles de la
barre), gallery README, aide Lua et traductions ; crédit d'inspiration
minflair (GPL-3.0). Cette étude modifie uniquement `design-plans/`.
