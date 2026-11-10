# hyprdesk
*[English version](README.en.md)*

**Un bureau à la demande pour Hyprland.** En tuiles, les fenêtres couvrent
l'écran et le bureau n'existe plus. hyprdesk le rend visible d'une touche,
avec un fond d'écran, des icônes et des widgets. Chaque workspace
peut avoir son propre bureau. La configuration est en Lua, dans le style de
`hyprland.lua` (Hyprland ≥ 0.57).

hyprdesk est la **racine** de la session graphique.
- **Racine visuelle.** Le bureau est toujours *sous* toutes les fenêtres,
  jamais par-dessus.
  - Au repos, il dessine le fond d'écran du workspace affiché, et remplace
    hyprpaper.
  - Appelé (SUPER + D), il met de côté les fenêtres du workspace
    (`special:hyprdesk`) et apparaît avec ses icônes et ses widgets. Ce
    qu'on ouvre depuis lui s'ouvre **dessus**, flottant et centré.
  - Masqué, il rend les fenêtres mises de côté.
  - Le mode `glass` est un effet sur le fond : fond d'écran flouté et teinté.
- **Racine de session.** hyprdesk lance et supervise les composants déclarés
  avec `hd.component` (ex. `wayle shell`). Il les relance s'ils plantent et
  les arrête avec lui.
- **Barre native.** `hd.bar` affiche une barre au-dessus des fenêtres, avec
  sa zone réservée. Les fenêtres et la grille du bureau se rangent à côté.
  - Ses modules sont des widgets : workspaces, fenêtre active, volume,
    bouton bureau, et tout widget ou plugin en version compacte.
  - Elle se règle aussi dans Réglages → Barre.
- **Dock** en bas au centre : les fenêtres ouvertes, si l'option est activée
  (un clic donne le focus à la fenêtre), puis le bouton ⚙ des réglages.

## Réglages (⚙)

Chaque entrée des réglages est un **paquet intégré** (manifeste, messages,
état, vue, clés CLI) ; conception des paquets, des widgets et du lanceur :
[`docs/PLUGINS.md`](docs/PLUGINS.md).

Le bouton ⚙ du dock, ou `hyprdesk settings [bureau|icones|fond|effets]`, ouvre
un panneau par-dessus le bureau, sur l'écran actif. Il regroupe les réglages suivants :

| Onglet | Réglages |
|---|---|
| Bureau | bureau sur tous les écrans, l'écran actif ou l'écran principal ; fermer après un lancement ; fenêtres ouvertes flottantes ; mode opaque ou verre, par défaut et pour ce workspace ; opacité du voile ; raccourcis (suspension, touches, conflits, raccourcis universels) ; session (composants, état, relance) |
| Icônes | raccourcis vers des applications installées (partout ou sur ce workspace) ; taille des icônes et des cases ; thème d'icônes ; remise à zéro des positions |
| Lanceur | dock du bureau : fenêtres affichées (aucune, ce workspace, toutes) ; regroupement (aucun, par application, par workspace) |
| Fond d'écran | portée (tous les bureaux, ce workspace, cet écran) ; image choisie dans un dossier (vignettes), diaporama (intervalle, ordre aléatoire) ou couleur unie ; ajustement |
| Barre | afficher, position, hauteur, écrans (tous, principal, un seul) ; modules par zone (gauche, centre, droite) : ajouter, ordonner, changer de zone, retirer |
| Widgets | widgets disponibles (intégrés et plugins détectés), activation à l'unité ; placement des widgets du bureau : ancrage 3×3, décalages, taille, carte ; glisser-déposer sur le bureau |
| Écrans | écrans détectés (désactivés compris) sur un plan à l'échelle ; position par rapport à un voisin et alignement ; orientation et miroir ; résolution et fréquence ; échelle ; activation ; écran principal ; disposition automatique — chaque changement s'applique à chaud, puis se garde (écrit dans `monitors.lua`) ou revient seul au bout de 12 s |
| Workspaces | nombre de workspaces permanents (ou laissés à Hyprland) ; affectation d'une application à un workspace, avec ou sans y aller |
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

Un plugin peut faire plus que du texte. C'est ce qui permet d'ajouter à la
barre tout ce qui manque, de façon classique :

```lua
return {
    name = "VPN", description = "État du VPN", interval = 5, anchor = "top-right",
    options = { interface = "wg0" },
    -- texte, ou table : icône (de préférence « …-symbolic », teintée selon le thème),
    -- info-bulle, état (normal, active, warning, critical, hidden)
    render = function(o)
        local up = hd.exec("ip link show " .. o.interface .. " 2>/dev/null") ~= ""
        return { icon = up and "network-vpn-symbolic" or "network-vpn-disabled-symbolic",
                 tooltip = up and "VPN actif" or "VPN coupé", state = up and "active" or "normal" }
    end,
    on_click  = function(o) end,              -- clic (clic droit si un menu existe)
    on_scroll = function(o, sens) end,        -- molette : +1 / -1
    menu = function(o)                        -- menu déroulant sous le module
        return {
            { header = "VPN" },
            { label = "Connecter", icon = "network-vpn-symbolic", action = function() hd.spawn("wg-quick up wg0") end },
            "-",
            { label = "Tout couper…", confirm = "Couper le VPN ?", action = function() hd.exec("wg-quick down wg0") end },
        }
    end,
}
```

- **`hd.exec(cmd)`** rend la sortie d'une commande.
- **`hd.spawn(cmd)`** lance un programme sans attendre, par Hyprland.
- **Entrées de menu** : `label`, `detail`, `icon`, `checked`, `disabled`,
  `action` et `confirm`. Une entrée avec `confirm` demande une confirmation
  dans le menu même.

**Plugins livrés.** Ils sont intégrés au binaire, et un fichier du même nom
les remplace :

| Plugin | Rôle |
|---|---|
| `reseau` | filaire et Wi-Fi, réseaux à proximité, connexion, activation du Wi-Fi |
| `bluetooth` | activation, appareils appairés (connexion, déconnexion) |
| `batterie` | batterie de l'ordinateur et des périphériques (souris, clavier) ; masqué s'il n'y en a pas |
| `notifications` | Ne pas déranger, dernières notifications, tout effacer (service de wayle) |
| `session` | verrouiller, veille, déconnexion, redémarrage, extinction (avec confirmation) |

**Modules propres à la barre.**
- `workspaces` : pastilles ; clic pour y aller, molette pour passer d'un
  workspace à l'autre.
- `window` : titre de la fenêtre active.
- `volume` : menu avec curseur et coupure ; clic du milieu pour couper,
  molette pour régler.
- `tray` : zone de notification. Clic pour activer l'application, clic
  droit pour son menu natif (dbusmenu).
- `desktop` : bouton qui affiche ou masque le bureau vide.

L'horloge ouvre un calendrier. `hyprdesk menu <module>` ouvre un menu au
clavier, par exemple `hyprdesk menu volume` ou `hyprdesk menu reseau`.

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

Il faut Rust 1.99+ (édition 2024), Hyprland 0.57+ et un pilote Vulkan ou GL.

```bash
./scripts/install.sh
```

Le script :
- compile `hyprdesk` et le place dans `~/.local/bin` ;
- écrit `~/.config/hypr/hyprdesk.lua` s'il n'existe pas, ainsi que les
  définitions LuaLS ;
- active le service systemd utilisateur.

Il ne modifie pas `hyprland.lua`. Les lignes recommandées s'obtiennent avec
`hyprdesk snippet` : le marquage `submap_universal` des raccourcis à garder
sur le bureau (voir ci-dessous), et le flou derrière la barre.

Pour que hyprdesk soit la racine de la session :
- retirez de l'autostart de Hyprland ce qu'il supervise désormais
  (`hd.component`) ;
- retirez `hyprpaper` ;
- masquez la barre de wayle si vous activez celle de hyprdesk.

## Raccourcis

- **Les raccourcis de hyprdesk sont installés par hyprdesk lui-même**, avec
  ceux de `hd.config({ keys = … })` ou de Réglages → Bureau :
  - `toggle`, `SUPER + D` par défaut, affiche ou masque le bureau ;
  - `settings`, `SUPER + SHIFT + D` par défaut, ouvre les réglages (bureau
    affiché compris, sans changer de mode) ;
  - `lock` verrouille la session (`hyprdesk lock`). Non réglé par défaut :
    c'est le `SUPER + L` de `hyprland.lua`, posé par `scripts/install.sh`.
    Réglé dans hyprdesk, il le remplace (la combinaison est libérée) ;
  - `leave`, `SUPER + Escape` par défaut, est la sortie de secours.

  Ils sont réinstallés après chaque `hyprctl reload`.
- **Pas de conflit avec Hyprland.** Avant d'installer un raccourci, hyprdesk
  vérifie qu'il n'est pas déjà lié. S'il l'est, il ne l'installe pas hors du
  bureau (celui de `hyprland.lua` reste maître) et le signale par une
  notification et dans les réglages ; bureau affiché, les raccourcis de
  Hyprland étant suspendus, hyprdesk l'installe quand même.
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
`$XDG_RUNTIME_DIR/hypr/<instance>/hyprdesk.lock` pendant toute sa vie.
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
| `hyprdesk show` (`full-desktop`) · `launcher-only` · `hide` (`no-desktop`) | les trois modes du bureau : complet (fenêtres mises de côté), lanceur seul (dock accessible, les fenêtres en mosaïque lui laissent la place), masqué |
| `hyprdesk toggle` · `toggle-launcher` | full-desktop / launcher-only, ou no-desktop si on y est déjà (raccourcis SUPER + D, SUPER + ALT + D ; SUPER + CTRL + D : no-desktop) |
| `hyprdesk cycle` | mode suivant : no-desktop → launcher-only → full-desktop → no-desktop (bouton Bureau de la barre ; clic droit : no-desktop) |
| `hyprdesk settings [paquet]` | ouvre les réglages sans changer le mode du bureau (`desktop`, `launcher`, `screens`, `fond`…) |
| `hyprdesk theme [list]` · `theme new <image\|pack.zip\|dossier> [--name NOM]` · `theme apply <nom>` | « Mes thèmes » ; un thème tiré d'une scène (couleurs de l'image, scène étalée sur tous les écrans, bureau et verrouillage) |
| `hyprdesk wallpaper split <image> [dossier] [--scale N]` | découpe une scène selon la disposition des écrans : une image par écran |
| `hyprdesk export [fichier] [--theme] [--screens]` · `hyprdesk import <archive> [--theme] [--dry-run] [--screens]` | partager son setup : configs, plugins, fonds d'écran — ou, avec `--theme`, le thème seul (couleurs, fonds, effets) ; l'import sauvegarde l'actuel |
| `hyprdesk open <rôle>` · `hyprdesk context` | application par défaut d'un rôle (`browser`, `terminal`, `ai`…) ; contexte de l'environnement pour un assistant IA |
| `hyprdesk config [list [paquet]] \| get \| set \| unset` | réglages en ligne de commande, par paquet, écrits dans `settings.lua` comme depuis l'écran (ex. `hyprdesk config set launcher_group app`) |
| `hyprdesk reload` | relit la configuration (elle est aussi relue à chaque enregistrement) |
| `hyprdesk status` | état : affiché ou caché, workspaces, config en erreur ou non |
| `hyprdesk check [fichier]` | vérifie une configuration sans toucher au démon |
| `hyprdesk init` · `stubs` · `snippet` | config d'exemple, définitions LuaLS, lignes pour `hyprland.lua` |

Options et variables :
- `--config <fichier>` ou `$HYPRDESK_CONFIG` : fichier de configuration à utiliser ;
- `$HYPRDESK_LOG` : niveau de journal (`debug`, `info`…).

## Écrans — `~/.config/hypr/monitors.lua`

hyprdesk gère les écrans (il remplace hyprdmc) : l'onglet **Écrans** détecte
les sorties, les place, les tourne, règle mode et échelle, et désigne l'écran
principal. Un changement s'applique à chaud ; **Garder** l'écrit dans
`monitors.lua` (requis par `hyprland.lua` : `require("monitors")`), sinon
l'affichage d'avant revient seul. Les écrans débranchés gardent leur ligne, et
un écran inconnu est accueilli en mode préféré, placé automatiquement. Un
`monitors.lua` écrit par un autre outil est conservé une fois en
`monitors.lua.bak`.

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

hd.config({ primary = "HDMI-A-2", show_on = "primary" })                  -- écran principal
hd.bar({ monitor = "primary" })                                            -- barre sur l'écran principal
-- Écrans étroits ou verticaux : barre compacte (auto), et modules propres aux écrans verticaux.
hd.bar({ compact = "auto", left = { "workspaces", "window" }, right = { "tray", "volume", "session" },
         portrait = { left = { "workspaces" }, right = { "volume", "session" } } })

hd.config({ workspaces = 7 })                                              -- 1 à 7 toujours présents
hd.config({ workspace_monitors = { ["HDMI-A-2"] = { 1, 2, 3, 4, 5 }, primary = { 6, 7 } } }) -- écran de chaque workspace
hd.assign({ app = "firefox", workspace = 2 })                              -- s'ouvre sur 2, et y emmène
hd.assign({ class = "^(org\\.telegram\\..*)$", workspace = 5, silent = true }) -- sans y aller

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
de réglages, puis le bureau. Les raccourcis Hyprland marqués `submap_universal` restent actifs : changer de workspace bureau ouvert affiche le bureau de
ce workspace.

**Lancement.** Les programmes sont lancés par Hyprland
(`hl.dsp.exec_cmd`) : ils lui appartiennent, et ses règles d'exec s'appliquent.

## Développement

```bash
cargo test --locked                 # config Lua, résolution, placement, IPC, .desktop
cargo run --locked -- --config exemple.lua   # démon avec une config de test
```

`Cargo.lock` épingle `winit-core` en `0.31.0-beta.2`, parce que la beta.4 casse
`iced_exdevtools`, une dépendance d'`iced_layershell` 0.19. Il faut donc
compiler avec `--locked`.

L'IPC Hyprland vient de `hyprpanel-core`
([mindlog.hyprland_tools](https://github.com/jacquesh82/mindlog.hyprland_tools)),
épinglé sur un commit.

Code sous licence MIT ; police Mindlog sous [SIL OFL 1.1](assets/fonts/mindlog/OFL.txt).

## Finition et typographie

Mindlog 0.7 est la police par défaut sur le bureau, la barre, les menus et
les réglages. Ses cinq graisses (Light, Regular, Medium, SemiBold et Bold) sont
embarquées dans le binaire ; `scripts/install.sh` les installe aussi dans
`${XDG_DATA_HOME:-~/.local/share}/fonts/hyprdesk/Mindlog`, avec leur licence OFL.
Réglages → Thème → Texte propose Mindlog et les familles installées
sur la machine, un aperçu et la taille du texte. Le choix est enregistré dans
`settings.lua` ; `hd.config({ font = "Mindlog", font_size = 14 })` reste
prioritaire s’il est déclaré dans le fichier utilisateur.

**Langue.** L'interface existe en français et en anglais : Réglages → Bureau →
Langue, `hyprdesk config set language fr|en|auto`, ou
`hd.config({ language = "en" })`. `auto` suit la langue du système (`LANG`) ;
une langue choisie vaut aussi pour les dates. Les plugins Lua traduisent avec
`hd.t("texte")` (détails : `docs/PLUGINS.md`, § 9).

**Layouts.** Réglages → Layouts règle la disposition des fenêtres par Hyprland :
layout par défaut et par workspace (Dwindle, Master, Scrolling, Monocle), leurs
options, et quatre layouts personnalisés écrits en Lua (grille, colonnes,
rangées, maître centré). Tout s'applique à chaud ; seuls les réglages faits
dans hyprdesk remplacent ceux de `hyprland.lua`. En Lua :
`hd.config({ layout = "master", layout_workspaces = { [3] = "monocle" }, master = { mfact = 0.6 } })` ;
en ligne de commande : `hyprdesk config set layout grid`, `layout.3 monocle`,
`master.orientation center`.

Les cartes et le dock partagent un verre fumé, des contours discrets et un
accent iris. Les icônes d’applications proviennent du thème système ; les
commandes possèdent des secours SVG intégrés. `transition = "none"` désactive
les nouveaux mouvements. Les fonds d’écran et couleurs explicitement choisis
restent prioritaires. Voir [DESIGN.md](DESIGN.md) pour les règles communes.

La zone de notification peut fournir son propre `StatusNotifierWatcher` quand
aucun autre service ne le fournit. Elle cohabite avec un watcher existant.
Les widgets réseau, Bluetooth et notifications utilisent respectivement
`nmcli`, `bluetoothctl` et `wayle`. Volume utilise `wpctl`, la liste des polices
`fc-list`, et les actions de session utilisent `hyprlock` et `systemctl`.
Une commande manquante ou trop lente est signalée par le widget.

## Fiabilité et limites d’exécution

Les déplacements de fenêtres et changements de submap sont traités dans
l’ordre par un acteur dédié. La restitution est vérifiée avant d’oublier une
entrée du journal ; une erreur IPC conserve les retours encore en attente.
Les fichiers d’état sont écrits atomiquement, et une sauvegarde ancienne ne
peut pas remplacer une modification plus récente.

Un widget ne lance qu’un relevé à la fois. Les résultats des anciennes
configurations sont écartés. `hd.exec` limite la durée à cinq secondes et la
sortie à 64 Kio par flux ; le widget `command` conserve son option `timeout`.
Les commandes dépassant leurs limites sont arrêtées avec leur groupe de
processus. Les appels Lua possèdent un budget de cinq secondes, également
appliqué aux coroutines.

Lua reste un environnement local de confiance, avec accès aux bibliothèques
standard : ces limites ne constituent pas un sandbox pour du code non fiable.
`check` exécute le Lua et charge les plugins ; il ne s’agit pas uniquement
d’une validation syntaxique.

## Boucle autonome de vérification

Rust 1.99 ou supérieur est requis. `Cargo.lock` doit rester utilisé.

```bash
bash scripts/verify.sh            # format, Clippy strict, tests, build, D-Bus isolé
bash scripts/verify.sh --visual   # ajoute les captures à 1440, 900 et 640 px
cargo run --locked -- preview     # aperçu interactif sans modifier la session
```

L’aperçu emploie les vues réelles avec des données de démonstration. Il ne
lance pas d’applications et n’écrit pas les réglages. Les captures sont placées
par défaut dans `/tmp/hyprdesk-visual` et permettent de revoir les neuf vues.
Les contrôles de l’aperçu sont limités à la navigation et à la typographie.
Le flou fourni par le compositeur et les interactions multi-écrans nécessitent
aussi une vérification dans une session Hyprland.

### Hyprshot et les outils de capture

Les raccourcis de capture doivent rester actifs dans la submap du bureau.
Ajouter `submap_universal = true` aux déclarations existantes, par exemple :

```lua
hl.bind("Print", hl.dsp.exec_cmd("hyprshot -m region --clipboard-only"), { submap_universal = true })
hl.bind("SUPER + Print", hl.dsp.exec_cmd("hyprshot -m output -m active"), { submap_universal = true })
```

La même option s’applique à `SHIFT + Print`, `ALT + Print` et à un raccourci
personnalisé comme `SUPER + SHIFT + S`. Les modes région et écran capturent le
bureau et les surfaces layer-shell. Le mode fenêtre de Hyprshot vise une
fenêtre d’application ; le bureau et la barre ne sont pas des fenêtres XDG.
