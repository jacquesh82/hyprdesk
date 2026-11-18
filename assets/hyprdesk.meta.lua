---@meta
-- Définitions LuaLS de l'API hyprdesk (`hd.*`), à côté de celles de Hyprland
-- (/usr/share/hypr/stubs/hl.meta.lua). Générées par `hyprdesk stubs`.

---@alias HD.Selector string|integer|(string|integer)[]
---@alias HD.Anchor "top-left"|"top"|"top-right"|"left"|"center"|"right"|"bottom-left"|"bottom"|"bottom-right"
---@alias HD.Color string # "#rrggbb", "#rrggbbaa", "rgb(rrggbb)", "rgba(rrggbbaa)"

---@class HD.ConfigSpec
---@field keys? { toggle?: string|false, launcher?: string|false, hide?: string|false, settings?: string|false, lock?: string|false, leave?: string|false, switch?: string|false, search?: string|false, search_desktop?: string|false, overview?: string|false, take?: string[]|false, media?: boolean }
---@field suspend_binds? boolean # bureau affiché : suspendre les raccourcis de Hyprland
---@field clipboard_history? boolean # historique du presse-papiers : hyprdesk lance cliphist (« :clip » dans la recherche)
---@field plugin_options? table<string, table<string, boolean|number|string>> # options des plugins (Réglages → Plugins → Configurer), pour chaque instance sauf les options écrites sur l'instance
---@field update_commands? table<string, { check?: string, upgrade?: string }> # commandes des mises à jour par source (pacman, aur, apt, dnf, zypper, flatpak), en shell ; vide : celle de hyprdesk
---@field forced_modes? table<string, "no-desktop"|"launcher-only"|"full-desktop"> # mode imposé : écran ("DP-1") ou workspace ("ws:5") ; no-desktop : jamais de bureau là ; les autres : le bureau y passe en arrivant
---@field panorama? { screens?: string[], pages?: ("media"|"system"|"clock")[], declined?: string[], sleep_minutes?: integer } # Panorama (écrans bandeau tactiles) : écrans au rôle (connecteur "DP-3" ou identifiant EDID), pages dans l'ordre, propositions refusées, minutes avant l'horloge de veille (0 : jamais)
---@field autostart? boolean # lancer les applications du démarrage automatique XDG (~/.config/autostart, /etc/xdg/autostart), une fois par session
---@field desktop_files? boolean # fichiers du dossier Bureau (xdg-user-dir DESKTOP) et Corbeille en icônes du bureau
---@field float_launched? boolean # ouvert depuis le bureau affiché : fenêtre flottante et centrée
---@field fx? table # animations et effets de Hyprland : { ["decoration.blur.variant"] = 9, animations = { windowsIn = { speed = 4, curve = "spring", style = "popin" } } }
---@field lock_fingerprint? boolean # empreinte (fprintd) à la place du mot de passe, si un lecteur est là (défaut : false)
---@field idle_manage? boolean # hyprdesk gère la veille : écrit la config de hypridle et le lance
---@field idle_dim? integer # secondes sans activité avant d'assombrir l'écran (0 : jamais)
---@field idle_lock? integer # secondes avant de verrouiller (0 : jamais)
---@field idle_dpms? integer # secondes avant d'éteindre les écrans (0 : jamais)
---@field idle_suspend? integer # secondes avant la mise en veille (0 : jamais)
---@field mode? "opaque"|"glass"
---@field fade_ms? integer
---@field transition? "fade"|"slide"|"none"
---@field crossfade_ms? integer
---@field running_apps? "off"|"workspace"|"all"
---@field show_on? "all"|"focused"|"primary"
---@field apps_style? "auto"|"plein"|"central" # lanceur d'applications : grande feuille en plein bureau et carte centrée sinon (auto), ou toujours l'une
---@field apps_suggestions? integer # applications les plus lancées mises en avant (0 à 16, 8 par défaut)
---@field apps_categories? boolean # catégories dans la grande feuille
---@field apps_unified? boolean # le même lanceur dans tous les modes (keys.search pris à hyprland.lua) ; false : celui de hyprland.lua hors plein bureau
---@field launcher_group? "none"|"app"|"workspace" # lanceur : une entrée par fenêtre, par application, ou par workspace
---@field primary? string|false # écran principal (connecteur, ex. "DP-1") : ancre en 0×0, bureau/barre/réglages
---@field close_on_launch? boolean
---@field icon_scope? "global"|"screen"|"workspace"
---@field workspace_selector? "relief"|"rail"|"miniatures"|"icones" # style du sélecteur de workspaces dans la barre
---@field icon_double_click? boolean
---@field icon_theme? string
---@field font? string # "Mindlog" par défaut (embarquée), ou une famille installée
---@field font_size? number
---@field advanced_settings? boolean Écran Réglages : tous les détails (sinon l'essentiel)
---@field tour_seen? boolean Visite guidée faite ou écartée (plus proposée)
---@field language? "auto"|"fr"|"en" Langue de l'interface (auto : celle du système)
---@field renderer? "hardware"|"software" Moteur de rendu de l'interface : carte graphique (wgpu, repli logiciel) ou processeur (tiny-skia) ; pris en compte à la relance
---@field gpu? "low"|"high" Carte graphique du rendu matériel : économe (intégrée) ou puissante (dédiée) ; à la relance
---@field worker_threads? integer Fils d'exécution de l'interface (défaut 4, 0 : un par cœur) ; à la relance
---@field layout? string Layout par défaut : "dwindle", "master", "scrolling", "monocle", "grid", "columns", "rows", "centered" ou "lua:<nom>"
---@field layout_workspaces? table<string|integer, string> Layout de certains workspaces ({ [3] = "monocle" })
---@field dwindle? table<string, boolean|number> Options de Dwindle (preserve_split, smart_split, force_split, default_split_ratio…)
---@field master? table<string, boolean|number|string> Options de Master (orientation, mfact, new_status, new_on_top…)
---@field scrolling? table<string, boolean|number|string> Options de Scrolling (direction, column_width, follow_focus…)
---@field terminal? string # "kitty" → "kitty -e <cmd>" ; ou gabarit "foot {cmd}"
---@field glass_opacity? number # 0..1
---@field background? HD.Color
---@field workspace_monitors? table<string, (string|integer)[]> # écran (ou "primary") → workspaces ; le premier est affiché par défaut
---@field workspaces? integer|false # workspaces 1..N permanents (1 à 30) ; false : laissés à Hyprland
---@field grid? { cell?: integer, gap?: integer, margin?: integer, icon_size?: integer }
---@field theme? HD.Theme

---@class HD.Effect
---@field opacity? number # 0..1 : opacité du fond
---@field blur? boolean # flou de ce qu'il y a derrière (pas pour les widgets)

---@class HD.Theme
---@field name? string
---@field fg? HD.Color # texte
---@field bg? HD.Color # surfaces
---@field accent? HD.Color
---@field effects? { bar?: HD.Effect, menus?: HD.Effect, launcher?: HD.Effect, settings?: HD.Effect, widgets?: HD.Effect }

---@class HD.RuleSpec
---@field workspace? HD.Selector # "*" = tous
---@field monitor? HD.Selector

---@class HD.WallpaperSpec: HD.RuleSpec
---@field path? string
---@field rotate? { dir: string, interval?: number, shuffle?: boolean }
---@field color? HD.Color
---@field fit? "cover"|"contain"|"fill"|"center"
---@field span? boolean|"lock"|"desktop" # une scène sur tous les écrans, découpée selon leur disposition ; "lock" : au verrouillage seulement (pas avec color)

---@class HD.DesktopSpec: HD.RuleSpec
---@field mode? "opaque"|"glass"
---@field inherit? boolean # false : ignore les icônes et widgets globaux

---@class HD.IconSpec: HD.RuleSpec
---@field id? string
---@field label? string
---@field app? string # identifiant .desktop
---@field path? string
---@field exec? string
---@field terminal? boolean
---@field icon? string # nom d'icône du thème ou chemin d'image
---@field pos? integer[] # { colonne, ligne }, à partir de 0

---@class HD.WidgetSpec: HD.RuleSpec
---@field id? string
---@field anchor? HD.Anchor
---@field x? number
---@field y? number
---@field size? number
---@field interval? number # secondes
---@field card? boolean
---@field format? string # clock : strftime
---@field show? string[] # system : "cpu", "ram", "load", "uptime", "disk:<point de montage>"
---@field player? string # media : sous-chaîne du nom MPRIS, "*" = tous
---@field exec? string # command
---@field timeout? number # command, secondes
---@field render? fun(): string|number|HD.Rendered # lua
---@field on_click? fun()
---@field on_toggle? fun()
---@field on_scroll? fun(direction: integer) # -1 ou +1
---@field menu? fun(): (HD.MenuItem|string)[]

---@class HD.Rendered
---@field text? string
---@field icon? string
---@field tooltip? string
---@field toggled? boolean
---@field progress? number # 0 à 1 : barre de progression dans le module
---@field state? "normal"|"active"|"warning"|"critical"|"hidden"

---@class HD.MenuItem
---@field label? string
---@field detail? string
---@field icon? string
---@field checked? boolean
---@field disabled? boolean
---@field action? fun()
---@field confirm? string
---@field header? string

---@class HD.HookEvent
---@field event "show"|"hide"|"workspace"
---@field monitor string
---@field workspace string

---@class HD.API
---@field version string
---@field exe string Chemin du hyprdesk en cours (ex. hd.spawn(hd.exe .. " lock")).
---@field runtime_dir string? Dossier d'exécution privé et vérifié (0700) ; nil s'il n'y en a pas. Jamais /tmp.
hd = {}

---@param spec HD.ConfigSpec
function hd.config(spec) end

---@param spec HD.WallpaperSpec
function hd.wallpaper(spec) end

---@param spec HD.DesktopSpec
function hd.desktop(spec) end

---@class HD.AssignSpec
---@field app? string # identifiant .desktop (classe de fenêtre déduite)
---@field class? string # ou expression régulière de classe, pour Hyprland
---@field workspace string|integer
---@field silent? boolean # s'ouvrir là-bas sans y emmener l'utilisateur

--- Les fenêtres d'une application s'ouvrent sur un workspace.
---@param spec HD.AssignSpec
function hd.assign(spec) end

---@param spec HD.IconSpec
function hd.icon(spec) end

---@param kind "controls"|"clock"|"calendar"|"system"|"media"|"command"|"lua"|string # ou nom d'un plugin
---@param spec? HD.WidgetSpec
function hd.widget(kind, spec) end

--- Repositionne un widget par son id (`clock#0` pour le premier `hd.widget("clock")`).
---@param id string
---@param spec { anchor?: HD.Anchor, x?: number, y?: number, size?: number, card?: boolean }
function hd.place(id, spec) end

--- Retire un widget par son id (écrit par l'écran Réglages quand on éteint un widget de hyprdesk.lua).
---@param id string
function hd.disable(id) end

---@param event "show"|"hide"|"workspace"
---@param fn fun(e: HD.HookEvent)
function hd.on(event, fn) end

---@alias HD.BarModule "workspaces"|"window"|"volume"|"brightness"|"tray"|"desktop"|string|table # ou widget : "clock", { "clock", format = "%H:%M" }

---@class HD.BarSpec
---@field monitor? HD.Selector # "primary" : l'écran principal
---@field compact? "auto"|"always"|"never" # auto : selon la largeur mesurée des modules, ou écran vertical
---@field portrait? { left?: table, center?: table, right?: table } # modules des écrans verticaux
---@field position? "top"|"bottom"
---@field height? integer # défaut du style : 36 en Continuité, 48 en Graphite
---@field margin? integer # marge au bord de l'écran (0 : collée) ; défaut du style : 0 dans les deux styles
---@field style? "continuite"|"graphite" # rendu complet de la barre ; graphite se pose en bas par défaut
---@field left? HD.BarModule[]
---@field center? HD.BarModule[]
---@field right? HD.BarModule[]

--- Barre native de hyprdesk ; `false` la retire.
---@param spec HD.BarSpec|false
function hd.bar(spec) end

--- Composant de session lancé et supervisé par hyprdesk.
---@param name string
---@param spec { exec: string, restart?: boolean }
function hd.component(name, spec) end

---@class HD.Rendered
---@field text? string
---@field icon? string # nom d'icône du thème (« …-symbolic » : teintée) ou chemin
---@field tooltip? string
---@field toggled? boolean
---@field progress? number # 0 à 1 : barre de progression dans le module
---@field state? "normal"|"active"|"warning"|"critical"|"hidden"

---@class HD.MenuEntry
---@field header? string # titre de section
---@field label? string
---@field detail? string
---@field icon? string
---@field checked? boolean
---@field disabled? boolean
---@field confirm? string # question de confirmation avant l'action
---@field input? string # champ de saisie (texte d'aide) : Entrée appelle action(texte)
---@field image? string # image locale (chemin ou file:///…), une pochette par exemple
---@field secret? boolean # saisie masquée (mot de passe)
---@field action? fun(text?: string)

---@class HD.Plugin
---@field name? string
---@field description? string
---@field interval? number
---@field anchor? HD.Anchor
---@field options? table<string, any>
---@field render fun(o: table): string|HD.Rendered
---@field on_click? fun(o: table)
---@field on_scroll? fun(o: table, dir: integer)
---@field menu? fun(o: table): (HD.MenuEntry|"-")[]

--- Lance un programme sans attendre (par Hyprland).
---@param cmd string
function hd.spawn(cmd) end

--- Exécute une commande shell et rend sa sortie standard.
---@param cmd string
---@return string
function hd.exec(cmd) end

--- Crée un fichier 0600 de façon exclusive, sans suivre de lien, et y écrit `data`.
---@param path string
---@param data string
---@return boolean ok, string? erreur
function hd.write_private(path, data) end

--- Lance une commande par Hyprland, sans attendre sa sortie.
---@param cmd string
function hd.spawn(cmd) end

-- État système sans processus : tenu à jour par D-Bus (signaux) dans
-- hyprdesk, lu aussitôt. nil : service injoignable, à remplacer par hd.exec.

---@class HD.NetConnection
---@field name string # nom de la connexion NetworkManager
---@field type string # "wifi", "ethernet", "vpn", "wireguard", "bridge"…
---@field active boolean # activée
---@field external boolean # créée hors de NetworkManager (pont docker…)
---@field primary boolean
---@field ssid? string # Wi-Fi : réseau du point d'accès actif
---@field strength? integer # Wi-Fi : force du signal, 0 à 100

---@class HD.Network
---@field state string # "connected", "connecting", "disconnected", "asleep"…
---@field connected boolean
---@field wifi_enabled boolean
---@field primary? HD.NetConnection
---@field connections HD.NetConnection[]

--- NetworkManager : état, Wi-Fi, connexions actives.
---@return HD.Network?
function hd.network() end

---@class HD.BtDevice
---@field name string
---@field address string
---@field paired boolean
---@field connected boolean
---@field battery? integer # pourcentage, si l'appareil le donne

---@class HD.Bluetooth
---@field adapter boolean # au moins un adaptateur
---@field powered boolean
---@field devices HD.BtDevice[] # appairés ou connectés

--- BlueZ : adaptateur et appareils.
---@return HD.Bluetooth?
function hd.bluetooth() end

--- Profil d'énergie (power-profiles-daemon) : actif et proposés.
---@return { active: string, profiles: string[] }?
function hd.power_profile() end

--- Batteries de /sys/class/power_supply (jamais nil : liste vide sans batterie).
---@return { name: string, cap: integer, status: string, device: boolean }[]
function hd.batteries() end

---@class HD.Gpu
---@field address string # adresse PCI (0000:01:00.0)
---@field name string # « GeForce RTX 3070 »
---@field driver string # nvidia | vfio-pci | none
---@field state "nvidia"|"display"|"vfio"|"vm"|"none" # display : nvidia avec les sorties d'affichage (nvidia_drm) ; vm : sur vfio-pci et tenu par une VM
---@field boot "nvidia"|"display"|"vfio" # pilote des prochains démarrages
---@field boot_default "nvidia"|"vfio" # celui du démarrage sans choix de hyprdesk
---@field boot_saved boolean # un choix de hyprdesk est gardé pour le démarrage
---@field nvidia_module boolean # le pilote nvidia est installé pour ce noyau
---@field display boolean # la carte a ses sorties d'affichage (nvidia_drm)
---@field busy boolean # une bascule `hyprdesk gpu` est en cours
---@field ready boolean # un hyprdesk root sait basculer (pkexec)

--- GPU NVIDIA basculables entre vfio-pci (VM) et nvidia, lus dans sysfs
--- (jamais nil : liste vide sans passthrough). Bascule : `hyprdesk gpu`.
---@return HD.Gpu[]
function hd.gpus() end

--- Démon d'index de fichiers (son socket) ; nil s'il est arrêté.
---@return { phase: string, fichiers: integer, progression: integer }?
function hd.index_status() end

--- Notifications, quand hyprdesk tient lui-même le serveur ; nil sinon.
---@return { source: "hyprdesk", dnd: boolean, count: integer, list: { app: string, title: string, body: string }[] }?
function hd.notifications() end

--- Texte dans la langue de l'interface : le français sert de clé dans le
--- catalogue de hyprdesk (locales/en.toml) ; absent, il est rendu tel quel.
---@param text string
---@return string
function hd.t(text) end

--- Langue de l'interface : "fr" ou "en" (pour les plugins qui ont leurs textes).
---@return "fr"|"en"
function hd.lang() end
