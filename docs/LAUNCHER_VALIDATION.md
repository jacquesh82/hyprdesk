# Validation du launcher

Seconde passe : apparition du dock et des bulles, temporisation annulable,
ancrage sur les limites rendues du bouton, fermeture réversible, signal de
lancement borné et contraste adapté au fond. Les couleurs et l’opacité
minimale du thème restent prises en compte.

## Vérifications de comportement

`cargo test --locked ui::plugins::launcher` couvre 13 tests : regroupement,
survol continu interrompu, isolation entre écrans, mouvement désactivé,
annulation des ouvertures et fermetures périmées, reprise d’une fermeture,
limites des menus, origine de leur expansion après contrainte aux bords,
fenêtre nouvellement créée, expiration et échec d’un lancement, contraste et
ajustement des images de fond.

La résolution de l’entrée `.desktop` se fait dans un thread de travail. Les
résultats de résolution, d’envoi et d’expiration portent un jeton propre au
lancement ; un ancien résultat ne peut pas terminer une nouvelle tentative.

## Rendu et interactions natifs

Essai dans Hyprland 0.57.2 isolé, avec un écran virtuel de 1280 × 800 à
144 Hz, rendu par une instance Wayland imbriquée. L’application de test et
le pointeur virtuel appartiennent à cette instance. Le démon de la session
principale n’est pas remplacé.

- Ouverture après le délai de survol, puis fermeture après sortie.
- Position de la bulle inchangée pendant quatre déplacements du curseur à
  l’intérieur du bouton : x = 531, y = 663 pixels logiques.
- Lancement réel d’une fenêtre GTK de test et arrêt du signal d’attente.
- Quatre bords de l’écran ; échelles natives 1×, 1,25× et 2×.
- Flou activé puis désactivé sur un fond à damier de 4 px : différence
  moyenne d’environ 12 niveaux RGB sur la zone de carte comparée.
- Sur 270 horodatages de callbacks distincts pendant les déplacements :
  intervalle médian 7 ms, 95e percentile 13 ms. Traitement des messages UI :
  95e percentile 64 µs. Ce sont les mesures de cet essai imbriqué, pas une
  garantie de fréquence sur tout matériel.
- Aucun message d’animation pendant une seconde au repos après stabilisation.
  Les relevés périodiques normaux du bureau continuent.

Rapport de cet essai : `/tmp/hyprdesk-native-qa/report.json`. Captures :
`/tmp/hyprdesk-native-qa/captures/`.

## Aperçus à reproduire

Les captures utilisent les mêmes vues et conservent les dimensions logiques :

```sh
HYPRDESK_PREVIEW_WIDTH=900 HYPRDESK_PREVIEW_SCALE=1.25 \
HYPRDESK_PREVIEW_SCENES=launcher-idle,launcher-opening,launcher-menu-opening,launcher-launching,launcher-bright,launcher-dark,launcher-menu-long \
target/debug/hyprdesk preview /tmp/launcher-125
```

Vérifications supplémentaires à 1,5× et 2× : contours, indicateurs,
quatre positions et longue liste défilante. Dans l’aperçu interactif,
F12 enregistre le rendu courant dans `/tmp/hyprdesk-preview.png`.
