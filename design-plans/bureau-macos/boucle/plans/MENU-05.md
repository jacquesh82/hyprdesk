# Menus de la barre : un menu « Aide » en dernier, avec un champ de recherche en tête (HIG MENU-05)

Écrit contre : `9955515` (intégration `hig/plan-d-action`)

## Chaîne de preuve

- Surface : les titres de menus en tête de la barre (`titles()`, `src/ui/plugins/bar/menus.rs`), tous styles de barre, et la carte qu'ils ouvrent.
- Problème : la barre n'a aucune entrée d'aide. La seule commande d'aide du shell est « Visite guidée », dans l'en-tête des Réglages (`src/ui/panel.rs`, bouton `Visite guidée`, `PanelMsg::Tour(TourMsg::Start)`).
- Preuve de design : `03-menus-and-actions.md`, the-menu-bar › Help menu (« The Help menu — located at the trailing end of the menu bar — provides access to an app's help documentation »), the-menu-bar › Anatomie et cotes (champ de recherche en tête). Décision du propriétaire D1 : « hyprdesk », puis Fenêtre, Espaces et Aide.
- Inexactitude de la preuve, à reporter en remarque du registre : « Documentation » dans `src/ui/plugins/launcher/grid.rs` (`CATEGORIES`) est une **catégorie d'applications** (Bureautique : Office, TextEditor, Documentation), non une commande de documentation. L'écart reste réel.
- Propriétaire : `src/ui/plugins/bar/menus.rs` (`titles`, `entries`, `act`, constantes d'action), `popover::Request` et `Msg::OpenPopover` (`src/ui/plugins/bar/popover.rs`, `src/ui/mod.rs`), le composant de menu (`MenuEntry::Input`, `pop_input`, `PMsg::Submit`).
- Portée et surfaces touchées : `menus.rs`, `popover.rs` (une variante de `Request`), `src/ui/mod.rs` (ouverture), `src/ui/preview.rs` (scène), guides, `locales/en.toml`.
- Incertitude : (1) le chemin qui ouvre la recherche des Réglages avec des mots (l'outil `hyprdesk_open_settings` accepte `?mots` : il existe, à retrouver dans le code) ; (2) si la visite guidée se lance sans les Réglages ouverts (`src/ui/tour.rs`) ; (3) ce qu'ouvre « Documentation » : aucun guide installé n'est connu.

## Décision de design

1. **Emplacement** : `titles()` gagne un quatrième titre, en dernier : `("menus:help", "Aide", popover::Request::Help)`. Comme les trois autres, il appartient à la barre : présent dans tous les styles, jamais débordé, jamais déplacé.
2. **Contenu** (`menus::help_entries()`), dans cet ordre :
   - un champ de recherche en tête (`MenuEntry::Input`, sans libellé, texte d'invite « Rechercher dans les réglages ») : la saisie validée ouvre la recherche des Réglages sur ces mots ;
   - un trait ;
   - « Visite guidée… » : la visite des Réglages, lancée comme du bouton de leur en-tête ;
   - « Raccourcis clavier » : la page des raccourcis des Réglages ;
   - « Documentation » : le guide du projet (voir l'incertitude 3) ;
   - un trait, « À propos de hyprdesk » : la même action que dans « hyprdesk » (`ABOUT`), qui garde son entrée — la HIG place « About » dans le menu de l'application, la colonne Correction du registre le demande aussi ici.
3. **Carte** : celle de tous les menus de la barre (`Kind::Menu { source: MenuSource::Shell }`). `act()` gagne les actions du menu `Aide`, numérotées à la suite de celles de « hyprdesk ».

Cela résout la cause : la barre a désormais, en dernière position, un menu d'accès à l'aide, muni du champ de recherche que la HIG donne en tête de ce menu.

## Réutilisation

- `titles()`, `entries()`, `act()`, `ABOUT`, la fermeture `item` de `entries()` (exemplaire : `src/ui/plugins/bar/menus.rs`).
- `MenuEntry::Input` et `pop_input` (champ de saisie d'un menu : libellé, invite, secret) ; `PMsg::Input`, `PMsg::Submit` (`src/ui/plugins/bar/popover.rs`).
- `Msg::Panel(PanelMsg::Tour(tour::TourMsg::Start))` (visite guidée, `src/ui/panel.rs`), `Msg::Control(Command::Settings(page))` (Réglages).
- Scènes « bar-menu-shell », « bar-menu-window » de `src/ui/preview.rs` (idiome : ancre provisoire, icônes résolues tout de suite).

Aucune primitive nouvelle : un titre, un menu de plus, le composant de menu existant.

## Changements

1. `src/ui/plugins/bar/menus.rs`
   - Change : `titles()` (quatrième titre), `help_entries()`, constantes `HELP_SEARCH`, `HELP_TOUR`, `HELP_KEYS`, `HELP_DOCS`, branches de `act()`.
   - Preserve : l'ordre et le contenu de « hyprdesk », « Fenêtre », « Espaces » ; les numéros d'action existants.
   - Verify : test unitaire (quatre titres, « Aide » en dernier ; contenu du menu ; chaque action).
2. `src/ui/plugins/bar/popover.rs`
   - Change : `Request::Help` ; `PMsg::Submit` ouvre la recherche des Réglages pour un menu de source `MenuSource::Shell`.
   - Preserve : `Submit` des menus Lua (texte transmis au Lua), menus de la zone de notification et de la fenêtre.
   - Verify : tests existants ; test de `Submit` sur le menu `Aide`.
3. `src/ui/mod.rs`
   - Change : ouverture de `Request::Help` (contenu fixe, comme `Request::Shell`).
   - Preserve : ouverture des autres menus.
   - Verify : `cargo clippy` (match exhaustif).
4. `src/ui/preview.rs`, `scripts/verify.sh`
   - Change : scène « bar-menu-help », ajoutée aux captures.
   - Verify : capture regardée aux trois largeurs (le titre de plus tient dans la barre à 640 px).
5. Documentation : `docs/GUIDE.fr.md`, `docs/GUIDE.md` (quatre menus), `DESIGN.md` (règle des menus de la barre), `locales/en.toml` (« Aide », « Rechercher dans les réglages », « Raccourcis clavier », « Documentation »).

## Portée

- Héritent : tous les styles de barre (le titre est hors catalogue).
- À vérifier : largeur minimale de la zone amont (`src/ui/plugins/bar/surface.rs`, tests de `menus.rs`), ancres des scènes « bar-menu-shell », « bar-menu-window », « bar-menu-workspaces » (provisoires).
- Exclus : l'aide contextuelle des Réglages, le contenu de la visite guidée, la page « À propos », un menu d'aide dans les autres surfaces (dock, Lanceur).

## Validation

- Produit : la barre montre « hyprdesk », « Fenêtre », « Espaces », « Aide ». « Aide » ouvre : champ de recherche, « Visite guidée… », « Raccourcis clavier », « Documentation », « À propos de hyprdesk ». Une saisie validée ouvre la recherche des Réglages sur ces mots et referme la carte.
- Interface : carte de 328 px, champ en tête, texte d'invite ; entrée sans effet affichée et inactive ; `transition = "none"` respecté.
- Système : une seule façon de dessiner une ligne et un champ de menu ; aucun second type de menu.
- Dépôt : `CARGO_BUILD_JOBS=6 bash scripts/verify.sh` → code de sortie 0 ; `scripts/verify.sh --visual /tmp/hig-visual-C` → la scène « bar-menu-help » se regarde.

## Conditions d'arrêt

- Arrêter si aucun chemin n'ouvre la recherche des Réglages avec des mots : le champ filtrerait alors les seules entrées du menu, ce qui est un autre design, à revoir.
- Arrêter si la visite guidée exige des Réglages ouverts sans qu'on puisse les ouvrir depuis la barre : « Visite guidée… » n'aurait pas d'effet fiable.
- « Documentation » : si aucun guide n'est installé par le paquet (à relever dans la recette d'installation), l'entrée reste affichée et inactive, et l'écart le note en remarque — jamais un lien vers un emplacement qui n'existe pas.

## Documentation de design

- Après acceptation : `DESIGN.md`, règle des menus de la barre (quatre titres, « Aide » en dernier avec son champ de recherche) ; guides des deux langues.

## Lecture du plan, à froid

Pas d'agent indépendant disponible ; relu après l'écriture du registre.

- Qu'est-ce qui casse ? Un titre de plus élargit la zone amont de la barre : à 640 px, les modules d'aval perdent la place qu'il prend ; les scènes « bar-menu-shell », « bar-menu-window » et « bar-menu-workspaces » posent des ancres provisoires, à décaler. `PMsg::Submit` n'agit aujourd'hui que pour les menus Lua.
- Que manque-t-il ? Le chemin exact qui ouvre la recherche des Réglages avec des mots, l'exigence de la visite guidée sur des Réglages ouverts, la destination de « Documentation ». Les trois sont des conditions d'arrêt : à lever en premier, avant d'écrire le code.
- Le plan résout-il l'écart ? Oui si la recherche des Réglages s'ouvre depuis la barre : la barre a un menu « Aide » en dernier, avec le champ de recherche en tête et les entrées d'aide demandées. Sinon il s'arrête, et l'écart reste « à faire ».
