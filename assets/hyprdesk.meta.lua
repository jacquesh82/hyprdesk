---@meta
-- Définitions LuaLS de l'API hyprdesk (`hd.*`), à côté de celles de Hyprland
-- (/usr/share/hypr/stubs/hl.meta.lua). Générées par `hyprdesk stubs`.

---@alias HD.Selector string|integer|(string|integer)[]
---@alias HD.Anchor "top-left"|"top"|"top-right"|"left"|"center"|"right"|"bottom-left"|"bottom"|"bottom-right"
---@alias HD.Color string # "#rrggbb", "#rrggbbaa", "rgb(rrggbb)", "rgba(rrggbbaa)"

---@class HD.ConfigSpec
---@field mode? "opaque"|"glass"
---@field fade_ms? integer
---@field transition? "fade"|"slide"|"none"
---@field crossfade_ms? integer
---@field running_apps? "off"|"workspace"|"all"
---@field show_on? "all"|"focused"
---@field close_on_launch? boolean
---@field icon_theme? string
---@field font? string
---@field font_size? number
---@field terminal? string # "kitty" → "kitty -e <cmd>" ; ou gabarit "foot {cmd}"
---@field glass_opacity? number # 0..1
---@field background? HD.Color
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
---@field inherit? boolean # false : ignore les icônes et widgets déclarés pour tous les workspaces

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
---@field render? fun(): string # lua

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

---@param spec HD.IconSpec
function hd.icon(spec) end

---@param kind "clock"|"calendar"|"system"|"media"|"command"|"lua"
---@param spec? HD.WidgetSpec
function hd.widget(kind, spec) end

---@param event "show"|"hide"|"workspace"
---@param fn fun(e: HD.HookEvent)
function hd.on(event, fn) end

--- Exécute une commande shell et rend sa sortie standard.
---@param cmd string
---@return string
function hd.exec(cmd) end
