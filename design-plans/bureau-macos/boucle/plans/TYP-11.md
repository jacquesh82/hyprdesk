# TYP-11 — Les libellés de navigation se passent du possessif

Écrit contre : `aff1a6d` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : Réglages → Thème (section « Mes thèmes », bouton « Enregistrer dans Mes thèmes », avis « Thème créé… dans Mes thèmes »), Réglages → Animations → Courbes (section « Vos courbes »), en-tête de la discussion avec LYRA (« Votre assistante du bureau »), assistant myMFA (titre « Créer votre coffre myMFA »), aide de la ligne de commande (`theme [list] « Mes thèmes »`), message d'erreur de `theme remove`.
- Problème : revérifié à HEAD, les libellés sont là où la preuve les place (`src/ui/plugins/theme.rs:452, 948, 1195`, `src/ui/curves.rs:211`, `src/ui/chat.rs:1083`, `src/ui/mymfa.rs:740`, `src/lib.rs:139`, `src/theme.rs:493`) ; les numéros de `locales/en.toml` ont glissé de quelques lignes.
- Preuve de design : HIG writing › Best practices — « Use possessive pronouns sparingly. » ; « “Favorites” conveys the same message as “Your Favorites,” and is more succinct. »
- Propriétaire : les libellés eux-mêmes, et leur traduction dans `locales/en.toml`.
- Périmètre : libellés de navigation, titres et en-têtes, plus les messages qui nomment ces libellés (pour rester cohérents) ; la documentation (`docs/PLUGINS.md`, `docs/PLUGINS.en.md`, `docs/ADVANCED.md`, `docs/THEME-ENGINE.md`) et les commentaires de code qui citent le libellé.
- Incertitude : « Un espace à vous » (`src/ui/preview.rs:367`) est le titre fictif d'une fenêtre dans les scènes d'aperçu, pas un libellé : hors correction. Les phrases explicatives (« Les vôtres sont écrites dans animations.lua ») gardent leur pronom : la HIG dit « sparingly », pas « jamais ».

## Décision de design

| Avant | Après | Anglais |
| --- | --- | --- |
| Mes thèmes | Thèmes personnels | Custom themes |
| Enregistrer dans Mes thèmes | Enregistrer dans les thèmes personnels | Save to Custom themes |
| Thème créé : {out} — à appliquer d'un clic dans Mes thèmes. | Thème créé : {out} — à appliquer d'un clic dans les thèmes personnels. | Theme created: {out} — apply it in one click from Custom themes. |
| seuls les thèmes de « Mes thèmes » se retirent | seuls les thèmes personnels se retirent | only custom themes can be removed |
| Vos courbes | Courbes personnelles | Custom curves |
| Votre assistante du bureau | Assistante du bureau | Desktop assistant |
| Créer votre coffre myMFA | Créer un coffre myMFA | Create a myMFA vault |
| … les plugins tiers, Mes thèmes, … (description de l'archive de configuration, `src/ui/plugins/share.rs`) | … les plugins tiers, les thèmes personnels, … | … third-party plugins, Custom themes, … |
| `theme [list]      « Mes thèmes »` | `theme [list]      « Thèmes personnels »` | `"Custom themes"` |

« Personnel » plutôt que « Mes » : l'adjectif garde la distinction avec les thèmes livrés sans s'adresser à l'utilisateur ; « Custom » est le mot de macOS pour ce que l'utilisateur a créé.

Relecture du plan par un agent indépendant (lecture seule) : aucun libellé ne sert de clé ; la recherche des Réglages indexe `self.section("…")` depuis le source et trouve « Thèmes personnels » sans rien faire ; le seul `tr!` à paramètre garde `{out}` ; il manquait le texte affiché de `src/ui/plugins/share.rs` (« les plugins tiers, Mes thèmes »), ajouté, et l'avis dit « dans les thèmes personnels », comme le bouton. `CHANGELOG.md` est un historique : non touché, exclu du contrôle.

## Réutilisation

- Le catalogue `locales/en.toml` ; les tests `i18n::tests::catalogue_complet` et `aide_traduite` vérifient que chaque texte marqué et chaque ligne d'aide ont leur traduction.

Pas de primitive nouvelle.

## Changements

1. `src/ui/plugins/theme.rs`, `src/ui/curves.rs`, `src/ui/chat.rs`, `src/ui/mymfa.rs`, `src/lib.rs`, `src/theme.rs` : remplacer les libellés selon le tableau ; commentaires qui les citent mis à jour (`src/theme.rs`, `src/share.rs`, `src/main.rs`, `src/ui/plugins/theme.rs`, `src/ui/plugins/share.rs`).
   - Préserver : le dossier `hyprdesk/themes/`, les commandes `theme list`, le comportement.
2. `locales/en.toml` : retirer les anciennes clés, ajouter les nouvelles, avec la traduction du tableau.
3. `docs/PLUGINS.md`, `docs/PLUGINS.en.md`, `docs/ADVANCED.md`, `docs/THEME-ENGINE.md` : même renommage.
   - Vérifier : `grep -rn 'Mes thèmes\|My themes\|Vos courbes\|Your curves\|Votre assistante\|Créer votre coffre'` ne trouve plus rien hors de `design-plans` et de `CHANGELOG.md`.

## Périmètre

- Hérite : les surfaces citées.
- Vérifier : la recherche des Réglages (elle indexe les titres de section) trouve « Thèmes personnels ».
- Exclure : « Un espace à vous » (contenu d'aperçu) ; les phrases explicatives ; les textes de l'assistant de premier démarrage et de LYRA hors en-tête.

## Validation

- Produit : Réglages → Thème montre « Thèmes personnels » ; Animations montre « Courbes personnelles ».
- Interface : scènes `theme`, `chat`, `animations` de `scripts/verify.sh --visual`.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh` → code de sortie 0.

## Conditions d'arrêt

- S'arrêter si un libellé sert de clé de configuration ou de sélecteur (Lua, ligne de commande) : il ne se renomme pas.

## Documentation de design

- Aucune décision nouvelle pour `DESIGN.md` ; la documentation utilisateur suit les libellés.
