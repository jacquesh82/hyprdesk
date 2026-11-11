-- Clavier virtuel (wvkbd) : l'afficher ou le masquer depuis la barre.
-- Sans wvkbd : icône estompée, le clic dit quel paquet installer.
-- wvkbd reçoit SIGRTMIN pour se montrer ou se cacher ; il est lancé caché
-- (--hidden) au premier clic s'il ne tourne pas.

local PROGS = { "wvkbd-mobintl", "wvkbd-deskintl", "wvkbd" }

local function prog()
    for _, p in ipairs(PROGS) do
        if hd.exec("command -v " .. p .. " 2>/dev/null") ~= "" then return p end
    end
    return nil
end

local function running(p)
    return hd.exec("pgrep -x " .. p .. " 2>/dev/null") ~= ""
end

local function toggle(p)
    if running(p) then
        hd.exec("pkill -RTMIN -x " .. p)
    else
        hd.spawn(p .. " -L 280")
    end
end

return {
    name        = hd.t("Clavier virtuel"),
    description = hd.t("Affiche ou masque le clavier à l'écran (wvkbd)."),
    interval    = 10,
    anchor      = "top-right",
    options     = {},
    render = function()
        local p = prog()
        if not p then
            return { icon = "input-keyboard-symbolic", state = "warning",
                     tooltip = hd.t("Clavier virtuel : wvkbd n'est pas installé (paquet wvkbd)") }
        end
        return { icon = "input-keyboard-symbolic", state = running(p) and "active" or "normal",
                 tooltip = hd.t("Clavier virtuel — clic : afficher ou masquer") }
    end,
    on_click = function()
        local p = prog()
        if p then
            toggle(p)
        else
            hd.spawn("notify-send -a hyprdesk " .. string.format("%q", hd.t("Clavier virtuel"))
                .. " " .. string.format("%q", hd.t("wvkbd n'est pas installé : installez le paquet wvkbd.")))
        end
    end,
    menu = function()
        local p = prog()
        local m = { { header = hd.t("Clavier virtuel") } }
        if not p then
            m[#m + 1] = { label = hd.t("wvkbd n'est pas installé") }
            return m
        end
        m[#m + 1] = { label = hd.t("Afficher ou masquer"), action = function() toggle(p) end }
        if running(p) then
            m[#m + 1] = { label = hd.t("Quitter le clavier"), action = function() hd.exec("pkill -x " .. p) end }
        end
        return m
    end,
}
