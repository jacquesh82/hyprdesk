# Les commandes du centre de contrôle ont une taille compacte

Écrit contre : `9619c6b` (branche `hig/plan-A`)

## Chaîne de preuve

- Surface : les tuiles Wi-Fi et Bluetooth des réglages rapides (`src/ui/controls.rs`, `control_tile`, widget et carte `controls`) et les pilules Wi-Fi et Bluetooth du centre de contrôle Continuité (`src/ui/plugins/bar/popover/continuity.rs`, `dashboard_connection`).
- Problème : une seule taille, toujours avec titre et valeur. Seules options : largeur et colonnes (`src/design_options.rs`).
- Règle : « In Control Center, a control displays its symbol and, at larger sizes, its title and value. » (controls › Anatomy).
- Propriétaire : `design_options` (option de composant), `control_tile`, `dashboard_connection` / `dashboard_pill_mix`.
- Relecture indépendante (corrigée) : la pastille compacte bascule, le menu passe au clic droit ; la pilule compacte n'a plus d'espacement de libellé (39 px carrés), l'infobulle porte toujours le titre, et l'erreur exacte en compact ; le menu d'une tuile compacte à bascule n'est atteint qu'à la souris (la tuile n'avait pas de navigation au clavier) ; dans Continuité, le bouton « › » des détails reste, c'est un symbole.
- Incertitude : aucune.

## Décision

Une option de composant `controls.compact` (interrupteur, Thème › Composants › Réglages rapides, `hyprdesk config set controls.compact on`), éteinte par défaut. Allumée, chaque commande ne montre que son symbole :
- tuile des réglages rapides : la pastille seule (32 px), qui garde son action (bascule, sinon menu) ; le titre et la valeur passent en infobulle ; le bouton « › » des détails disparaît, le menu passe au clic droit sur la pastille ;
- pilule Continuité : l'icône seule, la pilule devient carrée (39 px) ; l'infobulle existante porte déjà le détail, le titre y est ajouté.
La taille actuelle reste la taille par défaut.

## Réutilisation

- `design_options::opt` / `flag` (comme `clock.date`, `dock.autohide`) : rendu automatique dans Thème › Composants, ligne de commande, Lua.
- `iced::widget::tooltip` comme la pilule Continuité ; styles et fondus existants (SYS-06, SYS-07).

## Changements

1. `src/design_options.rs` : `opt("controls", "compact", Kind::Bool, Def::Bool(false), "Commandes compactes", "on | off — commandes Wi-Fi et Bluetooth réduites à leur symbole, titre et état en infobulle")`.
2. `src/ui/controls.rs` : `control_tile` compact : la pastille dans un bouton, infobulle « titre · valeur ».
3. `src/ui/plugins/bar/popover/continuity.rs` : `dashboard_pill_mix` sans libellé quand il est vide ; `dashboard_connection` compact passe un libellé vide, largeur 39, et le titre dans l'infobulle.
4. `locales/en.toml`, guides (`docs/GUIDE.fr.md`, `docs/GUIDE.md`).

## Portée

- Hérite : widget `controls` du bureau, carte des réglages rapides de la barre, centre de contrôle Continuité.
- Vérifier : colonnes (1 à 4), largeur réglée, navigation au clavier du centre de contrôle (les pilules restent des boutons).
- Exclure : curseurs (luminosité, son), cartes média et notifications.

## Validation

- Produit : l'option allumée, Wi-Fi et Bluetooth tiennent en deux symboles, la bascule marche, le survol dit le titre et l'état.
- Dépôt : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh` → 0 ; test : l'option existe, éteinte par défaut, se pose.

## Conditions d'arrêt

- S'arrêter si la pilule compacte casse la navigation au clavier du centre de contrôle.

## Documentation

- Guides : la taille compacte des commandes.
