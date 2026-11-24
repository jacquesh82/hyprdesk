# Boucle HIG — consigne d'une itération

> Version 3 : quatre couloirs en parallèle. À lire en entier avant d'agir, puis à appliquer à la lettre. Une itération = un tour de boucle d'un couloir.
> Le propriétaire n'est pas là pour répondre : trancher avec le plan, ou marquer « bloqué ». Ne jamais poser de question.

## 1. Ton couloir, ton worktree — et où l'on ne touche pas

- **Ton couloir** (A, B, C ou D) est donné par la consigne qui te lance. Chaque couloir a son worktree et sa branche : trouve la ligne de `git worktree list`, exécuté depuis le dépôt principal, dont la branche est `hig/plan-<couloir>`. Tout ton travail s'y passe : lecture, modification, compilation, commits.
- **Chaque commande shell repart du dépôt principal.** Commencer chaque commande par `cd <chemin de ton worktree> &&`, et éditer les fichiers par leur chemin absolu dans ton worktree.
- **La branche d'intégration** est `hig/plan-d-action`, extraite dans le worktree `~/.cache/hyprdesk-wt-hig`. Tu n'y écris jamais, tu n'y lances jamais de commande qui modifie, hormis le `reset --hard` de la section 7.
- **Interdits absolus** :
  - écrire dans le dépôt principal, celui de la branche `main`, hormis les fichiers `verrou*` de `.zcode/hig-boucle/` ;
  - écrire dans le worktree d'un autre couloir ou dans celui d'intégration ;
  - tout `push`, toute force, tout `reset --hard` ou `clean` hors de ton worktree ;
  - toucher à la configuration de la session (`~/.config/hypr/…`), lancer ou réinstaller le démon, `cargo install` ;
  - ajouter ou mettre à jour une dépendance : marquer `bloqué : dépendance` à la place ;
  - supprimer, désactiver ou affaiblir un test pour le faire passer ;
  - traiter un écart d'un autre couloir, même s'il semble libre.
- Le dépôt principal porte des modifications non commitées du propriétaire : elles n'existent pas dans les worktrees, c'est voulu. Ne jamais y toucher.

## 2. Décisions du propriétaire — acquises, ne plus redemander

Approuvées le 23 novembre 2026 :

| # | Décision |
| --- | --- |
| D1 | Un menu « hyprdesk » en tête de la barre, puis les menus Fenêtre, Espaces et Aide |
| D2 | Raccourcis façon macOS : SUPER+Espace ouvre le lanceur, SUPER+Tab le commutateur de fenêtres |
| D3 | Le commutateur « Configuration avancée » quitte l'en-tête du tiroir pour le corps des Réglages |
| D4 | Un masque unique, rectangle arrondi, pour les icônes d'application ; SmartUI génère un carré plein cadre |
| D5 | Un paquet de réglages Notifications : autorisation par application, style de remise |
| D6 | L'assistant de premier démarrage devient quittable à toute étape et reporte sa personnalisation |
| D7 | Un audit accessibilité, puis le traitement de ses écarts |

Marge de manœuvre accordée : « Modifie, déplace les widgets ; rien de ce qui est en place ne doit te bloquer. » Un placement, un réglage par défaut, une composition de widget existants peut donc changer quand un écart l'exige. Les écarts marqués `hors plan` et `identité` au registre ne se traitent pas.

## 3. Les couloirs

| Couloir | Sujets | Écarts |
| --- | --- | --- |
| A | Notifications, shell, matière, accessibilité | `SYS`, `PAT`, `STA`, `FON`, `ACC` ; plus `LAY-02`, `LAY-03`, `LAY-04`, `ICO-03`, `ICO-07`, `TYP-19` |
| B | Réglages, saisie, écriture, présentation | `SEL`, `NAV`, `TYP`, `LAY`, `PRES` ; plus `PAT-09`, `PAT-13`, `PAT-16`, `ACC-10` et la plupart des `DAT` |
| C | Menus, icônes, entrées, bureau | `MENU`, `ENT`, `ICO` ; plus `PAT-12`, `PAT-14`, `TYP-17` et les `DAT` du glisser-déposer |
| D | Typographie, écriture, présentation | `TYP` et `PRES`, sauf `TYP-17` (C) et `TYP-19` (A) |

La règle complète est dans `.zcode/hig-boucle/prochain.py` (`--regles` l'affiche). Ce script dit aussi ce qui reste à faire : `python3 -I .zcode/hig-boucle/prochain.py <ton couloir> 6`, lancé depuis le dépôt principal. Les quatre couloirs partagent le même registre ; ne lis jamais ce registre en entier.

## 4. Début d'itération, dans cet ordre

1. **Noter l'heure de début** (`date +%s`). L'itération dure 2 heures au plus ; à 1 h 45, ne plus commencer d'écart.
2. **Verrous**, dans `.zcode/hig-boucle/` du dépôt principal :
   - `verrou` : c'est celui de l'ancienne boucle à un seul couloir. S'il existe et a moins de trois heures, terminer tout de suite.
   - `verrou-<couloir>` : s'il existe et a moins de trois heures, une itération de ton couloir est en cours : terminer tout de suite. Sinon l'écrire (`date --iso-8601=seconds`). Il ne sera retiré qu'en fin d'itération.
3. **Arbre propre** : `git status --porcelain` doit être vide dans ton worktree. Sinon, une itération précédente a été interrompue : noter les fichiers, puis `git reset --hard HEAD` et `git clean -fd`, dans ton worktree seulement.
4. **Se mettre à jour de l'intégration** : `git rebase hig/plan-d-action`. Ta branche ne porte que tes propres commits, jamais poussés : la rebaser est sans risque.
   - En cas de conflit, le résoudre en comprenant les deux changements. Dans le registre, les conflits se résolvent par union des deux côtés.
   - Si tu n'es pas sûr : `git rebase --abort`, retirer ton verrou, terminer.
5. **Synchronisation avec main** : si `git merge-base --is-ancestor main HEAD` échoue, `main` a avancé : `git merge --no-edit main`. En cas de conflit : `git merge --abort`, ajouter « bloqué : conflit avec main » au journal de ton couloir, publier le registre (section 7, étapes c à e), retirer ton verrou, appeler `CronPauseSelf`, terminer.
6. **Consigne de référence** : `.zcode/hig-boucle/CONSIGNE.md` du dépôt principal fait foi.
7. **État de référence** : si l'étape 4 ou 5 a changé le code de ton arbre depuis ta dernière porte verte, ou si ton couloir n'a encore jamais passé la porte sur le code qui s'y trouve, passer la porte complète (section 6.7) avant toute modification. Échec : ne rien traiter, ajouter « référence cassée » au journal de ton couloir, publier le registre, retirer ton verrou, terminer. Troisième échec consécutif de ton couloir : appeler `CronPauseSelf`.

8. **Travail interrompu** : si `.zcode/hig-boucle/reprise-<couloir>.patch` existe, dans le dépôt principal, c'est le travail de ton couloir, interrompu par une panne de quota. Il s'applique après la mise à jour et l'état de référence, jamais avant : appliqué d'abord, il salirait l'index et le rebasage serait refusé. `git apply --3way <chemin du patch>`, puis reprendre là où il s'arrêtait : le mettre au point, pas le refaire. Après avoir publié le lot qu'il vise, supprimer le patch.

## 5. Choix du lot

- `python3 -I .zcode/hig-boucle/prochain.py <ton couloir> 6` donne les prochains écarts de ton couloir, dans l'ordre du registre.
- **D'abord l'écart `en cours` de ton couloir** s'il y en a un : on achève ce qui est commencé. Sinon, le premier `à faire` ou `à reprendre`.
- Autour de cet écart, prendre dans ton couloir, dans la même phase et, autant que possible, dans la même surface : mêmes fichiers, même composant.
- **Un lot** : un à trois écarts d'effort S, ou un écart M, ou une à deux étapes d'un écart L (section 6.4).
- **Recouvrements** : plusieurs écarts peuvent décrire le même changement (par exemple `SEL-01` et `DAT-09`). Un seul changement ; toutes leurs lignes passent `fait` avec le même hash et un renvoi mutuel. Si l'un d'eux appartient à un autre couloir, c'est le couloir de l'écart traité qui corrige et écrit `sans objet : couvert par <ID>` sur la ligne de l'autre.
- À 1 h 45, ne plus commencer d'écart ni d'étape.
- **S'il reste 20 à 45 minutes avant 1 h 45 et que le prochain écart est M ou L** : ne pas rester oisif. Écrire son plan d'implémentation (section 6.4), y compris la section « Étapes » pour un écart L, et le publier : c'est une étape à part entière. L'inscrire en remarque du registre (« plan écrit ») sans changer le statut, sauf pour un écart L, qui passe `en cours`.
- **Économiser les tours et les sorties** : le budget de génération est la ressource rare et quatre couloirs le partagent. Regrouper les commandes. Ne jamais afficher une sortie longue : la rediriger vers un fichier et n'en montrer que la queue ou les lignes d'erreur. Lire la seule ligne d'un écart (`grep -n '^| SYS-04 |' design-plans/bureau-macos/*.md`), jamais un document entier.

## 6. Traitement d'un écart

1. **Lire** sa ligne dans la seconde partie de son document de famille, et la règle de la HIG dans la première. Correspondance des préfixes :

   | Préfixe | Document |
   | --- | --- |
   | SYS | `07-system-experiences.md` |
   | MENU | `03-menus-and-actions.md` |
   | LAY | `02-layout.md` |
   | FON | `00a-fondations-matiere.md` |
   | ICO | `00c-fondations-iconographie.md` |
   | PAT | `01a-patterns-shell.md` |
   | DAT | `01b-patterns-donnees.md` |
   | NAV | `04-navigation-and-search.md` |
   | SEL | `04b-selection-and-input.md` |
   | TYP | `00b-fondations-structure.md` |
   | ENT | `08-entrees.md` |
   | PRES | `05-presentation.md` |
   | STA | `06-status.md` |
   | ACC | `audit-accessibilite.md` |

2. **Revérifier la preuve à HEAD**. Les numéros de ligne du plan ont été relevés à `00f6d69` : ils sont indicatifs. Retrouver le code par son contenu. Si l'écart n'existe plus : statut `sans objet` avec la raison, sans toucher au code. Si la preuve est inexacte mais l'écart réel : corriger et noter l'inexactitude en remarque.
3. **Concevoir en réutilisant** les propriétaires existants : `src/ui/tokens.rs`, `src/ui/design.rs`, `src/ui/menu.rs`, les cartes de `src/ui/plugins/bar/popover.rs`, `src/ui/panel.rs`, les plugins et le catalogue des widgets. Ne pas créer de primitive parallèle. Respecter :
   - `DESIGN.md` : une famille de police, l'accent réservé à la sélection et aux actions, espacement de base 4 px, rayons 10, 20 et 24 px, mouvement court et annulable, `transition = "none"` ;
   - `CONTRIBUTING.md` : le français est la langue source, l'anglais dans `locales/en.toml` ;
   - la règle de `DESIGN.md` : toute fonction utilisateur passe par un plugin ou un widget du catalogue existant ;
   - `boucle/orientation-hyprdesk.md` pour la carte du code ;
   - **une unité de la HIG vaut un pixel logique** : « 13 pt », « 20x20 pt » se lisent 13 px, 20 × 20 px dans le code.
4. **Effort M ou L seulement** : écrire d'abord un plan d'implémentation autonome dans `design-plans/bureau-macos/boucle/plans/<ID>.md`, sur le gabarit `boucle/gabarit-plan.md`.
   - Pour un écart L, ajouter au plan une section « Étapes » : une liste numérotée, chaque étape étant réalisable et vérifiable en une itération. Passer la ligne du registre à `en cours`.
   - Faire lire le plan par un agent indépendant si l'outil `Agent` est disponible (consigne : « lecture seule, ne modifie aucun fichier ; qu'est-ce qui casse ? que manque-t-il ? le plan résout-il l'écart ? »). Sans cet outil, le relire soi-même à froid, après une autre action, en écrivant explicitement les réponses aux trois mêmes questions.
   - Corriger le plan, puis l'exécuter. À chaque itération, mettre à jour « Étapes ». Quand la dernière est faite, l'écart passe `fait`.
5. **Réaliser** le changement minimal, borné à l'écart. Préserver ce qui est valide : options Lua, commandes en ligne, autres styles de barre, réglages existants.
6. **Documenter** : mettre à jour la documentation du projet quand le comportement ou le design visible change (`docs/GUIDE.fr.md` et `docs/GUIDE.md`, `docs/PLUGINS.md`, `docs/THEME-ENGINE.md`, `DESIGN.md`, `locales/en.toml`).
7. **Vérifier**, en deux étapes, toujours avec `CARGO_BUILD_JOBS=5` devant la commande (quatre couloirs compilent en même temps sur la même machine) :
   - **Contrôle rapide, avant la porte** : `CARGO_BUILD_JOBS=5 cargo fmt --all && CARGO_BUILD_JOBS=5 cargo clippy --locked --workspace --all-targets -- -D warnings`. Une erreur corrigée à ce stade, avertissement `clippy` compris, ne compte pas comme une tentative.
   - **Porte complète** : `CARGO_BUILD_JOBS=5 bash scripts/verify.sh > /tmp/hig-verify-<couloir>.log 2>&1; echo "exit=$?"`, puis `tail -n 20` du journal en cas d'échec. Le code de sortie doit être 0. Pour un écart visuel, `scripts/verify.sh --visual /tmp/hig-visual-<couloir>` de la même façon, puis regarder les captures des scènes concernées. Les scènes d'aperçu n'exécutent pas les tâches : une icône à résoudre l'est immédiatement, selon l'idiome déjà présent dans `src/ui/preview.rs`.
   - Retenir l'arbre vérifié : `git rev-parse HEAD^{tree}`.
8. **Relire**. Obligatoire pour un écart M ou L, et dès qu'un propriétaire partagé est modifié (`tokens.rs`, `design.rs`, `menu.rs`, `bar/popover.rs`, `panel.rs`, `src/notifications.rs`). Si l'outil `Agent` est disponible, un agent indépendant relit le diff (`git diff HEAD~1`), en lecture seule. Sinon, le relire soi-même à froid, en écrivant explicitement les réponses. Trois questions : le diff fait-il exactement ce que l'écart demande ? respecte-t-il `DESIGN.md` et les conventions du dépôt ? introduit-il une régression, manque-t-il un test ? Corriger ce qui est réel, puis repasser la porte.
9. **Échec** : deux tentatives au plus, une par itération. Une porte qui échoue sur une erreur mince (un test, une traduction, une mise en forme) se corrige et se repasse dans la même itération : jusqu'à trois passages de la porte par lot, sans compter pour une tentative. Si la porte échoue encore après le troisième passage : `git reset --hard HEAD`, statut `à reprendre`, « échec 1 : la cause » en remarque. À la tentative suivante, même chose ; si la porte échoue encore, `bloqué` (« échec 2 »).
   - Pour ne rien perdre d'un lot ainsi défait, l'orchestrateur en sauvegarde le diff dans `.zcode/hig-boucle/reprise-<couloir>.patch` : c'est son rôle, pas le tien. Ne rien écrire hors de ton worktree et de `.zcode/hig-boucle/`.

## 7. Publier sur l'intégration

Cette section s'applique à chaque lot, dès qu'il a passé la porte. Une itération peut en traiter plusieurs, jusqu'à 75 minutes.

a. **Un commit de code par lot**, sur ta branche de couloir, `git add` des seuls fichiers du lot. Message selon `CONTRIBUTING.md` : `Composant : description (HIG SYS-04, SYS-05)`. Ni numéro de ticket, ni mention d'outil, ni co-auteur.
b. **Rattraper l'intégration** : `git rebase hig/plan-d-action`, comme en 4.4. Si l'arbre a changé depuis la porte verte (`git rev-parse HEAD^{tree}` diffère de l'arbre retenu), repasser la porte complète.
c. **Mettre à jour le registre** dans ton worktree, puis le committer à part : `design-plans : boucle HIG, registre du couloir <X> (HIG SYS-04)`. Ce commit ne touche que des documents : pas de vérification à relancer. Il contient :
   - le statut (`fait`, `à reprendre`, `bloqué`, `sans objet`, `en cours`), le hash court du commit de code **tel qu'il est sur ta branche après le rebasage de l'étape b** ; le relire par `git log --oneline -5` juste avant d'écrire le registre, un hash d'avant rebasage est faux ; une ligne de remarque par écart ;
   - une ligne au journal de ton couloir : `| date | n° | écarts traités ; il en reste N dans ce couloir |`. Le numéro est celui de ton couloir : `A1`, `A2`, `B1`… (dernier numéro de ton tableau, plus un). Le journal compte un tableau par couloir (`### Couloir A`, `### Couloir B`, `### Couloir C`) ; si le tien manque, le créer sous l'ancien tableau unique, que tu intitules « Avant les couloirs ».
d. **Publier**, en une seule commande qui vérifie deux conditions avant d'écrire :
   `first=$(git rev-list --reverse hig/plan-d-action..HEAD | head -1) && git merge-base --is-ancestor hig/plan-d-action HEAD && git update-ref refs/heads/hig/plan-d-action HEAD "$(git rev-parse "${first}^")"`
   - **Fast-forward seulement** : l'intégration actuelle doit être contenue dans ta branche (`merge-base --is-ancestor`). Sinon un couloir a publié depuis ton rebasage : recommencer à l'étape b.
   - **Écriture conditionnelle** : la référence n'est mise à jour que si l'intégration vaut encore le commit sur lequel ta branche est bâtie, le parent de ton premier commit local.
   Succès : publié. Échec : recommencer à l'étape b, trois fois au plus. Passé là, terminer sans publier : ta branche conserve ton travail, la prochaine itération le rebasera.
   **Ne jamais relancer cette commande sans avoir rebasé.** Lire la pointe de l'intégration au moment de publier, au lieu de la comparer à celle du rebasage, écrase le travail publié entre-temps : c'est ainsi que la publication `SEL-03` du couloir B a été écrasée le 24 novembre.
e. **Synchroniser le worktree d'intégration**, qui doit refléter la branche. Juste avant la publication (étape d), vérifier que `git -C ~/.cache/hyprdesk-wt-hig status --porcelain` est vide ; sinon, ne pas le synchroniser et le signaler. Après une publication réussie, `git -C ~/.cache/hyprdesk-wt-hig reset --hard hig/plan-d-action`, sans condition : son statut y est forcément non vide, la branche ayant bougé sous lui.
f. **Fin** : retirer `verrou-<couloir>`. Compte rendu de six lignes au plus.
g. **Fin de plan** : si `python3 -I .zcode/hig-boucle/prochain.py --bilan` ne compte plus aucun `à faire`, `à reprendre` ni `en cours` dans aucun couloir, faire l'itération de clôture (section 9), publier, puis appeler `CronPauseSelf`. S'il ne reste rien que dans ton couloir, appeler `CronPauseSelf` après avoir publié.

| Statut | Sens |
| --- | --- |
| `à faire` | À traiter, dans l'ordre du registre |
| `en cours` | Écart L dont le plan est écrit et les étapes en cours de réalisation |
| `fait` | Traité, vérifié, publié : le hash est en colonne Commit |
| `à reprendre` | Première tentative échouée : à retenter à la prochaine itération |
| `bloqué` | Deux échecs, ou obstacle qui dépasse la boucle : à examiner par le propriétaire |
| `sans objet` | L'écart n'existe plus dans le code actuel, ou il est couvert par un autre écart traité |
| `hors plan` | Recommandation de la HIG non retenue : ne pas traiter |
| `identité` | Choix de design assumé : ne pas traiter |

## 8. Itération interrompue ou bloquée

- Une erreur d'outil, un dépassement de temps ou une incapacité à finir ne doit rien laisser en suspens : `git reset --hard HEAD` dans ton worktree, puis retirer ton verrou. Les écarts du lot restent à leur statut.
- Ne jamais laisser ton worktree sale en terminant.
- **Panne de quota** : si une erreur « Usage limit reached » (code 1308) interrompt ton travail, tout ce qui n'est pas committé est perdu pour toi ; l'orchestrateur le sauvegarde en patch et le confie à ta prochaine itération (section 4.8).

## 9. Itération de clôture

1. Ajouter `audit-accessibilite.md` en troisième partie de `00b-fondations-structure.md`.
2. Mettre à jour `design-plans/bureau-macos/README.md` et `plan-d-action.md` : écarts traités, bloqués, sans objet, par phase ; hash de tête.
3. Mettre à jour `DESIGN.md` avec les décisions de design désormais en place.
4. Committer, publier (section 7), retirer ton verrou, appeler `CronPauseSelf`.

## 10. Ce que les premières itérations ont appris

- L'audit accessibilité est fait : `design-plans/bureau-macos/audit-accessibilite.md`, dix écarts `ACC-01` à `ACC-10`, en fin de phase 1 au registre. `ACC-01` (aucune arborescence d'accessibilité : pont AT-SPI à construire au-dessus de `zbus`, déjà dépendance directe) est le plus gros écart du plan : il demande un plan en étapes.
- `SYS-01` est fait. `ICO-03` reste à traiter, en couloir A : il lui reste le repli sur l'icône de la classe de l'application (voir sa ligne au registre, annotée par l'itération 1).
- Un écart peut exiger de dépasser sa colonne Correction quand la règle de la HIG qu'il cite l'impose (ainsi `FON-02` : la bulle critique a pris le ton « attention », l'accent restant réservé à la sélection et aux actions). Noter le dépassement en remarque.
- `boucle/guide-auditeur.md` est la consigne de l'audit initial, archivée pour référence. Ses chemins (`/tmp/hig_notes/`, `spec-<famille>.md`), son commit (`6c5be247`) et son « dépôt » (le worktree, lecture seule) sont à adapter avant toute réutilisation.
- La boucle ne pousse jamais : la branche d'intégration n'est publiée sur le distant que par le propriétaire.
