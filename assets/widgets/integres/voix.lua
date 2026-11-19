-- LYRA vocale dans la barre : écoute du mot « Lyra » et réponses à voix
-- haute, sans passer par le Panorama.
--
-- L'état est écrit par le démon hyprdesk dans $XDG_RUNTIME_DIR/hyprdesk/voix
-- (lignes clé=valeur : state, detail, settings). state : absent (LYRA
-- coupée : le module se cache), off, installing, listening, awake, thinking,
-- speaking, error. Tant que ce module est dans la barre, LYRA vocale peut
-- écouter même si le paquet Panorama est coupé.

local function read()
    local out = { state = "absent", detail = "", settings = "assistant/voix" }
    local dir = os.getenv("XDG_RUNTIME_DIR")
    if not dir then return out end
    local f = io.open(dir .. "/hyprdesk/voix", "r")
    if not f then return out end
    for line in f:lines() do
        local k, v = line:match("^([a-z]+)=(.*)$")
        if k then out[k] = v end
    end
    f:close()
    return out
end

local function hyprdesk(args)
    hd.spawn(string.format("%q %s", hd.exe, args))
end

local MIC = "audio-input-microphone-symbolic"
local MUTED = "microphone-sensitivity-muted-symbolic"

local function with(detail, text)
    if detail ~= nil and detail ~= "" then return text .. " — « " .. detail .. " »" end
    return text
end

return {
    name        = hd.t("LYRA vocale"),
    description = hd.t("Écoute du mot « Lyra » et réponses à voix haute : clic pour activer ou couper le micro."),
    interval    = 1,
    anchor      = "top-right",
    render = function()
        local v = read()
        local s = v.state
        if s == "absent" then return { state = "hidden" } end
        if s == "off" then
            return { icon = MUTED, state = "normal",
                     tooltip = hd.t("LYRA vocale coupée — clic : l'activer (la voix s'installe au premier usage, ~350 Mo)") }
        end
        if s == "installing" then
            return { icon = MIC, state = "warning",
                     tooltip = with(v.detail, hd.t("LYRA vocale : installation de la voix…")) }
        end
        if s == "error" then
            return { icon = MUTED, state = "critical",
                     tooltip = with(v.detail, hd.t("LYRA vocale : erreur")) }
        end
        if s == "awake" then
            return { icon = MIC, state = "active", text = hd.t("Je vous écoute…"),
                     tooltip = with(v.detail, hd.t("LYRA vous écoute")) }
        end
        if s == "thinking" then
            return { icon = MIC, state = "active", text = hd.t("LYRA réfléchit…"),
                     tooltip = with(v.detail, hd.t("LYRA réfléchit")) }
        end
        if s == "speaking" then
            return { icon = MIC, state = "active", text = hd.t("LYRA parle…"),
                     tooltip = hd.t("LYRA répond à voix haute") }
        end
        return { icon = MIC, state = "active",
                 tooltip = hd.t("LYRA vocale à l'écoute : dites « Lyra » — clic pour couper le micro") }
    end,
    on_click = function()
        hyprdesk("panorama voice toggle")
    end,
    menu = function()
        local v = read()
        local m = { { header = hd.t("LYRA vocale") } }
        if v.state == "off" or v.state == "error" then
            m[#m + 1] = { label = hd.t("Activer l'écoute"), icon = MIC,
                          action = function() hyprdesk("panorama voice on") end }
        elseif v.state ~= "absent" and v.state ~= "installing" then
            m[#m + 1] = { label = hd.t("Couper le micro"), icon = MUTED,
                          action = function() hyprdesk("panorama voice off") end }
        end
        m[#m + 1] = { label = hd.t("Réglages de LYRA vocale…"), icon = "preferences-system-symbolic",
                      action = function() hyprdesk("settings " .. (v.settings or "assistant/voix")) end }
        return m
    end,
}
