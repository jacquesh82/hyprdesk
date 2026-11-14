-- Réseau (NetworkManager) : filaire et Wi-Fi, réseaux à proximité (recherche
-- relancée à l'ouverture du menu), connexion à un réseau protégé avec mot de
-- passe, VPN et WireGuard, mode avion.
--
-- Le mot de passe ne passe jamais sur une ligne de commande (visible par ps) :
-- il est écrit dans un fichier de $XDG_RUNTIME_DIR (dossier privé de la
-- session), lu par `nmcli connection up … passwd-file`, puis effacé.
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

-- Réseau protégé choisi, dont on attend le mot de passe (menu rouvert).
local pending = nil

-- Connexions enregistrées : { nom = { type, active } }.
local function saved()
    local t = {}
    for _, l in ipairs(lines("nmcli -t -f NAME,TYPE,ACTIVE connection show 2>/dev/null")) do
        local f = fields(l)
        t[f[1]] = { type = f[2], active = f[3] == "yes" }
    end
    return t
end

local function airplane()
    local out = hd.exec("rfkill -n -o SOFT 2>/dev/null")
    if out ~= "" then return not out:find("unblocked") end
    return hd.exec("LC_ALL=C nmcli -t radio 2>/dev/null"):match("^%a+:(%a+)") == "disabled"
end

local function set_airplane(on)
    if hd.exec("command -v rfkill") ~= "" then
        hd.spawn("rfkill " .. (on and "block" or "unblock") .. " all")
    else
        hd.spawn("nmcli radio all " .. (on and "off" or "on"))
    end
end

-- Connexion avec mot de passe, sans le mettre sur une ligne de commande.
local function connect_with_password(ssid, password)
    local dir = os.getenv("XDG_RUNTIME_DIR") or "/tmp"
    local file = dir .. "/hyprdesk-wifi-" .. tostring(os.time()) .. ".secret"
    local f = io.open(file, "w")
    if not f then return end
    f:write("802-11-wireless-security.psk:" .. password .. "\n")
    f:close()
    hd.spawn("sh -c " .. q(
        "umask 077; nmcli connection add type wifi con-name " .. q(ssid) .. " ssid " .. q(ssid)
        .. " wifi-sec.key-mgmt wpa-psk >/dev/null && nmcli connection up id " .. q(ssid)
        .. " passwd-file " .. q(file) .. " || nmcli connection delete id " .. q(ssid)
        .. "; rm -f " .. q(file)))
end

return {
    name        = hd.t("Réseau"),
    description = hd.t("Connexion filaire et Wi-Fi, réseaux à proximité (NetworkManager)."),
    interval    = 5,
    anchor      = "top-right",
    options     = { texte = false },
    on_toggle = function()
        local on = hd.exec("LC_ALL=C nmcli radio wifi 2>/dev/null") == "enabled"
        hd.exec("LC_ALL=C nmcli radio wifi " .. (on and "off" or "on"))
        local after = hd.exec("LC_ALL=C nmcli radio wifi 2>/dev/null")
        if after ~= (on and "disabled" or "enabled") then error(hd.t("Le changement Wi-Fi n'a pas été confirmé")) end
    end,
    render = function(o)
        local wifi, wired = connections()
        local toggled = hd.exec("LC_ALL=C nmcli radio wifi 2>/dev/null") == "enabled"
        if wired then
            return { icon = "network-wired-symbolic", text = o.texte and wired or "", toggled = toggled,
                     tooltip = hd.t("Filaire — ") .. wired .. (wifi and (hd.t("\nWi-Fi — ") .. wifi) or "") }
        end
        if wifi then
            local s = 0
            for _, n in ipairs(networks()) do if n.used then s = n.signal end end
            return { icon = wifi_icon(s), text = o.texte and wifi or "", tooltip = hd.t("Wi-Fi — ") .. wifi .. " (" .. s .. " %)", toggled = toggled }
        end
        return { icon = "network-offline-symbolic", text = o.texte and hd.t("Hors ligne") or "", tooltip = hd.t("Aucune connexion"), state = "warning", toggled = toggled }
    end,
    menu = function()
        local wifi, wired = connections()
        local radio = hd.exec("LC_ALL=C nmcli radio wifi 2>/dev/null") == "enabled"
        local known = saved()
        local plane = airplane()
        -- Recherche en arrière-plan : la prochaine ouverture montre les nouveaux réseaux.
        if radio then hd.spawn("nmcli dev wifi rescan") end
        local m = { { header = hd.t("Réseau") } }
        if pending then
            local ssid = pending
            pending = nil
            m[#m + 1] = { input = hd.t("Mot de passe"), label = string.format(hd.t("Wi-Fi « %s »"), ssid), secret = true,
                          action = function(pw) if pw and pw ~= "" then connect_with_password(ssid, pw) end end }
            m[#m + 1] = "-"
        end
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
                local action = nil
                if not net.used then
                    if net.secure and not known[net.ssid] then
                        -- Mot de passe demandé : le menu se rouvre avec un champ de saisie.
                        action = function()
                            pending = net.ssid
                            hd.spawn(string.format("%q menu reseau", hd.exe))
                        end
                    else
                        action = function() hd.spawn("nmcli dev wifi connect " .. q(net.ssid)) end
                    end
                end
                m[#m + 1] = {
                    label = net.ssid,
                    detail = net.signal .. " %" .. (net.secure and hd.t(" · sécurisé") or ""),
                    icon = wifi_icon(net.signal),
                    checked = net.used or nil,
                    action = action,
                }
            end
            m[#m + 1] = { label = hd.t("Rechercher les réseaux"), action = function() hd.spawn("nmcli dev wifi rescan") end }
        end
        -- VPN et WireGuard enregistrés.
        local vpns = {}
        for name, c in pairs(known) do
            if c.type == "vpn" or c.type == "wireguard" then vpns[#vpns + 1] = { name = name, active = c.active } end
        end
        table.sort(vpns, function(a, b) return a.name < b.name end)
        if #vpns > 0 then
            m[#m + 1] = { header = hd.t("VPN") }
            for _, v in ipairs(vpns) do
                m[#m + 1] = { label = v.name, checked = v.active,
                              action = function() hd.spawn("nmcli connection " .. (v.active and "down" or "up") .. " id " .. q(v.name)) end }
            end
        end
        m[#m + 1] = "-"
        m[#m + 1] = { label = hd.t("Mode avion"), checked = plane, action = function() set_airplane(not plane) end }
        m[#m + 1] = { label = hd.t("Réglages réseau…"), action = function() hd.spawn("nm-connection-editor") end }
        return m
    end,
}
