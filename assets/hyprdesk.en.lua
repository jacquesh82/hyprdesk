-- hyprdesk — on-demand desktop for Hyprland
-- File: ~/.config/hypr/hyprdesk.lua (hot-reloaded on every save)
-- Modules: ~/.config/hypr/hyprdesk/<name>.lua, loaded with require("<name>")
-- Autocompletion: `hyprdesk stubs` writes the LuaLS definitions (---@meta).
--
-- Point-and-click settings: the desktop's ⚙ button (or `hyprdesk settings`). They are
-- written to ~/.config/hypr/hyprdesk/settings.lua, loaded BEFORE this file:
-- whatever is written here has the last word (the panel points it out).
--
-- Selectors: every rule accepts `workspace` and `monitor` (string, number,
-- list, or "*" for all). Wallpaper and mode: the most specific rule
-- wins. Icons and widgets: they add up.

-- Anything uncommented here takes precedence over the ⚙ screen.
hd.config({
    terminal = "kitty",
    -- keys = { toggle = "SUPER + D", settings = "SUPER + SHIFT + D", lock = "SUPER + ALT + L", leave = "SUPER + Escape" },
    -- suspend_binds   = true,       -- desktop shown: Hyprland shortcuts suspended
    -- float_launched  = true,       -- opened from the shown desktop: floating and centered
    -- mode            = "opaque",   -- "opaque" (wallpaper) or "glass" (blurred windows)
    -- show_on         = "all",      -- "all" or "focused"
    -- running_apps    = "off",      -- dock of open windows: "off", "workspace", "all"
    -- transition      = "fade",     -- "fade", "slide", "none"
    -- fade_ms         = 150,
    -- crossfade_ms    = 700,
    -- icon_scope      = "global",   -- icons: "global", "screen" (per screen) or "workspace"
    -- icon_double_click = true,    -- double click to open (one click selects)
    -- icon_theme      = "Papirus-Dark",
    -- font            = "Mindlog",   -- embedded font by default, or an installed family
    -- font_size       = 14,
    -- grid  = { cell = 96, gap = 16, margin = 32, icon_size = 48 },
    -- theme = { name = "Ardoise", fg = "#f2f4fc", bg = "#171b27", accent = "#a69cff",
    --           effects = { bar = { opacity = 0.72, blur = true }, widgets = { opacity = 0.84 } } },
})

-- Wallpapers (or the "Wallpaper" tab of the ⚙ screen) -----------------------
-- hd.wallpaper({ path = "~/.wallpapers/default.jpg", fit = "cover" })
-- hd.wallpaper({ monitor = "DP-1", path = "~/.wallpapers/portrait.png" })
-- hd.wallpaper({ workspace = 3, rotate = { dir = "~/.wallpapers", interval = 600, shuffle = true } })
-- One scene for the whole layout: each screen shows its share of it.
-- hd.wallpaper({ path = "~/.wallpapers/panorama.jpg", span = true })

-- Desktops per workspace ----------------------------------------------------
-- hd.desktop({ workspace = 4, mode = "glass" })
-- hd.desktop({ workspace = "special:magic", inherit = false })

-- Icons (app = .desktop identifier, path = file/folder, exec = command)
hd.icon({ app = "kitty" })
hd.icon({ label = "Home folder", path = "~", icon = "user-home" })
hd.icon({ label = "htop", exec = "htop", terminal = true, icon = "utilities-system-monitor" })

-- Widgets -------------------------------------------------------------------
hd.widget("clock",    { anchor = "top-right", format = "%H:%M", size = 72, card = false })
hd.widget("calendar", { anchor = "right" })
hd.widget("system",   { anchor = "bottom-right", show = { "cpu", "ram", "disk:/" } })
hd.widget("media",    { anchor = "bottom-left", x = 48 })
-- hd.widget("command", { anchor = "top-left", exec = "curl -s 'wttr.in/?format=3'", interval = 900 })
-- hd.widget("lua", { anchor = "left", interval = 60, render = function()
--     return "Hello " .. (os.getenv("USER") or "") .. " — " .. os.date("%A %d %B")
-- end })

-- Native bar (or Settings ⚙ → Bar): above the windows, reserved area.
-- Modules: "workspaces", "window", "volume", "desktop", or any widget/plugin.
hd.bar({
    position = "top", height = 36,
    left   = { "workspaces", "window" },
    center = { { "clock", format = "%a %d %b  %H:%M" } },
    right  = { "tray", "media", "reseau", "bluetooth", "batterie", "volume", "notifications", "desktop", "session" },
})

-- Session: components launched and supervised by hyprdesk (restarted if they
-- crash, stopped along with it). Remove them from Hyprland's autostart then.
-- hd.component("wayle", { exec = "wayle shell" })
-- hd.component("clipboard", { exec = "wl-paste --type text --watch cliphist store" })

-- Hooks ---------------------------------------------------------------------
-- hd.on("show", function(e) print("desktop shown on " .. e.monitor .. ", workspace " .. e.workspace) end)
