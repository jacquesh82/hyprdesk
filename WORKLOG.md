# Boucle autonome — hyprdesk

Objectif : corriger les problèmes de l’audit et harmoniser bureau, barre, menus et réglages, avec choix de police.

Boucle : implémenter → tests ciblés → compilation/Clippy → aperçu visuel isolé → corriger → documenter.

- [x] Base : journal conservé après échec, écritures atomiques ordonnées, acteur des fenêtres, relances séquentielles.
- [x] Limites : commandes bornées en durée/mémoire, groupe enfant arrêté, budget Lua, file bornée, entrées numériques finies.
- [x] Ordonnanceur : un relevé par widget, génération des résultats ; serveur IPC concurrent borné.
- [x] Première passe visuelle : surfaces partagées, navigation latérale, icônes SVG de secours, sélection des polices installées et taille.
- [x] Terminer les transitions, le dock, les cartes, le contraste et les petits écrans.
- [x] Finir les régressions (journal IPC, relance lente, Lua, police, concurrence, menus périmés).
- [x] Vérifier et durcir le watcher natif, les requêtes D-Bus et les dépendances CLI.
- [x] Mettre en cohérence LuaLS, README, licence, version Rust et CI.
- [x] Faire passer fmt, Clippy strict, tests et build.
- [x] Capturer et inspecter visuellement dans l’aperçu isolé et sous Hyprland ; corriger les défauts observés.

Contraintes : préserver les modifications locales préexistantes, installation et redémarrage autorisés pour corriger les problèmes du démon actif. Les essais de processus et IPC utilisent /tmp.

Résultats : fmt et Clippy strict passent ; 65 tests unitaires passent, plus le test D-Bus sur bus isolé. Build release validé. Captures contrôlées à 1440, 900 et 640 px, puis calendrier mesuré sous Hyprland (336 × 277 px).

Corrections des retours utilisateur : raccourcis Hyprshot universels dans hyprland.lua ; copies bar:* retirées de settings.lua, migration et sérialisation protégées ; compteurs d’IDs de bureau indépendants de la barre ; placements conservés lors de la lecture autonome de settings.lua ; dimensionnement des menus d’après le contenu rendu.

Sauvegardes de la session : /tmp/hyprdesk-capture-fix et /tmp/hyprdesk-settings-repair. Captures : /tmp/hyprdesk-visual et /tmp/hyprdesk-settings-repair/calendar-native.png.

Police Mindlog 0.7 : cinq TTF originaux et licence OFL intégrés dans assets/fonts/mindlog ; chargement embarqué au démarrage du démon et de l’aperçu, famille par défaut pour toutes les surfaces. Le setup installe aussi les cinq graisses et la licence dans le dossier de polices utilisateur, puis actualise Fontconfig. Installation réelle et redémarrage validés ; fc-match sélectionne Mindlog Regular. Clippy strict, 72 tests et build release passent ; rendu typographique inspecté dans /tmp/hyprdesk-mindlog-preview/typography.png.
