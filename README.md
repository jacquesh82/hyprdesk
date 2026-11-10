# hyprdesk

**Un bureau à la demande pour Hyprland.** En tuiles, les fenêtres couvrent
l'écran et le bureau n'existe plus. hyprdesk le rend visible d'une touche, par-dessus
les fenêtres, avec un fond d'écran, des icônes et des widgets. Chaque workspace
peut avoir son propre bureau. La configuration est en Lua, dans le style de
`hyprland.lua` (Hyprland ≥ 0.57).

- **Au repos**, hyprdesk dessine le fond d'écran du workspace affiché sur le
  calque `background`. Il remplace hyprpaper, qui ne connaît qu'un fond par écran.
- **Appelé** (SUPER + D), il agit comme « afficher le bureau » :
  - les fenêtres du workspace sont mises de côté (workspace spécial
    `special:hyprdesk`) ;
  - le bureau, resté **sous** les fenêtres, apparaît avec ses icônes et ses
    widgets ;
  - ce qu'on ouvre depuis le bureau s'ouvre **dessus**, flottant et centré
    (`float_launched`) : le bureau contient ces fenêtres ;
  - masqué, il rend les fenêtres mises de côté, à côté de celles ouvertes
    entre-temps.

  La liste des fenêtres mises de côté est gardée sur disque : après un arrêt
  brutal, hyprdesk les rend au démarrage suivant.
- **Mode `glass`** : le bureau passe au-dessus des fenêtres, en voile
  translucide, sans les déplacer. Les fenêtres restent floutées derrière.
- **Dock** en bas au centre : les fenêtres ouvertes, si l'option est activée
  (un clic donne le focus à la fenêtre), puis le bouton ⚙ des réglages.

## Réglages (⚙)

Le bouton ⚙ du dock, ou `hyprdesk settings [bureau|icones|fond|effets]`, ouvre
un panneau par-dessus le bureau, sur l'écran actif. Il a quatre onglets :

| Onglet | Réglages |
|---|---|
| Bureau | bureau sur tous les écrans ou l'écran actif ; fermer après un lancement ; mode opaque ou verre, par défaut et pour ce workspace ; opacité du voile ; raccourcis (suspension, touches, conflits, raccourcis universels) |
| Icônes | fenêtres ouvertes dans le dock (non, ce workspace, toutes) ; raccourcis vers des applications installées (partout ou sur ce workspace) ; taille des icônes et des cases ; thème d'icônes ; remise à zéro des positions |
| Fond d'écran | portée (tous les bureaux, ce workspace, cet écran) ; image choisie dans un dossier (vignettes), diaporama (intervalle, ordre aléatoire) ou couleur unie ; ajustement |
| Widgets | widgets disponibles (intégrés et plugins détectés), activation à l'unité ; placement des widgets du bureau : ancrage 3×3, décalages, taille, carte ; glisser-déposer sur le bureau |
| Effets | transition (fondu, glissement, aucune) et sa durée ; fondu entre deux fonds ; couleur d'accent |

Tout s'applique **en direct**. Le panneau est un tiroir à droite, et le bureau
reste visible et utilisable à côté.

## Widgets plug-n-play

Les widgets intégrés sont `clock`, `calendar`, `system` et `media`, plus
`command` et `lua`, qui se déclarent dans le Lua. Pour en ajouter un autre, il
suffit de déposer un fichier Lua dans `~/.config/hypr/hyprdesk/widgets/`, sans
rien recompiler :
- le fichier est détecté aussitôt ;
- il apparaît dans Réglages → Widgets, où on l'active d'un clic ;
- on peut aussi l'utiliser en Lua avec `hd.widget("<nom>", { … })`.

```lua
-- ~/.config/hypr/hyprdesk/widgets/meteo.lua
return {
    name = "Météo", description = "wttr.in", interval = 900, anchor = "top-left",
    options = { ville = "Paris" },                       -- valeurs par défaut
    render = function(o)
        return hd.exec("curl -s 'wttr.in/" .. o.ville .. "?format=3'")
    end,
}
```

Emplacements des plugins :
- un fichier `<nom>.lua`, ou un dossier `<nom>/init.lua` ;
- cherchés aussi dans `~/.local/share/hyprdesk/widgets` et
  `/usr/share/hyprdesk/widgets` ; le premier dossier trouvé l'emporte ;
- `hyprdesk init` installe deux exemples : `meteo` et `notes`.

Un plugin en erreur est signalé dans les réglages sans bloquer le reste.

Dans Réglages → Widgets, chaque interrupteur reflète l'état réel du bureau,
que le widget vienne des réglages ou de `hyprdesk.lua`. Éteindre un widget de
`hyprdesk.lua` écrit `hd.disable("clock#0")` dans `settings.lua`, sans
modifier votre fichier. Le rallumer retire cette ligne.

Pour repositionner un widget déclaré dans `hyprdesk.lua`, on utilise
`hd.place("clock#0", { anchor = "top", y = 20 })`. Quand on le déplace depuis
les réglages, c'est ce `hd.place` qui est écrit dans `settings.lua`.

Les réglages sont enregistrés en Lua généré, dans
`~/.config/hypr/hyprdesk/settings.lua`, comme le fait hyprdmc pour
`monitors.lua`. Ce fichier est chargé **avant** `hyprdesk.lua` : tout ce que vous
écrivez à la main garde le dernier mot. Votre fichier n'est jamais réécrit, et le
panneau signale les réglages que `hyprdesk.lua` surcharge.

## Installation

Il faut Rust (édition 2024), Hyprland 0.57+ et un pilote Vulkan ou GL.

```bash
./scripts/install.sh
```

Le script :
- compile `hyprdesk` et le place dans `~/.local/bin` ;
- écrit `~/.config/hypr/hyprdesk.lua` s'il n'existe pas, ainsi que les
  définitions LuaLS ;
- active le service systemd utilisateur.

Il ne modifie pas `hyprland.lua`. Les lignes recommandées s'obtiennent avec
`hyprdesk snippet` : la règle de flou du mode verre, et le marquage
`submap_universal` des raccourcis à garder sur le bureau (voir ci-dessous).

## Raccourcis

- **Les raccourcis de hyprdesk sont installés par hyprdesk lui-même**, avec
  ceux de `hd.config({ keys = … })` ou de Réglages → Bureau :
  - `toggle`, `SUPER + D` par défaut, affiche ou masque le bureau ;
  - `settings` ouvre les réglages ;
  - `leave`, `SUPER + Escape` par défaut, est la sortie de secours.

  Ils sont réinstallés après chaque `hyprctl reload`.
- **Pas de conflit avec Hyprland.** Avant d'installer un raccourci, hyprdesk
  vérifie qu'il n'est pas déjà lié. S'il l'est, il ne l'installe pas et le
  signale par une notification et dans les réglages.
- **Bureau affiché, les raccourcis de Hyprland sont suspendus.** hyprdesk
  active la submap `hyprdesk` : SUPER + Q, SUPER + F, etc. ne visent plus
  les fenêtres. Dès qu'une fenêtre est ouverte depuis le bureau, ils sont
  rendus pour agir sur elle. Pendant la suspension, restent actifs :
  - les raccourcis de hyprdesk ;
  - ceux que vous marquez `{ submap_universal = true }` dans `hyprland.lua`,
    typiquement le changement de workspace, et le bureau suit alors.

  `suspend_binds = false` désactive ce comportement.
- **On ne reste jamais bloqué.** Les raccourcis de la submap rendent la main
  à Hyprland *avant* d'appeler hyprdesk. Au démarrage, hyprdesk referme une
  submap laissée ouverte par un arrêt brutal.

Pensez aussi à retirer `hyprpaper` de l'autostart.

## Une seule instance

Un seul démon tourne par session : il tient un verrou exclusif (`flock`) sur
`$XDG_RUNTIME_DIR/hyprdesk.lock` pendant toute sa vie.
- **Seconde instance** : qu'elle vienne du service, d'un `hl.exec_cmd` ou d'un
  lancement manuel, elle signale celle en place (son PID) et s'arrête sans
  erreur. systemd ne la relance donc pas en boucle.
- **Remplacement** : `hyprdesk daemon --replace` remplace proprement
  l'instance en place. Le service systemd démarre ainsi.
- **Arrêt propre** : sur `SIGTERM` (`systemctl --user stop`) ou `SIGINT`,
  hyprdesk rend les fenêtres mises de côté et retire ses raccourcis.
- **Arrêt brutal** : le noyau libère le verrou ; au démarrage suivant,
  hyprdesk rend les fenêtres restées de côté.

## Commandes

| Commande | Effet |
|---|---|
| `hyprdesk` / `hyprdesk daemon` | lance le démon (une seule instance par session) |
| `hyprdesk daemon --replace` | remplace proprement l'instance en place |
| `hyprdesk toggle` · `show` · `hide` | affiche / masque le bureau |
| `hyprdesk settings [onglet]` | affiche le bureau et ouvre les réglages (`bureau`, `icones`, `fond`, `effets`) |
| `hyprdesk reload` | relit la configuration (elle est aussi relue à chaque enregistrement) |
| `hyprdesk status` | état : affiché ou caché, workspaces, config en erreur ou non |
| `hyprdesk check [fichier]` | vérifie une configuration sans toucher au démon |
| `hyprdesk init` · `stubs` · `snippet` | config d'exemple, définitions LuaLS, lignes pour `hyprland.lua` |

Options et variables :
- `--config <fichier>` ou `$HYPRDESK_CONFIG` : fichier de configuration à utiliser ;
- `$HYPRDESK_LOG` : niveau de journal (`debug`, `info`…).

## Configuration — `~/.config/hypr/hyprdesk.lua`

```lua
hd.config({ mode = "opaque", show_on = "all", running_apps = "workspace", icon_theme = "Papirus-Dark",
            transition = "slide", fade_ms = 150, crossfade_ms = 700,
            grid = { cell = 96, gap = 16, margin = 32, icon_size = 48 },
            theme = { fg = "#f8f8f2", bg = "#22212c", accent = "#9580ff" } })

hd.wallpaper({ path = "~/.wallpapers/default.jpg" })                       -- partout
hd.wallpaper({ monitor = "DP-1", path = "~/.wallpapers/portrait.png" })    -- un écran
hd.wallpaper({ workspace = 3, rotate = { dir = "~/.wallpapers", interval = 600, shuffle = true } })
hd.desktop({ workspace = 4, mode = "glass" })
hd.desktop({ workspace = "special:magic", inherit = false })               -- sans les éléments globaux

hd.icon({ app = "firefox" })                                               -- identifiant .desktop
hd.icon({ workspace = 2, label = "Projets", path = "~/Documents/DEV", pos = { 0, 1 } })
hd.icon({ label = "htop", exec = "htop", terminal = true, icon = "utilities-system-monitor" })

hd.widget("clock",    { anchor = "top-right", format = "%H:%M", size = 72, card = false })
hd.widget("calendar", { anchor = "right" })
hd.widget("system",   { anchor = "bottom-right", show = { "cpu", "ram", "disk:/", "uptime" } })
hd.widget("media",    { anchor = "bottom", player = "*" })
hd.widget("command",  { exec = "curl -s 'wttr.in/?format=3'", interval = 900 })
hd.widget("lua",      { interval = 5, render = function() return os.date("%A") end })

hd.on("show", function(e) print(e.monitor, e.workspace) end)               -- show, hide, workspace
```

**Sélecteurs.** Chaque règle accepte `workspace` et `monitor` (une chaîne, un
nombre, une liste, ou `"*"`). Les règles se combinent ainsi :
- pour le fond d'écran et le mode, la règle la plus précise l'emporte
  (workspace + écran > workspace > écran > tous) ; à égalité, la dernière
  déclarée gagne ;
- les icônes et les widgets s'additionnent.

**Erreurs.** Hyprdesk réagit comme Hyprland face à une erreur :
- un champ inconnu, une valeur invalide ou une erreur Lua produisent une
  notification Hyprland qui indique `fichier:ligne` ;
- la dernière configuration valide reste active.

**Modules.** `require("x")` charge `~/.config/hypr/hyprdesk/x.lua`. Ces modules
sont surveillés eux aussi.

**Disposition.** Les icônes se placent colonne par colonne. Une icône déplacée à
la souris garde sa place, par écran et par workspace, dans
`~/.local/state/hyprdesk/layout.toml`. Le Lua n'est jamais réécrit.

**Clavier.** Les flèches sélectionnent, Entrée lance, Échap ferme le panneau
de réglages, puis le bureau. Les raccourcis
Hyprland restent actifs : changer de workspace bureau ouvert affiche le bureau de
ce workspace.

**Lancement.** Les programmes sont lancés par Hyprland
(`hl.dsp.exec_cmd`) : ils lui appartiennent, et ses règles d'exec s'appliquent.

## Développement

```bash
cargo test                          # config Lua, résolution, placement, IPC, .desktop
cargo run -- --config exemple.lua   # démon avec une config de test
```

`Cargo.lock` épingle `winit-core` en `0.31.0-beta.2`, parce que la beta.4 casse
`iced_exdevtools`, une dépendance d'`iced_layershell` 0.19. Il faut donc
compiler avec `--locked`.

L'IPC Hyprland vient de `hyprpanel-core`
([mindlog.hyprland_tools](https://github.com/jacquesh82/mindlog.hyprland_tools)),
épinglé sur un commit.

Licence MIT.
