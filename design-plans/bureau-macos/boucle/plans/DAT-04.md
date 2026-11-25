# DAT-04 — Le dock accepte le dépôt d'une application pour l'épingler

Écrit contre : `55b0b5a` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : dock (`src/ui/plugins/dock/surface.rs`, `view_launcher_content`, carte de verre `glass::card`) ; réception des dépôts (`src/ui/mod.rs`, `DropEvent`, `on_drop`, abonnement `FileHovered` / `FileDropped`).
- Problème : le dock n'a aucune zone de dépôt. Une application glissée depuis un gestionnaire de fichiers (son fichier `.desktop`) ne s'y épingle pas ; seul le menu au clic droit (« Épingler au dock ») le fait.
- Preuve inexacte au registre : « Le shell n'accepte aucun dépôt » est faux. Les surfaces reçoivent déjà les dépôts de fichiers du compositeur (`iced::window::Event::FileHovered`, `FileDropped`, `FilesHoveredLeft`) ; `on_drop` ne les accepte que sur le bureau (icône posée dans la case sous le pointeur). L'écart est réel pour le dock. Effort L maintenu : le transport (`vendor/layershellev/src/dnd.rs`) ne lit l'offre `text/uri-list` qu'au dépôt, et `vendor/iced_layershell/src/multi_window.rs` émet `FileHovered` avec un chemin vide ; il accepte aussi l'action « copie » pour tout fichier. Sans chemin au survol, la cible ne peut pas savoir si elle accepte.
- Preuve de design : HIG drag-and-drop › Best practices — « As much as possible, support drag and drop throughout your app. » ; « Provide feedback when a drop target is valid » (bordure d'accent, comme la carte cible de l'écran des espaces, DAT-08).
- Propriétaires : épinglage (`App::pinned`, `settings.launcher_pinned`, `commit`, comme `CMsg::Pin` dans `src/ui/context.rs`) ; applications connues (`App::apps`, `AppInfo::id` = nom du fichier sans `.desktop`, `src/launch.rs`).
- Incertitude : un glisser interne (grille d'applications du lanceur vers le dock) n'existe pas — pas de glisser entre surfaces dans le shell ; hors de ce lot.

## Décision de design

- Le dock d'un écran (surface de `launcher_surfaces`) devient cible de dépôt pour un fichier `.desktop` d'une application connue (`apps`, id = nom du fichier) et pas encore épinglée.
- **Retour au survol** : `FileHovered` porte désormais son chemin (`DropEvent::Hover(PathBuf)`). Sur un dock, si le fichier est acceptable, `dock_drop = Some((surface, id))` et la carte du dock prend une bordure d'accent de 2 px (l'accent sert ici l'action en cours, comme la cible de l'écran des espaces). Fichier non acceptable : aucun retour, la cible ne s'allume pas.
- **Dépôt** : `FileDropped` sur ce dock → l'application s'ajoute en fin des épinglées (`settings.launcher_pinned`, `commit`) ; l'état de survol s'efface. `FilesHoveredLeft` l'efface aussi.
- Le bureau garde son comportement (icône déposée), inchangé.
- Hors lot : position d'insertion sous le pointeur, dock intégré à la barre Graphite et dock convexe (pas de carte `glass::card`), glisser interne depuis la grille.

## Relecture du plan

Relu par un agent indépendant avant exécution. Retenu : le chemin du survol est vide dans le transport (point 1), d'où l'étape 2 ; plusieurs fichiers déposés : un `FileDropped` par fichier, chacun relu contre les épinglées à jour, tous les `.desktop` valables s'épinglent ; le bureau dessous ne reçoit rien (l'entrée vise la seule surface sous le pointeur) ; le dock masqué se révèle au `CursorMoved` de l'entrée mais ne se referme pas à la sortie d'un glisser sans dépôt (`DndLeave` ne donne pas `CursorLeft`) — comportement antérieur, noté pour l'étape 2.

## Étapes

1. **Fait.** Dépôt : un `.desktop` d'une application connue et non épinglée, déposé sur le dock, l'épingle en fin de dock (`DropEvent` routé vers `dock_drop_event` pour les surfaces du dock, `dock_drop_app` pur et testé) ; la bordure d'accent est en place, allumée par `dock_drop` au survol, inerte tant que le transport ne donne pas le chemin.
2. **Fait.** Transport (`vendor/layershellev`, `vendor/iced_layershell`, correctifs régénérés par `scripts/verifier-vendor.sh --regenerer`, vérifiés sans option) : l'offre `text/uri-list` est lue dès l'entrée (`DndEnter { paths }`), dans un fil à part pour ne pas figer le glisser à chaque surface (Wayland crée une offre par entrée), redite au mouvement si elle arrive après 30 ms, et resservie au dépôt ; un `leave` reçu après `drop` n'émet plus de sortie sans cible, et le dépôt finit lui-même le survol (`FilesHoveredLeft`, même vide ou refusé) ; un `FileHovered` par fichier avec son chemin (vide si l'offre n'en porte pas) ; `DndLeave` émet aussi `CursorLeft`, pour qu'un dock masqué se referme après un glisser sans dépôt. La bordure s'allume. Non fait, noté : refuser l'offre (`accept(None)`) quand la surface n'en veut pas — le transport ne sait pas ce que veut l'application ; le curseur de la source peut donc promettre un dépôt que le dock ignore.

## Réutilisation

- `DropEvent`, `on_drop`, l'abonnement des dépôts existants ; `pinned()` et la séquence de `CMsg::Pin` ; style de la carte (`design::launcher_surface`) dont seule la bordure change.

Pas de primitive nouvelle.

## Changements

1. `src/ui/mod.rs` : `DropEvent::Hover(PathBuf)` ; champ `dock_drop: Option<(Id, String)>` ; `on_drop` route les événements d'une surface de dock vers `dock_drop_event` avant le bureau.
2. `src/ui/plugins/dock/surface.rs` : `dock_drop_app(path) -> Option<String>` (pur sur `apps` et `pinned`) ; `dock_drop_event` (survol, départ, dépôt) ; bordure d'accent dans `view_launcher_content` quand `dock_drop` vise la surface de cet écran.
3. `src/ui/context.rs` : `pinned()` visible du dock (`pub(in crate::ui)`).
4. Test : `dock_drop_app` accepte `…/firefox.desktop` connu et non épinglé, refuse un épinglé, un inconnu, un fichier qui n'est pas `.desktop`.
5. Docs : `docs/GUIDE.fr.md`, `docs/GUIDE.md` (dock), `DESIGN.md` (glisser-déposer).

## Validation

- Produit : glisser `firefox.desktop` depuis un gestionnaire de fichiers sur le dock → la bordure s'allume ; déposer → Firefox est épinglé en fin de dock.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh` → 0.

## Conditions d'arrêt

- Arrêter si les surfaces de calque du dock ne reçoivent pas `FileHovered` (le compositeur ne livre les dépôts qu'aux surfaces qui ont un `wl_data_device` : celui d'iced est commun à toutes ses surfaces, comme pour le bureau).

## Documentation de design

- `DESIGN.md` : « Le dock accepte le dépôt d'une application (fichier .desktop) pour l'épingler ; cible signalée par une bordure d'accent ».
