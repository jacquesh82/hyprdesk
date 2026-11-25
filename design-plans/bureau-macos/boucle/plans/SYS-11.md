# Chaque widget de la galerie montre un aperçu

Écrit contre : `8aa2b0c` (branche `hig/plan-A`)

## Chaîne de preuve

- Surface : Réglages › Widgets, catalogue (`src/ui/plugins/widgets.rs`, `tab_widgets`).
- Problème : chaque rangée tient en nom, origine, description et interrupteur ; ni aperçu ni forme provisoire.
- Règle : « display realistic simulated data instead » ; « Create an effective placeholder appearance » (widgets › Previews and placeholders).
- Propriétaire : `tab_widgets` (rangée du catalogue) ; jetons `tokens::ts`, `ts_metric`, `design::SMALL_RADIUS`.
- Portée : la rangée du catalogue seule ; les widgets placés et leur vue (`view_widget`) ne changent pas.
- Relecture indépendante (corrigée) : le lanceur manquait ; la vignette suit l'échelle du texte.
- Incertitude : un plugin (Lua, livré ou de l'utilisateur) n'a pas de rendu connu sans l'exécuter : il prend la forme provisoire.

## Décision

À gauche de chaque rangée, une vignette de 112 × 56 px, au fond du ton de surface (encre à 6 %, rayon 10) : les widgets intégrés y montrent des données simulées réalistes, dans la police et les rôles de leur vue réelle — l'heure qu'il est (horloge), le jour et la date (calendrier), deux jauges CPU et RAM à 12 et 26 % (système), un titre et un artiste (média), trois pastilles (réglages rapides), quatre icônes (lanceur). Les autres (commande, Lua, plugins livrés ou de l'utilisateur) prennent une forme provisoire : deux traits estompés, comme un texte en attente. Rien n'est actionnable dans la vignette ; l'accent n'y paraît pas. La vignette et sa colonne de libellés grandissent avec le texte (`tokens::text_growth`, HIG TYP-08), bornées à une fois et demie (relecture du diff : à 200 %, une vignette de 224 px écrasait la description dans un tiroir étroit).

## Réutilisation

- `format_now` (heure, date) et `numeric_font` comme `view_widget` ; `iced::widget::progress_bar` comme le widget système.
- Exemplaire : `src/ui/widget_view.rs` (compositions horloge, calendrier, système).

## Changements

1. `src/ui/plugins/widgets.rs`
   - `App::catalog_preview(type_id) -> Element` : vignette décrite plus haut ; `row![preview, info, control]` dans la rangée.
   - Préserver : interrupteur, textes, erreurs, ordre.
2. `locales/en.toml` : libellés simulés (« Titre du morceau », « Artiste »).
3. Documentation : `docs/GUIDE.fr.md`, `docs/GUIDE.md` (galerie de widgets).

## Portée

- Hérite : toutes les rangées du catalogue.
- Vérifier : largeur du tiroir des Réglages (la rangée garde sa description lisible), thème clair et sombre, taille du texte à 200 %.
- Exclure : rendu réel des plugins.

## Validation

- Produit : la galerie montre à quoi ressemble chaque widget avant de l'activer.
- Système : pas de nouveau jeton ; compositions de la vue existante.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh` → 0 ; test : les six widgets intégrés activables d'un clic (horloge, calendrier, système, média, réglages rapides, lanceur) ont leur aperçu simulé ; commande, Lua et plugins ont la forme provisoire.

## Conditions d'arrêt

- S'arrêter si la vignette demande d'exécuter un plugin.

## Documentation

- Guides : la galerie montre un aperçu de chaque widget.
