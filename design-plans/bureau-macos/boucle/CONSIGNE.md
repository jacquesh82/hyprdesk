# Boucle HIG — consigne d'une itération

> À lire en entier avant d'agir, puis à appliquer à la lettre. Une itération = un tour de boucle.
> Le propriétaire n'est pas là pour répondre : trancher avec le plan, ou marquer « bloqué ». Ne jamais poser de question.

## 1. Où l'on travaille — et où l'on ne touche pas

- **Le worktree** : celui dont la branche est `hig/plan-d-action`. Le trouver par `git worktree list`, exécuté depuis le dépôt principal. Tout s'y passe : lecture, modification, compilation, commits.
- **Chaque commande shell repart du dépôt principal.** Commencer chaque commande par `cd <chemin du worktree> &&`, et éditer les fichiers par leur chemin absolu dans le worktree. Une commande lancée sans cette précaution agit sur `main` : c'est interdit.
- **Interdits absolus** :
  - toute écriture dans le dépôt principal, celui de la branche `main`, hormis le fichier `verrou` ;
  - tout `push`, toute force, tout `reset --hard` ou `clean` hors du worktree ;
  - toucher à la configuration de la session (`~/.config/hypr/…`), lancer ou réinstaller le démon, `cargo install` ;
  - ajouter ou mettre à jour une dépendance : marquer `bloqué : dépendance` à la place ;
  - supprimer, désactiver ou affaiblir un test pour le faire passer.
- Le dépôt principal porte des modifications non commitées du propriétaire : elles n'existent pas dans le worktree, c'est voulu. Ne jamais y toucher.

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

## 3. Début d'itération, dans cet ordre

1. **Noter l'heure de début** (`date +%s`). L'itération dure 90 minutes au plus.
2. **Verrou** : le fichier `.zcode/hig-boucle/verrou` du dépôt principal contient un horodatage. S'il existe et a moins de trois heures, terminer tout de suite : une itération est en cours. Sinon l'écrire (`date --iso-8601=seconds`). Il ne sera retiré qu'en fin d'itération (section 6).
3. **Arbre propre** : `git status --porcelain` doit être vide. Sinon, une itération précédente a été interrompue : noter les fichiers, puis `git reset --hard HEAD` et `git clean -fd`, dans le worktree seulement. Les écarts en cours restent à leur statut.
4. **Synchronisation avec main** : si `git merge-base --is-ancestor main HEAD` échoue, `main` a avancé : `git merge --no-edit main`. En cas de conflit : `git merge --abort`, ajouter une ligne « bloqué : conflit avec main » au journal du registre, committer le registre, retirer le verrou, appeler `CronPauseSelf` si l'outil est disponible, terminer.
5. **Consigne de référence** : le `CONSIGNE.md` du worktree fait foi, il peut avoir été amendée.
6. **État de référence** : seulement si `main` vient d'être fusionné (étape 4), ou si la ligne « Référence verte » manque au registre. Lancer alors `bash scripts/verify.sh` comme indiqué en 5.7, avant toute modification. Échec : ne rien traiter, ajouter « référence cassée » au journal (avec le compteur), retirer le verrou, terminer. Troisième échec consécutif : appeler `CronPauseSelf` si l'outil est disponible. Succès : mettre « Référence verte » à HEAD et annuler le compteur.

## 4. Choix du lot

- Le registre `design-plans/bureau-macos/boucle/avancement.md` donne l'ordre de traitement. Il dépasse 200 lignes : le lire par extraits (`grep -n '| en cours |\|| à reprendre |\|| à faire |' … | head -12`), jamais en entier.
- **D'abord l'écart `en cours` le plus ancien**, s'il y en a un : on achève ce qui est commencé. Sinon, le premier `à faire` ou `à reprendre`.
- Autour de cet écart, prendre dans la même phase et, autant que possible, dans la même surface : mêmes fichiers, même composant.
- **Un lot** : un à trois écarts d'effort S, ou un écart M, ou une à deux étapes d'un écart L (section 5.4).
- **Recouvrements** : plusieurs écarts peuvent décrire le même changement (par exemple `SYS-01` et `ICO-03`, `SEL-01` et `DAT-09`, `PAT-12` et `DAT-12`). Les prendre dans le même lot : un seul changement, et toutes leurs lignes passent `fait` avec le même hash et un renvoi mutuel.
- À 75 minutes, ne plus commencer d'écart ni d'étape. Ce qui ne rentre pas dans le temps restant attend la prochaine itération.
- **Économiser les tours et les sorties** : le budget de génération est la ressource rare, et une itération d'essai en a consommé 22 millions de jetons. Regrouper les commandes. Ne jamais afficher une sortie longue : la rediriger vers un fichier et n'en montrer que la queue ou les lignes d'erreur (`| tail -n 20`, `grep -n error`). Lire la seule ligne d'un écart (`grep -n '^| SYS-01 |' design-plans/bureau-macos/*.md`), jamais un document entier.

## 5. Traitement d'un écart

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
   - Corriger le plan, puis exécuter. À chaque itération, mettre à jour « Étapes » (faites, restantes). Quand la dernière est faite, l'écart passe `fait`.
5. **Réaliser** le changement minimal, borné à l'écart. Préserver ce qui est valide : options Lua, commandes en ligne, autres styles de barre, réglages existants.
6. **Documenter** : mettre à jour la documentation du projet quand le comportement ou le design visible change (`docs/GUIDE.fr.md` et `docs/GUIDE.md`, `docs/PLUGINS.md`, `docs/THEME-ENGINE.md`, `DESIGN.md`, `locales/en.toml`).
7. **Vérifier**, en deux étapes :
   - **Contrôle rapide, avant la porte** : `cargo fmt --all && cargo check --locked --workspace --all-targets`. Une erreur corrigée à ce stade ne compte pas comme une tentative.
   - **Porte complète** : `bash scripts/verify.sh > /tmp/hig-verify.log 2>&1; echo "exit=$?"`, puis `tail -n 20 /tmp/hig-verify.log` en cas d'échec. Le code de sortie doit être 0. Pour un écart visuel, `bash scripts/verify.sh --visual /tmp/hig-visual` de la même façon, puis regarder les captures des scènes concernées dans `/tmp/hig-visual/<largeur>/`. Les scènes d'aperçu n'exécutent pas les tâches : une icône à résoudre l'est immédiatement, selon l'idiome déjà présent dans `src/ui/preview.rs`.
8. **Relire**. Obligatoire pour un écart M ou L, et dès qu'un propriétaire partagé est modifié (`tokens.rs`, `design.rs`, `menu.rs`, `bar/popover.rs`, `panel.rs`, `src/notifications.rs`). Si l'outil `Agent` est disponible, un agent indépendant relit le diff (`git diff HEAD~1`), en lecture seule. Sinon, le relire soi-même à froid, en écrivant explicitement les réponses. Trois questions : le diff fait-il exactement ce que l'écart demande ? respecte-t-il `DESIGN.md` et les conventions du dépôt ? introduit-il une régression, manque-t-il un test ? Corriger ce qui est réel, puis repasser la porte.
9. **Échec** : deux tentatives au plus, comptées par échec de la porte complète. Si elle ne passe pas : `git reset --hard HEAD`, statut `à reprendre`, « échec N : la cause » en remarque. Au deuxième échec, `bloqué`.

## 6. Commit, registre, fin d'itération

- **Un commit de code par lot**, sur la branche du worktree, `git add` des seuls fichiers du lot. Message selon `CONTRIBUTING.md` : `Composant : description (HIG SYS-01, SYS-04)`. Ni numéro de ticket, ni mention d'outil, ni co-auteur.
- **Le registre n'entre pas dans ce commit** : un commit ne peut pas contenir son propre hash. Il est committé à part, une fois par lot et une fois en fin d'itération : `design-plans : boucle HIG, registre (HIG SYS-01)`. Ce commit ne touche que des documents : la vérification du lot couvre un code identique, il n'y a pas à la relancer.
- **Ce que contient la mise à jour du registre** : le statut (`fait`, `à reprendre`, `bloqué`, `sans objet`, `en cours`), le hash court du commit de code, une ligne de remarque par écart (ce qui a changé, décision prise, documentation mise à jour).
- **Le journal** : une ligne par itération, `| date | n° | écarts traités ; il en reste N |`. Le numéro suit le dernier numéro écrit au journal. « Référence verte » porte le hash du dernier commit de code qui a passé la porte complète.
- **Fin** : retirer le verrou. Compter `fait`, `à faire`, `à reprendre`, `en cours`, `bloqué`, `sans objet`. Un compte rendu de six lignes au plus.
- **Fin de plan** : s'il ne reste aucun `à faire`, `à reprendre` ni `en cours`, faire l'itération de clôture (section 8), puis appeler `CronPauseSelf` si l'outil est disponible.

| Statut | Sens |
| --- | --- |
| `à faire` | À traiter, dans l'ordre du registre |
| `en cours` | Écart L dont le plan est écrit et les étapes en cours de réalisation |
| `fait` | Traité, vérifié, committé : le hash est en colonne Commit |
| `à reprendre` | Première tentative échouée : à retenter à la prochaine itération |
| `bloqué` | Deux échecs, ou obstacle qui dépasse la boucle : à examiner par le propriétaire |
| `sans objet` | L'écart n'existe plus dans le code actuel, ou il est couvert par un autre écart traité |
| `hors plan` | Recommandation de la HIG non retenue : ne pas traiter |
| `identité` | Choix de design assumé : ne pas traiter |

## 7. Itération interrompue ou bloquée

- Une erreur d'outil, un dépassement de temps ou une incapacité à finir ne doit rien laisser en suspens : `git reset --hard HEAD`, le verrou est retiré. Les écarts du lot restent à leur statut.
- Ne jamais laisser le worktree sale en terminant.

## 8. Itération de clôture

1. Ajouter `audit-accessibilite.md` en troisième partie de `00b-fondations-structure.md`.
2. Mettre à jour `design-plans/bureau-macos/README.md` et `plan-d-action.md` : écarts traités, bloqués, sans objet, par phase ; hashes de tête.
3. Mettre à jour `DESIGN.md` avec les décisions de design désormais en place.
4. Committer, retirer le verrou, appeler `CronPauseSelf` si l'outil est disponible.

## 9. Ce que la première itération a appris

- L'audit accessibilité est fait : `design-plans/bureau-macos/audit-accessibilite.md`, dix écarts `ACC-01` à `ACC-10`, en fin de phase 1 au registre. `ACC-01` (aucune arborescence d'accessibilité : pont AT-SPI à construire au-dessus de `zbus`, déjà dépendance directe) est le plus gros écart du plan : il demande un plan en étapes.
- `SYS-01` est fait ; `ICO-03` décrit le même changement et doit passer `sans objet : couvert par SYS-01`.
- `boucle/guide-auditeur.md` est la consigne de l'audit initial, archivée pour référence. Ses chemins (`/tmp/hig_notes/`, `spec-<famille>.md`), son commit (`6c5be247`) et son « dépôt » (le worktree, lecture seule) sont à adapter avant toute réutilisation.
