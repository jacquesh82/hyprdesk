-- Bluetooth (bluetoothctl) : activation, appareils connectés et appairés
-- (batterie quand l'appareil la donne), recherche d'appareils à proximité,
-- appairage en un clic, appareils oubliés depuis « Gérer les appareils… ».
local function devices(kind)
    local list = {}
    for mac, name in hd.exec("LC_ALL=C bluetoothctl devices " .. kind .. " 2>/dev/null"):gmatch("Device (%S+) ([^\n]+)") do
        list[#list + 1] = { mac = mac, name = name }
    end
    return list
end

local function q(s) return "'" .. s:gsub("'", "'\\''") .. "'" end

-- Menu rouvert en mode gestion (oublier un appareil).
local manage = false

-- Batterie d'un appareil (« Battery Percentage: 0x55 (85) »), si elle est connue.
local function battery(mac)
    return tonumber(hd.exec("LC_ALL=C bluetoothctl info " .. mac .. " 2>/dev/null"):match("Battery Percentage: %S+ %((%d+)%)") or "")
end

local function reopen()
    hd.spawn(string.format("%q menu bluetooth", hd.exe))
end

local function powered()
    local show = hd.exec("LC_ALL=C bluetoothctl show 2>/dev/null")
    if show == "" then return nil end
    return show:find("Powered: yes") ~= nil
end

return {
    name        = hd.t("Bluetooth"),
    description = hd.t("Activation et appareils (connexion, déconnexion)."),
    interval    = 10,
    anchor      = "top-right",
    options     = { texte = false },
    render = function(o)
        local on = powered()
        if on == nil then return { state = "hidden" } end
        if not on then return { icon = "bluetooth-disabled-symbolic", tooltip = hd.t("Bluetooth désactivé") } end
        local c = devices("Connected")
        local names = {}
        for _, d in ipairs(c) do names[#names + 1] = d.name end
        return {
            icon = #c > 0 and "bluetooth-active-symbolic" or "bluetooth-symbolic",
            text = (o.texte and #c > 0) and table.concat(names, ", ") or "",
            tooltip = #c > 0 and (hd.t("Connectés : ") .. table.concat(names, ", ")) or hd.t("Bluetooth activé"),
            state = #c > 0 and "active" or "normal",
        }
    end,
    menu = function()
        local on = powered()
        local m = { { header = hd.t("Bluetooth") },
                    { label = hd.t("Bluetooth"), checked = on and true or false,
                      action = function() hd.exec("LC_ALL=C bluetoothctl power " .. (on and "off" or "on")) end } }
        if not on then return m end
        local paired = devices("Paired")
        if manage then
            manage = false
            m[#m + 1] = { header = hd.t("Oublier un appareil") }
            for _, d in ipairs(paired) do
                m[#m + 1] = { label = d.name, icon = "window-close-symbolic",
                              confirm = string.format(hd.t("Oublier « %s » ?"), d.name),
                              action = function() hd.spawn("bluetoothctl remove " .. d.mac) end }
            end
            return m
        end
        local connected = {}
        for _, d in ipairs(devices("Connected")) do connected[d.mac] = true end
        local known = {}
        if #paired > 0 then m[#m + 1] = { header = hd.t("Appareils") } end
        for _, d in ipairs(paired) do
            known[d.mac] = true
            local c = connected[d.mac]
            local detail = nil
            if c then
                local b = battery(d.mac)
                detail = hd.t("connecté") .. (b and (" · " .. b .. " %") or "")
            end
            m[#m + 1] = { label = d.name, detail = detail, checked = c or nil,
                          action = function()
                              if c then hd.spawn("bluetoothctl disconnect " .. d.mac)
                              else hd.spawn("sh -c " .. q("bluetoothctl trust " .. d.mac .. " >/dev/null; bluetoothctl connect " .. d.mac)) end
                          end }
        end
        -- Appareils vus par la dernière recherche, pas encore appairés.
        local nearby = {}
        for _, d in ipairs(devices("")) do
            if not known[d.mac] and not d.name:match("^%x%x[-:]%x%x[-:]") then nearby[#nearby + 1] = d end
        end
        if #nearby > 0 then
            m[#m + 1] = { header = hd.t("À proximité") }
            for i, d in ipairs(nearby) do
                if i > 6 then break end
                m[#m + 1] = { label = d.name, detail = hd.t("appairer"),
                              action = function()
                                  hd.spawn("sh -c " .. q("bluetoothctl pair " .. d.mac .. " && bluetoothctl trust " .. d.mac
                                      .. " && bluetoothctl connect " .. d.mac))
                              end }
            end
        end
        m[#m + 1] = "-"
        m[#m + 1] = { label = hd.t("Rechercher des appareils"), detail = hd.t("20 s"),
                      action = function() hd.spawn("bluetoothctl --timeout 20 scan on") end }
        if #paired > 0 then
            m[#m + 1] = { label = hd.t("Gérer les appareils…"), action = function() manage = true; reopen() end }
        end
        m[#m + 1] = { label = hd.t("Réglages Bluetooth…"), action = function() hd.spawn("blueman-manager") end }
        return m
    end,
}
