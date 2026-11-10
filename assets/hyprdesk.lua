-- hyprdesk — bureau à la demande pour Hyprland
-- Fichier : ~/.config/hypr/hyprdesk.lua (rechargé à chaud à chaque enregistrement)
-- Modules : ~/.config/hypr/hyprdesk/<nom>.lua, chargés avec require("<nom>")
-- Autocomplétion : `hyprdesk stubs` écrit les définitions LuaLS (---@meta).
--
-- Sélecteurs : chaque règle accepte `workspace` et `monitor` (chaîne, nombre,
-- liste, ou "*" pour tous). Fond d'écran et mode : la règle la plus précise
-- l'emporte. Icônes et widgets : ils s'additionnent.

hd.config({
    mode            = "opaque",   -- "opaque" (fond d'écran) ou "glass" (fenêtres floutées)
    fade_ms         = 150,
    show_on         = "all",      -- "all" ou "focused"
    close_on_launch = true,
    terminal        = "kitty",
    -- icon_theme   = "Papirus-Dark",
    grid  = { cell = 96, gap = 16, margin = 32, icon_size = 48 },
    theme = { fg = "#f8f8f2", bg = "#22212c", accent = "#9580ff" },
})

-- Fonds d'écran -------------------------------------------------------------
hd.wallpaper({ color = "#22212c" })
-- hd.wallpaper({ path = "~/.wallpapers/default.jpg", fit = "cover" })
-- hd.wallpaper({ monitor = "DP-1", path = "~/.wallpapers/portrait.png" })
-- hd.wallpaper({ workspace = 3, rotate = { dir = "~/.wallpapers", interval = 600, shuffle = true } })

-- Bureaux par workspace -----------------------------------------------------
-- hd.desktop({ workspace = 4, mode = "glass" })
-- hd.desktop({ workspace = "special:magic", inherit = false })

-- Icônes (app = identifiant .desktop, path = fichier/dossier, exec = commande)
hd.icon({ app = "kitty" })
hd.icon({ label = "Dossier perso", path = "~", icon = "user-home" })
hd.icon({ label = "htop", exec = "htop", terminal = true, icon = "utilities-system-monitor" })

-- Widgets -------------------------------------------------------------------
hd.widget("clock",    { anchor = "top-right", format = "%H:%M", size = 72, card = false })
hd.widget("calendar", { anchor = "right" })
hd.widget("system",   { anchor = "bottom-right", show = { "cpu", "ram", "disk:/" } })
hd.widget("media",    { anchor = "bottom" })
-- hd.widget("command", { anchor = "top-left", exec = "curl -s 'wttr.in/?format=3'", interval = 900 })
-- hd.widget("lua", { anchor = "left", interval = 60, render = function()
--     return "Bonjour " .. (os.getenv("USER") or "") .. " — " .. os.date("%A %d %B")
-- end })

-- Hooks ---------------------------------------------------------------------
-- hd.on("show", function(e) print("bureau affiché sur " .. e.monitor .. ", workspace " .. e.workspace) end)
