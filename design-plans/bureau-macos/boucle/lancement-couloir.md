# Lancement d'une itération de couloir

> Texte à donner à un agent, une fois par couloir, en remplaçant `<X>` par A, B ou C. Les trois lancements se font en parallèle.

---

Exécute une itération du couloir <X> de la boucle agentique qui traite le plan d'action « bureau macOS d'après la HIG » du projet hyprdesk. Quatre couloirs (A, B, C, D) travaillent en parallèle, chacun dans son worktree ; tu es le couloir <X>.

1. Lis `.zcode/hig-boucle/CONSIGNE.md` à la racine du dépôt : c'est la consigne qui fait foi (version 3, trois couloirs). Lis-la en entier.

2. Applique-la à la lettre pour le couloir <X>, dans l'ordre : verrous (le `verrou` de transition, puis `verrou-<X>`, dans `.zcode/hig-boucle/` du dépôt principal) ; arbre propre dans ton worktree (branche `hig/plan-<X>`, trouvée par `git worktree list`) ; mise à jour par `git rebase hig/plan-d-action` ; synchronisation avec `main` ; état de référence ; travail interrompu (section 4.8 : `.zcode/hig-boucle/reprise-<X>.patch`) ; choix du lot par `python3 -I .zcode/hig-boucle/prochain.py <X> 6` ; traitement des écarts ; vérification (contrôle rapide, puis porte complète `bash scripts/verify.sh`, avec `CARGO_BUILD_JOBS=5`) ; commit de code ; mise à jour du registre ; publication sur l'intégration par `git update-ref` ; synchronisation du worktree d'intégration ; retrait de ton verrou.

3. Règles absolues : rien ne s'écrit dans le dépôt principal (celui de la branche `main`), hormis les fichiers `verrou*` et la suppression de ton patch de reprise ; rien dans le worktree d'un autre couloir ni dans celui d'intégration, hormis le `reset --hard` de synchronisation ; aucun `push` ; aucun commit de code sans `bash scripts/verify.sh` au code de sortie 0 ; aucun écart d'un autre couloir ; aucune question au propriétaire — trancher avec le plan, ou marquer l'écart « bloqué » ; ne crée, ne planifie et ne configure aucune autre tâche ni automatisation.

4. L'itération est bornée à 2 heures ; à 1 h 45, ne plus commencer d'écart.

5. Termine par un compte rendu de huit lignes au plus : écarts traités, écarts bloqués, publié ou non sur l'intégration, ce qui reste dans ton couloir. Puis, sur la dernière ligne, un seul mot d'état parmi :

   - `continuer` : il reste du travail dans ton couloir et la boucle peut suivre ;
   - `couloir-termine` : ton couloir n'a plus rien à traiter ;
   - `conflit-main` : la fusion de `main` est en conflit ;
   - `reference-cassee` : l'état de référence a échoué ;
   - `interrompu` : tu n'as pas pu terminer normalement.

N'appelle pas `CronPauseSelf` : l'arrêt de la boucle est décidé par l'orchestrateur, d'après ton mot d'état. N'effectue pas l'itération de clôture : l'orchestrateur la déclenche quand tous les couloirs sont terminés.
