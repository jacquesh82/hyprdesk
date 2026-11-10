# hyprdesk

**Un bureau à la demande pour Hyprland.** En tuiles, les fenêtres couvrent
l'écran et le bureau n'existe plus. hyprdesk le rend visible d'une touche, par-dessus
les fenêtres, avec un fond d'écran, des icônes et des widgets. Chaque workspace
peut avoir son propre bureau. La configuration est en Lua, dans le style de
`hyprland.lua` (Hyprland ≥ 0.57).

- **Au repos**, hyprdesk dessine le fond d'écran du workspace affiché sur le
  calque `background`. Il remplace hyprpaper, qui ne connaît qu'un fond par écran.
- **Appelé** (`hyprdesk toggle`), il passe sur le calque `top` en fondu :
  au-dessus des fenêtres, sous la barre. Il prend le clavier et affiche icônes
  et widgets. Échap, la même touche ou un lancement le renvoient.
- **Mode `glass`** : au lieu du fond d'écran, un voile translucide, avec les
  fenêtres floutées par Hyprland.
- **Dock** en bas au centre : les fenêtres ouvertes, si l'option est activée
  (un clic donne le focus à la fenêtre), puis le bouton ⚙ des réglages.

## Réglages (⚙)

Le bouton ⚙ du dock, ou `hyprdesk settings [bureau|icones|fond|effets]`, ouvre
un panneau par-dessus le bureau, sur l'écran actif. Il a quatre onglets :

| Onglet | Réglages |
|---|---|
| Bureau | bureau sur tous les écrans ou l'écran actif ; fermer après un lancement ; mode opaque ou verre, par défaut et pour ce workspace ; opacité du voile |
| Icônes | fenêtres ouvertes dans le dock (non, ce workspace, toutes) ; raccourcis vers des applications installées (partout ou sur ce workspace) ; taille des icônes et des cases ; thème d'icônes ; remise à zéro des positions |
| Fond d'écran | portée (tous les bureaux, ce workspace, cet écran) ; image choisie dans un dossier (vignettes), diaporama (intervalle, ordre aléatoire) ou couleur unie ; ajustement |
| Effets | transition (fondu, glissement, aucune) et sa durée ; fondu entre deux fonds ; couleur d'accent |

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

Il ne modifie pas `hyprland.lua`. Les lignes à y ajouter s'obtiennent avec
`hyprdesk snippet` :

```lua
hl.bind(mainMod .. " + D", hl.dsp.exec_cmd("hyprdesk toggle"))

-- flou du mode glass (xray = false : flouter les fenêtres, pas le seul fond)
hl.layer_rule({ name = "hyprdesk-blur", match = { namespace = "^hyprdesk$" },
                blur = true, ignore_alpha = 0.05, xray = false })
```

Pensez aussi à retirer `hyprpaper` de l'autostart.

## Commandes

| Commande | Effet |
|---|---|
| `hyprdesk` / `hyprdesk daemon` | lance le démon |
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
