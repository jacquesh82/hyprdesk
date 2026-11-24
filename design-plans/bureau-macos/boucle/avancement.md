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
| 2026-11-24 | A3 | ICO-07, LAY-04, SYS-14, ICO-03, SYS-05, SYS-18 traités ; il en reste 37 dans ce couloir |
| 2026-11-24 | A4 | PAT-04 abordé, non terminé : échec 1 de la porte, plan commité, changement conservé en patch ; il en reste 37 dans ce couloir |
| 2026-11-24 | A5 | PAT-04 traité ; il en reste 36 dans ce couloir |

### Couloir B

| Date | Itération | Résultat |
| --- | --- | --- |
| 2026-11-23 | B1 | SEL-01, DAT-09, NAV-01, NAV-02, NAV-03 traités ; il en reste 52 dans ce couloir |
| 2026-11-24 | B2 | PAT-09, PRES-01, TYP-03 traités ; il en reste 49 dans ce couloir |
| 2026-11-24 | B3 | TYP-01, TYP-07 traités ; il en reste 47 dans ce couloir |
| 2026-11-24 | B4 | ACC-10 traité ; il en reste 46 dans ce couloir |
| 2026-11-24 | B5 | DAT-02, PRES-04 traités ; il en reste 44 dans ce couloir |
| 2026-11-24 | B6 | SEL-03 traité ; LAY-05 non traité (effort corrigé S → M, plan à écrire) ; il en reste 43 dans ce couloir |
| 2026-11-24 | B7 | LAY-05 traité ; il en reste 42 dans ce couloir |

### Couloir C

| Date | Itération | Résultat |
| --- | --- | --- |
| 2026-11-24 | C1 | MENU-02, ENT-05, DAT-08 traités (travail de l'itération interrompue par la panne de quota, repris et mis au point) ; il en reste 40 dans ce couloir |
| 2026-11-24 | C2 | ENT-04 traité ; il en reste 39 dans ce couloir |
| 2026-11-24 | C3 | ICO-02, ICO-09 traités ; il en reste 37 dans ce couloir |
| 2026-11-24 | C4 | MENU-12, MENU-15 traités ; il en reste 35 dans ce couloir |
| 2026-11-24 | C5 | MENU-01 (étapes 1 et 2 sur 3) traitées ; il en reste 35 dans ce couloir |
| 2026-11-24 | C6 | MENU-01 (étape 3 sur 3) traité ; il en reste 34 dans ce couloir |

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
| PAT-09 | Dernier volet des Réglages | ÉCART | S | haute | fait | 8ad5cc3 | le dernier paquet consulté est retenu sur `App::panel_tab` (`src/ui/mod.rs`), comme les sous-onglets : chaque changement d'onglet passe par `App::panel_show` (`src/ui/panel.rs`) et le tiroir rouvre dessus, les données du paquet chargées par `plugins::on_show` ; un paquet qui n'est plus affiché (plugin coupé, « share » sans configuration avancée) ramène au Bureau. Preuve exacte à HEAD. Propriétaire partagé `panel.rs` : relu à froid, sans outil d'agent — le diff fait ce que demande l'écart ; `DESIGN.md` et les conventions du dépôt sont respectés ; aucune régression trouvée, un test manquait, il est ajouté (`le_tiroir_rouvre_sur_le_dernier_paquet_consulte`). Pas de capture : aucun visuel ne change, les scènes de l'aperçu choisissent toujours leur onglet. |
| ENT-04 | Ordre des modificatrices affichées | ÉCART | S | haute | fait | 1a09503 | Une seule table, `hypr::MODIFIER_ORDER` (CTRL, ALT, SHIFT, SUPER), et un rang partagé `modifier_rank` (synonymes de `modifier_bit` : CONTROL, WIN, LOGO, META, MOD1, MOD4). `essentials::pretty` ordonne les modificateurs de toute combinaison affichée, chacun gardant le nom écrit ; la capture des Réglages (`combo_of`, `keys.rs`), `modmask_name` et `universal_binds` construisent dans cet ordre. La configuration, ses valeurs par défaut (`SUPER + ALT + D`) et l'aide de `hyprdesk config` gardent leur écriture : l'analyse (`parse_combo`, `same_text`) ne dépend jamais de l'ordre. L'exemple des trois messages « raccourci illisible » (`keys.rs`, `launcher.rs`, `cli_config.rs`, `locales/en.toml`) passe au nouvel ordre ; une phrase dans les deux guides et dans `DESIGN.md`. Tests : attentes de `capture`, `combinaisons_lisibles` (plus deux cas d'ordre quelconque), `essentiels_face_a_hyprland_lua`, `workspaces_bureau_affiche`. Preuve à HEAD juste. |
| ENT-05 | MAJ+Tab dans les menus et la vue d'ensemble | ÉCART | S | haute | fait | fee0adc | `icon_menu_key` et `overview_key` reçoivent l'état de Maj (`self.mods.shift()`) : Tab avance, Maj+Tab recule comme la flèche haut, dans les menus du clic droit comme dans l'aperçu des espaces. Même changement que MENU-02 pour les menus (mêmes fonctions). Tests `maj_tab_recule_et_la_carte_suite`, `tab_et_maj_tab_changent_de_rang`. |
| PRES-01 | Alertes problème et polkit | ÉCART | S | haute | fait | 8ad5cc3 | Échap ferme l'alerte problème (`PrMsg::Close`, branche ajoutée en tête du traitement d'Échap de `src/ui/mod.rs`, avant le menu d'icône, la carte, le tiroir et l'assistant) et annule la carte polkit (`PMsg::Cancel`, abonnement aux touches de l'agent, `src/ui/polkit.rs`). Précision sur la preuve : dans le démon, la carte polkit n'est qu'une scène de l'aperçu (`view_polkit` ne rend que `Msg::Noop`), l'agent polkit est un processus à part — c'est lui qui reçoit Échap. Tests `echap_ferme_l_alerte` et `echap_annule`. |
| PAT-08 | Échec d'un lancement | ÉCART | S | haute | fait | 0dad66c | une entrée sans commande (`LaunchEnd::Unresolved`) ou une commande refusée par Hyprland (`LaunchEnd::Refused` : la cause remonte de `run_from_desktop_on`, `src/ui/mod.rs`) affiche une notification de Hyprland qui nomme l'application et la cause (`launcher_launch_ended`, `src/ui/plugins/dock/lifecycle.rs`), comme le renommage d'espace. Même lot que STA-01. |
| PAT-11 | Échecs sans message | ÉCART | S | haute | fait | c929ed3 | trois chemins ne se contentent plus du journal : copie d'un code ou d'un identifiant myMFA et ouverture d'un fichier depuis la recherche (`src/ui/search.rs`), action ou fermeture d'une notification que son application n'a pas reçue (`src/ui/toasts.rs`) ; chacun affiche une notification de Hyprland avec la cause, comme le renommage d'espace. |
| STA-01 | Indicateur de lancement du dock : fin sans retour | ÉCART | S | haute | fait | 0dad66c | au délai de 5 s sans qu'aucune fenêtre arrive, la fin du lancement est annoncée par une notification discrète (indice, 6 s) qui propose de relancer l'application ; une fenêtre d'une autre classe arrivée entre-temps n'est pas un échec, rien n'est annoncé. Tests `une_fin_de_lancement_est_annoncee_une_fois_et_jamais_si_une_fenetre_est_arrivee` et `une_fin_de_lancement_est_annoncee_par_le_nom_de_l_application_et_sa_cause`. Même lot que PAT-08. |
| TYP-03 | Écriture — renommage d'une icône du bureau | ÉCART | S | haute | fait | 8ad5cc3 | `rename` (`src/ui/context.rs`) rend la forme corrective « Choisissez un nom non vide, sans barre oblique (/) » (en : « Choose a name that isn’t empty and contains no slash », `locales/en.toml`). La seconde occurrence de « Nom invalide. » est gardée : elle garde un fichier sans dossier parent, hors d'atteinte d'une icône du bureau, où le nom n'est pas en cause. Les noms « . » et « .. » reçoivent le même message, donné tel quel par la colonne Correction. Pas de test : libellé seul, le test de `rename` ne regarde que l'écrasement et le succès. |
| TYP-01 | Typographie — plancher de taille de texte | ÉCART | S | haute | fait | 78d1ffb | `ts`, `ts_label` et `ts_metric` (`src/ui/tokens.rs`) ne rendent plus rien sous 10 px (constante `MIN_TEXT`) : le plancher s'applique après le rôle du thème et les deux échelles. À 50 % sur un écran, `ts(12.0)` vaut 10 px et non 6 ; à 75 % pour l'utilisateur, `ts(11.0)` vaut 10 px et non 8,25 ; au-dessus du plancher rien ne change, et à l'échelle 1 plus aucune taille du dépôt ne passe dessous (TYP-07). Les curseurs gardent leurs bornes (75 à 200 %, 50 à 200 %) : sous le plancher, les petits corps se raccordent à 10 px. Propriétaire partagé `tokens.rs` : relu à froid, sans outil d'agent — le diff fait ce que demande l'écart ; `DESIGN.md` et les conventions du dépôt sont respectés ; aucune régression trouvée, et le test `jetons_posés_lus_par_les_surfaces`, qui attendait 7,8 px à 50 %, affirme désormais le plancher (corps, légende, libellé, mesure) et que l'échelle s'applique au-dessus. Documenté dans `DESIGN.md` (Typographie) et `docs/THEME-ENGINE.md`. Porte complète avec captures (`scripts/verify.sh --visual`) : à l'échelle 1, aucune scène ne change. |
| TYP-07 | Typographie — corps sous le minimum | ÉCART | S | haute | fait | 78d1ffb | les deux corps à 9 px passent à `ts(10.0)` : nom de l'espace du sélecteur de la barre (`src/ui/plugins/bar/style/selector.rs`) et jours du graphique d'usage myMFA (`src/ui/mymfa_fenetre.rs`). Dépassement de la colonne Correction : la ligne du nom, fixée à 11 px, suit désormais le texte (`head_h = ts(10.0) + 2`), sinon un texte de 10 px débordait sur la minimap, qui prend ce qui reste (1 px de moins). Preuve exacte à HEAD, numéros de ligne décalés. Capture vérifiée : `bar-convex-switch` à 1440 px, noms nets au-dessus des miniatures. La fenêtre myMFA n'a pas de scène d'aperçu. Pas de test : taille de vue seule. |
| FON-01 | Barre et widgets du bureau : lisibilité sur fond clair | ÉCART | S | haute (mécanisme) ; ratios estimés | fait | a2b2895 | échec 1 (test du défaut `transparence_adaptative`), puis réussite : `Place::default_effect` active la densification de `bar` et `widgets`, plafond 0,95 (celui du lanceur ; le préréglage `glass_regular` du code est à 0,94, la preuve annonçait 0,95). Au-delà de la colonne Correction, requis par le changement : `Preset::effects` suit la même règle ; la rédaction des réglages écrit `adaptive = false` quand le plafond est coupé là où le défaut en a un, sans quoi « off » n'était plus conservé au rechargement. Les packs livrés restent tels quels : une surface qui n'en dit rien hérite du défaut (`apply_effect`). Tests `barre_et_widgets_se_densifient_sur_le_fond_par_defaut` et `barre_et_widgets_des_packs_livrés_se_densifient`. Porte visuelle passée ; les scènes sont sur fond sombre, la densification n'y est pas visible. |
| FON-03 | Texte secondaire dessiné à alpha fixe | ÉCART | S | haute | fait | a2b2895 | les trois textes secondaires à alpha fixe prennent `ink().dim` : `fx_origin` et `fx_effects` (`src/ui/plugins/animations.rs`), les libellés des nuances du sélecteur de couleurs (`src/ui/color_picker.rs`, qui reçoit `dim` de la page Couleurs de la barre). Capture `animations-effets` regardée : texte lisible. |
| ICO-07 | Fermeture, lecture et pause | ÉCART | S | haute | fait | c22ba14 | `icons::glyph` (`src/ui/icons.rs`, module ouvert à l'écran de verrouillage) pose un glyphe du jeu, teinté et dimensionné : `window-close` remplace ✕ et × dans les douze endroits cités (bulle, centre de contrôle, convexe, courbes, variables d'environnement, règles, permissions, assistant, Panorama notifications et tâches), `media-playback-start` et `media-playback-pause` remplacent ▶ et ⏸ (verrouillage, morceau en cours de Panorama), l'éclair `⚡` de la batterie du verrouillage devient la batterie du jeu (`battery-level-N`, `-charging`). Preuve incomplète, corrigée : deux « × » de retrait non cités traités aussi (zones de widget de la barre, `delete_button` des paquets). Laissés hors de l'écart, sans glyphe équivalent dans le jeu : flèches ↑ ↓ ← → de déplacement et « ▶ » de page suivante de Panorama ; les replis `text("×")` du tiroir et du lanceur ne s'affichent jamais. Captures vérifiées (`bar-controls`, `bar-convex-notification`, `notifications`). |
| ICO-02 | Commandes média et volume, bouton Réglages du dock | ÉCART | S | haute | fait | 5d8abbb | Preuve exacte. `reload_icons` ne cherche le thème que pour les noms d'`UI_ICONS` que le jeu de hyprdesk ne dessine pas (comme `want_icon`) ; `App::ui_icon` teinte le glyphe comme les autres commandes (widget média, 1,3 × conservé) ; le dock autonome prend le même glyphe Réglages à 0,85 × que le dock intégré (avant : icône du thème à 0,6 × et un repli texte) ; constante `ICON_SETTINGS` retirée. Test `icones_interface_du_jeu`. Les noms de commande des menus qui restent hors jeu sont à ICO-06. Conflit à la publication avec ICO-07 (couloir A, publié pendant la porte) résolu par union : `ui_icon` pose le glyphe par `icons::glyph`, venu de l'autre couloir. |
| ICO-09 | Icône de remplacement (application sans icône) | ÉCART | S | haute | fait | 5d8abbb | Preuve exacte, numéros de ligne décalés (lanceur `src/ui/plugins/launcher/grid.rs:273-305`, bureau `src/ui/mod.rs:6085-6113`, dock `src/ui/plugins/dock/surface.rs:474-504`). Une seule fonction, `icons::fallback_icon` (initiale par `icons::initial`, graisse de titre, 0,5 × le côté, tuile carrée d'accent plein, rayon 0,25 ×, encre `design::on_accent`), appelée par les trois surfaces ; tests `initiale_de_replacement` et `icone_de_replacement_partagee`. Dépassement de la colonne Correction, pour un seul dessin : tuile d'accent plein dans le dock (avant 16 %) ; graisse de titre au dock et au bureau (avant graisse du corps) ; tuile carrée au lanceur (avant : largeur de l'icône, hauteur du texte) ; initiale du bureau en majuscule (avant : première lettre telle quelle), encrée pour rester lisible sur l'accent. `DESIGN.md` (ligne Icônes) à jour. |
| DAT-08 | Aperçu des espaces : cible de dépôt | ÉCART | S | haute | fait | fee0adc | `is_drop_target` (`src/ui/overview.rs`) : la bordure d'accent et l'ombre ne marquent que les cartes autres que celle d'origine de la fenêtre glissée ; y déposer ne fait toujours rien. Test `la_carte_d_origine_n_est_pas_une_cible`. |

## Phase 1 — Notifications

| Écart | Objet | Nature | Effort | Confiance | Statut | Commit | Remarque |
| --- | --- | --- | --- | --- | --- | --- | --- |
| LAY-04 | Lists and tables — corps des notifications | ÉCART | S | moyenne | fait | fd091e2 | `card_text` (`src/ui/toasts.rs`, constantes `TITLE_CHARS` = 42 et `BODY_CHARS` = 74) borne le titre et le corps des cartes de la barre et du volet Notifications de Panorama (`panes.rs`), sans saut de ligne, comme le centre Continuité. Preuve exacte. Test : `une_carte_borne_titre_et_corps`. |
| SYS-14 | Barre Island : icône de l'app | ÉCART | S | haute | fait | fd091e2 | La prise de main Island (`WidgetKind::Clock`, `src/ui/plugins/bar/surface.rs`) affiche la cloche du jeu (`icons::glyph("notifications")`), monochrome, à la place de l'icône de l'application ; capture `bar-island-notif` vérifiée. Preuve exacte. |
| ICO-03 | Bulle de notification | ABSENT | S | haute | fait | fd091e2 | `notif_icon` (`src/ui/toasts.rs`) prend l'icône envoyée par l'application, sinon celle de son application : `class_icon_name` cherche, parmi les applications installées, l'entrée dont l'identifiant ou la classe correspond au `desktop-entry` ; cette icône est demandée à l'arrivée (`want_notif_icons`), sous Silence aussi ; la cloche reste le dernier repli et le centre Continuité passe par la même fonction. Dépasse la colonne Correction : le nom de l'application sert de classe quand `desktop-entry` manque. Preuve exacte. Test : `une_notification_sans_icone_prend_celle_de_son_application`. |
| PAT-04 | Durée du Silence | ABSENT | S | haute | fait | dd9ad0b | plus lourd que l'effort inscrit (A3) : traité comme M, plan `boucle/plans/PAT-04.md`. échec 1 (A4 : deux tests de mon cru, corrigés depuis), puis réussite (A5). Le Silence porte une échéance facultative, gardée dans le même fichier `dnd` (`Silence`, `SilenceEnd`, `src/notifications.rs` ; un ancien contenu `1\n` se relit sans fin, une échéance passée n'est jamais relayée). La tuile « Silence » éteinte du centre de contrôle ouvre la carte de menu existante (`Request::Silence`, `MenuSource::Silence`) : une heure, jusqu'à ce soir (18 h, entrée retirée le soir), jusqu'à désactivation ; allumée, elle nomme sa fin (« Silence · jusqu'à 18:00 ») et un clic l'éteint. Abonnement `toasts::silence_ends` (réveil borné à 5 min, heure relue à l'horloge murale à chaque notification, pour la mise en veille). `hyprdesk notifications dnd on [1h\|soir]` ; `dnd` seul, `dnd on`, `dnd off` gardent leur sens ; `soir` est refusé le soir. Modules de barre Lua : `dnd_until`. Tests `fin_du_silence`, `silence_garde_d_une_session_a_l_autre`, `etat_avec_la_fin_du_silence`, `la_fin_du_silence_se_choisit_a_l_activation`. Travail repris du patch de reprise, mis au point sans être refait (deux conflits additifs avec MENU-01 et SEL-03, résolus par union). Propriétaires partagés `src/notifications.rs` et `bar/popover.rs` : relu à froid, sans outil d'agent — le diff fait ce que demande l'écart ; `DESIGN.md` et les conventions du dépôt sont respectés ; aucune régression, les tests cités couvrent les trois fins, le soir passé et la relecture. Documentation : `docs/CONTROL.md`, `docs/GUIDE.fr.md`, `docs/GUIDE.md`, `locales/en.toml`. |
| SYS-05 | Aperçus masquables | ABSENT | S | haute | fait | bee562a | Option `notifications.hide_previews` du registre `design_options` (`on` ou `off`, `off` par défaut, libellé « Masquer les aperçus ») : le corps de la bulle, de la carte Notifications de la barre, du centre Continuité et du volet Panorama devient « Notification » (`card_text`, `src/ui/toasts.rs`) ; un corps vide reste vide. Limite assumée : le fichier d'état `hyprdesk-notifications.txt` (module de barre, `hyprdesk notifications`) porte encore le corps, `Toasts::publish` n'a pas accès à la configuration. Relecture à froid faite (propriétaire partagé `continuity.rs`). Test : `une_carte_borne_titre_et_corps`. Preuve exacte. |
| SYS-17 | Actions destructrices | ABSENT | S | moyenne | à faire | — | — |
| SYS-18 | Icônes des actions | ABSENT | S | moyenne | fait | 615bd3e | Le serveur déclare la capacité `action-icons` ; `parse_action_icons` (`src/notifications.rs`) lit le hint : une icône par action, dans l'ordre de `actions` (`-` ou vide : aucune ; préfixe `image://` retiré), gardée dans `Notification::action_icons`. À l'arrivée, `want_notif_icons` demande ces icônes ; dans la bulle, le bouton d'action porte son icône après le libellé quand le thème la connaît. `Notification` dépasse 200 octets : l'événement `Shown` porte `#[allow(clippy::large_enum_variant)]`, boxifier la variante toucherait seize sites. Relecture à froid faite (propriétaire partagé `notifications.rs`). Tests : `icones_des_actions`, cas ajouté à `une_notification_sans_icone_prend_celle_de_son_application`. Preuve exacte. |
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
| ACC-10 | Boutons sans bordure trop rapprochés | PISTE | S | moyenne | fait | 0e7240c | trois écarts portés à 12 px : en-tête des cartes de règles et de variables (« ↑ », « ↓ », « × », `src/ui/hyprlists.rs`), de 6 à 12 px, et « × », qui supprime l'élément, se tient à 24 px de ce qui le précède (12 px de marge gauche en plus), la valeur de la HIG pour les boutons sans bordure ; actions d'une bulle de notification (`src/ui/toasts.rs`), de 8 à 12 px ; « Confirmer » et « Annuler » de la remise par défaut (`src/ui/panel.rs`), de 8 à 12 px. Preuve exacte à HEAD, numéros de ligne décalés ; la croix est depuis ICO-07 le glyphe du jeu, seul l'écart est en cause. Propriétaire partagé `panel.rs` : relu à froid, sans outil d'agent — le diff fait ce que demande l'écart ; `DESIGN.md` (base 4 px) et les conventions du dépôt sont respectés ; aucune régression trouvée ; pas de test, écarts de vue. Porte complète avec captures avant le dernier rebasage : `fenetres-regles` et `environnement` (croix en place, à 12 px du bord de la carte), bulle à deux actions de la scène `fenetres-regles` (elles tiennent dans la bulle) ; aucune scène ne montre « ↑ », « ↓ » et « × » ensemble, ni la confirmation de la remise par défaut. Après rebasage sur les publications des couloirs A et C, porte complète sans captures. |

## Phase 2 — Menus du shell et clavier

| Écart | Objet | Nature | Effort | Confiance | Statut | Commit | Remarque |
| --- | --- | --- | --- | --- | --- | --- | --- |
| MENU-04 | Commandes accessibles seulement par clic droit | ÉCART | S | haute | à faire | — | Attend MENU-01 (barre des menus, L), comme le demande sa ligne du plan : à traiter juste après, pas avant. |
| MENU-15 | Icônes des actions `.desktop` | ÉCART | S | moyenne | fait | cd5a220 | Preuve exacte (`action_lines`, `src/ui/context.rs`). Le groupe des actions ne porte d'icônes que si chacune en déclare une (`action_icon` : un nom, jamais un chemin) ; sinon aucune n'en a et le groupe se sépare de « Ouvrir » (`place_actions`), dans le menu de l'icône du bureau comme dans ceux du dock et de la grille, qui listent les mêmes actions. Plus aucune icône de secours `application-x-executable-symbolic`. Test `actions_tous_ou_aucune_icone`. Au-delà de la colonne Correction, que la règle « tous ou aucune » du même groupe impose : les actions sans icônes forment un groupe à part au lieu de rester avec « Ouvrir », qui a la sienne. |
| MENU-03 | Sous-menus d'un niveau | ABSENT | M | haute | à faire | — | — |
| MENU-05 | Menu Aide | ABSENT | M | haute | à faire | — | — |
| MENU-01 | Barre des menus : inventaire des commandes du shell | ABSENT | L | haute | fait | 41aa821 | Plan en trois étapes (`boucle/plans/MENU-01.md`), toutes faites. Étapes 1 et 2 (C5) : « hyprdesk » toujours en tête de la zone amont (hors `BarItem`, priorité `layout::PINNED`), puis « Fenêtre » et « Espaces » comme menus titrés ; sans fenêtre active, « Fenêtre » reste affiché et inactif. Dépasse la colonne Correction : « À propos de hyprdesk » ouvre le menu (règle du menu d'application) ; le groupe de puissance est sans icône, faute d'icône sûre pour la mise en veille. Étape 3 (C6) : scène d'aperçu `bar-menu-shell`, le menu ouvert sous son titre, capturée aux trois largeurs par `scripts/verify.sh --visual`. |
| MENU-06 | Menus contextuels d'une surface à l'autre | PISTE | S | moyenne | à faire | — | — |
| MENU-07 | Titre de la fenêtre active dans la barre | PISTE | S | haute | à faire | — | — |
| MENU-09 | Ordre des entrées du menu du dock | PISTE | S | moyenne | à faire | — | — |
| MENU-12 | Menu contextuel d'une icône du bureau | PISTE | S | haute | fait | cd5a220 | Preuve exacte (`icon_lines`, `src/ui/context.rs`). « Propriétés » et « Réglages des icônes… » passent au dernier groupe, avant « Retirer du bureau » et « Mettre à la corbeille » (toujours en dernier et en rouge) ; le groupe central retient Renommer…, Partager…, Copier le chemin, Définir comme fond d'écran, Épingler au dock, cinq au plus. Trois groupes toujours (`menu_d_un_fichier`). Test `proprietes_au_dernier_groupe`. |
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
| DAT-02 | Champs de couleur et de texte des réglages | ÉCART | S | haute | fait | 88eb59d | la valeur d'un champ d'option (couleur, dégradé, côtés, texte) est analysée à chaque frappe : `Msg::Draft` porte désormais le magasin (`Draft(Store, String, String)`) et pose le message dans `opt_errors` dès que la valeur est invalide, avec le rendu existant sous le champ (`draft_error`, `src/ui/hyprset.rs`) ; il disparaît à la frappe valide suivante. Un champ vidé (qui retire l'option à Entrée) et une option inconnue n'ont pas de message. Conséquence assumée par la colonne Correction : le message paraît aussi au milieu d'une saisie incomplète (« #1 » le temps d'écrire « #123456 »). Test `la_saisie_est_validee_a_chaque_frappe`. Pas de documentation à changer : les guides ne décrivent pas la validation par Entrée. |
| DAT-09 | Champs numériques des listes de règles | ÉCART | S | moyenne | fait | 0824a6e | même changement que SEL-01 : un champ `Field::Num` n'accepte qu'un nombre fini, la saisie refusée reste au champ et l'erreur s'affiche dessous. |
| LAY-05 | Labels — textes utiles des réglages | ÉCART | M | haute | fait | e935df8 | composant `selectable` (`src/ui/selectable.rs`) : un `text_editor` d'Iced en lecture seule, dont la sélection vit dans l'arbre des widgets — aucun état ni message dans les modèles, à la différence de `chat.rs`. Ses actions sont traitées sur place (éditions ignorées, copier fait par l'éditeur), la molette rendue au conteneur, la largeur ramenée à celle d'un `text`. Six sites : résultat de la remise par défaut et note de `field` (`panel.rs`), même note au féminin (`workspaces.rs`, non citée par l'audit), `hyprlists.rs`, `hyprset.rs`, note Hyprbars (`desktop.rs`). Preuve exacte, numéros de ligne décalés. Plan : `boucle/plans/LAY-05.md` (B6 : premier essai non concluant, effort corrigé S → M). 4 tests unitaires, une règle dans `DESIGN.md`. Limites : caret clignotant après un clic et frappe absorbée jusqu'au clic ailleurs, comme dans tout champ ; chemin des événements vérifié par une capture (1440 × 1300), non par un test. |
| PRES-04 | Tiroir des Réglages : barres de défilement | ÉCART | S | moyenne | fait | 88eb59d | la zone de contenu du tiroir reçoit la barre de 4 px de la navigation (`src/ui/panel.rs`), au lieu de la barre par défaut d'Iced : les deux barres du panneau ont la même taille. Propriétaire partagé `panel.rs` : relu à froid, sans outil d'agent — le diff fait ce que demande l'écart ; `DESIGN.md` et les conventions du dépôt sont respectés ; aucune régression trouvée ; pas de test, taille de vue seule. Porte complète avec captures avant le dernier rebasage (`scripts/verify.sh --visual`) : `fenetres-regles` à 1440 px, la barre de la zone de contenu a la même épaisseur fine que celle de la navigation ; après rebasage sur la publication du couloir A (aucun fichier du lot touché), porte complète sans captures. |
| SEL-03 | Commutateur « Configuration avancée » | ÉCART | S | moyenne | fait | 5d46b85 | le commutateur quitte la ligne d'actions de l'en-tête et prend sa propre ligne dans la vue de détail du tiroir, sous « Modifications enregistrées automatiquement » (`src/ui/panel.rs`) ; même widget, même message `Advanced`. Visite guidée, `docs/GUIDE.fr.md`, `docs/GUIDE.md`, `docs/PLUGINS.md` : emplacement mis à jour. Propriétaire partagé `panel.rs` : relu à froid, sans outil d'agent — le diff fait ce que demande l'écart ; `DESIGN.md` et les conventions du dépôt sont respectés ; aucune régression (la ligne d'actions de l'en-tête est seulement plus courte, le tiroir étroit y gagne) ; pas de test, taille de vue seule. Porte complète, puis porte complète avec captures (`scripts/verify.sh --visual`) : scène `tour` à 1440 px, le commutateur est dans le corps, l'en-tête porte la recherche, la visite guidée, l'assistant et la fermeture. Porte repassée après le rebasage (MENU-01 du couloir C publié entre-temps). |
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
