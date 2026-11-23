# hyprdesk — fiche d'orientation pour l'audit HIG

> Document de travail (ne fait pas partie du dépôt). Rédigé le 23 novembre 2026 pour les auditeurs.
> Sauf mention « à vérifier », les faits ci-dessous ont été relevés directement dans le code, `DESIGN.md` ou la configuration active.

## 1. Le produit

hyprdesk 0.2.0 : un bureau pour le compositeur **Hyprland** (Wayland), écrit en Rust, interface **Iced** (`iced_layershell`). Il fournit : fond d'écran, barre, dock, lanceur d'applications et recherche de fichiers, notifications (démon freedesktop), widgets de bureau, centre de contrôle (`controls`), écran Réglages, aperçu des espaces de travail, commutateur de fenêtres, verrouillage, écran de connexion, assistante LYRA, Panorama (écrans bandeau tactiles) et un moteur de thème.

- Dépôt : `/home/jacques/Documents/DEV/Perso/LeLab-apps/mindlog.today/mindlog.hyprland_desktop`
- Commit audité : HEAD au moment de ta lecture (`00f6d69` « Menus : un seul composant, menus du dock, de la grille, d'édition, partage, menu de fenêtre » au 23 novembre à 20 h 20 ; le dépôt avance pendant l'audit), avec des fichiers modifiés non commités dans l'arbre de travail. Indique le hash que donne `git rev-parse --short HEAD` en tête de ton rapport. Citer les numéros de ligne de l'arbre de travail.
- Le commit `00f6d69` vient d'introduire un composant de menu unique (dock, grille d'applications, édition, partage, menu de fenêtre) : l'ouvrir avant de conclure qu'un menu est ABSENT.
- **Lecture seule** : ne modifier aucun fichier du dépôt, ne pas lancer de build, de test, d'installation, ni toucher à la configuration de la session.

## 2. Architecture de l'interface

- `src/ui/mod.rs` (7084 lignes) : un démon `iced_layershell`, **une surface par écran** ; au repos sur le calque `background`, elle dessine le fond d'écran. Objet `App` central.
- **Jetons** : `src/ui/tokens.rs` (typographie par rôle, courbe et durée de mouvement, verre par surface, états survolé/pressé/sélection/désactivé, focus clavier, contraste renforcé, mouvement réduit, transparence réduite, échelle de texte globale et par écran) ; `src/ui/design.rs` (rayons 20 et 10, surfaces flottante/panneau/encart, verre du lanceur) ; `src/theme_engine/design.rs` (jetons `Design`, `Glass`, `Motion`, `Contrast`, `States`) ; packs livrés dans `src/theme_engine/packs/*.toml` (ambre, ardoise, clair, contraste-eleve, encre, foret, verre) ; documentation `docs/THEME-ENGINE.md`.
- **Verre** : `src/ui/glass.rs`, `src/ui/lens.rs` (lentille réfractant le fond), `src/ui/behind.rs`, `src/ui/kinetic.rs` (ressorts).
- **Barre** : `src/ui/plugins/bar.rs` (modèle), `bar/surface.rs` (rendu, 2630 lignes), `bar/layout.rs`, `bar/motion.rs`, `bar/style/{continuite,graphite,island,convexe,selector}.rs`.
- **Cartes et menus de la barre** : `src/ui/plugins/bar/popover.rs` — sorties `Notifications`, `Volume`, `Brightness`, `Calendar`, `Widget`, `Workspaces`, `Overflow`, `Menu` (entrées venant de Lua ou de la zone de notification) ; `bar/popover/continuity.rs` (centre de contrôle du style Continuité) et `bar/popover/continuity/keyboard.rs`.
- **Dock** : `src/ui/plugins/dock.rs`, `dock/{surface,motion,lifecycle,keyboard,contrast}.rs`.
- **Lanceur d'applications** : `src/ui/plugins/launcher.rs`, `launcher/grid.rs`. **Recherche** : `src/ui/search.rs`, `src/search.rs`, index de fichiers `src/fileindex/`, réglages `src/ui/plugins/index.rs`.
- **Notifications** : serveur `src/notifications.rs` (`org.freedesktop.Notifications` 1.2, historique de 50), bulles `src/ui/toasts.rs`.
- **OSD** : `src/ui/osd.rs`, `src/osd.rs`.
- **Écran Réglages** : coquille en tiroir `src/ui/panel.rs` (1472 lignes) ; un « paquet » par domaine dans `src/ui/plugins/*.rs`, registre `BUILTIN` de `src/ui/plugins/mod.rs` ; `src/ui/hyprset.rs`, `src/ui/hyprlists.rs`, `src/ui/settings_search.rs` ; réglages des modules de barre `src/ui/plugins/modules.rs`, `src/ui/plugins/bar.rs`.
- **Espaces** : aperçu `src/ui/overview.rs`, réglages `src/ui/plugins/workspaces.rs` ; commutateur de fenêtres `src/ui/switcher.rs`, `src/switcher.rs` ; placement des widgets `src/ui/desk_layout.rs`.
- **Widgets** : catalogue `src/widgets/mod.rs` — intégrés `controls`, `launcher`, `clock`, `calendar`, `system`, `media`, `command`, `lua`, plus des plugins Lua (`src/widgets/plugins.rs`) ; rendu `src/ui/widget_view.rs`, `src/widgets/{system,media,command,controls}.rs` ; réglages `src/ui/plugins/widgets.rs`. Le widget `controls` compose Wi-Fi, Bluetooth, luminosité, son et mode Bureau (`src/ui/controls.rs`).
- **Verrouillage et première utilisation** : `src/lock/`, `src/ui/wizard.rs`, `src/ui/tour.rs` ; alertes et demandes : `src/ui/problem.rs`, `src/ui/polkit.rs`.
- **Zone de notification (systray)** : `src/tray.rs`.
- **Raccourcis** : `src/ui/plugins/keys.rs`, `src/ui/key_conflict.rs`, `src/binds_file.rs`, `src/hypr.rs`.
- **Fenêtres côté Hyprland** : `src/ui/plugins/windows.rs`, `src/ui/plugins/layouts.rs`, `src/tiling.rs`, `src/windows.rs`, `src/hyprbars.rs` (à vérifier : barres de titre du module Hyprland), `src/zones.rs` (à vérifier).

## 3. Langage visuel documenté (`DESIGN.md`, source actuelle)

- Espace calme, contrôles immédiatement reconnaissables ; fond libre ; barre compacte ; réglages en couche de travail latérale.
- Palette par défaut : texte `#f2f4fc`, surfaces `#171b27`, accent iris `#a69cff` ; couleurs définies en Lua ; accent réservé à la sélection et aux actions.
- Matières : verre fumé pour les petites cartes et le dock, surface presque opaque pour les réglages ; contour clair discret, une seule ombre douce.
- Typographie : Mindlog 0.7, cinq graisses de Light à Bold ; poids normal, medium et semibold pour la hiérarchie.
- Icônes : thèmes système pour les applications ; icônes symboliques pour les commandes ; secours vectoriels sur grille 24 px, trait arrondi de 1,7 px (`src/ui/icons.rs`).
- Espacement : base 4 px, petits groupes 8–12 px, sections 20–24 px. Rayons 10 px (commandes), 20 px (cartes), 24 px (panneau).
- Mouvement : retour visuel au pressage, survol discret, apparition courte sans rebond décoratif ; toujours repartir de la valeur affichée lors d'une interruption ; `transition = "none"` désactive les mouvements.
- Lanceur (dock) : survol continu à 140 ms maximum, agrandissement de 9 % maximum, levée de 2,5 px ; point pour une application ouverte, trait iris pour la fenêtre active ; bulles ouvertes après 180 ms de survol ; apparition en fondu et glissement de 8 px en 180 ms maximum.
- Barre Continuité : cartes ancrées à 10 px de la barre, apparition 180 ms, changement de contenu 160 ms, fermeture 120 ms ; opacité par défaut des menus 0,94.
- Écran « Espaces de travail » (`hyprdesk overview`, SUPER+Tab) : cartes par espace, fenêtres en miniatures, glisser-déposer d'une fenêtre vers une autre carte.

## 4. Décisions d'identité — à signaler comme IDENTITÉ, jamais à corriger

Ces différences avec macOS sont délibérées et documentées ; elles ne sont pas des défauts.

- Police **Mindlog** plutôt que SF Pro ; **accent iris** plutôt que le bleu Apple.
- Quatre styles de barre : **Continuité** (barre « Unix » et dock autonome), **Graphite** (référence Windows 11 : barre en bas de 48 px, boutons de rayon 4), **Island** (capsule centrale), **Convex** (cadre continu).
- `design-plans/barre-continuite.md` : « L'inspiration Apple porte sur la retenue, la précision typographique, les matières et la continuité des gestes. L'identité reste celle de hyprdesk : Mindlog, accent iris, organisation par workspaces, configuration Lua et icônes système/symboliques. »
- Configuration en Lua de bout en bout, plugins et widgets Lua, thèmes remplaçables par des packs.

En revanche, l'utilisateur demande aujourd'hui un bureau à la hauteur de macOS : un composant macOS **absent** de hyprdesk est une proposition légitime, pas une identité à préserver.

## 5. Valeurs actives de la session (relevées le 23 novembre 2026)

- Mode du bureau : `no-desktop` (fond d'écran seul) ; écrans DP-1, HDMI-A-1, DP-2.
- Barre : en haut, hauteur 36, style `continuity`, disposition `continuous`, toujours visible, opacité et couleurs du thème.
- Dock : en bas, icônes de 42 px, fenêtres du workspace courant seulement, groupement par application, boutons Réglages et Assistante affichés, aucune application épinglée.
- Lanceur : style automatique (grande feuille en plein bureau, carte centrée sinon), 8 suggestions, largeur 680, rayon 28, marge intérieure 22, 7 colonnes, rangées de 48 px, 8 résultats visibles, ancrage centré.
- Workspaces permanents 1 à 5. Widgets actifs sur le bureau : horloge, calendrier, système, média.

## 6. Rappel de la méthode d'audit

1. Ne conclure que sur des preuves : une règle HIG citée (citation anglaise et page) **et** un chemin de rendu tracé dans le code (`fichier:ligne`).
2. Catégories de conclusion : CONFORME, ÉCART (règle impérative non respectée), PISTE (recommandation HIG non suivie), ABSENT (composant macOS sans équivalent), IDENTITÉ (choix délibéré documenté), À VÉRIFIER (ne se juge que sur un rendu), SANS OBJET.
3. Tenter de falsifier chaque écart avant de le rendre : rouvrir le fichier cité, chercher une option de configuration, un autre style de barre, un widget ou un chemin de rendu qui satisferait déjà la règle.
4. Hors périmètre : bugs de fonctionnement, architecture, performances, qualité du code, accessibilité (non auditée).
