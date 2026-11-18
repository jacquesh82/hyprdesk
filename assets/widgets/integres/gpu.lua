-- GPU NVIDIA réservé aux VM (passthrough vfio-pci) : bascule à chaud vers le
-- pilote nvidia (CUDA, NVENC, conteneurs) et retour, sans redémarrer (MLG-162).
--
-- L'état vient de hd.gpus() (sysfs, sans processus). La bascule passe par
-- `hyprdesk gpu`, qui demande le droit d'administration (pkexec) ; la carte
-- n'affiche jamais le bureau (nvidia_drm reste déchargé). À chaque bascule,
-- le menu pose la question : cette session seulement, ou aussi aux prochains
-- démarrages (service système hyprdesk-gpu, la ligne de démarrage n'est pas
-- touchée).
--
-- Sans GPU à basculer (pas de passthrough), le module se cache.

local LABELS = { nvidia = "NVIDIA", vfio = "VFIO", vm = "VM", none = "?" }

local function driver_name(t)
    return t == "nvidia" and "nvidia" or "vfio-pci"
end

local function cmd(g, target, boot)
    return string.format("%q gpu %s --device %s%s", hd.exe, target, g.address, boot and " --boot" or "")
end

local function tooltip(g)
    local lines = { g.name .. " (" .. g.address:gsub("^0000:", "") .. ")" }
    if g.busy then
        lines[#lines + 1] = hd.t("Bascule en cours…")
    elseif g.state == "nvidia" then
        lines[#lines + 1] = hd.t("Sur l'hôte (nvidia) : CUDA, NVENC, conteneurs")
    elseif g.state == "vm" then
        lines[#lines + 1] = hd.t("Utilisé par une VM : bascule impossible tant qu'elle tourne")
    elseif g.state == "vfio" then
        lines[#lines + 1] = hd.t("Réservé aux VM (vfio-pci)")
    else
        lines[#lines + 1] = hd.t("Sans pilote")
    end
    lines[#lines + 1] = string.format(hd.t("Au démarrage : %s"), driver_name(g.boot))
    if not g.ready then lines[#lines + 1] = hd.t("Bascule indisponible : hyprdesk root trop ancien") end
    return table.concat(lines, "\n")
end

-- Question posée à chaque bascule : la garder au démarrage ou non.
local function ask(m, g, target, title)
    m[#m + 1] = { header = title }
    if g.boot == target then
        -- Le démarrage donne déjà ce pilote : rien à garder.
        m[#m + 1] = { label = hd.t("Basculer"), icon = "hyprdesk-gpu-symbolic",
                      detail = string.format(hd.t("déjà le pilote du démarrage (%s)"), driver_name(target)),
                      action = function() hd.spawn(cmd(g, target, false)) end }
        return
    end
    m[#m + 1] = { label = hd.t("Cette session seulement"), icon = "hyprdesk-gpu-symbolic",
                  detail = string.format(hd.t("au prochain démarrage : %s"), driver_name(g.boot)),
                  action = function() hd.spawn(cmd(g, target, false)) end }
    m[#m + 1] = { label = hd.t("Aussi aux prochains démarrages"), icon = "hyprdesk-gpu-symbolic",
                  detail = string.format(hd.t("le démarrage passera sur %s"), driver_name(target)),
                  confirm = string.format(hd.t("Démarrer désormais avec %s sur %s ?"), driver_name(target), g.name),
                  action = function() hd.spawn(cmd(g, target, true)) end }
end

local function gpu_menu(m, g)
    m[#m + 1] = { header = g.name .. " — " .. g.address:gsub("^0000:", "") }
    m[#m + 1] = { label = string.format(hd.t("Pilote actuel : %s"), g.driver), disabled = true,
                  detail = string.format(hd.t("démarrage : %s%s"), driver_name(g.boot),
                      g.boot_saved and hd.t(" (choix gardé par hyprdesk)") or "") }
    if g.busy then
        m[#m + 1] = { label = hd.t("Bascule en cours…"), disabled = true }
        return
    end
    if not g.ready then
        -- Le hyprdesk root (pkexec) ne connaît pas encore `gpu` : rien ne
        -- basculerait, on le dit ici plutôt qu'après la confirmation.
        m[#m + 1] = { label = hd.t("Bascule indisponible"), disabled = true,
                      detail = hd.t("le hyprdesk root est trop ancien : mettre à jour le paquet hyprdesk") }
        return
    end
    if g.state == "vm" then
        m[#m + 1] = { label = hd.t("Utilisé par une VM"), disabled = true,
                      detail = hd.t("éteindre la VM pour basculer") }
        return
    end
    if g.state == "nvidia" then
        ask(m, g, "vfio", hd.t("Rendre aux VM (vfio-pci) — garder ce choix au démarrage ?"))
    elseif not g.nvidia_module then
        m[#m + 1] = { label = hd.t("Pilote nvidia absent pour ce noyau"), disabled = true,
                      detail = hd.t("paquet nvidia (ou nvidia-open) à installer") }
    else
        ask(m, g, "nvidia", hd.t("Passer sur nvidia — garder ce choix au démarrage ?"))
    end
    -- Démarrage différent du pilote actuel : le garder sans basculer.
    local cur = g.state == "nvidia" and "nvidia" or (g.state ~= "none" and "vfio" or nil)
    if cur and g.boot ~= cur then
        m[#m + 1] = "-"
        m[#m + 1] = { label = string.format(hd.t("Démarrer aussi sur %s"), driver_name(cur)),
                      detail = string.format(hd.t("au lieu de %s, sans rien basculer maintenant"), driver_name(g.boot)),
                      action = function() hd.spawn(cmd(g, cur, true)) end }
    end
end

return {
    name        = hd.t("GPU (VFIO / NVIDIA)"),
    description = hd.t("GPU NVIDIA réservé aux VM : bascule à chaud entre vfio-pci et nvidia, pour cette session ou aussi au démarrage. Caché sans passthrough."),
    interval    = 3,
    anchor      = "top-right",
    -- texte : NVIDIA / VFIO / VM à côté de l'icône.
    options     = { texte = true },
    render = function(o)
        local gpus = hd.gpus()
        local g = gpus[1]
        if not g then return { state = "hidden" } end
        local label = g.busy and "…" or LABELS[g.state] or "?"
        if #gpus > 1 then label = label .. " +" .. (#gpus - 1) end
        return {
            icon = "hyprdesk-gpu-symbolic",
            text = o.texte and label or "",
            tooltip = tooltip(g),
            state = g.busy and "active" or g.state == "nvidia" and "active"
                or g.state == "vm" and "warning" or "normal",
        }
    end,
    menu = function()
        local gpus = hd.gpus()
        local m = {}
        if #gpus == 0 then
            return { { header = hd.t("Aucun GPU NVIDIA à basculer") } }
        end
        for i, g in ipairs(gpus) do
            if i > 1 then m[#m + 1] = "-" end
            gpu_menu(m, g)
        end
        return m
    end,
}
