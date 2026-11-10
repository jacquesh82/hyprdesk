-- Bluetooth (bluetoothctl) : activation, appareils connectés et appairés.
local function devices(kind)
    local list = {}
    for mac, name in hd.exec("bluetoothctl devices " .. kind .. " 2>/dev/null"):gmatch("Device (%S+) ([^\n]+)") do
        list[#list + 1] = { mac = mac, name = name }
    end
    return list
end

local function powered()
    local show = hd.exec("bluetoothctl show 2>/dev/null")
    if show == "" then return nil end
    return show:find("Powered: yes") ~= nil
end

return {
    name        = "Bluetooth",
    description = "Activation et appareils (connexion, déconnexion).",
    interval    = 10,
    anchor      = "top-right",
    options     = { texte = false },
    render = function(o)
        local on = powered()
        if on == nil then return { state = "hidden" } end
        if not on then return { icon = "bluetooth-disabled-symbolic", tooltip = "Bluetooth désactivé" } end
        local c = devices("Connected")
        local names = {}
        for _, d in ipairs(c) do names[#names + 1] = d.name end
        return {
            icon = #c > 0 and "bluetooth-active-symbolic" or "bluetooth-symbolic",
            text = (o.texte and #c > 0) and table.concat(names, ", ") or "",
            tooltip = #c > 0 and ("Connectés : " .. table.concat(names, ", ")) or "Bluetooth activé",
            state = #c > 0 and "active" or "normal",
        }
    end,
    menu = function()
        local on = powered()
        local m = { { header = "Bluetooth" },
                    { label = "Bluetooth", checked = on and true or false,
                      action = function() hd.exec("bluetoothctl power " .. (on and "off" or "on")) end } }
        if on then
            local connected = {}
            for _, d in ipairs(devices("Connected")) do connected[d.mac] = true end
            local paired = devices("Paired")
            if #paired > 0 then m[#m + 1] = { header = "Appareils" } end
            for _, d in ipairs(paired) do
                local c = connected[d.mac]
                m[#m + 1] = { label = d.name, detail = c and "connecté" or nil, checked = c or nil,
                              action = function() hd.spawn("bluetoothctl " .. (c and "disconnect " or "connect ") .. d.mac) end }
            end
        end
        m[#m + 1] = "-"
        m[#m + 1] = { label = "Réglages Bluetooth…", action = function() hd.spawn("blueman-manager") end }
        return m
    end,
}
