-- Batteries : celle de l'ordinateur, et celles des périphériques (souris, clavier…).
local function read(p)
    local f = io.open(p, "r")
    if not f then return nil end
    local v = f:read("*l")
    f:close()
    return v
end

local function batteries()
    local list = {}
    for name in hd.exec("ls /sys/class/power_supply 2>/dev/null"):gmatch("[^\n]+") do
        local d = "/sys/class/power_supply/" .. name
        if read(d .. "/type") == "Battery" then
            local cap = tonumber(read(d .. "/capacity") or "")
            if cap then
                list[#list + 1] = {
                    name = read(d .. "/model_name") or name,
                    cap = cap,
                    status = read(d .. "/status") or "",
                    device = (read(d .. "/scope") or "System") == "Device",
                }
            end
        end
    end
    return list
end

local function icon(b)
    local level = math.floor((b.cap + 5) / 10) * 10
    local suffix = (b.status == "Charging") and "-charging" or ""
    return "battery-level-" .. level .. suffix .. "-symbolic"
end

local statuses = { Charging = hd.t("en charge"), Discharging = hd.t("sur batterie"), Full = hd.t("chargée"), ["Not charging"] = hd.t("branchée") }

return {
    name        = hd.t("Batterie"),
    description = hd.t("Batterie de l'ordinateur et des périphériques ; masqué s'il n'y en a pas."),
    interval    = 30,
    anchor      = "top-right",
    options     = { peripheriques = true, texte = true },
    render = function(o)
        local all, main = batteries(), nil
        for _, b in ipairs(all) do if not b.device then main = b; break end end
        if not main and o.peripheriques then
            for _, b in ipairs(all) do if not main or b.cap < main.cap then main = b end end
        end
        if not main then return { state = "hidden" } end
        local tips = {}
        for _, b in ipairs(all) do tips[#tips + 1] = b.name .. " — " .. b.cap .. " %" end
        return {
            icon = icon(main),
            text = o.texte and (main.cap .. " %") or "",
            tooltip = table.concat(tips, "\n"),
            state = (main.cap <= 10 and "critical") or (main.cap <= 20 and "warning") or "normal",
        }
    end,
    menu = function()
        local m = { { header = hd.t("Batteries") } }
        for _, b in ipairs(batteries()) do
            m[#m + 1] = { label = b.name, detail = b.cap .. " % · " .. (statuses[b.status] or b.status), icon = icon(b) }
        end
        return m
    end,
}
