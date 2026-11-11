-- Captures et enregistrement de l'écran : menu de captures (zone, fenêtre,
-- écran ; copiées et enregistrées dans Images/Captures) et enregistrement
-- vidéo, avec un indicateur rouge et la durée tant qu'il tourne.
--
-- Tout passe par `hyprdesk capture …` (hyprshot, wf-recorder, slurp), comme
-- les touches Impr. L'enregistrement en cours est lu dans le fichier d'état
-- $XDG_RUNTIME_DIR/hyprdesk-recording (pid, début, fichier).

local function cmd(args)
    hd.spawn(string.format("%q capture %s", hd.exe, args))
end

local function recording()
    local dir = os.getenv("XDG_RUNTIME_DIR")
    if not dir then return nil end
    local f = io.open(dir .. "/hyprdesk-recording", "r")
    if not f then return nil end
    local pid, started, file = f:read("l", "l", "l")
    f:close()
    pid, started = tonumber(pid), tonumber(started)
    if not pid or not started then return nil end
    -- Processus disparu (plantage) : pas d'indicateur.
    local comm = io.open("/proc/" .. pid .. "/comm", "r")
    if not comm then return nil end
    local name = comm:read("l")
    comm:close()
    if name ~= "wf-recorder" then return nil end
    return { started = started, file = file }
end

local function duration(s)
    if s >= 3600 then return string.format("%d:%02d:%02d", s // 3600, s % 3600 // 60, s % 60) end
    return string.format("%d:%02d", s // 60, s % 60)
end

return {
    name        = hd.t("Capture"),
    description = hd.t("Captures d'écran et enregistrement vidéo (hyprshot, wf-recorder)."),
    interval    = 1,
    anchor      = "top-right",
    options     = { toujours = true },
    render = function(o)
        local r = recording()
        if r then
            return { icon = "media-record-symbolic", state = "critical",
                     text = duration(math.max(0, os.time() - r.started)),
                     tooltip = hd.t("Enregistrement en cours") }
        end
        if not o.toujours then return { state = "hidden" } end
        return { icon = "camera-photo-symbolic", text = "", tooltip = hd.t("Capture d'écran") }
    end,
    menu = function()
        local r = recording()
        local m = { { header = hd.t("Capture") } }
        if r then
            m[#m + 1] = { label = hd.t("Arrêter l'enregistrement"), icon = "media-playback-stop-symbolic",
                          detail = duration(math.max(0, os.time() - r.started)),
                          action = function() cmd("arreter") end }
            return m
        end
        m[#m + 1] = { label = hd.t("Zone"), detail = hd.t("Impr"), action = function() cmd("zone") end }
        m[#m + 1] = { label = hd.t("Fenêtre"), detail = hd.t("Alt+Impr"), action = function() cmd("fenetre") end }
        m[#m + 1] = { label = hd.t("Écran"), detail = hd.t("Maj+Impr"), action = function() cmd("ecran") end }
        m[#m + 1] = { header = hd.t("Enregistrer") }
        m[#m + 1] = { label = hd.t("Enregistrer l'écran"), icon = "media-record-symbolic",
                      action = function() cmd("enregistrer") end }
        m[#m + 1] = { label = hd.t("Enregistrer une zone"), action = function() cmd("enregistrer zone") end }
        m[#m + 1] = "-"
        m[#m + 1] = { label = hd.t("Ouvrir le dossier des captures"),
                      action = function() hd.spawn("sh -c 'xdg-open \"$(xdg-user-dir PICTURES)/Captures\"'") end }
        return m
    end,
}
