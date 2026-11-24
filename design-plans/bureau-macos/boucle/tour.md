# Tour de la boucle HIG — prompt de l'automatisation

> Version du 24 novembre 2026, après le tour de 3 h 45. C'est le texte à donner à l'automatisation « En continu : boucle agentique HIG, trois couloirs en parallèle » (toutes les deux heures). L'automatisation en place porte encore la version précédente, sans l'étape 4b.
> Pour lancer un tour à la main : appliquer ce texte.

---

Exécute un tour de la boucle agentique qui traite le plan d'action « bureau macOS d'après la HIG » du projet hyprdesk. Trois couloirs (A, B, C) travaillent en parallèle, chacun dans son worktree et sur sa branche ; un tour lance les trois.

0. **Veille de quota.** Si le fichier `.zcode/hig-boucle/stand-by` existe, dans ce dossier, et contient une date ISO postérieure à l'heure actuelle, termine tout de suite : le quota de génération est épuisé jusqu'à cette date. Si la date est passée, supprime ce fichier et continue.

1. Lis `.zcode/hig-boucle/CONSIGNE.md` à la racine du dépôt : c'est la consigne qui fait foi pour les couloirs (version 3).

2. Si le fichier `.zcode/hig-boucle/verrou` existe, dans ce dossier, et a moins de trois heures, termine tout de suite : une itération de l'ancienne boucle, à un seul couloir, est encore en cours.

3. Lance en parallèle, avec l'outil Agent (en arrière-plan, type general-purpose), trois itérations, une par couloir : pour X valant A, B et C, donne à l'agent le texte de `.zcode/hig-boucle/lancement-couloir.md`, en remplaçant `<X>` par la lettre du couloir. Chaque agent applique seul la consigne : verrous, mise à jour, lot, traitement, vérification, commit, registre, publication sur l'intégration.

4. Attends le retour des trois (TaskOutput, en bloquant), puis établis le bilan : `python3 -I .zcode/hig-boucle/prochain.py --bilan`.

4b. **Intégrité.** Si le bilan signale qu'un couloir a des commits sur sa branche qui ne sont pas sur l'intégration (ligne « ATTENTION »), son travail publié a été écrasé ou n'a pas été publié : fais-le republier par ce couloir, et par lui seul. Reprends son agent (SendMessage) ou lance un agent avec le texte de lancement, avec pour seule tâche de poser son verrou, de rebaser sur `hig/plan-d-action`, de repasser la porte si l'arbre a changé, puis de publier selon la section 7 de la consigne. Ne publie rien toi-même.

5. **Panne de quota.** Si un agent a échoué avec « Usage limit reached » (code 1308), le quota est épuisé pour tout le monde : n'en relance aucun. Écris dans `.zcode/hig-boucle/stand-by` la date de remise à zéro donnée par l'erreur, plus dix minutes. Pour chaque couloir arrêté ainsi : sauvegarde son travail non committé (`git -C <son worktree> diff > .zcode/hig-boucle/reprise-<X>.patch`), remets son worktree propre (`git -C <son worktree> reset --hard HEAD`, puis `git clean -fd`), retire son `verrou-<X>`. Termine.

6. Décisions d'arrêt, d'après le mot d'état de la dernière ligne de chaque compte rendu. N'appelle `CronPauseSelf` que dans l'un de ces trois cas, et jamais pour une autre raison :
   - les trois couloirs répondent `couloir-termine`, et le bilan ne compte plus aucun `à faire`, `à reprendre` ni `en cours` : lance d'abord, avec l'outil Agent, l'itération de clôture décrite par la consigne (section 9), dans le worktree du couloir A ; quand elle est publiée, appelle `CronPauseSelf` ;
   - au moins un couloir répond `conflit-main` : appelle `CronPauseSelf` ;
   - les trois répondent `reference-cassee` : appelle `CronPauseSelf`.
   Un couloir qui répond `continuer` ou `interrompu` n'arrête rien : il repartira au prochain tour.

7. Hors des fichiers de `.zcode/hig-boucle/` et du nettoyage de l'étape 5, ne modifie aucun fichier du dépôt toi-même : tout le travail passe par les agents de couloir. N'écris rien dans le dépôt principal, hors de `.zcode/hig-boucle/`. Ne crée, ne planifie et ne configure aucune autre tâche ni automatisation.

8. Termine par un compte rendu de huit lignes au plus, un par couloir plus le bilan.
