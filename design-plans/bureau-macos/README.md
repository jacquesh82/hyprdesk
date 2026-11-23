# Bureau macOS d'après la HIG — spécifications, audit de hyprdesk et plan d'action

> Proposition de design du 23 novembre 2026. **Rien n'est implémenté** : ce dossier ne modifie ni le code, ni `DESIGN.md`, ni la configuration.
> Référentiel : Human Interface Guidelines (HIG) d'Apple, export Markdown officiel relevé le 23 novembre 2026, 95 pages.
> Code audité : commit `00f6d69`, arbre de travail avec huit fichiers modifiés non commités.

## Par où commencer

1. `plan-d-action.md` : lecture d'ensemble, sept chantiers, cinq phases, décisions à prendre.
2. Un document par famille, chacun en deux parties : la spécification d'après la HIG, puis ce que hyprdesk fait aujourd'hui, ses écarts et l'action proposée pour chacun.

| Document | Famille | Écarts |
| --- | --- | --- |
| `00a-fondations-matiere.md` | Matière, couleur, mouvement, principes de design | 14 |
| `00b-fondations-structure.md` | Typographie, écriture, disposition générale | 19 |
| `00c-fondations-iconographie.md` | Icônes et images | 16 |
| `01a-patterns-shell.md` | Patterns : comportements du shell | 21 |
| `01b-patterns-donnees.md` | Patterns : données, documents, accompagnement | 20 |
| `02-layout.md` | Composants : disposition et organisation | 16 |
| `03-menus-and-actions.md` | Composants : menus et actions | 21 |
| `04-navigation-and-search.md` | Composants : navigation et recherche | 15 |
| `04b-selection-and-input.md` | Composants : sélection et saisie | 14 |
| `05-presentation.md` | Composants : présentation | 16 |
| `06-status.md` | Composants : statut | 4 |
| `07-system-experiences.md` | Composants : expériences système | 23 |
| `08-entrees.md` | Entrées : clavier, pointeur, gestes, focus | 14 |

## Le bureau macOS en une page

Ce que la HIG décrit du bureau, avec le chapitre où le détail et les citations se trouvent.

- **Barre des menus** (`03`) : 24 pt de haut. Le menu Apple vient toujours en tête, les éléments de barre (menu bar extras) toujours en queue, et l'ordre des menus est fixe : application, Fichier, Édition, Présentation, Fenêtre, Aide. Elle est l'inventaire des commandes : une commande disponible ailleurs y figure quand même. Une commande sans effet se désactive, elle ne se masque jamais.
- **Dock et menus du Dock** (`03`) : le menu du Dock liste les fenêtres ouvertes, puis les commandes propres à l'application.
- **Notifications** (`07`, `01a`) : une bulle porte l'icône de l'application, un titre, un corps, au plus quatre actions. La demande d'autorisation précède tout envoi ; les styles de remise, le regroupement et les niveaux d'interruption se règlent par application.
- **Centre de contrôle** (`07`) : des modules qui affichent un symbole et, en grande taille, un titre et une valeur.
- **Widgets** (`07`) : sur le bureau et dans le Centre de notifications, en quatre tailles sur Mac ; un clic sur le widget ouvre l'application.
- **Fenêtres et surfaces** (`05`) : trois états de fenêtre ; la fenêtre active se repère à ses boutons en couleur. La feuille et l'alerte sont modales, le popover est transitoire et non modal, le panneau flotte.
- **Espaces et plein écran** (`01a`) : plusieurs espaces, du multitâche, un mode plein écran où la barre des menus se masque mais se révèle au pointeur.
- **Réglages** (`01a`, `04`) : une fenêtre à volets, barre latérale à gauche, recherche en tête de la sidebar, dernier volet restauré.
- **Clavier et gestes** (`08`) : 116 raccourcis standard à ne pas détourner, l'ordre Control, Option, Shift, Command, et un raccourci de focus par zone (Control-F2 à F6).
- **Matières et couleurs** (`00a`) : le verre Liquid Glass pour la couche des contrôles et de la navigation, les matériaux standard pour le contenu ; des couleurs sémantiques ; un mode sombre réglé par le système.
- **Typographie** (`00b`) : 13 pt par défaut sur macOS, 10 pt au minimum, onze styles de texte.

## Quatre limites du référentiel, à connaître avant de lire

1. **La HIG ne publie presque aucune cote.** Ni hauteur d'élément, ni rayon, ni délai, ni taille de cible. Elle fixe des règles de structure et de comportement. Les cotes souvent avancées pour « reproduire macOS » ne viennent pas d'elle.
2. **Ses pages « Patterns » parlent aux applications, pas au bureau.** Elles ne décrivent ni Spotlight, ni le Centre de notifications, ni Spaces, Stage Manager ou Mission Control. Ces sujets ne sont donc pas spécifiés ici.
3. **Aucune page ne donne la taille du Dock ni son grossissement.**
4. **L'audit porte sur le code, pas sur un rendu.** Aucune capture n'a été prise. Ce qui ne se juge que visuellement est listé « À vérifier sur un rendu » en fin de chaque audit, sans conclusion.

## Chiffres

| | |
| --- | --- |
| Chapitres de spécification | 13, 4 477 lignes |
| Citations rattachées à une page HIG | 2 233, contrôlées mot pour mot contre les pages d'origine |
| Citations signalées par l'outil | 24, examinées une à une : textes alternatifs d'images et guillemets imbriqués, aucune inexacte |
| Audits du code | 13, 967 lignes |
| Écarts relevés | 213 : 66 ÉCART, 42 ABSENT, 89 PISTE, 16 IDENTITÉ |
| Corrections d'un jour au plus | 134 |

Chaque auditeur a tenté de falsifier ses propres écarts avant de les rendre ; plusieurs pistes ont été retirées ainsi. Les écarts majeurs de chaque audit ont en outre été rouverts ligne par ligne lors de l'assemblage : tous confirmés.

## Langage de design

- **Surface auditée** : le shell hyprdesk entier — barre, dock, lanceur, recherche, notifications, réglages, widgets, aperçu des espaces, verrouillage, écran de connexion —, périmètre fixé par la demande.
- **Sources de design** : `DESIGN.md` ; `src/ui/design.rs` ; `src/ui/tokens.rs` ; `src/theme_engine/design.rs` ; packs de `src/theme_engine/packs/`.
- **Décisions documentées** : police Mindlog, accent réservé à la sélection et aux actions, verre fumé pour les petites cartes, espacement de base 4 px, rayons 10, 20 et 24 px, mouvement court et annulable, accent iris par défaut.
- **Propriétaires** : `tokens.rs` (jetons de typographie, verre, états, mouvement) ; `design.rs` (rayons et surfaces) ; `menu.rs` (composant de menu unique, commit `00f6d69`) ; `plugins/bar/popover.rs` et `popover/continuity.rs` (cartes de la barre et centre de contrôle) ; `toasts.rs` et `src/notifications.rs` (notifications) ; `panel.rs` (tiroir des Réglages) ; `context.rs` (menus contextuels).
- **Exceptions explicites** : les 16 IDENTITÉ de l'index. Les principales : les quatre styles de barre ; le verre sur les widgets et le tiroir des Réglages ; le mouvement sur le survol du Dock ; le décor animé de Panorama et de LYRA ; le volume des Réglages ; l'absence de « Quitter » dans le menu du Dock ; les thèmes d'icônes système pour les applications.

## Les écarts à traiter en premier

Classés par preuve, impact, portée et coût. Le détail et les preuves complètes sont dans le document de chaque famille.

| # | Problème | Preuve | Changement proposé | Portée | Confiance |
| --- | --- | --- | --- | --- | --- |
| SYS-03, PAT-02 | Toute notification est acceptée : ni autorisation par application, ni style de remise | `src/notifications.rs:228-258` ; aucun paquet Réglages | Un paquet Notifications : autorisation et style par application | Notifications | Haute |
| MENU-01 | Aucune barre des menus : les commandes du shell sont dispersées | `src/config/model.rs:952-957` ; `src/ui/plugins/bar/window.rs:1-6` | Un menu « hyprdesk », toujours premier, qui rassemble les commandes du shell | Barre | Haute |
| ENT-01, ENT-02 | Ni accès complet au clavier, ni raccourci de focus par surface | `src/ui/plugins/bar/surface.rs:666` ; `src/ui/toasts.rs:414` | Un mode d'accès clavier et un raccourci de focus par zone | Barre, dock, notifications | Haute |
| PAT-01 | Le dock disparaît en plein écran et ne peut pas être rappelé | `src/ui/plugins/dock/surface.rs:356` | Une zone de révélation du dock en plein écran | Dock | Haute |
| SYS-01 | La bulle n'affiche jamais l'icône de l'application | `src/ui/toasts.rs:36` | Dessiner l'icône de l'application émettrice | Notifications | Haute |
| TYP-01 | Le plancher de taille de texte est à 6 px, pour un minimum de 10 pt | `src/ui/tokens.rs:242`, `:263` | Porter le plancher à 10 px | Toute l'interface | Haute |
| FON-01 | La barre et les widgets ne se densifient pas sur un fond clair | `src/config/model.rs:739-753` ; aucun des 7 packs | Densification adaptative par défaut | Barre, widgets | Haute |
| PAT-08, PAT-11, STA-01 | Un lancement ou une commande qui échoue ne laisse qu'une ligne de journal | `src/ui/mod.rs:3588-3596` ; `src/ui/plugins/dock/lifecycle.rs:212-227` | Un retour nommant l'opération et sa cause | Dock, lanceur, recherche | Haute |

## À traiter en premier

**L'icône de l'application dans la bulle de notification (SYS-01).** Toute notification affiche la même cloche générique de 18 px, bien que le serveur relève déjà l'icône envoyée et l'entrée de bureau de l'application. Deux audits indépendants (système et icônes) le constatent, la correction tient en une journée, et elle est visible à chaque notification.

## Décisions à prendre

Sept décisions appartiennent au propriétaire du projet et conditionnent le plan : `plan-d-action.md`, section 4.

## Méthode

1. **Spécification** : 95 pages HIG condensées en un fichier par famille (images, journaux de changement et sections propres aux plateformes mobiles retirés ; titres de section intacts). Treize chapitres rédigés en français, où chaque règle chiffrée porte sa citation anglaise. Les citations ont été vérifiées par script contre les pages d'origine.
2. **Audit** : lecture seule du code, surface par surface, en suivant le chemin de rendu réel. Chaque écart porte la règle HIG citée, les lignes de code qui rendent la surface, et une seule action proposée.
3. **Falsification** : avant de rendre son audit, chaque auditeur a rouvert les fichiers cités et cherché ce qui infirmait ses écarts (option de configuration, autre style de barre, widget du catalogue). Les écarts majeurs ont été rouverts une seconde fois lors de l'assemblage.

### Les cinq natures d'écart

| Nature | Sens |
| --- | --- |
| ÉCART | La HIG emploie un impératif, une cote ou un tableau, et le code ne le respecte pas |
| ABSENT | Composant ou comportement macOS sans équivalent, recherché dans le code, les styles de barre et le catalogue des widgets |
| PISTE | La HIG ne fait que recommander ; à peser, pas à corriger d'office |
| IDENTITÉ | Choix délibéré documenté dans `DESIGN.md` ou un plan de `design-plans/` ; signalé, jamais corrigé |
| À VÉRIFIER | Ne se juge que sur un rendu ; listé en fin d'audit, sans conclusion |

## Hors périmètre

- **L'accessibilité n'a pas été auditée.** La page HIG qui la décrit figure dans `00b`, mais la méthode d'audit l'exclut sauf demande explicite. Les réglages existants (contraste renforcé, mouvement et transparence réduits, échelle de texte) ont néanmoins été relevés dans `00a`.
- **Les bugs de fonctionnement, l'architecture, les performances et la qualité du code** ne sont pas traités.
- **Les zones non ouvertes** par les auditeurs sont signalées en tête de leur audit : `layouts.rs`, `src/windows.rs`, `src/apptheme/`, la forme du lanceur.
- **Le rendu** : les listes « À vérifier sur un rendu » de chaque audit attendent une session réelle.
