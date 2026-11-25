# LAY-01 — Les sections avancées des Réglages se replient derrière un triangle de divulgation intitulé

Rédigé contre : `93a7b71`

## Chaîne de preuve

- Surface : tiroir Réglages, configuration avancée allumée (Bureau, Entrée, Fenêtres, Raccourcis, Panorama, Barre, Couleurs, Index…).
- Problème : aucune section de réglage ne se replie. Tout ce qui est avancé dépend du seul interrupteur « Configuration avancée », qui montre partout à la fois les onglets avancés, les sections encadrées en cyan et les groupes d'options brutes de Hyprland ; allumé, il allonge d'autant chaque page.
- Preuve de design : `design-plans/bureau-macos/02-layout.md`, première partie, « Disclosure controls » — « Use a disclosure control to hide details until they’re relevant » ; « Place controls that people are most likely to use at the top of the disclosure hierarchy…, with more advanced functionality hidden by default » ; « A disclosure triangle points inward from the leading edge when its content is hidden and down when it’s visible » ; « Make sure your labels indicate what is disclosed or hidden ».
- Propriétaire : `App::adv` et `adv_frame` (`src/ui/plugins/mod.rs`) ; 33 appels `self.adv(…)` et le cadre des `ADVANCED_PLACES` (`plugins::view`).
- Périmètre et surfaces touchées : `src/ui/plugins/mod.rs`, `src/ui/mod.rs`, `src/ui/panel.rs`, `src/ui/hyprset.rs`, les fichiers de paquets qui appellent `self.adv(…)`, `src/i18n.rs`, `locales/en.toml`, `docs/GUIDE.fr.md`, `docs/GUIDE.md`.
- Incertitude : les scènes d'aperçu ne montrent l'état déplié que si une scène le demande ; l'alignement du triangle est à regarder sur les captures.

## Décision de design

En configuration avancée, chaque section avancée devient un **triangle de divulgation intitulé, replié par défaut** : une rangée-bouton « ▸ Titre ». Un clic déploie son contenu dessous, en retrait de 20 px (sous le libellé, après le triangle), sans cadre ; le triangle pointe alors vers le bas (« ▾ »). Le cadre cyan permanent disparaît : la page ne s'allonge plus quand on allume la configuration avancée, et ne s'allonge que là où l'on a déplié.

- **Intitulé** : ce que la section révèle — le titre de sa section quand elle en a un, sinon le sujet de ses champs. Deux appels qui formaient un seul bloc (un champ et son aide) ne font qu'un triangle.
- **Section d'un seul champ** : le libellé du champ est l'intitulé du triangle ; seuls son contrôle et l'avertissement « surchargé dans hyprdesk.lua » se déploient dessous (`field("", clé, contrôle)`, que `field` rend sans ligne de libellé). Le libellé n'apparaît donc pas deux fois, et garde sa traduction anglaise.
- **Intitulés uniques par page** : la clé d'état est « paquet · onglet · titre », deux triangles d'une même page ne peuvent donc pas partager leur titre (Animations : « Réglages fins : <groupe> »).
- **Options de Hyprland** (`hyprset`) : un triangle par section du registre (`hyprset::GROUPS`), intitulé par son titre ; le contenu n'en répète pas le titre.
- **État** : en mémoire pour la session, `App::adv_open`, clé « paquet · onglet · titre » — comme `subtabs`, qui retient le dernier onglet d'un paquet. Message `PanelMsg::AdvToggle(clé)`.
- **Inchangé** : en configuration de base, la section ne s'affiche pas (le réglage existant garde son sens : D3 conserve l'interrupteur) ; onglets et paquets avancés, pastille « Avancé », fiole et compte de la navigation.
- **Recherche** : pendant une recherche sur la page, le bandeau promet des correspondances « surlignée(s) » : les sections de la page sont alors toutes dépliées, sinon des correspondances seraient invisibles.

## Réutilisation

- Idiome du triangle en texte « ▸ / ▾ » : `src/ui/chat.rs` (étapes techniques de l'assistante, repliées par défaut).
- `self.marked` (texte surligné par la recherche), `button::text`, `self.ink().fg`, `crate::ui::tokens::ts`, `hyprset::GROUPS`.
- Exemple : `App::delete_button` (`src/ui/plugins/mod.rs`) — bouton d'état porté par le paquet.

Pas de primitive parallèle : la divulgation vit dans `App::adv`, qui remplace `adv_frame` (supprimé).

## Changements

1. `src/ui/plugins/mod.rs`
   - Changement : `adv(&self, titre: &str, e)` ; replié par défaut, déplié si la clé est dans `adv_open` ou si une recherche est en cours ; suppression de `adv_frame` ; `view` déploie les groupes de `ADVANCED_PLACES` derrière leurs triangles.
   - Préserver : configuration de base → rien, sans espace dans la colonne ; comptage `self.adv(` de `ADV_SOURCES`.
   - Vérifier : test — chaque `self.adv(` des sources est suivi d'un intitulé littéral.
2. `src/ui/mod.rs` : champ `adv_open: HashSet<String>`.
3. `src/ui/panel.rs` : variante `PanelMsg::AdvToggle(String)` et son traitement ; `field` ne dessine pas de ligne de libellé quand il est vide (les autres appels sont inchangés).
4. `src/ui/hyprset.rs` : `hyprset_sections(clé)` donne (titre, champs) par section du registre ; le repli de `hyprset_tab` les replie derrière leurs triangles.
5. Appels `self.adv(…)` de `hyprlists.rs`, `animations.rs`, `assistant.rs`, `bar.rs`, `colors.rs`, `desktop.rs`, `index.rs`, `input.rs`, `keys.rs`, `layouts.rs`, `lock.rs`, `panorama.rs`, `panorama/design.rs`, `panorama/modules.rs`, `plugins.rs`, `screens.rs`, `workspaces.rs`, `windows.rs` : chacun reçoit son intitulé ; un titre de section répété dans le contenu est retiré.
6. `src/i18n.rs` : `".adv("` rejoint les appels dont le premier littéral doit avoir une traduction anglaise.
7. `locales/en.toml` : traductions des nouveaux intitulés.
8. Documentation : `docs/GUIDE.fr.md`, `docs/GUIDE.md` (sections avancées repliées, plus encadrées).

## Périmètre

- Héritent : toutes les pages de réglage en configuration avancée.
- À vérifier : recherche des Réglages (bandeau, surlignage), navigation (fiole et compte), scènes d'aperçu des Réglages.
- Exclu : onglets et paquets avancés (`ADVANCED_TABS`, `ADVANCED_ONLY`), pastille « Avancé », tout autre écart.

## Validation

- Produit : allumer « Configuration avancée » → la page ne gagne que des triangles repliés ; cliquer → le contenu se déploie sans cadre ; cliquer encore → il se replie.
- Interface : pages à plusieurs sections (Raccourcis, Entrée, Fenetres, Panorama) ; recherche d'un réglage avancé ; configuration de base.
- Système : aucun cadre `adv_frame` ne reste ; un seul composant de divulgation pour les réglages.
- Dépôt : `CARGO_BUILD_JOBS=6 bash scripts/verify.sh` → code de sortie 0 ; `scripts/verify.sh --visual /tmp/hig-visual-B` → captures des scènes de réglages regardées.

## Conditions d'arrêt

- S'arrêter si `self.adv(…)` servait hors des pages de réglage, ou si un appel ne peut recevoir d'intitulé qui dise ce qu'il révèle.

## Documentation de design

- Après acceptation : `docs/GUIDE.fr.md` et `docs/GUIDE.md` décrivent les sections avancées repliées.

## Relecture à froid (sans outil d'agent)

Relu après la lecture des 33 appels, avant d'écrire le code.

- **Qu'est-ce qui casse ?** Trois points, corrigés dans le plan. (1) Deux triangles de même titre sur une page basculeraient ensemble (Animations : un « Réglages fins » par groupe) : intitulés uniques par page. (2) Un triangle qui répète le libellé de son unique champ l'afficherait deux fois : le libellé devient l'intitulé, le contenu est le contrôle seul. (3) Une recherche promet des correspondances « surlignée(s) » : repliées, elles seraient invisibles — d'où le dépliage pendant la recherche. Rien d'autre : `adv_frame` n'a que deux appelants, tous deux dans `mod.rs` ; en configuration de base `adv` rend toujours un espace nul ; le comptage de `ADV_SOURCES` compte toujours `self.adv(`.
- **Que manque-t-il ?** Les traductions anglaises des intitulés nouveaux, et le moyen de ne pas en oublier : `".adv("` rejoint les appels surveillés par `catalogue_complet`. Un test qui impose un intitulé littéral à chaque appel. La documentation des guides, qui parle de sections « encadrées en cyan ». `panel.rs` et `plugins/mod.rs` sont des propriétaires partagés : relecture du diff obligatoire.
- **Le plan résout-il l'écart ?** Oui : chaque section `adv(…)` et chaque groupe d'options brutes devient un triangle intitulé, replié par défaut, qui déploie dans la page ; le cadre permanent disparaît ; la page ne s'allonge plus à l'allumage de la configuration avancée.

### Relecture du diff (écart M ; `panel.rs` et `plugins/mod.rs` sont des propriétaires partagés)

Relu à froid, sans outil d'agent, après le contrôle rapide (`fmt`, `clippy` à zéro avertissement), sur `git diff HEAD` avant le commit, puis sur les captures de `scripts/verify.sh --visual` (`animations-effets`, 1440 px : « ▸ Fine-tuning: Windows », « ▸ Fine-tuning: Blur », repliés, sans cadre).

- **Le diff fait-il exactement ce que l'écart demande ?** Oui : 28 appels `self.adv(…)` intitulés et repliés par défaut ; les groupes du registre repliés dans une page aussi (`hyprset_folded`, `ADVANCED_PLACES` compris) ; `adv_frame` supprimé. Deux appels qui formaient un seul bloc (Raccourcis, Index) ne font plus qu'un triangle.
- **Respecte-t-il `DESIGN.md` et les conventions ?** Une famille de police (triangle textuel, idiome de `chat.rs`), pas d'accent, espacements de 4, 8 et 24 px, aucun mouvement ; français source et douze traductions anglaises ; commentaires en français.
- **Régression, test manquant ?** Trois points, corrigés avant la publication. (1) Les titres de section répétés dans un contenu replié ont disparu du source : ils sont de nouveau cherchables, les intitulés des triangles sont indexés (motif `.adv(")`). (2) Ce motif, d'abord écrit `self.adv("`, citait l'appel et faussait `sections_avancees_comptees` — un seul test, corrigé et porte repassée. (3) Le compte de la navigation restait à garantir après le passage à `hyprset_folded` : `FOLDED_GROUPS`, testé par `groupes_replies_coherents`. Ajoutés : `sections_avancees_intitulees`, surveillance de `.adv(` dans `catalogue_complet`. Manque, assumé : un test de rendu de `adv`, qui demande une `App` complète. Le conflit de `locales/en.toml` au rebasage (deux ajouts en fin de fichier) a été résolu par union.
