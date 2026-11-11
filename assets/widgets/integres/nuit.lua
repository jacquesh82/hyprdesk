-- Éclairage nocturne (hyprsunset) : écran plus chaud le soir, selon un
-- horaire (options debut / fin) ou à la demande depuis le menu.
--
-- hyprdesk pilote hyprsunset par son IPC (`hyprctl hyprsunset temperature
-- <K>` / `identity`) et le lance s'il ne tourne pas. Un autre outil déjà
-- en place (wlsunset, gammastep, redshift) garde la main : le module le
-- signale et ne touche à rien (deux outils se disputeraient l'écran).

local mode = "auto"      -- auto | on | off
local temp_choice = nil  -- température choisie dans le menu (sinon l'option)
local applied = nil      -- dernier état appliqué : "on:<K>" ou "off"

local TEMPS = { 3400, 4000, 4500, 5000 }

local function minutes(hhmm, default)
    local h, m = tostring(hhmm or ""):match("^(%d%d?):(%d%d)$")
    if not h then return default end
    return tonumber(h) * 60 + tonumber(m)
end

-- Dans la plage du soir ? (la plage peut passer minuit)
local function evening(o)
    local t = os.date("*t")
    local now = t.hour * 60 + t.min
    local from, to = minutes(o.debut, 19 * 60 + 30), minutes(o.fin, 7 * 60)
    if from == to then return false end
    if from < to then return now >= from and now < to end
    return now >= from or now < to
end

local function foreign()
    for _, p in ipairs({ "wlsunset", "gammastep", "redshift" }) do
        if hd.exec("pgrep -x " .. p .. " 2>/dev/null") ~= "" then return p end
    end
    return nil
end

local function temperature(o)
    local t = tonumber(temp_choice or o.temperature) or 4000
    return math.max(1000, math.min(6500, math.floor(t)))
end

local function wanted(o)
    if mode == "on" then return true end
    if mode == "off" then return false end
    return evening(o)
end

local function apply(on, k)
    local key = on and ("on:" .. k) or "off"
    if key == applied then return end
    if hd.exec("pgrep -x hyprsunset 2>/dev/null") == "" then
        hd.spawn(on and ("hyprsunset -t " .. k) or "hyprsunset -i")
    else
        hd.exec("hyprctl hyprsunset " .. (on and ("temperature " .. k) or "identity"))
    end
    applied = key
end

return {
    name        = hd.t("Éclairage nocturne"),
    description = hd.t("Écran plus chaud le soir (hyprsunset) : horaire ou à la demande."),
    interval    = 30,
    anchor      = "top-right",
    options     = { debut = "19:30", fin = "07:00", temperature = 4000 },
    render = function(o)
        local other = foreign()
        if other then
            return { icon = "night-light-symbolic",
                     tooltip = string.format(hd.t("Éclairage nocturne géré par %s"), other) }
        end
        if hd.exec("command -v hyprsunset") == "" then return { state = "hidden" } end
        local on = wanted(o)
        apply(on, temperature(o))
        return {
            icon = on and "night-light-symbolic" or "night-light-disabled-symbolic",
            state = on and "active" or "normal",
            tooltip = on and string.format(hd.t("Éclairage nocturne : %d K"), temperature(o))
                or hd.t("Éclairage nocturne désactivé"),
        }
    end,
    menu = function(o)
        local m = { { header = hd.t("Éclairage nocturne") } }
        local other = foreign()
        if other then
            m[#m + 1] = { label = string.format(hd.t("Géré par %s"), other),
                          detail = hd.t("à retirer de hyprland.lua pour laisser hyprdesk le gérer") }
            return m
        end
        local function set(v)
            return function() mode = v; apply(wanted(o), temperature(o)) end
        end
        m[#m + 1] = { label = hd.t("Automatique"), detail = (o.debut or "19:30") .. " → " .. (o.fin or "07:00"),
                      checked = mode == "auto", action = set("auto") }
        m[#m + 1] = { label = hd.t("Activé"), checked = mode == "on", action = set("on") }
        m[#m + 1] = { label = hd.t("Désactivé"), checked = mode == "off", action = set("off") }
        m[#m + 1] = { header = hd.t("Température") }
        for _, k in ipairs(TEMPS) do
            m[#m + 1] = { label = k .. " K", detail = k <= 3400 and hd.t("très chaud") or nil,
                          checked = temperature(o) == k,
                          action = function() temp_choice = k; apply(wanted(o), temperature(o)) end }
        end
        return m
    end,
}
