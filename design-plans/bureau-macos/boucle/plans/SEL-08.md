# SEL-08 — La valeur d'un curseur des Réglages se saisit au clavier

Écrit contre : `ca90d6d` (intégration `hig/plan-d-action`).

## Chaîne de preuve

- Surface : tiroir des Réglages, chaque champ à curseur (paquets intégrés, configuration avancée de Hyprland).
- Problème : la valeur d'un curseur est un texte qu'on ne peut pas modifier — `text(format!("{} px", l.size))`, `text(format!("{:+.2}", cur.sensitivity))`… Entrer une valeur exacte impose de traîner le curseur jusqu'à elle.
- Preuve de design : HIG sliders › Best practices — « Consider supplementing a slider with a corresponding text field and stepper. »
- Recensement à HEAD : la preuve du plan, relevée à `00f6d69`, citait quatre endroits. Il y en a quarante-cinq, assemblés à la main sur le même modèle `row![slider(…), text(format!(…))]`, dans dix-neuf fichiers, sans propriétaire commun : `plugins/bar.rs` (6), `theme.rs` (6), `input.rs` (4), `panorama.rs` (4), `widgets.rs` (3), `screens.rs` (3), `icons.rs`, `effects.rs`, `lock.rs`, `desktop.rs` (2 chacun), `animations.rs`, `dock.rs`, `colors.rs`, `sound.rs`, `wallpaper.rs`, `launcher.rs`, `plugins/panorama/surface/modules/hue.rs` (2), `hyprset.rs` (2), `curves.rs`. L'effort inscrit (M) sous-estime l'écart : il est L.
- Propriétaires : `App::field` (`src/ui/panel.rs`), sous lequel chaque curseur est posé ; `Panel` et `PanelMsg` (`src/ui/panel.rs`), l'état du tiroir ; le clavier du tiroir (`src/ui/mod.rs`, gestion des touches quand `self.panel.is_some()`).
- Périmètre et surfaces touchées : `src/ui/panel.rs`, `src/ui/mod.rs`, les dix-neuf fichiers de curseurs, `src/ui/preview.rs`, `scripts/verify.sh`, `locales/en.toml`, `docs/GUIDE.fr.md`, `docs/GUIDE.md`, `DESIGN.md`.
- Incertitude : la HIG parle aussi d'un stepper. La colonne Correction du registre ne retient que la valeur saisissable, validée à Entrée : le curseur, qui avance par son pas, tient lieu d'incrément — pas de stepper. Les curseurs des menus de la barre (volume, luminosité : `plugins/bar/popover.rs`, `controls.rs`, `plugins/bar/popover/continuity.rs`) restent hors de la correction (« des curseurs des Réglages ») : rangées de 32 px dans des menus qu'un changement de focus referme.

## Décision de design

- Au repos, rien ne change : le curseur, puis sa valeur en texte plein, dans le format d'aujourd'hui.
- La valeur est un bouton. Un clic la change en champ : le nombre seul, sélectionné, aligné à droite sur une largeur fixe ; le champ prend le focus. Son infobulle dit « Saisir une valeur ».
- Entrée valide : le nombre est lu — point ou virgule décimaux, l'unité tapée est ignorée —, borné au domaine du curseur et ramené à son pas ; le curseur saute à la valeur, la configuration s'enregistre comme après un relâchement. Entrée sur un texte illisible : rien, le champ garde la saisie.
- Échap referme le champ sans rien changer. Changer de paquet, ou fermer le tiroir, le referme aussi.
- Un seul champ ouvert à la fois.

Pourquoi cela résout la racine : une valeur exacte s'écrit au clavier depuis chaque curseur du tiroir, par un seul composant, sans second mécanisme de saisie.

## Réutilisation

- `App::field` : le nouveau `App::slider` se pose comme son contrôle, même espacement (12), même taille de texte que le site (13 ou 14).
- Les messages de chaque paquet : poser la valeur pendant le glisser (`Msg::Size(v)`), enregistrer au relâcher (`Msg::SizeCommit`). Le composant prend `poser : fn(f64) -> UiMsg` et `enregistrer : UiMsg` : des pointeurs de fonction et un message clonable, gardables dans `Panel` sans fermeture allouée.
- La validation à Entrée d'une valeur de réglage : `hyprset.rs` (`Msg::Submit`).
- Le focus d'un champ du tiroir : `SEARCH_ID`, `iced::widget::operation::focus` (`panel.rs`).
- Style du champ : celui de la recherche du tiroir. Infobulle : le widget d'infobulle tel que le pose le bouton à glyphe seul (`plugins/mod.rs`, HIG LAY-12).

Pas de primitive nouvelle : `App::slider` est un composé de widgets du catalogue (slider, bouton, champ de texte) rangé auprès de `App::field`. Il n'ajoute ni jeton ni style.

## Changements

1. `src/ui/panel.rs`
   - Changement : `Panel::slider: Option<SliderEdit>` — clé du curseur, saisie, domaine, pas, `poser`, `enregistrer` ; `PanelMsg::SliderEdit(…)`, `SliderDraft(String)`, `SliderSubmit`, `SliderClose` ; `fn slider_value(saisie: &str, domaine: (f64, f64), pas: f64) -> Option<f64>` ; `pub(in crate::ui) fn slider(&self, clé: &'static str, domaine, pas, valeur: f64, affiche: fn(f64) -> String, taille: f32, poser, enregistrer)`.
   - Préserver : le rendu au repos (format, tailles, largeur du curseur) et le glisser (poser pendant, enregistrer au relâcher). Les sites entiers convertissent explicitement (`v as u16`) dans `poser`, comme aujourd'hui : le composant parle en `f64`.
   - Vérifier : tests unitaires de `slider_value` ; scènes existantes des paquets inchangées.
2. `src/ui/panel.rs`, `src/ui/mod.rs`
   - Changement : `panel_show` et la fermeture du tiroir referment le champ ; Échap, dans le gestionnaire clavier du tiroir, le referme avant toute autre action.
   - Préserver : Échap sur le tiroir fait ailleurs ce qu'il fait aujourd'hui.
   - Vérifier : scène du champ ouvert.
3. Les quarante-cinq endroits, en trois étapes (voir « Étapes ») (voir « Étapes ») : `row![slider(…), text(…)]` devient `self.slider(…)`, chacun gardant son `affiche`.
4. `src/ui/preview.rs`, `scripts/verify.sh`
   - Changement : scène `reglages-valeur-saisie` (paquet Bureau, champ « Taille des icônes » ouvert), ajoutée à `SCENES` — sa taille déclarée augmente d'autant — et à la liste de la porte visuelle.
   - Préserver : les scènes existantes et leur ordre.
   - Vérifier : la capture existe aux trois largeurs.
5. `locales/en.toml`, `docs/GUIDE.fr.md`, `docs/GUIDE.md`, `DESIGN.md`
   - Changement : traduction de l'infobulle ; dans les guides, la phrase « Trouver un réglage » dit que la valeur d'un curseur se saisit ; une ligne dans `DESIGN.md`.
   - Vérifier : `catalogue_complet` passe.

## Étapes

1. **Propriétaire et pilote.** `SliderEdit`, les messages, `slider_value` et ses tests, `App::slider`, Échap et fermetures ; appliqué au seul champ « Taille des icônes » du dock (`plugins/dock.rs`, 24 à 96 px). Scène `reglages-valeur-saisie`, guides, `DESIGN.md`. Vérifiable : le champ s'ouvre, Entrée pose et enregistre, Échap referme, la scène existe.
2. **Curseurs entiers à unité** (vingt-et-un endroits) : `plugins/bar.rs` (6), `icons.rs` (2), `effects.rs` (2), `lock.rs` (délai de mise en veille), `wallpaper.rs`, `theme.rs` (opacité, échelle du texte, verre, assombrissement du verrouillage), `animations.rs`, `launcher.rs`, `screens.rs` (zoom), `panorama.rs` (survol, veille). Vérifiable : chaque domaine, pas et unité inchangés ; scènes des paquets inchangées au repos.
3. **Curseurs décimaux, signés ou de domaine propre** (vingt-trois endroits) : `input.rs` (4), `lock.rs` (assombrissement), `desktop.rs` (2), `sound.rs`, `colors.rs`, `theme.rs` (2 numériques), `curves.rs`, `widgets.rs` (3), `screens.rs` (2), `panorama.rs` (2), `hue.rs` (2), `hyprset.rs` (2). Vérifiable : `+0,50`, `1,5` et `1500` se lisent ; hors domaine, la valeur est bornée. L'écart passe alors `fait`.

## Périmètre

- Héritent : tous les curseurs du tiroir.
- À vérifier : les scènes des paquets touchés (dock, barre, thème, bureau, écrans…) à 1440, 900 et 640 px ; le clavier du tiroir (Échap, Entrée dans le champ de recherche).
- Exclus : les curseurs des menus de la barre ; un stepper ; l'assistant de premier démarrage (`wizard.rs`) et le sélecteur de couleurs (`color_picker.rs`), qui ne sont pas des réglages du tiroir.

## Validation

- Produit : Réglages → Bureau → « Taille des icônes », clic sur « 48 px », taper `72`, Entrée → le curseur saute à 72, la valeur du dock change, la configuration est enregistrée.
- Interface : valeur hors domaine (1, puis 500) → bornée à 24 et 96 ; `72 px` → 72 ; texte illisible → rien ; Échap → valeur d'avant ; deux curseurs à la suite → un seul champ ouvert ; changement de paquet champ ouvert → refermé.
- Système : un seul composant (`App::slider`), plus aucun `text(format!(…))` de valeur à côté d'un `slider(` dans le tiroir ; les messages `poser` et `enregistrer` existants, aucun message de valeur par curseur.
- Dépôt : `CARGO_BUILD_JOBS=6 bash scripts/verify.sh` → code 0 ; `CARGO_BUILD_JOBS=6 bash scripts/verify.sh --visual /tmp/hig-visual-B` → code 0, scène `reglages-valeur-saisie` regardée aux trois largeurs.

## Conditions d'arrêt

- Arrêter si `poser` puis `enregistrer` dans le même message ne donne pas le même état qu'un relâchement du curseur ; si Échap ne peut pas être pris avant l'action qu'il a aujourd'hui dans le tiroir ; si un site ne peut pas rendre sa valeur par une fonction sans capture.

## Documentation de design

- Après acceptation : `docs/GUIDE.fr.md` et `docs/GUIDE.md` (« Trouver un réglage ») ; `DESIGN.md` : « La valeur d'un curseur des Réglages se saisit : clic sur la valeur, Entrée valide. »

## Relecture à froid (sans outil d'agent)

Relu après avoir revérifié deux appuis dans le code : Échap, quand le tiroir est ouvert, le referme (`src/ui/mod.rs`, `if self.panel.is_some() { return self.close_panel(); }`) ; l'infobulle du bouton à glyphe seul est posée dans `plugins/mod.rs`, sans fonction à part.

- **Qu'est-ce qui casse ?** Échap dans le champ fermerait le tiroir et perdrait la saisie : le changement 2 pose sa garde avant. Un curseur entier rend son `f32` (71,999…) : `poser` garde la conversion du site et `slider_value` arrondit au pas, sinon « 71 px » s'afficherait 72. Tab s'arrête désormais sur la valeur de chaque curseur : c'est un contrôle, comme le curseur lui-même.
- **Que manque-t-il ?** La perte de focus (clic ailleurs) ne referme pas le champ : `iced` 0.13 ne la signale pas, et un widget à soi serait une primitive de trop — Échap, Entrée validé, changement de paquet et fermeture suffisent. L'index des réglages ne bouge pas : les libellés sont dans `App::field`, pas dans la valeur. Les comptes par étape sont indicatifs : le helper de `launcher.rs` sert plusieurs champs, à reverifier à l'étape 2.
- **Le plan résout-il l'écart ?** Oui, pour la correction du registre : la valeur de chaque curseur des Réglages se saisit, validée à Entrée, par un seul composant. Restent hors correction, à dire au propriétaire : les curseurs des menus de la barre.
