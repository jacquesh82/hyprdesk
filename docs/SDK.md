# SDK de hyprdesk

*[README](../README.fr.md) · [Guide avancé](ADVANCED.md)*

On peut étendre hyprdesk, le piloter et s'y intégrer par **plusieurs
surfaces publiques**. Chacune a son contrat et sa référence. Ce document dit
laquelle choisir, ce qu'elle garantit, et renvoie à sa documentation
détaillée.

Ce n'est pas une bibliothèque à lier. Une extension de hyprdesk, c'est des
**données** (un fichier Lua, un pack de thème) ou un **client** (d'un fichier
publié, d'un socket, d'une ligne de commande). Aucun code compilé ne se
charge dans le démon : le Lua y tourne dans un bac à sable borné, et un
plugin qui échoue est signalé sans que le bureau s'arrête.

Version décrite : hyprdesk **0.2.0**, `hyprdesk-theme-client` **0.1.0**.

## 1. Choisir sa surface

| Je veux… | Surface | Référence |
|---|---|---|
| ajouter un widget de bureau ou un module de barre | plugin Lua | [Écrire un plugin](WRITING_PLUGINS.md), §3 |
| décrire mon bureau par du code (fonds, règles, barre, Panorama) | `hyprdesk.lua` (`hd.config`…) | [Référence de configuration](ADVANCED.md#configuration-reference), [Panorama](PANORAMA.md#configuration-lua) |
| changer un réglage comme le ferait l'écran Réglages | `hyprdesk config` | §4 |
| piloter le bureau depuis un script, un raccourci ou un agent | socket de commande, ou CLI | [Socket de commande](CONTROL.md), §5 |
| prendre les couleurs du thème dans mon application | palette publiée, crate `hyprdesk-theme-client` | [Intégrer une application au thème](THEME-CLIENT.md), §6 |
| distribuer un thème, ou thémer un logiciel sans API | pack de thème (`theme.toml` et gabarits) | [Spécification du moteur de thème](THEME-ENGINE.md), §6 |
| donner le contexte du bureau à un assistant IA | `hyprdesk context`, serveur MCP hyprpanel | §7 |

```
                     ┌────────────────── démon hyprdesk ──────────────────┐
 plugins Lua ───────▶│ widgets, modules de barre (budget 5 s, isolés)     │
 hyprdesk.lua ──────▶│ configuration (hd.config, hd.bar, hd.on…)          │
 hyprdesk config ───▶│ settings.lua (même effet que les Réglages)         │
 socket de commande ▶│ modes, surfaces, LYRA, Panorama                    │
                     └────────────────────────────────────────────────────┘
 packs de thème ───▶ hyprdesk-theme-engine ──▶ palette.json ──▶ applications
                                            └─▶ GTK, Qt, kitty, Hyprland…
```

## 2. Ce qui est garanti

| Surface | Niveau | Ce qui fait foi |
|---|---|---|
| `palette.json` | **versionné** (`schema: 1`) : un client refuse tout autre schéma | [THEME-CLIENT.md §2](THEME-CLIENT.md#2-le-fichier-palettejson-canal-canonique) |
| `hyprdesk-theme-client` | **versionné** (semver de la crate) | `crates/hyprdesk-theme-client` |
| packs de thème | **spécifié** : manifeste, DSL et filtres en liste fermée, limites de sécurité | [THEME-ENGINE.md](THEME-ENGINE.md) |
| API Lua (`hd.*`, contrat d'un plugin) | **documenté**, non versionné : `hd.version` est la version de hyprdesk | `assets/hyprdesk.meta.lua` (écrit par `hyprdesk stubs`) |
| clés de `hyprdesk config` | **documenté**, non versionné | `hyprdesk config list`, [ADVANCED.md](ADVANCED.md#configuration-reference) |
| socket de commande | **documenté**, non versionné, réponses en texte traduit | [CONTROL.md](CONTROL.md) |
| `hyprdesk status`, sortie lisible du CLI | **non stable** : pour un humain, ne pas l'analyser | — |
| plugins Rust intégrés (Barre, Dock, Lanceur, Panorama et ses modules), `src/` | **interne** | — |

Une surface **documentée** peut changer d'une version à l'autre. Les
changements sont annoncés dans le [CHANGELOG](../CHANGELOG.md). Une surface
**interne** n'a aucun contrat.

## 3. Plugins Lua

Un plugin est **un fichier Lua qui renvoie une table**. Il sert à la fois
de widget de bureau et de module de barre. Il est relu dès qu'on
l'enregistre, sans recompiler ni redémarrer.

```lua
-- ~/.config/hypr/hyprdesk/widgets/disque.lua
return {
  name = "Disque",
  interval = 60,                       -- secondes entre deux rendus
  options = { mount = "/" },
  render = function(o)
    local pct = tonumber(hd.exec("df --output=pcent " .. o.mount .. " | tail -1 | tr -dc 0-9")) or 0
    return {
      text = pct .. " %",
      icon = "drive-harddisk-symbolic",
      progress = pct / 100,
      state = pct > 90 and "critical" or pct > 75 and "warning" or "normal",
    }
  end,
  on_click = function(o) hd.spawn("baobab " .. o.mount) end,
}
```

**Contrat d'un plugin** (`HD.Plugin`) : `render(o)` (obligatoire, rend un
texte ou une table `HD.Rendered`), `name`, `description`, `interval`,
`anchor`, `options`, `on_click(o)`, `on_scroll(o, dir)`, et `menu(o)`, qui
rend des entrées `HD.MenuEntry` ou `"-"` (séparateur). Une entrée peut porter
un champ de saisie (`input`, `secret`), une confirmation (`confirm`) ou une
image (`image`).

**Fonctions offertes** :

| Domaine | Fonctions |
|---|---|
| exécution | `hd.spawn(cmd)` (sans attendre, par Hyprland), `hd.exec(cmd)` (rend la sortie standard), `hd.write_private(path, data)` (fichier 0600 exclusif, sans suivre de lien) |
| état système, sans processus (D-Bus, sysfs) | `hd.network()`, `hd.bluetooth()`, `hd.power_profile()`, `hd.batteries()`, `hd.gpus()`, `hd.index_status()`, `hd.notifications()` ; `nil` si le service ne répond pas |
| langue | `hd.t(texte)` (catalogue de hyprdesk, le français sert de clé), `hd.lang()` |
| environnement | `hd.version`, `hd.exe` (chemin du hyprdesk en cours), `hd.runtime_dir` (dossier privé 0700, jamais `/tmp`) |

Pour la configuration (`hyprdesk.lua`), il y a en plus `hd.config`,
`hd.wallpaper`, `hd.desktop`, `hd.assign`, `hd.icon`, `hd.widget`,
`hd.place`, `hd.disable`, `hd.bar`, `hd.component`, et
`hd.on("show"|"hide"|"workspace", fn)` pour réagir aux changements de mode
et de workspace.

**Règles du bac à sable** : 5 s par appel au Lua, après quoi le plugin est
arrêté et signalé. `render` ne tourne que pendant que le widget est visible.
Tout état global est perdu à chaque rechargement. Ce qui peut dépasser une
seconde passe par `hd.spawn` ou par un fichier en cache.

**Outillage** :
- `hyprdesk stubs [dossier]` : définitions LuaLS (`hyprdesk.meta.lua`) pour
  l'autocomplétion ;
- `hyprdesk check [fichier]` : exécute la configuration sans toucher au démon ;
- journal : `journalctl --user -u hyprdesk -f` (`print()` y arrive), plus de
  détail avec `HYPRDESK_LOG=info,hyprdesk=debug`.

**Distribution** : dossiers `~/.config/hypr/hyprdesk/widgets/`, puis
`$XDG_DATA_HOME/hyprdesk/widgets/`, puis `$XDG_DATA_DIRS/*/hyprdesk/widgets/`
(paquet de distribution). Le premier trouvé l'emporte. Un fichier qui porte
l'id d'un plugin livré le remplace. Installation par Réglages → Plugins →
Ajouter (fichier, dossier ou dépôt git). `hyprdesk export` et `hyprdesk import`
emportent les plugins avec le reste du setup.

Guide pas à pas : [WRITING_PLUGINS.md](WRITING_PLUGINS.md). Contrat interne
des paquets de réglages : [PLUGINS.md](PLUGINS.md).

## 4. Réglages en ligne de commande

`hyprdesk config` a le même effet que l'écran Réglages : c'est lui qui écrit
`settings.lua` et prévient le démon. **N'éditez jamais `settings.lua`,
`inputs.lua`, `monitors.lua` ni `autostart.lua` à la main** : hyprdesk les
tient.

```sh
hyprdesk config list [paquet]          # clés et valeurs
hyprdesk config get bar.position
hyprdesk config set bar.position bottom
hyprdesk config unset bar.position
hyprdesk config set assign.firefox 2 silent
hyprdesk config history --json         # 30 jours d'historique
hyprdesk config rollback 2026-11-21T18:00 --yes
```

Les clés dynamiques (`assign.<app>`, `widget.<type>`, `bar.color.<rôle>`,
`fx.<section.option>`, `forced.<écran>`, `screen_zones.<écran>`,
`panorama.zones.…`) sont décrites par `hyprdesk config help`. Sans argument,
`hyprdesk config` équivaut à `list`.

## 5. Piloter le bureau

Deux voies, qui font le même échange :

- **le CLI** : `hyprdesk <commande>`. C'est le plus simple dans un script
  ou un raccourci, et il rend un code de sortie ;
- **le socket** `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/hyprdesk.sock` :
  une ligne par connexion, réponse `ok` ou un message d'erreur. C'est le
  bon choix depuis un programme au long cours.

Référence complète des commandes et du protocole : [CONTROL.md](CONTROL.md).

**Sorties exploitables par un programme** :

| Commande | Format |
|---|---|
| `hyprdesk config history --json` | JSON |
| `hyprdesk hyprland-check --json` | JSON |
| `hyprdesk index query <mots> --json` | JSON |
| `hyprdesk media status` | `clé=valeur` |
| `hyprdesk doctor --missing` | une ligne par dépendance absente, champs séparés par des tabulations |
| `hyprdesk context` | Markdown (pour un assistant) |
| `hyprdesk-theme-engine palette --format json\|env\|css\|hex` | voir §6 |

## 6. Thème

Le moteur de thème (`hyprdesk-theme-engine`) fait foi : il résout le pack
actif, applique ses cibles et **publie** la palette. Une application ne fait
que **lire** cette palette.

| L'application est… | Canal |
|---|---|
| en Rust | crate `hyprdesk-theme-client` : `ThemeClient::new().palette()`, `subscribe(…)` |
| dans un autre langage, avec une boucle d'événements | suivre le **dossier** de `$XDG_STATE_HOME/hyprdesk/theme-engine/palette.json` et comparer `revision` |
| un script, un TUI, un prompt | `hyprdesk-theme-engine palette --format env\|css\|json\|hex`, ou les variables `HYPRDESK_*` reçues au lancement |
| GTK 3/4, libadwaita, Qt | rien à coder |
| un logiciel tiers sans API | cible générique dans un pack (gabarit et rechargement borné) |

```rust
use hyprdesk_theme_client::ThemeClient;

let client = ThemeClient::new();
let accent = client.palette().ok().and_then(|p| p.slot("accent"));
let _watcher = client.subscribe(|pal| { /* message à l'application, puis redessin */ })?;
```

Une application **n'appelle jamais** `use`, `apply`, `install` ni `remove` :
le thème, c'est l'utilisateur qui le choisit. Le guide
[THEME-CLIENT.md](THEME-CLIENT.md) est écrit comme une consigne à confier
telle quelle à un agent qui doit intégrer une application.

Pour **créer** un thème : manifeste `theme.toml`, palette de slots
sémantiques, variantes, héritage, effets par surface, options Hyprland en
liste blanche, gabarits `{{palette.accent|lighten(8%)|fmt(rgb)}}`. Tout est
dans la [spécification](THEME-ENGINE.md). Outils :
`hyprdesk-theme-engine build | verify | export | install`, et archives
`.hyprtheme`.

## 7. Assistants IA

- `hyprdesk context` décrit l'environnement hyprdesk en Markdown, prêt à
  joindre à une conversation.
- Le serveur MCP **hyprpanel** (dépôt `mindlog.hyprland_tools`) expose
  hyprdesk à un agent : `hyprdesk_status`, `hyprdesk_config_*`,
  `hyprdesk_set_mode`, `hyprdesk_keys`, `hyprdesk_doctor`…
- `ask` et `problem` (socket ou CLI) remettent une question ou un rapport
  d'erreur à LYRA, l'assistant intégré.

## 8. Règles de bonne conduite

- Ne pas écrire dans les fichiers que tient hyprdesk (`settings.lua`,
  `inputs.lua`, `monitors.lua`, `autostart.lua`), ni dans l'état du moteur de
  thème (`palette.json`, `active.json`). Passer par `hyprdesk config`, les
  Réglages ou `hyprdesk autostart`.
- Pas de fichier temporaire dans `/tmp` : utiliser `hd.runtime_dir` et
  `hd.write_private`.
- Ne pas bloquer : rien de plus d'une seconde dans un appel Lua, pas de
  sondage en boucle d'un fichier ou du socket.
- Tester la réponse `ok` du socket, et montrer tout le reste tel quel.
- Lire `status` et les sorties lisibles du CLI sans jamais les analyser.

## 9. Pas encore disponible

Ce qui suit **n'existe pas encore**. Ne pas en dépendre :

- **état du bureau publié** (mode, écrans, workspaces, Réglages ouverts) sous
  forme de fichier versionné comme `palette.json`, avec un suivi des
  changements. En attendant : `hd.on(…)` en Lua, et le socket d'événements de
  Hyprland (`.socket2.sock`) pour les workspaces et les fenêtres ;
- **socket versionné** : commande `version`, réponses en JSON ;
- **crate cliente du socket** (`hyprdesk-client`) ;
- **modules du Panorama en Lua** : aujourd'hui en Rust seulement, le Lua ne
  fait que les configurer ;
- **portail `org.freedesktop.appearance`** (MLG-236).
