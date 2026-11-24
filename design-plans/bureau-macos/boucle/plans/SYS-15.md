# Les widgets du bureau ont des tailles qui ajoutent de l'information

Écrit contre : `a45173e` (branche `hig/plan-A`)

## Chaîne de preuve

- Surface : widgets du bureau (`src/ui/widget_view.rs`, `view_widget`), leur réglage (Réglages › Widgets, `src/ui/plugins/widgets.rs`), leur déclaration (`src/config/lua.rs`, options communes `src/widgets/mod.rs` `COMMON_OPTIONS`), leurs réglages enregistrés (`src/settings.rs`).
- Problème : un widget n'a qu'une taille. `size` (« Taille du texte ») agrandit toute la mise en page d'un même facteur : horloge, calendrier, système et média grossissent sans rien montrer de plus.
- Règle : quatre tailles de widget sur Mac (widgets › System family widgets) ; « Avoid expanding a smaller widget's content to simply fill a larger area. » (widgets › Best practices).
- Propriétaire : `WidgetDef` (`src/config/model.rs`), `view_widget`, le détail d'un widget dans `tab_widgets`.
- Portée : les quatre widgets intégrés qui affichent des données (horloge, calendrier, système, média). Lanceur, réglages rapides, commande, Lua : une seule taille, inchangée.
- Incertitude : les couches d'information ne viennent que de sources déjà en place (aucune dépendance) ; deux demandent d'élargir une lecture existante, nommées aux étapes 4 et 5.
- Relecture indépendante (corrigée) : `MediaInfo` n'a ni album ni position ni durée — ils sont dans les mêmes propriétés MPRIS, lues par `media::Status` : à porter dans `media_from_props` ; le relevé système n'échantillonne que `show` : charge et durée de fonctionnement s'y ajoutent selon la taille, qui entre donc dans `same_content` ; « disques montés » n'a pas de source (aucune énumération) : remplacé par le disque racine, `SysItem::Disk("/")` existant ; l'étape 1 doit passer par `Placement` (`placement()`, `set_placement`, `placement_fields`, `hd.place`, `edit_placement`, `WidgetReset`) ; `family` n'entre pas dans `COMMON_OPTIONS`, réservées aux plugins, mais dans les options propres des quatre widgets intégrés.

## Décision

Un champ `family` par widget, à quatre valeurs — `small`, `medium`, `large`, `xlarge` (Petit, Moyen, Grand, Très grand) —, `medium` par défaut, qui reproduit exactement l'affichage actuel. `size` reste la taille du texte, indépendante. Chaque taille ajoute une couche, sans agrandir celle d'avant :

| Widget | Petit | Moyen (actuel) | Grand | Très grand |
| --- | --- | --- | --- | --- |
| Horloge | l'heure | l'heure, la date (si `clock.date`) | + le jour de l'année et la semaine (`%j`, `%V`) | + la grille du mois (vue du calendrier) |
| Calendrier | le jour et sa date | la grille du mois | + le mois suivant | + les deux mois suivants |
| Système | la première jauge | les jauges choisies (`show`) | + charge et durée de fonctionnement | + le disque racine (`/`) |
| Média | pochette et lecture | l'actuel (pochette, titre, commandes) | + la progression (position / durée) | + l'album et le lecteur |

## Réutilisation

- `WidgetDef`, `COMMON_OPTIONS`, la lecture Lua des options communes ; `Settings` (widgets placés dans les réglages) ; les compositions existantes de `view_widget` (grille du calendrier `view_calendar`, jauges `SysLine`, lecteur `Value::Media`).
- Le sélecteur à boutons radio déjà utilisé pour l'ancrage dans le détail d'un widget.

## Changements

Voir « Étapes » ; état : étape 1 faite (d3a58fe), « Rétablir » garde la taille (c'est le contenu du widget, pas sa place), `hd.place` ne pose pas de taille sur un widget à une seule taille ; étape 2 faite (7add7b8, relue à froid : le choix passe par `edit_placement`, donc `hd.place` pour un widget de hyprdesk.lua ; pas de test d'interface, faute de banc pour ce détail) ; prochaine : étape 3.

## Portée

- Hérite : widgets du bureau et volets « widget: » du Panorama (même `view_widget`).
- Vérifier : `size`, cartes ou texte nu, échelle du texte, aperçus de la galerie (SYS-11), placement (`desk_layout` mesure les widgets).
- Exclure : widgets de la barre (`in_bar`), plugins.

## Étapes

1. **Modèle** : `WidgetFamily` (`Small`, `Medium`, `Large`, `XLarge`, défaut `Medium`) dans `src/config/model.rs`, champ `family` de `WidgetDef` (les onze littéraux) et de `Placement` (`placement()`, `set_placement`, `placement_fields` de `src/settings.rs`, clés et correctif de `hd.place`, `apply_places`, `edit_placement`, `WidgetReset`) ; option propre `family` des widgets `clock`, `calendar`, `system`, `media` (`Builtin::options`), pas commune ; `same_content` la compte. Lua : `hd.widget("clock", { family = "large" })`, erreur claire sur une valeur inconnue. Tests : lecture Lua, aller-retour des réglages et de `hd.place`, défaut `Medium`, `WidgetReset`. Aucun changement visible.
2. **Réglage** : dans le détail d'un widget (Réglages › Widgets), « Taille du widget » à quatre choix, pour les quatre widgets concernés seulement ; `WidgetFamily` passe par un message du plugin ; libellés traduits. Test : le choix se pose et s'enregistre.
3. **Horloge et calendrier** : couches du tableau dans `view_widget` ; `Medium` identique à l'actuel (test de non-régression sur la composition), scènes d'aperçu `widgets-tailles`.
4. **Système** : relevé élargi selon la taille (charge, durée de fonctionnement, disque racine ajoutés à `show` sans doublon, dans `sample_system`) ; couches du tableau ; mêmes vérifications.
5. **Média** : `MediaInfo` porte album, position et durée, lus des mêmes propriétés MPRIS (`media_from_props`) ; couches du tableau ; mêmes vérifications.
6. **Documentation et galerie** : guides (FR, EN), `docs/PLUGINS.md` (option commune), aperçu de la galerie à la taille moyenne ; l'écart passe `fait`. Aperçus de la galerie à la taille moyenne seulement (données simulées existantes).

## Validation

- Produit : passer l'horloge de Petit à Très grand ajoute la date, la semaine, puis le mois, sans grossir l'heure.
- Système : aucune primitive nouvelle ; `size` garde son sens.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh` → 0 à chaque étape ; `--visual` aux étapes 3 à 5.

## Conditions d'arrêt

- S'arrêter si une couche demande une source de données nouvelle ou une dépendance : la retirer du tableau et le noter.

## Documentation

- Guides et `docs/PLUGINS.md` à l'étape 6.
