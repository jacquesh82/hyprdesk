-- hyprdesk — bureau à la demande pour Hyprland
-- Fichier : ~/.config/hypr/hyprdesk.lua (rechargé à chaud à chaque enregistrement)
-- Modules : ~/.config/hypr/hyprdesk/<nom>.lua, chargés avec require("<nom>")
-- Autocomplétion : `hyprdesk stubs` écrit les définitions LuaLS (---@meta).
--
-- Réglages à la souris : bouton ⚙ du bureau (ou `hyprdesk settings`). Ils sont
-- écrits dans ~/.config/hypr/hyprdesk/settings.lua, chargé AVANT ce fichier :
-- ce qui est écrit ici garde le dernier mot (le panneau le signale).
--
-- Sélecteurs : chaque règle accepte `workspace` et `monitor` (chaîne, nombre,
-- liste, ou "*" pour tous). Fond d'écran et mode : la règle la plus précise
-- l'emporte. Icônes et widgets : ils s'additionnent.

-- Tout ce qui est décommenté ici prend le pas sur l'écran ⚙.
hd.config({
    terminal = "kitty",
    -- keys = { toggle = "SUPER + D", settings = "SUPER + SHIFT + D", leave = "SUPER + Escape" },
    -- suspend_binds   = true,       -- bureau affiché : raccourcis de Hyprland suspendus
    -- float_launched  = true,       -- ouvert depuis le bureau affiché : flottant et centré
    -- mode            = "opaque",   -- "opaque" (fond d'écran) ou "glass" (fenêtres floutées)
    -- show_on         = "all",      -- "all" ou "focused"
    -- running_apps    = "off",      -- dock des fenêtres ouvertes : "off", "workspace", "all"
    -- transition      = "fade",     -- "fade", "slide", "none"
    -- fade_ms         = 150,
    -- crossfade_ms    = 700,
    -- close_on_launch = true,
    -- icon_theme      = "Papirus-Dark",
    -- grid  = { cell = 96, gap = 16, margin = 32, icon_size = 48 },
    -- theme = { fg = "#f8f8f2", bg = "#22212c", accent = "#9580ff" },
})

-- Fonds d'écran (ou onglet « Fond d'écran » de l'écran ⚙) ---------------------
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
hd.widget("media",    { anchor = "bottom-left", x = 48 })
-- hd.widget("command", { anchor = "top-left", exec = "curl -s 'wttr.in/?format=3'", interval = 900 })
-- hd.widget("lua", { anchor = "left", interval = 60, render = function()
--     return "Bonjour " .. (os.getenv("USER") or "") .. " — " .. os.date("%A %d %B")
-- end })

-- Barre native (ou Réglages ⚙ → Barre) : au-dessus des fenêtres, zone réservée.
-- Modules : "workspaces", "window", "volume", "desktop", ou tout widget/plugin.
-- hd.bar({
--     position = "top", height = 36,
--     left   = { "workspaces", "window" },
--     center = { { "clock", format = "%a %d %b  %H:%M" } },
--     right  = { "media", "volume", "desktop" },
-- })

-- Session : composants lancés et supervisés par hyprdesk (relancés s'ils
-- plantent, arrêtés avec lui). Retirez-les alors de l'autostart de Hyprland.
-- hd.component("wayle", { exec = "wayle shell" })
-- hd.component("presse-papiers", { exec = "wl-paste --type text --watch cliphist store" })

-- Hooks ---------------------------------------------------------------------
-- hd.on("show", function(e) print("bureau affiché sur " .. e.monitor .. ", workspace " .. e.workspace) end)
