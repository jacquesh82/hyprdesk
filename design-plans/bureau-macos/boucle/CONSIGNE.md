# Boucle HIG — consigne d'une itération

> À lire en entier avant d'agir, puis à appliquer à la lettre. Une itération = un tour de boucle.
> Le propriétaire n'est pas là pour répondre : trancher avec le plan, ou marquer « bloqué ». Ne jamais poser de question.

## 1. Où l'on travaille — et où l'on ne touche pas

- **Le worktree** : celui dont la branche est `hig/plan-d-action`. Le trouver par `git worktree list`, exécuté depuis le dépôt principal. Tout s'y passe : lecture, modification, compilation, commits.
- **Chaque commande shell repart du dépôt principal.** Commencer chaque commande par `cd <chemin du worktree> &&`, et éditer les fichiers par leur chemin absolu dans le worktree. Une commande lancée sans cette précaution agit sur `main` : c'est interdit.
- **Interdits absolus** :
  - toute écriture dans le dépôt principal, celui de la branche `main` ;
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
2. **Verrou** : le fichier `.zcode/hig-boucle/verrou` du dépôt principal contient un horodatage. S'il existe et a moins de trois heures, terminer tout de suite : une itération est en cours. Sinon l'écrire (`date --iso-8601=seconds`). Il sera retiré en fin d'itération.
3. **Arbre propre** : `git status --porcelain` doit être vide. Sinon, une itération précédente a été interrompue : inscrire les fichiers en remarque du registre (« à reprendre : … »), puis `git reset --hard HEAD` et `git clean -fd`, dans le worktree seulement.
4. **Synchronisation avec main** : si `git merge-base --is-ancestor main HEAD` échoue, `main` a avancé : `git merge --no-edit main`. En cas de conflit : `git merge --abort`, ajouter une ligne « bloqué : conflit avec main » au journal du registre, committer le registre, retirer le verrou, appeler `CronPauseSelf`, terminer.
5. **Consigne de référence** : le `CONSIGNE.md` du worktree fait foi, il peut avoir été amendée. Le lire.
6. **État de référence** : si le hash noté « Référence verte » dans le registre n'est pas HEAD, lancer `bash scripts/verify.sh` avant toute modification. Échec : ne rien traiter, ajouter « référence cassée » au journal (avec le compteur), retirer le verrou, terminer. Troisième échec consécutif : appeler `CronPauseSelf`. Succès : mettre « Référence verte » à jour.

## 4. Choix du lot

- Le registre `design-plans/bureau-macos/boucle/avancement.md` donne l'ordre de traitement. Seuls les statuts `à faire` et `à reprendre` se traitent.
- Prendre les premiers dans l'ordre, en restant dans la même phase et, autant que possible, dans la même surface : mêmes fichiers, même composant.
- Un lot : un à trois écarts d'effort S, ou un écart M ou L.
- À 75 minutes, ne plus commencer d'écart. Un écart dont on estime qu'il ne rentrera pas dans le temps restant attend la prochaine itération.
- Économiser les tours : le budget de génération est la ressource rare. Regrouper les commandes, lire la seule ligne de l'écart (`grep -n '^| SYS-01 |' design-plans/bureau-macos/*.md`), ne pas relire un document entier.

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

2. **Revérifier la preuve à HEAD**. Les numéros de ligne du plan ont été relevés à `00f6d69` : ils sont indicatifs. Retrouver le code par son contenu. Si l'écart n'existe plus : statut `sans objet` avec la raison, sans toucher au code.
3. **Concevoir en réutilisant** les propriétaires existants : `src/ui/tokens.rs`, `src/ui/design.rs`, `src/ui/menu.rs`, les cartes de `src/ui/plugins/bar/popover.rs`, `src/ui/panel.rs`, les plugins et le catalogue des widgets. Ne pas créer de primitive parallèle. Respecter :
   - `DESIGN.md` : une famille de police, l'accent réservé à la sélection et aux actions, espacement de base 4 px, rayons 10, 20 et 24 px, mouvement court et annulable, `transition = "none"` ;
   - `CONTRIBUTING.md` : le français est la langue source, l'anglais dans `locales/en.toml` ;
   - la règle de `DESIGN.md` : toute fonction utilisateur passe par un plugin ou un widget du catalogue existant ;
   - `boucle/orientation-hyprdesk.md` pour la carte du code.
4. **Effort M ou L seulement** : écrire d'abord un plan d'implémentation autonome dans `design-plans/bureau-macos/boucle/plans/<ID>.md`, sur le gabarit `boucle/gabarit-plan.md`. Le faire lire par un agent indépendant (outil Agent, consigne « lecture seule, ne modifie aucun fichier ») : « qu'est-ce qui casse ? que manque-t-il ? le plan résout-il l'écart ? ». Corriger le plan, puis l'exécuter.
5. **Réaliser** le changement minimal, borné à l'écart. Préserver ce qui est valide : options Lua, commandes en ligne, autres styles de barre, réglages existants.
6. **Documenter** : mettre à jour la documentation du projet quand le comportement ou le design visible change (`docs/GUIDE.fr.md` et `docs/GUIDE.md`, `docs/PLUGINS.md`, `docs/THEME-ENGINE.md`, `DESIGN.md`, `locales/en.toml`).
7. **Vérifier** : `cargo fmt --all`, puis `bash scripts/verify.sh`. Code de sortie 0 obligatoire. Pour un écart visuel, `bash scripts/verify.sh --visual /tmp/hig-visual` puis regarder les captures des scènes concernées dans `/tmp/hig-visual/<largeur>/`.
8. **Relire**. Obligatoire pour un écart M ou L, et dès qu'un propriétaire partagé est modifié (`tokens.rs`, `design.rs`, `menu.rs`, `bar/popover.rs`, `panel.rs`, `src/notifications.rs`) : un agent indépendant relit le diff (`git diff`), en lecture seule. Trois questions : le diff fait-il exactement ce que l'écart demande ? respecte-t-il `DESIGN.md` et les conventions du dépôt ? introduit-il une régression, manque-t-il un test ? Corriger ce qui est réel.
9. **Échec** : deux tentatives au plus par itération. Si la vérification ne passe pas : `git reset --hard HEAD`, statut `à reprendre`, « échec N : la cause » en remarque. Au deuxième échec, `bloqué`.

## 6. Commit, registre, fin d'itération

- **Un commit par lot**, sur la branche du worktree, `git add` des seuls fichiers du lot. Message selon `CONTRIBUTING.md` : `Composant : description (HIG SYS-01, SYS-04)`. Ni numéro de ticket, ni mention d'outil, ni co-auteur.
- **Le registre entre dans le même commit** : statut `fait`, hash court du commit, une ligne de remarque par écart (ce qui a changé, décision prise, documentation mise à jour). Une ligne au journal : `| date | n° d'itération | écarts traités ; il en reste N |`. « Référence verte » porte le hash vérifié.
- **Fin** : retirer le verrou. Compter `fait`, `à faire`, `à reprendre`, `bloqué`, `sans objet`. Un compte rendu de six lignes au plus.
- **Fin de plan** : s'il ne reste aucun `à faire` ni `à reprendre`, faire l'itération de clôture (section 9), puis appeler `CronPauseSelf`.

| Statut | Sens |
| --- | --- |
| `à faire` | À traiter, dans l'ordre du registre |
| `fait` | Traité, vérifié, committé : le hash est en colonne Commit |
| `à reprendre` | Première tentative échouée : à retenter à la prochaine itération |
| `bloqué` | Deux échecs, ou obstacle qui dépasse la boucle : à examiner par le propriétaire |
| `sans objet` | L'écart n'existe plus dans le code actuel |
| `hors plan` | Recommandation de la HIG non retenue : ne pas traiter |
| `identité` | Choix de design assumé : ne pas traiter |

## 7. Itération interrompue ou bloquée

- Une erreur d'outil, un dépassement de temps ou une incapacité à finir ne doit rien laisser en suspens : `git reset --hard HEAD`, les écarts du lot restent `à faire` ou passent `à reprendre`, le verrou est retiré.
- Ne jamais laisser le worktree sale en terminant, sauf conflit de fusion avorté.

## 8. Audit accessibilité — première tâche du registre

La ligne `AUDIT-A11Y` se traite en premier :

1. Auditer le code du worktree face à la page « Accessibility » de la HIG, spécifiée dans la première partie de `00b-fondations-structure.md`. Appliquer `boucle/guide-auditeur.md` à la lettre, avec une exception : l'accessibilité est ici le sujet, la clause qui l'exclut ne s'applique pas.
2. Écrire `design-plans/bureau-macos/audit-accessibilite.md`, identifiants `ACC-01`, `ACC-02`…, classés du plus important au moins important. Viser 10 à 25 écarts prouvés.
3. Ajouter leurs lignes au registre, à la fin de la phase 1, avec le statut `à faire`.
4. Marquer `AUDIT-A11Y` `fait`, committer, retirer le verrou. Poursuivre avec la phase 0 si le temps le permet.

## 9. Itération de clôture

1. Ajouter `audit-accessibilite.md` en troisième partie de `00b-fondations-structure.md`.
2. Mettre à jour `design-plans/bureau-macos/README.md` et `plan-d-action.md` : écarts traités, bloqués, sans objet, par phase ; hashes de tête.
3. Mettre à jour `DESIGN.md` avec les décisions de design désormais en place.
4. Committer, retirer le verrou, appeler `CronPauseSelf`.
