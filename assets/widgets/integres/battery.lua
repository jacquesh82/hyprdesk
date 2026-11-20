-- Batteries : celle de l'ordinateur, et celles des périphériques (souris, clavier…).
-- Lues dans /sys/class/power_supply par hyprdesk (hd.batteries, sans processus).
local function batteries()
    return hd.batteries()
end

local function icon(b)
    local level = math.floor((b.cap + 5) / 10) * 10
    local suffix = (b.status == "Charging") and "-charging" or ""
    return "battery-level-" .. level .. suffix .. "-symbolic"
end

local statuses = { Charging = hd.t("en charge"), Discharging = hd.t("sur batterie"), Full = hd.t("chargée"), ["Not charging"] = hd.t("branchée") }


-- Profils d'énergie (power-profiles-daemon), si présents.
local PROFILES = {
    { id = "performance", label = hd.t("Performance") },
    { id = "balanced", label = hd.t("Équilibré") },
    { id = "power-saver", label = hd.t("Économie d'énergie") },
}

-- Profil actif : par D-Bus (hd.power_profile, sans processus), sinon
-- powerprofilesctl. Rend aussi l'état D-Bus, pour la liste des profils.
local function profile()
    local pp = hd.power_profile()
    if pp then return (pp.active ~= "" and pp.active or nil), pp end
    local p = hd.exec("powerprofilesctl get 2>/dev/null")
    return p ~= "" and p or nil
end

local function available_profiles(pp)
    local out, has = {}, nil
    if pp then
        local set = {}
        for _, id in ipairs(pp.profiles) do set[id] = true end
        has = function(id) return set[id] end
    else
        local have = hd.exec("powerprofilesctl list 2>/dev/null")
        has = function(id) return have:find(id .. ":", 1, true) end
    end
    for _, p in ipairs(PROFILES) do
        if has(p.id) then out[#out + 1] = p end
    end
    return out
end

-- Alertes de batterie faible : une fois par décharge, au seuil « faible »
-- puis au seuil « critique » (options, 20 et 10 % par défaut).
local alerted = 100

local function alert(b, o)
    local low, crit = tonumber(o.faible) or 20, tonumber(o.critique) or 10
    if b.status ~= "Discharging" then alerted = 100; return end
    if b.cap > low then alerted = 100; return end
    local level = b.cap <= crit and crit or low
    if level >= alerted then return end
    alerted = level
    hd.spawn(string.format("notify-send -a hyprdesk -u %s %q %q",
        level == crit and "critical" or "normal",
        string.format(hd.t("Batterie faible : %d %%"), b.cap),
        hd.t("Branchez l'ordinateur pour continuer à travailler.")))
end

return {
    name        = hd.t("Batterie"),
    description = hd.t("Batterie de l'ordinateur et des périphériques ; masqué s'il n'y en a pas."),
    interval    = 30,
    anchor      = "top-right",
    options     = { peripheriques = true, texte = true, faible = 20, critique = 10 },
    render = function(o)
        local all, main = batteries(), nil
        for _, b in ipairs(all) do if not b.device then main = b; break end end
        if not main and o.peripheriques then
            for _, b in ipairs(all) do if not main or b.cap < main.cap then main = b end end
        end
        if not main then
            -- Sans batterie : le profil d'énergie seul, s'il y en a.
            local p = profile()
            if not p then return { state = "hidden" } end
            return { icon = "power-profile-" .. p .. "-symbolic", text = "",
                     tooltip = hd.t("Profil d'énergie : ") .. p }
        end
        if not main.device then alert(main, o) end
        local tips = {}
        for _, b in ipairs(all) do tips[#tips + 1] = b.name .. " — " .. b.cap .. " %" end
        return {
            icon = icon(main),
            text = o.texte and (main.cap .. " %") or "",
            tooltip = table.concat(tips, "\n"),
            state = (main.cap <= (tonumber(o.critique) or 10) and "critical")
                or (main.cap <= (tonumber(o.faible) or 20) and "warning") or "normal",
        }
    end,
    menu = function()
        local m = {}
        local all = batteries()
        if #all > 0 then
            m[#m + 1] = { header = hd.t("Batteries") }
            for _, b in ipairs(all) do
                m[#m + 1] = { label = b.name, detail = b.cap .. " % · " .. (statuses[b.status] or b.status), icon = icon(b) }
            end
        end
        local current, pp = profile()
        if current then
            m[#m + 1] = { header = hd.t("Profil d'énergie") }
            for _, p in ipairs(available_profiles(pp)) do
                m[#m + 1] = { label = p.label, icon = "power-profile-" .. p.id .. "-symbolic",
                              checked = p.id == current,
                              action = function() hd.spawn("powerprofilesctl set " .. p.id) end }
            end
        end
        return m
    end,
}
