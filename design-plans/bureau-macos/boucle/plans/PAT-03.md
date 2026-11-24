# Par-dessus une fenêtre plein écran, la barre se retire au bord et paraît au survol

Écrit contre : `d0d3796` (branche `hig/plan-A`)

## Chaîne de preuve

- Surface : la barre de chaque écran (`src/ui/plugins/bar/surface.rs`, `sync_bars`, `view_bar`, `view_bar_vertical`, `bar_hover`, `bar_reveal_apply`).
- Problème : par défaut (`BarVisibility::Always`, `BarLayer::Top`), une fenêtre en vrai plein écran passe par-dessus la barre ; aucune bande de rappel n'existe hors `AutoHide`. Ses menus (D1), l'horloge, les réglages rapides sont alors inaccessibles sans quitter le plein écran.
- Règle : going-full-screen › Best practices — « Continue to provide access to essential features and controls so people can complete their task without exiting full-screen mode. »
- Propriétaire : `sync_bars` (calque et zone réservée à la création), `BarSurface`, la mécanique « masquée hors survol » de la barre (MLG-277 : `bar_reveal`, `BarReveal`, `bar_reveal_apply`, `reveal_zone`).
- Exemple : PAT-01 (`e7b7e47`), le même traitement pour le dock : `fullscreen_on(monitor)`, `fullscreen_spaces` relu à chaque `Update::Clients`, `dock_covered`, `dock_cover_changed`, `dock_hides`, surface recréée sur `Overlay`.
- Incertitude : aucune source nouvelle ; Convex masqué hors survol garde son propre geste (exclu, comme pour le dock).

## Décision

Quand l'espace affiché d'un écran porte une fenêtre en vrai plein écran (`fullscreen_on`), la barre de cet écran est recréée sur le calque `Overlay`, sans zone réservée, réduite à sa bande de rappel (`shape.reveal_zone`), et se montre au survol de ce bord, puis se retire, exactement comme une barre `AutoHide` (mêmes délais `show_delay`, `hide_delay`, menu ouvert qui la tient). Le plein écran parti, elle revient à sa forme réglée. Exclus : une barre déjà sur `Overlay` (visible par-dessus, choix de l'utilisateur) et Convex masqué hors survol (son geste propre).

## Réutilisation

- `fullscreen_on`, `fullscreen_spaces` (PAT-01) ; `bar_reveal`, `BarReveal`, `bar_reveal_apply`, `reveal_zone` (MLG-277).
- Exemple : `src/ui/plugins/dock/surface.rs` (`dock_covered`, `dock_cover_changed`, `dock_hides`, recréation dans `sync_launchers`).

## Changements

1. `src/ui/plugins/bar/surface.rs`
   - `BarSurface::over_fullscreen` ; `bar_covered(monitor, shape)` (plein écran, calque ≠ `Overlay`, pas Convex masqué) ; `bar_cover_changed()` ; `bar_hides(id)` = `AutoHide` ou `over_fullscreen`.
   - `sync_bars` : recrée une surface dont `over_fullscreen` ne correspond plus ; à la création couverte : calque `Overlay`, `exclusive_zone` 0, taille initiale `reveal_zone`, `bar_reveal` posé à masqué.
   - `view_bar`, `view_bar_vertical`, `bar_hover`, et le rendu final (`mouse_area` de révélation) testent `bar_hides(id)` au lieu de `visibility == AutoHide`.
   - Préserver : `AutoHide` réglé, `reserve_space`, formes, Convex, barre sur `Overlay` ou `Bottom` hors plein écran.
   - Vérifier : test unitaire de `bar_covered` (calque, Convex, plein écran) et que `bar_hides` vaut `AutoHide` hors plein écran.
2. `src/ui/mod.rs` : là où `dock_cover_changed()` relance `sync_launchers` (`Update::Clients`, changement d'espace affiché), `bar_cover_changed()` relance `sync_bars`.
3. Documentation : `docs/GUIDE.fr.md`, `docs/GUIDE.md`, à côté du paragraphe du dock en plein écran (PAT-01).

## Relecture du plan (agent indépendant), prise en compte

- Taille de création couverte : `(width, reveal_zone)`, la largeur réglée ou mesurée est gardée (comme `bar_reveal_apply`).
- La recréation ferme un menu ouvert (`destroy_popover` quand sa barre disparaît) : accepté et documenté au guide ; l'entrée en plein écran est un changement de contexte.
- Le capteur du menu (`open_popover`) passe sur `Overlay` quand sa barre est couverte : sinon un clic hors du menu irait à la fenêtre plein écran.
- Le `BarReveal(id, false)` d'`AutoHide` est sauté pour une barre couverte, née masquée.
- Convex non masqué : son cadre reste sur `Top`, sous la fenêtre ; la barre seule passe dessus, comme le dock en PAT-01.
- `Update::Monitors` appelle déjà `sync_bars` ; le relais par `bar_cover_changed` y reste, sans effet de bord (aucune recréation si rien n'a changé).

## Portée

- Hérite : toutes les barres d'écran, horizontales et verticales.
- Vérifier : barre `AutoHide` (inchangée), menus de la barre ouverts (la tiennent visible), `bar_full_width`/`bar_fit_content` (largeur conservée), scènes `bar-*` de l'aperçu (aucun plein écran simulé : inchangées).
- Exclure : Convex masqué hors survol, barre réglée sur `Overlay`, Panorama (pas de barre).

## Validation

- Produit : une fenêtre en plein écran (`fullscreen 2`) : la barre disparaît sous 4 px au bord ; le pointeur au bord la fait paraître par-dessus la fenêtre ; elle se retire en partant ; quitter le plein écran la rend à sa place, zone réservée comprise.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh` → 0.

## Conditions d'arrêt

- S'arrêter si la recréation de la barre fait perdre un état utile (menu ouvert, largeur mesurée) d'une façon que le plan ne couvre pas : le noter et marquer « à reprendre ».

## Documentation

- Guides FR et EN : la barre, comme le dock, se retire au bord en plein écran.
