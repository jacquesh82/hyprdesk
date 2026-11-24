# ENT-10 — Forcer une application à quitter : une boîte qui liste les applications et tue leurs fenêtres

Écrit contre : `61c39a1` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : toutes ; la boîte s'ouvre par raccourci, sans égard au mode du bureau.
- Problème : une application bloquée ne peut pas être quittée depuis le shell. Seul « Fermer la fenêtre » existe (`SUPER + Q`, `hl.dsp.window.close()`), et l'application peut s'y opposer.
- Preuve de design : HIG keyboards › Standard keyboard shortcuts — « Option-Command-Esc — Ouvrir la boîte de dialogue Forcer à quitter. ». Sur macOS, la boîte liste les applications ; on en choisit une, « Forcer à quitter », confirmation, puis elle est tuée.
- Recensement à HEAD (`61c39a1`) : ni boîte ni commande — `forcer`, `force_quit`, `kill_window`, `force-quit` sont absents de `src/ui`, `src/control.rs`, `src/hypr.rs`. La seule fermeture est `hypr::close_window` (`src/hypr.rs:248`), polie : `hl.dsp.window.close({ window = "address:…" })`. `window.kill(` ne sert qu'à reconnaître une liaison déjà posée (`src/essentials.rs:378`). Les lignes du plan (relevées à `00f6d69`) sont exactes à un décalage près.
- Propriétaires : `Keys` et `keymap::actions` (`src/keymap.rs`), `control::Command` (`src/control.rs`), le bloc `Msg::Control` et le routage des touches de `src/ui/mod.rs` ; la carte du commutateur (`src/ui/switcher.rs` : surface exclusive sur le calque overlay, lignes 70-84 ; flèches, Entrée, Échap, `mod.rs:5528`) ; la confirmation en deux temps de « Effacer l'historique » (`src/ui/plugins/notifications.rs`) ; `self.clients`, tenu à jour par `hypr::Update::Clients` (`mod.rs:4374`).
- Périmètre et surfaces touchées : les douze fichiers que touche un raccourci de `Keys` (empreinte de `keys.focus_bar`), `src/hypr.rs`, `src/control.rs`, un nouveau `src/ui/force_quit.rs`, `src/ui/mod.rs`, `src/ui/preview.rs`, `locales/en.toml`, `docs/GUIDE.fr.md`, `docs/GUIDE.md`, `DESIGN.md`.
- Incertitude : Hyprland ne signale pas une application « ne répond pas » — la boîte n'en dira rien. La forme exacte de l'appel Lua qui tue une fenêtre désignée est à lire dans l'API de Hyprland avant d'écrire la fonction (condition d'arrêt).

## Décision de design

- Un raccourci de `Keys`, `keys.force_quit`, défaut `ALT + SUPER + Escape` (modèle Option-Command-Esc ; modificateurs dans l'ordre CTRL, ALT, SHIFT, SUPER, HIG ENT-04) ; `none` pour couper. Partout, comme `keys.settings`. Une commande : `hyprdesk force-quit`, qui ouvre la boîte ou la referme.
- La boîte : une carte centrée sur le calque overlay, clavier exclusif, comme celle du commutateur. Titre « Forcer à quitter ». Une ligne par application — icône de classe, nom, nombre de ses fenêtres —, l'application active choisie d'abord. Deux commandes : « Annuler » et « Forcer à quitter », celle-ci au ton danger (DESIGN.md : une action destructive prend le ton « danger »), sans effet tant qu'aucune ligne n'est choisie.
- Forcer à quitter se confirme en deux temps, comme « Effacer l'historique » : la commande devient « Forcer « Nom » à quitter ? », « Annuler » à côté. C'est le seul chemin : un blocage est rare, une erreur est irréversible.
- Clavier : flèches haut et bas changent de ligne, Entrée force la main sur la commande choisie (ou demande la confirmation), Échap referme — la confirmation d'abord, la boîte ensuite. Aucune fenêtre : la boîte le dit et n'a qu'« Annuler ».
- Forcer une application à quitter = tuer chacune de ses fenêtres (`hypr::kill_window`, par adresse). La liste vient de `self.clients`, groupées par classe : pas de lecture supplémentaire, et la boîte suit `Update::Clients` — l'application disparaît avec sa dernière fenêtre. La confirmation nomme le nombre : « Forcer « foot » à quitter (3 fenêtres) ? » — sous Hyprland, une classe rassemble souvent plusieurs processus.
- Liste défilante, bornée comme celle du commutateur (`SHOWN` lignes visibles). Ouvrir la boîte referme le commutateur : deux cartes à clavier exclusif ne se superposent pas.

## Réutilisation

- `hypr::close_window` (`src/hypr.rs:248`) : `kill_window` s'écrit sur son modèle, même échappement, même journal.
- La carte du commutateur (`src/ui/switcher.rs`) : surface, calque, taille, lignes d'icône et de titre, touche Entrée. La boîte en reprend la mise en page et le rendu des classes (`builtin_icon`, `handle_for`).
- La confirmation en deux temps de « Effacer l'historique » (`src/ui/plugins/notifications.rs`) : même état `confirm`, mêmes deux commandes, même ton.
- Les cartes de `src/ui/menu.rs` et `src/ui/design.rs` pour le fond, le rayon (10, 20, 24 px) et l'espacement de 4 px ; `src/ui/tokens.rs` pour la taille du texte.
- Aucune primitive nouvelle : ni widget, ni opération, ni token.

## Changements

1. `src/hypr.rs`
   - Change : `pub fn kill_window(address: &str) -> Result<(), ipc::IpcError>`, l'appel Lua qui tue la fenêtre désignée, lu dans l'API de Hyprland avant l'écriture ; à défaut d'un argument `window`, `ipc::dispatch("killwindow address:…")`.
   - Preserve : `close_window` inchangé ; l'essentiel « Fermer la fenêtre » reste poli.
   - Verify : test comme `lua_des_raccourcis` : la commande émise porte l'adresse, échappée par `ipc::lua_escape`.
2. `src/config/model.rs`, `src/config/lua.rs`, `src/settings.rs`, `src/cli_config.rs`, `src/ui/key_conflict.rs`, `src/defaults.rs`, `src/assets/hyprdesk.meta*.lua`
   - Change : `keys.force_quit`, lu et écrit en Lua, exposé par `hyprdesk config`, remis par défaut avec le paquet « keys », annoté dans les métadonnées Lua — sur le modèle ligne à ligne de `keys.focus_bar`.
   - Preserve : les options existantes ; vide, aucun raccourci.
   - Verify : `hyprdesk config get keys.force_quit` rend `ALT + SUPER + Escape`.
3. `src/keymap.rs`, `src/hypr.rs`
   - Change : le libellé « Forcer une application à quitter », l'action `force-quit`, portée partout, posée dans les deux tables.
   - Preserve : les indices de `LABELS`, où les portées sont citées par numéro ; le libellé va dans un tableau à part, `DIALOG_LABELS`, chaîné à `LABELS` dans `libelles_traduits`, comme `FOCUS_LABELS`.
   - Verify : test comme `raccourcis_de_focus` ; un conflit avec `keys.leave` (`SUPER + Escape`) est impossible — combinaisons différentes.
4. `src/control.rs`
   - Change : `Command::ForceQuit`, lue par `"force-quit"` ; `as_str`.
   - Preserve : rien d'autre.
   - Verify : `aller_retour_des_commandes`.
5. `src/ui/force_quit.rs` (nouveau)
   - Change : `ForceQuit { win, selected: Option<usize>, confirm: bool }` ; `open_force_quit` (surface exclusive, l'application active choisie), `close_force_quit`, `force_quit_update` (Sélection, Forcer, Annuler), `force_quit_kill` (tue chaque fenêtre de la classe choisie, puis referme), `view_force_quit`. Les applications : `self.clients` groupées par classe, la classe de `active_window` en tête.
   - Preserve : le mode du bureau ; les fenêtres des autres applications.
   - Verify : tests d'état de `App` — ouverte, la boîte choisit l'application active ; deux applications, tuer la première ne ferme que ses fenêtres ; sans fenêtre, `kill` n'est pas appelé.
6. `src/ui/mod.rs`, `src/ui/plugins/keys.rs`, `src/ui/plugins/mod.rs`, `src/ui/panel.rs`, `src/ui/preview.rs`, `locales/en.toml`
   - Change : le bras `Command::ForceQuit`, `Msg::ForceQuit`, les touches (flèches, Entrée, Échap), `listens_to_events`, le routage de la vue ; la rangée du raccourci (case 15 de `State::keys`) ; la scène d'aperçu `forcer-quitter` ; libellés anglais.
   - Preserve : le commutateur et ses touches ; Échap dans les autres surfaces.
   - Verify : `cargo test --locked --lib force_quit` ; `libelles_traduits` reste vert.
7. `docs/GUIDE.fr.md`, `docs/GUIDE.md`, `docs/CONTROL.md`, `docs/PLUGINS.md`, `docs/PLUGINS.en.md`, `README.fr.md`, `README.md`, `DESIGN.md`
   - Change : le raccourci et la boîte, après « Le clavier à chaque surface » ; la commande ; la clé ; dans `DESIGN.md`, la décision (boîte, deux temps, ton danger).
   - Verify : les deux guides disent la même combinaison.

## Ordre

Lire l'API de Hyprland pour `kill` (condition d'arrêt), puis 1 → 2 → 3 → 4 → 5 → 6 → 7. Un seul lot : sans la boîte, le raccourci n'a rien à ouvrir ; sans le raccourci, la boîte est hors d'atteinte.

## Périmètre

- Héritent : la page Raccourcis des Réglages, `hyprdesk keys`, `hyprdesk config`, `binds.lua`, l'inventaire des raccourcis.
- À vérifier : le commutateur (`alt_tab_in_full_desktop`), `keys.leave` et la sortie de secours, les scènes d'aperçu `raccourcis`.
- Hors périmètre : « Ne répond pas » (Hyprland ne le dit pas) ; tuer un processus par son pid ; la fermeture polie de l'application entière ; le menu du shell.

## Validation

- Produit : `ALT + SUPER + Escape` ouvre la boîte sur l'application active ; flèche bas, Entrée : la confirmation ; Entrée : l'application disparaît, ses fenêtres avec ; Échap à chaque étape recule.
- Interface : plein bureau et bureau masqué ; une, dix, aucune fenêtre ; noms longs.
- Système : une seule boîte de ce genre par surface, sur le modèle du commutateur ; aucune primitive parallèle.
- Dépôt : `CARGO_BUILD_JOBS=6 bash scripts/verify.sh --visual /tmp/hig-visual-C` → code de sortie 0.

## Conditions d'arrêt

- Arrêter si Hyprland n'offre aucun appel qui tue une fenêtre désignée par son adresse : marquer « bloqué » avec l'appel trouvé à la place.
- Arrêter si la liste par application s'avère fausse (deux applications d'une même classe) : passer aux lignes par fenêtre, le noter.

## Documentation de design

- Après acceptation : dans `DESIGN.md`, la règle « Une action irréversible du shell se confirme en deux temps : la commande change de libellé, « Annuler » reste à côté. « Forcer à quitter » (ALT + SUPER + Escape) : une ligne par application, ton danger. »

## Relecture à froid (sans outil d'agent)

Relu après la correction du registre, à `61c39a1`.

- Qu'est-ce qui casse ? Trois défauts de la première version, corrigés ci-dessus : une classe Hyprland rassemble plusieurs processus — tuer « foot » tue tous les terminaux —, d'où le nombre de fenêtres dans la confirmation ; la liste n'était pas bornée ; la boîte pouvait s'ouvrir sur le commutateur, deux prises exclusives l'une sur l'autre. Reste : `ALT + SUPER + Escape` n'a pas été vérifié contre les liaisons de hyprland.lua — un conflit sera signalé par `keymap::analyse`, comme pour tout raccourci, et le raccourci n'agira alors que bureau affiché.
- Que manque-t-il ? L'appel Lua exact qui tue une fenêtre désignée : première condition d'arrêt, à lire avant d'écrire. L'état « ne répond pas » : hors périmètre, Hyprland ne le dit pas.
- Le plan résout-il l'écart ? Oui : une boîte qui liste les applications et force leur fermeture, en deux temps, et son raccourci.

## Relecture du diff (itération C20, agent indépendant)

Travail de l'itération interrompue par la panne de quota, repris par `reprise-C.patch` et mis au point. Défauts relevés et corrigés :

- Entrée maintenue confirmait ce que le premier appui venait d'armer : la répétition d'Entrée devient `Msg::EnterRepeat`, sans effet dans la boîte, un appui ordinaire ailleurs.
- La fenêtre d'aperçu du Panorama appartient au démon : la tuer tuait hyprdesk. Elle n'est ni listée ni visée.
- Une fenêtre déjà disparue (processus partagé, tué avec la précédente) n'est plus visée : `hypr::kill_windows` relit les fenêtres avant chaque destruction.
- Le commutateur ouvert par-dessus la boîte la referme, comme l'inverse.
- La ligne affiche le nom de l'entrée `.desktop` (classe en repli), borné à 24 caractères dans la confirmation ; sans image, la pastille de repli du dock.
- L'application active passe en tête sans déranger l'ordre des autres.

Laissé tel quel : l'ordre « Annuler », puis la commande danger à droite — celui de la HIG pour une boîte (le bouton d'action à droite), qui diffère de « Effacer l'historique ».
