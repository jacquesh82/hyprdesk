-- Mises à jour : paquets en attente (pacman, AUR, apt, dnf, zypper, flatpak),
-- mise à jour en un clic. Le travail long est fait par `hyprdesk updates`
-- (relevé, mise à jour non interactive par pkexec) ; ce plugin lit l'état
-- qu'il écrit et l'affiche. Un échec est confié au démon : l'assistant LYRA
-- s'il est là, sinon une fenêtre de détail copiable.

local STATE = (os.getenv("XDG_RUNTIME_DIR") or "/tmp") .. "/hyprdesk-updates.txt"
-- Relevé relancé au plus toutes les heures.
local MAX_AGE = 3600

local function exe(arg)
    return string.format("%q updates %s", hd.exe, arg)
end

local function read()
    local f = io.open(STATE, "r")
    if not f then return nil end
    local s = { state = "idle", checked = 0, total = 0, sources = {} }
    for line in f:lines() do
        local k, v = line:match("^([a-z]+)=(.*)$")
        if k == "state" then s.state = v
        elseif k == "checked" then s.checked = tonumber(v) or 0
        elseif k == "total" then s.total = tonumber(v) or 0
        elseif k == "source" then
            local label, n, pkgs = v:match("^(.-)|(%d+)|(.*)$")
            if label then
                local list = {}
                for p in pkgs:gmatch("%S+") do list[#list + 1] = p end
                s.sources[#s.sources + 1] = { label = label, n = tonumber(n), packages = list }
            end
        end
    end
    f:close()
    return s
end

local function refresh_if_old(s)
    local busy = s and (s.state == "checking" or s.state == "running")
    if not busy and (not s or os.time() - s.checked > MAX_AGE) then
        hd.spawn(exe("--refresh"))
    end
end

return {
    name        = hd.t("Mises à jour"),
    description = hd.t("Paquets à mettre à jour ; mise à jour en un clic. Tout à jour : une coche discrète (option masquer_a_jour pour la cacher)."),
    interval    = 60,
    anchor      = "top-right",
    options     = { texte = true, masquer_a_jour = false },
    render = function(o)
        local s = read()
        refresh_if_old(s)
        if not s or s.state == "checking" and s.checked == 0 then
            return { icon = "software-update-available-symbolic", text = "",
                     tooltip = hd.t("Recherche des mises à jour…") }
        end
        if s.state == "running" then
            return { icon = "software-update-available-symbolic", text = o.texte and "…" or "",
                     tooltip = hd.t("Mise à jour en cours…"), state = "active" }
        end
        if s.state == "failed" then
            return { icon = "software-update-available-symbolic", text = o.texte and "!" or "",
                     tooltip = hd.t("La dernière mise à jour a échoué"), state = "critical" }
        end
        if s.total == 0 then
            if o.masquer_a_jour then return { state = "hidden" } end
            return { icon = "software-update-none-symbolic", text = "",
                     tooltip = hd.t("Le système est à jour") }
        end
        return { icon = "software-update-available-symbolic", text = o.texte and tostring(s.total) or "",
                 tooltip = string.format(hd.t("%d paquet(s) à mettre à jour"), s.total),
                 state = s.total >= 50 and "warning" or "normal" }
    end,
    menu = function()
        local s = read() or { state = "idle", total = 0, sources = {} }
        local m = { { header = s.total > 0 and string.format(hd.t("%d paquet(s) à mettre à jour"), s.total)
                                  or hd.t("Le système est à jour") } }
        for _, src in ipairs(s.sources) do
            if src.n > 0 then
                local first = {}
                for i = 1, math.min(4, #src.packages) do first[#first + 1] = src.packages[i] end
                m[#m + 1] = { label = src.label .. " : " .. src.n, detail = table.concat(first, ", ") }
            end
        end
        m[#m + 1] = "-"
        if s.state == "running" then
            m[#m + 1] = { label = hd.t("Mise à jour en cours…") }
        elseif s.total > 0 then
            m[#m + 1] = { label = hd.t("Tout mettre à jour"), icon = "software-update-available-symbolic",
                          confirm = string.format(hd.t("Mettre à jour %d paquet(s) ?"), s.total),
                          action = function() hd.spawn(exe("--upgrade")) end }
        end
        if s.state == "failed" then
            m[#m + 1] = { label = hd.t("Voir le problème…"), action = function() hd.spawn(exe("--report")) end }
        end
        m[#m + 1] = { label = hd.t("Vérifier maintenant"), action = function() hd.spawn(exe("--refresh")) end }
        return m
    end,
}
