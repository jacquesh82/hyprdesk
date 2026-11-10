-- Réseau (NetworkManager) : filaire et Wi-Fi, liste des réseaux, connexion.
local function lines(cmd)
    local t = {}
    for l in (hd.exec(cmd) .. "\n"):gmatch("(.-)\n") do
        if l ~= "" then t[#t + 1] = l end
    end
    return t
end

-- nmcli -t sépare par « : » et échappe « \: ».
local function fields(l)
    local out, cur, i = {}, "", 1
    while i <= #l do
        local c = l:sub(i, i)
        if c == "\\" and i < #l then cur = cur .. l:sub(i + 1, i + 1); i = i + 2
        elseif c == ":" then out[#out + 1] = cur; cur = ""; i = i + 1
        else cur = cur .. c; i = i + 1 end
    end
    out[#out + 1] = cur
    return out
end

local function connections()
    local wifi, wired
    for _, l in ipairs(lines("nmcli -t -f TYPE,STATE,CONNECTION device 2>/dev/null")) do
        local f = fields(l)
        if f[2] == "connected" then
            if f[1] == "wifi" and not wifi then wifi = f[3]
            elseif f[1] == "ethernet" and not wired then wired = f[3] end
        end
    end
    return wifi, wired
end

local function wifi_icon(s)
    s = tonumber(s) or 0
    if s >= 75 then return "network-wireless-signal-excellent-symbolic"
    elseif s >= 50 then return "network-wireless-signal-good-symbolic"
    elseif s >= 25 then return "network-wireless-signal-ok-symbolic" end
    return "network-wireless-signal-weak-symbolic"
end

local function networks()
    local list, seen = {}, {}
    for _, l in ipairs(lines("nmcli -t -f IN-USE,SSID,SIGNAL,SECURITY dev wifi list --rescan no 2>/dev/null")) do
        local f = fields(l)
        if f[2] ~= "" and not seen[f[2]] then
            seen[f[2]] = true
            list[#list + 1] = { used = f[1] == "*", ssid = f[2], signal = tonumber(f[3]) or 0, secure = f[4] ~= "" and f[4] ~= "--" }
        end
    end
    return list
end

local function q(s) return "'" .. s:gsub("'", "'\\''") .. "'" end

return {
    name        = hd.t("Réseau"),
    description = hd.t("Connexion filaire et Wi-Fi, réseaux à proximité (NetworkManager)."),
    interval    = 5,
    anchor      = "top-right",
    options     = { texte = false },
    render = function(o)
        local wifi, wired = connections()
        if wired then
            return { icon = "network-wired-symbolic", text = o.texte and wired or "",
                     tooltip = hd.t("Filaire — ") .. wired .. (wifi and (hd.t("\nWi-Fi — ") .. wifi) or "") }
        end
        if wifi then
            local s = 0
            for _, n in ipairs(networks()) do if n.used then s = n.signal end end
            return { icon = wifi_icon(s), text = o.texte and wifi or "", tooltip = hd.t("Wi-Fi — ") .. wifi .. " (" .. s .. " %)" }
        end
        return { icon = "network-offline-symbolic", text = o.texte and hd.t("Hors ligne") or "", tooltip = hd.t("Aucune connexion"), state = "warning" }
    end,
    menu = function()
        local wifi, wired = connections()
        local radio = hd.exec("LC_ALL=C nmcli radio wifi 2>/dev/null") == "enabled"
        local m = { { header = hd.t("Réseau") } }
        if wired then
            m[#m + 1] = { label = hd.t("Ethernet"), detail = wired, icon = "network-wired-symbolic", checked = true }
        end
        m[#m + 1] = { label = hd.t("Wi-Fi"), checked = radio,
                      action = function() hd.exec("LC_ALL=C nmcli radio wifi " .. (radio and "off" or "on")) end }
        if radio then
            local n = 0
            for _, net in ipairs(networks()) do
                n = n + 1
                if n > 8 then break end
                m[#m + 1] = {
                    label = net.ssid,
                    detail = net.signal .. " %" .. (net.secure and hd.t(" · sécurisé") or ""),
                    icon = wifi_icon(net.signal),
                    checked = net.used or nil,
                    action = not net.used and function() hd.spawn("nmcli dev wifi connect " .. q(net.ssid)) end or nil,
                }
            end
        end
        m[#m + 1] = "-"
        m[#m + 1] = { label = hd.t("Réglages réseau…"), action = function() hd.spawn("nm-connection-editor") end }
        return m
    end,
}
