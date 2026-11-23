# Audit HIG — guide de l'auditeur

> Document de travail. À lire avec `orientation-hyprdesk.md` (même dossier).

## Ta tâche

Comparer l'implémentation de hyprdesk au chapitre de spécification HIG de TA famille, et écrire le chapitre « hyprdesk face à la HIG » correspondant. Ton chapitre sera assemblé derrière la spécification, dans un document lu par le propriétaire du projet puis par l'agent qui réalisera les changements retenus.

## Lectures obligatoires, dans l'ordre

1. `orientation-hyprdesk.md` (même dossier) (architecture, décisions d'identité, méthode).
2. Le chapitre de spécification de ta famille, EN ENTIER.
3. Le code de hyprdesk, en lecture seule absolue : aucun fichier du dépôt modifié, aucun build, test ou installation, aucune configuration touchée.

## Économiser les tours

Le débit de génération est la ressource rare. Chaque aller-retour coûte cher.

- Regrouper : un seul appel Bash peut enchaîner plusieurs `grep -n` et `sed -n 'a,bp'` avec des en-têtes `echo`. Viser 6 à 10 appels au total pour toute la lecture du code, pas trente.
- Cibler : partir des fichiers indicatifs, chercher par mot-clé (taille, rayon, durée, délai, fermeture, clavier…), lire les seules régions utiles.
- Puis écrire le chapitre en un seul `Write` (ou `Write` puis `Edit` au-delà de 300 lignes). Pas d'aller-retour de relecture.

## Catégories de conclusion — une seule par écart

| Nature | Sens | Condition |
| --- | --- | --- |
| ÉCART | Règle HIG impérative non respectée | La HIG dit « Make sure », « Always », « Never », « Don't », une cote ou un tableau ; le chemin de rendu prouve l'écart |
| PISTE | Recommandation HIG non suivie | La HIG dit « Consider », « Prefer », « Generally » ; à peser, pas à corriger d'office |
| ABSENT | Composant ou comportement macOS sans équivalent | Recherché dans le code, aux styles de barre et au catalogue des widgets : introuvable |
| IDENTITÉ | Choix délibéré documenté | `DESIGN.md` ou un plan de `design-plans/` le dit ; à signaler, jamais à corriger |
| À VÉRIFIER | Ne se juge que sur un rendu | Densité perçue, hiérarchie, lisibilité réelle du verre ; aucune conclusion |

Hors périmètre : bugs de fonctionnement, architecture, performances, qualité du code, accessibilité.

## Preuve exigée

Chaque ligne ÉCART, PISTE ou ABSENT doit porter :

- **Le contrat** : la règle HIG avec sa page et sa section (`slug › section`), et ses mots-clés de force (impératif ou recommandation).
- **Le réel** : `chemin:ligne` du code qui rend la surface concernée, avec la valeur ou le comportement effectif. Pas de « semblait », pas de ligne non ouverte.
- **Une seule action** : la correction que commande la preuve. Ni alternative, ni intention de produit inventée.

## Falsifier avant de rendre

Pour chaque ÉCART et ABSENT, rouvrir le fichier cité et chercher activement ce qui l'infirme : une option de configuration, un autre style de barre, un widget du catalogue, un réglage de thème, un second chemin de rendu. Supprimer ce qui ne résiste pas. Un écart supprimé coûte moins cher qu'un écart faux.

## Livrable

`/tmp/hig_notes/audit-<famille>.md`, en français :

```
# <Famille> — hyprdesk face à la HIG
> Référentiel : spec-<famille>.md (HIG, relevé du 23 novembre 2026). Code : commit 6c5be247, arbre de travail avec modifications non commitées.

## Ce que hyprdesk fait aujourd'hui
| Surface | Ce qui existe | Valeurs clés | Preuves |
(liste des surfaces utiles de la famille ; une ligne par surface ; chemins:lignes)

## Écarts
| # | Composant | Nature | Force | Règle HIG (slug › section) | Réel dans hyprdesk (preuve) | Action proposée | Effort | Confiance |

## Ce qui est déjà conforme
(règle | preuve, en liste compacte)

## À vérifier sur un rendu
(ce qui demande une capture ou un essai en session, sans conclusion)
```

- Identifiants : préfixe de la famille, numérotation continue (`NOT-01`, `MENU-01`…). Le plan d'action les reprendra tels quels.
- Effort : S (moins d'un jour), M (un à trois jours), L (plus de trois jours) — ordre de grandeur.
- Confiance : haute (règle explicite, chemin tracé, falsification tentée), moyenne, faible.
- Proposer 10 à 25 écarts utiles, classés du plus important au moins important. Mieux vaut peu et prouvé que beaucoup et douteux.

Terminer par une réponse de 10 lignes au maximum : fichier, nombre d'écarts par nature, les 3 plus importants.
