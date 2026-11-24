# PAT-13 — Le lanceur trouve un réglage et ouvre le tiroir dessus

Écrit contre : `ad444a1` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : lanceur (`SUPER+Espace`, décision D2), carte du lanceur ; tiroir des Réglages.
- Problème : un réglage ne se trouve que depuis le tiroir, par son propre champ (`view_panel`, `src/ui/panel.rs`). Le lanceur, lieu unique de recherche du bureau, connaît quatre vues — applications, presse-papiers (`:clip`), mots de passe (`:2fa`, `:mdp`), fichiers (`:f`) — et aucune ne touche aux réglages (`Search::view_kind`, `src/ui/search.rs`).
- Preuve de design : HIG searching › Best practices — « Aim to make your app's content searchable through a single location ».
- Propriétaires : `src/ui/settings_search.rs` (index des réglages : `search`, `ranges`, `block`), `src/ui/search.rs` (le lanceur : `Search`, `view_kind`, `search_update`, les vues par genre), `src/ui/panel.rs` (`PanelMsg::Search`, `PanelMsg::SearchOpen`, `open_panel`, `panel_update`).
- Périmètre et surfaces touchées : `src/ui/settings_search.rs`, `src/ui/search.rs`, `src/ui/panel.rs`, `src/ui/preview.rs`, `scripts/verify.sh` (une scène), `locales/en.toml`, `docs/GUIDE.fr.md`, `docs/GUIDE.md`.
- Incertitude : la HIG ne dit pas si les réglages doivent paraître dans les résultats généraux (comme sous Spotlight) ou derrière un préfixe. Retenu : un préfixe, comme les trois autres sources du lanceur — la vue des applications, sa fonction principale, ne change pas. Montrer les réglages sans préfixe est une autre décision, à proposer au propriétaire une fois ce mode en place.

## Décision de design

- Un cinquième genre de vue au lanceur, **réglages**, ouvert par le préfixe `:r` (forme longue : `:reglages`) : `:r fond d'écran`, `:reglages opacité`.
- La liste est celle de `settings_search::search`, avec le filtre de visibilité du tiroir (paquet actif, configuration avancée assez large) : ce que le lanceur trouve est ce que le tiroir peut montrer.
- Chaque ligne : l'icône du paquet, le texte du réglage (mots cherchés en caractère de titre et en texte plein, comme `App::marked`), le nom du paquet en texte secondaire. Entrée ou clic : le lanceur se ferme, le tiroir s'ouvre sur le paquet, son champ porte la requête et les mots cherchés y sont surlignés — le réglage apparaît dans son contexte.
- Sous `:r` sans mot : une seule ligne d'invite, pas de liste (`search` ne rend rien pour une requête vide).
- Sans préfixe, rien ne change : applications, presse-papiers, mots de passe et fichiers se comportent comme aujourd'hui.

Pourquoi cela résout la racine : le lanceur devient le lieu unique d'où l'on atteint aussi un réglage, sans second index et sans dupliquer la recherche du tiroir.

## Réutilisation

- `settings_search::search`, `ranges`, `Entry { page, tab, text, section }` : l'index et le surlignage existants.
- Vue presse-papiers (`view_clips`) : même liste de lignes, même clavier (flèches, Entrée), même état vide.
- Analyse de préfixe : règle de `fileindex::proto::query` (le préfixe est suivi d'une espace ou de la fin du champ : `:r` seul est le mode, `:rouge` ne l'est pas).
- Ouverture : l'enchaînement de `control::Command::Tour` et de `Command::Settings("?…")` dans `src/ui/mod.rs` — `let o = self.open_panel(); Task::batch([o, Task::done(Msg::Panel(PanelMsg::Search(q))), Task::done(Msg::Panel(PanelMsg::SearchOpen(page, tab)))])` — précédé de `let close = self.close_search();` (idiome de `search_update`). `Search` pose la requête, `SearchOpen` quitte la liste des résultats et montre le paquet ; l'ordre compte.
- État vide : « Aucun réglage ne correspond à « {q} ». » (chaîne existante).
- Couleurs : `ink()` du thème (`fg`, `dim`).

## Changements

1. `src/ui/settings_search.rs`
   - Changement : `pub fn query(q: &str) -> Option<&str>` — préfixe `:r` ou `:reglages`, rend la requête sans lui.
   - Préserver : `search`, `ranges`, `block`, l'index.
   - Vérifier : test unitaire sur le modèle de celui de `fileindex::proto::query`.
2. `src/ui/panel.rs`
   - Changement : le filtre de visibilité de `view_panel` (`self.cfg.plugin_on(id) && plugins::visible(id, adv)`) devient `fn setting_visible(&self, id: &str) -> bool`, utilisé par le tiroir et le lanceur.
   - Préserver : `view_panel` filtre exactement comme avant.
   - Vérifier : scène `bureau` inchangée ; recherche du tiroir inchangée.
3. `src/ui/search.rs`
   - Changement : champ `Search::settings: Vec<&'static Entry>` ; `view_kind` rend 4 quand `settings_search::query(&self.query)` est `Some` ; dans `search_update`, `SMsg::Query` remplit `s.settings` par `search(mot, |id| self.setting_visible(id))` ; `SMsg::Launch(k)` en genre 4 ferme le lanceur, ouvre le tiroir et lui passe `Search(q)` puis `SearchOpen(e.page, e.tab)` ; vue `view_settings` sur le modèle de `view_clips`, avec la ligne d'invite sous `:r` sans mot ; le champ garde la loupe et prend un texte de substitution propre au genre (chaîne nouvelle, à traduire), comme aux changements de genre actuels.
   - Préserver : les quatre genres actuels, leur clavier, le bouton Clear, le retour de focus au changement de genre.
   - Vérifier : `:r fond` liste les réglages attendus ; Entrée ouvre le tiroir sur le premier ; `:rouge` reste la vue des applications.
4. `src/ui/preview.rs`, `scripts/verify.sh`
   - Changement : scène `reglages` (requête `:r fond`) ajoutée à `SCENES` et à la liste des captures que la porte visuelle contrôle.
   - Préserver : les scènes existantes et leur ordre.
   - Vérifier : la capture existe aux trois largeurs.
5. `locales/en.toml`, `docs/GUIDE.fr.md`, `docs/GUIDE.md`
   - Changement : traduction de la ligne d'invite et du texte de substitution du champ ; la liste des modes du lanceur (`:f`, `:clip`, `:2fa`, `:mdp`) gagne `:r`.
   - Vérifier : `catalogue_complet` passe.

## Périmètre

- Héritent : rien hors du lanceur et du tiroir.
- À vérifier : les quatre vues du lanceur (scènes `launcher-*`, `recherche`, `presse-papiers`) et le tiroir (scènes `bureau`, `tour`), à 1440, 900 et 640 px.
- Exclus : les réglages sans préfixe dans la vue des applications ; un mode « commandes du shell » au lanceur (autre écart) ; le défilement du tiroir jusqu'au réglage exact (le paquet s'ouvre, les mots sont surlignés).

## Validation

- Produit : `SUPER+Espace`, taper `:r opacité` → des réglages ; Entrée → le lanceur se ferme, le tiroir s'ouvre sur le paquet du premier, « opacité » y est surligné.
- Interface : `:r` sans mot (ligne d'invite seule) ; mot sans correspondance (état vide) ; `:rouge` (vue des applications) ; configuration avancée coupée (les réglages avancés n'apparaissent pas) ; tiroir déjà ouvert sur un autre paquet ; paquet coupé dans Plugins (ses réglages n'apparaissent pas).
- Système : un seul index (`settings_search`), un seul filtre de visibilité (`setting_visible`), aucune liste parallèle.
- Dépôt : `CARGO_BUILD_JOBS=6 bash scripts/verify.sh` → code 0 ; `CARGO_BUILD_JOBS=6 bash scripts/verify.sh --visual /tmp/hig-visual-B` → code 0, scène `reglages` regardée aux trois largeurs.

## Conditions d'arrêt

- Arrêter si fermer le lanceur et ouvrir le tiroir dans le même message fait perdre le focus au tiroir ; si `settings_search::search` ne rend pas hors du tiroir ce qu'il lui rend ; si le périmètre impose de toucher à la vue des applications.

## Documentation de design

- Après acceptation : `docs/GUIDE.fr.md` et `docs/GUIDE.md` (modes du lanceur) ; une ligne dans `DESIGN.md` si le propriétaire entérine « le lanceur est le lieu unique de recherche, réglages compris ».

## Relecture à froid (sans outil d'agent)

Relu après avoir revérifié dans le code les trois appuis du plan : `open_panel` (ouvre sur l'écran du clic, sinon l'écran actif ; un seul tiroir à la fois), l'enchaînement `Command::Settings("?…")` de `src/ui/mod.rs`, et `settings_search::search` (rend une liste vide pour une requête vide).

- **Qu'est-ce qui casse ?** Le passage au genre 4 passe par le même aiguillage que les autres genres (`view_kind` aux lignes 796, 807, 1016 et 1020 : changement de genre, retour de focus au champ) : sans texte de substitution propre, le champ garderait celui des applications — ajouté au changement 3. `Search(q)` du tiroir affiche la liste des résultats ; `SearchOpen` la quitte : inverser les deux ouvrirait la liste, pas le paquet — l'ordre est écrit. Rien d'autre ne bouge : sans préfixe, `view_kind` rend ce qu'il rend aujourd'hui.
- **Que manque-t-il ?** Les traductions des deux chaînes nouvelles (invite, substitution) — elles sont au changement 5, et `catalogue_complet` les réclame. Un plafond de liste : inutile, la carte défile comme celle du presse-papiers. Un test de vue : aucun, la vue n'est pas testée ailleurs ; l'analyse du préfixe, elle, a son test. Le cas « tiroir déjà ouvert sur un autre écran » : `open_panel` le ramène, comportement actuel, à vérifier dans Validation.
- **Le plan résout-il l'écart ?** Oui, pour la correction telle qu'elle est écrite — les résultats des réglages entrent au lanceur par `settings_search`, et le tiroir s'ouvre sur le réglage choisi. Limite, dite en Incertitude : c'est derrière un préfixe ; les réglages dans les résultats généraux, à la Spotlight, sont une autre décision, qui n'appartient pas à cet écart.

## Relèvements à l'exécution (itération B15)

Exécuté comme écrit, à quatre points près que le code a imposés :

- **Plafond de liste.** La relecture ci-dessus disait la carte défilante : seule la carte du mode convexe l'est (`scrollable` dans `search_card`) ; la carte standard enveloppe la liste d'un simple conteneur (`tokens::content`), et le presse-papiers est plafonné par `rank_clips`. La liste des réglages est donc plafonnée à `SHOWN` (huit lignes), comme les quatre autres modes.
- **Classement.** Huit lignes seulement, et l'ordre du tiroir groupe par paquet : pour « fond », les huit premières entrées seraient celles des premiers paquets, sans la page « Fond d'écran ». `settings_search::launcher` garde l'index et le filtre de `search`, et classe par nature (page, onglet, libellé, aide), puis par libellé le plus court. Le tiroir garde son groupement.
- **Textes longs.** Une rangée tient sur une ligne (`ROW`) : un libellé de plus de 44 caractères — les textes d'aide — est coupé d'une ellipse, après traduction.
- **Découverte.** Le texte de substitution du champ des applications, du presse-papiers et du coffre liste `:r`.

`marked` s'est partagé en `marked_with` (requête donnée) : le surlignage du lanceur est celui du tiroir, sur la requête du lanceur. Le focus clavier du tiroir après l'ouverture n'a pas pu être regardé à l'exécution (la porte ne lance pas le démon) : il suit le chemin de `hyprdesk settings ?mots`.
