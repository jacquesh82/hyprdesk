# Un clic sur le fond d'un widget du bureau ouvre son contenu

Écrit contre : `b7dca06` (branche `hig/plan-A`)

## Chaîne de preuve

- Surface : widgets du bureau (`src/ui/widget_view.rs`, `App::view_widget`).
- Problème : hors Réglages ouverts, le fond d'un widget ne reçoit aucun clic ; la zone de clic n'existe que pour la sélection pendant l'édition. Seuls les boutons du lecteur média agissent.
- Règle : « People tap or click a widget to launch its corresponding app. » ; « When people interact with your widget in areas that aren't buttons or toggles, the interaction launches your app. » (widgets › Adding interactivity).
- Propriétaire : `view_widget` (zone de clic), `App::update` (nouveau message), les ouvertures existantes : `open_menu_by_type` (carte Calendrier ancrée à l'horloge de la barre), `Msg::OpenPopover(.., Request::Calendar)`, `control::Command::Settings`, `widgets::media_control` (MPRIS).
- Portée : horloge, calendrier, système, média. Lanceur et réglages rapides sont faits de commandes : inchangés. Les widgets Lua et commande n'ont pas de contenu lié connu : inchangés.
- Relecture indépendante (corrigée) : `media_control` est câblé sur l'interface `Player` : fonction à part ; ouverture à l'appui (un relâché suivrait aussi un glissé venu d'ailleurs, une icône du bureau par exemple) ; les boutons capturent leur appui (`mouse_area` met l'enfant à jour d'abord).
- Incertitude : `Raise` n'est pas implémenté par tous les lecteurs MPRIS (`CanRaise`) ; un échec est journalisé, sans effet visible.

## Décision

Réglages fermés, le fond de chaque widget intégré qui a un contenu lié devient une zone de clic (curseur main) qui l'ouvre : horloge et calendrier ouvrent la carte Calendrier de la barre de l'écran actif (ancrée à son horloge si elle en a une), système ouvre Réglages › Performance, média met le lecteur au premier plan (`Raise` de `org.mpris.MediaPlayer2`). Les boutons du widget gardent leur clic (ils capturent l'événement). Réglages ouverts, rien ne change : le clic sélectionne.

## Réutilisation

- `open_menu_by_type("clock")` et `focused_bar()` (`src/ui/mod.rs`) ; `Msg::OpenPopover` ; `Msg::Control(Command::Settings("performance"))`.
- MPRIS de `src/widgets/media.rs` : une fonction `media_raise` à côté de `media_control` (interface racine), sans relevé forcé.
- `mouse_area` comme la sélection d'édition.

## Changements

1. `src/ui/widget_view.rs` : `App::widget_link(w) -> bool` ; quand `self.panel` est `None` et qu'un lien existe, `mouse_area(boxed).on_press(lien).interaction(Pointer)`.
2. `src/ui/mod.rs` : `Msg::WidgetOpen(String)` → `widget_open(id)` : calendrier par `OpenPopover(barre active, clé, Calendar)`, la clé étant l'identifiant de l'horloge de la barre (ancrage), sinon `clock` (centre de la barre, repli de `open_popover`) ; sans barre, rien. Aussi les volets « widget: » du Panorama, qui passent par `view_widget`.
3. `src/widgets/media.rs` : `media_raise(bus)` appelle `Raise` sur l'interface racine ; aucun lien quand aucun lecteur ne joue (`Value::Media(None)`).
4. Documentation : `docs/GUIDE.fr.md`, `docs/GUIDE.md`, paragraphe des widgets.

## Portée

- Hérite : tous les écrans, widgets en carte ou nus.
- Vérifier : édition (Réglages ouverts) inchangée ; boutons média ; mode bureau masqué (widgets non affichés).
- Exclure : widgets Lua et commande, lanceur, réglages rapides.

## Validation

- Produit : un clic sur l'horloge du bureau ouvre le calendrier de la barre ; sur le widget système, Réglages › Performance ; sur la pochette ou le titre du média, le lecteur.
- Système : aucune primitive nouvelle ; ouvertures existantes.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh` → 0 ; test : `widget_link` donne le bon message par sorte, aucun pendant l'édition.

## Conditions d'arrêt

- S'arrêter si la zone de clic avale les boutons du lecteur ou la sélection d'édition.

## Documentation

- Guides : le clic sur un widget ouvre son contenu.
