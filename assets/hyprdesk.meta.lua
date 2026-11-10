---@meta
-- Définitions LuaLS de l'API hyprdesk (`hd.*`), à côté de celles de Hyprland
-- (/usr/share/hypr/stubs/hl.meta.lua). Générées par `hyprdesk stubs`.

---@alias HD.Selector string|integer|(string|integer)[]
---@alias HD.Anchor "top-left"|"top"|"top-right"|"left"|"center"|"right"|"bottom-left"|"bottom"|"bottom-right"
---@alias HD.Color string # "#rrggbb", "#rrggbbaa", "rgb(rrggbb)", "rgba(rrggbbaa)"

---@class HD.ConfigSpec
---@field keys? { toggle?: string|false, settings?: string|false, leave?: string|false }
---@field suspend_binds? boolean # bureau affiché : suspendre les raccourcis de Hyprland
---@field float_launched? boolean # ouvert depuis le bureau affiché : fenêtre flottante et centrée
---@field mode? "opaque"|"glass"
---@field fade_ms? integer
---@field transition? "fade"|"slide"|"none"
---@field crossfade_ms? integer
---@field running_apps? "off"|"workspace"|"all"
---@field show_on? "all"|"focused"|"primary"
---@field launcher_group? "none"|"app"|"workspace" # lanceur : une entrée par fenêtre, par application, ou par workspace
---@field primary? string|false # écran principal (connecteur, ex. "DP-1") : ancre en 0×0, bureau/barre/réglages
---@field close_on_launch? boolean
---@field icon_theme? string
---@field font? string # "Mindlog" par défaut (embarquée), ou une famille installée
---@field font_size? number
---@field terminal? string # "kitty" → "kitty -e <cmd>" ; ou gabarit "foot {cmd}"
---@field glass_opacity? number # 0..1
---@field background? HD.Color
---@field workspace_monitors? table<string, (string|integer)[]> # écran (ou "primary") → workspaces ; le premier est affiché par défaut
---@field workspaces? integer|false # workspaces 1..N permanents (1 à 30) ; false : laissés à Hyprland
---@field grid? { cell?: integer, gap?: integer, margin?: integer, icon_size?: integer }
---@field theme? { fg?: HD.Color, bg?: HD.Color, accent?: HD.Color }

---@class HD.RuleSpec
---@field workspace? HD.Selector # "*" = tous
---@field monitor? HD.Selector

---@class HD.WallpaperSpec: HD.RuleSpec
---@field path? string
---@field rotate? { dir: string, interval?: number, shuffle?: boolean }
---@field color? HD.Color
---@field fit? "cover"|"contain"|"fill"|"center"

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
---@field on_scroll? fun(direction: integer) # -1 ou +1
---@field menu? fun(): (HD.MenuItem|string)[]

---@class HD.Rendered
---@field text? string
---@field icon? string
---@field tooltip? string
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

---@param kind "clock"|"calendar"|"system"|"media"|"command"|"lua"|string # ou nom d'un plugin
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

---@alias HD.BarModule "workspaces"|"window"|"volume"|"tray"|"desktop"|string|table # ou widget : "clock", { "clock", format = "%H:%M" }

---@class HD.BarSpec
---@field monitor? HD.Selector # "primary" : l'écran principal
---@field compact? "auto"|"always"|"never" # auto : écrans de moins de 1600 px ou verticaux
---@field portrait? { left?: table, center?: table, right?: table } # modules des écrans verticaux
---@field position? "top"|"bottom"
---@field height? integer
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
---@field state? "normal"|"active"|"warning"|"critical"|"hidden"

---@class HD.MenuEntry
---@field header? string # titre de section
---@field label? string
---@field detail? string
---@field icon? string
---@field checked? boolean
---@field disabled? boolean
---@field confirm? string # question de confirmation avant l'action
---@field action? fun()

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

--- Lance une commande par Hyprland, sans attendre sa sortie.
---@param cmd string
function hd.spawn(cmd) end
