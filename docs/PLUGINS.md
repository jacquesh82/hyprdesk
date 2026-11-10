# Conception — paquets, widgets et lanceur
*[English version](PLUGINS.en.md)*

hyprdesk est découpé en **paquets** : chaque fonction réglable (bureau,
icônes, lanceur, fond d'écran, widgets, barre, écrans, workspaces, effets) est
un module autonome, livré avec le binaire avec le statut de *paquet intégré*
(`builtin: true`). Ce document décrit le contrat d'un paquet, celui d'un
widget, et la place du lanceur.

| Notion | Où | Ce que c'est |
|---|---|---|
| **Paquet de réglages** | `src/ui/plugins/<id>.rs` | une entrée de l'écran Réglages : manifeste, messages, état, vue, mise à jour, clés CLI |
| **Widget** | `src/widgets/`, `src/ui/widget_view.rs` | un élément posé sur le bureau ou dans la barre, alimenté par une source de données |
| **Plugin de widget** | `~/.config/hypr/hyprdesk/widgets/<nom>.lua` | un widget écrit en Lua, détecté sans recompiler |
| **Lanceur** | `src/ui/plugins/launcher.rs` | le dock du bureau : widget autonome, seul contrôle du mode `launcher-only` |

## 1. Paquets de réglages

### Contrat

Un paquet est un module de `src/ui/plugins/` qui expose :

```rust
pub const INFO: PluginInfo = PluginInfo {
    id: "screens",            // identifiant stable : onglet, `hyprdesk settings screens`
    label: "Écrans",          // navigation
    icon: "video-display",    // glyphe (ui/icons.rs) ou icône du thème
    summary: "Détection, disposition, …",
    builtin: true,            // livré avec hyprdesk
};

#[derive(Debug, Clone)]
pub enum Msg { … }                         // ses messages, rien d'autre
pub fn wrap(m: Msg) -> UiMsg { … }         // enveloppe vers l'application
pub struct State { … }                     // son état d'écran (facultatif)

impl App {
    pub(in crate::ui) fn tab_screens(&self) -> Element<'_, UiMsg>;       // vue
    pub(in crate::ui) fn screens_update(&mut self, m: Msg) -> Task<UiMsg>; // mise à jour
}
```

Le paquet ne connaît que ses propres messages. Il en émet par `wrap(Msg::…)`;
s'il doit parler à un autre paquet, il passe par `super::autre::wrap(…)`, ce
qui rend la dépendance visible.

### Registre et coquille

`src/ui/plugins/mod.rs` est le **registre** :

- `BUILTIN` : les manifestes, dans l'ordre de la navigation ;
- `PluginMsg` : une variante par paquet, enveloppée dans `PanelMsg::Plugin` ;
- `update`, `view`, `on_show` : le routage vers le paquet concerné ;
- `find` : retrouve un paquet par identifiant ou par alias (`fond`,
  `écrans`…), pour `hyprdesk settings <paquet>`.

`src/ui/panel.rs` est la **coquille** du tiroir : sa surface (couche propre
au-dessus des fenêtres), l'ouverture et la fermeture, la navigation
construite depuis `BUILTIN`, et les aides partagées (`section`, `field`,
`apply`, `commit`, `commit_reload`, `panel_ctx`). Elle ne contient aucun
réglage : ses seuls messages sont `Tab`, `Close`, `Commit` et `Plugin(..)`.

### Cycle d'un réglage

1. La vue émet `wrap(Msg::X)`.
2. La mise à jour du paquet modifie `App::settings` (le modèle de
   `src/settings.rs`, miroir de `settings.lua`).
3. Elle appelle l'une des aides de la coquille :
   - `apply()` : la configuration effective change en mémoire, sans
     enregistrer. C'est ce qui sert pendant le glissé d'un curseur.
   - `commit()` : comme `apply()`, puis écrit `~/.config/hypr/hyprdesk/settings.lua`.
   - `commit_reload()` : écrit, puis relit toute la configuration. C'est
     nécessaire quand du Lua doit être réexécuté, par exemple pour activer un
     widget.
4. Le démon surveille `hyprdesk/` : une écriture venue d'ailleurs (CLI,
   éditeur) est relue de la même façon.

`settings.lua` est chargé **avant** `hyprdesk.lua` : ce que l'utilisateur écrit
à la main garde le dernier mot, et `field(…, "clé")` le signale dans la vue.
`settings.lua` est aussi relu **seul**, sans `hyprdesk.lua`, pour l'état de
l'écran. Un réglage qui vise un élément de `hyprdesk.lua` doit donc survivre à
cette lecture solitaire (c'est le cas de `hd.place` et de `hd.disable`).

### Ligne de commande

Chaque paquet déclare ses clés dans `src/cli_config.rs` (table `KEYS`, champ
`plugin`). Une clé, c'est un nom, une aide, et trois fonctions : lire la valeur
effective, écrire dans `Settings`, retirer. `hyprdesk config` les range par
paquet, dans l'ordre de `BUILTIN`, et écrit le même `settings.lua` que l'écran
Réglages :

```sh
hyprdesk config                       # toutes les clés, par paquet, valeurs effectives
hyprdesk config list screens          # un paquet
hyprdesk config get launcher_group
hyprdesk config set show_on primary
hyprdesk config set keys.launcher "SUPER + ALT + D"
hyprdesk config set assign.firefox 2 silent   # clé dynamique (workspaces)
hyprdesk config set widget.calendar on        # clé dynamique (widgets)
hyprdesk config unset show_on                 # retour à la valeur par défaut
```

Clés par paquet :

| Paquet | Clés |
|---|---|
| `desktop` | `show_on`, `mode`, `glass_opacity`, `close_on_launch`, `float_launched`, `suspend_binds`, `font`, `font_size`, `keys.toggle`, `keys.launcher`, `keys.hide`, `keys.settings`, `keys.lock`, `keys.leave` |
| `icons` | `grid.icon_size`, `grid.cell`, `icon_theme` |
| `launcher` | `running_apps`, `launcher_group` |
| `wallpaper` | `wallpaper` (image ou `#couleur`, tous les bureaux), `wallpaper.fit`, `wallpaper.span` (`on` : étalée sur tous les écrans) |
| `widgets` | `widget.<type>` (`on` / `off`) |
| `bar` | `bar`, `bar.position`, `bar.height`, `bar.monitor` (`all`, `primary`, ou des écrans séparés par des virgules) |
| `screens` | `primary` |
| `workspaces` | `workspaces`, `workspace.<n>` (écran : connecteur, `primary`, `auto`), `assign.<app>` |
| `layouts` | `layout`, `layout.<n>`, `dwindle.<option>`, `master.<option>`, `scrolling.<option>` |
| `apptheme` | `app_themes` (`kitty`, `none`) |
| `effects` | `transition`, `fade_ms`, `crossfade_ms` |
| `theme` | `theme.preset` (thème livré), `theme.name`, `theme.fg`, `theme.bg`, `theme.accent`, `background`, `theme.effects.<endroit>.opacity` (0..1), `theme.effects.<endroit>.blur` (endroits : `bar`, `menus`, `launcher`, `settings`, `widgets` — sans flou) |
| `launcher` (apparence) | `launcher_position`, `launcher_size`, `launcher_pinned`, `launcher_show_settings`, `launcher_show_assistant` |
| `apps` | `apps.<rôle>` (`browser`, `terminal`, `files`, `editor`, `image`, `video`, `audio`, `pdf`, `mail`, `ai`) |
| `assistant` | `assistant_mode` (`window`, `terminal`) |
| `plugins` | `plugin.<id>` (`on` / `off`) |

La disposition des écrans (position, rotation, mode) reste dans l'écran
Réglages. Chaque changement y est appliqué à chaud, puis doit être confirmé
avant d'être écrit dans `monitors.lua`. Une écriture aveugle depuis la ligne
de commande n'aurait pas ce filet de sécurité.

### Navigation et configuration avancée

La navigation des Réglages range les paquets par groupes (`plugins::GROUPS`) :
Bureau, Apparence, Barre et lanceur, Fenêtres et écrans, Applications, Système.
Un nouveau paquet s'ajoute à un groupe (un test vérifie que chaque paquet est
dans un seul). Tiroir large : les onglets du paquet choisi s'affichent sous lui,
en sous-entrées ; tiroir étroit : en pastilles au-dessus de la page.

« Configuration avancée » (en-tête du tiroir, clé `advanced_settings`) : cochée,
tout est affiché ; décochée, l'essentiel. Sont alors masqués les paquets de
`ADVANCED_ONLY`, les onglets de `ADVANCED_TABS`, et les champs qu'une vue
entoure de `self.adv(…)`. Rien ne disparaît de la configuration : seul
l'affichage change.

### Onglets des paquets chargés

| Paquet | Onglets |
|---|---|
| Bureau | Affichage · Raccourcis · Session |
| Thème | Thèmes · Couleurs · Texte (police, taille) · Effets |
| Lanceur | Contenu · Apparence · Épinglées |
| Barre | Général · Modules |
| Écrans | Disposition · Écran (un clic sur le plan y mène) |
| Workspaces | Nombre et écrans · Applications |
| Layouts | Général · Dwindle · Master · Scrolling · Personnalisés |
| Widgets | Sur ce bureau (un widget choisi sur le bureau y mène) · Disponibles |

### Ajouter un paquet

1. Créer `src/ui/plugins/<id>.rs` avec `INFO`, `Msg`, `wrap`, l'état si besoin,
   la vue et `<id>_update`.
2. Le déclarer dans `plugins/mod.rs` : `pub mod`, une ligne dans `BUILTIN`, une
   variante dans `PluginMsg`, un bras dans `update`, `view` et, si besoin,
   `on_show`.
3. Si le paquet a un état d'écran, ajouter son champ à `Panel` (`panel.rs`).
   S'il a beaucoup de réglages, les répartir en onglets : une constante
   `TABS` (`(identifiant, libellé)`), un bras dans `plugins::tabs`, et la vue
   n'affiche que l'onglet de `self.subtab(INFO.id)`. Le tiroir dessine les
   pastilles et retient le choix ; `hyprdesk settings <paquet>/<onglet>`
   l'ouvre directement.
4. Persister dans `Settings` (`src/settings.rs`) : le champ, `from_config`,
   `apply_to` et `to_lua`. Côté Lua, la clé doit être reconnue dans
   `config/lua.rs` et documentée dans `assets/hyprdesk.meta.lua`.
5. Déclarer les clés CLI dans `cli_config.rs`.
6. Ajouter un test du paquet, et une scène dans l'aperçu
   (`ui/preview.rs`, `scripts/verify.sh`).

Le test `plugins::tests::registre` vérifie que les identifiants sont uniques.
Le test `cli_config::tests::chaque_cle_appartient_a_un_paquet` vérifie que
chaque clé CLI pointe vers un paquet existant.

### Paquets intégrés

| id | Libellé | Règle | Domaine hors interface |
|---|---|---|---|
| `desktop` | Bureau | écrans où le bureau s'affiche, mode, police, raccourcis des trois modes, session | `config::General` |
| `icons` | Icônes | raccourcis d'applications, taille, thème | `launch.rs`, `layout.rs` |
| `launcher` | Lanceur | fenêtres affichées, regroupement | — (widget, § 3) |
| `theme` | Thème | couleurs, fonds, effet de chaque endroit (opacité, flou), mode du bureau, voile du verrouillage ; thèmes livrés et « Mes thèmes » | `theme.rs`, `config::Theme` |
| `apptheme` | Thème des applications | le thème appliqué à d'autres logiciels (kitty : `~/.config/kitty/hyprdesk-theme.conf`, inclus à la fin de kitty.conf, relu par SIGUSR1) | `apptheme.rs` |
| `wallpaper` | Fond d'écran | image, diaporama, couleur ; portée | `config::WallpaperRule` |
| `widgets` | Widgets | activation, placement | `widgets/` (§ 2) |
| `bar` | Barre | position, hauteur, écrans, modules | `ui/bar.rs` |
| `screens` | Écrans | détection, disposition, orientation, mode, écran principal | `screens.rs` → `monitors.lua` |
| `workspaces` | Workspaces | workspaces permanents, affectation des applications | `hypr.rs` (règles Hyprland) |
| `layouts` | Layouts | layout par défaut et par workspace, options de Dwindle, Master, Scrolling, layouts personnalisés (`lua:<nom>`), raccourcis de chaque layout | `tiling.rs` |
| `effects` | Effets | transitions, fondu entre fonds | `config::General` |
| `apps` | Applications | applications par défaut (navigateur, terminal, fichiers, éditeur, images, vidéo, audio, PDF, mail, IA), appliquées au système (`xdg-mime`) ; seules les applications installées sont proposées (programme de `TryExec` / `Exec` présent), reconnues au type de fichier du rôle ou à leur catégorie freedesktop (`WebBrowser`, `TextEditor`, `FileManager`, `Email`, `TerminalEmulator`), un choix devenu absent est signalé « non installée » | `apps.rs` |
| `assistant` | Assistant IA | LYRA (mindlog.hyprland_tools) avec Claude Code, Codex ou Zcode, et le contexte de hyprdesk | `assistant.rs` |
| `share` | Partage | export / import d'un setup ou du thème seul (archive) | `share.rs` |
| `plugins` | Plugins | état et activation des paquets, plugins Lua : actifs, tiers, ajout, retrait | `widgets/plugins.rs` |

Le **domaine** (modèle, Lua, règles Hyprland, `monitors.lua`) ne dépend pas de
l'interface. Le paquet fait le lien entre les deux. C'est ce qui permettra plus
tard d'en faire des *crates* séparées : le domaine d'abord, l'interface
ensuite. Pour l'instant, les paquets sont des modules du binaire, parce que
leurs vues ont besoin de l'état de l'application (`App`).

## 2. Widgets

### Modèle

Un widget, c'est un **type** et une **instance** :

- **Type** (`config::WidgetType`, catalogue `Config::catalog`) :
  - intégré, déclaré dans `widgets::BUILTINS` : `clock`, `calendar`,
    `system`, `media`, `command`, `lua` ;
  - ou plugin Lua détecté (§ 2.3).
- **Instance** (`config::WidgetDef`) : un type et ses options, plus son
  placement (`anchor` sur une grille 3×3, décalages `x`/`y`, `size`, `card`).
  Elle porte aussi sa portée (`workspace`, `monitor`) et son origine
  (`hyprdesk.lua` ou `settings.lua`).

Identifiants. Une instance déclarée sans `id` reçoit `<type>#<n>`, compté
par type **et** par origine : un widget ajouté depuis les réglages ne décale
pas ceux de `hyprdesk.lua`. Les modules de la barre ont leurs propres
identifiants (`bar:<zone>:<n>`) et ne comptent pas.

### Cycle de vie

1. **Déclaration** : `hd.widget("calendar", { anchor = "right" })` dans
   `hyprdesk.lua`, ou activation dans Réglages → Widgets (écrit dans
   `settings.lua`).
2. **Placement** : `hd.place("calendar#0", { … })` repositionne un widget de
   `hyprdesk.lua` sans toucher au fichier.
3. **Désactivation** : `hd.disable("calendar#0")` éteint un widget de
   `hyprdesk.lua`. Rallumer retire la ligne.
4. **Relevé** : l'ordonnanceur (`ui/mod.rs`, `poll`) relève chaque widget à
   son `interval`, hors du thread de l'interface, et seulement si le widget est
   visible. Les résultats d'une configuration périmée sont ignorés (numéro de
   génération).
5. **Rendu** : `ui/widget_view.rs` traduit la valeur (`widgets::Value` : texte,
   lignes système, média, contenu riche) en éléments iced, dans sa carte.

### Plugins Lua

Un plugin est un fichier `<nom>.lua` ou un dossier `<nom>/init.lua`. Dossiers
de recherche, le premier trouvé l'emporte :

1. `~/.config/hypr/hyprdesk/widgets`
2. `~/.local/share/hyprdesk/widgets`
3. `/usr/share/hyprdesk/widgets`

Les plugins livrés (`reseau`, `bluetooth`, `batterie`, `notifications`,
`session`) sont intégrés au binaire, et un fichier du même nom les remplace.

```lua
return {
    name = "VPN", description = "État du VPN", interval = 5, anchor = "top-right",
    options = { interface = "wg0" },          -- valeurs par défaut des options
    render = function(o)                      -- texte, ou table riche :
        return { icon = "network-vpn-symbolic", tooltip = "VPN actif", state = "active" }
    end,                                      -- state : normal, active, warning, critical, hidden
    on_click  = function(o) end,
    on_scroll = function(o, sens) end,        -- +1 / -1
    menu = function(o) return { { header = "VPN" }, { label = "Couper", action = function() end } } end,
}
```

Le même plugin sert sur le bureau et dans la barre (`hd.bar({ right = { "vpn" } })`).
Aides disponibles : `hd.exec` (sortie d'une commande, temps borné) et
`hd.spawn` (lancement par Hyprland). Le Lua s'exécute sur un thread dédié,
avec un budget de temps par appel. Un plugin en erreur est signalé dans les
réglages sans bloquer les autres.

### Barre

Les modules de la barre sont des widgets placés dans une zone (`left`,
`center`, `right`) de `hd.bar`, avec `in_bar = true`. Certains sont propres à
la barre et ne sont pas des widgets : `workspaces`, `window`, `volume`, `tray`,
`desktop`. `hd.bar({ monitor = "primary" })` affiche la barre sur l'écran
principal.

La barre s'adapte à chaque écran :

- **Mode compact** (`compact = "auto" | "always" | "never"`, `auto` par
  défaut) : sur un écran de moins de 1600 px de large ou vertical, le titre de
  fenêtre est raccourci, l'horloge réduite à l'heure, et les icônes perdent
  leur libellé (volume, plugins).
- **Modules des écrans verticaux** (`portrait = { left, center, right }`) :
  ils se règlent aussi dans Barre → Modules → Écrans verticaux.

Chaque zone est rognée : même trop longues, elles ne se chevauchent plus.

## 3. Le lanceur

Le lanceur était jusqu'ici un morceau du bureau. C'est désormais un **widget
autonome**, avec son paquet (`launcher`) :

- **Rôle.** C'est le dock en bas au centre, avec les fenêtres ouvertes puis le
  bouton ⚙. C'est le seul contrôle du mode `launcher-only`, où les fenêtres
  en mosaïque lui laissent sa place (règle `gaps_out` par écran).
- **Fenêtres affichées** (`running_apps`) :
  - `off` : seulement le bouton des réglages ;
  - `workspace` : les fenêtres du workspace affiché ;
  - `all` : toutes les fenêtres, y compris celles mises de côté, pour pouvoir
    les rappeler.
- **Regroupement** (`launcher_group`) :
  - `none` : une entrée par fenêtre ;
  - `app` : une entrée par application, c'est-à-dire par classe de fenêtre,
    donc par binaire. Des points indiquent le nombre de fenêtres, et un clic
    passe de l'une à l'autre ;
  - `workspace` : les fenêtres rangées sous le numéro de leur workspace.
- **Règle d'or.** L'utiliser ne change jamais le mode du bureau :
  - en `launcher-only`, un clic donne le focus à la fenêtre ;
  - en `full-desktop`, la fenêtre choisie revient seule par-dessus le bureau,
    et les autres restent de côté ;
  - ⚙ ouvre les réglages dans leur propre couche.

## 4. Les trois modes du bureau

| Mode | Fenêtres | Visible | Raccourci |
|---|---|---|---|
| `no-desktop` | normales | fond d'écran | SUPER + CTRL + D |
| `launcher-only` | en place, dégagent le lanceur | lanceur | SUPER + ALT + D |
| `full-desktop` | mises de côté (`special:hyprdesk`) | icônes, widgets, lanceur | SUPER + D |

Le raccourci du mode en cours rend `no-desktop`. Le bouton Bureau de la barre
passe d'un mode au suivant : `no-desktop → launcher-only → full-desktop`. Ni
le lanceur ni les réglages ne changent le mode.

## 5. Activer, couper, ajouter

Le paquet **Plugins** montre tout ce qui est installé :

- **Paquets intégrés.** Chacun a un interrupteur. Couper un paquet
  (`disabled_plugins`, ou `hyprdesk config set plugin.bar off`) le retire de
  la navigation **et** coupe sa fonction : pas de barre, pas de lanceur, pas
  de règles de workspaces, pas d'icônes ni de widgets sur le bureau, fond uni,
  pas d'avis de nouvel écran. `Config::plugin_on(id)` est le seul test, posé
  au point où chaque fonction agit. Bureau, Effets, Thème et Plugins forment
  le cœur et restent toujours actifs (`CORE_PLUGINS`).
- **Plugins Lua.** Chacun indique s'il est actif et d'où il vient : intégré,
  livré, ou tiers (avec son chemin). Un plugin tiers s'installe depuis un
  fichier `.lua`, un dossier `init.lua` ou un dépôt git, dans
  `~/.config/hypr/hyprdesk/widgets/` (`widgets::plugins::install`). Le
  retirer le déplace dans `.retired/`, d'où on peut le remettre.

Un paquet *compilé* tiers n'est pas chargeable à chaud. L'extension d'un tiers
passe donc par les plugins Lua, ou par une contribution au registre `BUILTIN`.

## 6. Lanceur : sa propre couche

Le lanceur a une surface par écran, sur la couche `Top` de Hyprland, au-dessus
des fenêtres. Sa **zone réservée** (comme la barre) fait que les fenêtres lui
laissent sa place, sans règle de marge. Infobulles et liste des fenêtres d'une
application regroupée s'ouvrent dans une **bulle**, elle aussi une surface
(`Overlay`) placée au curseur. Un délai de 250 ms laisse le temps de passer du
lanceur à la bulle. Réglages : bord (`bottom`, `top`, `left`, `right`),
taille, applications épinglées, boutons de l'assistant et des réglages.

## 7. Assistant IA et LYRA

Le paquet `assistant` ouvre **LYRA** (`hyprpanel-chat`, dépôt
mindlog.hyprland_tools). LYRA travaille avec l'abonnement choisi pour le rôle
`ai` (Claude Code, Codex, Zcode). hyprdesk lui fournit son environnement :

- `hyprdesk context` : modes et raccourcis, paquets, applications par défaut,
  lanceur, workspaces, barre, widgets, fichiers, commandes ;
- le même texte dans `~/.config/hyprpanel/context.d/hyprdesk.md`, tenu à
  jour à chaque changement de configuration.

Contrat attendu côté LYRA, détecté dans son `--help` :

- `--provider <zcode|claude_code|codex>` : fournisseur de la session ;
- lecture des fichiers de `~/.config/hyprpanel/context.d/*.md`, **ajoutés**
  au contexte (sans remplacer le rôle de LYRA).

Tant que LYRA ne les propose pas, elle garde son propre choix de fournisseur,
et le contexte attend dans `context.d`.

### Le tiroir de l'assistant (protocole `serve`)

Quand LYRA propose `hyprpanel-chat serve` (vu dans son `--help`) et que
l'assistant s'ouvre « dans sa fenêtre », hyprdesk **dessine lui-même** la
conversation dans un tiroir au format des Réglages (même couche, en-tête,
cartes, Échap) ; LYRA garde tout le moteur : fournisseurs, outils,
confirmations, mémoire, périmètre, `context.d`. Un seul tiroir à la fois.
Sans `serve`, LYRA s'ouvre dans sa fenêtre, placée comme le tiroir.

Le moteur est un processus enfant, lancé à l'ouverture et arrêté à la
fermeture. Il dialogue en **JSON Lines** (un objet par ligne, distingué par
`type` ; champs inconnus ignorés) — code : `src/chat.rs`, vue : `src/ui/chat.rs`.

| Sens | `type` | Champs |
|---|---|---|
| → LYRA | `hello` | `client`, `protocol` (1), `language` |
| → LYRA | `message` | `id`, `text`, `attachments` [`{path}`] |
| → LYRA | `confirm` | `request`, `accept` |
| → LYRA | `choice` | `request`, `value` |
| → LYRA | `cancel`, `new_conversation` | — |
| → LYRA | `provider` | `value` |
| ← LYRA | `ready` | `protocol`, `provider`, `providers`, `fs` |
| ← LYRA | `turn_start` / `turn_end` | `id` (+ `text` complet en fin) |
| ← LYRA | `delta` | `text` (Markdown, au fil de l'eau) |
| ← LYRA | `thinking` / `progress` | `active` / `phase`, `fraction` |
| ← LYRA | `tool` | `id`, `name`, `summary`, `status` (`running`, `done`, `error`), `result` |
| ← LYRA | `confirm_request` | `id`, `command`, `reason`, `risk` (`low`, `high`) |
| ← LYRA | `choice_request` | `id`, `kind`, `prompt`, `options` [`{label, value, preview}`] |
| ← LYRA | `error` | `message`, `fatal` |

Une confirmation n'est **jamais** acceptée par défaut : elle attend le clic.
Pour essayer sans LYRA : `HYPRDESK_CHAT_ENGINE="hyprdesk chat-mock"` dans
l'environnement du démon (faux moteur : texte en flux, un outil, une
confirmation si le message parle de « gaps », un choix s'il parle de « couleur »).

## 8. Partager un setup

`hyprdesk export [fichier] [--theme] [--screens]` et `hyprdesk import
<archive> [--theme] [--dry-run] [--screens]`, ou le paquet **Partage**, qui
propose deux portées : **tout le setup**, ou **le thème seul**.

- **L'archive** contient `hyprdesk.lua`, `settings.lua`, les plugins tiers,
  les **images** des fonds d'écran, un `manifest.json`, et `monitors.lua`
  sur demande (il décrit les écrans de la machine).
- **À l'import** :
  - la configuration importée est vérifiée avant tout changement ;
  - la configuration en place est sauvegardée dans `hyprdesk/backups/` ;
  - les fonds sont copiés dans `~/.local/share/hyprdesk/wallpapers/<archive>/`,
    et leurs chemins réécrits ;
  - les plugins déjà présents sont gardés.

### Le thème seul

Un **thème** (`theme.rs`) réunit ce qui se voit : nom, couleurs (texte,
surfaces, accent, fond sans image), fonds d'écran, effet de chaque endroit
(opacité du fond et flou Hyprland de la barre, des menus, du lanceur, des
réglages ; opacité des widgets), mode du bureau et voile du verre, voile et
flou du verrouillage, transitions.

- **Export** (`--theme`, ou « Thème seul ») : le thème **effectif** (réglages
  et `hyprdesk.lua` confondus), écrit dans un `settings.lua` qui ne contient
  que lui, avec les images de ses fonds ; `manifest.json` porte
  `kind = "theme"` et un résumé (`theme` : nom, couleurs).
- **Import** : seul le thème change ; barre, raccourcis, widgets, plugins et
  écrans restent en place. Une archive de thème s'importe toujours ainsi ;
  le thème d'une archive complète aussi, sur demande (`--theme`). Un fond
  qui n'est ni dans l'archive ni présent sur la machine est écarté.
- **Mes thèmes** : des archives de thème rangées dans `hyprdesk/themes/`
  (paquet Thème : « Enregistrer dans Mes thèmes », un clic pour appliquer).
  Les thèmes livrés (`theme::PRESETS`) changent couleurs et effets, pas les
  fonds.

### Scènes multi-écrans

Une **scène** est une image pensée pour toute la disposition des écrans.

- **Découpe automatique** (`span.rs`) : `hd.wallpaper({ path = …, span = true })`
  (ou « Étaler sur tous les écrans », ou `wallpaper.span on`). Le canevas
  englobe les écrans tels que Hyprland les place (positions et tailles
  logiques) ; l'image le couvre, centrée, et chaque écran en montre sa part.
  Une disposition en L, empilée ou décalée reste continue ; les zones du
  canevas sans écran ne se voient pas. Un diaporama s'étale de même.
- **Résolution** : choisir une image trop petite pour les écrans visés (ou
  pour tout le canevas si elle est étalée) affiche un avertissement, non
  bloquant : facteur d'agrandissement et taille conseillée, en pixels
  physiques (échelle de Hyprland comprise).
- **Thème tiré d'une scène** (`theme::create`, `hyprdesk theme new`, ou
  « Créer depuis une image » dans le paquet Thème) : couleurs extraites de
  l'image (surfaces dans la teinte des ombres, accent dans la teinte vive
  dominante), scène étalée, surfaces translucides et floutées ; rangé dans
  Mes thèmes. La source peut être une image, ou un **pack** (dossier ou
  `.zip`) dont le `manifest.json` désigne la scène maîtresse (`master`) et
  son nom (`theme`) : les découpes figées du pack sont ignorées, la découpe
  se fait selon les écrans réels.
- **Verrouillage** : il montre la scène entière (même image que le bureau),
  sauf image propre (`lock_image`). La découpe par écran au verrouillage
  viendra quand l'écran de verrouillage saura quelle sortie porte chaque
  surface.
- `hyprdesk wallpaper split <image> [dossier] [--scale N]` écrit la part de
  chaque écran (`<écran>.png`), pour un autre outil ou pour vérifier.

## 9. Langue de l'interface (i18n)

hyprdesk parle français (langue source) ou anglais :
`hd.config({ language = "auto" | "fr" | "en" })`, Réglages → Bureau → Langue, ou
`hyprdesk config set language en`. `auto` suit `LC_ALL`, `LC_MESSAGES` puis `LANG`
(français si la langue est le français, anglais sinon). Une langue choisie vaut
aussi pour les dates ; `auto` laisse les dates suivre `LC_TIME`.

**Le texte français est la clé** (à la manière de gettext) : le catalogue
`locales/en.toml`, intégré au binaire, associe chaque texte français à sa
traduction. Un texte absent s'affiche en français : rien ne casse.

- **Vues (Rust)** : `text` vient de `crate::i18n` (et non d'iced) dans les vues ;
  il traduit tout texte présent au catalogue, littéral ou calculé. Les autres
  libellés passent par `crate::i18n::tr("…")` (`radio`, `.label`, `text_input`,
  `placeholder`, champs `label: …`) ; un texte à trous s'écrit
  `crate::tr!("Écran {mon}", mon = m)`. Les constantes (`INFO`, `TABS`, widgets
  intégrés) sont traduites à l'affichage.
- **Ligne de commande** : messages en `tr!`, aides traduites ligne à ligne
  (`i18n::tr_lines`, colonnes comprises).
- **Plugins Lua** : `hd.t("texte")` traduit, `hd.lang()` rend `"fr"` ou `"en"`
  pour un plugin qui a ses propres textes.
- **Restent en français** : journaux, fichiers générés (`settings.lua`,
  `monitors.lua`), contexte transmis à LYRA.
- **Vérification** : `i18n::tests::catalogue_complet` (textes marqués dans
  `src/` et `assets/`), `registres_traduits` (paquets, onglets, widgets),
  `aide_traduite` et `aide_config_traduite`. Pour compléter le catalogue :
  `HYPRDESK_I18N_TODO=manque.toml HYPRDESK_I18N_TODO_DIR=dossier cargo test i18n`
  écrit les entrées manquantes, à traduire puis ajouter à `locales/en.toml`.
