-- Mises à jour : paquets en attente (pacman, AUR, apt, dnf, zypper, flatpak),
-- mise à jour en un clic. Le travail long est fait par `hyprdesk updates`
-- (relevé, mise à jour non interactive par pkexec) ; ce plugin lit l'état
-- qu'il écrit et l'affiche. Un échec est confié au démon : l'assistant LYRA
-- s'il est là, sinon une fenêtre de détail copiable.
--
-- Pendant la mise à jour (MLG-161), l'avancement : « 42 % » en mode compact,
-- une barre de progression et le pourcentage en mode détaillé (option
-- `detaille`) ; l'info-bulle dit la source, la phase et le paquet en cours.
--
-- Le nombre de paquets en attente se lit dans un ovale posé sur le module,
-- comme les non-lues des notifications (HIG SYS-02).

-- Écrit par hyprdesk dans son dossier d'exécution privé (jamais /tmp).
local STATE = (hd.runtime_dir or os.getenv("XDG_RUNTIME_DIR") or "/nonexistent") .. "/hyprdesk-updates.txt"
-- Relevé relancé au plus toutes les heures.
local MAX_AGE = 3600
-- Choix « un seul mot de passe par mise à jour » (MLG-160), gardé par hyprdesk.
local POLKIT = (os.getenv("XDG_STATE_HOME") or ((os.getenv("HOME") or "") .. "/.local/state"))
    .. "/hyprdesk/updates-polkit"

local PHASES = {
    prepare  = hd.t("préparation"),
    download = hd.t("téléchargement"),
    check    = hd.t("vérifications"),
    install  = hd.t("installation"),
    hooks    = hd.t("finalisation"),
}

local function polkit_keep()
    local f = io.open(POLKIT, "r")
    if not f then return false end
    local v = f:read("*l")
    f:close()
    return v == "on"
end

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
        elseif k == "pid" then s.pid = tonumber(v)
        elseif k == "checked" then s.checked = tonumber(v) or 0
        elseif k == "total" then s.total = tonumber(v) or 0
        elseif k == "progress" then s.progress = tonumber(v)
        elseif k == "step" then s.step = v
        elseif k == "phase" then s.phase = v
        elseif k == "current" then s.current = v
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
    -- Tâche interrompue (processus disparu) : on ne reste pas « en cours ».
    if (s.state == "running" or s.state == "checking") and s.pid then
        local p = io.open("/proc/" .. s.pid .. "/stat", "r")
        if p then p:close() else s.state = "idle"; s.checked = 0 end
    end
    return s
end

-- « Dépôts Arch — installation : firefox »
local function progress_detail(s)
    local parts = {}
    if s.step and s.step ~= "" then parts[#parts + 1] = s.step end
    local phase = PHASES[s.phase or ""]
    if phase then
        parts[#parts + 1] = s.current and s.current ~= "" and (phase .. " : " .. s.current) or phase
    end
    return table.concat(parts, " — ")
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
    -- Lecture d'un petit fichier : assez souvent pour suivre une mise à jour.
    interval    = 2,
    anchor      = "top-right",
    -- texte : % pendant la mise à jour et « ! » après un échec (le nombre de
    -- paquets est dans l'ovale) ; detaille : barre de progression et %.
    options     = { texte = true, masquer_a_jour = false, detaille = false },
    render = function(o)
        local s = read()
        refresh_if_old(s)
        if not s or s.state == "checking" and s.checked == 0 then
            return { icon = "software-update-available-symbolic", text = "",
                     tooltip = hd.t("Recherche des mises à jour…") }
        end
        if s.state == "running" then
            local pct = s.progress
            local detail = progress_detail(s)
            local tip = pct and string.format(hd.t("Mise à jour en cours : %d %%"), pct)
                or hd.t("Mise à jour en cours…")
            if detail ~= "" then tip = tip .. "\n" .. detail end
            local label = pct and string.format("%d %%", pct) or "…"
            return { icon = "software-update-available-symbolic",
                     text = o.texte and label or "",
                     progress = o.detaille and (pct or 0) / 100 or nil,
                     tooltip = tip, state = "active" }
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
        return { icon = "software-update-available-symbolic", badge = s.total,
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
            m[#m + 1] = { label = s.progress and string.format(hd.t("Mise à jour en cours : %d %%"), s.progress)
                                  or hd.t("Mise à jour en cours…"),
                          detail = progress_detail(s) ~= "" and progress_detail(s) or nil }
        elseif s.total > 0 then
            m[#m + 1] = { label = hd.t("Tout mettre à jour"), icon = "software-update-available-symbolic",
                          confirm = string.format(hd.t("Mettre à jour %d paquet(s) ?"), s.total),
                          action = function() hd.spawn(exe("--upgrade")) end }
        end
        if s.state == "failed" then
            m[#m + 1] = { label = hd.t("Voir le problème…"), action = function() hd.spawn(exe("--report")) end }
        end
        m[#m + 1] = { label = hd.t("Vérifier maintenant"), action = function() hd.spawn(exe("--refresh")) end }
        m[#m + 1] = "-"
        m[#m + 1] = { label = hd.t("Un seul mot de passe par mise à jour"), checked = polkit_keep(),
                      detail = hd.t("5 min, gestionnaires de paquets (règle polkit)"),
                      action = function() hd.spawn(exe("--polkit toggle")) end }
        return m
    end,
}
