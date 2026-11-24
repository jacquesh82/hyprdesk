# DAT-07 — Des astuces courtes, près de leur surface, une fois par fonction et au plus une par jour

Écrit contre : `ec686f2` (intégration `hig/plan-d-action`).

## Relecture du plan

Relu par un agent indépendant avant exécution. Corrections retenues : (1) le lanceur a trois rendus (carte, feuille plein bureau, surface convexe) et une hauteur fixe — l'astuce se pose donc dans la surface plein écran du lanceur, au bas, hors de la carte, en bulle de verre (`glass::card`, rayon 10), et non sous le champ ; (2) les préfixes `:f`, `:r`, `:clip` sont traités par des bras gardés de `search_update` : le marquage `used` se fait avant le `match` (`search_update` appelle `search_dispatch`) ; (3) l'écriture réunit l'état et le fichier (`State::merge`, temporaire puis renommage) : deux écritures dans le désordre ou deux instances ne font rien reculer, et l'ouverture relit le fichier avant de choisir ; (4) l'aperçu des espaces ne réserve que la rangée d'aide (`overview_layout`, `HINTS_H`) et relit ses rectangles au dépôt : son astuce passe à l'étape 2, avec le dock et la barre. En aperçu et en test (`App::preview`), l'état n'est ni lu ni écrit. Diff relu de même : l'astuce se choisit à la première image visible (`SMsg::Shown`), seulement si le lanceur s'ouvre sans requête (`hyprdesk search`, `:clip`… l'auraient consommée sans la montrer) et hors mode convexe (la bulle ne suivrait pas le tiroir) ; symboles par `ui::icons::get`, comme le champ ; `:r` reconnu par `settings_search::query` (préfixe long compris) ; un temporaire par écriture.

## Chaîne de preuve

- Surface : lanceur (`src/ui/search.rs`, `src/ui/plugins/launcher/grid.rs::view_spotlight`), aperçu des espaces (`src/ui/overview.rs`), dock, barre.
- Problème : une seule astuce dans tout le shell, figée (« Astuce : réglages ouverts, glissez un widget… », `src/ui/plugins/widgets.rs`) ; ni cadence, ni éligibilité. Les préfixes du lanceur (`:f`, `:r`, `:clip`) ne se découvrent que par le texte indicatif du champ ; l'aide de l'aperçu des espaces dit Naviguer, Ouvrir, Fermer, pas le glisser d'une fenêtre vers un autre espace (`overview.rs`, rangée `hints`).
- Preuve de design : HIG onboarding › Best practices — « Consider providing a collection of context-specific tips instead of a single onboarding flow. » ; offering-help › Creating tips — une ou deux phrases, « once every 24 hours », règles d'éligibilité par événement, « Display a popover tip when you want to preserve the content flow, or an inline tip when you want to ensure that surrounding information is visible », symbole de la fonction.
- Propriétaires : aucun moteur d'astuces. État persistant à l'exemple de `src/bar_offer.rs` (`$XDG_STATE_HOME/hyprdesk/…`, une ligne par fait).
- Incertitude : la barre et le dock sont des surfaces étroites ; une astuce en pop-over y demande une surface de calque (popover de la barre, `bar/popover.rs`). Traité en étape 2.

## Décision de design

- **Moteur pur** `src/tips.rs` : catalogue `TIPS` (identifiant, surface, symbole, texte d'une ou deux phrases), état `State { shown: id → époque, used: ids }` lu et écrit dans `$XDG_STATE_HOME/hyprdesk/tips` (lignes `shown <id> <époque>`, `used <id>`). `State::eligible(surface, now)` rend la première astuce de la surface ni montrée ni « utilisée », et seulement si aucune astuce n'a été montrée depuis 24 heures. Une astuce est montrée une fois : à l'ouverture de sa surface, marquée `shown`, l'écriture partant hors du fil de l'interface. Une fonction utilisée (événement) marque `used` son astuce : elle ne sera plus proposée.
- **Forme : astuce en ligne** (variante de la HIG, l'entourage reste visible) dans les surfaces plein écran, lanceur et aperçu des espaces, et non un pop-over à part : leur contenu est le contexte de l'astuce. Une rangée : symbole de la fonction 16 px, texte 12 px encre secondaire, bouton Fermer (`window-close`) ; fond `Color { a: 0.06, ..fg }`, rayon 10, marge 8 × 12. Pas d'accent (réservé à la sélection et aux actions).
- **Astuces de l'étape 1** :
  - lanceur, `lanceur-fichiers` : « Tapez :f puis un nom pour trouver un fichier, sur tous vos disques. » — utilisée quand une requête `:f` est tapée ;
  - lanceur, `lanceur-reglages` : « Tapez :r puis un mot pour aller droit à un réglage. » — utilisée sur une requête `:r` ;
  - lanceur, `lanceur-presse-papiers` : « Tapez :clip pour retrouver ce que vous avez copié. » — utilisée sur `:clip` ;
  - aperçu des espaces, `apercu-glisser` : « Glissez une fenêtre sur un autre espace pour l'y déplacer. » — utilisée quand une fenêtre est déposée sur un autre espace (`move_task`).
- Lanceur : l'astuce s'affiche sous le champ quand la requête est vide ; la liste perd une ligne pour que la carte garde sa hauteur. Aperçu des espaces : l'astuce prend place au-dessus de la rangée d'aide.
- Scènes d'aperçu et tests (`App::preview`) : pas d'astuce, pour que les captures restent stables.

## Étapes

1. **Fait.** Moteur `src/tips.rs` et ses tests (éligibilité, cadence de 24 h, `used`, union des écritures) ; astuces du lanceur (`:f`, `:r`, `:clip`) en bulle au bas de sa surface ; guides, `DESIGN.md`, `locales/en.toml`.
2. **Fait.** Aperçu des espaces : astuce `apercu-glisser` (déjà au catalogue), place réservée dans `overview_layout` tant qu'elle est là, `used` marqué dans `overview_release` juste avant `move_task`, après le test `same` ; rangée de 48 px (`TIP_H`) au-dessus de l'aide, bouton Fermer (`OMsg::TipClose`) ; test `l_astuce_a_sa_place` ; proposée seulement avec deux espaces au moins et une fenêtre (diff relu par un agent indépendant).
3. Dock et barre : astuce en pop-over près de l'élément (dock : bouton de la grille d'applications ; barre : menu hyprdesk), par la surface de popover de la barre ; l'astuce figée de `widgets.rs` (ligne « Astuce : réglages ouverts… ») passe au moteur.

## Réutilisation

- `crate::i18n::tr`, `App::font`, `crate::ui::tokens::ts`, encres `dim`/`fg` des surfaces ; symbole par l'idiome d'icône de chaque surface (`icon_named` du lanceur).
- Persistance : idiome de `bar_offer::state_path` (`XDG_STATE_HOME`, repli `~/.local/state`).

Pas de primitive visuelle nouvelle : une rangée composée de widgets existants.

## Changements (étape 1)

1. `src/tips.rs` (nouveau, déclaré dans `src/lib.rs`) : `Surface`, `Tip`, `TIPS`, `State::{load, save, eligible, mark_shown, mark_used}`, format texte ; tests.
2. `src/ui/mod.rs` : champ `App::tips: crate::tips::State` (vide et inactif en aperçu) ; méthodes `App::take_tip(surface) -> Option<&'static Tip>` et `App::tip_used(id) -> Task<Msg>` (écriture hors du fil).
3. `src/ui/search.rs` : `Search::tip` posé à l'ouverture ; `SMsg::Query` marque les préfixes utilisés ; `SMsg::TipClose`.
4. `src/ui/plugins/launcher/grid.rs` : rangée d'astuce sous le champ, requête vide.
5. `src/ui/overview.rs` : `Overview::tip` à l'ouverture, rangée au-dessus de l'aide, `used` au dépôt d'une fenêtre, `OMsg::TipClose`.

## Validation

- Produit : premier lanceur ouvert → l'astuce `:f` ; refermé et rouvert le même jour → plus d'astuce ; `:f` tapé → cette astuce ne revient jamais.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh` → 0.

## Conditions d'arrêt

- Arrêter si la hauteur fixe de la carte du lanceur ne peut pas accueillir la rangée sans casser une scène d'aperçu.

## Documentation de design

- `DESIGN.md` : « Une astuce : une ou deux phrases, une fois par fonction, au plus une par jour, en ligne dans sa surface ». Guides : une phrase.
