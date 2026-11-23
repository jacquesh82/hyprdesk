# Plan d'action — rapprocher hyprdesk du bureau macOS d'après la HIG

Écrit contre : commit `00f6d69`, arbre de travail du 23 novembre 2026 avec huit fichiers modifiés non commités.

> Proposition : rien n'est implémenté. Les identifiants (`SYS-01`, `MENU-02`…) renvoient au tableau d'écarts du document de famille correspondant, où se trouvent la règle HIG, les lignes de code et l'action complète.

## 1. Lecture d'ensemble

L'audit de hyprdesk face à la HIG donne 213 écarts : 66 ÉCART, 42 ABSENT, 89 PISTE et 16 choix d'identité assumés.

**Les fondations sont solides.** Matière, couleur et mouvement : aucun composant absent, six écarts d'un jour chacun. Le moteur de jetons garantit déjà le contraste par rôle, respecte l'apparence sombre du système, réduit transparence et mouvement, et coupe toute animation. Le composant de menu unique arrivé avec `00f6d69` respecte à lui seul une vingtaine de règles.

**Les faiblesses se concentrent sur quatre sujets :**

| Sujet | Constat | Écarts convergents |
| --- | --- | --- |
| Notifications | Icône générique, pas de non-lues, pas d'autorisation, cartes non cliquables, historique à plat | 20, relevés par cinq audits |
| Commandes du shell | Aucune barre des menus ; verrouiller n'est qu'un bouton de 38 px ; veille, redémarrage et extinction n'existent que sur l'écran de connexion | 12 |
| Clavier | La barre et les bulles ignorent le clavier ; aucun raccourci ne donne le focus à une surface | 9 |
| Échecs muets | Un lancement, une copie, une ouverture qui échoue n'écrivent que dans le journal | 12 |

**L'ordre de grandeur.** Si l'on traitait les 197 écarts qui ne sont pas des choix d'identité, en sommant les efforts écart par écart (134 d'un jour au plus, 48 d'un à trois jours, 11 de plus de trois jours), cela représenterait 160 à 390 jours. Ce total n'est pas un objectif : il montre qu'il faut choisir. Les phases ci-dessous traitent 140 écarts, dont l'essentiel des ÉCART et des ABSENT.

## 2. Sept chantiers

### A. Notifications

**Constat.** La surface la plus faible du bureau, et la plus visible au quotidien.

**Que faire, dans l'ordre :**

1. La bulle : icône de l'application émettrice (`SYS-01`), repère non chromatique de l'urgence critique (`FON-02`), urgence basse livrée sans bulle (`PAT-06`).
2. La carte de la barre : application et corps en couleur secondaire (`LAY-03`), titre et corps bornés (`LAY-04`).
3. Le centre : cartes cliquables, action par défaut sinon ouverture de l'application (`SYS-04`), historique groupé par application (`LAY-02`), état « non lue » et compteur sur l'horloge (`SYS-02`).
4. Les réglages : un paquet Notifications — autorisation par application, style de remise selon l'urgence (`SYS-03`, `PAT-02`, `PAT-07`) ; aperçus masquables (`SYS-05`) ; Silence avec une fin (`PAT-04`).

**Effet sur l'écran.** Chaque notification dit qui parle, une notification critique se reconnaît sans couleur, on sait ce qu'on n'a pas lu, et une application bruyante peut être calmée.

**Écarts :** 20 (12 d'un jour au plus, 5 d'un à trois jours, 3 plus). Hors phase : les quatre tailles de widgets et l'ajout de modules par appui long relèvent du chantier G.

### B. Menus du shell

**Constat.** hyprdesk n'a pas de barre des menus. L'absence d'un menu global pour les applications tierces est argumentée dans le code (`src/ui/plugins/bar/window.rs:1-6` : GTK4 et libadwaita ne publient plus leurs menus). Cet argument vaut pour les tierces, pas pour les commandes propres au shell, qui restent dispersées. Il n'est consigné ni dans `DESIGN.md` ni dans `design-plans/`.

**Que faire :**

1. Un menu « hyprdesk », toujours premier et non retirable : Réglages…, Verrouiller, Espaces de travail, Organiser les fenêtres…, mise en veille, redémarrer, éteindre (`MENU-01`).
2. Le menu de la fenêtre et celui des espaces comme menus titrés à sa suite ; un menu « Aide » en dernier (`MENU-05`).
3. Les commandes aujourd'hui accessibles seulement par clic droit y sont recensées (`MENU-04`).
4. Le composant de menu gagne un sous-menu d'un niveau (`MENU-03`) et la rangée « Organiser les fenêtres… » prend le composant commun (`MENU-19`).

**Effet sur l'écran.** Toute commande du shell se trouve à un endroit prévisible, et s'atteint au clavier.

**Écarts :** 12 (8, 3, 1). Dépend de la décision D1.

### C. Clavier et accès

**Constat.** La barre et les bulles déclarent `KeyboardInteractivity::None`. Tab ne circule qu'entre les champs de saisie des Réglages. macOS réserve Control-F2 à F6 au focus de chaque zone.

**Que faire :**

1. Un raccourci de focus par zone : barre, dock, notifications, panneau (`ENT-02`), puis la navigation dans chacune (`ENT-01`).
2. Les gestes système : vue d'ensemble, liste des applications, notifications, bureau (`ENT-03`).
3. « Réduire la fenêtre active » au clavier (`ENT-08`), « Forcer à quitter » (`ENT-10`), Ctrl+Tab entre les groupes du centre de contrôle (`ENT-11`).

**Écarts :** 9 (5, 2, 2), dont `ENT-05` et `PRES-01` dès la phase 0. L'ordre des modificatrices (`ENT-04`) est rattaché au chantier E.

### D. Réglages robustes

**Constat.** La structure est juste : au plus six onglets, sélection persistante, résultats groupés par paquet, avancé masqué par défaut. Le texte et la validation pèchent.

**Que faire :**

1. La validation : champ numérique qui refuse une valeur non numérique (`SEL-01`, `DAT-09`), analyse à chaque frappe (`DAT-02`), valeur hexadécimale gardée et expliquée (`SEL-07`).
2. La lisibilité : sections avancées repliables au lieu d'un cadre permanent (`LAY-01`), textes utiles sélectionnables (`LAY-05`), une seule couleur secondaire (`LAY-08`), libellés qui reviennent à la ligne (`TYP-08`).
3. Les contrôles : puits de couleur sur la couleur en cours (`SEL-02`), valeur des curseurs saisissable (`SEL-08`), commutateur d'avancé sorti de l'en-tête (`SEL-03`).

**Écarts :** 28 (19, 9). Hors phase : cacher la colonne de navigation (`NAV-09`, `LAY-14`) et sa taille réglable (`NAV-10`).

### E. Repères cohérents

**Constat.** Deux familles d'icônes se côtoient dans les mêmes menus, la fermeture est un caractère de police en onze endroits, trois mots disent « arrêté », et 23 corps de texte coexistent quand la HIG en propose onze.

**Que faire :**

1. Les icônes : un seul jeu de glyphes pour les commandes (`ICO-02`, `ICO-06`, `ICO-07`, `ICO-08`, `ICO-09`), icônes d'application cherchées à la taille d'affichage et à l'échelle de l'écran (`ICO-01`), taille des glyphes liée au texte voisin (`ICO-13`).
2. Les couleurs : densification adaptative par défaut (`FON-01`), encres remises dans la palette (`FON-03`, `FON-05`, `FON-06`), état marqué sans couleur dans le centre de contrôle (`SEL-04`).
3. Le mouvement réduit : il coupe aussi le grossissement du Dock, les fondés du tiroir et les animations de fenêtres (`FON-07`, `FON-08`).
4. L'écrit : une table des termes d'état et une casse uniques (`TYP-06`, `TYP-14`), une échelle de corps ramenée au tableau de la HIG (`TYP-04`), semibold au lieu de Bold sur les titres (`TYP-05`).

**Écarts :** 38 (33, 5), `ENT-04`, `ICO-02` et `TYP-18` rattachés.

### F. Échecs, annulation et accompagnement

**Constat.** Quatre commandes écrivent dans le journal sans rien dire. Aucune action du shell ne s'annule. L'assistant de premier démarrage s'ouvre seul en dix étapes sans sortie.

**Que faire :**

1. Un retour pour chaque échec : lancement, copie, ouverture de fichier, action de notification, « + Nouvel espace » à la limite (`PAT-08`, `PAT-11`, `PAT-10`, `STA-01`, `TYP-13`).
2. Des messages qui disent quoi faire : « Nom invalide. », l'état sans résultat du lanceur, l'alerte problème (`TYP-03`, `TYP-02`, `PRES-05`).
3. L'annulation d'un déplacement de fenêtre (`PAT-12`, `DAT-12`).
4. Un assistant quittable à toute étape, sa personnalisation reportée (`DAT-01`).

**Écarts :** 18 (16, 2).

### G. Bureau et interactions

**Constat.** Le dock disparaît en plein écran. Le shell n'accepte aucun dépôt. Les widgets ne s'ouvrent pas au clic et n'ont qu'une taille.

**Que faire :**

1. Le dock atteignable en plein écran par une zone de révélation (`PAT-01`), la barre révélable de même (`PAT-03`).
2. Le dock et l'aperçu comme cibles de dépôt, la cible marquée seulement quand le dépôt a un effet (`DAT-04`, `DAT-08`).
3. Les widgets : clic sur le fond, tailles qui ajoutent de l'information (`SYS-10`, `SYS-15`), marge basse mesurée d'après le dock (`TYP-17`).

**Écarts :** 18 (6, 9, 3).

## 3. Cinq phases

| Phase | Contenu | Écarts nouveaux | Effort |
| --- | --- | --- | --- |
| 0 | Corrections rapides, sans décision de design | 26 | 3 à 5 semaines |
| 1 | Notifications | 15 | 4 à 10 semaines |
| 2 | Menus du shell et clavier | 18 | 4 à 11 semaines |
| 3 | Réglages | 20 | 3 à 7 semaines |
| 4 | Repères cohérents | 30 | 4 à 8 semaines |
| 5 | Bureau et interactions, puis accompagnement | 31 | 6 à 16 semaines |

Les efforts sont des ordres de grandeur, sommés écart par écart : un demi-jour à un jour, de un à trois jours, plus de trois jours, à raison de cinq jours par semaine. Traiter plusieurs écarts d'une même surface en une fois coûte moins que leur somme. Les écarts déjà réglés en phase 0 ne sont comptés qu'une fois.

### Phase 0 — Corrections rapides

Vingt-six corrections d'un jour au plus, toutes de confiance haute, qui ne changent aucune décision de design.

| Sujet | Écarts |
| --- | --- |
| Notifications | `SYS-01`, `SYS-04`, `FON-02`, `LAY-03`, `PAT-06` |
| Menus | `MENU-02` (menus contextuels défilants et bornés) |
| Réglages et saisie | `SEL-01`, `NAV-01`, `NAV-02`, `NAV-03`, `PAT-09` |
| Clavier | `ENT-04`, `ENT-05`, `PRES-01` |
| Échecs muets | `PAT-08`, `PAT-11`, `STA-01`, `TYP-03` |
| Lisibilité | `TYP-01`, `TYP-07`, `FON-01`, `FON-03` |
| Icônes | `ICO-02`, `ICO-07`, `ICO-09` |
| Aperçu des espaces | `DAT-08` |

`PAT-08`, `PAT-11` et `STA-01` relèvent du même changement : un seul chemin de retour d'échec, partagé par le dock, la grille, la recherche et les notifications.

### Phase 1 — Notifications

Le chantier A dans son ordre ; cinq de ses vingt écarts sont déjà traités en phase 0. Le paquet de réglages dépend de la décision D5.

### Phase 2 — Menus du shell et clavier

Le chantier B, puis le chantier C. Dépend des décisions D1 et D2.

### Phase 3 — Réglages

Le chantier D. `SEL-03` attend la décision D3.

### Phase 4 — Repères cohérents

Le chantier E. `ICO-04` et `ICO-05` attendent la décision D4.

### Phase 5 — Bureau et interactions

Le chantier G, puis l'accompagnement du chantier F.

## 4. Décisions à prendre

Ces sept décisions changent l'identité ou les habitudes du bureau. Elles appartiennent au propriétaire du projet ; aucune n'est prise par ce plan.

| # | Décision | Ce qui en dépend |
| --- | --- | --- |
| D1 | **Un menu « hyprdesk » en tête de la barre.** Il ajoute un élément à la zone amont du style Continuité et change la silhouette de la barre. Le constat d'absence doit aussi être consigné dans `DESIGN.md`. | `MENU-01`, `MENU-04`, `MENU-05` |
| D2 | **Les raccourcis par défaut.** SUPER+Espace pour le lanceur et SUPER+Tab pour le commutateur suivent macOS ; aujourd'hui SUPER+R ouvre le lanceur et SUPER+Tab l'aperçu. Changer un défaut déplace les habitudes des utilisateurs actuels. | `ENT-06`, `ENT-09`, `PAT-14` |
| D3 | **Le commutateur « Configuration avancée ».** La HIG réserve les commutateurs au corps de la fenêtre. Le déplacer change l'en-tête du tiroir. | `SEL-03` |
| D4 | **Un masque unique pour les icônes d'application.** Il donnerait aux icônes la courbure des cartes de l'interface, au prix de la forme du thème d'icônes installé — aujourd'hui un choix d'identité. SmartUI devrait alors générer un carré plein cadre. | `ICO-04`, `ICO-05`, `ICO-16` |
| D5 | **Un paquet de réglages Notifications.** Autorisation par application et style de remise : trois jours au moins, et une nouvelle page aux Réglages. | `SYS-03`, `PAT-07` |
| D6 | **L'assistant de premier démarrage.** Le rendre quittable à toute étape et reporter sa personnalisation va contre une première prise en main guidée. | `DAT-01`, `DAT-07`, `DAT-11` |
| D7 | **Auditer l'accessibilité.** La page HIG qui la décrit est spécifiée mais son application dans le code n'a pas été vérifiée. | Nouvel audit |

Une adaptation est acquise et ne demande pas de décision : **Super tient le rôle de Command** sur Hyprland.

## 5. Ce que ce plan ne traite pas

- **Les 89 PISTE non citées ci-dessus.** La HIG ne fait que les recommander ; elles restent documentées dans les tableaux de chaque famille.
- **Les 16 choix d'identité.** Signalés, jamais corrigés.
- **L'accessibilité** (décision D7).
- **Les zones non ouvertes** par les auditeurs : `layouts.rs`, `src/windows.rs`, `src/apptheme/`, la forme du lanceur.
- **Le rendu.** Chaque audit se termine par une liste « À vérifier sur un rendu » : lisibilité du pack « Verre » sur un papier peint clair, bulles au-dessus du verrouillage, rangées de résultats à texte long, cibles du centre de contrôle au trackpad. Ces points demandent une session réelle.
- **Ce que la HIG ne spécifie pas** : la taille du Dock, les résultats de Spotlight, le comportement de Stage Manager et de Mission Control.

## 6. Passer à l'exécution

Chaque écart cité dans ce plan devient un plan d'implémentation autonome, rédigé pour un agent qui n'aura pas lu ce dossier : chaîne de preuves, décision, réutilisation des composants existants, changements fichier par fichier, validation. L'ordre conseillé :

1. La phase 0 en entier : elle ne demande aucune des sept décisions.
2. Le chantier A, dès que D5 est tranchée.
3. Le chantier B, dès que D1 et D2 sont tranchées.
