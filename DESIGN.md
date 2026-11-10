# hyprdesk — langage visuel

Le bureau est un espace calme, avec des contrôles immédiatement reconnaissables. Le fond reste libre ; la barre est compacte et les réglages forment une couche de travail latérale.

- Palette par défaut : texte #f2f4fc, surfaces #171b27, accent iris #a69cff. Respecter les couleurs définies en Lua. L’accent sert à la sélection et aux actions, sans remplacer le texte courant.
- Matières : verre fumé pour les petites cartes et le dock, surface presque opaque pour les réglages. Contour clair discret, une seule ombre douce. Implémentation partagée : `src/ui/design.rs`.
- Typographie : Mindlog 0.7 par défaut, embarquée et livrée avec le setup (SIL OFL 1.1) ; cinq graisses de Light à Bold. Choix parmi les familles installées dans Réglages → Bureau. Une famille pour toute l’interface ; poids normal, medium et semibold pour la hiérarchie. Échantillon et taille modifiables en direct.
- Icônes : thèmes système pour les applications ; icônes symboliques pour les commandes. Secours vectoriels sur une grille 24 px, trait arrondi 1,7 px, dans `src/ui/icons.rs`. Aucun glyphe de police obligatoire.
- Espacement : base 4 px, petits groupes 8–12 px, sections 20–24 px. Rayons 10 px pour les commandes, 20 px pour les cartes, 24 px pour le panneau.
- Mouvement : retour visuel au pressage, survol discret, apparition courte sans rebond décoratif. Toujours repartir de la valeur affichée lors d’une interruption. `transition = "none"` désactive les mouvements ajoutés.
- Lanceur : verre fumé légèrement éclairé en haut, contour fin et ombre courte ; flou fourni par Hyprland sur les surfaces du dock et de ses bulles. Survol continu à 140 ms maximum, influence atténuée sur deux voisins, agrandissement de 9 % maximum et levée de 2,5 px. Les boîtes de clic restent fixes ; les épingles conservent leur rang à l’ouverture d’une fenêtre. Point pour une application ouverte, trait iris pour la fenêtre active, indicateurs orientés vers le bord de l’écran.
- Disposition : garder les widgets bas au-dessus du dock ; limiter le dock à la largeur disponible. Navigation des réglages réduite aux icônes sur petits écrans. Menus longs défilants et bornés à la hauteur de l’écran.

## Vérification visuelle

`hyprdesk preview <dossier>` capture les vues réelles avec des données de démonstration, sans IPC Hyprland, persistance ni lancement de programmes. Les images couvrent le bureau, les six onglets, le volume et le calendrier. Utiliser `scripts/verify.sh --visual` pour les largeurs 1440, 900 et 640 px, puis inspecter les captures avant validation.

Le lanceur dispose aussi des scènes `launcher-idle`, `launcher-hover`, `launcher-top`, `launcher-left`, `launcher-right` et `launcher-popup`. `HYPRDESK_PREVIEW_SCENES=launcher-idle,launcher-hover` limite les captures aux scènes choisies. L’aperçu reproduit les zones réservées ; le flou du compositeur doit être évalué en session Hyprland.

Dans l’aperçu interactif, F12 enregistre le rendu Iced dans `/tmp/hyprdesk-preview.png`, y compris pendant un survol ou une pression.

Les menus utilisent la mesure de leur contenu rendu pour ajuster leur fenêtre : ne pas ajouter de marge de sécurité estimée à la hauteur finale. Le calendrier occupe toute la largeur intérieure de sa carte.
