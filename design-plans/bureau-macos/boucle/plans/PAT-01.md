# Le dock se rappelle par le bord pendant le plein écran

Écrit contre : `e9869d7` (branche `hig/plan-A`)

## Chaîne de preuve

- Surface : le dock (`src/ui/plugins/dock/surface.rs`, `sync_launchers`), une surface layer-shell par écran.
- Problème : la surface est toujours créée sur le calque `Top`, que Hyprland place sous une fenêtre plein écran. Le masquage hors survol (`dock.autohide`) rétrécit la même surface `Top` : sous un plein écran, aucun geste ne rappelle le dock.
- Règle : « Except in games, let people reveal the Dock while your iPadOS or macOS app is in full-screen mode » (going-full-screen › Best practices).
- Propriétaire : `sync_launchers` (création), `bar_hover` et `bar_reveal_apply` (`src/ui/plugins/bar/surface.rs`, bande de rappel), la vue du dock dans `App::view` (`src/ui/mod.rs`).
- Portée : le dock seul ; la barre est l'écart PAT-03.
- Incertitude : Hyprland ne permet pas de changer le calque d'une surface existante : on la recrée.
- Relecture indépendante (corrigée) : `hasfullscreen` des workspaces vaut aussi pour une fenêtre agrandie (mode 1), sous laquelle les calques `top` restent visibles ; le vrai plein écran (mode 2) se lit dans `clients` (`WindowGeo::fullscreen`), l'événement `fullscreen` relit donc aussi les fenêtres. `sync_bars` effaçait l'état de rappel du dock : il le garde. Le dock masqué qui prend le clavier se montre. Seconde relecture (diff) : il se retire quand il rend le clavier ; Convex masqué hors survol (`convex_auto_hide`) est exclu, sa bande de réveil étant sur le cadre, lui-même sous la fenêtre plein écran.

## Décision

Quand l'espace affiché d'un écran compte une fenêtre plein écran, le dock de cet écran est recréé sur le calque `Overlay`, sans zone exclusive, réduit à sa bande de rappel de 4 px, et révélé au survol du bord, exactement comme le dock masqué hors survol. Le plein écran fini, il revient sur `Top` avec son comportement réglé.

## Réutilisation

- La bande de rappel et ses délais du dock masqué (MLG-279) : `bar_reveal`, `bar_hover`, `bar_reveal_apply`, `Msg::BarReveal`.
- Les géométries de fenêtres déjà lues avec `clients` (`hypr::WindowGeo`), qui portent désormais le vrai plein écran ; `App::fullscreen_spaces` et `fullscreen_on(monitor)` en découlent. `screen_visible` (pause du fond vidéo) reste sur `hasfullscreen`.

## Changements

1. `src/ui/plugins/dock/surface.rs`
   - `LauncherSurface` porte `over_fullscreen` ; `sync_launchers` compare aussi cet état et recrée la surface quand il change : calque `Overlay`, zone exclusive 0, taille de départ 4 px, entrée `bar_reveal` à « masqué ».
   - `App::dock_hides(id)` : `dock_autohide()` ou dock posé sur un plein écran ; remplace `dock_autohide()` pour la zone exclusive et le premier masquage.
   - Préserver : positions, épaisseur, Convex, clavier (`grabbed`), masquage réglé.
2. `src/ui/plugins/bar/surface.rs` : `bar_hover` donne au dock les délais du masquage quand `dock_hides(id)`.
3. `src/ui/mod.rs` : `fullscreen_spaces` mis à jour par `Update::Clients` ; la vue du dock utilise `dock_hides(id)` ; les mises à jour `Clients` et `Monitors` (changement d'espace affiché) relancent `sync_launchers` quand l'état plein écran d'un écran à dock change (`dock_cover_changed`).
4. `src/hypr.rs` : `WindowGeo::fullscreen` (mode & 2) ; l'événement `fullscreen` relit les fenêtres.
5. `src/ui/plugins/dock/keyboard.rs` : le dock masqué qui prend le clavier sans renaître se montre (`bar_reveal_apply`).

## Portée

- Hérite : tous les docks, toutes positions.
- Vérifier : dock masqué réglé, Convex (`convex_auto_hide`), dock incorporé à la barre (aucune surface : inchangé).
- Exclure : la barre (PAT-03), Convex masqué hors survol (son réveil est sur le cadre, à traiter avec PAT-03), les jeux (aucune détection fiable : la bande de 4 px ne gêne pas).

## Validation

- Produit : une fenêtre en plein écran, le pointeur au bord du dock le fait paraître par-dessus ; il se retire en sortant.
- Système : aucune nouvelle primitive ; la bande de rappel est celle du masquage réglé.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh` → 0 ; test : `sync_launchers` recrée le dock sur `Overlay` quand l'espace passe en plein écran, et revient sur `Top` ensuite.

## Conditions d'arrêt

- S'arrêter si la recréation de surface casse le clavier du dock ouvert au raccourci, ou si la vue ne peut plus distinguer la bande de rappel.

## Documentation

- `docs/GUIDE.fr.md` et `docs/GUIDE.md` (dock) : le dock se rappelle par le bord pendant le plein écran.
