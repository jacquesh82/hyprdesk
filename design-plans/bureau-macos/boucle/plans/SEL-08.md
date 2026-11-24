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

1. **Propriétaire et pilote.** — faite (`96dac5a`, itération B17). `SliderEdit`, les messages, `slider_value` et ses tests, `App::slider`, Échap et fermetures ; appliqué au seul champ « Taille des icônes » du dock (`plugins/dock.rs`, 24 à 96 px). Scène `reglages-valeur-saisie`, guides, `DESIGN.md`. Vérifiable : le champ s'ouvre, Entrée pose et enregistre, Échap referme, la scène existe. Écart au plan : la valeur est posée dans le traitement du message (`update_inner`), comme `App::commit` applique avant d'enregistrer, pour que le test lise le réglage posé ; la marge du champ ouvert est de `[1, 8]`, non celle de la recherche (`[6, 10]`), pour que la rangée grandisse de 4 px et non de 16.
2. **Curseurs entiers à unité** : `plugins/bar.rs` (6), `icons.rs` (2), `effects.rs` (2), `lock.rs` (délai de mise en veille), `wallpaper.rs`, `theme.rs` (opacité, échelle du texte, verre, assombrissement du verrouillage), `animations.rs`, `launcher.rs` (helper de forme, cinq champs), `screens.rs` (3), `panorama.rs` (survol, veille). Vérifiable : chaque domaine, pas et unité inchangés ; scènes des paquets inchangées au repos. À savoir depuis B17 : une vingtaine de sites (`theme.rs`, `input.rs`, `hyprset.rs`, `curves.rs`…) alignent curseur et valeur au centre (`align_y(Alignment::Center)`), quand le site du dock les aligne en haut ; donner l'alignement au composant plutôt que déplacer les sites.
   - **Faits (B19, treize curseurs)** : le composant prend la présentation de son site, `SliderLook` — `fixed` : curseur largeur fixe, valeur à sa largeur ; `fill` : curseur souple, valeur largeur fixe — et `SliderSpec::new`. Convertis : `bar.rs` (hauteur, marge), `icons.rs` (2), `effects.rs` (2), `wallpaper.rs`, `theme.rs` (échelle du texte, verre, verrouillage), `launcher.rs` (suggestions), `panorama.rs` (survol, veille). Le pilote du dock suit la nouvelle signature, aligné en haut (`Alignment::Start`).
   - **Restent (douze curseurs)** : `bar.rs` (marges des côtés, rayon des coins ; l'helper « Convexe », trois champs), `theme.rs` (opacité par endroit), `launcher.rs` (helper de forme), `lock.rs` (quatre étapes de veille), `screens.rs` (barre et texte de l'écran, taille de zone), `animations.rs` (vitesse). Leur `poser` a besoin d'une donnée du rendu — une forme, un endroit, une étape, un écran, un nom —, qu'un pointeur de fonction ne garde pas : voir « Révision du plan » ci-dessous, à exécuter avant la fin de cette étape.
   - **Faits (B20, étape 2 achevée)** : la révision exécutée, avec trois écarts (voir « Révision du plan »). Convertis : `bar.rs` (marges des côtés, rayon des coins ; « Convexe » déroulé en trois curseurs fixes à 260 px), `theme.rs` (opacité par endroit, plus les deux options numériques de l'étape 3), `launcher.rs` (helper de forme, cinq champs), `lock.rs` (quatre étapes de veille), `screens.rs` (barre et texte de l'écran, taille de zone), `animations.rs` (vitesse). **Étape 2 faite.**
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

## Relecture du diff de l'étape 1 (itération B17, sans outil d'agent)

Travail interrompu repris par `reprise-B.patch`, relu à froid après vérification dans les sources d'iced 0.14.2 que fixe `Cargo.lock`.

- **Qu'est-ce qui casse ?** Trois défauts, corrigés. Le travail repris ne compilait pas : `Task::chain` prend une tâche, plus une fermeture depuis iced 0.14. Le bouton de la valeur portait la marge par défaut d'un bouton (5 et 10 px) : au repos la valeur glissait de 10 px et sa rangée grandissait de 10 px, contre « au repos, rien ne change » — il n'a plus de marge. Le champ ouvert, haut de 33 px, poussait de 16 px ce qui le suit — sa marge passe à `[1, 8]`, la rangée grandit de 4 px.
- **Que manque-t-il ?** Un test de l'effet d'Entrée : la valeur est désormais posée dans le traitement du message, et le test lit le réglage posé (72, l'unité tapée ignorée ; 500 borné à 96). Vérifié au passage : un champ qui a le focus capture Échap pour le perdre, mais `event::listen_with` livre aussi les événements capturés — le gardien de `Msg::Key` referme le champ dès le premier appui, sans fermer le tiroir.
- **Le diff fait-il exactement ce que l'écart demande ?** Oui pour l'étape 1, rien de plus : un composant, un site pilote. Les quarante-quatre autres curseurs attendent les étapes 2 et 3.
- **Respect de `DESIGN.md` et des conventions ?** Oui : des widgets du catalogue seulement (curseur, bouton, champ, infobulle), ni jeton ni style nouveaux ; français source, anglais dans `locales/en.toml`, entrée rangée auprès de celles du tiroir.
- **Régression, test manquant ?** Aucune trouvée : au repos la valeur garde sa place, le glissé pose toujours la valeur pendant le mouvement et l'enregistre au relâchement, Échap referme le tiroir hors champ ouvert. Reste non vérifié à l'exécution (pas de démon lancé) : le focus et la sélection du nombre à l'ouverture du champ, qui passent par des tâches que l'aperçu n'exécute pas.

## Relecture du diff de l'étape 2, première partie (itération B19, sans outil d'agent)

Relu à froid après la comparaison des scènes d'aperçu : rendues avant le changement depuis le binaire de tête, puis deux fois après, aux trois largeurs. Deux rendus d'un même binaire diffèrent déjà par ce que l'heure change (horloges de la barre, du widget en haut à gauche et du Panorama, badge de notifications, télémétrie) et par l'état d'une recherche asynchrone (« Recherche du plugin… » de la page Bureau) : retirées, il ne reste aucune cellule changée.

- **Qu'est-ce qui casse ?** Trois défauts, corrigés avant la porte. `App::slider` à huit arguments, refusé par `clippy::too_many_arguments` : la présentation est passée dans `SliderLook`. `Alignment::Top` n'existe pas (iced : `Start`, `Center`, `End`). Un curseur souple pris dans une rangée resserrée aurait rempli tout le tiroir et rejeté le troisième élément (bascule « Flou » du verrouillage, libellé de la taille du texte) : la rangée du composant remplit comme son curseur.
- **Que manque-t-il ?** Un test d'un site à pas : `la_valeur_saisie_dun_curseur_a_pas_est_ramenee_au_pas` (`254 ms` → 250, `5000` → 1000, sur la durée du fondu). Un réglage forgé hors domaine s'affiche désormais borné, comme la poignée le montre (avant : « 100 px » à côté d'une poignée calée à 64). À recenser avant de clore l'écart : la page Dock montre deux autres curseurs (« Icon spacing », « Hover magnification ») qu'un comptage de `slider(` ne voit pas.
- **Le diff fait-il exactement ce que l'étape demande ?** Treize curseurs entiers : domaine, pas, unité, messages de pose et d'enregistrement inchangés ; ni plus. Douze restent.
- **Respect de `DESIGN.md` et des conventions ?** Oui : widgets du catalogue seulement, ni jeton ni style nouveaux ; la valeur ne revient pas à la ligne, ce que les sites de `theme.rs` demandaient déjà ; rien à traduire.
- **Régression ?** Aucune trouvée : les pages converties sont identiques à celles d'avant, au pixel près (hauteur et marge de la barre, icônes, effets, fond d'écran, thème, lanceur, Panorama ; 1440, 900 et 640 px).

## Révision du plan (itération B19, sans outil d'agent)

À exécuter pour finir l'étape 2, puis pour l'étape 3. À relire à froid au début de l'itération qui l'exécute.

**Constat.** Treize curseurs de l'étape 2 ont un `poser` qui ne dépend que de la valeur. Les douze autres, et une bonne part de l'étape 3 (`hyprset.rs`, `colors.rs`, `curves.rs`, `panorama/design.rs`, `hue.rs`, `sound.rs`), posent la valeur par un message qui emporte une donnée du rendu : une forme entière (`Msg::Shape(s, commit)`, `Msg::AppsShape(s, commit)`), une zone (`Msg::PanoramaZone(Some(Zone { .. }))`), un endroit (`Msg::Opacity(place, v)`), une étape de veille, la clé d'un écran ou d'une option, le nom d'une animation. Un pointeur de fonction ne garde rien : c'est la condition d'arrêt du plan (« un site [qui] ne peut pas rendre sa valeur par une fonction sans capture »).

**Décision.** `poser` garde son rôle et prend une forme plus générale : `SliderSpec::new(clé, domaine, pas, poser, enregistrer)` accepte `poser : impl Fn(f64) -> Msg + 'static`, rangé dans une `Rc<dyn Fn(f64) -> Msg>`.

- Les treize sites convertis ne changent pas : leur fermeture sans capture passe telle quelle. Le pilote du dock (littéral de structure) passe par `new`, qui n'est plus `const`.
- `SliderSpec` garde `Clone` ; son `Debug` s'écrit à la main (clé, domaine, pas, enregistrement ; la fermeture ne se montre pas), `PanelMsg` et `Msg` dérivant `Debug`.
- Aucun message nouveau, aucune donnée dupliquée dans le tiroir : la fermeture porte ce que le glissé du site portait déjà.
- Coût : une allocation par curseur rendu (une quarantaine par image du tiroir, hors de mesure). C'est un écart au plan d'origine (« sans fermeture allouée »), assumé : la forme sans allocation exigerait un message par famille pour les formes et les clés dynamiques.
- Helper « Convexe » de `bar.rs` : déroulé en trois `self.slider`, curseur `fixed` à 260 px comme ses voisins. Le plafond souple qu'il avait (`Fill`, 260 au plus) ne jouait que sous 330 px de contenu, que le tiroir (936 px) ne présente pas.

**Garde contre une forme périmée.** Une fermeture qui emporte une forme la repose entière à Entrée. Si la forme change entre l'ouverture du champ et Entrée — un autre réglage bougé à la souris, une remise par défaut —, la valeur tapée écraserait ce changement ; or le champ reste ouvert quand il perd le focus (B17). Donc : tout message d'un paquet au tiroir (`PanelMsg::Plugin`) referme le champ avant `plugins::update` — un autre geste ferme la saisie, comme un changement de paquet. À vérifier d'abord : qu'aucun message de paquet n'arrive au tiroir sans geste de l'utilisateur (rafraîchissement périodique, événement du démon routé par `PanelMsg::Plugin`) ; sinon la saisie se fermerait toute seule, et il faut alors se limiter à refermer à `PanelMsg::Defaults` et dire le piège au propriétaire.

**Vérification demandée ci-dessus (B20).** Aucun abonnement ne rend de message de paquet (`subscription` de `src/ui/mod.rs` : `Control`, `Hypr`, volume, thème… ; `plugins::update` n'est appelé que par `PanelMsg::Plugin`). Deux exceptions, aux conséquences bornées : une tâche lancée par un geste rend un message de paquet à son achèvement (`Task::done(wrap(Msg::Status))` de `colors.rs`, et semblables) — une saisie ouverte pendant une de ces tâches se referme quand elle finit ; la carte des écrans émet `Msg::DragEnd` en sortant de son survol (`screens.rs`). Dans les deux cas la saisie est perdue, aucun réglage ne l'est. La garde reste : sans elle, Entrée sur un curseur de forme reprendrait un champ bougé depuis.

**Écarts à cette révision, constatés en l'exécutant (B20).**

- `Msg` doit être `Send` : un `Rc` ne compile pas. La fermeture est un `Arc<dyn Fn(f64) -> Msg + Send + Sync>`.
- La clé est un `Arc<str>`, non un `&'static str` : les options de design et les surcharges d'écran ont un nom composé au rendu (`screen.DP-1.text_scale`). Deux curseurs de noms différents ne sont pas le même champ ouvert (test).
- Le piège que la révision ne voyait pas : le message de relâchement de plusieurs sites portait la valeur d'avant le glissé (`Msg::DesignOpt(c, v, true)`, `Msg::AppsShape(shape, true)`). Copié à l'ouverture du champ, il aurait écrasé la valeur tapée à Entrée. `enregistrer` prend donc un message sans valeur : `Msg::Commit` du paquet après une pose qui pose, `PanelMsg::Commit` pour la forme du lanceur, `UiMsg::Noop` quand la pose enregistre déjà (zone Panorama). Le relâchement s'en trouve égal : la valeur est posée pendant le glissé, l'enregistrement la suit.
- `SliderLook` prend l'écart du site (`.spacing(10)`) : la rangée de la taille de zone écartait le curseur de sa valeur de 10 px.
- « Voile derrière » du lanceur : en pour cent, comme sa valeur affichée (0 à 80, pas de 1). Auparavant son domaine était 0 à 0,8 sans pas déclaré : or depuis iced 0.14 le pas par défaut d'un curseur est 1, il sautait de 0 à 80 %.

**Relecture à froid**, écrite après la comparaison des scènes.

- **Qu'est-ce qui casse ?** `SliderEdit` n'est plus `Debug` par dérivation : d'où le `Debug` à la main. La garde referme le champ au premier glissé du curseur lui-même : cohérent, la saisie n'a alors plus d'objet. Les messages de pose et d'enregistrement rendus par Entrée reviennent au tiroir par `PanelMsg::Plugin` une fois le champ refermé : la garde ne les arrête pas.
- **Que manque-t-il ?** La vérification sur le routage des messages de paquet, dite plus haut ; un test qu'un message de paquet referme le champ ; un test de la pose par fermeture sur un site à forme (`Msg::Shape`), lisant le réglage posé.
- **Le plan résout-il l'écart ?** Oui : tous les curseurs restants passent au composant avec leurs messages existants ; plus aucune valeur du tiroir ne reste en lecture seule.

## Relecture du diff de l'étape 2, seconde partie (itération B20, sans outil d'agent)

Relu à froid après la porte visuelle. Scènes rendues avant le changement (binaire de
tête, copié hors du worktree) et après, aux trois largeurs : les 174 scènes, puis neuf
rendus ciblés par onglet (`HYPRDESK_PREVIEW_SUBTAB` : forme du lanceur, Barre, Écran,
Transparence, Animations, Veille, Bureau, Clavier, Son), où se trouvent les curseurs
convertis.

- **Qu'est-ce qui casse ?** Trois défauts, corrigés avant la publication. Le `Debug` à la
  main employait `debug_struct(..).field("key", ..)` : le relevé du catalogue
  (`i18n::marked`, qui lit `.field("…")` comme un libellé) réclamait la traduction de
  « key », « range », « step », « save » — la porte a échoué là-dessus une fois ; le
  `Debug` s'écrit avec `write!`. `Row::spacing` d'iced 0.14 prend des `Pixels`, non un
  entier. La rangée de la taille de zone écartait le curseur de sa valeur de 10 px, non
  12 : `SliderLook::spacing`. Une seule différence hors horloges et télémétrie de
  l'en-tête : sur la page Barre en style Convexe, la barre de défilement est 16 px plus à
  gauche qu'avant — à la place qu'elle a sur la même page en style Continuité et sur les
  autres pages ; le contenu visible est identique au pixel près.
- **Que manque-t-il ?** Les deux tests que la révision demandait : un message de paquet
  referme le champ ; la pose par fermeture sur un site à forme (`Msg::Shape`) lit le
  réglage posé, le reste de la forme et la marge posée avant étant gardés. S'y ajoute un
  test de deux clés composées distinctes. Non vérifié à l'exécution, comme depuis B17 :
  le focus et la sélection du nombre à l'ouverture du champ.
- **Le diff fait-il exactement ce que l'étape demande ?** Oui : domaine, pas, unité et
  messages de pose de chaque site inchangés ; l'enregistrement prend un message sans
  valeur là où celui du relâchement en portait une. Deux dépassements, notés au registre :
  « Voile derrière » en pour cent (0 à 80, pas de 1) au lieu de 0 à 0,8 sans pas déclaré ;
  et les trois curseurs « Convexe » fixes à 260 px, comme la révision le décidait.
- **Respect de `DESIGN.md` et des conventions du dépôt ?** Oui : des widgets du catalogue
  seulement (curseur, bouton, champ, infobulle), ni jeton ni style nouveaux ; aucune
  chaîne nouvelle à traduire ; une allocation par curseur rendu (fermeture, clé composée),
  assumée par la révision.
- **Régression, test manquant ?** Aucune trouvée sur les pages rendues : Dock, Barre,
  Lanceur, Écrans, Thème, Animations, Verrouillage, Bureau, Clavier, Son identiques au
  pixel près dans le tiroir. Reste à dire au propriétaire : un geste lancé avant la saisie
  qui rend un message de paquet à son achèvement referme le champ (la saisie est perdue,
  pas le réglage).
