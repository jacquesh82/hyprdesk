# Composer un bandeau Panorama

Les réglages sont dans **Réglages → Panorama** :

- **Style** : quatre compositions de départ, image de fond, couleurs, police,
  opacité des cartes et contours, arrondis, espacement, marges et taille du texte.
- **Composition** : activer la composition libre, ajouter ou retirer des zones,
  choisir leur widget et régler position et dimensions en pourcentage.
- **Raccourcis** : grille tactile, nombre de colonnes, nom, icône et commande de
  chaque touche. Les commandes ne sont exécutées qu'au clic ou au toucher.
- **Pages** : pages classiques et dispositions partagées ; sources vidéo et veille.
- **Écrans** : attribution du rôle, tactile et fenêtre Panorama interactive.
- **Modules** : modules de la bande du profil Raccourcis, réglages de Hue et de
  Notes.

## Modules

Panorama est un plugin de hyprdesk ; ses **modules** sont ce que la bande du
profil Raccourcis montre côte à côte (elle défile de côté, la tuile « + »
ajoute ou retire un module) :

| Module | Widget | Rôle |
| --- | --- | --- |
| Notes | `notes` | Lance l'éditeur réglé sur le workspace actif de l'écran choisi |
| Lumières | `hue` | Pont Philips Hue local : pièces, lampes, curseurs, « Tout éteindre » |
| LYRA vocale | `voice` | Micro de LYRA : ouvrir, couper, dernière phrase entendue |
| Musique | `media` | Lecture en cours (MPRIS) |
| Plateformes vidéo | `playlist` | Services du lecteur |
| Horloge, Calendrier, Système | `clock`, `calendar`, `system` | Widgets du Panorama |

Un média en cours (vidéo, radio, musique) se loge sous le sélecteur de
profils, à la place des touches. Le pont Hue s'appaire dans Réglages →
Panorama → Modules (bouton du pont, puis « Appairer ») ; sa clé reste dans
`$XDG_STATE_HOME/hyprdesk/hue.json` (0600). Options des modules :
`plugin_options.panorama`, clés `hue.hidden` (pièces masquées),
`notes.command`, `notes.monitor`.

Code : logique dans `src/panorama/modules/`, tuiles dans
`src/ui/plugins/panorama/surface/modules/`, réglages dans
`src/ui/plugins/panorama/modules.rs`.

## Compositions de départ

| Préréglage | Composition | Style |
| --- | --- | --- |
| `forest` | Raccourcis, courbes système, horloge à aiguilles | Vert sombre, accent menthe ; choisir votre photo dans Image de fond |
| `telemetry` | Horloge à volets, cadrans système, calendrier, lecteur compact | Fond lumineux bleu/violet, accent jaune |
| `studio` | Grande horloge, pochette et vinyle, grille de raccourcis | Bleu/violet, zones sans contour |
| `applications` | Source web/vidéo et lecteur compact | Surface sombre, zones larges |

Appliquer un préréglage remplace sa composition et ses réglages de style, en
conservant l'image choisie et les raccourcis personnels. Les douze raccourcis de
départ ouvrent les fonctions existantes de hyprdesk ; ils sont modifiables.

La zone Vidéo héberge une fenêtre Chromium, avec les services et le profil du
navigateur configurés dans Pages. Les applications natives comme OBS restent
des fenêtres gérées par Hyprland.

## Styles des widgets

- Horloge : `digital`, `flip`, `analog`, avec secondes et date indépendantes.
- Système : `cards`, `gauges`, `graphs`. Mesures : `cpu`, `ram`, `disk`, `load`,
  `uptime`, `cpu_temp`, `gpu_temp`. Les températures viennent de hwmon ; un
  capteur absent affiche **—**. Aucun chiffre n'est inventé. Les courbes
  conservent 15 à 600 relevés d'une seconde lorsque Panorama est affiché.
- Média : `standard`, `vinyl`, `compact`. Le lecteur et la pochette viennent de
  MPRIS ; les boutons pilotent le lecteur en cours.
- Calendrier : mois courant, date du jour mise en évidence.
- Raccourcis : 2 à 8 colonnes, jusqu'à 64 touches.
- `widget:<identifiant>` : instance déjà configurée dans le catalogue de widgets,
  y compris un widget Lua personnel. Sa source et son intervalle restent ceux
  de cette instance ; Panorama partage ses données et ses actions.

## Configuration Lua

```lua
hd.config({ panorama = {
  screens = { "DP-3" },
  layout = "dashboard",
  design = {
    preset = "studio",
    wallpaper = "~/.wallpapers/bandeau.jpg",
    dimming = 0.35,
    background = "#111032",
    foreground = "#f1f5ff",
    accent = "#a4aeff",
    font_family = "DejaVu Sans",
    card_opacity = 0.05,
    border_opacity = 0,
    radius = 18,
    gap = 12,
    padding = 20,
    text_scale = 1,
    effects = false,
    controls = false,
    clock_style = "digital",
    seconds = true,
    date = true,
    system_style = "graphs",
    metrics = "cpu,ram,cpu_temp,gpu_temp",
    history_seconds = 120,
    media_style = "vinyl",
    shortcut_columns = 4,
    tiles = {
      { widget = "clock", x = 0,    y = 0, w = 0.25, h = 1 },
      { widget = "media", x = 0.27, y = 0, w = 0.46, h = 1 },
      { widget = "shortcuts", x = 0.75, y = 0, w = 0.25, h = 1 },
    },
    shortcuts = {
      { name = "Fichiers", icon = "system-file-manager-symbolic", command = "hyprdesk files" },
      { name = "Lecture", icon = "media-playback-start-symbolic", command = "hyprdesk media play-pause" },
    },
  },
} })
```

Les coordonnées des zones vont de 0 à 1 et restent dans le bandeau. Une zone
mesure au moins 5 % sur chaque axe ; la composition contient 1 à 16 zones.
Le portrait inverse les axes de composition en gardant les widgets droits.
Les zones peuvent se superposer : la dernière apparaît au-dessus. Les marges
et espacements sont exprimés en pixels logiques. Une couleur ou police vide
hérite du thème. L'image est cadrée en conservant ses proportions.

### Voile autour de la vidéo

Quand une vidéo joue dans la zone Vidéo, un voile sombre couvre le reste de
la surface en fondu ; la fenêtre vidéo, sous son trou, n'est jamais voilée.
La souris sur le Panorama l'atténue, il revient quand elle part (la fenêtre
vidéo reçoit le pointeur : la survoler compte comme partir).

```lua
hd.config({ panorama = {
  video_dim = {
    on = true,          -- défaut
    level = 55,         -- intensité en % (10..90)
    scope = "all",      -- all : fond et zones | background : fond seul
    paused = false,     -- garder le voile en pause
    fade = 600,         -- durée du fondu, ms (0..3000)
    hover = 35,         -- % du voile gardé sous la souris (100 : inchangé)
  },
} })
```

Réglages → Panorama → Lecteur → Vidéo, ou `hyprdesk config set
panorama.video_dim.<champ>`. Peint par `fx::Stage` sur le complément des
trous vidéo (`ui/plugins/panorama/surface/fx.rs`).

`controls = false` masque les commandes superposées du bandeau ; les réglages
restent accessibles depuis le lanceur et `hyprdesk settings panorama`.
`effects = false` enlève le décor géométrique ; `transition = "none"` dans la
configuration générale désactive également les mouvements d'interaction.

## Ligne de commande

```sh
hyprdesk config set panorama.design.preset studio
hyprdesk config set panorama.design.clock_style analog
hyprdesk config set panorama.design.wallpaper ~/.wallpapers/foret.jpg
hyprdesk config set panorama.design.card_opacity 0.15
hyprdesk config set panorama.design.metrics cpu,ram,cpu_temp,gpu_temp
hyprdesk config set panorama.design.system_style graphs
hyprdesk config set panorama.design.tiles '[{"widget":"clock","x":0,"y":0,"w":0.4,"h":1},{"widget":"system","x":0.42,"y":0,"w":0.58,"h":1}]'
hyprdesk panorama preview 1920x540
```

Toutes les propriétés de `design` sont exposées par `hyprdesk config list
panorama`. Les tableaux `tiles` et `shortcuts` utilisent du JSON en CLI.
Une entrée incorrecte est refusée avant l'écriture de la configuration.

## Vérification visuelle isolée

Les scènes `panorama-forest`, `panorama-telemetry`, `panorama-studio`,
`panorama-applications`, `panorama-video-veil` (vidéo posée et voile),
`panorama-config-style`, `panorama-config-composition`,
`panorama-config-shortcuts` et `panorama-config-video` utilisent les vrais rendus avec des données de
démonstration, sans lancer les raccourcis ni les applications.

```sh
HYPRDESK_PREVIEW_SCENES=panorama-studio,panorama-telemetry \
HYPRDESK_PREVIEW_WIDTH=1920 HYPRDESK_PREVIEW_HEIGHT=540 \
hyprdesk preview /tmp/panorama-design
```
