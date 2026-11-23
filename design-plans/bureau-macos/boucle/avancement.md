# Registre d'avancement de la boucle HIG

> Une ligne par écart, dans l'ordre de traitement. Trois couloirs (A, B, C) le traitent en parallèle ; la consigne qui fait foi est `.zcode/hig-boucle/CONSIGNE.md` du dépôt principal, sa section 7 dit comment publier.
> Les identifiants renvoient au tableau d'écarts du document de famille (`../<famille>.md`).
> Statuts : `à faire`, `fait`, `à reprendre`, `bloqué`, `sans objet`, `hors plan`, `identité`.

## Journal de la boucle

### Avant les couloirs

| Date | Itération | Résultat |
| --- | --- | --- |
| 2026-11-23 | mise en place | worktree créé, mise en forme rustfmt (edbb517), registre initial de 213 écarts ; décisions D1 à D7 approuvées |
| 2026-11-23 | 1 | AUDIT-A11Y, SYS-01 traités ; il en reste 152 |
| 2026-11-23 | 2 | SYS-04, FON-02, LAY-03 traités ; il en reste 149 |

### Couloir A

| Date | Itération | Résultat |
| --- | --- | --- |
| 2026-11-23 | A1 | PAT-06, PAT-08, PAT-11, STA-01 traités ; il en reste 45 dans ce couloir |
| 2026-11-24 | A2 | FON-01, FON-03 traités ; il en reste 43 dans ce couloir |

### Couloir B

| Date | Itération | Résultat |
| --- | --- | --- |
| 2026-11-23 | B1 | SEL-01, DAT-09, NAV-01, NAV-02, NAV-03 traités ; il en reste 52 dans ce couloir |

### Couloir C

| Date | Itération | Résultat |
| --- | --- | --- |
| 2026-11-24 | C1 | MENU-02, ENT-05, DAT-08 traités (travail de l'itération interrompue par la panne de quota, repris et mis au point) ; il en reste 40 dans ce couloir |
| 2026-11-24 | C2 | ENT-04 traité ; il en reste 39 dans ce couloir |

Référence verte : `b7c6f1f` (code vérifié par `scripts/verify.sh --visual` le 23 novembre 2026 à 23 h 17 : code de sortie 0 ; précédente référence : `4d5193d`).

## Préalable — audit accessibilité (décision D7)

| Écart | Objet | Nature | Effort | Confiance | Statut | Commit | Remarque |
| --- | --- | --- | --- | --- | --- | --- | --- |
| AUDIT-A11Y | Audit du code face à la page HIG « Accessibility » (`../00b-fondations-structure.md`, première partie), écrit dans `../audit-accessibilite.md` ; ses écarts `ACC-xx` entrent ensuite au registre, en fin de phase 1 | audit | M | — | fait | c9ad686 | `../audit-accessibilite.md` écrit à `be6cf05` : 10 écarts prouvés (1 ABSENT, 5 ÉCART, 4 PISTE), classés par importance, ajoutés en fin de phase 1 ; convention des cotes : 1 pt = 1 px logique ; ACC-01 (lecteur d'écran) est coté L et se traite sans dépendance nouvelle (`zbus` déjà directe) |

## Phase 0 — Corrections rapides

| Écart | Objet | Nature | Effort | Confiance | Statut | Commit | Remarque |
| --- | --- | --- | --- | --- | --- | --- | --- |
| SYS-01 | Icône de l'app dans la bulle et le centre | ABSENT | S | haute | fait | 4d5193d | échec 1 (bed7073), puis réussite : `notif_icon` (`src/ui/toasts.rs`) dessine l'icône de l'application, demandée à l'arrivée de la notification, même sous Silence ; 36 px au bord gauche de la bulle, 28 px à gauche des cartes du centre de la barre et du centre de contrôle Continuité ; la cloche reste le repli, dans la même place. Test `l_icone_de_l_application_est_demandee_meme_sous_silence`, captures `notifications` à 1440 et 640 regardées. Aucun propriétaire partagé modifié. Documentation : rien à mettre à jour, la bulle n'y est pas décrite. |
| SYS-04 | Clic sur une notification du centre | ABSENT | S | haute | fait | b7c6f1f | échec 1 (082561b : clippy), puis réussite : les cartes du centre sont cliquables (`card_press` puis `TMsg::Card`, `src/ui/toasts.rs`). Dans le centre de contrôle Continuité (`bar/popover/continuity.rs`), une tuile comme les autres, au clavier aussi ; dans la carte Notifications des autres styles de barre, une zone de clic au pointeur. Un clic demande l'action `default` si l'application en propose une, sinon ouvre son application (hint `desktop-entry`, suffixe `.desktop` toléré) par le lanceur (`Msg::LauncherLaunch`), et seulement si son entrée `.desktop` est installée : le hint vient du bus, il ne sert jamais de commande, un chemin est refusé. Le centre se referme sur le clic. Décision : sans action `default` ni hint `desktop-entry`, l'application ne peut pas être identifiée et la carte reste non cliquable. Preuve à HEAD décalée par SYS-01 (`toasts.rs:743-790`, `continuity.rs:1095-1134`). Tests `un_clic_sur_une_carte_du_centre_sait_ou_mener`, `la_carte_du_centre_lance_l_action_par_defaut`. Capture `bar-controls` regardée (cartes sans action : aspect inchangé). Documentation : `docs/GUIDE.fr.md` et `docs/GUIDE.md`, ligne du widget `notifications`. Aucun propriétaire partagé modifié. |
| FON-02 | Notification critique | ÉCART | S | haute | fait | b7c6f1f | échec 1 (082561b : clippy), puis réussite : la bulle critique porte le losange de l'urgence (`hyprdesk-urgent`, déjà celui de la barre), 10 px avant le nom de l'application, demandé à l'arrivée de la notification (`toasts.rs:627`). Décision : le filet et le losange prennent le ton « attention » (`tone::warning()`) au lieu de l'accent, qui reste réservé à la sélection et aux actions (seconde règle citée par l'écart) ; la colonne Correction ne demandait que le doublement. Dans le style Convex, qui ne dessine pas de filet, le losange est le seul repère. Preuve à HEAD décalée par SYS-01 (`toasts.rs:618`). Test `seule_une_notification_critique_demande_son_repere`. Captures `notifications` à 1440 et 640 regardées. Documentation : rien à mettre à jour, la bulle n'y est pas décrite. Aucun propriétaire partagé modifié. |
| LAY-03 | Labels — carte Notifications de la barre | ÉCART | S | haute | fait | b7c6f1f | échec 1 (082561b : clippy), puis réussite : dans la carte Notifications de la barre (`view_notification_history`, `src/ui/toasts.rs`), l'application et le corps passent en couleur secondaire, le titre garde celle du texte ; même opacité 0,65 que la bulle (`DIM`), appliquée à l'alpha de la carte. Le centre de contrôle Continuité utilisait déjà `pal.muted` : non touché. Preuve à HEAD décalée par SYS-01 (`toasts.rs:743-770`). Aucune scène d'aperçu n'ouvre cette carte : vérifiée par relecture, pas par capture. Documentation : rien à mettre à jour. Aucun propriétaire partagé modifié. |
| PAT-06 | Urgence basse | ÉCART | S | haute | fait | 0dad66c | une notification de basse urgence ne montre pas de bulle : elle va à l'historique seul (`src/ui/toasts.rs`), le chemin déjà pris sous Silence ; test `une_notification_basse_va_a_l_historique_sans_bulle`. Travail commencé par l'itération interrompue par une panne de quota, repris et mis au point. |
| MENU-02 | Menus contextuels (bureau, icônes, widgets, dock, grille, édition) | ÉCART | S | haute | fait | fee0adc | Travail interrompu par la panne de quota, repris et mis au point. `spawn_menu` plafonne la surface du clic droit à la hauteur de l'écran (`IconMenu::cap`, `src/ui/context.rs`) : la carte garde son contour et son ombre, ses lignes défilent dans un `scrollable` (`hyprdesk-icon-menu-list`), le renommage reste en bas et, sur la page Propriétés, le titre et les commandes restent ; le clavier garde l'entrée choisie visible (`follow` puis `scroll_to`, l'idiome du dock et du centre de contrôle). Une carte qui tient ne change pas. Plafond à la hauteur de l'écran, comme le bornage déjà en place du clic droit ; les menus de la barre retirent en plus la hauteur de la barre, choix non repris ici. Preuve à HEAD juste. Tests `carte_plafonnee_a_l_ecran`, `le_defilement_suite_l_entre_choisie` ; scène d'aperçu nouvelle `menu-dock-long` (soixante fenêtres), regardée à 1440. |
| SEL-01 | Champ numérique des listes de règles Hyprland | ÉCART | S | haute | fait | 0824a6e | un champ `Field::Num` refuse tout texte qui n'est pas un nombre fini (`inf` et `nan` compris, qui posaient `null`) : la saisie reste au champ, l'erreur (« abc » : nombre attendu) s'affiche sous le champ comme pour les options de Hyprland, l'élément et Hyprland ne changent pas. Même changement que DAT-09. Travail commencé par l'itération interrompue par une panne de quota, repris et mis au point : l'erreur, à droite du champ, passe dessous. Tests `champ_numerique_refuse_le_texte` et `cle_d_un_champ_d_element`. |
| NAV-01 | Champ de recherche du lanceur | ÉCART | S | haute | fait | cb15560 | les quatre modes (applications, `:f`, `:clip`, `:2fa` et `:mdp`) passent par `launcher_field` (`src/ui/plugins/launcher/grid.rs`) : loupe à gauche, bouton Clear à droite dès que la requête n'est pas vide, avec sa bulle « Effacer la recherche ». Un clic vide la requête, préfixe de mode compris (`:clip` seul s'efface aussi et le lanceur revient aux applications), et rend le focus au champ (`SMsg::Clear`). Travail commencé par l'itération interrompue par une panne de quota, repris et mis au point : la couche du bouton occupe tout le champ, sans quoi l'empilement le posait à gauche, sur la loupe. Couvre NAV-03 (même changement). |
| NAV-02 | Champ de recherche des Réglages | ÉCART | S | haute | fait | bf7711f | bouton Clear dans le champ de recherche de l'en-tête du tiroir (`src/ui/panel.rs`), affiché dès que la requête n'est pas vide, avec sa bulle « Effacer la recherche » qui s'ouvre dessous ; un clic envoie `PanelMsg::SearchClear` et rend le focus au champ (`SEARCH_ID`). Le bouton texte « Effacer » du bandeau des résultats garde son message : il rend le focus aussi. Captures à 1440 et 640 px (requête posée dans l'aperçu, puis retirée). Propriétaire partagé `panel.rs` : relu à froid. Pas de test : code de vue, aucun constructeur d'`App` de test au dépôt. |
| NAV-03 | Icône Search | ÉCART | S | haute | fait | cb15560 | couvert par NAV-01 (même changement) : les quatre modes du lanceur passent par `launcher_field`, la loupe partout. |
| PAT-09 | Dernier volet des Réglages | ÉCART | S | haute | à faire | — | — |
| ENT-04 | Ordre des modificatrices affichées | ÉCART | S | haute | fait | 1a09503 | Une seule table, `hypr::MODIFIER_ORDER` (CTRL, ALT, SHIFT, SUPER), et un rang partagé `modifier_rank` (synonymes de `modifier_bit` : CONTROL, WIN, LOGO, META, MOD1, MOD4). `essentials::pretty` ordonne les modificateurs de toute combinaison affichée, chacun gardant le nom écrit ; la capture des Réglages (`combo_of`, `keys.rs`), `modmask_name` et `universal_binds` construisent dans cet ordre. La configuration, ses valeurs par défaut (`SUPER + ALT + D`) et l'aide de `hyprdesk config` gardent leur écriture : l'analyse (`parse_combo`, `same_text`) ne dépend jamais de l'ordre. L'exemple des trois messages « raccourci illisible » (`keys.rs`, `launcher.rs`, `cli_config.rs`, `locales/en.toml`) passe au nouvel ordre ; une phrase dans les deux guides et dans `DESIGN.md`. Tests : attentes de `capture`, `combinaisons_lisibles` (plus deux cas d'ordre quelconque), `essentiels_face_a_hyprland_lua`, `workspaces_bureau_affiche`. Preuve à HEAD juste. |
| ENT-05 | MAJ+Tab dans les menus et la vue d'ensemble | ÉCART | S | haute | fait | fee0adc | `icon_menu_key` et `overview_key` reçoivent l'état de Maj (`self.mods.shift()`) : Tab avance, Maj+Tab recule comme la flèche haut, dans les menus du clic droit comme dans l'aperçu des espaces. Même changement que MENU-02 pour les menus (mêmes fonctions). Tests `maj_tab_recule_et_la_carte_suite`, `tab_et_maj_tab_changent_de_rang`. |
| PRES-01 | Alertes problème et polkit | ÉCART | S | haute | à faire | — | — |
| PAT-08 | Échec d'un lancement | ÉCART | S | haute | fait | 0dad66c | une entrée sans commande (`LaunchEnd::Unresolved`) ou une commande refusée par Hyprland (`LaunchEnd::Refused` : la cause remonte de `run_from_desktop_on`, `src/ui/mod.rs`) affiche une notification de Hyprland qui nomme l'application et la cause (`launcher_launch_ended`, `src/ui/plugins/dock/lifecycle.rs`), comme le renommage d'espace. Même lot que STA-01. |
| PAT-11 | Échecs sans message | ÉCART | S | haute | fait | c929ed3 | trois chemins ne se contentent plus du journal : copie d'un code ou d'un identifiant myMFA et ouverture d'un fichier depuis la recherche (`src/ui/search.rs`), action ou fermeture d'une notification que son application n'a pas reçue (`src/ui/toasts.rs`) ; chacun affiche une notification de Hyprland avec la cause, comme le renommage d'espace. |
| STA-01 | Indicateur de lancement du dock : fin sans retour | ÉCART | S | haute | fait | 0dad66c | au délai de 5 s sans qu'aucune fenêtre arrive, la fin du lancement est annoncée par une notification discrète (indice, 6 s) qui propose de relancer l'application ; une fenêtre d'une autre classe arrivée entre-temps n'est pas un échec, rien n'est annoncé. Tests `une_fin_de_lancement_est_annoncee_une_fois_et_jamais_si_une_fenetre_est_arrivee` et `une_fin_de_lancement_est_annoncee_par_le_nom_de_l_application_et_sa_cause`. Même lot que PAT-08. |
| TYP-03 | Écriture — renommage d'une icône du bureau | ÉCART | S | haute | à faire | — | — |
| TYP-01 | Typographie — plancher de taille de texte | ÉCART | S | haute | à faire | — | — |
| TYP-07 | Typographie — corps sous le minimum | ÉCART | S | haute | à faire | — | — |
| FON-01 | Barre et widgets du bureau : lisibilité sur fond clair | ÉCART | S | haute (mécanisme) ; ratios estimés | fait | a2b2895 | échec 1 (test du défaut `transparence_adaptative`), puis réussite : `Place::default_effect` active la densification de `bar` et `widgets`, plafond 0,95 (celui du lanceur ; le préréglage `glass_regular` du code est à 0,94, la preuve annonçait 0,95). Au-delà de la colonne Correction, requis par le changement : `Preset::effects` suit la même règle ; la rédaction des réglages écrit `adaptive = false` quand le plafond est coupé là où le défaut en a un, sans quoi « off » n'était plus conservé au rechargement. Les packs livrés restent tels quels : une surface qui n'en dit rien hérite du défaut (`apply_effect`). Tests `barre_et_widgets_se_densifient_sur_le_fond_par_defaut` et `barre_et_widgets_des_packs_livrés_se_densifient`. Porte visuelle passée ; les scènes sont sur fond sombre, la densification n'y est pas visible. |
| FON-03 | Texte secondaire dessiné à alpha fixe | ÉCART | S | haute | fait | a2b2895 | les trois textes secondaires à alpha fixe prennent `ink().dim` : `fx_origin` et `fx_effects` (`src/ui/plugins/animations.rs`), les libellés des nuances du sélecteur de couleurs (`src/ui/color_picker.rs`, qui reçoit `dim` de la page Couleurs de la barre). Capture `animations-effets` regardée : texte lisible. |
| ICO-02 | Commandes média et volume, bouton Réglages du dock | ÉCART | S | haute | à faire | — | — |
| ICO-07 | Fermeture, lecture et pause | ÉCART | S | haute | à faire | — | — |
| ICO-09 | Icône de remplacement (application sans icône) | ÉCART | S | haute | à faire | — | — |
| DAT-08 | Aperçu des espaces : cible de dépôt | ÉCART | S | haute | fait | fee0adc | `is_drop_target` (`src/ui/overview.rs`) : la bordure d'accent et l'ombre ne marquent que les cartes autres que celle d'origine de la fenêtre glissée ; y déposer ne fait toujours rien. Test `la_carte_d_origine_n_est_pas_une_cible`. |

## Phase 1 — Notifications

| Écart | Objet | Nature | Effort | Confiance | Statut | Commit | Remarque |
| --- | --- | --- | --- | --- | --- | --- | --- |
| LAY-04 | Lists and tables — corps des notifications | ÉCART | S | moyenne | à faire | — | — |
| SYS-14 | Barre Island : icône de l'app | ÉCART | S | haute | à faire | — | — |
| ICO-03 | Bulle de notification | ABSENT | S | haute | à faire | — | bulle faite avec SYS-01 (4d5193d) ; reste le repli sur l'icône de la classe quand `n.icon` est vide |
| PAT-04 | Durée du Silence | ABSENT | S | haute | à faire | — | — |
| SYS-05 | Aperçus masquables | ABSENT | S | haute | à faire | — | — |
| SYS-17 | Actions destructrices | ABSENT | S | moyenne | à faire | — | — |
| SYS-18 | Icônes des actions | ABSENT | S | moyenne | à faire | — | — |
| LAY-02 | Lists and tables — historique | ABSENT | M | haute | à faire | — | — |
| PAT-05 | Niveaux d'interruption | ABSENT | M | haute | à faire | — | — |
| SYS-02 | Compteur de non-lues | ABSENT | M | haute | à faire | — | — |
| SYS-13 | Barre Island : activité en cours | ABSENT | M | haute | à faire | — | — |
| PAT-02 | Autorisation des applications | ABSENT | L | haute | à faire | — | — |
| PAT-07 | Réglages des notifications | ABSENT | L | haute | à faire | — | — |
| SYS-03 | Réglages des notifications | ABSENT | L | haute | à faire | — | — |
| PAT-20 | Résumé programmé | PISTE | M | moyenne | à faire | — | — |
| ACC-01 | Lecteur d'écran : arbre d'accessibilité (AT-SPI) | ABSENT | L | haute | à faire | — | traiter en dernier ; pont au-dessus de `zbus`, déjà dépendance directe |
| ACC-02 | Verrouillage : contraste sur l'image de fond | ÉCART | S | haute (mécanisme) ; ratios calculés | à faire | — | — |
| ACC-03 | Verrouillage : texte estompé au repos | ÉCART | S | moyenne | à faire | — | — |
| ACC-04 | Bulles de notification : fermeture et actions sous 20 px | ÉCART | S | haute | à faire | — | — |
| ACC-05 | Verrouillage : dérive continue malgré « Mouvement réduit » | ÉCART | S | haute | à faire | — | — |
| ACC-06 | Verrouillage et écran de connexion : texte hors de l'échelle | ÉCART | S | haute | à faire | — | — |
| ACC-07 | Notifications : délai demandé par l'application, sans plancher | PISTE | S | haute | à faire | — | recouvre SYS-03 et PAT-07 |
| ACC-08 | Verrouillage : lecture et pause à la souris seulement | PISTE | S | moyenne | à faire | — | — |
| ACC-09 | Barre : plancher de cible de 16 px | PISTE | S | haute | à faire | — | — |
| ACC-10 | Boutons sans bordure trop rapprochés | PISTE | S | moyenne | à faire | — | — |

## Phase 2 — Menus du shell et clavier

| Écart | Objet | Nature | Effort | Confiance | Statut | Commit | Remarque |
| --- | --- | --- | --- | --- | --- | --- | --- |
| MENU-04 | Commandes accessibles seulement par clic droit | ÉCART | S | haute | à faire | — | — |
| MENU-15 | Icônes des actions `.desktop` | ÉCART | S | moyenne | à faire | — | — |
| MENU-03 | Sous-menus d'un niveau | ABSENT | M | haute | à faire | — | — |
| MENU-05 | Menu Aide | ABSENT | M | haute | à faire | — | — |
| MENU-01 | Barre des menus : inventaire des commandes du shell | ABSENT | L | haute | à faire | — | — |
| MENU-06 | Menus contextuels d'une surface à l'autre | PISTE | S | moyenne | à faire | — | — |
| MENU-07 | Titre de la fenêtre active dans la barre | PISTE | S | haute | à faire | — | — |
| MENU-09 | Ordre des entrées du menu du dock | PISTE | S | moyenne | à faire | — | — |
| MENU-12 | Menu contextuel d'une icône du bureau | PISTE | S | haute | à faire | — | — |
| MENU-19 | Rangée « Organiser les fenêtres… » | PISTE | S | haute | à faire | — | — |
| ENT-07 | Menus contextuels (modules de barre, notifications, espaces de l'aperçu) | PISTE | M | haute | à faire | — | — |
| ENT-08 | Réduire la fenêtre active au clavier | ABSENT | S | haute | à faire | — | — |
| ENT-11 | Ctrl+Tab entre les groupes du centre de contrôle | ABSENT | S | haute | à faire | — | — |
| ENT-02 | Focus clavier par surface (barre, dock, notifications) | ABSENT | M | haute | à faire | — | — |
| ENT-10 | Forcer une application à quitter | ABSENT | M | haute | à faire | — | — |
| ENT-01 | Accès complet au clavier | ABSENT | L | haute | à faire | — | — |
| ENT-03 | Gestes système (vue d'ensemble, liste des apps, notifications, bureau) | ABSENT | L | haute | à faire | — | — |
| ENT-06 | SUPER+Tab | PISTE | S | haute | à faire | — | — |
| ENT-09 | Raccourci du lanceur | PISTE | S | haute | à faire | — | — |
| ENT-14 | Focus de la vue d'ensemble quand la tuile disparaît | PISTE | S | moyenne | à faire | — | — |
| PAT-14 | Raccourci des Réglages | PISTE | S | haute | à faire | — | — |

## Phase 3 — Réglages

| Écart | Objet | Nature | Effort | Confiance | Statut | Commit | Remarque |
| --- | --- | --- | --- | --- | --- | --- | --- |
| DAT-02 | Champs de couleur et de texte des réglages | ÉCART | S | haute | à faire | — | — |
| DAT-09 | Champs numériques des listes de règles | ÉCART | S | moyenne | fait | 0824a6e | même changement que SEL-01 : un champ `Field::Num` n'accepte qu'un nombre fini, la saisie refusée reste au champ et l'erreur s'affiche dessous. |
| LAY-05 | Labels — textes utiles des réglages | ÉCART | S | haute | à faire | — | — |
| PRES-04 | Tiroir des Réglages : barres de défilement | ÉCART | S | moyenne | à faire | — | — |
| SEL-03 | Commutateur « Configuration avancée » | ÉCART | S | moyenne | à faire | — | — |
| SEL-05 | Segments sans info-bulle | ÉCART | S | haute | à faire | — | — |
| SEL-07 | Champ hexadécimal du sélecteur de couleur | ÉCART | S | haute | à faire | — | — |
| ICO-06 | Menus (bureau, fenêtre, widget) et Panorama | ÉCART | M | haute | à faire | — | — |
| LAY-01 | Disclosure controls | ABSENT | M | haute | à faire | — | — |
| SEL-02 | Puits de couleur | ABSENT | M | haute | à faire | — | — |
| LAY-07 | Split views — onglet courant | PISTE | S | moyenne | à faire | — | — |
| LAY-08 | Labels — texte secondaire | PISTE | S | haute | à faire | — | — |
| LAY-12 | Labels — boutons à glyphe seul | PISTE | S | haute | à faire | — | — |
| NAV-07 | Recherche des Réglages : colonne de navigation | PISTE | S | moyenne | à faire | — | — |
| PAT-16 | Titre du tiroir | PISTE | S | haute | à faire | — | — |
| SEL-14 | Listes déroulantes sans valeur posée | PISTE | S | moyenne | à faire | — | — |
| PAT-13 | Point d'entrée de la recherche | PISTE | M | haute | à faire | — | — |
| SEL-08 | Valeur des curseurs en lecture seule | PISTE | M | haute | à faire | — | — |
| SEL-09 | Graduations des curseurs à pas discret | PISTE | M | moyenne | à faire | — | — |
| TYP-08 | Disposition — texte agrandi dans les réglages | PISTE | M | moyenne | à faire | — | — |

## Phase 4 — Repères cohérents

| Écart | Objet | Nature | Effort | Confiance | Statut | Commit | Remarque |
| --- | --- | --- | --- | --- | --- | --- | --- |
| FON-04 | Halo du verre qui suit le pointeur | ÉCART | S | moyenne (la hig range cet effet dans le mouvement sans le nommer animation) | à faire | — | — |
| FON-05 | Accent employé comme couleur de texte | ÉCART | S | haute | à faire | — | — |
| FON-06 | Encre d'erreur et d'avertissement | ÉCART | S | haute | à faire | — | — |
| ICO-05 | SmartUI design | ÉCART | S | haute | à faire | — | — |
| ICO-08 | Variantes « indisponible » | ÉCART | S | haute | à faire | — | — |
| SEL-04 | Pastilles Wi-Fi, Bluetooth, Silence du centre de contrôle | ÉCART | S | moyenne | à faire | — | — |
| SEL-06 | Libellés des segments « Affichage du bureau » | ÉCART | S | haute | à faire | — | — |
| SYS-06 | Centre de contrôle : bascule | ÉCART | S | haute | à faire | — | — |
| SYS-07 | Centre de contrôle : action en cours | ÉCART | S | haute | à faire | — | — |
| SYS-08 | Centre de contrôle : symboles du Wi-Fi | ÉCART | S | haute | à faire | — | — |
| SYS-09 | Galerie de widgets : descriptions | ÉCART | S | haute | à faire | — | — |
| SYS-16 | Widget Système : alerte | ÉCART | S | moyenne | à faire | — | — |
| TYP-06 | Écriture — casse des états | ÉCART | S | haute | à faire | — | — |
| ICO-01 | Icônes d'application (lanceur, dock, aperçu, commutateur, sélecteur d'activité, menu de fenêtre) | ÉCART | M | haute | à faire | — | — |
| TYP-19 | Typographie — rôle infobulle | ABSENT | S | moyenne | à faire | — | — |
| ICO-04 | Icônes d'application (dock, lanceur, aperçu) | ABSENT | M | moyenne | à faire | — | — |
| FON-07 | Couverture de « Mouvement réduit » | PISTE | S | haute | à faire | — | — |
| FON-08 | Deux interrupteurs de réduction du mouvement | PISTE | S | haute | à faire | — | — |
| ICO-13 | Taille des glyphes hors barre | PISTE | S | haute | à faire | — | — |
| SEL-13 | Largeur des segments « Affichage du bureau » | PISTE | S | haute | à faire | — | — |
| TYP-05 | Typographie — graisse des titres | PISTE | S | moyenne | à faire | — | — |
| TYP-09 | Typographie — barre et échelle de texte | PISTE | S | moyenne | à faire | — | — |
| TYP-10 | Écriture — boutons du conflit de raccourcis | PISTE | S | haute | à faire | — | — |
| TYP-12 | Écriture — fin de parcours | PISTE | S | haute | à faire | — | — |
| TYP-14 | Écriture — vocabulaire des états | PISTE | S | haute | à faire | — | — |
| TYP-18 | Disposition — ajustement du fond d'écran | PISTE | S | moyenne | à faire | — | — |
| ICO-12 | Graisse des glyphes | PISTE | M | haute | à faire | — | — |
| TYP-04 | Typographie — échelle des corps | PISTE | M | moyenne | à faire | — | — |
| TYP-11 | Écriture — pronoms possessifs | PISTE | M | haute | à faire | — | — |

## Phase 5 — Bureau, interactions et accompagnement

| Écart | Objet | Nature | Effort | Confiance | Statut | Commit | Remarque |
| --- | --- | --- | --- | --- | --- | --- | --- |
| DAT-19 | Seuil d'apparition de l'image de glissement | ÉCART | S | haute | à faire | — | — |
| DAT-20 | Infobulle du bouton « Selon le mode » | ÉCART | S | haute | à faire | — | — |
| PAT-01 | Dock en plein écran | ÉCART | M | haute | à faire | — | — |
| ICO-11 | Icônes de fichiers du bureau | ABSENT | M | moyenne | à faire | — | — |
| SYS-10 | Widgets de bureau : clic sur le fond | ABSENT | M | haute | à faire | — | — |
| SYS-11 | Galerie de widgets : aperçus | ABSENT | M | haute | à faire | — | — |
| SYS-12 | Centre de contrôle : tailles | ABSENT | M | haute | à faire | — | — |
| DAT-04 | Dock comme cible de dépôt | ABSENT | L | haute | à faire | — | — |
| ICO-10 | Apparence des icônes d'application | ABSENT | L | moyenne | à faire | — | — |
| SYS-15 | Widgets de bureau : tailles | ABSENT | L | haute | à faire | — | — |
| DAT-16 | Image de glissement de l'aperçu | PISTE | S | haute | à faire | — | — |
| DAT-17 | Curseur pendant le glisser d'une fenêtre | PISTE | S | haute | à faire | — | — |
| PRES-08 | Dock : clic sur une application groupée | PISTE | S | moyenne | à faire | — | — |
| TYP-17 | Disposition — marge basse des widgets | PISTE | S | moyenne | à faire | — | — |
| PAT-03 | Barre en plein écran | PISTE | M | haute | à faire | — | — |
| PRES-02 | Cartes de la barre : changement de taille | PISTE | M | moyenne | à faire | — | — |
| DAT-01 | Assistant de premier démarrage | ÉCART | S | haute | à faire | — | — |
| DAT-06 | Écran de connexion : envoi à vide | ÉCART | S | haute | à faire | — | — |
| PAT-10 | « + Nouvel espace » | ÉCART | S | haute | à faire | — | — |
| PRES-06 | Alerte problème : texte d'accompagnement | ÉCART | S | moyenne | à faire | — | — |
| PRES-10 | Assistant de configuration, étape Récapitulatif | ÉCART | S | haute | à faire | — | — |
| TYP-02 | Écriture — état sans résultat du lanceur | ÉCART | S | haute | à faire | — | — |
| TYP-13 | Écriture — échec sans message | ÉCART | S | moyenne | à faire | — | — |
| DAT-03 | Résultats de recherche de fichiers | ÉCART | M | haute | à faire | — | — |
| PRES-13 | Alerte problème répétée | ABSENT | S | moyenne | à faire | — | — |
| PRES-14 | Alerte problème : image | ABSENT | S | haute | à faire | — | — |
| DAT-07 | Découverte des fonctions du shell | ABSENT | M | haute | à faire | — | — |
| PAT-12 | Annulation des actions du shell | ABSENT | M | moyenne | à faire | — | — |
| DAT-12 | Annulation d'un déplacement de fenêtre | PISTE | S | haute | à faire | — | — |
| PRES-05 | Alerte problème : titre | PISTE | S | haute | à faire | — | — |
| PRES-09 | Confirmation dans les menus de la barre | PISTE | S | haute | à faire | — | — |
| DAT-11 | Visite des Réglages | PISTE | M | haute | à faire | — | — |

## Hors plan

Écarts relevés par les audits mais non retenus par le plan d'action : recommandations de la HIG à peser, et choix d'identité assumés. À ne pas traiter.

| Écart | Objet | Nature | Effort | Confiance | Statut | Commit | Remarque |
| --- | --- | --- | --- | --- | --- | --- | --- |
| DAT-10 | Extensions dans les résultats de fichiers | ÉCART | S | moyenne (la transposition suppose un réglage du bureau qui n'existe pas ici) | hors plan | — | — |
| ICO-15 | Images embarquées | ÉCART | S | haute | hors plan | — | — |
| MENU-13 | Libellés anglais des menus | ÉCART | S | moyenne | hors plan | — | — |
| MENU-14 | Boutons de confirmation et « Par défaut… » | ÉCART | S | moyenne | hors plan | — | — |
| PRES-11 | Visite guidée : points d'avancement | ÉCART | S | haute | hors plan | — | — |
| DAT-05 | Retrait d'un casque pendant une lecture | ABSENT | M | moyenne (un réglage de wireplumber, hors dépôt, pourrait déjà le faire) | hors plan | — | — |
| NAV-05 | Jetons de recherche | ABSENT | M | haute | hors plan | — | — |
| NAV-06 | Chemin d'un résultat de fichier | ABSENT | M | haute | hors plan | — | — |
| NAV-10 | Taille des rangées de la colonne | ABSENT | M | moyenne | hors plan | — | — |
| DAT-13 | Jauges du widget Système et instruments de Panorama | PISTE | S | haute | hors plan | — | — |
| DAT-15 | Fichiers récents dans la recherche | PISTE | S | haute | hors plan | — | — |
| DAT-18 | Infobulles d'état des modules et des widgets | PISTE | S | moyenne | hors plan | — | — |
| ENT-12 | Pointeurs de glisser-déposer dans la vue d'ensemble | PISTE | S | haute | hors plan | — | — |
| ENT-13 | Apparence du focus clavier selon la surface | PISTE | S | moyenne | hors plan | — | — |
| FON-09 | Écran de verrouillage | PISTE | S | moyenne (rendu du verrouillage non lu) | hors plan | — | — |
| FON-10 | Pack « Verre » sur des surfaces de texte | PISTE | S | moyenne | hors plan | — | — |
| FON-11 | Effet de bord au défilement | PISTE | S | moyenne (à peser : l'effet hig vise le contenu passant sous une barre) | hors plan | — | — |
| LAY-06 | Boxes — contenu avancé | PISTE | S | haute | hors plan | — | — |
| LAY-09 | Collections — tuiles de la grille | PISTE | S | moyenne | hors plan | — | — |
| LAY-10 | Lists and tables — textes rognés | PISTE | S | haute | hors plan | — | — |
| LAY-13 | Labels — navigation en icônes seules | PISTE | S | haute | hors plan | — | — |
| LAY-15 | Collections — cartes de l'aperçu | PISTE | S | moyenne | hors plan | — | — |
| MENU-08 | « Fermer les N fenêtres » | PISTE | S | moyenne | hors plan | — | — |
| MENU-11 | Titre des menus contextuels | PISTE | S | haute | hors plan | — | — |
| MENU-16 | « Confirmer » d'une action destructive | PISTE | S | moyenne | hors plan | — | — |
| MENU-17 | Libellés du menu de la fenêtre | PISTE | S | moyenne | hors plan | — | — |
| MENU-18 | Raccourcis du menu de la fenêtre | PISTE | S | faible | hors plan | — | — |
| NAV-09 | Masquage de la colonne de navigation | PISTE | S | haute | hors plan | — | — |
| NAV-11 | Débordement des segments d'espaces | PISTE | S | moyenne | hors plan | — | — |
| NAV-12 | Rail d'icônes seules | PISTE | S | faible | hors plan | — | — |
| PAT-15 | Périmètre de la recherche | PISTE | S | moyenne | hors plan | — | — |
| PAT-17 | Fenêtre active dans l'aperçu | PISTE | S | haute | hors plan | — | — |
| PAT-19 | Historique d'utilisation | PISTE | S | moyenne | hors plan | — | — |
| PRES-12 | Fondu de bord de la grille du lanceur | PISTE | S | moyenne | hors plan | — | — |
| SEL-10 | Libellés et icônes des curseurs | PISTE | S | haute | hors plan | — | — |
| SEL-11 | Textes coupés sans info-bulle d'expansion | PISTE | S | haute | hors plan | — | — |
| SEL-12 | Sélecteur de couleur : couleurs gardées | PISTE | S | haute | hors plan | — | — |
| STA-02 | Jauges : aplat sans dégradé | PISTE | S | haute | hors plan | — | — |
| SYS-21 | Widgets de bureau : taille du texte | PISTE | S | haute | hors plan | — | — |
| TYP-15 | Écriture — ellipse des commandes | PISTE | S | moyenne | hors plan | — | — |
| DAT-14 | Tendance dans le widget Système | PISTE | M | haute | hors plan | — | — |
| ICO-14 | Changement d'état d'un symbole | PISTE | M | moyenne | hors plan | — | — |
| LAY-11 | Collections — choix d'un style visuel | PISTE | M | moyenne | hors plan | — | — |
| LAY-14 | Split views — volets des réglages | PISTE | M | moyenne | hors plan | — | — |
| MENU-10 | Menu d'édition des champs | PISTE | M | moyenne | hors plan | — | — |
| NAV-08 | Recherches récentes et termes suggérés | PISTE | M | haute | hors plan | — | — |
| PAT-18 | Attente d'indexation | PISTE | M | haute | hors plan | — | — |
| PRES-03 | Barres de titre hyprbars | PISTE | M | moyenne | hors plan | — | — |
| PRES-07 | Alerte problème : informe seulement | PISTE | M | moyenne | hors plan | — | — |
| SYS-19 | Bulle : application au premier plan | PISTE | M | moyenne | hors plan | — | — |
| SYS-20 | Centre de contrôle : ajout | PISTE | M | moyenne | hors plan | — | — |
| TYP-16 | Disposition — largeur du lanceur | PISTE | M | moyenne | hors plan | — | — |
| NAV-04 | Portée de la recherche du lanceur | PISTE | L | moyenne | hors plan | — | — |
| PRES-15 | Cartes de la barre : épinglage | PISTE | L | moyenne | hors plan | — | — |
| FON-12 | Verre sur les petites cartes, les widgets et le tiroir des réglages | IDENTITÉ | - | — | identité | — | — |
| FON-13 | Mouvement sur les interactions fréquentes | IDENTITÉ | - | — | identité | — | — |
| FON-14 | Décor animé en continu (Panorama, LYRA) | IDENTITÉ | - | — | identité | — | — |
| ICO-16 | Thèmes d'icônes système pour les applications | IDENTITÉ | - | — | identité | — | décision D4 approuvée : le masque s'applique par-dessus |
| LAY-16 | Labels — police | IDENTITÉ | - | haute | identité | — | — |
| MENU-20 | Hauteur et styles de barre | IDENTITÉ | - | haute | identité | — | — |
| MENU-21 | Menu du dock sans « Quitter » | IDENTITÉ | - | haute | identité | — | — |
| NAV-13 | Couleur des icônes de la colonne | IDENTITÉ | - | haute | identité | — | — |
| NAV-14 | Réduction aux icônes sur petit écran | IDENTITÉ | - | haute | identité | — | — |
| NAV-15 | Champ de recherche sous les résultats (style Convex) | IDENTITÉ | - | haute | identité | — | — |
| PAT-21 | Volume des Réglages | IDENTITÉ | - | haute | identité | — | — |
| PRES-16 | Cartes de la barre et bulles du dock : pas de flèche | IDENTITÉ | - | haute | identité | — | — |
| STA-03 | Indicateur de lancement : marque iris respirante, fixe si mouvement coupé, bornée à 5 s | IDENTITÉ | - | haute | identité | — | — |
| STA-04 | Assistant LYRA : statut en texte à côté de l'indicateur de réflexion | IDENTITÉ | - | haute | identité | — | — |
| SYS-22 | Widgets de bureau : police | IDENTITÉ | - | — | identité | — | — |
| SYS-23 | Barre Island : une seule activité | IDENTITÉ | - | — | identité | — | — |
